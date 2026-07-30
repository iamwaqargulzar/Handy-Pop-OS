# Pop!_OS Linux Migration Design

## Goal

Convert the existing Windows-origin Handy working copy into a clean Pop!_OS
development and build workspace while preserving the Linux-compatible custom
features already present in the repository.

## Preserved functionality

- Local transcription with dynamic CPU backends and Vulkan acceleration.
- Multi-model post-processing fallback.
- Post-processing reasoning-effort selection.
- Dynamic model-switch global shortcuts.
- CLI model switching through `--load-model`.
- Sleep/resume shortcut recovery where supported on Linux.
- Linux overlay, packaging, and runtime-library handling.

The removed Intel OpenVINO NPU backend is intentionally out of scope because it
was Windows-only and is no longer required.

## Cleanup scope

- Remove generated Windows DLLs and stale build output.
- Remove and reinstall project dependencies created by the Windows environment.
- Remove Windows-only installer configuration and NSIS assets.
- Normalize Windows CRLF line endings to Linux LF without changing file content.
- Retain platform-gated shared source where removing it could destabilize the
  application; the Rust compiler excludes those sections from Linux builds.
- Do not read, modify, or delete anything outside the Handy project except for
  installing explicitly required Pop!_OS system packages.

## Dependency locations

- Install native compiler and development packages with Pop!_OS/Ubuntu `apt`,
  which places managed headers and libraries in the distribution's standard
  system locations.
- Keep Rust under the current user's rustup-managed directory.
- Keep Bun in its standard user-managed location.
- Install frontend packages into the project's `node_modules` directory.
- Keep Cargo build output in `src-tauri/target`; do not use the Windows
  short-path target-directory workaround.

## Documentation

Update `LINUX.md` and related Linux build instructions to:

- reflect version 0.9.4;
- remove Windows NPU guidance;
- use wildcard package filenames where practical;
- install app-private runtime libraries under `/usr/lib/Handy`;
- document Pop!_OS dependencies and Wayland limitations accurately.

## Verification

1. Confirm generated Windows artifacts are absent.
2. Confirm Linux build configuration still enables Vulkan and dynamic backends.
3. Install frontend dependencies.
4. Run formatting checks, frontend lint/type checks, and Rust checks.
5. Build a Linux `.deb` bundle.
6. Inspect the bundle for the executable, resources, and app-private runtime
   libraries.

The application will not be installed system-wide unless the build succeeds.
