# Pop!\_OS Linux Migration Design

## Goal

Convert the existing Windows-origin Handy working copy into a clean Pop!\_OS
development and build workspace while preserving the Linux-compatible custom
features already present in the repository.

## Preserved functionality

- Local transcription with dynamic CPU backends and Vulkan acceleration.
- Multi-model post-processing fallback.
- Post-processing reasoning-effort selection.
- Dynamic model-switch global shortcuts.
- CLI model switching through `--load-model`.
- Best-effort sleep/resume shortcut recovery on Linux.
- Native GTK layer-shell overlay with streaming text support.
- Cool-blue Linux tray states matching the dark interface.
- Upstream update checks disabled to preserve the custom Linux fork.
- Linux packaging and app-private runtime-library handling.

The removed Intel OpenVINO NPU backend is intentionally out of scope because it
was Windows-only and is no longer required.

## Cleanup scope

- Remove generated Windows DLLs and stale build output.
- Remove and reinstall project dependencies created by the Windows environment.
- Preserve tracked Windows source, installer configuration, and NSIS assets;
  platform selection excludes them from Linux builds.
- Normalize Windows CRLF line endings to Linux LF without changing file content.
- Retain platform-gated shared source where removing it could destabilize the
  application; the Rust compiler excludes those sections from Linux builds.
- Do not read, modify, or delete anything outside the Handy project except for
  installing explicitly required Pop!\_OS system packages.

## Dependency locations

- Install native compiler and development packages with Pop!\_OS/Ubuntu `apt`,
  which places managed headers and libraries in the distribution's standard
  system locations.
- Keep Rust under the current user's rustup-managed directory.
- Install Bun once in its standard user-managed location.
- Install frontend packages into the project's `node_modules` directory.
- Keep Cargo build output in `src-tauri/target`; do not use the Windows
  short-path target-directory workaround.

The system packages are shared by all projects. Bun and Cargo retain shared
download caches, while each project keeps its own resolved dependency tree and
build output so incompatible project versions do not conflict.

## Linux shortcut recovery

- Compile the Windows WTS lock/unlock watcher only on Windows.
- Retain the platform-neutral elapsed-time recovery on Linux. It recreates the
  shortcut manager after a long scheduler gap such as suspend/resume.
- Describe this as best-effort recovery. It does not bypass Wayland's
  restrictions on application-owned global shortcuts.
- Update Windows-specific comments around the elapsed-time check so the source
  accurately describes its Linux behavior.

## Visual theme

- Preserve the current layout and component structure.
- Make the application dark themed by default.
- Replace the pink/orange emphasis with a restrained charcoal palette, neutral
  dark surfaces, readable off-white text, and a subtle cool accent.
- Apply the palette through the shared theme tokens so the main window,
  controls, icons, and recording overlay remain visually consistent.
- Tint the Linux colored tray state family to the same cool-blue accent without
  changing the source icon silhouettes.
- Keep the recording dot, status label, voice indicators, and cancel control
  visually separated from the overlay's lower border.
- On COSMIC Wayland, select the native layer-shell output through the existing
  Tauri/Enigo cursor monitor and map it to GDK by logical geometry; do not rely
  on GDK's unavailable global pointer coordinates.
- Clear live transcript state at the start of every overlay session. Size live
  mode to 35% of the selected output width (400–760 logical pixels), grow with
  wrapped text up to half the output height, and scroll beyond that limit.
- Preserve accessible contrast and visible focus, hover, warning, and error
  states.

## Documentation

Update `LINUX.md` and related Linux build instructions to:

- reflect version 0.9.4;
- remove Windows NPU guidance;
- use wildcard package filenames where practical;
- install app-private runtime libraries under `/usr/lib/Handy`;
- document Pop!\_OS dependencies, shared installation locations, shortcut
  recovery, and Wayland limitations accurately.
- document the one-time `input` group permission needed by low-level global
  shortcuts on a fresh installation.

## Verification

1. Confirm generated Windows artifacts are absent.
2. Confirm Linux build configuration still enables Vulkan and dynamic backends.
3. Install frontend dependencies.
4. Run formatting checks, frontend lint/type checks, and Rust checks.
5. Verify the main window and recording overlay use the shared dark palette.
6. Build a Linux `.deb` bundle.
7. Inspect the bundle for the executable, resources, and app-private runtime
   libraries.

The application will not be installed system-wide unless the build succeeds.
