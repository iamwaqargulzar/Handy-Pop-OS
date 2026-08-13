#include <arpa/inet.h>
#include <fcntl.h>
#include <sys/socket.h>
#include <sys/prctl.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <signal.h>
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
#include <openvino/genai/tokenizer.hpp>
#include "whisper/feature_extractor.hpp"
#include "eddy/backends/openvino_backend.hpp"
#include "eddy/models/parakeet-v2/parakeet.hpp"
#include "eddy/models/parakeet-v2/parakeet_openvino.hpp"

namespace {
using json = nlohmann::json;
constexpr std::uint32_t kProtocolVersion = 1;
constexpr std::uint32_t kMaxJsonBytes = 1024 * 1024;
constexpr std::size_t kMaxAudioBytes = 30ULL * 60 * 16000 * sizeof(float);
constexpr const char* kOpenVinoCacheVersion = "openvino-2026.3";

class Qwen3AsrNpu {
  public:
    Qwen3AsrNpu(const std::filesystem::path& path, const std::filesystem::path& cache)
        : model_dir_(path), feature_extractor_(path / "preprocessor_config.json"),
          tokenizer_(path, ov::AnyMap{{"CACHE_DIR", (cache / "tokenizer").string()}}) {
        ov::Core core;

        auto encoder_model = core.read_model(path / "openvino_encoder_model.xml");
        encoder_model->reshape({{"input_features", ov::PartialShape{1, 128, 100}}});
        encoder_ = core.compile_model(
            encoder_model, "NPU",
            ov::AnyMap{{"CACHE_DIR", (cache / "encoder").string()},
                       {"CACHE_MODE", "OPTIMIZE_SIZE"}}).create_infer_request();

        embeddings_ = core.compile_model(
            path / "openvino_embeddings_model.xml", "CPU",
            ov::AnyMap{{"CACHE_DIR", (cache / "embeddings").string()}}).create_infer_request();

        auto decoder_model = core.read_model(path / "openvino_decoder_model.xml");
        decoder_model->reshape({{"inputs_embeds", ov::PartialShape{1, -1, 2048}},
                                {"attention_mask", ov::PartialShape{1, -1}},
                                {"position_ids", ov::PartialShape{1, -1}}});
        decoder_ = core.compile_model(
            decoder_model, "NPU",
            ov::AnyMap{{"NPU_USE_NPUW", "YES"},
                       {"NPUW_LLM", "YES"},
                       {"NPUW_LLM_BATCH_DIM", 0},
                       {"NPUW_LLM_SEQ_LEN_DIM", 2},
                       {"NPUW_LLM_MAX_PROMPT_LEN", 1024},
                       {"NPUW_LLM_MIN_RESPONSE_LEN", 256},
                       {"NPUW_LLM_PREFILL_HINT", "STATIC"},
                       {"CACHE_DIR", (cache / "decoder").string()},
                       {"CACHE_MODE", "OPTIMIZE_SIZE"}}).create_infer_request();
    }

    std::string transcribe(const std::vector<float>& audio, const std::string& language) {
        constexpr std::size_t chunk_frames = 100;
        constexpr std::size_t encoder_tokens_per_chunk = 13;
        constexpr std::size_t hidden_size = 2048;
        constexpr std::size_t vocab_size = 151936;
        constexpr std::size_t max_new_tokens = 256;
        constexpr int64_t audio_token_id = 151676;
        constexpr int64_t eos_token_id = 151643;
        constexpr int64_t im_end_token_id = 151645;

        std::cerr << "Qwen NPU: extracting features\n";
        const auto features = feature_extractor_.extract(audio, false);
        if (features.n_frames == 0) return {};
        const std::size_t chunk_count = (features.n_frames + chunk_frames - 1) / chunk_frames;
        const std::size_t remainder = features.n_frames % chunk_frames;
        const std::size_t final_tokens = remainder == 0
            ? encoder_tokens_per_chunk
            : (remainder * encoder_tokens_per_chunk + chunk_frames - 1) / chunk_frames;
        const std::size_t audio_tokens = (chunk_count - 1) * encoder_tokens_per_chunk + final_tokens;
        if (audio_tokens + 32 > 1024)
            throw std::runtime_error("Qwen audio is too long for the 30-second NPU prompt window");

        std::cerr << "Qwen NPU: encoding " << features.n_frames << " frames into " << audio_tokens << " tokens\n";
        ov::Tensor hidden(ov::element::f32, {1, audio_tokens, hidden_size});
        float* hidden_out = hidden.data<float>();
        for (std::size_t chunk = 0; chunk < chunk_count; ++chunk) {
            ov::Tensor input(ov::element::f32, {1, features.feature_size, chunk_frames});
            std::fill_n(input.data<float>(), features.feature_size * chunk_frames, 0.0f);
            const std::size_t frame_start = chunk * chunk_frames;
            const std::size_t frames = std::min(chunk_frames, features.n_frames - frame_start);
            for (std::size_t mel = 0; mel < features.feature_size; ++mel) {
                std::memcpy(input.data<float>() + mel * chunk_frames,
                            features.data.data() + mel * features.n_frames + frame_start,
                            frames * sizeof(float));
            }
            encoder_.set_tensor("input_features", input);
            ov::Tensor encoded_host(ov::element::f32, {1, encoder_tokens_per_chunk, hidden_size});
            encoder_.set_tensor("last_hidden_state", encoded_host);
            encoder_.infer();
            const ov::Tensor encoded = encoder_.get_tensor("last_hidden_state");
            const std::size_t keep = chunk + 1 == chunk_count ? final_tokens : encoder_tokens_per_chunk;
            std::memcpy(hidden_out + chunk * encoder_tokens_per_chunk * hidden_size,
                        encoded.data<const float>(), keep * hidden_size * sizeof(float));
        }

        std::string prompt = "<|im_start|>system\n<|im_end|>\n<|im_start|>user\n"
                             "<|audio_start|>";
        for (std::size_t i = 0; i < audio_tokens; ++i) prompt += "<|audio_pad|>";
        prompt += "<|audio_end|><|im_end|>\n<|im_start|>assistant\n";
        if (!language.empty()) prompt += "language " + language_name(language) + "<asr_text>";
        std::cerr << "Qwen NPU: tokenizing prompt\n";
        auto tokenized = tokenizer_.encode(prompt);
        ov::Tensor input_ids = tokenized.input_ids;

        std::cerr << "Qwen NPU: building prompt embeddings\n";
        embeddings_.set_tensor("input_ids", input_ids);
        embeddings_.set_tensor("encoder_hidden_states", hidden);
        embeddings_.infer();
        const std::size_t prompt_length = input_ids.get_shape().at(1);
        const ov::Tensor prompt_embeddings = embeddings_.get_output_tensor();
        ov::Tensor current_embeddings(ov::element::f32, {1, prompt_length, hidden_size});
        std::memcpy(current_embeddings.data<float>(), prompt_embeddings.data<const float>(),
                    current_embeddings.get_byte_size());

        std::cerr << "Qwen NPU: decoding " << prompt_length << " prompt tokens\n";
        decoder_.reset_state();
        std::vector<int64_t> generated;
        for (std::size_t step = 0; step < max_new_tokens; ++step) {
            ov::Tensor attention(ov::element::i64, {1, prompt_length + step});
            std::fill_n(attention.data<int64_t>(), prompt_length + step, 1);
            ov::Tensor positions(ov::element::i64, {1, step == 0 ? prompt_length : 1});
            if (step == 0) {
                for (std::size_t i = 0; i < prompt_length; ++i) positions.data<int64_t>()[i] = i;
            } else {
                positions.data<int64_t>()[0] = prompt_length + step - 1;
            }
            ov::Tensor beam(ov::element::i32, {1});
            beam.data<int32_t>()[0] = 0;
            decoder_.set_tensor("inputs_embeds", current_embeddings);
            decoder_.set_tensor("attention_mask", attention);
            decoder_.set_tensor("position_ids", positions);
            decoder_.set_tensor("beam_idx", beam);
            decoder_.infer();
            const ov::Tensor logits = decoder_.get_tensor("logits");
            const auto shape = logits.get_shape();
            const std::size_t vocab = shape.back();
            const float* scores = logits.data<const float>() + (shape.at(1) - 1) * vocab;
            int64_t token = static_cast<int64_t>(std::max_element(scores, scores + vocab) - scores);
            // NPU INT8 can produce a near-tie that repeats a word around punctuation.
            // Suppress only that exact A-punctuation-A artifact; do not apply a broad
            // repetition penalty that could alter legitimate dictated repetitions.
            if (generated.size() >= 2 && token == generated[generated.size() - 2] &&
                is_punctuation(generated.back())) {
                const float* best = nullptr;
                for (std::size_t i = 0; i < vocab; ++i) {
                    if (static_cast<int64_t>(i) == token) continue;
                    if (!best || scores[i] > *best) best = &scores[i];
                }
                token = static_cast<int64_t>(best - scores);
            }
            if (token == eos_token_id || token == im_end_token_id) break;
            generated.push_back(token);

            ov::Tensor next_ids(ov::element::i64, {1, 1});
            next_ids.data<int64_t>()[0] = token;
            embeddings_.set_tensor("input_ids", next_ids);
            embeddings_.set_tensor("encoder_hidden_states", hidden);
            embeddings_.infer();
            const ov::Tensor next_embeddings = embeddings_.get_output_tensor();
            current_embeddings = ov::Tensor(ov::element::f32, {1, 1, hidden_size});
            std::memcpy(current_embeddings.data<float>(), next_embeddings.data<const float>(),
                        current_embeddings.get_byte_size());
        }
        return tokenizer_.decode(generated);
    }

  private:
    static bool is_punctuation(int64_t token) {
        return token == 11 || token == 13 || token == 25 || token == 26;
    }

    static std::string language_name(const std::string& language) {
        std::string code = language;
        if (code.size() > 4 && code.starts_with("<|") && code.ends_with("|>"))
            code = code.substr(2, code.size() - 4);
        if (code == "en") return "English";
        if (code == "ur") return "Urdu";
        if (code == "es") return "Spanish";
        if (code == "de") return "German";
        if (code == "fr") return "French";
        if (code == "it") return "Italian";
        if (code == "pt") return "Portuguese";
        if (code == "zh") return "Chinese";
        if (code == "ja") return "Japanese";
        if (code == "ko") return "Korean";
        return code;
    }

    std::filesystem::path model_dir_;
    ov::genai::WhisperFeatureExtractor feature_extractor_;
    ov::genai::Tokenizer tokenizer_;
    ov::InferRequest encoder_;
    ov::InferRequest embeddings_;
    ov::InferRequest decoder_;
};

std::filesystem::path model_cache_dir(const std::filesystem::path& model_dir) {
    const auto cache = model_dir / ".handy-npu-cache" / kOpenVinoCacheVersion;
    std::filesystem::create_directories(cache);
    return cache;
}

void release_unused_heap() {
#ifdef __GLIBC__
    ::malloc_trim(0);
#endif
}

struct WorkerState {
    std::mutex mutex;
    std::unique_ptr<ov::genai::ASRPipeline> pipeline;
    std::shared_ptr<eddy::parakeet::OpenVINOParakeet> parakeet;
    std::unique_ptr<Qwen3AsrNpu> qwen;
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
                        {"loaded", state.pipeline != nullptr || state.parakeet != nullptr || state.qwen != nullptr},
                        {"model_path", state.model_path},
                        {"device", (state.pipeline || state.parakeet || state.qwen) ? "NPU" : ""},
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
            const bool is_parakeet =
                std::filesystem::is_regular_file(std::filesystem::path(path) / "parakeet_encoder.xml") &&
                std::filesystem::is_regular_file(std::filesystem::path(path) / "parakeet_decoder.xml") &&
                std::filesystem::is_regular_file(std::filesystem::path(path) / "parakeet_joint.xml");
            const bool is_qwen = std::filesystem::is_regular_file(
                std::filesystem::path(path) / "handy_qwen_npu.json");

            std::unique_ptr<ov::genai::ASRPipeline> pipeline;
            std::shared_ptr<eddy::parakeet::OpenVINOParakeet> parakeet;
            std::unique_ptr<Qwen3AsrNpu> qwen;
            if (is_parakeet) {
                eddy::OpenVINOOptions backend_options;
                backend_options.device = "NPU";
                backend_options.cache_dir = model_cache_dir(path).string();
                auto backend = std::make_shared<eddy::OpenVINOBackend>(backend_options);
                const std::filesystem::path model_dir(path);
                eddy::parakeet::ModelPaths paths{
                    .preprocessor = {.path = (model_dir / "parakeet_melspectogram.xml").string()},
                    .encoder = {.path = (model_dir / "parakeet_encoder.xml").string()},
                    .decoder = {.path = (model_dir / "parakeet_decoder.xml").string()},
                    .joint = {.path = (model_dir / "parakeet_joint.xml").string()},
                    .tokenizer_json = (model_dir / "parakeet_v3_vocab.json").string()};
                eddy::parakeet::RuntimeConfig config{
                    .device = "NPU", .blank_token_id = 8192, .duration_bins = {0, 1, 2, 3, 4}};
                parakeet = eddy::parakeet::make_openvino_parakeet(backend, paths, config);
                // Fail the load before reporting success if any NPU graph cannot compile.
                parakeet->warmup();
            } else if (is_qwen) {
                qwen = std::make_unique<Qwen3AsrNpu>(path, model_cache_dir(path));
            } else {
                const auto cache_dir = model_cache_dir(path);
                pipeline = std::make_unique<ov::genai::ASRPipeline>(
                    path, "NPU",
                    ov::AnyMap{{"CACHE_DIR", cache_dir.string()},
                               {"CACHE_MODE", "OPTIMIZE_SIZE"}});
            }
            std::lock_guard<std::mutex> lock(state.mutex);
            state.pipeline = std::move(pipeline);
            state.parakeet = std::move(parakeet);
            state.qwen = std::move(qwen);
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
        state.parakeet.reset();
        state.qwen.reset();
        state.model_path.clear();
        release_unused_heap();
        return {{"protocol_version", kProtocolVersion}, {"ok", true}, {"unloaded", true}};
    }

    if (command == "transcribe") {
        if (payload.empty() || payload.size() % sizeof(float) != 0)
            return error_response("invalid_audio", "payload must contain little-endian f32 samples");
        std::lock_guard<std::mutex> lock(state.mutex);
        if (!state.pipeline && !state.parakeet && !state.qwen)
            return error_response("model_not_loaded", "load a model first");
        std::vector<float> audio(payload.size() / sizeof(float));
        std::memcpy(audio.data(), payload.data(), payload.size());
        const std::string task = request.value("task", "transcribe");
        if (task != "transcribe" && task != "translate")
            return error_response("invalid_task", "task must be transcribe or translate");
        const auto started = std::chrono::steady_clock::now();
        std::string text;
        json chunks = json::array();
        if (state.parakeet) {
            if (task == "translate")
                return error_response("unsupported_task", "Parakeet does not translate to English");
            eddy::parakeet::AudioSegment segment;
            segment.sample_rate = 16000;
            segment.pcm = std::move(audio);
            text = state.parakeet->infer(segment, {}).text;
        } else if (state.qwen) {
            if (task == "translate")
                return error_response("unsupported_task", "Qwen3-ASR NPU does not translate to English");
            text = state.qwen->transcribe(audio, request.value("language", ""));
        } else {
            auto config = state.pipeline->get_generation_config();
            const std::string language = request.value("language", "");
            if (!language.empty()) config.language = language;
            config.task = task;
            config.return_timestamps = true;
            config.word_timestamps = false;
            auto result = state.pipeline->generate(audio, config);
            text = result.texts.empty() ? "" : result.texts.front();
            if (result.chunks && !result.chunks->empty()) {
                for (const auto& chunk : result.chunks->front())
                    chunks.push_back(json::object({{"start", chunk.start_ts},
                                                   {"end", chunk.end_ts},
                                                   {"text", chunk.text}}));
            }
        }
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - started).count();
        return {{"protocol_version", kProtocolVersion}, {"ok", true},
                {"transcription", {{"text", text},
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
    // The worker can hold several gigabytes of compiled model state. Ensure it
    // cannot survive an application crash or forced termination and become a
    // hidden orphan. Re-check the parent after prctl to close the small race in
    // which Handy exits between fork/exec and installing the death signal.
    const pid_t parent_pid = ::getppid();
    if (::prctl(PR_SET_PDEATHSIG, SIGTERM) != 0) {
        std::cerr << "cannot configure parent-death cleanup: " << std::strerror(errno) << "\n";
        return 1;
    }
    if (parent_pid == 1 || ::getppid() != parent_pid) return 1;

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
