# Complete NPU Feature Removal Contract

The OpenVINO feature is not allowed to become a prerequisite for normal Handy.
Before Gate 3, every production change will be recorded in this file under one
of these removable units:

1. native worker source and executable;
2. private OpenVINO and Intel NPU runtime package manifest;
3. OpenVINO-only model catalogue records and download metadata;
4. the Handy-to-worker protocol adapter and feature detection;
5. OpenVINO-specific settings/localization; and
6. worker/protocol/package tests.

Removal must leave the existing `transcribe-cpp` and `transcribe-rs` model
paths unchanged and buildable. OpenVINO shared libraries remain worker-private
and are never linked into `/usr/bin/handy`.

## Evaluation and removal trigger

Gate 2 must measure rather than infer the steady state. Previous platform
observations are not user-defined limits. Reject and remove the feature if it
proves unreliable, leaks memory across repeated transcription, or the user
decides its measured product tradeoffs are not worthwhile.

Cold compilation peak, warm idle, every repeated run, unload, and process exit
must be reported separately. A peak-RSS-only number is insufficient.

## Current removable production units

- `src-tauri/openvino-worker/`
- `src-tauri/src/managers/openvino_npu.rs`
- OpenVINO variants in `src-tauri/src/managers/model.rs` and
  `src-tauri/src/managers/transcription.rs`
- the Linux-only runtime staging block in `src-tauri/build.rs`
- the OpenCL package dependency and staged-resource entry in
  `src-tauri/tauri.conf.json`
- generated binding variants corresponding to the OpenVINO engine/source
- `docs/openvino-npu-linux-design.md` and `experiments/openvino-gate*/`

The main Handy executable is not linked to OpenVINO. Removing these units leaves
the existing transcribe-cpp and transcribe-rs engines intact.

## Current decision

The isolated branch now includes the production adapter and package closure.
Removal follows the units above and then regenerates bindings and the Debian
package. Downloaded models, runtimes, caches, audio, reports, and temporary
build inputs remain disposable according to `ARTIFACTS.md`.
