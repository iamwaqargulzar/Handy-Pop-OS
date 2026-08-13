# Gate 2 Results: Worker Reliability and Memory

Date: 2026-08-13

Status: memory optimization exhausted; full Large V3 configuration rejected

## Isolation

The prototype remains entirely under `experiments/openvino-gate2` on the
separate `codex/openvino-gate1` worktree. No production Handy source, installed
package, settings, model catalogue, or user model directory was changed.

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

The user-defined acceptance target is approximately 2.5 GiB sustained warm
RSS. Measurements came directly from `/proc/<worker>/status`, not a peak-only
summary.

| State | RSS / high-water result |
| --- | ---: |
| Worker before model load | about 41 MiB RSS |
| First untrimmed compilation peak | 7,268,324 KiB HWM (6.93 GiB) |
| First untrimmed warm idle | about 5.64 GiB RSS |
| First unload | about 471 MiB RSS |
| Trimmed compilation time | 156.148 seconds |
| Trimmed compilation peak | 6,909,728 KiB HWM (6.59 GiB) |
| Trimmed warm idle | about 4.51 GiB RSS |
| Trimmed unload | 459,704 KiB RSS (about 449 MiB) |
| After worker exit | zero; process and socket absent |

Calling glibc `malloc_trim(0)` after load reduced the retained warm footprint
by roughly 1.1 GiB but did not bring it within budget. The remaining memory is
not merely all reclaimable compiler heap under this runtime configuration.

## Repeated transcription

Five consecutive runs transcribed the same approximately 11-second JFK clip
correctly with forced English and segment timestamps:

| Run | Inference | RSS afterward |
| --- | ---: | ---: |
| 1 | 2,357 ms | 4,731,260 KiB |
| 2 | 1,884 ms | 4,731,348 KiB |
| 3 | 1,730 ms | 4,731,348 KiB |
| 4 | 1,742 ms | 4,731,360 KiB |
| 5 | 1,632 ms | 4,731,360 KiB |

RSS changed by only 100 KiB across the five completed runs. There is no sign
of per-transcription growth in this test. The NPU remained faster than real
time and honestly reported `actual_device: NPU`.

## Current decision

Do **not** integrate this configuration into production Handy. It is stable and
does not leak across repeated transcriptions, but approximately 4.51 GiB warm
RSS materially exceeds the accepted 2.5 GiB budget.

Gate 2 may continue only with bounded memory-reduction experiments that retain
full Whisper Large V3 accuracy. The threshold must not be silently relaxed.
If no reasonable runtime/driver/configuration change reaches the target, apply
the documented complete-removal path and keep conventional Handy unchanged.

## Memory-reduction experiments

The following bounded experiments preserved full Whisper Large V3 and real NPU
execution unless noted otherwise:

| Experiment | Result |
| --- | --- |
| OpenVINO 2026.3 `ASRPipeline`, glibc trim | about 4.51 GiB warm RSS |
| OpenVINO 2026.3 legacy `WhisperPipeline` | about 4.50 GiB warm RSS; stable forced English |
| Bypass UMD cache + defer weight load + idle pruning enabled | about 4.52 GiB; slower inference |
| Official October 2024 stateless INT8 export on 2026.3 | rejected by OpenVINO state conversion |
| Same stateless export on 2025.4 | rejected by the same conversion |
| Same stateless export on its 2024.6 runtime generation | repository lacks the required `decoder_with_past` graph |
| OpenVINO 2026.2 + Intel NPU driver 1.35 + native Level Zero | about 4.58 GiB warm RSS; 7.31 GiB peak |

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

The lowest correct sustained Linux result was approximately 4.50 GiB, about
2 GiB over the accepted 2.5 GiB budget. Repeated runs do not leak and unload
returns the worker to roughly 449 MiB, but loading again costs approximately
2.5 minutes. An unload-after-every-transcription workaround is therefore not a
usable product design.

The full Large V3 OpenVINO route is rejected for production in its current
form. Do not merge the worker or package OpenVINO into Handy. A future attempt
requires a materially different Intel runtime/driver or a different model that
the user explicitly accepts; it must start from the same 2.5 GiB memory gate.

## Reliability tests not pursued after memory rejection

- malformed, oversized, truncated, and wrong-version protocol frames;
- concurrent request rejection while load/inference is active;
- missing and corrupt model files;
- worker termination during load and inference;
- repeated load/unload cycles;
- runtime/driver failure classification; and
- suspend/resume recovery.
