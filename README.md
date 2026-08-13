# Handy Pop!\_OS

## Offline speech-to-text for Pop!\_OS and Ubuntu 24.04 with Intel NPU support

Handy Pop!\_OS is a privacy-first desktop dictation application optimized for
Pop!\_OS 24.04, COSMIC Wayland, Ubuntu 24.04, and PipeWire. It transcribes speech
locally with Whisper, Qwen3-ASR, or Parakeet, pastes the result into the active
application, and adds optional Intel NPU acceleration, a multi-monitor overlay,
crash-safe audio ducking, resilient shortcuts, and a dark Linux interface.

Pop!\_OS 24.04 with COSMIC Wayland is the fully tested platform. Ubuntu 24.04
LTS on x86_64 is expected to run the application and its CPU/Vulkan backends,
but Ubuntu GNOME Wayland overlay placement and global shortcuts still require
distribution-specific validation. Ubuntu 22.04 is not supported by the current
Intel NPU build.

**An Intel NPU is optional.** Handy uses one package for every supported
computer. On a machine without a compatible Intel NPU, the application works
normally with its CPU and Vulkan models; the unsupported NPU-only choices are
simply hidden from the Models page.

| Project fact       | Details                                                                               |
| ------------------ | ------------------------------------------------------------------------------------- |
| Current release    | [`0.9.5-popos.1`](docs/releases/0.9.5-popos.1.md)                                     |
| Primary platform   | Pop!\_OS 24.04 on x86_64                                                              |
| Ubuntu support     | Ubuntu 24.04 LTS expected compatible; GNOME Wayland QA pending                        |
| Desktop support    | COSMIC Wayland tested; best-effort support for GNOME and other Linux desktops         |
| Speech recognition | Local Whisper, Qwen3-ASR, Parakeet, and other Handy-compatible models                 |
| Acceleration       | Intel NPU for supported OpenVINO models; Vulkan or CPU for conventional models        |
| Audio stack        | PipeWire and WirePlumber                                                              |
| Privacy            | Speech recognition stays on the computer; optional post-processing is user-configured |
| License            | MIT                                                                                   |
| Last verified      | 13 August 2026                                                                        |

## What is Handy Pop!\_OS?

Handy Pop!\_OS is an open-source Linux voice-typing tool for people who want
offline speech recognition on Pop!\_OS. Press a global shortcut, speak, and stop
recording; the application converts the audio to text locally and pastes it
into the text field that was active when recording began.

The project is designed for Pop!\_OS and Ubuntu users who need private offline
speech-to-text, a visible recording overlay, reliable Linux pasting, and
temporary audio reduction without repeatedly showing the desktop volume popup.
Its COSMIC-specific multi-monitor placement is tested on Pop!\_OS; other
Wayland compositors use the available Linux fallback behavior.

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

- Completely local speech recognition with downloadable Whisper, Qwen3-ASR,
  Parakeet, and other Handy-compatible models.
- Intel NPU-only OpenVINO models are shown automatically on supported systems;
  model weights remain optional downloads rather than inflating the installer.
- Verified Intel NPU paths for Whisper Large V3 INT8, Qwen3-ASR 1.7B INT8, and
  Parakeet TDT 0.6B V3, with persistent compiled-model caches for faster reloads.
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
does not include a language model chosen on the user's behalf. Users do not
need an Intel NPU or OpenVINO model to install and use Handy.

## Pop!\_OS and Ubuntu compatibility

### Does Handy Pop!\_OS work on Ubuntu 24.04?

The current x86_64 Debian package is expected to install and run on Ubuntu
24.04 LTS. Its declared GTK 3, WebKitGTK 4.1, PipeWire, WirePlumber, OpenBLAS,
AppIndicator, and GTK Layer Shell dependencies are available in Ubuntu 24.04.
CPU transcription and Vulkan acceleration do not require Intel NPU hardware.

Pop!\_OS 24.04 with COSMIC Wayland remains the only fully verified desktop.
Ubuntu's default GNOME Wayland session differs from COSMIC: GNOME may handle
global shortcuts, layer-shell surfaces, pointer-output selection, tray icons,
and multi-monitor overlay placement differently. Handy falls back to a regular
overlay window when GTK Layer Shell is unavailable, but that fallback has not
yet completed the same multi-monitor QA performed on COSMIC.

| Environment                      | Current status                  | What users should expect                                                                                        |
| -------------------------------- | ------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Pop!\_OS 24.04 + COSMIC Wayland  | Tested and supported            | Full custom overlay, tray, shortcut, PipeWire, CPU, Vulkan, and supported Intel NPU behavior                    |
| Ubuntu 24.04 + GNOME Wayland     | Expected compatible; QA pending | Core dictation should work; overlay placement and global shortcuts may vary by GNOME configuration              |
| Ubuntu 24.04 + X11               | Expected compatible; QA pending | Core dictation and ordinary window positioning should work; not yet release-tested                              |
| Ubuntu 22.04                     | Unsupported by this build       | Do not install the current Intel NPU package; its runtime and current Intel driver baseline target Ubuntu 24.04 |
| Other Debian-based distributions | Best effort                     | Dependency versions, compositor behavior, and hardware drivers must be checked locally                          |

### What is required for Intel NPU speech recognition on Ubuntu?

NPU acceleration requires compatible Intel hardware and a working Linux NPU
device. The package includes Handy's private OpenVINO userspace runtime, but it
cannot bundle or replace the operating system's kernel module and firmware.
Before NPU models appear, the machine must provide:

- A supported Intel NPU, currently in Meteor Lake, Arrow Lake, Lunar Lake,
  Panther Lake, or another device supported by Intel's current Linux NPU driver.
- The Linux `intel_vpu` kernel module and matching NPU firmware.
- A usable `/dev/accel/accel0` device and permission to access it, commonly
  through the `render` group.
- Ubuntu 24.04 with a compatible kernel/driver combination.

Handy probes the NPU at runtime. If the probe succeeds, the Models page exposes
the NPU filter and OpenVINO downloads. If it fails, those entries remain hidden
and conventional CPU/Vulkan models continue to work normally without warnings
or a separate package. Handy never labels a CPU or GPU fallback as NPU
execution.

Intel documents its supported hardware, kernel-module checks, firmware, and
Ubuntu 24.04 packages in the
[Linux NPU driver documentation](https://github.com/intel/linux-npu-driver/blob/main/docs/overview.md)
and [verified driver releases](https://github.com/intel/linux-npu-driver/releases).

### Which models run on the Intel NPU?

- **Whisper Large V3 INT8:** multilingual transcription and supported
  speech-to-English translation.
- **Qwen3-ASR 1.7B INT8:** multilingual transcription only. Qwen does not
  perform speech translation, and Urdu is not in its official language list;
  unsupported Urdu speech may be identified as Hindi.
- **Parakeet TDT 0.6B V3:** fast multilingual transcription only; it does not
  translate speech into English.

Selecting a language in a transcription-only model constrains transcription;
it does not turn that model into a translator. Use Whisper Large V3 and enable
translation when speech in another language must become English text.

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
- Installed-package OpenVINO NPU transcriptions with Whisper Large V3 INT8,
  Qwen3-ASR 1.7B INT8, and Parakeet TDT 0.6B V3

The detailed implementation evidence is recorded in [HANDOVER.md](HANDOVER.md)
and [the volume-reduction investigation](docs/2026-08-10-v0.9.5-volume-reduction.md).

## Known limitations

- Pop!\_OS 24.04 on x86_64 is the primary tested target.
- Ubuntu 24.04 LTS is expected to be compatible, but GNOME Wayland overlay,
  tray, and global-shortcut behavior has not completed release QA.
- Ubuntu 22.04 is not supported by the current NPU-enabled package.
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
- Intel NPU models require supported hardware, the system `intel_vpu` driver,
  firmware, `/dev/accel/accel0`, and appropriate device permissions. These
  system components are not replaced by the Handy package.

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

### Is this an offline speech-to-text app for Ubuntu 24.04?

Yes, the Debian package is expected to provide offline voice typing on Ubuntu
24.04 LTS. CPU and Vulkan transcription should not require an Intel NPU. Ubuntu
GNOME Wayland is not yet as thoroughly tested as Pop!\_OS COSMIC, so shortcut,
tray, and multi-monitor overlay behavior should be considered best effort until
the Ubuntu test matrix is complete.

### Does Intel NPU support work automatically on every Ubuntu computer?

No. NPU models appear only when Handy detects compatible Intel NPU hardware and
a functioning Linux NPU driver. Computers without a supported NPU can continue
using the normal CPU or Vulkan models without installing a separate Handy
edition.

### Can Qwen3-ASR translate Urdu speech into English?

No. Qwen3-ASR is used for transcription, not speech translation, and Urdu is
not officially supported by that model. For Urdu-to-English or other
speech-to-English translation, select Whisper Large V3 and enable translation.

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
