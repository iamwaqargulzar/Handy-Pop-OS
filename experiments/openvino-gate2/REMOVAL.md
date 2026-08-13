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

## Current removable files

- `docs/openvino-npu-linux-design.md`
- `experiments/openvino-gate1/`
- `experiments/openvino-gate2/`

No production Handy source or package manifest has been modified at this point.

## Current decision

No production feature has been integrated, so removing the experiment requires
no edit to Handy. All experiment downloads, models, runtimes, caches, audio,
reports, and build outputs are disposable and are cleaned according to
`ARTIFACTS.md`. Only the tracked feasibility source and evidence remain for
future reference.
