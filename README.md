# Handy Pop!\_OS

## Offline speech-to-text and voice typing for Pop!\_OS and COSMIC Wayland

Handy Pop!\_OS is a privacy-first desktop dictation application optimized for
Pop!\_OS 24.04, COSMIC Wayland, and PipeWire. It transcribes speech locally with
Whisper or Parakeet, pastes the result into the active application, and adds a
multi-monitor overlay, crash-safe audio ducking, resilient shortcuts, and a
dark COSMIC-friendly interface.

| Project fact       | Details                                                                               |
| ------------------ | ------------------------------------------------------------------------------------- |
| Current release    | [`0.9.5-popos.1`](docs/releases/0.9.5-popos.1.md)                                     |
| Primary platform   | Pop!\_OS 24.04 on x86_64                                                              |
| Desktop support    | COSMIC Wayland; best-effort support for other Linux desktops                          |
| Speech recognition | Local Whisper-family and Parakeet models                                              |
| Audio stack        | PipeWire and WirePlumber                                                              |
| Privacy            | Speech recognition stays on the computer; optional post-processing is user-configured |
| License            | MIT                                                                                   |
| Last verified      | 11 August 2026                                                                        |

## What is Handy Pop!\_OS?

Handy Pop!\_OS is an open-source Linux voice-typing tool for people who want
offline speech recognition on Pop!\_OS. Press a global shortcut, speak, and stop
recording; the application converts the audio to text locally and pastes it
into the text field that was active when recording began.

The project is designed for COSMIC users who need a visible recording overlay,
correct multi-monitor placement, reliable Wayland pasting, and temporary audio
reduction without repeatedly showing the desktop volume popup.

## Why was this Pop!\_OS version created?

The standard cross-platform experience did not fully match COSMIC Wayland.
Overlay placement could select the wrong monitor, the recording card did not
fit the desktop styling, shortcut recovery after suspend was inconsistent, and
lowering playback through normal volume controls triggered COSMIC's on-screen
display. This version addresses those Linux-specific problems directly.

## What was modified, how, and why?

| Area                    | Modification                                                                                                                         | Why it was needed                                                                                                     | Result                                                                                   |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| Multi-monitor overlay   | A fresh GTK layer-shell probe identifies the output under the pointer at every recording trigger.                                    | Static monitor selection placed the overlay on the wrong display.                                                     | The card follows the pointer across mixed multi-monitor layouts.                         |
| Overlay layout          | The card starts at 72 logical pixels high and 400 pixels wide, then sizes to 35% of the selected output, clamped to 400–760 pixels.  | The previous card was too tall and could begin at an inconsistent width.                                              | A compact first frame with predictable monitor-relative sizing.                          |
| Overlay animation       | The first live words trigger a 200 ms cubic ease-out width transition.                                                               | Instant resizing looked abrupt.                                                                                       | A smooth transition from the compact state to live transcription.                        |
| Overlay appearance      | An 82%-opaque charcoal surface, soft black shadow, cool-blue controls, and explicit lower padding were added.                        | The previous pink/orange presentation did not match the dark COSMIC desktop.                                          | A readable overlay that remains slightly transparent and visually balanced.              |
| Streaming text          | Tentative and committed text are forwarded to the native Linux card for models that support streaming.                               | Recording state alone did not show live recognition progress.                                                         | Live transcription appears without stealing focus from the target application.           |
| Playback reduction      | Active PipeWire links are routed temporarily through a private gain node.                                                            | Changing the physical sink triggered COSMIC's volume OSD and sound; changing app volumes could persist after a crash. | Playback becomes quieter without changing the master level or saved application volumes. |
| Crash recovery          | An independent watchdog stores the original PipeWire graph links and restores them if recording ends or Handy is terminated.         | Audio could remain at a reduced level when a process stopped unexpectedly.                                            | Normal stop, disappearing streams, and forced `SIGKILL` restore routing safely.          |
| Tray and theme          | The interface defaults to charcoal and the Linux tray uses the same `#7AA2F7` blue accent in idle, recording, and processing states. | The tray and application previously used mismatched colors.                                                           | A consistent dark Pop!\_OS appearance.                                                   |
| Linux startup and paste | Native Wayland paste paths run before the slower X11/Enigo fallback initialization.                                                  | Hidden startup and transcription paste could stall while X11 keyboard mapping initialized.                            | Faster tray startup and more responsive Wayland pasting.                                 |
| Suspend recovery        | An elapsed-time watchdog re-registers shortcuts after a pause longer than five seconds.                                              | Suspend/resume can invalidate application-owned shortcut hooks.                                                       | Best-effort shortcut recovery without pretending to bypass compositor security.          |
| Model selection         | Downloaded models can receive global shortcuts, and `handy --load-model <query>` switches models from the command line.              | Switching transcription models required opening the settings window.                                                  | Fast model changes from a keyboard shortcut, script, or macro.                           |
| Post-processing         | Three priority model choices are stored per provider and retried in order after API or rate-limit failures.                          | One unavailable model could stop the entire post-processing action.                                                   | Automatic fallback with provider-specific reasoning-effort settings.                     |
| Updates                 | Upstream update checks are disabled on Linux.                                                                                        | A standard update could overwrite the Pop!\_OS-specific behavior.                                                     | This version changes only when the user installs a new Handy Pop!\_OS package.           |

## Key features

- Completely local speech recognition with downloadable Whisper-family and
  Parakeet models.
- Vulkan acceleration when supported, with CPU inference available as a
  fallback.
- Configurable toggle-to-record and push-to-talk shortcuts.
- Native, non-focusable Linux recording overlay for COSMIC Wayland.
- Live voice indicators, recording status, streaming text, processing state,
  and a cancel control.
- Correct pointer-based placement across multiple displays.
- Dark neutral interface and cool-blue Linux tray states.
- Configurable 0–90% playback reduction while recording.
- Crash-safe PipeWire cleanup that leaves the speaker master level and saved
  application volumes unchanged.
- Automatic transcription pasting through Wayland-aware methods.
- Local history, custom vocabulary, language selection, and translation where
  supported by the selected model.
- Optional LLM post-processing with a three-model fallback chain.
- Global shortcuts and CLI commands for switching downloaded models.
- Best-effort shortcut recovery after sleep or resume.
- Disabled automatic updates to protect the Pop!\_OS customization.

## How to install Handy Pop!\_OS

The recommended installation method is the Debian package attached to the
[latest Handy Pop!\_OS release](https://github.com/iamwaqargulzar/Handy-Pop-OS/releases/latest).

1. Download `Handy_0.9.5_amd64.deb` from the release page.
2. Open a terminal in the download directory.
3. Install the package and its declared dependencies:

   ```bash
   sudo apt install ./Handy_0.9.5_amd64.deb
   ```

4. Allow the low-level shortcut listener to read Linux input devices:

   ```bash
   sudo usermod -aG input "$USER"
   ```

5. Log out and back in so the new group membership takes effect.
6. Launch **Handy** from the application menu, download a transcription model,
   and configure the recording shortcut.

Speech-recognition models are downloaded separately on first use. The package
does not include a language model chosen on the user's behalf.

## How does crash-safe audio ducking work?

Handy Pop!\_OS lowers other playback without changing the speaker's master
volume. At recording start, active PipeWire playback links are moved through a
temporary gain node set to the requested reduction. At recording stop, an
independent watchdog reconnects the original links and removes the temporary
node.

This design specifically avoids two failure modes:

- COSMIC's volume popup and feedback sound are not triggered on every
  transcription because the physical output volume is untouched.
- Browser and media-player volumes cannot become permanently quiet because
  their saved per-application volume values are never edited.

The watchdog was verified with normal stop, a playback stream disappearing
during recording, and a forced Handy `SIGKILL`. In all three cases, routing
returned to the physical speaker; the 99% master level and saved application
volumes remained unchanged.

## Handy Pop!\_OS compared with the standard cross-platform behavior

| Capability                  | Handy Pop!\_OS                                        | Standard cross-platform behavior  |
| --------------------------- | ----------------------------------------------------- | --------------------------------- |
| COSMIC-focused overlay      | Native GTK layer shell with pointer-monitor detection | General platform overlay          |
| Multi-monitor placement     | Re-evaluated at every recording trigger               | Platform-dependent                |
| Overlay presentation        | Compact, translucent, animated, dark blue             | General application styling       |
| Recording audio reduction   | Private PipeWire gain path with crash watchdog        | Platform volume control           |
| COSMIC volume OSD avoidance | Yes                                                   | Not specifically targeted         |
| Linux tray palette          | Cool blue in every state                              | General tray assets               |
| Suspend shortcut recovery   | Best-effort elapsed-time watchdog                     | Platform-dependent                |
| Model-switch hotkeys        | Per downloaded model                                  | Not part of the baseline workflow |
| CLI model switching         | `handy --load-model <query>`                          | Not part of the baseline workflow |
| Post-processing fallback    | Three ordered model choices                           | Single selected model             |
| Linux update checks         | Disabled                                              | Normally configurable             |

## Command-line controls

```bash
handy --toggle-transcription       # Start or stop local dictation
handy --toggle-post-process        # Record with optional post-processing
handy --cancel                     # Cancel the active operation
handy --load-model large           # Switch to a downloaded model by name
handy --start-hidden               # Start in the tray
handy --debug                      # Enable detailed diagnostic logging
```

The application uses single-instance forwarding, so these commands can
control an already-running Handy process from COSMIC shortcuts, shell scripts,
or supported macro tools.

## Build Handy Pop!\_OS from source

### Required development tools

- Pop!\_OS 24.04 or a compatible Ubuntu/Debian-based distribution
- Rust stable
- Bun
- CMake, GTK 3, WebKitGTK 4.1, OpenBLAS, Vulkan build tools, PipeWire,
  WirePlumber, and GTK layer shell development packages

The complete reusable dependency list is in [LINUX.md](LINUX.md).

### Build steps

```bash
bun install
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx \
  https://blob.handy.computer/silero_vad_v4.onnx
bun run lint
cargo test --manifest-path src-tauri/Cargo.toml --lib
bun run tauri build --bundles deb --no-sign
```

The Debian package is generated at:

```text
src-tauri/target/release/bundle/deb/Handy_0.9.5_amd64.deb
```

Local release builds use `--no-sign` because the public updater key does not
include the corresponding private signing key.

## Tested environment and verification

The `0.9.5-popos.1` source and package were verified on Pop!\_OS 24.04 with
COSMIC Wayland, PipeWire, WirePlumber, multiple displays, and an x86_64 system.
The latest checkpoint passed:

- 180 Rust library tests
- ESLint frontend checks
- TypeScript and Vite production build
- Rust formatting and type checking
- Rust Clippy checks, with only pre-existing unrelated warnings
- Normal and forced-crash PipeWire routing restoration tests
- Multi-monitor pointer-following overlay tests

The detailed implementation evidence is recorded in [HANDOVER.md](HANDOVER.md)
and [the volume-reduction investigation](docs/2026-08-10-v0.9.5-volume-reduction.md).

## Known limitations

- Pop!\_OS 24.04 on x86_64 is the primary tested target.
- Low-level application-owned shortcuts need `/dev/input` access through the
  `input` group. COSMIC or another Wayland compositor can still impose its own
  shortcut restrictions.
- The native overlay requires compositor support for GTK layer shell. It can
  be disabled with `HANDY_NO_GTK_LAYER_SHELL=1` if necessary.
- Playback streams already active when recording begins are ducked. A new
  stream that starts during the same recording may remain at its normal level
  until the next recording.
- Optional LLM post-processing can send completed text to a provider selected
  by the user; local speech recognition itself does not require that feature.

## Frequently asked questions

### Does Handy Pop!\_OS work offline?

Yes. Whisper-family and Parakeet speech-recognition models run locally after
they are downloaded, so microphone audio does not need to be sent to a cloud
transcription service. Optional LLM post-processing is separate, disabled
unless configured, and may use a remote provider chosen by the user.

### Does it support Pop!\_OS COSMIC Wayland?

Yes. Pop!\_OS 24.04 with COSMIC Wayland is the primary tested environment. The
custom native overlay uses GTK layer shell, follows the pointer to the active
display, and avoids taking keyboard focus from the application receiving the
transcribed text.

### Can it type into any Linux application?

Handy pastes transcription into the application that was active when recording
started. Compatibility depends on the Wayland compositor and target
application, so Wayland-aware paste tools and fallback methods are provided.

### Why does it need access to the Linux input group?

The low-level global shortcut listener reads keyboard events from `/dev/input`.
Membership in the `input` group allows the listener to detect the configured
shortcut outside the Handy window. The permission takes effect only after the
user logs out and back in.

### Will lowering playback permanently change my volume?

No. The Pop!\_OS implementation does not edit the physical output level or
saved per-application volumes. It uses a temporary PipeWire routing graph and
an independent cleanup watchdog designed to restore the original route even
if the Handy process crashes.

### Does the overlay show live transcription?

Yes, when the selected model exposes streaming recognition. Models without
streaming support retain the compact recording and processing states and show
the final transcription after recognition completes.

### Are automatic updates enabled?

No. Linux update checks are disabled in this project to prevent a standard
package from replacing the Pop!\_OS-specific overlay, tray, shortcut, and audio
behavior. New versions must be installed deliberately from this repository.

## Documentation

- [Pop!\_OS build and deployment guide](LINUX.md)
- [Handy Pop!\_OS 0.9.5-popos.1 release notes](docs/releases/0.9.5-popos.1.md)
- [Technical handover and verified behavior](HANDOVER.md)
- [Volume-reduction design and crash recovery](docs/2026-08-10-v0.9.5-volume-reduction.md)
- [Overlay investigation](docs/2026-08-01-overlay-changes-investigation.md)
- [General contributor guide](CONTRIBUTING.md)

## License

Handy Pop!\_OS is distributed under the [MIT License](LICENSE). The existing
copyright and permission notice are retained.

## Attribution

Handy Pop!\_OS is based on [Handy, the original open-source project by CJ Pais](https://github.com/cjpais/Handy).
