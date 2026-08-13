# Handy Pop!\_OS Handover

## OpenVINO NPU integration outcome (2026-08-13)

The Linux OpenVINO work remains isolated on `codex/openvino-gate1`. Full Whisper Large V3 INT8
now runs through a worker integrated with Handy and faster than real time on
the Lunar Lake NPU. The best measured sustained warm worker RSS is about
4.50 GiB. Cold compilation peaked around 6.6-7.3 GiB and took roughly 2.5
minutes.

Tests covered OpenVINO 2026.2 and 2026.3, native and compatibility allocation
paths, both speech APIs, allocator trimming, NPU cache/weight properties, and
the historical stateless export route. Repeated transcription was stable and
did not leak. The selected integration target is the newer OpenVINO 2026.3 C++
`ASRPipeline`. The model is conditionally exposed only after the packaged
worker enumerates an NPU, downloads its pinned snapshot on demand, and is kept
out of the main Handy process.

The generated Debian package was tested from its extracted payload. It
enumerated `CPU` and `NPU`, loaded the model explicitly on `NPU` in 166.769
seconds, transcribed the 11-second JFK sample correctly in 2.355 seconds, and
repeated the warm transcription in 1.405 seconds. Unload, shutdown, process
exit, and socket removal all completed successfully. The CPU plug-in is part
of the private runtime because `ASRPipeline` needs it during initialization;
the speech model itself remains explicitly targeted to NPU.

The selected Handy language must be passed explicitly to the NPU pipeline for
every transcription rather than relying on automatic language detection.
Forced decoding is not assumed to suppress speech in other languages: that
behavior requires a dedicated mixed-language validation test before any such
claim or filtering feature is implemented.

Temporary package-test runtime extraction and sockets are removed after the
final results are recorded. The built `.deb` and the two Qwen model directories
are retained as deliverables. See
`docs/openvino-npu-linux-design.md` and
`experiments/openvino-gate2/GATE2_RESULTS.md`.

Isolated NPU-enabled package artifact:

```text
/home/waqar/Documents/projects/mine/Handy/.worktrees/openvino-gate1/src-tauri/target/release/bundle/deb/Handy_0.9.5_amd64.deb
SHA-256: b151aa836963871f8a66dd1eef661b326446e631103bc0bb0fd907f8394f9fe7
```

## Upstream review policy and result (2026-08-13)

The upstream-only post-v0.9.5 history was reviewed before this build. Our
Pop!\_OS overlay, theme, tray, shortcut, update, and recording-volume behavior
has priority over upstream changes. Two safe, independent fixes were adopted:
compressed API responses (`7d2717d0`) and ydotool syntax detection
(`6ac26648`). The upstream Wayland overlay work was not adopted because our
native GTK/COSMIC monitor-following implementation is more complete.

Two upstream changes overlap custom behavior and remain deliberately
unapplied pending an explicit user choice: atomic tray-icon updates overlap the
custom blue non-template tray, and upstream theme propagation overlaps the
custom native overlay/theme path. No other reviewed change was added merely
because it was newer; anything beneficial that conflicts with fork behavior
must be presented before application.

## NPU catalogue and Auto-language correction (2026-08-13)

The initial installed integration exposed only full Whisper Large V3 INT8. It
also sent the literal string `auto` to OpenVINO, causing three user
transcriptions to fail after the model had correctly compiled on NPU. Auto now
leaves the worker language unset so OpenVINO performs detection; explicit
languages are normalized to Whisper tokens such as `<|en|>`.

The NPU-only catalogue now contains 42 entries: 39 official
OpenVINO Whisper revisions plus the independently verified Parakeet TDT V3
revision:
Whisper Turbo, Large V2/V3, Distil Large V2/V3, Medium, Small, Base, and Tiny;
multilingual and English-only variants; and INT4, INT8, and FP16 precision where
published, the verified Parakeet TDT V3 revision, and Handy's Qwen3-ASR 1.7B
INT8/INT4 NPU formats. Model downloads obtain a snapshot manifest, enforce file
sizes, and verify every Hugging Face LFS SHA-256. Non-Whisper architectures are
never falsely routed through Whisper's ASRPipeline.

The Models page displays an NPU-only filter beside Streaming and Translation
only when backend probing exposes NPU models. The installed package was tested
with the previously failing Auto-language recording: cold load completed in
159.545 seconds and NPU transcription completed in 1.505 seconds with
`actual_device: NPU`.

### NPU model-loading feedback

Selecting an OpenVINO NPU model now opens a non-dismissible loading dialog.
It names the selected model, explains that the initial OpenVINO compilation of
a large model can take several minutes, and displays an indeterminate progress
bar until the backend emits `loading_completed` or `loading_failed`. OpenVINO
does not expose a reliable compilation percentage, so the UI deliberately does
not invent one. The dialog is driven by the existing `model-state-changed`
lifecycle and applies to selections from both model-selection interfaces.

OpenVINO's persistent compiled-model cache is enabled for every NPU model. The
cache lives below each downloaded model at
`.handy-npu-cache/openvino-2026.3`, so different models and runtime versions do
not collide and deleting a model also deletes its cache. A real Whisper Large
V3 INT8 measurement improved from 169.968 seconds for the first compilation to
7.629 seconds for the cached reload (about 22 times faster). Its weightless
compiled cache occupied 2.9 GB in addition to the 1.57 GB downloaded model.
The loading dialog explains the one-time compilation and notes that cached
loading still reads model data and allocates memory.

When Translate to English is enabled while a transcription-only model such as
Parakeet is active, Handy now coerces only that run to transcription. The
stored preference is preserved for translation-capable Whisper models.

The NPU filter is mutually exclusive with the Streaming and Translation
filters. Enabling NPU clears those capability filters so the independently
verified Parakeet entry cannot be hidden by stale filter state.

The native worker installs Linux `PR_SET_PDEATHSIG` supervision before opening
its socket. If Handy crashes or is force-killed, the kernel terminates the
multi-gigabyte worker instead of leaving an orphan process and stale NPU state.
Normal shutdown still uses the bounded worker protocol and reaps the child.

### Parakeet TDT V3 and Qwen3-ASR implementation

Parakeet TDT V3 is integrated into the same isolated native worker using the
minimal Apache-2.0 Eddy OpenVINO decoder pinned at commit
`07028cf333f97244f0f3ff718cc748d7dd0a8915`. Handy disables Eddy's silent CPU
fallback. On the local Intel Core Ultra 9 288V, all three speech graphs
(encoder, decoder, and joint) compiled on NPU; only mel preprocessing runs on
CPU by design. The pinned model revision is
`FluidInference/parakeet-tdt-0.6b-v3-ov@dfd55eb6c85a9a8546a162bed84784245d5743c2`.
Cold load took 30.849 seconds and the 11-second JFK sample transcribed correctly
in 229 ms through the integrated worker protocol.

The final Debian payload was tested independently of the development worker.
It enumerated `CPU` and `NPU`, compiled the Parakeet encoder, decoder, and joint
graphs on `NPU`, loaded cold in 27.196 seconds, and transcribed the same sample
correctly in 220 ms (239 ms wall time). The package is 128,487,172 bytes, only
102,812 bytes larger than the preserved Whisper-only NPU installer because the
Parakeet weights remain an on-demand model download. Its SHA-256 is
`570c351081bb89a3cf6b2547dea96e76a98ed96fdbcc36c36f3f0810adb39553`.
The prior installer is preserved outside the repository at
`/home/waqar/.cache/handy-openvino-build/installers/Handy_0.9.5_openvino-whisper-only_amd64.deb`.

The subsequently hardened installed package is 128,490,080 bytes with SHA-256
`e3ca15e1a8bfb63392264d8d4b92a642bc58aca7a3155160a3630b69a537e7c3`.
Its installed Parakeet path loaded from cache in 383 ms and transcribed the
11-second JFK sample twice in 282 ms and 141 ms. A forced Handy termination
also verified that no NPU worker survived.

Local installation still requires administrator authentication. The tested
Parakeet snapshot has been moved, without duplication, to
`~/.local/share/com.pais.handy/models/openvino-parakeet-tdt-v3` so Handy can
recognize it immediately after package installation.

Qwen3-ASR 1.7B is now implemented as an NPU-native pipeline. The official ASR
encoder remains speech-trained Qwen and is compiled on NPU with a fixed
100-frame chunk. Qwen's speech-trained language weights are mapped without
modification into the standard Qwen3 causal decoder architecture, exported
with the conventional `inputs_embeds + attention_mask + position_ids`
contract, and compiled through bounded NPUW on NPU. CPU performs only tokenizer,
mel-spectrogram, embedding lookup/audio merge, and greedy token selection; no
speech encoder or language-model layer silently falls back to CPU/GPU.

The model format includes a marker (`handy_qwen_npu.json`), encoder, prompt
embedding graph, standard stateful decoder, tokenizer/detokenizer, and metadata.
INT8 is the recommended quality format (2.1 GiB locally); INT4 is retained as a
smaller 1.4 GiB option with an accuracy tradeoff. Both remain on-demand model
downloads and are not bundled into the 122.6 MiB installer.

The integrated native worker loaded the INT8 model with
`EXECUTION_DEVICES=NPU` and transcribed the 11-second JFK sample in 2.037 seconds:
“And so, my fellow Americans, ask not what your country can do for you. Ask what
you can do for your country.” The NPU INT8 near-tie around punctuation is handled
by a deliberately narrow A-punctuation-A duplicate suppression rule; a broad
repetition penalty was tested and rejected because it altered legitimate text.
The decoder uses a 1,024-token prompt bucket (approximately the model's
30-second ASR window) and a 256-token response allowance. Compiled blobs use the
same per-model OpenVINO 2026.3 cache as Whisper and Parakeet.

The final Debian package was installed over Handy 0.9.5 on 2026-08-13. The
worker at `/usr/lib/Handy/handy-openvino-npu` then loaded the INT8 model on NPU
and produced the same complete reference transcript in 1.722 seconds. Handy was
restarted with `--start-hidden`; the installed application and both Qwen model
directories are active.

The installed-package regression pass later found two independent failures.
All model workers had reused a PID-only Unix socket, so switching models could
let the retiring worker shut down and unlink the replacement worker's socket.
Workers now receive a unique monotonic socket name, and a dead/disconnected
worker is recreated once with the same model before reporting an error. Qwen
also incorrectly disabled NPUW's shared language-model head and forwarded the
CPU embedding graph's padded 1,024-token output rather than its actual sequence.
The supported shared-head default is restored and prompt/token embeddings are
trimmed to their real lengths.

The corrected Debian package is 128,521,528 bytes with SHA-256
`b888179e34dc10d5ccf5d3ab3bd929006a29738015430aa9f7917886a37a63dd`.
After installing that exact package, the same 11-second JFK WAV passed through
all three installed NPU backends: Whisper Large V3 INT8 in 3.797 seconds,
Qwen3-ASR 1.7B INT8 in 2.447 seconds, and Parakeet TDT 0.6B V3 in 240 ms.
Every result reported `bound_backend: openvino-npu` and produced the complete
reference sentence.

Local model paths:

```text
~/.local/share/com.pais.handy/models/handy-qwen3-asr-1.7b-int8-npu
~/.local/share/com.pais.handy/models/handy-qwen3-asr-1.7b-int4-npu
```

Last verified: 2026-08-13 (Asia/Karachi)

## Purpose

This repository contains a custom Linux build of Handy for Pop!\_OS/COSMIC.
The Windows-only NPU work is intentionally out of scope. The retained fork
features are the dark interface, post-processing fallback chain, dynamic model
hotkeys, CLI model switching, shortcut recovery, and the Linux-specific tray
and overlay work described below.

## Current state

- Branch: `codex/popos-linux-build`
- Upstream release integrated: signed tag `v0.9.5`
- Pop!\_OS integration commit: `0b4ad7b`
- Application version: `0.9.5`
- Source and documentation changes are checkpointed in Git. Local backup and
  recovery artifacts are intentionally untracked and are not part of the
  public repository.
- The project was restored exactly from the complete 2026-08-01 backup before
  the final overlay correction. The backup itself remains untouched.
- The final pointer-probe, opacity, height, hidden-start, responsive-startup,
  and recording-volume reduction fixes are packaged. Installation of the
  latest package requires local administrator authentication.
- After package installation, the application resolves to `/usr/bin/handy`.
- `~/.local/bin/handy` resolves to `/usr/bin/handy`; this prevents the older
  user-local installation from shadowing the packaged executable.
- Upstream update checks are disabled and the persisted setting is `false`.

Local backup-time recovery artifacts are not tracked or published. Install the
newly generated package under `src-tauri/target/release/bundle/deb/` for the
final behavior.

Package artifact:

```text
src-tauri/target/release/bundle/deb/Handy_0.9.5_amd64.deb
```

Artifact SHA-256:

```text
318734973bd15ce93611c8d8e25f75882a8c4615974d61e16bc7804eb7efbf5c
```

The SHA-256 of the executable inside this package is:

```text
6c7ff2bc26f2e8959ea58f8f61a2848b93b5078b48f7675c24b6dc7987695780
```

## Implemented behavior

### Native Linux overlay

`src-tauri/src/overlay.rs` installs a native GTK overlay on Linux because the
Tauri WebKitGTK layer-shell surface could map without submitting a visible
buffer on COSMIC Wayland.

The overlay:

- follows the pointer to the active monitor when recording begins;
- creates a transparent 1×1 layer-shell probe at every trigger, lets COSMIC map
  that fresh surface on the current pointer output, assigns the visible card to
  the probe's GDK monitor, and destroys the probe immediately;
- anchors at the configured top or bottom edge;
- stays non-focusable so typing remains in the active application;
- uses an 82%-opaque charcoal surface, a soft black shadow, and the cool-blue
  application palette;
- shows the recording dot, status label, live voice-level indicators, and
  cancel button;
- applies explicit lower padding to keep all four control groups clear of the
  bottom border;
- shows recording, transcribing, and post-processing states; and
- clears transcript state before every session so stale text cannot reappear;
- expands to show tentative and committed text for streaming-capable models;
- uses 35% of the selected output width, clamped to 400–760 logical pixels; and
- starts at 72 logical pixels high (one live-text line rather than the old
  three-line presentation), grows with wrapped live text up to half the
  selected output height, then
  preserves the complete transcript in a vertical scroller.

Live mode intentionally starts at the compact 400-pixel width. Its first
non-empty transcription update triggers a single 200ms cubic ease-out to the
prescribed monitor-relative width (672 pixels on a 1920-pixel output). Later
updates resize height as needed without replaying the width animation.

Linux registers shortcuts during backend startup and initializes Enigo on a
background worker. Native Wayland paste paths no longer wait for the X11
keyboard-map scan, while Enigo remains available as a fallback. This keeps
`--start-hidden`, tray-only, and autostart launches responsive.

### Lower output volume while recording

General Settings > Sound provides a 0–90% **Lower Volume While Recording**
slider. The percentage is relative to the current output level, and Handy
leaves saved application volumes and the physical output level unchanged. On
Linux, Handy temporarily redirects active PipeWire playback links through a
private gain node. This keeps COSMIC's volume OSD and feedback sound from
appearing at every recording start and stop. A separate cleanup watchdog knows
the original graph links and restores them on normal stop, stream recovery, or
a full Handy crash before removing the gain node. A value of 0% disables
attenuation; full **Mute While Recording** takes precedence. The live Pop!\_OS
checks used a 70% reduction (30% gain), including a disappearing playback stream
and a forced `SIGKILL`; the master stayed at 99%, saved Brave/Music settings did
not change, and playback links returned to the speaker in both cases.

The compact controls were verified in the live release at `y=39`, height `26`,
inside a `72`-pixel-high frame, leaving a visible 7-pixel lower gap. The GTK
row uses a 6-pixel bottom margin; the remaining pixel comes from the border.

`src-tauri/src/managers/transcription.rs` forwards live transcription updates
to the native Linux overlay. Streaming text only appears when the selected
model advertises streaming support. Non-streaming models keep the compact
recording and processing states.

`src-tauri/build.rs`, `src-tauri/src/tray_i18n.rs`, and
`src/i18n/locales/en/translation.json` provide the Linux overlay labels and
translation plumbing.

### Cool-blue Linux tray states

`src-tauri/src/tray.rs` preserves the existing idle, recording, and
transcribing silhouettes but recolors every visible pixel to `#7AA2F7` while
preserving alpha.

Linux icons are not marked as template icons, because COSMIC or another shell
may recolor template icons. `src-tauri/src/lib.rs` also applies the blue tint
to the initial idle icon at startup instead of waiting for the first state
transition.

The icon exported through the live StatusNotifierItem was inspected directly
and confirmed blue.

### Updates disabled on Linux

This fork must not be replaced automatically by an upstream Handy release.

- Linux defaults `update_checks_enabled` to `false`.
- Existing saved `true` values are forced to `false` when settings load.
- Attempts to enable update checks through the Linux settings command remain
  disabled and report `false` to the frontend.
- Automatic and manual checks therefore do not run.
- Future custom releases must be installed manually from their `.deb` files.

Relevant files are `src-tauri/src/settings.rs`,
`src-tauri/src/shortcut/mod.rs`, and `src-tauri/src/lib.rs`.

### Shortcuts and suspend recovery

The platform-neutral elapsed-time watchdog re-registers shortcuts after a gap
longer than five seconds, which provides best-effort recovery after
suspend/resume. The Windows WTS lock watcher remains Windows-only and is not
used on Linux.

Low-level Linux shortcuts read `/dev/input`. The user must belong to the
`input` group and start a new login session after being added. Wayland
compositor policy can still limit application-owned global shortcuts; the
watchdog does not bypass those restrictions.

### Theme and retained fork features

- New installations default to the dark neutral theme.
- Post-processing retains three priority models and automatic fallback.
- Downloaded models can have dynamic global model-switch hotkeys.
- `handy --load-model <query>` switches the active downloaded model.
- The old Windows-only NPU feature is not required for this Linux build.

## Verification completed

The final source was checked with:

```bash
bun run lint
cargo test --manifest-path src-tauri/Cargo.toml --lib
git diff --check
```

Results:

- ESLint passed.
- The TypeScript/Vite production frontend build passed.
- All 180 Rust library tests passed, including monitor-relative live-overlay
  sizing, Linux clipboard restoration, volume attenuation, clamping, and Linux
  backend parsing.
- Formatting and whitespace checks passed.
- The production Debian package completed successfully.
- The rebuilt release was run directly and tested on COSMIC Wayland.
- The selected streaming-capable Parakeet model opened the live overlay on all
  three tested pointer outputs. Logs recorded `(0,0 1920x1200)`,
  `(1920,0 1920x1200)`, and `(3840,0 1920x1200)`.
- Live transcription finalized correctly. The installed v0.9.5 build logs
  shortcuts, tray, Enigo, and the GTK overlay as ready within the same second;
  Enigo's keyboard scan no longer blocks native Linux paste paths.
- The native overlay was visible and its lower spacing was verified through
  its accessibility geometry.
- The exported tray PNG was inspected and confirmed to use the blue accent.
- The installed executable hash matches the final package payload.
- The saved update-check setting is `false`.

Existing non-blocking Rust warnings remain in unrelated code:

- an unused `super::*` import in the clamshell tests; and
- an unused platform-gated `Emitter` import in secure-input code; and
- an initial assignment to `model_takes_initial_prompt` that is overwritten
  before it is read.

## Build and install

Prerequisites are Rust stable, Bun, the packages listed in `LINUX.md` and
`BUILD.md`, and the Silero VAD model under
`src-tauri/resources/models/silero_vad_v4.onnx`.

Install JavaScript dependencies and build the Debian package:

```bash
bun install
bun run tauri build --bundles deb --no-sign
```

Install or reinstall the exact custom package:

```bash
sudo dpkg -i \
  "./src-tauri/target/release/bundle/deb/Handy_0.9.5_amd64.deb"
```

The installed package version is `0.9.5`. Quit Handy before replacing the
package, then launch it normally.

On a fresh Linux account, grant shortcut access once:

```bash
sudo usermod -aG input "$USER"
```

Log out and back in afterward.

## Operational checks

Confirm the installed package and active executable:

```bash
dpkg-query -W -f='${Status} ${Version}\n' handy
readlink -f "/proc/$(pgrep -n handy)/exe"
readlink -f ~/.local/bin/handy
```

Expected executable and launcher result:

```text
/usr/bin/handy
```

Confirm updates remain disabled:

```bash
jq '.settings.update_checks_enabled' \
  ~/.local/share/com.pais.handy/settings_store.json
```

Expected value:

```text
false
```

If the installed application appears unchanged, check for an older
`~/.local/opt/Handy` executable shadowing `/usr/bin/handy`. Repair the launcher
resolution with:

```bash
ln -sfn /usr/bin/handy ~/.local/bin/handy
```

## Known limitations

- Linux shortcut recovery is best effort under Wayland.
- Live overlay text depends on model streaming capability.
- `HANDY_NO_GTK_LAYER_SHELL=1` disables layer-shell integration for
  compositor troubleshooting and uses the fallback window behavior.
- The raw release executable needs the packaged runtime resources and native
  transcription libraries. Prefer the `.deb` rather than copying only the
  binary.
- Automatic updates are intentionally unavailable. Rebuild and reinstall the
  package manually for future changes.
- The current work is uncommitted. Review and commit it before changing
  branches or handing the repository to a clean environment.

## Modified files

- `BUILD.md`
- `LINUX.md`
- `README.md`
- `docs/superpowers/specs/2026-07-30-popos-linux-migration-design.md`
- `src-tauri/build.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/managers/transcription.rs`
- `src-tauri/src/overlay.rs`
- `src-tauri/src/settings.rs`
- `src-tauri/src/shortcut/mod.rs`
- `src-tauri/src/tray.rs`
- `src-tauri/src/tray_i18n.rs`
- `src/i18n/locales/en/translation.json`
- `HANDOVER.md`
- `docs/2026-08-01-overlay-changes-investigation.md`
- `docs/investigation-2026-08-01/README.md`

## Documentation map

- `LINUX.md`: authoritative Pop!\_OS installation, build, behavior, and
  troubleshooting guide.
- `BUILD.md`: cross-platform build prerequisites and package commands.
- `README.md`: user-facing Linux behavior and limitations.
- `docs/superpowers/specs/2026-07-30-popos-linux-migration-design.md`: design
  decisions and retained scope.
- `HANDOVER.md`: verified project state and continuation guide.
- `docs/2026-08-01-overlay-changes-investigation.md`: complete stale-text,
  multi-monitor, and streaming-size diagnosis and implementation record.
- `docs/investigation-2026-08-01/README.md`: concise evidence ledger and manual
  regression matrix; no temporary probe programs are retained.
- `docs/2026-08-10-v0.9.5-volume-reduction.md`: upstream integration conflict
  decisions, recording-volume semantics, backend order, and verification.

## Next maintainer checklist

1. Preserve the source commits and the untracked backup/recovery artifacts.
2. Read `AGENTS.md`, `LINUX.md`, and this handover before editing.
3. Run lint, Rust tests, and `git diff --check` after changes.
4. Rebuild only the Debian bundle with `--no-sign`.
5. Reinstall the locally built `.deb` explicitly.
6. Verify the running process resolves to `/usr/bin/handy`.
7. Test the shortcut, all tray states, compact overlay, streaming overlay when
   a streaming-capable model is available, cancel action, and suspend/resume.
8. Keep upstream update checks disabled unless the custom fork is given its
   own controlled update channel and signing key.
