# Handy-NPU — Linux Build & Deployment Guide (Pop!_OS / Ubuntu)

> **Branch**: `codex/whisper-npu-v0.9.0` (based on upstream `v0.9.3`)
>
> This document covers everything needed to build and run the custom Handy-NPU
> fork on **Pop!_OS** (and any Ubuntu/Debian-based distro). It lists every
> custom feature, explains which ones carry over to Linux and which are
> Windows-only, and provides copy-paste build instructions.

---

## Table of Contents

1. [Feature Matrix — Linux vs Windows](#1-feature-matrix--linux-vs-windows)
2. [Prerequisites (Pop!_OS / Ubuntu)](#2-prerequisites-popos--ubuntu)
3. [Clone & Build](#3-clone--build)
4. [Install the .deb Package](#4-install-the-deb-package)
5. [Vulkan GPU Acceleration](#5-vulkan-gpu-acceleration)
6. [Custom Feature Details (All Platforms)](#6-custom-feature-details-all-platforms)
7. [Windows-Only Features (Excluded on Linux)](#7-windows-only-features-excluded-on-linux)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Feature Matrix — Linux vs Windows

| Feature | Linux (Pop!_OS) | Windows |
|---|:---:|:---:|
| Local Whisper transcription (CPU) | ✅ | ✅ |
| Vulkan GPU acceleration | ✅ | ✅ |
| Dynamic CPU ISA backends (AVX2, AVX-512, etc.) | ✅ | ✅ |
| Multi-model fallback chain (Priority 1/2/3) | ✅ | ✅ |
| Post-processing reasoning effort control | ✅ | ✅ |
| Dynamic model-switch global hotkeys | ✅ | ✅ |
| CLI `--load-model <name>` switching | ✅ | ✅ |
| Logitech G HUB / macro mouse integration | ✅ (via CLI) | ✅ |
| GTK overlay transcription window | ✅ (native) | N/A (Webview) |
| Sleep/resume watchdog (time-delta) | ✅ | ✅ |
| WTS session lock/unlock watchdog | ❌ (not needed) | ✅ |
| Intel OpenVINO NPU server | ❌ (Windows-only) | ✅ |
| NSIS / MSI installer | ❌ | ✅ |
| `.deb` / `.rpm` / `.AppImage` installer | ✅ | ❌ |

### Why the WTS watchdog isn't needed on Linux

On Windows, locking the screen silently strips low-level keyboard hooks
(without suspending background threads), so a dedicated Win32 WTS API watcher
is required to detect lock→unlock transitions and re-register hooks.

Linux desktop environments (GNOME, KDE, etc.) do **not** strip input hooks on
screen lock. The code compiles a no-op stub for `is_session_locked()` on Linux
(`#[cfg(not(target_os = "windows"))]` → always returns `false`), so this
feature safely compiles out.

---

## 2. Prerequisites (Pop!_OS / Ubuntu)

Pop!_OS is Ubuntu-based, so all Ubuntu/Debian packages apply directly.

### 2.1 System Packages

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
  patchelf
```

### 2.2 Rust (latest stable)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

### 2.3 Bun (JavaScript runtime & package manager)

```bash
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc   # or restart your terminal
```

### 2.4 Vulkan Drivers

**NVIDIA (Pop!_OS ships these pre-installed on the NVIDIA ISO):**

```bash
# Verify Vulkan is working:
vulkaninfo --summary
```

If `vulkaninfo` shows your GPU, you're good. If not:

```bash
# Pop!_OS NVIDIA ISO already includes drivers, but if needed:
sudo apt install nvidia-driver-560   # or the latest available
```

**AMD (Mesa RADV — ships with Pop!_OS):**

```bash
sudo apt install mesa-vulkan-drivers
vulkaninfo --summary
```

**Intel (ANV):**

```bash
sudo apt install mesa-vulkan-drivers intel-gpu-tools
vulkaninfo --summary
```

---

## 3. Clone & Build

### 3.1 Clone the Fork

```bash
git clone https://github.com/<your-fork>/Handy.git Handy-npu
cd Handy-npu
git checkout codex/whisper-npu-v0.9.0
```

> **Note**: Replace `<your-fork>` with the actual repository URL. If working
> from a local copy (e.g., USB transfer from the Windows machine), just `cd`
> into the directory and ensure you're on the correct branch.

### 3.2 Install Frontend Dependencies

```bash
bun install
```

### 3.3 Development Mode

```bash
bun run tauri dev
```

This compiles and launches Handy in development mode with hot-reload.

### 3.4 Production Build (Installers)

```bash
bun run tauri build
```

This produces:
- **Deb package**: `src-tauri/target/release/bundle/deb/Handy_0.9.3_amd64.deb`
- **RPM package**: `src-tauri/target/release/bundle/rpm/Handy-0.9.3-1.x86_64.rpm`
- **AppImage**: `src-tauri/target/release/bundle/appimage/Handy_0.9.3_amd64.AppImage`

> **No `--no-sign` needed on Linux** — the Azure code signing workaround is
> Windows-specific. Linux builds don't have a `signCommand` configured.

> **No `CARGO_TARGET_DIR` workaround needed on Linux** — the Windows
> `MAX_PATH` 260-character limit doesn't exist on Linux.

---

## 4. Install the .deb Package

### Option A: Direct Install (Recommended for Pop!_OS)

```bash
sudo dpkg -i src-tauri/target/release/bundle/deb/Handy_0.9.3_amd64.deb
sudo apt install -f   # resolve any missing dependencies
```

### Option B: Manual Install (from deb extraction)

```bash
cd /tmp
ar x /path/to/Handy_0.9.3_amd64.deb data.tar.gz
tar xzf data.tar.gz
sudo cp usr/bin/handy /usr/bin/
sudo cp -a usr/lib/. /usr/lib/
sudo cp -r usr/share/icons/hicolor/* /usr/share/icons/hicolor/
sudo cp usr/share/applications/Handy.desktop /usr/share/applications/
sudo ldconfig
```

### After Subsequent Rebuilds

Only the binary and runtime libraries need updating:

```bash
sudo cp src-tauri/target/release/handy /usr/bin/
sudo cp -a src-tauri/transcribe-libs/. /usr/lib/
sudo ldconfig
```

---

## 5. Vulkan GPU Acceleration

Vulkan is **enabled by default** on this branch for Linux. The
[Cargo.toml](src-tauri/Cargo.toml) Linux target section is:

```toml
[target.'cfg(target_os = "linux")'.dependencies]
gtk-layer-shell = { version = "0.8", features = ["v0_6"] }
gtk = "0.18"
transcribe-cpp = { version = "0.1.3", default-features = false, features = [
  "dynamic-backends",
  "vulkan",
] }
```

### How It Works

- At build time, the Vulkan shader generator (`vulkan-shaders-gen`) compiles
  GLSL compute shaders to SPIR-V using `glslc` (from the `glslc` package).
- At runtime, Handy loads `libggml-vulkan.so` alongside the CPU backends.
- When a Vulkan-capable GPU is detected, transcription runs on the GPU.
- If no Vulkan GPU is found, Handy transparently falls back to the best
  available CPU backend (dynamic ISA scoring selects AVX-512, AVX2, SSE4.2,
  etc. automatically).

### Verify Vulkan Is Working

After launching Handy, check the terminal output for lines like:

```
ggml_vulkan: Found 1 Vulkan device:
ggml_vulkan: 0 = NVIDIA GeForce RTX 4070 (NVIDIA) | ...
```

---

## 6. Custom Feature Details (All Platforms)

All features below work identically on Linux and Windows unless noted.

### 6.1 Multi-Model Fallback Chain (Priority 1/2/3)

**What it does**: Allows setting up to 3 post-processing AI models in priority
order. If the primary model hits a rate limit (HTTP 429), timeout, or server
error, Handy automatically retries with the next model in the chain.

**Files involved**:
- `src-tauri/src/actions.rs` — Fallback retry loop logic
- `src-tauri/src/settings.rs` — Schema for pipe-delimited model storage
- `src/components/settings/post-processing/PostProcessingSettings.tsx` — 3 priority dropdowns
- `src/components/settings/PostProcessingSettingsApi/usePostProcessProviderState.ts` — State parsing

**Storage format**: Models are stored as `model1|model2|model3` in the existing
`post_process_models` map (pipe-delimited), maintaining backward compatibility
with single-model configs.

**UI**: The Settings → Post-Processing page shows three stacked "Priority 1/2/3"
dropdown selectors. Priority 2 and 3 include a "None (No Fallback)" option.

---

### 6.2 Post-Processing Reasoning Effort Control

**What it does**: Adds a per-provider "reasoning effort" parameter
(`low`/`medium`/`high`/`default`) sent in the AI API request. Setting
`default` omits the field entirely — required for custom gateways like
Console Go (opencode.ai) running DeepSeek that reject unknown parameters.

**Files involved**:
- `src-tauri/src/settings.rs` — `post_process_reasoning_efforts` HashMap
- `src-tauri/src/actions.rs` — Conditional inclusion in JSON payload

---

### 6.3 Dynamic Model Switch Global Hotkeys

**What it does**: Each downloaded Whisper model can have a global keyboard
shortcut assigned. Pressing the shortcut instantly switches the active
transcription model in the background with an audio chime confirmation.

**How to use**:
1. Open Settings → Models
2. Next to each downloaded model, there's an inline hotkey recorder
3. Click the recorder and press your desired key combination
4. The hotkey is now globally registered

**Files involved**:
- `src-tauri/src/shortcut/handler.rs` — Intercepts `model:<id>` binding events
- `src-tauri/src/shortcut/mod.rs` — Registers dynamic `model:*` bindings
- `src/stores/settingsStore.ts` — Dynamic binding construction
- `src/components/onboarding/ModelCard.tsx` — Inline hotkey recorder UI
- `src/components/settings/ShortcutInput.tsx` — `plain` prop for compact layout
- `src/components/settings/GlobalShortcutInput.tsx` — `plain` layout support
- `src/components/settings/HandyKeysShortcutInput.tsx` — `plain` layout support

**Storage**: Hotkeys are stored in `settings.bindings` with the key format
`model:<model_id>` (e.g., `model:ggml-large-v3-turbo-q5_0`).

---

### 6.4 CLI Model Switching (`--load-model`)

**What it does**: Switch the active Whisper model from the command line or a
macro button without opening the UI.

**Usage**:
```bash
# Switch to Whisper V3 Large:
handy --load-model large

# Switch to Parakeet TDT:
handy --load-model parakeet

# Switch to any model by partial name (case-insensitive):
handy --load-model turbo
```

**How it works**: If a Handy instance is already running, the new process
sends the `--load-model` argument to the running instance via the Tauri
single-instance plugin IPC. The running instance scans all downloaded models,
finds the best substring match, switches models, and plays a chime.

**Macro mouse integration on Linux**: You can bind terminal commands to mouse
buttons using your desktop environment's input settings, `xdotool`,
`input-remapper`, or similar tools:

```bash
# Example with input-remapper or a custom script:
handy --load-model large
```

**File**: `src-tauri/src/lib.rs` — Single-instance argument handler

---

### 6.5 Sleep/Resume Watchdog (Time Delta)

**What it does**: Detects system suspend/hibernate by measuring elapsed time
between loop iterations. If the gap exceeds 5 seconds (indicating the OS
suspended the process), the global hotkey hook manager is reset and all
shortcuts are re-registered.

**Works on Linux**: Yes — this is pure Rust (`std::time::Instant`) with no
platform-specific APIs.

**File**: `src-tauri/src/shortcut/handy_keys.rs`

---

## 7. Windows-Only Features (Excluded on Linux)

### 7.1 Intel OpenVINO NPU Server

The embedded Python server (`backend/tools/server/run_whisper_npu_server.py`)
that offloads transcription to an Intel NPU via OpenVINO is **Windows-only**.
The portable Python environment and OpenVINO runtime are compiled for Windows.

On Linux, use the local Whisper model with Vulkan GPU acceleration instead —
it provides comparable or better performance on most GPUs.

### 7.2 WTS Session Lock/Unlock Watchdog

The Win32 `WTSQuerySessionInformationW` API call that detects screen
lock/unlock transitions compiles out on Linux (`#[cfg(target_os = "windows")]`).
A no-op stub is compiled in its place. This is not needed on Linux because
Linux desktop environments don't strip global keyboard hooks on screen lock.

### 7.3 NSIS / MSI Installers

Windows-specific installer formats. Linux uses `.deb`, `.rpm`, or `.AppImage`
instead.

---

## 8. Troubleshooting

### AppImage build fails on Pop!_OS

If the AppImage bundler fails (common on Ubuntu 24.04+), build only the deb:

```bash
bun run tauri build -- --bundles deb
```

### Vulkan shaders fail to compile

Ensure `glslc` is installed and on PATH:

```bash
which glslc
# If missing:
sudo apt install glslc
```

### `libgtk-layer-shell` not found

```bash
sudo apt install libgtk-layer-shell0 libgtk-layer-shell-dev
```

### WebKit2GTK not found

Pop!_OS 22.04+ uses webkit2gtk-4.1:

```bash
sudo apt install libwebkit2gtk-4.1-dev
```

### No Vulkan GPU detected at runtime

```bash
# Check Vulkan support:
vulkaninfo --summary

# For NVIDIA: ensure driver is installed
nvidia-smi

# For AMD: ensure Mesa RADV is installed
sudo apt install mesa-vulkan-drivers
```

### Hotkeys not working after resume from suspend

The sleep watchdog should handle this automatically. If hotkeys still don't
work after resume, restart Handy. Check terminal output for:

```
[watchdog] sleep detected (Δ=...s), resetting hook manager
```

---

## Quick Reference — Complete Build from Scratch

```bash
# 1. Install system dependencies
sudo apt update
sudo apt install -y build-essential pkg-config cmake libssl-dev libasound2-dev \
  libvulkan-dev vulkan-tools glslc spirv-headers glslang-tools \
  libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libgtk-layer-shell0 libgtk-layer-shell-dev patchelf

# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Install Bun
curl -fsSL https://bun.sh/install | bash
source ~/.bashrc

# 4. Clone and build
git clone <repo-url> Handy-npu && cd Handy-npu
git checkout codex/whisper-npu-v0.9.0
bun install
bun run tauri build

# 5. Install
sudo dpkg -i src-tauri/target/release/bundle/deb/Handy_0.9.3_amd64.deb
sudo apt install -f
```
