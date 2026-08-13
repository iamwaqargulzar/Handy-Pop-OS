# Gate 1 Results: OpenVINO Whisper Large V3 INT8

Date: 2026-08-13  
Status: Gate 1 complete; proceed to isolated worker reliability

## Isolation

The experiment lives on branch `codex/openvino-gate1` in a separate Git
worktree. It does not modify or communicate with Handy's Tauri application,
installed package, settings, model catalogue, or model directory.

All downloaded runtime, model, audio, cache, and generated report files are
ignored experiment-local artifacts.

## Tested stack

- Pop!_OS 24.04, kernel `7.0.11-76070011-generic`
- Intel Core Ultra 9 288V / Lunar Lake NPU `8086:643e`
- kernel driver `intel_vpu`
- `/dev/accel/accel0`, group `render`
- OpenVINO GenAI archive `2026.3.0.0` for Ubuntu 24.04
- Intel NPU Level Zero user-mode driver `1.35.0`
- Level Zero loader `1.28.2`
- Intel Rust crates `openvino` and `openvino-genai` `0.11.0`
- model `OpenVINO/whisper-large-v3-int8-ov`, pinned to Hugging Face commit
  `a888a75cc8b494a8a45400fd85f6bfa379ba3955`

The four large model blobs were checked against their published SHA-256
digests. The complete local snapshot is approximately 1.57 GB.

## Detection findings

Hardware and `/dev/accel` presence alone did not make an NPU visible to
OpenVINO.

1. Before `render` permission, the device node existed but was inaccessible;
   OpenVINO reported only `CPU`.
2. After adding the user to `render`, the device node was accessible but
   OpenVINO still reported only `CPU`.
3. After privately loading `libze_loader.so.1` and
   `libze_intel_npu.so.1.35.0`, OpenVINO reported `CPU` and `NPU`.

No Intel NPU packages were installed system-wide. The user-mode libraries were
extracted locally. This supports the planned single-package design: Handy can
ship these runtime libraries privately while relying on the distribution
kernel driver and the user's `render` permission.

## Correctness smoke test

Intel's official C sample and the Rust harness both transcribed the 2.461-second
Intel reference recording as:

```text
How are you doing today?
```

All five measured Rust NPU runs returned identical text and timestamp chunks.
The existing CPU backend also returned the same text, confirming the model
snapshot and audio input were valid independently of the NPU.

## NPU timing

The successful five-run Rust measurement used automatic language detection,
timestamps, and one persistent pipeline:

| Measurement | Result |
| --- | ---: |
| Model compile/load | 144.264 s |
| First inference | 1.542 s |
| Warm inference 2 | 0.787 s |
| Warm inference 3 | 0.767 s |
| Warm inference 4 | 0.756 s |
| Warm inference 5 | 0.747 s |
| Warm median | 0.762 s |
| Warm median real-time factor | 0.310 |
| Effective warm speed | approximately 3.23x real time |
| Peak resident memory | 7,093,960 KiB (approximately 6.77 GiB) |

The process must remain warm. Compiling Large V3 on every transcription is not
viable.

For comparison, the CPU smoke run loaded in 2.062 seconds but required 5.731
seconds for the same 2.461-second audio (real-time factor 2.329). The warm NPU
path was approximately 7.5x faster than that CPU inference measurement.

## Compatibility findings

With the default Linux Level Zero allocation path, Large V3 compiled but the
first generation returned an opaque native exception. Setting
`DISABLE_OPENVINO_GENAI_NPU_L0=1`, as recommended in Intel's NPU guidance for
execution failures, allowed both Intel's reference sample and the Rust harness
to complete on the physical NPU.

OpenVINO logged a minor Level Zero API mismatch: plugin API 1.16 versus driver
API 1.15. It continued successfully with graph extension 1.18. This combination
must be pinned and tested as a unit rather than mixing arbitrary versions.

The legacy Rust `WhisperPipeline` wrapper failed when English was forced.
Automatic language detection worked on the short Intel clip, but was unstable
on the 11-second JFK clip: the first run returned correct English while later
runs drifted to Norwegian and translation-like output.

OpenVINO 2026.3's newer C++ `ASRPipeline` solved that problem. With language
forced to `<|en|>`, task `transcribe`, segment timestamps enabled, and word
timestamps disabled, five consecutive runs returned the same correct English
transcript:

```text
And so my fellow Americans, ask not what your country can do for you, ask what you can do for your country.
```

The first inference took 2.018 seconds. The next four took 1.366, 1.374, 1.385,
and 1.378 seconds, giving a warm median of 1.376 seconds for approximately 11
seconds of audio (real-time factor 0.125, or about 8x real time). End-to-end
wall time including cold compilation and all five inferences was 167.46 seconds;
peak resident memory was 7,202,780 KiB (approximately 6.87 GiB).

Word-level timestamps caused an additional large graph compilation and did not
finish within eight minutes. Handy does not require them for transcription, so
the production worker should leave them disabled and retain segment timestamps.

## Compiled-cache investigation

OpenVINO's newer C++ `ASRPipeline` supports `CACHE_DIR`, but Rust crate 0.11.0's
safe `WhisperPipeline::new` does not expose constructor properties. The low-level
Rust bindings can pass them, so production has three options:

1. add a safe `new_with_properties` API to `openvino-genai`;
2. wrap the low-level C constructor locally; or
3. use the official C++ `ASRPipeline` inside the isolated worker.

A clean serial C++ cache creation produced one approximately 1.37 GB compiled
blob but still had not completed after ten minutes, so it was terminated.
Interactive cache creation is therefore not viable on this hardware as-is.
Production should keep one native worker and pipeline alive; any compiled-cache
optimization must happen explicitly in the background after model download and
needs a separate bounded experiment.

## Gate status

Completed:

- standalone Rust harness;
- runtime loading and device enumeration;
- real NPU detection with a privately supplied user-mode runtime;
- pinned and verified Large V3 INT8 snapshot;
- CPU correctness smoke test;
- physical NPU correctness smoke test; and
- repeated warm NPU timing.

After a full user logout/login activated `render` membership, a fresh probe
again reported `CPU` and `NPU` with read/write access to `/dev/accel/accel0`.
A completely new `ASRPipeline` process then compiled, transcribed the JFK clip
correctly on all five runs, and exited normally. Cold process wall time was
2:43.47, peak resident memory was 7,201,748 KiB, no swap was used, and the four
warm inference times were 1.382, 1.354, 1.361, and 1.360 seconds. This confirms
ordinary process restart after a new login; suspend/resume recovery remains a
separate test.

Still required before Gate 1 closes:

- none. Model-quality acceptance comes from the primary user's existing full
  Whisper Large V3 testing on Windows. Linux worker suspend/resume recovery is
  a Gate 2 lifecycle concern.

## Interim decision

Gate 1 is a **continue** decision, not yet a merge recommendation. Full Whisper
Large V3 INT8 demonstrably runs on the Lunar Lake NPU, forced English is stable
through the new `ASRPipeline`, and warm inference is comfortably faster than
real time. The production boundary is a persistent native C++ worker using
`ASRPipeline`, not the legacy Rust `WhisperPipeline` wrapper. Gate 2 owns the
remaining cold-start, high-memory, cancellation, and lifecycle risks.
