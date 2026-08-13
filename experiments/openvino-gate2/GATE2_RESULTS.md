# Gate 2 Results: Worker Reliability and Memory

Date: 2026-08-13

Status: selected pipeline integrated and package-level transcription verified

## Isolation

The work remains on the separate `codex/openvino-gate1` worktree. Production
integration is now present on that isolated branch, but no installed package,
production branch, user settings, or user model directory was changed.

The OpenVINO libraries are loaded only by `handy-openvino-npu`, never by the
main Handy process. Complete removal and artifact cleanup are specified in
`REMOVAL.md` and `ARTIFACTS.md`.

## Implemented protocol boundary

- owner-only (`0600`) Unix-domain socket;
- protocol-versioned, length-prefixed JSON frames;
- bounded JSON and raw f32 audio payloads;
- explicit `NPU` execution with no CPU/GPU fallback;
- `probe`, `status`, `load_model`, `transcribe`, `unload_model`, and `shutdown`;
- one active model/inference mutation at a time; and
- native C++ OpenVINO 2026.3 `ASRPipeline` retained across requests.

The initial shutdown implementation exposed a blocked-`accept` lifecycle bug:
it acknowledged shutdown but remained alive. The listener now uses a bounded
non-blocking accept loop. Retest confirmed that shutdown terminates the process
and removes the socket.

## Memory measurements

Measurements came directly from `/proc/<worker>/status`, not a peak-only
summary. Previous platform observations are not requirements or acceptance
thresholds.

| State                            |         RSS / high-water result |
| -------------------------------- | ------------------------------: |
| Worker before model load         |                about 41 MiB RSS |
| First untrimmed compilation peak |    7,268,324 KiB HWM (6.93 GiB) |
| First untrimmed warm idle        |              about 5.64 GiB RSS |
| First unload                     |               about 471 MiB RSS |
| Trimmed compilation time         |                 156.148 seconds |
| Trimmed compilation peak         |    6,909,728 KiB HWM (6.59 GiB) |
| Trimmed warm idle                |              about 4.51 GiB RSS |
| Trimmed unload                   | 459,704 KiB RSS (about 449 MiB) |
| After worker exit                | zero; process and socket absent |

Calling glibc `malloc_trim(0)` after load reduced the retained warm footprint
by roughly 1.1 GiB. The remaining memory is not merely all reclaimable compiler
heap under this runtime configuration.

## Repeated transcription

Five consecutive runs transcribed the same approximately 11-second JFK clip
correctly with forced English and segment timestamps:

| Run | Inference | RSS afterward |
| --- | --------: | ------------: |
| 1   |  2,357 ms | 4,731,260 KiB |
| 2   |  1,884 ms | 4,731,348 KiB |
| 3   |  1,730 ms | 4,731,348 KiB |
| 4   |  1,742 ms | 4,731,360 KiB |
| 5   |  1,632 ms | 4,731,360 KiB |

RSS changed by only 100 KiB across the five completed runs. There is no sign
of per-transcription growth in this test. The NPU remained faster than real
time and honestly reported `actual_device: NPU`.

## Current decision

The prototype is stable, does not leak across repeated transcriptions, and uses
approximately 4.51 GiB warm RSS. This is a measured implementation tradeoff,
not a failed user requirement. OpenVINO 2026.3 C++ `ASRPipeline` is the selected
forward path because it is the current API and reliably honored the explicit
language setting in testing. Production integration was subsequently completed
on the isolated branch and verified from the generated Debian payload below.

## Integrated Debian payload verification

The first lean payload enumerated `NPU` but failed model initialization because
ASRPipeline also requires the OpenVINO CPU plug-in. The packaging closure was
corrected to include that private plug-in. This does not change model execution:
the worker constructs `ASRPipeline(model, "NPU")` and reports the actual device.

Final extracted-package test:

| Check                               | Result                                                             |
| ----------------------------------- | ------------------------------------------------------------------ |
| Runtime devices                     | `CPU`, `NPU`                                                       |
| Requested and reported model device | `NPU`                                                              |
| Cold load                           | 166.769 s                                                          |
| First 11-second JFK transcription   | correct, 2.355 s                                                   |
| Second warm transcription           | correct, 1.405 s                                                   |
| Unload and shutdown                 | passed; worker and socket absent                                   |
| Debian size                         | 123 MiB compressed; 352,174 KiB installed                          |
| Debian SHA-256                      | `c58d4b0dd71bd06aaf983ca95bd2eb222b47185fd9f10196d12e5e4f4115a356` |

## Memory-reduction experiments

The following bounded experiments preserved full Whisper Large V3 and real NPU
execution unless noted otherwise:

| Experiment                                                  | Result                                                  |
| ----------------------------------------------------------- | ------------------------------------------------------- |
| OpenVINO 2026.3 `ASRPipeline`, glibc trim                   | about 4.51 GiB warm RSS                                 |
| OpenVINO 2026.3 legacy `WhisperPipeline`                    | about 4.50 GiB warm RSS; stable forced English          |
| Bypass UMD cache + defer weight load + idle pruning enabled | about 4.52 GiB; slower inference                        |
| Official October 2024 stateless INT8 export on 2026.3       | rejected by OpenVINO state conversion                   |
| Same stateless export on 2025.4                             | rejected by the same conversion                         |
| Same stateless export on its 2024.6 runtime generation      | repository lacks the required `decoder_with_past` graph |
| OpenVINO 2026.2 + Intel NPU driver 1.35 + native Level Zero | about 4.58 GiB warm RSS; 7.31 GiB peak                  |

The 2026.2/native run is particularly important. Intel validates NPU driver
1.35 against OpenVINO 2026.2, native Level Zero completed correctly without
`DISABLE_OPENVINO_GENAI_NPU_L0`, all five transcripts were identical, and warm
inference was 1.20-1.65 seconds. Moving allocations back to the native path did
not approach the Windows-reported footprint.

The earlier Windows implementation used Python `WhisperPipeline` and an FP16
model. Linux tests show that the pipeline wrapper and allocation path do not
explain the retained host memory. Windows Task Manager's reported category,
different model export/runtime, and Windows NPU driver accounting therefore
cannot be treated as a Linux memory forecast.

### Final memory decision

The lowest correct sustained Linux result was approximately 4.50 GiB. Repeated
runs do not leak and unload returns the worker to roughly 449 MiB, but loading
again costs approximately 2.5 minutes. An unload-after-every-transcription
workaround is therefore not a usable product design.

The full Large V3 OpenVINO route remains technically viable with a substantial
measured memory and cold-load cost. Integration is kept on the isolated branch
until the remaining fault-injection and clean-machine checks are complete.

## Reliability tests still pending

- malformed, oversized, truncated, and wrong-version protocol frames;
- concurrent request rejection while load/inference is active;
- missing and corrupt model files;
- worker termination during load and inference;
- repeated load/unload cycles;
- runtime/driver failure classification; and
- suspend/resume recovery; and
- clean-machine conventional-engine regression testing with and without an NPU.
