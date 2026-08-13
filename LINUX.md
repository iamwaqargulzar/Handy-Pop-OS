# Handy — Pop!\_OS Build and Deployment Guide

This guide covers the current Handy `0.9.5` source tree on Pop!\_OS 24.04 and
other Ubuntu/Debian-based distributions. The retired Windows Python/HTTP NPU
experiment is not part of this Linux build; it is replaced by an isolated
native Linux OpenVINO worker.

Tracked Windows source and installer configuration may remain in the
repository. Rust and Tauri select platform-specific code at build time, so
those files do not interfere with a Linux build. Generated directories such
as `node_modules/`, `dist/`, `src-tauri/target/`, and
`src-tauri/transcribe-libs/` can be regenerated and should not be copied
between operating systems.

## Supported custom behavior

The Linux build retains:

- Local transcription with CPU and Vulkan backends
- Conditional full Whisper Large V3 INT8 transcription on supported Intel NPUs
- Three-level post-processing model fallback
- Per-provider reasoning-effort settings
- Global hotkeys for switching downloaded models
- CLI model switching with `handy --load-model <query>`
- A dark neutral interface with a cool accent
- A matching cool-blue tray icon for idle, recording, and transcribing states
- A native dark GTK layer-shell overlay on Linux with recording levels,
  transcribing/processing states, and a cancel control
- Live text in the overlay when the selected model supports streaming
- Configurable 0–90% system-output reduction while recording, with exact
  volume restoration afterward
- Update checks disabled so upstream releases cannot overwrite this custom
  Linux build
- Best-effort shortcut re-registration after a suspend/resume time gap

The Win32 WTS session lock watcher remains Windows-only. On Linux, the
platform-neutral elapsed-time watchdog can recover shortcuts after a long
pause, but it cannot bypass compositor security restrictions.

## Dependency locations

Dependencies are intentionally installed at the broadest sensible scope:

- `apt` packages are distribution-managed system dependencies and can be
  reused by other projects.
- Rust and Bun are installed once for the current user and reused across
  projects.
- `node_modules/`, Cargo output, and generated transcription libraries remain
  project-specific because their exact versions and build options belong to
  this application.

Do not manually copy development libraries into `/usr/lib`. The packaged
Handy runtime libraries belong in the private `/usr/lib/Handy/` directory.

## Fresh Pop!\_OS installation

The generated Debian package contains the complete customized application,
including the Linux overlay and tray changes, frontend resources, VAD model,
and Handy's private conventional and OpenVINO transcription runtimes. The
roughly 1.57 GB OpenVINO model is not bundled; it appears only when the private
worker detects an NPU and is downloaded on user request.

Install the package with APT so its declared runtime dependencies are resolved:

```bash
sudo apt install ./Handy_0.9.5_amd64.deb
```

Handy's low-level global shortcut implementation reads `/dev/input`. Grant the
current user that one-time permission and then log out and back in:

```bash
sudo usermod -aG input "$USER"
```

Verify the new login session and installation:

```bash
id -nG | tr ' ' '\n' | grep -x input
find /dev/input -maxdepth 1 -type c -readable | head
dpkg-query -W -f='${Status} ${Version}\n' handy
```

The package does not silently change group membership. On a fresh machine,
package installation and the `input` group command are therefore the only
required system-level setup beyond normal Pop!\_OS graphics/NPU kernel support.
Development dependencies in the next section are needed only when building
from source.

## Install prerequisites

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  cmake \
  libssl-dev \
  libasound2-dev \
  libvulkan-dev \
  vulkan-tools \
  glslc \
  spirv-headers \
  glslang-tools \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libgtk-layer-shell0 \
  libgtk-layer-shell-dev \
  libopenblas-dev \
  ocl-icd-libopencl1 \
  pipewire-bin \
  pulseaudio-utils \
  wireplumber \
  patchelf \
  xdg-utils \
  wtype
```

Install Rust for the current user:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

Install Bun for the current user:

```bash
curl -fsSL https://bun.sh/install | bash
source "$HOME/.bashrc"
```

Verify Vulkan:

```bash
vulkaninfo --summary
```

Pop!\_OS normally provides the appropriate Mesa or NVIDIA runtime through its
normal driver tooling. Prefer the distribution's current recommended driver;
do not pin an old driver version from this guide.

## Prepare the source

From the repository root:

```bash
bun install
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx \
  https://blob.handy.computer/silero_vad_v4.onnx
```

If the VAD model already exists, the download is unnecessary.

Run the application in development mode:

```bash
bun run tauri dev
```

Run the verification commands:

```bash
bun run format:check
bun run lint
bun run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## OpenVINO NPU build inputs

The Linux package uses OpenVINO Runtime 2026.3.0, OpenVINO GenAI 2026.3.0.0,
Intel NPU user-mode driver 1.35.0, and Level Zero loader 1.32.0. These are build
inputs, not global development dependencies on the destination machine. Keep
their extracted trees outside the repository and provide these paths:

```bash
export HANDY_OPENVINO_GENAI_ROOT=/path/to/openvino_genai_ubuntu24_2026.3.0.0_x86_64
export HANDY_NPU_LEVEL_ZERO_LIB=/path/to/libze_intel_npu.so.1.35.0
export HANDY_LEVEL_ZERO_LOADER_LIB=/path/to/libze_loader.so.1.32.0
```

`HANDY_OPENVINO_GENAI_ROOT` must contain `runtime/include`,
`runtime/lib/intel64`, and `runtime/3rdparty/tbb/lib`. The build copies only the
recorded runtime closure, including the NPU and CPU plug-ins required by
ASRPipeline. It does not package headers, CMake files, Python, samples, tools,
the GPU plug-in, or model weights. If these variables are intentionally unset,
the conventional Handy package remains buildable and the NPU catalogue stays
hidden.

## Build the Pop!\_OS package

Build only the Debian bundle:

```bash
bun run tauri build --bundles deb --no-sign
```

The package is written under:

```text
src-tauri/target/release/bundle/deb/Handy_*_amd64.deb
```

The repository contains the public updater key used for official releases.
Local builds without the matching private key must pass `--no-sign`. Linux
does not need the Windows short `CARGO_TARGET_DIR` workaround.

The verified NPU-enabled artifact is 123 MiB compressed, declares 352,174 KiB
installed, and has SHA-256
`c58d4b0dd71bd06aaf983ca95bd2eb222b47185fd9f10196d12e5e4f4115a356`.

## Intel NPU runtime behavior

Handy starts `/usr/lib/Handy/handy-openvino-npu` with its private libraries and
`DISABLE_OPENVINO_GENAI_NPU_L0=1`. It shows the OpenVINO model only when the
worker enumerates `NPU`. The worker may also enumerate `CPU` because
ASRPipeline needs the CPU plug-in during initialization, but model creation is
explicitly `ASRPipeline(..., "NPU")`; it never labels a CPU/GPU fallback as NPU.

The first model load compiles the graph and can take roughly 2.5 minutes. The
verified extracted-package run loaded in 166.769 seconds and transcribed the
11-second JFK sample in 2.355 seconds on its first run and 1.405 seconds warm.

## Install or inspect the package

Install the bundle with APT so dependencies are resolved:

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/Handy_*_amd64.deb
```

To inspect it without installing:

```bash
inspection_dir="$(mktemp -d)"
dpkg-deb -x src-tauri/target/release/bundle/deb/Handy_*_amd64.deb \
  "$inspection_dir"
find "$inspection_dir" -maxdepth 4 -type f -print
```

For a manual refresh after a source rebuild, keep libraries in the
application-private directory:

```bash
sudo install -Dm755 src-tauri/target/release/handy /usr/bin/handy
sudo install -d /usr/lib/Handy
sudo cp -a src-tauri/transcribe-libs/. /usr/lib/Handy/
```

Installing the generated `.deb` is preferred because it also places icons,
desktop metadata, resources, and runtime libraries correctly.

This custom Linux build disables upstream update checks. Install newer custom
builds explicitly through their generated `.deb` package.

## Vulkan acceleration

The Linux target enables `transcribe-cpp` dynamic backends and Vulkan.
`glslc` compiles the compute shaders during the build. At runtime Handy tries
the available Vulkan backend and retains CPU backends as a fallback.

Check startup logs for a `ggml_vulkan` device entry. If none appears, confirm
that `vulkaninfo --summary` lists a physical GPU before debugging Handy.

## Shortcuts, suspend, and Wayland

The shortcut manager detects elapsed gaps longer than five seconds and
re-registers its hooks. This is useful after suspend/resume and severe
scheduling delays on both Linux and Windows.

Pop!\_OS may run a Wayland session. Wayland deliberately limits global input
capture. Handy's low-level shortcut mode works on Pop!\_OS after the user joins
the `input` group, but the watchdog does not override compositor security
boundaries on other Wayland desktops. If a shortcut still fails after resume:

1. Restart Handy.
2. Confirm the current session lists `input` in `id -nG`; logging out and back
   in is required after changing group membership.
3. Confirm at least one `/dev/input/event*` node is readable.
4. Try an X11 session to distinguish a Wayland policy limitation.
5. Confirm `wtype` is installed for Wayland text injection.
6. Review the log for shortcut registration or watchdog reset messages.

## Linux overlay behavior

On COSMIC Wayland, converting Tauri's WebKitGTK overlay window into a
layer-shell surface can leave the webview mapped without a submitted pixel
buffer. Handy replaces only that Linux overlay child with a native GTK card.
The normal main interface remains React/WebKitGTK.

The native card:

- follows the mouse pointer to the active monitor when a recording begins;
- maps a fresh invisible 1×1 layer-shell probe at trigger time and assigns the
  visible card to the GDK output COSMIC selected for that probe, avoiding
  unavailable global Wayland pointer coordinates and mixed-DPI conversion;
- anchors at the configured top or bottom edge without taking keyboard focus;
- uses an 82%-opaque dark charcoal surface, a subtle black shadow, and the
  cool-blue interface accent;
- keeps the recording dot, status label, voice-level indicators, and cancel
  control clear of the lower border with explicit bottom padding;
- displays live audio levels while listening;
- changes label for transcription and post-processing;
- displays committed and tentative live text for streaming-capable models;
- clears all transcript text before every new overlay session;
- sizes live mode to 35% of the selected monitor width, clamped to 400–760
  logical pixels; and
- starts at the compact 400-pixel width and uses a 200ms cubic ease-out to
  expand to that monitor-relative width when the first recognized text appears;
- performs this width transition only once per recording session; and
- grows with wrapped transcript text up to half the monitor height, then uses a
  vertical scroller while retaining the cancel button.

`Minimal` remains a compact status card. `Live` expands for incremental text
only when the selected model advertises streaming support; non-streaming models
continue to use the compact recording/transcribing states.

Layer-shell surfaces ignore ordinary window coordinates on Wayland. Handy
therefore asks COSMIC to map a new transparent probe whenever recording starts,
reads that mapped probe's output, assigns the native card to the same monitor,
and destroys the probe immediately. The production card never takes focus.
This was verified at all three logical output origins: `(0,0)`, `(1920,0)`, and
`(3840,0)`. See `docs/2026-08-01-overlay-changes-investigation.md` for the full
diagnosis and regression checklist.

Linux initializes shortcut state during hidden startup and prepares Enigo in
the background. Native Wayland paste methods (`wtype`/`wl-copy`) can proceed
without waiting for the X11 keyboard-map scan, while Enigo remains available
as a fallback after initialization.

The GTK layer-shell overlay can be disabled when a compositor does not support
it:

```bash
HANDY_NO_GTK_LAYER_SHELL=1 handy
```

## Lower output volume while recording

General Settings > Sound contains two related controls:

- **Mute While Recording** fully mutes system output; and
- **Lower Volume While Recording** routes active application playback through a
  temporary PipeWire gain node without changing saved application volumes.

Set the reduction slider to 0% to disable it. Full mute takes precedence when
both settings have values. On Pop!\_OS, Handy uses `pw-loopback`, `pw-dump`,
`pw-link`, and `wpctl` to create a private runtime-only gain path without
touching the master output device. This avoids COSMIC's volume OSD and feedback
sound when recording starts and stops. A cleanup watchdog restores the original
links even if Handy crashes. If those PipeWire tools are unavailable on another
Linux desktop, Handy retains `wpctl`, `pactl`, and `amixer` master-volume
fallbacks; COSMIC deliberately fails open instead of using those fallbacks and
triggering its OSD. Details and verification are recorded in
`docs/2026-08-10-v0.9.5-volume-reduction.md`.

## Troubleshooting

Build only the Debian package if AppImage bundling fails on Ubuntu 24.04 or
newer:

```bash
bun run tauri build --bundles deb --no-sign
```

If Vulkan shader generation fails:

```bash
command -v glslc
sudo apt install glslc
```

If GTK layer shell or WebKitGTK is missing:

```bash
sudo apt install \
  libgtk-layer-shell0 \
  libgtk-layer-shell-dev \
  libwebkit2gtk-4.1-dev
```

If the built application reports a missing shared library, inspect it before
copying anything:

```bash
LD_LIBRARY_PATH=src-tauri/transcribe-libs ldd src-tauri/target/release/handy
```

Runtime libraries generated for Handy should be packaged under
`/usr/lib/Handy/`, not added broadly to `/usr/lib`.

If a package reinstall appears to have made no visual difference, verify the
executable of the running process:

```bash
readlink -f "/proc/$(pgrep -n handy)/exe"
```

It should report `/usr/bin/handy`. A previous user-local installation can
shadow that binary through `~/.local/bin/handy`; repoint the launcher and
restart Handy:

```bash
ln -sfn /usr/bin/handy ~/.local/bin/handy
```

If `Ctrl+Space` does not trigger and the log reports permission denied opening
`/dev/input`, apply the fresh-install group step and start a new login session:

```bash
sudo usermod -aG input "$USER"
```
