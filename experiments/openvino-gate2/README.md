# Handy OpenVINO Gate 2

This directory contains the isolated native-worker reliability prototype. It
does not integrate with or launch Handy.

The worker uses a versioned Unix-domain socket. Each request is one connection:

1. four-byte big-endian JSON length;
2. UTF-8 JSON request header; and
3. exactly `payload_bytes` raw bytes, when declared.

Responses use the same framing. Protocol version 1 supports `probe`, `status`,
`load_model`, `transcribe`, `unload_model`, and `shutdown`. `transcribe` expects
little-endian mono f32 samples at 16 kHz. The worker hard-selects `NPU`; it has
no CPU/GPU fallback.

The socket is created mode `0600`. Frames larger than 1 MiB of JSON or 30
minutes of audio are rejected. Only one state-changing/inference request may
run at once; another receives `busy` rather than waiting silently.

Build against the experiment-local OpenVINO archive:

```bash
source ../openvino-gate1/runtime/openvino_genai_ubuntu24_2026.3.0.0_x86_64/setupvars.sh
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --parallel
```

Run with the private NPU user-mode libraries ahead of the runtime libraries:

```bash
export LD_LIBRARY_PATH="../openvino-gate1/runtime/npu-user-mode/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH"
export DISABLE_OPENVINO_GENAI_NPU_L0=1
build/handy-openvino-npu "$XDG_RUNTIME_DIR/handy-openvino-gate2.sock"
```

Gate 2 artifacts, model files, audio, reports, and build products remain local
and must not enter the release package.
