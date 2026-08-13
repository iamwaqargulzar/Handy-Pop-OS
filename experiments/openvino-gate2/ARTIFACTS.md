# OpenVINO Experiment Artifact Ledger

This ledger distinguishes durable work from disposable local material. It must
be updated whenever Gate 2 creates another artifact class.

## Durable, tracked files

These are intentional source or documentation and remain after cleanup:

- `docs/openvino-npu-linux-design.md`
- `experiments/openvino-gate1/.gitignore`
- `experiments/openvino-gate1/Cargo.toml`
- `experiments/openvino-gate1/Cargo.lock`
- `experiments/openvino-gate1/src/`
- `experiments/openvino-gate1/scripts/`
- `experiments/openvino-gate1/README.md`
- `experiments/openvino-gate1/GATE1_RESULTS.md`
- `experiments/openvino-gate2/.gitignore`
- `experiments/openvino-gate2/CMakeLists.txt`
- `experiments/openvino-gate2/src/`
- `experiments/openvino-gate2/README.md`
- `experiments/openvino-gate2/REMOVAL.md`
- `experiments/openvino-gate2/GATE2_RESULTS.md`
- this ledger

## Disposable experiment artifact classes

| Path | Purpose | Current approximate size | Final disposition |
| --- | --- | ---: | --- |
| `experiments/openvino-gate1/audio/` | Public test audio and converted WAVs | 1.6 MB | Delete |
| `experiments/openvino-gate1/models/` | Large V3 INT8 model snapshot | 1.5 GB | Delete or move to Handy's eventual user model directory only after Gate 3 |
| `experiments/openvino-gate1/reports/` | Raw local JSON reports | 32 KB | Summarize durable results, then delete |
| `experiments/openvino-gate1/runtime/` | Archives, extracted OpenVINO/NPU libraries, sample builds, failed cache experiments | 3.9 GB | Delete completely; release runtime is rebuilt from a recorded manifest |
| `experiments/openvino-gate1/target/` | Rust build outputs | 275 MB | Delete |
| `experiments/openvino-gate2/build/` | CMake cache and worker binary | currently under 1 MB; will grow | Delete |
| `$XDG_RUNTIME_DIR/handy-openvino-gate2.sock` | Runtime Unix socket | negligible | Worker removes it; verify absent |

The 3.9 GB runtime tree is intentionally much larger than any planned package.
It contains development archives, extracted packages, OpenVINO samples, C/C++
build trees, OpenCV build dependencies, and unsuccessful compiled-cache data.
None of those directories will be copied wholesale into Handy.

## Cleanup boundary

Cleanup is limited to the explicit ignored paths in the table above and the
single named runtime socket. Never clean the repository root, the parent
project directory, Handy's real user model directory, or any backup folder.

Before deletion:

1. stop the isolated worker;
2. copy all durable measurements into the tracked Gate results;
3. confirm `git status --ignored` classifies every target as ignored; and
4. resolve and inspect each exact target path.

After deletion:

1. confirm every disposable path and socket is absent;
2. run `git status --short --ignored` again;
3. ensure only tracked source/docs or intentional Git changes remain;
4. build conventional Handy without the experiment artifacts; and
5. record reclaimed disk space in the handover.

Production packaging is rebuilt from pinned external artifacts and copies only
the recorded private runtime closure. Disposable downloads are never committed
or copied wholesale into the package.

## Final cleanup result

Earlier Gate downloads were removed after their durable measurements were
copied into `GATE1_RESULTS.md` and `GATE2_RESULTS.md`. The final package-level
test used explicitly named `/tmp/handy-*20260813` paths for the runtime, model,
extracted Debian payload, audio, and socket. Those temporary paths are removed
after documentation; the generated `.deb` remains the only binary deliverable.
No user model directory, backup, parent-project file, or installed application
is included in cleanup.
