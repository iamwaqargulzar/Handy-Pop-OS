# Handy-NPU — Complete Windows Integration & Replication Guide

> **Target Version**: `v0.9.4` (Branch: `codex/whisper-npu-v0.9.0`)  
> **Purpose**: This self-contained document contains **every single code change, fix, workaround, and setup step** required to rebuild this custom Handy-NPU fork on a freshly formatted Windows PC. An AI agent or developer can follow this guide end-to-end to recreate the entire codebase without needing to debug or re-discover any issues.

---

## Table of Contents

1. [Prerequisites & Fresh PC Setup](#1-prerequisites--fresh-pc-setup)
2. [Critical Windows Compilation Workarounds (MUST KNOW)](#2-critical-windows-compilation-workarounds-must-know)
3. [Feature 1: Intel OpenVINO Whisper NPU Integration](#3-feature-1-intel-openvino-whisper-npu-integration)
4. [Feature 2: Vulkan GPU Acceleration for Local Models](#4-feature-2-vulkan-gpu-acceleration-for-local-models)
5. [Feature 3: Sleep & WTS Session Lock/Unlock Watchdogs](#5-feature-3-sleep--wts-session-lockunlock-watchdogs)
6. [Feature 4: Multi-Model Fallback Chain & Reasoning Effort Control](#6-feature-4-multi-model-fallback-chain--reasoning-effort-control)
7. [Feature 5: Dynamic Model-Switch Hotkeys & CLI Macro Support](#7-feature-5-dynamic-model-switch-hotkeys--cli-macro-support)
8. [Feature 6: Installer Process Management (NSIS)](#8-feature-6-installer-process-management-nsis)
9. [Step-by-Step Build & Installer Packaging Script](#9-step-by-step-build--installer-packaging-script)
10. [Logitech G HUB / Mouse Macro Setup](#10-logitech-g-hub--mouse-macro-setup)

---

## 1. Prerequisites & Fresh PC Setup

On a newly installed Windows 10/11 system, install the required toolchains:

### 1.1 Command-Line Tools (Run in PowerShell as Administrator)

```powershell
# 1. Install Git, CMake, and Vulkan SDK via winget
winget install Git.Git
winget install Kitware.CMake
winget install KhronosGroup.VulkanSDK

# 2. Install Visual Studio Build Tools 2022 (Desktop development with C++)
winget install Microsoft.VisualStudio.2022.BuildTools --override "--passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

_Re-open PowerShell after installation so `VULKAN_SDK` and `cmake` environment variables take effect._

### 1.2 Install Rust & Bun

```powershell
# 3. Install Rust (x86_64-pc-windows-msvc)
# Download from https://rustup.rs/ or run:
invoke-web-request -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
& "$env:TEMP\rustup-init.exe" -y --default-host x86_64-pc-windows-msvc

# 4. Install Bun (JavaScript runtime & package manager)
powershell -c "irm bun.sh/install.ps1 | iex"
```

---

## 2. Critical Windows Compilation Workarounds (MUST KNOW)

When building on Windows, two OS-level obstacles **will cause builds to fail** unless these exact flags are used:

### Workaround A: Windows `MAX_PATH` 260-Character Limit

- **Root Cause**: During `cargo build`, the nested `vulkan-shaders-gen` CMake build tree inside `transcribe-cpp-sys` creates deep directory paths (`target/release/build/transcribe-cpp-sys-<hash>/out/build/...`). This exceeds Windows' 260-character limit, causing `MSB3491`, `FTK1011`, or `CL.exe` errors.
- **The Fix**: Force Cargo to output build artifacts in a short root path like `C:\t`:
  ```powershell
  $env:CARGO_TARGET_DIR = "C:\t"
  ```

### Workaround B: Azure Code Signing Bypass

- **Root Cause**: Upstream `tauri.conf.json` defines a custom `signCommand` (`trusted-signing-cli`) tied to an Azure subscription. Without the developer's Azure credentials, `tauri build` fails with `program not found` during installer creation.
- **The Fix**: Append the `--no-sign` flag:
  ```powershell
  bun run tauri build --no-sign
  ```

---

## 3. Feature 1: Intel OpenVINO Whisper NPU Integration

Offloads Whisper speech recognition to an Intel NPU (Neural Processing Unit) via OpenVINO.

### 3.1 Embedded Python Environment Setup Script

Create script `backend/setup_python_embed.ps1`:

- Downloads portable Windows Python embeddable zip (`python-3.11.x-embed-amd64.zip`)
- Extracts to `./backend/python_embed`
- Enables `import site` in `python311._pth`
- Installs `pip` and requirements (`openvino-genai`, `numpy`)

### 3.2 Python NPU Server Implementation

Create `backend/tools/server/run_whisper_npu_server.py`:

- Listens on `http://127.0.0.1:44441` (or user-configured port)
- Accepts WAV audio via HTTP `POST /transcribe`
- Uses `openvino_genai.WhisperPipeline` targeting `device="NPU"`
- Default models: `OpenVINO/whisper-large-v3-fp16-ov` and `OpenVINO/whisper-large-v3-turbo-fp16-ov`

### 3.3 Rust Backend Integration

- **`src-tauri/src/settings.rs`**: Add fields to `AppSettings`:
  - `transcription_backend: String` (`"local"` vs `"remote"`)
  - `whisper_npu_server_url: String` (`"http://127.0.0.1:44441"`)
  - `whisper_npu_model_override: String`
  - `whisper_npu_timeout_seconds: u64`
- **`src-tauri/src/managers/remote_whisper_server.rs`**: Sends WAV audio buffer via `reqwest` HTTP POST to NPU server.
- **`src-tauri/src/managers/transcription.rs`**: Intercepts transcription requests. If backend is `"remote"`, bypasses local `.bin` checks and forwards to `remote_whisper_server.rs`.
- **`src/components/settings/RemoteWhisperBackendSettings.tsx`**: UI pane in settings to start/stop the NPU server and configure URL/models.

---

## 4. Feature 2: Vulkan GPU Acceleration for Local Models

Enables GPU-accelerated Whisper inference via Vulkan compute shaders.

### 4.1 `src-tauri/Cargo.toml` Configuration

Ensure `vulkan` and `dynamic-backends` feature flags are enabled under `transcribe-cpp`:

```toml
[target.'cfg(all(windows, target_arch = "x86_64"))'.dependencies]
transcribe-cpp = { version = "0.1.3", default-features = false, features = [
  "dynamic-backends",
  "vulkan",
] }

[target.'cfg(target_os = "linux")'.dependencies]
transcribe-cpp = { version = "0.1.3", default-features = false, features = [
  "dynamic-backends",
  "vulkan",
] }
```

### 4.2 Dynamic Backend Staging

During build, Cargo compiles:

- `ggml-vulkan.dll` (Vulkan compute shader backend)
- CPU ISA variants (`ggml-cpu-alderlake.dll`, `ggml-cpu-skylakex.dll`, `ggml-cpu-haswell.dll`, etc.)

These DLLs are automatically staged into `src-tauri/transcribe-libs/` and bundled alongside `handy.exe`.

---

## 5. Feature 3: Sleep & WTS Session Lock/Unlock Watchdogs

Fixes global keyboard shortcuts breaking after PC sleep/hibernate or screen locking.

### 5.1 Sleep Watchdog (Time Delta)

In `src-tauri/src/shortcut/handy_keys.rs`:

- Tracks `last_tick: std::time::Instant`.
- If `last_tick.elapsed() > 5 seconds`, the OS has suspended/resumed the PC.
- Drops the stale hook manager and re-registers all hotkeys.

### 5.2 Win32 WTS Session Lock/Unlock Watchdog

In `src-tauri/src/shortcut/handy_keys.rs`:

- Calls Win32 `WTSQuerySessionInformationW` API every 1 second to inspect session state flags.
- **CRITICAL MSVC 64-bit Alignment Fix**:
  - In 64-bit MSVC C++ layout, `WTSINFOEXW` contains a 4-byte `Level` field followed by 8-byte aligned time structures.
  - MSVC inserts **4 bytes of padding** between `Level` and the Level 1 struct.
  - `SessionFlags` must be read at byte offset **16** (not 12!). Offset 12 reads `SessionState` instead of `SessionFlags`, resulting in a false-locked status `0`.
- Session flag `0` = Locked (`WTS_SESSIONSTATE_LOCK`), `1` = Unlocked (`WTS_SESSIONSTATE_UNLOCK`).
- On transition `0 -> 1` (user unlocks PC), Handy immediately recreates `HotkeyManager` and restores all registered shortcuts.

---

## 6. Feature 4: Multi-Model Fallback Chain & Reasoning Effort Control

### 6.1 Multi-Model Fallback Chain (Priority 1/2/3)

Allows specifying up to 3 post-processing AI models in priority order. If Priority 1 hits rate-limits (HTTP 429), timeouts, or server errors, Handy automatically fails over to Priority 2, then Priority 3.

- **`src-tauri/src/settings.rs`**: Stores models as pipe-delimited string `model1|model2|model3` inside existing `post_process_models` map (100% backward compatible with single model settings).
- **`src-tauri/src/actions.rs`**: Splits string by `|` and executes a retry loop over all configured models until success.
- **`src/components/settings/PostProcessingSettingsApi/usePostProcessProviderState.ts`**: Parses pipe-delimited string into `modelPriority1`, `modelPriority2`, `modelPriority3`.
- **`src/components/settings/post-processing/PostProcessingSettings.tsx`**: Renders 3 stacked dropdown selectors for Priority 1, Priority 2, and Priority 3 (with "None (No Fallback)" option).

### 6.2 Reasoning Effort Control

Adds a per-provider reasoning effort parameter (`low`, `medium`, `high`, `default`).

- **`src-tauri/src/settings.rs`**: Adds `post_process_reasoning_efforts: HashMap<String, String>`.
- **`src-tauri/src/actions.rs`**: If set to `"default"`, reasoning is omitted from payload. (Prevents 400 Bad Request on gateways like Console Go / opencode.ai).

---

## 7. Feature 5: Dynamic Model-Switch Hotkeys & CLI Macro Support

### 7.1 Dynamic Hotkeys

- **`src-tauri/src/shortcut/mod.rs`**: Scans `settings.bindings` for keys starting with `model:<model_id>` and registers global hooks.
- **`src-tauri/src/shortcut/handler.rs`**: Intercepts `model:<id>` binding events and calls `switch_active_model(model_id)`, playing a chime on switch.
- **`src/components/settings/ShortcutInput.tsx`**: Added `plain?: boolean` prop to render compact hotkey recording buttons without extra label wrapping.
- **`src/components/onboarding/ModelCard.tsx`**: Embedded inline hotkey recorder next to each downloaded model card.

### 7.2 CLI Model Switching (`--load-model`)

- **`src-tauri/src/lib.rs`**: Added CLI argument handler inside Tauri single-instance plugin:
  ```bash
  handy.exe --load-model large
  handy.exe --load-model parakeet
  ```
- Performs case-insensitive substring match against downloaded models and switches active model instantly.

---

## 8. Feature 6: Installer Process Management (NSIS)

In `src-tauri/nsis/installer.nsi`:
Added pre-install and pre-uninstall hooks using `nsis_tauri_utils` / `KillProc` targeting:

- `handy.exe`
- `python.exe` (running inside `./backend/python_embed`)

This prevents installer file lock failures (`file in use`) when upgrading or reinstalling.

---

## 9. Step-by-Step Build & Installer Packaging Script

Run these exact commands on your fresh Windows setup to pull, build, and package:

```powershell
# 1. Clone your repository
git clone https://github.com/<your-username>/Handy.git Handy-npu
cd Handy-npu
git checkout codex/whisper-npu-v0.9.0

# 2. Install JavaScript dependencies
bun install

# 3. Test frontend compilation
bun run build

# 4. Clean old installer outputs
Remove-Item -Path "c:\Users\Administrator\Documents\projects\mine\handy\installers\*" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "C:\t\release\bundle\msi\*" -Force -ErrorAction SilentlyContinue
Remove-Item -Path "C:\t\release\bundle\nsis\*" -Force -ErrorAction SilentlyContinue

# 5. Compile release binary & build installers (MAX_PATH + --no-sign workarounds)
$env:CARGO_TARGET_DIR = "C:\t"
bun run tauri build --no-sign

# 6. Copy final installers to project installers directory
Copy-Item -Path "C:\t\release\bundle\nsis\Handy_0.9.4_x64-setup.exe" -Destination ".\installers\" -Force
Copy-Item -Path "C:\t\release\bundle\msi\Handy_0.9.4_x64_en-US.msi" -Destination ".\installers\" -Force

# 7. Verify generated installers
Get-ChildItem -Path ".\installers"
```

_Resulting installers will be located in `.\installers\`._

---

## 10. Logitech G HUB / Mouse Macro Setup

To assign model switching to your mouse buttons (e.g., Logitech G502):

### Method A: Keyboard Shortcuts (Recommended)

1. Open Handy → Settings → Models.
2. Record a hotkey for your desired model (e.g., `Ctrl+Alt+F1` for Large, `Ctrl+Alt+F2` for Parakeet).
3. In **Logitech G HUB**, select your mouse → **Assignments** → **Keys** or **Shortcuts**.
4. Bind `Ctrl+Alt+F1` to G7 and `Ctrl+Alt+F2` to G8.

### Method B: Application CLI Arguments

1. In **Logitech G HUB**, go to **Assignments** → **Macros**.
2. Create a new **NO REPEAT** macro.
3. Choose **Launch Application** → Select `C:\Program Files\Handy\handy.exe`.
4. Pass arguments: `--load-model large` (or `--load-model parakeet`).
5. Assign macro to your G-button.
