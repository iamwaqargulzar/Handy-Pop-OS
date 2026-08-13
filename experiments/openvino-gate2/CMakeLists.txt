cmake_minimum_required(VERSION 3.20)
project(handy_openvino_npu LANGUAGES CXX)

find_package(OpenVINOGenAI REQUIRED)
find_package(Threads REQUIRED)

set(GATE1_RUNTIME "${CMAKE_CURRENT_SOURCE_DIR}/../openvino-gate1/runtime/openvino_genai_ubuntu24_2026.3.0.0_x86_64")
set(NLOHMANN_INCLUDE "${GATE1_RUNTIME}/samples/cpp/thirdparty/nlohmann_json/single_include")

add_executable(handy-openvino-npu src/main.cpp)
target_compile_features(handy-openvino-npu PRIVATE cxx_std_17)
target_include_directories(handy-openvino-npu PRIVATE "${NLOHMANN_INCLUDE}")
target_link_libraries(handy-openvino-npu PRIVATE openvino::genai Threads::Threads)
target_compile_options(handy-openvino-npu PRIVATE -Wall -Wextra -Wpedantic)
set_target_properties(handy-openvino-npu PROPERTIES INSTALL_RPATH_USE_LINK_PATH ON)

add_executable(legacy-whisper-memory src/legacy_whisper_memory.cpp)
target_compile_features(legacy-whisper-memory PRIVATE cxx_std_17)
target_link_libraries(legacy-whisper-memory PRIVATE openvino::genai)
target_compile_options(legacy-whisper-memory PRIVATE -Wall -Wextra -Wpedantic)
set_target_properties(legacy-whisper-memory PROPERTIES INSTALL_RPATH_USE_LINK_PATH ON)
