# Handy OpenVINO Gate 1

This is a standalone feasibility harness. It is intentionally disconnected
from Handy's Tauri application, model catalogue, settings, and installed
package.

## Safety boundaries

- It never modifies or launches the installed Handy application.
- It never downloads a model automatically.
- It never silently falls back from the requested device.
- It accepts only explicit model and audio paths.
- Results are emitted as JSON for reproducible comparisons.

The optional download script pins the approved Hugging Face repository to
commit `a888a75cc8b494a8a45400fd85f6bfa379ba3955` and verifies the SHA-256 values
published for all large binary blobs:

```bash
bash scripts/download-large-v3-int8.sh
```

## Commands

```bash
cargo run --release -- probe --output reports/probe.json

cargo run --release -- benchmark \
  --device NPU \
  --model /path/to/whisper-large-v3-int8-ov \
  --wav /path/to/mono-16khz.wav \
  --runs 3 \
  --output reports/large-v3-int8-npu.json
```

The benchmark input must be mono 16 kHz WAV audio. This matches the normalized
audio that Handy will eventually send to a native worker and keeps resampling
outside the measured inference path.

Language arguments such as `en` are normalized to Whisper control tokens such
as `<|en|>`. Pass `--language auto` to use model language detection.

The Rust harness exercises the legacy OpenVINO GenAI Whisper C API and remains
useful for detection and diagnostic benchmarking. Gate 1 found that production
should instead use OpenVINO 2026.3's native C++ `ASRPipeline`: it supports
stable forced-English transcription on this model, while automatic language
detection in the legacy wrapper drifted on repeated longer input. Keep one
pipeline alive, enable segment timestamps, and disable word timestamps.

For OpenVINO 2026.3 on the tested Lunar Lake Linux stack, export
`DISABLE_OPENVINO_GENAI_NPU_L0=1` before loading Large V3. Without it, the
pipeline compiled but failed during the first generation. This environment
variable is an Intel-documented compatibility path and must be re-evaluated
when the NPU user-mode driver changes.

## Required runtime

Build with Rust stable. Runtime linking is enabled for `openvino` and
`openvino-genai`, so compilation does not require a system-wide OpenVINO
development installation. Execution does require a mutually compatible,
pinned OpenVINO Runtime, OpenVINO GenAI, Tokenizers, and NPU plugin.

For Gate 1 those libraries and the model belong under this experiment's local
untracked directories. They must not be copied into Handy's package or system
locations.
