#include <malloc.h>

#include <chrono>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <thread>
#include <vector>

#include <openvino/genai/whisper_pipeline.hpp>

namespace {
void print_memory(const std::string& state) {
    std::ifstream status("/proc/self/status");
    std::string line;
    std::cout << "memory_state=" << state;
    while (std::getline(status, line)) {
        if (line.rfind("VmRSS:", 0) == 0 || line.rfind("VmHWM:", 0) == 0 ||
            line.rfind("RssAnon:", 0) == 0 || line.rfind("RssFile:", 0) == 0) {
            std::cout << " " << line;
        }
    }
    std::cout << std::endl;
}

std::vector<float> read_f32(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input) throw std::runtime_error("cannot open raw audio");
    std::vector<char> bytes((std::istreambuf_iterator<char>(input)), {});
    if (bytes.empty() || bytes.size() % sizeof(float) != 0)
        throw std::runtime_error("raw audio must contain f32 samples");
    std::vector<float> samples(bytes.size() / sizeof(float));
    std::memcpy(samples.data(), bytes.data(), bytes.size());
    return samples;
}
}  // namespace

int main(int argc, char** argv) try {
    if (argc != 3) {
        std::cerr << "usage: " << argv[0] << " <model-directory> <raw-f32-audio>\n";
        return 2;
    }
    print_memory("baseline");
    ov::genai::WhisperPipeline pipeline(argv[1], "NPU");
    ::malloc_trim(0);
    print_memory("loaded_trimmed");

    const auto audio = read_f32(argv[2]);
    auto config = pipeline.get_generation_config();
    config.language = "<|en|>";
    config.task = "transcribe";
    config.return_timestamps = true;
    for (int run = 1; run <= 5; ++run) {
        const auto started = std::chrono::steady_clock::now();
        auto result = pipeline.generate(audio, config);
        const auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - started).count();
        std::cout << "run=" << run << " elapsed_ms=" << elapsed
                  << " text=" << (result.texts.empty() ? "" : result.texts.front()) << std::endl;
        ::malloc_trim(0);
        print_memory("after_run_" + std::to_string(run));
    }
    std::this_thread::sleep_for(std::chrono::seconds(5));
    return 0;
} catch (const std::exception& error) {
    std::cerr << "error=" << error.what() << std::endl;
    return 1;
}
