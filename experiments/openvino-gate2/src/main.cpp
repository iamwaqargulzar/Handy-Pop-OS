#include <arpa/inet.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>
#ifdef __GLIBC__
#include <malloc.h>
#endif

#include <atomic>
#include <chrono>
#include <cstring>
#include <filesystem>
#include <iostream>
#include <memory>
#include <mutex>
#include <optional>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

#include <nlohmann/json.hpp>
#include <openvino/openvino.hpp>
#include <openvino/genai/automatic_speech_recognition/pipeline.hpp>

namespace {
using json = nlohmann::json;
constexpr std::uint32_t kProtocolVersion = 1;
constexpr std::uint32_t kMaxJsonBytes = 1024 * 1024;
constexpr std::size_t kMaxAudioBytes = 30ULL * 60 * 16000 * sizeof(float);

void release_unused_heap() {
#ifdef __GLIBC__
    ::malloc_trim(0);
#endif
}

struct WorkerState {
    std::mutex mutex;
    std::unique_ptr<ov::genai::ASRPipeline> pipeline;
    std::string model_path;
    std::string last_error;
    std::atomic<bool> busy{false};
    std::atomic<bool> shutting_down{false};
};

class BusyGuard {
  public:
    explicit BusyGuard(std::atomic<bool>& busy) : busy_(busy) {
        bool expected = false;
        acquired_ = busy_.compare_exchange_strong(expected, true);
    }
    ~BusyGuard() {
        if (acquired_) busy_.store(false);
    }
    bool acquired() const { return acquired_; }

  private:
    std::atomic<bool>& busy_;
    bool acquired_ = false;
};

void read_exact(int fd, void* output, std::size_t bytes) {
    auto* cursor = static_cast<unsigned char*>(output);
    while (bytes > 0) {
        const ssize_t count = ::read(fd, cursor, bytes);
        if (count == 0) throw std::runtime_error("unexpected end of frame");
        if (count < 0) {
            if (errno == EINTR) continue;
            throw std::runtime_error(std::string("read failed: ") + std::strerror(errno));
        }
        cursor += count;
        bytes -= static_cast<std::size_t>(count);
    }
}

void write_exact(int fd, const void* input, std::size_t bytes) {
    const auto* cursor = static_cast<const unsigned char*>(input);
    while (bytes > 0) {
        const ssize_t count = ::write(fd, cursor, bytes);
        if (count < 0) {
            if (errno == EINTR) continue;
            throw std::runtime_error(std::string("write failed: ") + std::strerror(errno));
        }
        cursor += count;
        bytes -= static_cast<std::size_t>(count);
    }
}

json error_response(const std::string& code, const std::string& message) {
    return {{"protocol_version", kProtocolVersion}, {"ok", false},
            {"error", {{"code", code}, {"message", message}}}};
}

void send_response(int fd, const json& response) {
    const std::string body = response.dump();
    const std::uint32_t length = htonl(static_cast<std::uint32_t>(body.size()));
    write_exact(fd, &length, sizeof(length));
    write_exact(fd, body.data(), body.size());
}

json status_response(WorkerState& state) {
    std::lock_guard<std::mutex> lock(state.mutex);
    return {{"protocol_version", kProtocolVersion}, {"ok", true},
            {"status", {{"busy", state.busy.load()},
                        {"loaded", state.pipeline != nullptr},
                        {"model_path", state.model_path},
                        {"device", state.pipeline ? "NPU" : ""},
                        {"last_error", state.last_error}}}};
}

json probe() {
    ov::Core core;
    const auto devices = core.get_available_devices();
    const bool npu = std::find(devices.begin(), devices.end(), "NPU") != devices.end();
    return {{"protocol_version", kProtocolVersion}, {"ok", true},
            {"probe", {{"available_devices", devices}, {"npu_available", npu},
                       {"actual_device", npu ? "NPU" : ""}}}};
}

json process_request(const json& request, std::vector<unsigned char>& payload,
                     WorkerState& state) {
    if (request.value("protocol_version", 0U) != kProtocolVersion)
        return error_response("unsupported_version", "protocol_version must be 1");
    const std::string command = request.value("command", "");
    if (command == "probe") return probe();
    if (command == "status") return status_response(state);

    BusyGuard guard(state.busy);
    if (!guard.acquired()) return error_response("busy", "worker already has an active request");

    if (command == "load_model") {
        const std::string path = request.value("model_path", "");
        if (path.empty() || !std::filesystem::is_directory(path))
            return error_response("invalid_model", "model_path is not a directory");
        try {
            auto pipeline = std::make_unique<ov::genai::ASRPipeline>(path, "NPU", ov::AnyMap{});
            std::lock_guard<std::mutex> lock(state.mutex);
            state.pipeline = std::move(pipeline);
            state.model_path = path;
            state.last_error.clear();
            release_unused_heap();
            return {{"protocol_version", kProtocolVersion}, {"ok", true},
                    {"loaded", {{"model_path", path}, {"actual_device", "NPU"}}}};
        } catch (const std::exception& error) {
            std::lock_guard<std::mutex> lock(state.mutex);
            state.last_error = error.what();
            return error_response("model_load_failed", error.what());
        }
    }

    if (command == "unload_model") {
        std::lock_guard<std::mutex> lock(state.mutex);
        state.pipeline.reset();
        state.model_path.clear();
        release_unused_heap();
        return {{"protocol_version", kProtocolVersion}, {"ok", true}, {"unloaded", true}};
    }

    if (command == "transcribe") {
        if (payload.empty() || payload.size() % sizeof(float) != 0)
            return error_response("invalid_audio", "payload must contain little-endian f32 samples");
        std::lock_guard<std::mutex> lock(state.mutex);
        if (!state.pipeline) return error_response("model_not_loaded", "load a model first");
        std::vector<float> audio(payload.size() / sizeof(float));
        std::memcpy(audio.data(), payload.data(), payload.size());
        auto config = state.pipeline->get_generation_config();
        config.language = request.value("language", "<|en|>");
        config.task = "transcribe";
        config.return_timestamps = true;
        config.word_timestamps = false;
        const auto started = std::chrono::steady_clock::now();
        auto result = state.pipeline->generate(audio, config);
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - started).count();
        json chunks = json::array();
        if (result.chunks && !result.chunks->empty()) {
            for (const auto& chunk : result.chunks->front())
                chunks.push_back(json::object({{"start", chunk.start_ts},
                                               {"end", chunk.end_ts},
                                               {"text", chunk.text}}));
        }
        return {{"protocol_version", kProtocolVersion}, {"ok", true},
                {"transcription", {{"text", result.texts.empty() ? "" : result.texts.front()},
                                   {"segments", chunks}, {"elapsed_ms", elapsed},
                                   {"actual_device", "NPU"}}}};
    }

    if (command == "shutdown") {
        state.shutting_down.store(true);
        return {{"protocol_version", kProtocolVersion}, {"ok", true}, {"shutting_down", true}};
    }
    return error_response("unknown_command", "unsupported command");
}

void serve_client(int fd, WorkerState& state) {
    try {
        std::uint32_t network_length = 0;
        read_exact(fd, &network_length, sizeof(network_length));
        const std::uint32_t json_length = ntohl(network_length);
        if (json_length == 0 || json_length > kMaxJsonBytes)
            throw std::runtime_error("invalid JSON frame length");
        std::string body(json_length, '\0');
        read_exact(fd, body.data(), body.size());
        const json request = json::parse(body);
        const std::size_t payload_bytes = request.value("payload_bytes", 0ULL);
        if (payload_bytes > kMaxAudioBytes) throw std::runtime_error("audio payload too large");
        std::vector<unsigned char> payload(payload_bytes);
        if (!payload.empty()) read_exact(fd, payload.data(), payload.size());
        send_response(fd, process_request(request, payload, state));
    } catch (const nlohmann::json::exception& error) {
        send_response(fd, error_response("invalid_json", error.what()));
    } catch (const std::exception& error) {
        send_response(fd, error_response("invalid_frame", error.what()));
    }
    ::close(fd);
}
}  // namespace

int main(int argc, char** argv) {
    if (argc != 2) {
        std::cerr << "usage: " << argv[0] << " <unix-socket-path>\n";
        return 2;
    }
    const std::filesystem::path socket_path = argv[1];
    if (socket_path.string().size() >= sizeof(sockaddr_un::sun_path)) {
        std::cerr << "socket path is too long\n";
        return 2;
    }
    std::filesystem::remove(socket_path);
    const int server = ::socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0);
    if (server < 0) throw std::runtime_error("cannot create Unix socket");
    const int server_flags = ::fcntl(server, F_GETFL, 0);
    if (server_flags < 0 || ::fcntl(server, F_SETFL, server_flags | O_NONBLOCK) != 0)
        throw std::runtime_error("cannot make Unix socket non-blocking");
    sockaddr_un address{};
    address.sun_family = AF_UNIX;
    std::strncpy(address.sun_path, socket_path.c_str(), sizeof(address.sun_path) - 1);
    if (::bind(server, reinterpret_cast<sockaddr*>(&address), sizeof(address)) != 0)
        throw std::runtime_error(std::string("bind failed: ") + std::strerror(errno));
    ::chmod(socket_path.c_str(), S_IRUSR | S_IWUSR);
    if (::listen(server, 8) != 0) throw std::runtime_error("listen failed");

    WorkerState state;
    while (!state.shutting_down.load()) {
        const int client = ::accept4(server, nullptr, nullptr, SOCK_CLOEXEC);
        if (client < 0) {
            if (errno == EINTR) continue;
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                std::this_thread::sleep_for(std::chrono::milliseconds(20));
                continue;
            }
            break;
        }
        std::thread(serve_client, client, std::ref(state)).detach();
    }
    ::close(server);
    std::filesystem::remove(socket_path);
    return 0;
}
