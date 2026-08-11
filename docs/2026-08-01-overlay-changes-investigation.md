# Linux Overlay Changes and Investigation

Date: 2026-08-01  
Platform: Pop!\_OS 24.04 with COSMIC Wayland  
Source branch: `codex/popos-linux-build`

## Summary

The Linux overlay had two independent defects after the first native GTK
implementation:

1. a new live-transcription session could briefly display text from the
   previous session; and
2. the layer-shell surface could appear on the wrong monitor and retained a
   fixed `400 × 120` logical-pixel presentation regardless of monitor size or
   transcript length.

The final implementation clears the transcript on every overlay state entry,
uses a new compositor-selected layer-shell probe to identify the pointer
output at trigger time, and sizes the live card relative to that monitor. The
transcript starts at one line, grows vertically, and becomes scrollable when
the card reaches half of the monitor height.

## Restore boundary

Before making the final changes, the project was restored from the complete
backup at:

the local `2026-08-01` project backup (stored outside this repository).

The restore used an exact mirror operation limited to this project folder.
The backup was not modified. File counts and critical SHA-256 values were
compared afterward. The root `HANDOVER.md` survived because it was in the
backup; this investigation file and its companion folder did not, because the
other coding agent created them after the backup. They were deliberately
recreated as documentation only—none of the failed probe code was restored.

## Evidence and root cause

### Stale text

`update_native_linux_overlay()` previously cleared `stream_label` only when
the next state was not `streaming`. Entering another streaming session
therefore left the old label visible until the first tentative transcription
arrived.

The state transition now always empties the label and hides its scroller.
This makes every recording or streaming presentation start cleanly.

### Wrong monitor under COSMIC Wayland

The original GTK layer-shell selection asked GDK for the pointer position and
then called `monitor_at_point()`. During investigation, GDK repeatedly
reported the global pointer as `(0, 0)` under COSMIC Wayland, even while the
pointer was visibly on another monitor. That always biased selection toward
the first output.

Enigo/Xwayland coordinates were also unreliable: COSMIC uses a 5760-pixel
logical desktop while XWayland exposed a differently scaled 8640-pixel root,
and a pointer over a native Wayland client could be clamped to an XWayland
edge. COSMIC's foreign-toplevel protocol exposed window names but did not emit
the output membership needed for focused-window placement on this compositor
version.

The final solution does not request global pointer coordinates. At each
recording trigger it:

1. creates an invisible 1×1 GTK layer-shell probe with no requested monitor;
2. lets COSMIC map that fresh surface to the current pointer output;
3. reads the mapped probe's GDK monitor;
4. assigns the reusable visible overlay to the same monitor; and
5. immediately destroys the probe before revealing the card.

This stays inside normal Wayland/layer-shell behavior, needs no input-device
permission, connector-name matching, Codex helper, or system modification.

The observed GDK logical arrangement during investigation was:

| Output   | Geometry                | Scale            |
| -------- | ----------------------- | ---------------- |
| Output 1 | `(0, 0) 1920 × 1200`    | 1                |
| Output 2 | `(1920, 0) 1920 × 1200` | 1                |
| Output 3 | `(3840, 0) 1920 × 1200` | mixed-DPI layout |

### Fixed streaming geometry

The previous live overlay was always `400 × 120`. The final policy uses
logical pixels:

- width: 35% of the selected monitor;
- minimum width: 400;
- maximum width: 760;
- initial width before recognized text: 400;
- first-text transition: 200ms cubic ease-out to the selected monitor width;
- starting height: 72; and
- maximum height: 50% of the selected monitor.

The compact-to-wide transition is now intentional rather than a first-session
race: every recording resets its expansion state, and only the first non-empty
text update animates from 400 pixels to the prescribed width (672 pixels on a
1920-pixel output). The 72-pixel initial height is a single live-text line
rather than the previous roughly three-line card.

The native card background is `rgba(16, 18, 24, 0.82)`. Its restrained black
shadow is `0 6px 18px rgba(0, 0, 0, 0.28)`.

### Hidden-start paste failure

During three-output verification, live recognition finalized correctly but
paste failed with `Enigo state not initialized`. The main React UI normally
initializes Enigo and shortcuts after onboarding, but a hidden tray/autostart
launch is not guaranteed to run that frontend effect. Linux now initializes
both services during backend core startup. The commands remain idempotent, so
opening the UI and calling them again is harmless.

For each text update, GTK/Pango measures the wrapped label at the available
width. The window grows to the measured natural height plus its control-row
space. Once the maximum height is reached, a vertical scroller retains access
to the complete transcript and follows the latest content.

On each output, the horizontal layer-shell anchors are left unset, which
centers the requested surface width. The configured top or bottom edge remains
anchored with Handy's existing offset.

## Files changed for this fix

- `src-tauri/src/overlay.rs`
  - monitor mapping and layer-shell output assignment;
  - monitor-relative streaming dimensions;
  - transcript scroller and dynamic vertical sizing;
  - unconditional transcript reset on state entry; and
  - focused width-policy tests.
- `LINUX.md`
  - user and maintainer behavior for monitor selection and live-card sizing.
- `HANDOVER.md`
  - current state, verification, artifact, and continuation instructions.
- `docs/investigation-2026-08-01/README.md`
  - compact evidence ledger and regression checklist.

## Verification

The final source passed:

```bash
bun run lint
bun run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
git diff --check
bun run tauri build --bundles deb --no-sign
```

The Rust suite contains 133 passing library tests, including three new tests
covering the normal-monitor calculation and its minimum and maximum clamps.
Two unrelated, pre-existing Rust warnings remain documented in `HANDOVER.md`.

The debug build was started on COSMIC Wayland with the configured
streaming-capable Parakeet model. Real pointer testing confirmed the overlay on
all three displays; the log recorded probe mappings at `(0,0 1920x1200)`,
`(1920,0 1920x1200)`, and `(3840,0 1920x1200)`. Live recognition produced the
expected final text on each output. A subsequent hidden-start run logged
successful Enigo and shortcut initialization before overlay creation.
The final Debian bundle was then installed. Its package SHA-256 is
`d4d2c8560edc1528f0326ef5420012fc75501c18ca4d4c16936146136061c31a`;
the installed `/usr/bin/handy` exactly matches the package payload at SHA-256
`19136e3841abc7247614343623cde1445a8b88962289bc674b478905c06e6f7c`.

## Regression checklist

1. Move the pointer to each monitor and start recording; the compact card must
   appear centered on that output.
2. Repeat with a streaming-capable model; the card must begin at 400 pixels,
   ease once to its wider live width when the first text appears, and remain
   on the same output.
3. Stop and start a second live session; no previous transcript may appear.
4. Feed enough live text to wrap; the card must grow downward/upward from its
   configured edge without exceeding half the output height.
5. Continue feeding text beyond that limit; the full transcript must remain
   reachable through vertical scrolling.
6. Confirm the dot, recording label, voice indicators, and cancel button keep
   their lower padding and the overlay never takes keyboard focus.

## Known limitations

- Output selection depends on COSMIC/another Wayland compositor honoring a
  monitor-unspecified layer-shell surface. The ordinary-window fallback used
  when layer shell is disabled cannot provide the same guarantee.
- Global shortcuts remain subject to Wayland input restrictions.
- Live text is available only for models that advertise streaming support.
- `HANDY_NO_GTK_LAYER_SHELL=1` intentionally uses the ordinary-window fallback
  and therefore does not use compositor layer-shell placement.
