# AGENTS.md

This file provides guidance to AI coding assistants working with code in this repository.

## Development Commands

**Prerequisites:**

- [Rust](https://rustup.rs/) (latest stable)
- [Bun](https://bun.sh/) package manager

**Core Development:**

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev
# If cmake error on macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

# Build for production
bun run tauri build

# Frontend only development
bun run dev        # Start Vite dev server
bun run build      # Build frontend (TypeScript + Vite)
bun run preview    # Preview built frontend
```

**Linting and Formatting (run before committing):**

```bash
bun run lint              # ESLint for frontend
bun run lint:fix          # ESLint with auto-fix
bun run format            # Prettier + cargo fmt
bun run format:check      # Check formatting without changes
bun run format:frontend   # Prettier only
bun run format:backend    # cargo fmt only
```

**Model Setup (Required for Development):**

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

For detailed platform-specific build setup, see [BUILD.md](BUILD.md).

## Architecture Overview

Handy is a cross-platform desktop speech-to-text application built with Tauri 2.x (Rust backend + React/TypeScript frontend).

### Backend Structure (src-tauri/src/)

- `lib.rs` - Main entry point, Tauri setup, manager initialization, single-instance CLI handling (`--load-model`)
- `managers/` - Core business logic:
  - `audio.rs` - Audio recording and device management
  - `model.rs` - Model downloading and management
  - `transcription.rs` - Speech-to-text processing pipeline (with remote NPU backend bypass)
  - `history.rs` - Transcription history storage
  - `remote_whisper_server.rs` - Intel OpenVINO NPU Whisper server manager
- `audio_toolkit/` - Low-level audio processing:
  - `audio/` - Device enumeration, recording, resampling
  - `vad/` - Voice Activity Detection (Silero VAD)
- `commands/` - Tauri command handlers for frontend communication
- `cli.rs` - CLI argument definitions (clap derive)
- `shortcut/` - Global keyboard shortcut handling:
  - `mod.rs` - Shortcut registration, dynamic `model:<id>` binding management
  - `handler.rs` - Shortcut event dispatch (includes model-switch hotkey interception)
  - `handy_keys.rs` - Low-level hook manager with sleep/lock watchdog threads
- `actions.rs` - Post-processing pipeline with multi-model fallback chain
- `settings.rs` - Application settings management (includes NPU, reasoning effort, priority models)
- `overlay.rs` - Recording overlay window (platform-specific)
- `signal_handle.rs` - `send_transcription_input()` reusable function
- `utils.rs` - Platform detection helpers

### Frontend Structure (src/)

- `App.tsx` - Main component with onboarding flow
- `components/` - React UI components:
  - `settings/` - Settings UI
  - `model-selector/` - Model management interface
  - `onboarding/` - First-run experience
  - `overlay/` - Recording overlay UI
  - `update-checker/` - App update notifications
  - `shared/`, `ui/`, `icons/`, `footer/` - Shared components
- `hooks/useSettings.ts` - Settings state management hook
- `stores/settingsStore.ts` - Zustand store for settings
- `bindings.ts` - Auto-generated Tauri type bindings (via tauri-specta)
- `overlay/` - Recording overlay window entry point
- `lib/types.ts` - Shared TypeScript type definitions

### Key Architecture Patterns

**Manager Pattern:** Core functionality organized into managers (Audio, Model, Transcription) initialized at startup and managed via Tauri state.

**Command-Event Architecture:** Frontend → Backend via Tauri commands; Backend → Frontend via events.

**Pipeline Processing:** Audio → VAD → Whisper/Parakeet → Text output → (optional) Post-Processing with multi-model fallback → Clipboard/Paste

**State Flow:** Zustand → Tauri Command → Rust State → Persistence (tauri-plugin-store)

**Dynamic Hotkey Bindings:** Model-specific hotkeys are stored in `settings.bindings` with the key format `model:<model_id>`. On registration, the shortcut module loops all bindings (not just defaults) to register dynamic shortcuts. On trigger, `handler.rs` intercepts `model:*` binding IDs to switch the active model.

### Technology Stack

**Core Libraries:**

- `transcribe-cpp` - Local Whisper-family inference (GGML/GGUF) with GPU acceleration
- `transcribe-rs` - ONNX speech recognition (Parakeet, Moonshine, SenseVoice, etc.)
- `cpal` - Cross-platform audio I/O
- `vad-rs` - Voice Activity Detection
- `rdev` - Global keyboard shortcuts
- `rubato` - Audio resampling
- `rodio` - Audio playback for feedback sounds

### Application Flow

1. **Initialization:** App starts minimized to tray, loads settings, initializes managers
2. **Model Setup:** First-run downloads preferred Whisper model (Small/Medium/Turbo/Large)
3. **Recording:** Global shortcut triggers audio recording with VAD filtering
4. **Processing:** Audio sent to Whisper model for transcription
5. **Output:** Text pasted to active application via system clipboard

### Settings System

Settings are stored using Tauri's store plugin with reactive updates:

- Keyboard shortcuts (configurable, supports push-to-talk)
- Audio devices (microphone/output selection)
- Model preferences (Small/Medium/Turbo/Large Whisper variants)
- Audio feedback and translation options

### Single Instance Architecture

The app enforces single instance behavior — launching when already running brings the settings window to front rather than creating a new process. Remote control flags (`--toggle-transcription`, etc.) work by launching a second instance that sends args to the running instance via `tauri_plugin_single_instance`, then exits.

## Internationalization (i18n)

All user-facing strings must use i18next translations. ESLint enforces this (no hardcoded strings in JSX).

**Adding new text:**

1. Add key to `src/i18n/locales/en/translation.json`
2. Use in component: `const { t } = useTranslation(); t('key.path')`

**File structure:**

```
src/i18n/
├── index.ts           # i18n setup
├── languages.ts       # Language metadata
└── locales/
    ├── en/translation.json  # English (source)
    ├── de/, es/, fr/, ja/, ru/, zh/, ...
    └── ...
```

For translation contribution guidelines, see [CONTRIBUTING_TRANSLATIONS.md](CONTRIBUTING_TRANSLATIONS.md).

## Code Style

**Rust:**

- Run `cargo fmt` and `cargo clippy` before committing
- Handle errors explicitly (avoid unwrap in production)
- Use descriptive names, add doc comments for public APIs

**TypeScript/React:**

- Strict TypeScript, avoid `any` types
- Functional components with hooks
- Tailwind CSS for styling
- Path aliases: `@/` → `./src/`

## CLI Parameters

Handy supports command-line parameters on all platforms for integration with scripts, window managers, and autostart configurations.

**Implementation:** `cli.rs` (definitions), `main.rs` (parsing), `lib.rs` (applying), `signal_handle.rs` (shared logic)

| Flag                         | Description                                                     |
| ---------------------------- | --------------------------------------------------------------- |
| `--toggle-transcription`     | Toggle recording on/off on a running instance                   |
| `--toggle-post-process`      | Toggle recording with post-processing on/off                    |
| `--cancel`                   | Cancel the current operation on a running instance              |
| `--start-hidden`             | Launch without showing the main window (tray icon visible)      |
| `--no-tray`                  | Launch without system tray (closing window quits the app)       |
| `--debug`                    | Enable debug mode with verbose (Trace) logging                  |
| `--load-model <QUERY>`       | Switch the active model on a running instance (substring match) |

**Key design decisions:**

- CLI flags are runtime-only overrides — they do NOT modify persisted settings
- Remote control flags work via `tauri_plugin_single_instance`: second instance sends args, then exits
- `send_transcription_input()` in `signal_handle.rs` is shared between signal handlers and CLI
- `--load-model` performs a case-insensitive substring match against all downloaded models, switches the active model, and plays a confirmation chime. Useful for Logitech G HUB macro integration.

## Debug Mode

Access debug features: `Cmd+Shift+D` (macOS) or `Ctrl+Shift+D` (Windows/Linux)

## Platform Notes

- **macOS**: Metal acceleration, accessibility permissions required for keyboard shortcuts
- **Windows**: Vulkan acceleration, code signing. See [Windows compilation workaround](#windows-compilation-workaround) below.
- **Linux**: OpenBLAS + Vulkan, limited Wayland support, overlay uses GTK layer shell (disable with `HANDY_NO_GTK_LAYER_SHELL=1`)

## Windows Compilation Workaround

On Windows, the Vulkan shader generator creates deeply nested paths that exceed the 260-character `MAX_PATH` limit. **You must redirect Cargo's target directory** to a short path:

```powershell
# Type-check only:
$env:CARGO_TARGET_DIR="C:\t"; cargo check

# Full release build (skip code signing if no Azure certificate):
$env:CARGO_TARGET_DIR="C:\t"; bun run tauri build --no-sign
```

See [BUILD.md](BUILD.md) for detailed platform-specific instructions.

## Custom Fork Features (Handy-NPU)

This fork adds the following features on top of the upstream Handy codebase:

### Intel OpenVINO NPU Backend
A remote Whisper backend that runs on Intel NPU via OpenVINO GenAI. Configured via Settings → Remote Whisper Backend.
- Backend: `managers/remote_whisper_server.rs`, Python server in `backend/`
- Frontend: `components/settings/RemoteWhisperBackendSettings.tsx`

### Vulkan GPU Acceleration (Windows)
The `vulkan` feature flag is enabled for `transcribe-cpp` on `x86_64-pc-windows-msvc`, adding GPU-accelerated inference via `ggml-vulkan.dll`.

### Sleep/Resume & Session Lock Watchdogs
In `shortcut/handy_keys.rs`, two watchdog mechanisms detect when Windows silently strips low-level keyboard hooks:
1. **Sleep watchdog**: Detects >5s time gaps indicating system suspend/hibernation.
2. **WTS session lock watchdog**: Polls `WTSQuerySessionInformationW` every 1s to detect lock/unlock transitions. Uses byte offset **16** (not 12) for `SessionFlags` due to MSVC 64-bit struct padding alignment.

### Post-Processing Multi-Model Fallback Chain
Three priority model selectors (Priority 1, 2, 3) stored as pipe-delimited strings in `post_process_models`. On API error or rate-limit, the system automatically retries with the next model.
- Backend: `actions.rs` (fallback loop), `settings.rs` (schema)
- Frontend: `PostProcessingSettings.tsx` (3 dropdown selectors), `usePostProcessProviderState.ts` (pipe parsing)

### Dynamic Model Switch Global Hotkeys
Users can assign global keyboard hotkeys to any downloaded model directly from the Models page. Stored in `settings.bindings` as `model:<model_id>`.
- Backend: `shortcut/mod.rs` (registration), `shortcut/handler.rs` (event dispatch)
- Frontend: `ModelCard.tsx` (inline hotkey recorder), `ShortcutInput.tsx` (`plain` prop for inline layout)

### CLI Model Switching (`--load-model`)
Switch the active model from the command line: `handy.exe --load-model large`. Useful for Logitech G HUB macro integration.

## Troubleshooting

See the [Troubleshooting](README.md#troubleshooting) section in README.md.

## GitHub workflow for AI coding assistants

**MANDATORY. Before opening any PR, issue, or discussion in this repo: you MUST read the relevant template file and follow it strictly.** That includes sections that look "ceremonial" — checklists, AI Assistance disclosures, "Human Written Description". A generic Summary/Test-plan layout is not acceptable.

- **Opening a PR:** Read [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md). Every section listed there is mandatory. If a section requires a human-written paragraph (e.g. "Human Written Description"), leave a clear TODO placeholder and ask the human contributor to fill it in — do not invent their voice.
- **Opening an issue:** Read [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/). Blank issues are disabled; pick the right template (`bug_report.md` for bugs). Feature requests do not belong in issues — they go to [Discussions](https://github.com/cjpais/Handy/discussions) (see `.github/ISSUE_TEMPLATE/config.yml`).
- **Proposing a feature:** Handy is under a feature freeze. New features require community support gathered in [Discussions](https://github.com/cjpais/Handy/discussions) before any PR is opened — see the PR template's "Community Feedback" section.
- **Translations:** Follow [CONTRIBUTING_TRANSLATIONS.md](CONTRIBUTING_TRANSLATIONS.md).
- **Full contributor workflow:** [CONTRIBUTING.md](CONTRIBUTING.md).

**Commits:** Use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`). Focus the message on _why_, not _what_.
