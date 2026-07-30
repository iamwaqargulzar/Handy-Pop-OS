# Handy — Pop!\_OS Build and Deployment Guide

This guide covers the current Handy `0.9.4` source tree on Pop!\_OS 24.04 and
other Ubuntu/Debian-based distributions. The retired Windows-only Intel NPU
experiment is not part of this Linux build.

Tracked Windows source and installer configuration may remain in the
repository. Rust and Tauri select platform-specific code at build time, so
those files do not interfere with a Linux build. Generated directories such
as `node_modules/`, `dist/`, `src-tauri/target/`, and
`src-tauri/transcribe-libs/` can be regenerated and should not be copied
between operating systems.

## Supported custom behavior

The Linux build retains:

- Local transcription with CPU and Vulkan backends
- Three-level post-processing model fallback
- Per-provider reasoning-effort settings
- Global hotkeys for switching downloaded models
- CLI model switching with `handy --load-model <query>`
- A dark neutral interface with a cool accent
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
capture, so shortcut behavior depends on the compositor and the selected
keyboard implementation. The watchdog improves recovery but does not override
those security boundaries. If a shortcut still fails after resume:

1. Restart Handy.
2. Try an X11 session to distinguish a Wayland policy limitation.
3. Confirm `wtype` is installed for Wayland text injection.
4. Review the terminal log for the watchdog reset message.

The GTK layer-shell overlay can be disabled when a compositor does not support
it:

```bash
HANDY_NO_GTK_LAYER_SHELL=1 handy
```

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
