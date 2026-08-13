# Handy Pop!\_OS Handover

## OpenVINO NPU feasibility outcome (2026-08-13)

The Linux OpenVINO work remains isolated on `codex/openvino-gate1` and was not
merged into production Handy. Gate 1 proved full Whisper Large V3 INT8 runs
correctly and faster than real time on the Lunar Lake NPU. Gate 2 rejected the
production route because its best correct sustained warm RSS was approximately
4.50 GiB, above the user-approved 2.5 GiB limit. Cold compilation peaked around
6.6-7.3 GiB and took roughly 2.5 minutes.

Tests covered OpenVINO 2026.2 and 2026.3, native and compatibility allocation
paths, both speech APIs, allocator trimming, NPU cache/weight properties, and
the historical stateless export route. Repeated transcription was stable and
did not leak, but no configuration met the memory budget. Do not merge or
package the worker without a materially different runtime/driver/model and a
new memory gate.

All experiment-only models, runtimes, archives, audio, reports, caches, build
trees, generated binaries, sockets, and the invalid hybrid model in desktop
trash were removed after durable results were recorded. Approximately 9.6 GB
was reclaimed in total (8.1 GB from the worktree plus 1.5 GB from trash). No
production source, installed package, user model directory, or backup was
changed. See `docs/openvino-npu-linux-design.md` and
`experiments/openvino-gate2/GATE2_RESULTS.md`.

Last verified: 2026-08-10 (Asia/Karachi)

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
