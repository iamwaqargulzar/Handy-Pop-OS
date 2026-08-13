# OpenVINO NPU Integration for Handy Pop!\_OS

Status: integrated on isolated branch; package-level NPU transcription verified

Last updated: 2026-08-13

Primary target: Pop!\_OS 24.04 x86_64 on Intel Core Ultra systems

## Objective

Add optional Intel NPU transcription to the normal Handy model workflow without
shipping model weights, Python, an HTTP service, or a second user-facing
package. One Handy Pop!\_OS Debian package will contain the application, a
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

| State                | Meaning                                          | UI behavior                                        |
| -------------------- | ------------------------------------------------ | -------------------------------------------------- |
| `available`          | Private runtime enumerated and exercised `NPU`   | Show OpenVINO models                               |
| `runtime_error`      | Hardware is plausible but runtime loading failed | Hide models; retain diagnostic                     |
| `incompatible`       | No supported Intel NPU is available              | Hide OpenVINO models                               |
| `temporarily_failed` | A previously usable worker/device failed         | Preserve downloads; disable selection; offer retry |

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
or architecture-specific NPU pipeline (Parakeet / Qwen3-ASR)
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
- cancellation: Handy may terminate and restart the isolated worker when the
  runtime cannot cancel an active request;
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

The catalogue contains 39 pinned official OpenVINO Whisper repositories across
Turbo, Large V2/V3, Distil Large V2/V3, Medium, Small, Base, and Tiny; INT4,
INT8, and FP16; and multilingual/English variants where published. Download
manifests are resolved at immutable revisions, sizes are enforced, and LFS
files are SHA-256 verified. A fortieth entry adds the independently verified
Parakeet TDT V3 OpenVINO snapshot. It uses the minimal pinned Apache-2.0 Eddy
decoder in the isolated worker and is never routed through the Whisper
pipeline. Eddy's upstream silent NPU-to-CPU fallback is disabled: failure to
compile any encoder, decoder, or joint graph on NPU fails model loading. Entries
41 and 42 add Qwen3-ASR 1.7B INT8 and INT4 in Handy's NPU-native format.

Qwen's unmodified official ASR export cannot run directly on the 2026.3 NPU
plugin because its encoder and KV-cache shapes are dynamic. Handy keeps the
speech encoder, maps the speech-trained decoder weights into the equivalent
standard Qwen3 causal architecture, and exports a conventional stateful
`inputs_embeds`, `attention_mask`, and `position_ids` interface. The worker
chunks mel features into fixed 100-frame encoder calls on NPU, merges audio and
text embeddings on CPU, then runs the bounded decoder through NPUW on NPU. The
1,024-token prompt bucket covers the model's approximately 30-second ASR window.
INT8 is the quality default; INT4 is a smaller opt-in alternative.

Downloads must reuse Handy's progress, checksum, activation, hotkey, CLI, and
deletion behavior. Deletion invalidates only that model's cache.

Compiled blobs are stored below the downloaded model in a runtime-versioned
`.handy-npu-cache` directory. This applies uniformly to Whisper, Parakeet, and
future verified OpenVINO architectures; GGML, Vulkan, and ONNX models are
unaffected. Model deletion recursively removes the cache. Whisper Large V3
INT8 measured 169.968 seconds for its first compilation and 7.629 seconds for a
cache hit, while its weightless compiled cache used 2.9 GB of additional disk.
Qwen uses the same location for separate encoder, embedding, tokenizer, and
bounded-decoder caches.

The worker sets Linux `PR_SET_PDEATHSIG` and verifies that its parent did not
change while supervision was installed. This makes cleanup kernel-enforced
when Handy crashes or is force-killed, while the normal Drop path still asks
the worker to shut down gracefully and reaps the child.

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
Zero loader 1.32.0, the distribution `intel_vpu` kernel driver, and `render`
group access. The tested compatibility setting was
`DISABLE_OPENVINO_GENAI_NPU_L0=1`.

Release builds set four explicit native inputs: `HANDY_OPENVINO_GENAI_ROOT`
(the pinned 2026.3 SDK/runtime), `HANDY_OPENVINO_GENAI_SOURCE` (the matching
GenAI source tree used only to compile its Apache-2.0 Whisper feature extractor),
`HANDY_NPU_LEVEL_ZERO_LIB`, and `HANDY_LEVEL_ZERO_LOADER_LIB`. Headers and
source files are build inputs only; the Debian package receives only the worker
and verified runtime closure. Omitting the OpenVINO root keeps conventional
Handy builds possible and stages no NPU worker.

The verified package is 122.6 MiB compressed,
excluding all on-demand models (Qwen INT8 is 2.1 GiB; Qwen INT4 is 1.4 GiB).
This includes the CPU plug-in
needed by ASRPipeline initialization, but the model is compiled explicitly for
NPU and there is no silent CPU/GPU transcription fallback.

### Qwen model redistribution

The downloadable Qwen directory is generated from `Qwen/Qwen3-ASR-1.7B` by:

1. exporting the official stateful encoder and tokenizer;
2. loading the ASR text weights into a standard `Qwen3ForCausalLM` with an
   identical layer/weight mapping;
3. exporting a stateful inputs-embeds decoder;
4. producing symmetric INT8 per-channel and symmetric INT4 group-128 decoder
   variants; and
5. extracting a CPU prompt-embedding/audio-merge graph and writing
   `handy_qwen_npu.json`.

The source weights, generated OpenVINO models, and compiled caches are never
part of Git or the installer. Only the two model snapshots belong in the public
Hugging Face repositories referenced by the catalogue. After publishing, pin
each catalogue revision to its immutable Hugging Face commit rather than
leaving `main` in a release build.

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

### Gate 2 — memory and core worker reliability complete

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

### Gate 3 — core model integration complete; interactive UI QA pending

Implemented conditional catalogue visibility, pinned multi-file downloads and
checksums, selection, deletion, language/task forwarding, and honest backend
reporting. The extracted package's `--list-models` path exposed the NPU model
only after a successful packaged-worker probe. Interactive Settings UI,
hotkey, and CLI switching remain final installed-package QA items.

### Gate 4 — Debian payload test complete; clean-machine QA pending

One `.deb` was built and its extracted private runtime was inspected with no
unresolved dynamic dependencies. From that payload, NPU probe, cold model
load, two correct transcriptions, unload, shutdown, and socket cleanup passed.
The current Qwen-enabled artifact SHA-256 is
`b888179e34dc10d5ccf5d3ab3bd929006a29738015430aa9f7917886a37a63dd`.
The final Parakeet package payload additionally passed a cold NPU load in
27.196 seconds and a correct 11-second sample transcription in 220 ms. Its
compressed size increased by 102,812 bytes relative to the preserved
Whisper-only NPU package; no model weights are bundled.
Clean Pop!\_OS testing with and without a supported NPU remains pending;
conventional transcription must remain usable on both.

The Qwen INT8 pipeline additionally passed the complete worker protocol on the
local Core Ultra 9 288V: model load reported `actual_device: NPU`, the 11-second
JFK sample completed in 2.037 seconds with the expected full transcript, and
shutdown completed normally. INT4 compiles and runs on NPU but remains labelled
as an accuracy tradeoff rather than an equivalent-quality default.

The final installed-package regression additionally covers model switching and
all three downloadable NPU families. Worker sockets include a per-process
monotonic suffix so an old engine cannot unlink or shut down its replacement;
one reconnect/reload attempt handles an unexpectedly exited worker. Qwen uses
NPUW's supported shared-head configuration and trims the fixed-size embedding
graph output to the actual prompt/token length. The installed `.deb` produced
correct JFK transcripts with Whisper Large V3 INT8 (3.797 s), Qwen3-ASR 1.7B
INT8 (2.447 s), and Parakeet TDT 0.6B V3 (240 ms), all explicitly bound to the
OpenVINO NPU backend.

## Non-goals for the first release

- Bundled speech-model weights;
- the retired Windows Python/HTTP route;
- system-wide OpenVINO development tools;
- containers or network transcription;
- unverified streaming claims; or
- displaying untested OpenVINO conversions.
