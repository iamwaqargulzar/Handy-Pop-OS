# OpenVINO NPU Integration for Handy Pop!_OS

Status: OpenVINO 2026.3 ASRPipeline selected; remaining gates pending

Last updated: 2026-08-13

Primary target: Pop!_OS 24.04 x86_64 on Intel Core Ultra systems

## Objective

Add optional Intel NPU transcription to the normal Handy model workflow without
shipping model weights, Python, an HTTP service, or a second user-facing
package. One Handy Pop!_OS Debian package will contain the application, a
private native worker, and the minimal pinned OpenVINO runtime. Compatible
OpenVINO models appear in the Models page only when the NPU is usable, and are
downloaded on demand into Handy's existing user model directory.

The initial accuracy-first target is full Whisper Large V3 INT8. The primary
user already accepted full Large V3's accent handling in the earlier Windows
implementation; Gate 1 therefore validates Linux execution and performance,
not the already-established model quality.

## Retired Windows experiment

The Windows Python/HTTP implementation is deliberately not reused. Do not
restore embedded Python, a localhost server, WAV-over-HTTP, remote backend
settings, server ports, or server lifecycle controls.

## User experience

1. Handy probes the bundled private worker at startup.
2. A successful NPU probe reveals a clearly labelled OpenVINO model group.
3. The existing model controls download, select, switch, and delete its models.
4. Weights remain outside the `.deb` and are downloaded only by user action.
5. Unsupported systems hide this group and preserve all conventional engines.
6. Failures identify the actual execution device and never masquerade as NPU.

Detection has four states:

| State | Meaning | UI behavior |
| --- | --- | --- |
| `available` | Private runtime enumerated and exercised `NPU` | Show OpenVINO models |
| `runtime_error` | Hardware is plausible but runtime loading failed | Hide models; retain diagnostic |
| `incompatible` | No supported Intel NPU is available | Hide OpenVINO models |
| `temporarily_failed` | A previously usable worker/device failed | Preserve downloads; disable selection; offer retry |

PCI and `/dev/accel` are hints only. OpenVINO enumeration is authoritative.

## Architecture

```text
Handy recording/VAD pipeline
          |
          | versioned Unix socket + framed JSON + raw 16 kHz mono f32 audio
          v
/usr/lib/Handy/handy-openvino-npu
          |
          v
OpenVINO GenAI C++ ASRPipeline(device = "NPU")
```

Gate 1 proved that OpenVINO 2026.3's C++ `ASRPipeline` is the correct native
boundary. The Rust 0.11 `WhisperPipeline` wrapper was unstable when language
was forced and did not expose all required constructor properties. The worker
is a separate process so a native crash cannot kill Handy, strand recording
state, or prevent crash-safe PipeWire volume restoration.

### Language behavior

The worker must pass Handy's currently selected language explicitly to
`ASRPipeline` for each transcription. Changing the language in Handy must take
effect without downloading another copy of the model. Automatic language
detection must be used only when the user explicitly selects an automatic
mode.

Forced-language decoding controls the language Whisper uses to interpret and
generate text; it does not guarantee that speech in a different language will
be ignored. Mixed-language recordings must be tested separately. Handy must
not advertise other-language suppression unless that behavior is implemented
and verified independently of the pipeline's language setting.

### Removability requirement

The NPU route is optional even though it ships in the same package. Its source,
worker executable, private runtime, model-catalogue entries, backend adapter,
and tests must remain identifiable and separable. Existing GGML/GGUF and ONNX
engines must not depend on OpenVINO types or libraries. Removing the NPU route
must consist of deleting the worker/runtime packaging entries, OpenVINO model
metadata, and its adapter—not rewriting Handy's recording, overlay, shortcut,
clipboard, history, or volume-reduction pipelines.

No OpenVINO library may be loaded into the main Handy process. If the feature
is rejected later, the conventional application must build and run after the
isolated NPU files and feature registration are removed.

### Worker protocol v1

- `probe`: runtime/device/access facts and the actual enumerated devices;
- `load_model`: validate and load one OpenVINO model on `NPU`;
- `transcribe`: consume normalized f32 audio and return text, segment
  timestamps, timings, and actual device;
- `cancel`: cancel when supported, otherwise let Handy restart the worker;
- `unload_model`: release pipeline/device memory;
- `status`: readiness, active model/device, busy state, and last error; and
- `shutdown`: clean termination.

The socket lives under the user's runtime directory with owner-only
permissions. Every frame has explicit length and protocol version. Invalid,
oversized, out-of-order, or concurrent requests fail deterministically.

Keep one pipeline warm while its OpenVINO model is active. Gate 1 found a cold
compile of roughly 144-163 seconds and approximately 6.9 GiB peak memory, while
warm 11-second transcription took about 1.36 seconds. A compiled cache attempt
created about 1.37 GiB but did not finish within ten minutes. Cache creation is
therefore not part of the interactive load path; it needs a later bounded
background experiment.

## Model catalogue

OpenVINO entries require explicit metadata so GGML/GGUF and ONNX loaders never
attempt to open them:

```text
format: openvino_ir
backend: openvino_genai
preferred_device: intel_npu
precision: int8
architecture: whisper-large-v3
```

Initial entry: `OpenVINO/whisper-large-v3-int8-ov`. INT4, Turbo, and distilled
variants remain unapproved until their accuracy tradeoffs are measured. TDT is
a Parakeet decoder architecture, not a mode for Distil-Whisper.

Downloads must reuse Handy's progress, checksum, activation, hotkey, CLI, and
deletion behavior. Deletion invalidates only that model's cache.

## Single package

```text
/usr/bin/handy
/usr/lib/Handy/handy-openvino-npu
/usr/lib/Handy/openvino/...
/usr/share/doc/handy/...licences and notices...
```

Ship the pinned runtime closure only: no model weights, Python, headers, CMake
metadata, compilers, samples, benchmarks, or debug symbols. Gate 1 required a
private OpenVINO GenAI 2026.3 runtime, Intel NPU user-mode driver 1.35.0, Level
Zero loader 1.28.2, the distribution `intel_vpu` kernel driver, and `render`
group access. The tested compatibility setting was
`DISABLE_OPENVINO_GENAI_NPU_L0=1`.

The current package is approximately 47.5 MB compressed and 127 MB installed.
The responsible enhanced target remains about 80-105 MB compressed and
250-350 MB installed, excluding the roughly 1.57 GB on-demand model.

## Fallback policy

- Hide the catalogue when initial probing fails.
- If a selected NPU model fails, report it and offer a downloaded conventional
  model.
- Optional fallback may use the user's prior local model, but logs/history must
  identify the engine actually used.
- Never download a fallback without user action.
- Never silently execute an OpenVINO entry on CPU/GPU while labelling it NPU.

## Gates

### Gate 1 — complete: standalone feasibility

The isolated harness proved private-runtime NPU enumeration, verified model
files, correct full Large V3 INT8 execution, stable forced English through
`ASRPipeline`, repeatable warm timing, and clean process restart. See
`experiments/openvino-gate1/GATE1_RESULTS.md`.

### Gate 2 — memory research complete; reliability work pending

Test framing and malformed requests, cancellation/restart, corrupt models,
driver/runtime failure, suspend/resume, repeated load/unload, concurrent
trigger rejection, and isolation from Handy state.

Gate 2 measures process RSS before load, compilation peak, warm idle after
compilation, after each of at least five transcriptions, after unload, and
after worker exit. Previous platform observations are not requirements or
acceptance thresholds. Memory must not grow monotonically across
transcriptions.

Gate 2 measured a best correct warm RSS of approximately 4.50 GiB after testing
allocator trimming, both OpenVINO speech APIs, supported NPU memory properties,
native Level Zero, OpenVINO 2026.2/2026.3, and the historical stateless export
path. Repeated inference was stable without measurable per-transcription
growth. These measurements inform the product tradeoff but do not reject the
integration. Detailed evidence is in `experiments/openvino-gate2/GATE2_RESULTS.md`.

### Gate 3: model UI integration

Verify conditional visibility, download/checksum flow, selection, model
hotkeys, CLI switching, deletion/cache cleanup, localization, and honest device
reporting.

### Gate 4: clean-system package

Build one `.deb`, inspect runtime closure/licences, and test clean Pop!_OS
systems with and without a supported NPU. Conventional transcription must keep
working without NPU support.

## Non-goals for the first release

- Bundled speech-model weights;
- the retired Windows Python/HTTP route;
- system-wide OpenVINO development tools;
- containers or network transcription;
- unverified streaming claims; or
- displaying untested OpenVINO conversions.
