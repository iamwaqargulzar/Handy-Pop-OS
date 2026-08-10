# Overlay Investigation Evidence — 2026-08-01

This folder records the durable conclusions from the Pop!\_OS/COSMIC overlay
investigation. It intentionally contains documentation only. Temporary probe
programs and logs from the unsuccessful intermediate attempt were removed by
the requested full backup restore and were not reintroduced.

## Confirmed observations

- GDK's global pointer query returned `(0, 0)` on COSMIC Wayland, regardless
  of the visible pointer's output.
- Enigo/Xwayland coordinates were not a durable solution: their 8640-pixel
  physical root did not match COSMIC's 5760-pixel logical layout, and native
  Wayland surfaces could leave the pointer clamped to an XWayland edge.
- GDK exposed three logical output rectangles: `(0, 0)`, `(1920, 0)`, and
  `(3840, 0)`, each `1920 × 1200`; the last output used scale factor 2.
- A layer-shell surface must be assigned to the selected GDK monitor. Ordinary
  Tauri `set_position()` calls are ignored for compositor-managed layer-shell
  surfaces.
- The stale transcript was a state-reset defect, separate from placement.

## Final design decision

At every trigger, map a transparent 1×1 GTK layer-shell probe without an
explicit monitor. COSMIC selects the current pointer output for that new
surface. Read its GDK monitor, assign the real overlay there, destroy the probe,
and only then reveal the visible card.

Use a native GTK `ScrolledWindow` around the streaming label. Start at 72
logical pixels high, grow using Pango's wrapped natural height, and cap the
entire card at 50% of the selected output. Width starts at 400 pixels and uses
a one-time 200ms cubic ease-out to 35% of the output (clamped to 400–760) when
the first recognized text appears.

Use an 82%-opaque charcoal card and a soft black 6px/18px shadow at 28%
opacity. Initialize Enigo and shortcuts from Linux backend startup so hidden
tray/autostart sessions can paste without requiring the main UI to open.

## Source and documentation

- Implementation: `src-tauri/src/overlay.rs`
- Full investigation: `docs/2026-08-01-overlay-changes-investigation.md`
- Linux operations: `LINUX.md`
- Continuation state: `HANDOVER.md`

## Repeatable checks

```bash
bun run lint
bun run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
git diff --check
```

Manual testing must cover all three outputs, two consecutive live sessions,
enough transcript text to trigger scrolling, both top and bottom placement,
and the cancel control.
