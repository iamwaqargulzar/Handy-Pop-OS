# Pop!\_OS Linux Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a clean, dark-themed Pop!\_OS build of Handy that preserves the Linux-compatible custom transcription, post-processing, shortcut, and CLI behavior.

**Architecture:** Retain the existing Tauri/React application, its platform-gated shared code, and tracked Windows packaging configuration. Clean only generated Windows state, install reusable native dependencies through Pop!\_OS, isolate the Windows WTS watcher while retaining platform-neutral suspend recovery, and drive the visual update through shared CSS tokens.

**Tech Stack:** Pop!\_OS 24.04, apt, Rust stable, Bun, Tauri 2, React/TypeScript, Tailwind CSS, GTK/WebKitGTK, Vulkan, Bun tests, Cargo tests.

## Global Constraints

- Work only inside `/home/waqar/Documents/projects/mine/Handy`, except for installing explicitly required Pop!\_OS packages and per-user Rust/Bun tooling.
- Preserve all Linux-compatible custom application features.
- Do not restore the Windows-only Intel OpenVINO NPU backend.
- Install reusable native dependencies through `apt`; keep Bun and Rust user-managed.
- Preserve project-specific resolved dependencies and build outputs.
- Preserve the layout and component structure.
- Use a dark charcoal palette with neutral surfaces, readable off-white text, and a subtle cool accent.
- Treat Linux sleep/resume shortcut recovery as best-effort and do not claim that it bypasses Wayland restrictions.

---

### Task 1: Clean the Windows-origin workspace

**Files:**

- Delete generated: `node_modules/`
- Delete generated: `dist/`
- Delete generated: `src-tauri/target/`
- Delete generated: `src-tauri/transcribe-libs/`

**Interfaces:**

- Consumes: the committed custom Handy source and design specification.
- Produces: an LF-normalized Linux working tree with no Windows binaries or installer assets.

- [ ] **Step 1: Prove the apparent tracked-file changes are line-ending-only**

Run:

```bash
git diff --quiet --ignore-space-at-eol -- .
```

Expected: exit code `0`. Stop if it is nonzero because that would indicate uncommitted semantic work.

- [ ] **Step 2: Restore committed LF content**

Run:

```bash
git restore --worktree -- .
```

Expected: the mass CRLF-only modifications disappear from `git status --short`.

- [ ] **Step 3: Remove resolved generated directories**

After confirming each exact path is inside the repository, remove:

```text
/home/waqar/Documents/projects/mine/Handy/node_modules
/home/waqar/Documents/projects/mine/Handy/dist
/home/waqar/Documents/projects/mine/Handy/src-tauri/target
/home/waqar/Documents/projects/mine/Handy/src-tauri/transcribe-libs
```

Expected: no `.dll`, `.exe`, `.msi`, Windows-generated frontend output, or stale Cargo output remains.

- [ ] **Step 4: Preserve tracked platform configuration**

Confirm `src-tauri/tauri.windows.conf.json`, `src-tauri/nsis/installer.nsi`,
and the `bundle.windows` object in `src-tauri/tauri.conf.json` remain tracked.
Rust and Tauri will ignore these Windows-only paths during the Linux build.

- [ ] **Step 5: Verify cleanup**

Run:

```bash
find node_modules dist src-tauri/target src-tauri/transcribe-libs \
  -type f \( -iname '*.dll' -o -iname '*.exe' -o -iname '*.msi' \) \
  -print 2>/dev/null
git diff --check
```

Expected: the generated-artifact search returns no files and `git diff --check` passes.

- [ ] **Step 6: Confirm tracked Windows files remain unchanged**

```bash
git diff --exit-code -- src-tauri/tauri.conf.json \
  src-tauri/tauri.windows.conf.json src-tauri/nsis/installer.nsi
```

Expected: exit code `0`; generated cleanup requires no source commit.

### Task 2: Install reusable Pop!\_OS build dependencies

**Files:**

- Verify: `src-tauri/resources/models/silero_vad_v4.onnx`

**Interfaces:**

- Consumes: Pop!\_OS 24.04 package repositories and Bun's standard user installer.
- Produces: globally shared native build dependencies plus one per-user Bun installation.

- [ ] **Step 1: Install distribution-managed native dependencies**

Run:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config cmake libssl-dev libasound2-dev \
  libvulkan-dev vulkan-tools glslc spirv-headers glslang-tools \
  libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev libgtk-layer-shell0 libgtk-layer-shell-dev \
  libopenblas-dev patchelf xdg-utils wtype
```

Expected: apt reports every package installed. These packages live in distribution-managed system locations and are reusable by other projects.

- [ ] **Step 2: Install Bun once for the current user**

Run only because `bun` is currently missing:

```bash
curl -fsSL https://bun.sh/install | bash
```

Then expose `~/.bun/bin` in the current process and confirm `bun --version`. Do not install a second copy inside the repository.

- [ ] **Step 3: Verify native tooling**

Run:

```bash
rustc --version
cargo --version
bun --version
cmake --version
pkg-config --version
glslc --version
vulkaninfo --summary
```

Expected: every command succeeds and `vulkaninfo` lists at least one Vulkan-capable device.

- [ ] **Step 4: Verify the required VAD resource**

Run:

```bash
test -s src-tauri/resources/models/silero_vad_v4.onnx
```

Expected: success; the existing nonempty model is reused.

- [ ] **Step 5: Install project-resolved frontend dependencies**

Run:

```bash
bun install
```

Expected: dependencies resolve into project-local `node_modules`, with downloads reused from Bun's user cache.

### Task 3: Make shortcut recovery accurately platform-aware

**Files:**

- Modify: `src-tauri/src/shortcut/handy_keys.rs`
- Test: `src-tauri/src/shortcut/handy_keys.rs`

**Interfaces:**

- Consumes: `std::time::Duration` between shortcut-manager loop iterations.
- Produces: `should_recover_after_gap(elapsed: Duration) -> bool` and Windows-only WTS polling.

- [ ] **Step 1: Write failing threshold tests**

Add unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::should_recover_after_gap;
    use std::time::Duration;

    #[test]
    fn recovery_starts_only_after_five_seconds() {
        assert!(!should_recover_after_gap(Duration::from_secs(5)));
        assert!(should_recover_after_gap(Duration::from_secs(5) + Duration::from_millis(1)));
    }
}
```

- [ ] **Step 2: Verify the new test fails**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml shortcut::handy_keys::tests::recovery_starts_only_after_five_seconds
```

Expected: compilation fails because `should_recover_after_gap` does not exist.

- [ ] **Step 3: Implement the platform-neutral decision function**

Add:

```rust
fn should_recover_after_gap(elapsed: std::time::Duration) -> bool {
    elapsed > std::time::Duration::from_secs(5)
}
```

Use it in the manager loop. Wrap WTS-specific state and the once-per-second session-lock block in `#[cfg(target_os = "windows")]`. Retain elapsed-time recovery on Linux and rewrite its comments to describe suspend, resume, or severe scheduling delay without claiming Linux hooks are always removed.

- [ ] **Step 4: Run focused and full Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml shortcut::handy_keys::tests::recovery_starts_only_after_five_seconds
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: both commands pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shortcut/handy_keys.rs
git commit -m "fix: scope shortcut recovery to platform behavior"
```

### Task 4: Replace the warm palette with a dark neutral theme

**Files:**

- Create: `tests/theme-tokens.test.ts`
- Modify: `src/styles/theme.css`
- Modify: `src/App.css`
- Modify: `src/lib/utils/theme.ts`
- Modify: `src-tauri/src/settings.rs`
- Modify: `src/overlay/RecordingOverlay.css`
- Modify: `src/components/icons/MicrophoneIcon.tsx`
- Modify: `src/components/icons/TranscriptionIcon.tsx`
- Modify: `src/components/icons/CancelIcon.tsx`
- Modify: `src/components/icons/HandyTextLogo.tsx`
- Modify: `src/components/ui/AudioPlayer.tsx`
- Modify: `src/components/model-selector/ModelStatusButton.tsx`

**Interfaces:**

- Consumes: shared CSS custom properties and persisted `Theme`.
- Produces: a dark default palette shared by the main window, controls, icons, and overlay.

- [ ] **Step 1: Write a failing palette test**

Create `tests/theme-tokens.test.ts`:

```typescript
import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const theme = readFileSync("src/styles/theme.css", "utf8").toLowerCase();

test("uses the approved dark neutral palette without legacy pink", () => {
  expect(theme).toContain("--dark-color-background: #101216");
  expect(theme).toContain("--dark-color-text: #e6eaf2");
  expect(theme).toContain("--dark-color-logo-primary: #7aa2f7");
  expect(theme).toContain("--color-background-ui: #5f7fd6");
  expect(theme).not.toMatch(/#faa2ca|#f28cbb|#da5893|#fad1ed/);
});
```

- [ ] **Step 2: Verify the palette test fails**

Run:

```bash
bun test tests/theme-tokens.test.ts
```

Expected: failure because the current palette contains the legacy pink values.

- [ ] **Step 3: Implement the shared palette**

Use these exact dark tokens:

```css
--dark-color-text: #e6eaf2;
--dark-color-background: #101216;
--dark-color-logo-primary: #7aa2f7;
--dark-color-logo-stroke: #b7c5e0;
--color-background-ui: #5f7fd6;
--color-mid-gray: #7f8796;
```

Make the active root palette default to the dark variables. Keep the light option functional but neutralize its pink accent. Update comments in `App.css` and `RecordingOverlay.css` so they refer to a cool accent rather than pink or warm tint.

- [ ] **Step 4: Make dark the application default**

Change Rust's `default_theme()` to return `Theme::Dark`. Change the frontend fallback values in `getStoredTheme()` and `syncThemeFromSettings()` from `"system"` to `"dark"` while retaining all three user-selectable theme options.

- [ ] **Step 5: Remove hardcoded warm accents**

Make the three small icon components inherit `currentColor`, replace the AudioPlayer progress pink with `var(--color-logo-primary)`, change the hardcoded text-logo fill to the shared logo token, and replace the two `bg-orange-400` loading states with `bg-logo-primary`.

- [ ] **Step 6: Run theme and frontend verification**

Run:

```bash
bun test tests/theme-tokens.test.ts
bun run lint
bun run build
```

Expected: the palette test, lint, TypeScript compilation, and Vite build pass.

- [ ] **Step 7: Commit**

```bash
git add tests/theme-tokens.test.ts src/styles/theme.css src/App.css src/lib/utils/theme.ts src-tauri/src/settings.rs src/overlay/RecordingOverlay.css src/components/icons src/components/ui/AudioPlayer.tsx src/components/model-selector/ModelStatusButton.tsx
git commit -m "feat: adopt a dark neutral interface"
```

### Task 5: Correct and consolidate Linux documentation

**Files:**

- Modify: `LINUX.md`
- Modify: `BUILD.md`
- Modify: `AGENTS.md`

**Interfaces:**

- Consumes: the verified Pop!\_OS dependency list and current version `0.9.4`.
- Produces: accurate Linux build, packaging, runtime, Wayland, and shortcut-recovery instructions.

- [ ] **Step 1: Update stale version and package paths**

Replace fixed `0.9.3` artifact names with `0.9.4` or safe wildcards such as `Handy_*_amd64.deb`. Remove branch language that incorrectly says the current code is based only on upstream `v0.9.3`.

- [ ] **Step 2: Correct installation and dependency guidance**

Document apt-managed native dependencies as system-wide reusable packages, Bun/Rust as per-user tools, and frontend/Cargo outputs as project-specific. Copy refreshed runtime libraries only to `/usr/lib/Handy`, never directly into `/usr/lib`.

- [ ] **Step 3: Remove obsolete NPU and Windows claims**

Remove the absent OpenVINO server from the Linux guide. Describe WTS as Windows-only, elapsed-gap recovery as best-effort on Linux, and Wayland shortcut limitations accurately.

- [ ] **Step 4: Validate documentation**

Run:

```bash
rg -n "0\\.9\\.3|remote_whisper|OpenVINO|/usr/lib/$|nvidia-driver-560" LINUX.md BUILD.md AGENTS.md
git diff --check -- LINUX.md BUILD.md AGENTS.md
```

Expected: no stale Linux instructions and no whitespace errors.

- [ ] **Step 5: Commit**

```bash
git add LINUX.md BUILD.md AGENTS.md
git commit -m "docs: align Linux setup with Pop!_OS build"
```

### Task 6: Build and audit the Pop!\_OS package

**Files:**

- Generate: `src-tauri/target/release/bundle/deb/*.deb`

**Interfaces:**

- Consumes: cleaned source, installed native dependencies, project dependencies, VAD resource, and Linux packaging configuration.
- Produces: an audited Pop!\_OS-compatible `.deb` bundle.

- [ ] **Step 1: Run formatting and static checks**

Run:

```bash
bun run format:check
bun run lint
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all commands pass.

- [ ] **Step 2: Build only the requested Linux package**

Run:

```bash
bun run tauri build --bundles deb --no-sign
```

Expected: Tauri creates a `.deb` under `src-tauri/target/release/bundle/deb/`.

- [ ] **Step 3: Audit package contents**

Extract the generated package into a new temporary directory and verify:

```text
usr/bin/handy
usr/lib/Handy/
usr/share/applications/
usr/share/icons/
```

Confirm `usr/lib/Handy` contains the staged transcription runtime and CPU/Vulkan backend libraries, and confirm no `.dll`, `.exe`, or NSIS files are packaged.

- [ ] **Step 4: Smoke-check dynamic dependencies**

Run `ldd` against the extracted `usr/bin/handy` using the extracted app-private library directory in `LD_LIBRARY_PATH`. Expected: no `not found` dependencies.

- [ ] **Step 5: Report the artifact**

Provide the absolute `.deb` path, package size, verification results, and any remaining runtime caveat. Do not install the package system-wide unless separately requested.
