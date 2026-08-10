use crate::input;
use crate::settings;
use crate::settings::{OverlayPosition, OverlayStyle};
#[cfg(target_os = "linux")]
use crate::tray_i18n::get_tray_translations;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[cfg(not(target_os = "macos"))]
use log::debug;

#[cfg(not(target_os = "macos"))]
use tauri::WebviewWindowBuilder;

#[cfg(target_os = "macos")]
use tauri::WebviewUrl;

#[cfg(target_os = "macos")]
use tauri_nspanel::{tauri_panel, CollectionBehavior, PanelBuilder, PanelLevel, StyleMask};

#[cfg(target_os = "linux")]
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

#[cfg(target_os = "linux")]
use gtk::prelude::*;

#[cfg(target_os = "linux")]
use std::env;

#[cfg(target_os = "linux")]
use std::cell::Cell;
#[cfg(target_os = "linux")]
use std::cell::RefCell;

#[cfg(target_os = "linux")]
thread_local! {
    static LINUX_OVERLAY_WIDGETS: RefCell<Option<LinuxOverlayWidgets>> = const { RefCell::new(None) };
}

#[cfg(target_os = "linux")]
struct LinuxOverlayWidgets {
    status_label: gtk::Label,
    stream_label: gtk::Label,
    stream_scroller: gtk::ScrolledWindow,
    waveform: gtk::Box,
    bars: Vec<gtk::Label>,
    stream_expanded: Cell<bool>,
}

#[cfg(target_os = "macos")]
tauri_panel! {
    panel!(RecordingOverlayPanel {
        config: {
            can_become_key_window: false,
            is_floating_panel: true
        }
    })
}

// Native overlay window sizes (logical points). One window is reused for every
// state and resized in `show_overlay_state`; each size need only be at least as
// large as the card it hosts (the `--ov-*` vars in RecordingOverlay.css). The
// card is CSS-anchored flush to the screen edge, so window height doesn't move
// where the card sits — only OVERLAY_TOP_OFFSET / OVERLAY_BOTTOM_OFFSET do. Keep
// these in sync with the CSS card geometry.
//
// Compact overlay (Minimal / transcribing / processing): the 40h pill animates
// width from 172 (--ov-rest-w) to 216 (--ov-work-w) and expands from center, so
// the window must fit the widest state plus a little slack.
const OVERLAY_WIDTH: f64 = 256.0;
const OVERLAY_HEIGHT: f64 = 46.0;

// Live mode begins compact, then eases to its monitor-relative width when the
// first recognized text arrives. It grows vertically once that text wraps.
const OVERLAY_STREAM_WIDTH: f64 = 400.0;
const OVERLAY_STREAM_HEIGHT: f64 = 72.0;

// The live-transcription card scales in logical pixels so it occupies the same
// visual proportion on normal- and HiDPI monitors. Its width follows the
// selected monitor, while its content can grow vertically up to half of that
// monitor before the transcript becomes scrollable.
const STREAM_WIDTH_FRACTION: f64 = 0.35;
const STREAM_MAX_HEIGHT_FRACTION: f64 = 0.5;
const STREAM_MIN_WIDTH: f64 = 400.0;
const STREAM_MAX_WIDTH: f64 = 760.0;
const STREAM_CHROME_HEIGHT: f64 = 54.0;
#[cfg(target_os = "linux")]
const STREAM_EXPAND_DURATION_MS: u64 = 200;
#[cfg(target_os = "linux")]
const STREAM_EXPAND_FRAME_MS: u64 = 16;

/// Overlay window size (logical) for a given UI state.
fn overlay_dimensions(state: &str, app_handle: &AppHandle) -> (f64, f64) {
    if state != "streaming" {
        return (OVERLAY_WIDTH, OVERLAY_HEIGHT);
    }

    get_monitor_with_cursor(app_handle)
        .map(|monitor| {
            let scale = monitor.scale_factor();
            streaming_overlay_dimensions(
                monitor.size().width as f64 / scale,
                monitor.size().height as f64 / scale,
            )
        })
        .unwrap_or((OVERLAY_STREAM_WIDTH, OVERLAY_STREAM_HEIGHT))
}

fn streaming_overlay_dimensions(monitor_width: f64, _monitor_height: f64) -> (f64, f64) {
    (
        (monitor_width * STREAM_WIDTH_FRACTION).clamp(STREAM_MIN_WIDTH, STREAM_MAX_WIDTH),
        OVERLAY_STREAM_HEIGHT,
    )
}

static LAST_MIC_LEVEL_EMIT: AtomicU64 = AtomicU64::new(0);
const EMIT_THROTTLE_MS: u64 = 33; // ~30 FPS

#[cfg(target_os = "macos")]
const OVERLAY_TOP_OFFSET: f64 = 46.0;
#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_TOP_OFFSET: f64 = 4.0;

#[cfg(target_os = "macos")]
const OVERLAY_BOTTOM_OFFSET: f64 = 15.0;

#[cfg(any(target_os = "windows", target_os = "linux"))]
const OVERLAY_BOTTOM_OFFSET: f64 = 40.0;

/// Return the GDK output where the compositor actually mapped the overlay.
///
/// COSMIC Wayland does not expose a trustworthy global pointer position to
/// ordinary clients. Leaving the layer-shell output unspecified lets COSMIC
/// choose the pointer output when the surface is mapped. We then query the
/// mapped GDK window only for monitor-relative sizing; we never pin it back to
/// an output ourselves.
#[cfg(target_os = "linux")]
fn mapped_gtk_monitor(gtk_window: &gtk::ApplicationWindow) -> Option<gtk::gdk::Monitor> {
    let gdk_window = gtk_window.window()?;
    gtk_window.display().monitor_at_window(&gdk_window)
}

#[cfg(target_os = "linux")]
fn animate_stream_width(gtk_window: &gtk::ApplicationWindow, target_width: i32, height: i32) {
    let start_width = gtk_window.allocated_width().max(1);
    if start_width >= target_width {
        gtk_window.resize(target_width, height);
        return;
    }

    let window = gtk_window.clone();
    let total_frames = (STREAM_EXPAND_DURATION_MS / STREAM_EXPAND_FRAME_MS).max(1) as u32;
    let mut frame = 0_u32;
    gtk::glib::timeout_add_local(
        std::time::Duration::from_millis(STREAM_EXPAND_FRAME_MS),
        move || {
            frame += 1;
            let progress = (frame as f64 / total_frames as f64).min(1.0);
            // Cubic ease-out: quick enough to feel responsive, gentle at the
            // prescribed width so the card does not appear to snap into place.
            let eased = 1.0 - (1.0 - progress).powi(3);
            let width = start_width as f64 + (target_width - start_width) as f64 * eased;
            window.resize(width.round() as i32, height);
            window.queue_draw();

            if frame >= total_frames {
                gtk::glib::ControlFlow::Break
            } else {
                gtk::glib::ControlFlow::Continue
            }
        },
    );
}

#[cfg(target_os = "linux")]
fn update_gtk_layer_shell_anchors(
    overlay_window: &tauri::webview::WebviewWindow,
    width: f64,
    height: f64,
) {
    // Layer-shell surfaces are positioned by the compositor, so ordinary
    // set_position calls are ignored on Wayland. Select the output under the
    // pointer and express the edge offset as a layer-shell margin instead.
    let window_clone = overlay_window.clone();
    let _ = overlay_window.run_on_main_thread(move || {
        // Try to get the GTK window from the Tauri webview
        if let Ok(gtk_window) = window_clone.gtk_window() {
            let settings = settings::get_settings(window_clone.app_handle());

            // Leave the horizontal axis unanchored: layer-shell centers the
            // requested width on the selected output. This behaves consistently
            // for compact and streaming layouts and avoids mixed-DPI arithmetic
            // in global compositor coordinates.
            gtk_window.set_anchor(Edge::Left, false);
            gtk_window.set_anchor(Edge::Right, false);
            gtk_window.set_layer_shell_margin(Edge::Left, 0);
            gtk_window.set_layer_shell_margin(Edge::Right, 0);

            match settings.overlay_position {
                OverlayPosition::Top => {
                    gtk_window.set_anchor(Edge::Top, true);
                    gtk_window.set_anchor(Edge::Bottom, false);
                    gtk_window.set_layer_shell_margin(Edge::Top, OVERLAY_TOP_OFFSET as i32);
                    gtk_window.set_layer_shell_margin(Edge::Bottom, 0);
                }
                OverlayPosition::Bottom => {
                    gtk_window.set_anchor(Edge::Bottom, true);
                    gtk_window.set_anchor(Edge::Top, false);
                    gtk_window.set_layer_shell_margin(Edge::Bottom, OVERLAY_BOTTOM_OFFSET as i32);
                    gtk_window.set_layer_shell_margin(Edge::Top, 0);
                }
            }

            gtk_window.resize(width.round() as i32, height.round() as i32);
        }
    });
}

/// Ask COSMIC to select the pointer output without reading forbidden global
/// pointer coordinates. A fresh invisible layer surface is assigned to the
/// current pointer output by the compositor; once mapped, its GDK monitor is
/// applied to the reusable Handy overlay and the probe is destroyed.
#[cfg(target_os = "linux")]
fn show_linux_overlay_on_pointer_output(
    overlay_window: &tauri::webview::WebviewWindow,
    state: &str,
) {
    let window_clone = overlay_window.clone();
    let linux_state = state.to_owned();
    let _ = overlay_window.run_on_main_thread(move || {
        let Ok(gtk_window) = window_clone.gtk_window() else {
            return;
        };

        let probe = gtk::Window::new(gtk::WindowType::Popup);
        probe.set_decorated(false);
        probe.set_accept_focus(false);
        probe.set_app_paintable(true);
        probe.set_opacity(0.0);
        probe.resize(1, 1);
        probe.init_layer_shell();
        probe.set_layer(Layer::Overlay);
        probe.set_keyboard_mode(KeyboardMode::None);
        probe.set_exclusive_zone(0);

        let target_window = gtk_window.clone();
        probe.connect_map_event(move |probe_window, _| {
            if let Some(probe_gdk_window) = probe_window.window() {
                let monitor = probe_window.display().monitor_at_window(&probe_gdk_window);
                if let Some(monitor) = monitor {
                    let geometry = monitor.geometry();
                    debug!(
                        "overlay pointer output probe: ({},{} {}x{}) state={}",
                        geometry.x(),
                        geometry.y(),
                        geometry.width(),
                        geometry.height(),
                        linux_state
                    );
                    target_window.set_monitor(&monitor);
                    if linux_state == "streaming" {
                        target_window.resize(
                            OVERLAY_STREAM_WIDTH.round() as i32,
                            OVERLAY_STREAM_HEIGHT.round() as i32,
                        );
                    }
                }
            }

            target_window.show();
            target_window.queue_draw();
            probe_window.close();
            gtk::glib::Propagation::Proceed
        });
        probe.show();
    });
}

#[cfg(target_os = "linux")]
fn install_native_linux_overlay(
    gtk_window: &gtk::ApplicationWindow,
    app_handle: &tauri::AppHandle,
) {
    let settings = settings::get_settings(app_handle);
    let strings = get_tray_translations(Some(settings.app_language));
    LINUX_OVERLAY_WIDGETS.with(|stored| {
        if stored.borrow().is_some() {
            return;
        }

        if let Some(webview_child) = gtk_window.child() {
            gtk_window.remove(&webview_child);
        }

        // The Tauri WebKit child does not submit a Wayland buffer after its
        // window is converted to a layer-shell surface on COSMIC. A small
        // native GTK card keeps the overlay reliable while retaining the
        // rounded, dark visual language of the regular web overlay.
        gtk_window.set_app_paintable(true);
        if let Some(screen) = gtk::prelude::WidgetExt::screen(gtk_window) {
            if let Some(visual) = screen.rgba_visual() {
                gtk_window.set_visual(Some(&visual));
            }

            let css = gtk::CssProvider::new();
            if css
                .load_from_data(
                    br#"
                    window { background-color: transparent; }
                    #handy-card {
                        background-color: rgba(16, 18, 24, 0.82);
                        border: 1px solid rgba(255, 255, 255, 0.12);
                        border-radius: 18px;
                        padding: 0 12px;
                        box-shadow: 0 6px 18px rgba(0, 0, 0, 0.28);
                    }
                    .handy-dot {
                        color: #7aa2f7;
                        font-size: 15px;
                    }
                    .handy-status {
                        color: #eef1f7;
                        font-size: 13px;
                        font-weight: 600;
                    }
                    .handy-wave {
                        color: #7aa2f7;
                        font-size: 13px;
                    }
                    .handy-stream {
                        color: #d7dbe5;
                        font-size: 14px;
                    }
                    .handy-cancel {
                        color: #aeb4c0;
                        background: transparent;
                        border: 0;
                        box-shadow: none;
                        padding: 0 3px;
                        min-width: 20px;
                        min-height: 24px;
                        font-size: 18px;
                    }
                    .handy-cancel:hover {
                        color: #ffffff;
                        background-color: rgba(255, 255, 255, 0.08);
                        border-radius: 12px;
                    }
                    "#,
                )
                .is_ok()
            {
                gtk::StyleContext::add_provider_for_screen(
                    &screen,
                    &css,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
        }

        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.set_widget_name("handy-card");
        card.set_hexpand(true);
        card.set_vexpand(true);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 9);
        row.set_halign(gtk::Align::Fill);
        row.set_valign(gtk::Align::Center);
        row.set_hexpand(true);
        // Keep the complete control cluster—the dot, state label, waveform,
        // and cancel button—comfortably clear of the card's bottom border.
        row.set_margin_bottom(6);

        let dot = gtk::Label::new(Some("●"));
        dot.style_context().add_class("handy-dot");
        row.pack_start(&dot, false, false, 0);

        let status_label = gtk::Label::new(None);
        status_label.style_context().add_class("handy-status");
        status_label.set_halign(gtk::Align::Start);
        row.pack_start(&status_label, false, false, 0);

        let waveform = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        waveform.set_halign(gtk::Align::Center);
        waveform.set_hexpand(true);
        let mut bars = Vec::with_capacity(7);
        for _ in 0..7 {
            let bar = gtk::Label::new(Some("▮"));
            bar.style_context().add_class("handy-wave");
            bar.set_opacity(0.3);
            waveform.pack_start(&bar, false, false, 0);
            bars.push(bar);
        }
        row.pack_start(&waveform, true, true, 0);

        let cancel = gtk::Button::with_label("×");
        cancel.style_context().add_class("handy-cancel");
        cancel.set_relief(gtk::ReliefStyle::None);
        cancel.set_tooltip_text(Some(&strings.cancel));
        let app = app_handle.clone();
        cancel.connect_clicked(move |_| crate::utils::cancel_current_operation(&app));
        row.pack_end(&cancel, false, false, 0);

        let stream_label = gtk::Label::new(None);
        stream_label.style_context().add_class("handy-stream");
        stream_label.set_line_wrap(true);
        stream_label.set_line_wrap_mode(gtk::pango::WrapMode::WordChar);
        stream_label.set_ellipsize(gtk::pango::EllipsizeMode::None);
        stream_label.set_lines(-1);
        stream_label.set_xalign(0.0);
        stream_label.set_yalign(1.0);

        let stream_scroller =
            gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
        stream_scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        stream_scroller.set_shadow_type(gtk::ShadowType::None);
        stream_scroller.set_propagate_natural_height(true);
        stream_scroller.set_margin_top(8);
        stream_scroller.set_margin_bottom(10);
        stream_scroller.add(&stream_label);

        card.pack_end(&row, false, false, 0);
        card.pack_start(&stream_scroller, true, true, 0);
        gtk_window.add(&card);
        card.show_all();
        stream_scroller.hide();

        *stored.borrow_mut() = Some(LinuxOverlayWidgets {
            status_label,
            stream_label,
            stream_scroller,
            waveform,
            bars,
            stream_expanded: Cell::new(false),
        });
    });
}

#[cfg(target_os = "linux")]
fn update_native_linux_overlay(overlay_window: &tauri::webview::WebviewWindow, state: &str) {
    let settings = settings::get_settings(overlay_window.app_handle());
    let strings = get_tray_translations(Some(settings.app_language));
    let text = match state {
        "streaming" => strings.overlay_live,
        "transcribing" => strings.overlay_transcribing,
        "processing" => strings.overlay_processing,
        _ => strings.overlay_recording,
    };
    let show_waveform = matches!(state, "recording" | "streaming");
    let window_clone = overlay_window.clone();
    let _ = overlay_window.run_on_main_thread(move || {
        LINUX_OVERLAY_WIDGETS.with(|stored| {
            if let Some(widgets) = stored.borrow().as_ref() {
                widgets.status_label.set_text(&text);
                widgets.status_label.show();
                widgets.waveform.set_visible(show_waveform);
                // Every state transition starts a fresh presentation. In
                // particular, entering a second streaming session must not
                // display transcript text left over from the previous one.
                widgets.stream_label.set_text("");
                widgets.stream_scroller.hide();
                widgets.stream_expanded.set(false);
            }
        });
        if let Ok(gtk_window) = window_clone.gtk_window() {
            gtk_window.queue_draw();
        }
    });
}

/// Update the native Linux live-transcription card. The regular web overlay
/// receives the same data through Tauri events on Windows and macOS.
#[cfg(target_os = "linux")]
pub fn update_native_stream_text(app_handle: &AppHandle, committed: &str, tentative: &str) {
    let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") else {
        return;
    };
    let text = match (committed.trim(), tentative.trim()) {
        ("", tentative) => tentative.to_owned(),
        (committed, "") => committed.to_owned(),
        (committed, tentative) => format!("{committed} {tentative}"),
    };
    let window_clone = overlay_window.clone();
    let _ = overlay_window.run_on_main_thread(move || {
        LINUX_OVERLAY_WIDGETS.with(|stored| {
            if let Some(widgets) = stored.borrow().as_ref() {
                widgets.stream_label.set_text(&text);
                if text.is_empty() {
                    widgets.stream_scroller.hide();
                    return;
                }
                let animate_expansion = !widgets.stream_expanded.replace(true);

                let gtk_window = window_clone.gtk_window().ok();
                let mapped_geometry = gtk_window
                    .as_ref()
                    .and_then(mapped_gtk_monitor)
                    .map(|monitor| monitor.geometry());
                let (width, base_height) = mapped_geometry
                    .map(|geometry| {
                        streaming_overlay_dimensions(
                            geometry.width() as f64,
                            geometry.height() as f64,
                        )
                    })
                    .unwrap_or_else(|| overlay_dimensions("streaming", window_clone.app_handle()));
                let monitor_height = mapped_geometry
                    .map(|geometry| geometry.height() as f64)
                    .unwrap_or(1200.0);
                let max_height =
                    (monitor_height * STREAM_MAX_HEIGHT_FRACTION).max(OVERLAY_STREAM_HEIGHT);
                let max_content_height = (max_height - STREAM_CHROME_HEIGHT).round() as i32;
                widgets
                    .stream_scroller
                    .set_max_content_height(max_content_height);
                widgets.stream_scroller.show_all();

                // Ask Pango how tall the complete wrapped transcript wants to
                // be at this monitor-relative width. The card grows until half
                // the output height; after that its vertical scroller takes over.
                let label_width = (width - 24.0).max(1.0).round() as i32;
                let (_, natural_text_height) =
                    widgets.stream_label.preferred_height_for_width(label_width);
                let target_height = (natural_text_height as f64 + STREAM_CHROME_HEIGHT)
                    .clamp(base_height, max_height);

                if let Some(gtk_window) = gtk_window {
                    if animate_expansion {
                        animate_stream_width(
                            &gtk_window,
                            width.round() as i32,
                            target_height.round() as i32,
                        );
                    } else {
                        gtk_window.resize(width.round() as i32, target_height.round() as i32);
                        gtk_window.queue_draw();
                    }
                }

                let adjustment = widgets.stream_scroller.vadjustment();
                adjustment.set_value(adjustment.upper() - adjustment.page_size());
            }
        });
    });
}

/// Switch the native Linux live card to its finalizing or polishing state.
#[cfg(target_os = "linux")]
pub fn update_native_stream_working(app_handle: &AppHandle, polishing: bool) {
    let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") else {
        return;
    };
    let settings = settings::get_settings(app_handle);
    let strings = get_tray_translations(Some(settings.app_language));
    let text = if polishing {
        strings.overlay_processing
    } else {
        strings.overlay_transcribing
    };
    let _ = overlay_window.run_on_main_thread(move || {
        LINUX_OVERLAY_WIDGETS.with(|stored| {
            if let Some(widgets) = stored.borrow().as_ref() {
                widgets.status_label.set_text(&text);
                widgets.waveform.hide();
            }
        });
    });
}

#[cfg(target_os = "linux")]
fn update_native_linux_levels(app_handle: &AppHandle, levels: &[f32]) {
    let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") else {
        return;
    };
    let values: Vec<f64> = levels.iter().take(7).map(|level| *level as f64).collect();
    let _ = overlay_window.run_on_main_thread(move || {
        LINUX_OVERLAY_WIDGETS.with(|stored| {
            if let Some(widgets) = stored.borrow().as_ref() {
                for (index, bar) in widgets.bars.iter().enumerate() {
                    let level = values.get(index).copied().unwrap_or(0.0).clamp(0.0, 1.0);
                    bar.set_opacity(0.22 + level * 0.78);
                    bar.set_text(match level {
                        value if value > 0.80 => "█",
                        value if value > 0.60 => "▆",
                        value if value > 0.40 => "▄",
                        value if value > 0.20 => "▃",
                        _ => "▂",
                    });
                }
            }
        });
    });
}

/// Returns true when the environment variable is set to a truthy value
/// (e.g. "1", "true", "yes", "on").
/// "0", "false", "no", "off" and empty string are treated as falsy (case-insensitive).
/// Returns false when the variable is not set.
#[cfg(target_os = "linux")]
fn env_flag_enabled(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off"
        ),
        Err(_) => false,
    }
}

/// Initializes GTK layer shell for Linux overlay window
/// Returns true if layer shell was successfully initialized, false otherwise
#[cfg(target_os = "linux")]
fn init_gtk_layer_shell(overlay_window: &tauri::webview::WebviewWindow) -> bool {
    if env_flag_enabled("HANDY_NO_GTK_LAYER_SHELL") {
        debug!("Skipping GTK layer shell init (HANDY_NO_GTK_LAYER_SHELL is enabled)");
        return false;
    }

    if !gtk_layer_shell::is_supported() {
        return false;
    }

    // Try to get the GTK window from the Tauri webview
    if let Ok(gtk_window) = overlay_window.gtk_window() {
        // Initialize layer shell
        gtk_window.init_layer_shell();
        gtk_window.set_layer(Layer::Overlay);
        gtk_window.set_keyboard_mode(KeyboardMode::None);
        gtk_window.set_exclusive_zone(0);
        install_native_linux_overlay(&gtk_window, overlay_window.app_handle());

        update_gtk_layer_shell_anchors(overlay_window, OVERLAY_WIDTH, OVERLAY_HEIGHT);

        return true;
    }
    false
}

/// Forces a window to be topmost using Win32 API (Windows only)
/// This is more reliable than Tauri's set_always_on_top which can be overridden
#[cfg(target_os = "windows")]
fn force_overlay_topmost(overlay_window: &tauri::webview::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    // Clone because run_on_main_thread takes 'static
    let overlay_clone = overlay_window.clone();

    // Make sure the Win32 call happens on the UI thread
    let _ = overlay_clone.clone().run_on_main_thread(move || {
        if let Ok(hwnd) = overlay_clone.hwnd() {
            unsafe {
                // Force Z-order: make this window topmost without changing size/pos or stealing focus
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    });
}

fn get_monitor_with_cursor(app_handle: &AppHandle) -> Option<tauri::Monitor> {
    if let Some(mouse_location) = input::get_cursor_position(app_handle) {
        if let Ok(monitors) = app_handle.available_monitors() {
            for monitor in monitors {
                // On Windows both the cursor (enigo -> GetCursorPos) and the
                // monitor bounds are physical pixels, so compare them directly.
                #[cfg(target_os = "windows")]
                if is_mouse_within_monitor(mouse_location, monitor.position(), monitor.size()) {
                    return Some(monitor);
                }

                // macOS/Linux: enigo returns logical coords, so scale the bounds down.
                #[cfg(not(target_os = "windows"))]
                {
                    let scale = monitor.scale_factor();
                    let pos = PhysicalPosition::new(
                        (monitor.position().x as f64 / scale) as i32,
                        (monitor.position().y as f64 / scale) as i32,
                    );
                    let size = PhysicalSize::new(
                        (monitor.size().width as f64 / scale) as u32,
                        (monitor.size().height as f64 / scale) as u32,
                    );
                    if is_mouse_within_monitor(mouse_location, &pos, &size) {
                        return Some(monitor);
                    }
                }
            }
        }
    }

    app_handle.primary_monitor().ok().flatten()
}

fn is_mouse_within_monitor(
    mouse_pos: (i32, i32),
    monitor_pos: &PhysicalPosition<i32>,
    monitor_size: &PhysicalSize<u32>,
) -> bool {
    let (mouse_x, mouse_y) = mouse_pos;
    let PhysicalPosition {
        x: monitor_x,
        y: monitor_y,
    } = *monitor_pos;
    let PhysicalSize {
        width: monitor_width,
        height: monitor_height,
    } = *monitor_size;

    mouse_x >= monitor_x
        && mouse_x < (monitor_x + monitor_width as i32)
        && mouse_y >= monitor_y
        && mouse_y < (monitor_y + monitor_height as i32)
}

/// Returns overlay position in logical coordinates (points on macOS).
///
/// The Bottom anchor uses the macOS work area (visibleFrame) so the overlay
/// tracks the Dock — above it when shown, at the screen edge when hidden.
/// This relies on tauri 2.11's work_area.position.y fix (#14655), the same
/// bug that led PR #969 to abandon work_area for full monitor bounds. Top and
/// the other platforms keep full monitor bounds plus the fixed offsets
/// (work_area is unreliable on Wayland; Windows' offset clears the taskbar).
///
/// We must use LogicalPosition (not PhysicalPosition) because Tauri/tao
/// converts PhysicalPosition using the scale factor of the monitor the window
/// is *currently* on, which is wrong when moving cross-monitor. Windows uses
/// `place_windows_overlay` instead (no single logical space across mixed DPI).
fn calculate_overlay_position(
    app_handle: &AppHandle,
    width: f64,
    height: f64,
) -> Option<(f64, f64)> {
    let monitor = get_monitor_with_cursor(app_handle)?;
    let scale = monitor.scale_factor();
    let monitor_x = monitor.position().x as f64 / scale;
    let monitor_y = monitor.position().y as f64 / scale;
    let monitor_width = monitor.size().width as f64 / scale;

    let settings = settings::get_settings(app_handle);

    let x = monitor_x + (monitor_width - width) / 2.0;
    let y = match settings.overlay_position {
        OverlayPosition::Top => monitor_y + OVERLAY_TOP_OFFSET,
        OverlayPosition::Bottom => {
            // work_area.position shares monitor.position's global coordinate
            // space, so no monitor offset is added.
            #[cfg(target_os = "macos")]
            let bottom = {
                let wa = monitor.work_area();
                (wa.position.y as f64 + wa.size.height as f64) / scale
            };
            #[cfg(not(target_os = "macos"))]
            let bottom = monitor_y + monitor.size().height as f64 / scale;

            bottom - height - OVERLAY_BOTTOM_OFFSET
        }
    };

    Some((x, y))
}

/// Current overlay window size in logical units (points), for repositioning
/// without assuming a fixed size (compact vs. streaming).
#[cfg(not(target_os = "windows"))]
fn current_overlay_logical_size(window: &tauri::webview::WebviewWindow) -> Option<(f64, f64)> {
    let size = window.inner_size().ok()?;
    let scale = window.scale_factor().ok()?;
    Some((size.width as f64 / scale, size.height as f64 / scale))
}

#[cfg(target_os = "windows")]
static WINDOWS_OVERLAY_IS_STREAMING: AtomicBool = AtomicBool::new(false);

/// Overlay rectangle in the destination monitor's physical pixels, so nothing
/// is converted through the window's previous-monitor DPI.
#[cfg(target_os = "windows")]
fn windows_overlay_bounds(
    monitor_position: PhysicalPosition<i32>,
    monitor_size: PhysicalSize<u32>,
    scale: f64,
    logical_width: f64,
    logical_height: f64,
    overlay_position: OverlayPosition,
) -> (i32, i32, i32, i32) {
    let width = (logical_width * scale).round().max(1.0) as i32;
    let height = (logical_height * scale).round().max(1.0) as i32;
    let x = (monitor_position.x as f64 + (monitor_size.width as f64 - width as f64) / 2.0).round()
        as i32;
    let y = match overlay_position {
        OverlayPosition::Top => {
            (monitor_position.y as f64 + OVERLAY_TOP_OFFSET * scale).round() as i32
        }
        OverlayPosition::Bottom => (monitor_position.y as f64 + monitor_size.height as f64
            - height as f64
            - OVERLAY_BOTTOM_OFFSET * scale)
            .round() as i32,
    };

    (x, y, width, height)
}

/// Moves and sizes the overlay in one native SetWindowPos, bypassing tao's
/// current-DPI logical conversion that mislands cross-monitor moves.
#[cfg(target_os = "windows")]
fn place_windows_overlay(
    app_handle: &AppHandle,
    overlay_window: &tauri::webview::WebviewWindow,
    logical_width: f64,
    logical_height: f64,
) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let monitor = get_monitor_with_cursor(app_handle)
        .ok_or_else(|| "failed to determine the monitor containing the cursor".to_string())?;
    let (x, y, width, height) = windows_overlay_bounds(
        *monitor.position(),
        *monitor.size(),
        monitor.scale_factor(),
        logical_width,
        logical_height,
        settings::get_settings(app_handle).overlay_position,
    );
    let hwnd = overlay_window
        .hwnd()
        .map_err(|error| format!("failed to get overlay window handle: {error}"))?;

    unsafe {
        SetWindowPos(
            hwnd,
            None,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .map_err(|error| format!("failed to set overlay bounds: {error}"))?;
    }

    log::debug!(
        "windows overlay bounds: x={} y={} width={} height={} scale={}",
        x,
        y,
        width,
        height,
        monitor.scale_factor()
    );
    Ok(())
}

/// Creates the recording overlay window and keeps it hidden by default
#[cfg(not(target_os = "macos"))]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    // On Linux (Wayland), monitor detection often fails, but we don't need exact coordinates
    // for Layer Shell as we use anchors. On other platforms, we require a monitor.
    #[cfg(not(target_os = "linux"))]
    {
        let position = calculate_overlay_position(app_handle, OVERLAY_WIDTH, OVERLAY_HEIGHT);
        if position.is_none() {
            debug!("Failed to determine overlay position, not creating overlay window");
            return;
        }
    }

    // Position starts unset — update_overlay_position() sets the correct
    // LogicalPosition before the overlay is shown.
    let mut builder = WebviewWindowBuilder::new(
        app_handle,
        "recording_overlay",
        tauri::WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .resizable(false)
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .shadow(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .accept_first_mouse(true)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .focusable(false)
    .focused(false)
    .visible(false);

    if let Some(data_dir) = crate::portable::data_dir() {
        builder = builder.data_directory(data_dir.join("webview"));
    }

    #[allow(unused_variables)]
    match builder.build() {
        Ok(window) => {
            #[cfg(target_os = "linux")]
            {
                // Try to initialize GTK layer shell, ignore errors if compositor doesn't support it
                if init_gtk_layer_shell(&window) {
                    debug!("GTK layer shell initialized for overlay window");
                } else {
                    debug!("GTK layer shell not available, falling back to regular window");
                }
            }

            debug!("Recording overlay window created successfully (hidden)");
        }
        Err(e) => {
            debug!("Failed to create recording overlay window: {}", e);
        }
    }
}

/// Creates the recording overlay panel and keeps it hidden by default (macOS)
#[cfg(target_os = "macos")]
pub fn create_recording_overlay(app_handle: &AppHandle) {
    if let Some((x, y)) = calculate_overlay_position(app_handle, OVERLAY_WIDTH, OVERLAY_HEIGHT) {
        // PanelBuilder creates a Tauri window then converts it to NSPanel.
        // The window remains registered, so get_webview_window() still works.
        match PanelBuilder::<_, RecordingOverlayPanel>::new(app_handle, "recording_overlay")
            .url(WebviewUrl::App("src/overlay/index.html".into()))
            .title("Recording")
            .position(tauri::Position::Logical(tauri::LogicalPosition { x, y }))
            .level(PanelLevel::Status)
            .size(tauri::Size::Logical(tauri::LogicalSize {
                width: OVERLAY_WIDTH,
                height: OVERLAY_HEIGHT,
            }))
            .has_shadow(false)
            .transparent(true)
            .no_activate(true)
            .corner_radius(0.0)
            .style_mask(StyleMask::empty().borderless().nonactivating_panel())
            .with_window(|w| w.decorations(false).transparent(true).focusable(false))
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .build()
        {
            Ok(panel) => {
                panel.hide();
            }
            Err(e) => {
                log::error!("Failed to create recording overlay panel: {}", e);
            }
        }
    }
}

fn show_overlay_state(app_handle: &AppHandle, state: &str) {
    // Whether the overlay shows at all is governed by overlay_style; position
    // only chooses Top vs Bottom placement. Checked here (off the main thread)
    // so the common overlay-disabled case never pays for a main-thread hop.
    let settings = settings::get_settings(app_handle);
    if settings.overlay_style == OverlayStyle::None {
        return;
    }

    // The rest queries monitors and the cursor and mutates window geometry. On
    // Linux the monitor/cursor lookups hit GDK/Xlib on the process's shared X11
    // connection, which is only safe from the GTK main thread — running them on
    // a background thread corrupts the connection and hard-crashes the app
    // (issue #227). Hop to the main thread on every platform to keep the
    // geometry path uniform (a no-op cost on Windows, and it also keeps macOS's
    // NSScreen access main-thread-correct). run_on_main_thread runs the closure
    // inline when already on the main thread, so this never deadlocks.
    let handle = app_handle.clone();
    let state = state.to_string();
    let _ = app_handle.run_on_main_thread(move || show_overlay_state_on_main(&handle, &state));
}

fn show_overlay_state_on_main(app_handle: &AppHandle, state: &str) {
    // Size the overlay for this state (compact vs. streaming), then position it.
    let (width, height) = overlay_dimensions(state, app_handle);
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        #[cfg(target_os = "linux")]
        {
            update_gtk_layer_shell_anchors(&overlay_window, width, height);
            update_native_linux_overlay(&overlay_window, state);
        }

        let size_started = std::time::Instant::now();
        #[cfg(not(target_os = "windows"))]
        let _ = overlay_window.set_size(tauri::Size::Logical(tauri::LogicalSize { width, height }));
        #[cfg(target_os = "windows")]
        WINDOWS_OVERLAY_IS_STREAMING.store(state == "streaming", Ordering::Relaxed);
        let size_elapsed = size_started.elapsed();

        let pos_started = std::time::Instant::now();
        #[cfg(not(target_os = "windows"))]
        let set_pos_elapsed =
            if let Some((x, y)) = calculate_overlay_position(app_handle, width, height) {
                let set_pos_started = std::time::Instant::now();
                let _ = overlay_window
                    .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
                set_pos_started.elapsed()
            } else {
                std::time::Duration::ZERO
            };
        #[cfg(target_os = "windows")]
        let set_pos_elapsed = {
            let set_pos_started = std::time::Instant::now();
            if let Err(error) = place_windows_overlay(app_handle, &overlay_window, width, height) {
                log::error!("Failed to place recording overlay: {error}");
            }
            set_pos_started.elapsed()
        };
        let pos_calc_elapsed = pos_started.elapsed() - set_pos_elapsed;

        let show_started = std::time::Instant::now();
        #[cfg(not(target_os = "linux"))]
        let _ = overlay_window.show();
        #[cfg(target_os = "linux")]
        {
            show_linux_overlay_on_pointer_output(&overlay_window, state);
        }
        let show_elapsed = show_started.elapsed();

        // On Windows, aggressively re-assert "topmost" in the native Z-order after showing
        #[cfg(target_os = "windows")]
        force_overlay_topmost(&overlay_window);

        // Re-assert bounds after show(): the pre-show move crosses the DPI
        // boundary, and tao's WM_DPICHANGED reflow clobbers the first placement.
        #[cfg(target_os = "windows")]
        if let Err(error) = place_windows_overlay(app_handle, &overlay_window, width, height) {
            log::error!("Failed to re-assert recording overlay position: {error}");
        }

        #[cfg(not(target_os = "linux"))]
        let _ = overlay_window.emit("show-overlay", state);
        log::debug!(
            "overlay '{}': set_size={:?} pos_calc={:?} set_pos={:?} show={:?}",
            state,
            size_elapsed,
            pos_calc_elapsed,
            set_pos_elapsed,
            show_elapsed
        );
    }
}

/// Shows the recording overlay window with fade-in animation
pub fn show_recording_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "recording");
}

/// Shows the larger streaming overlay that displays live transcription text
pub fn show_streaming_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "streaming");
}

/// Shows the transcribing overlay window
pub fn show_transcribing_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "transcribing");
}

/// Shows the processing overlay window
pub fn show_processing_overlay(app_handle: &AppHandle) {
    show_overlay_state(app_handle, "processing");
}

/// Updates the overlay window position based on current settings
pub fn update_overlay_position(app_handle: &AppHandle) {
    // Positioning queries monitors/cursor (GDK/Xlib on Linux) and moves the
    // window, so it must run on the main thread — see show_overlay_state.
    let handle = app_handle.clone();
    let _ = app_handle.run_on_main_thread(move || update_overlay_position_on_main(&handle));
}

fn update_overlay_position_on_main(app_handle: &AppHandle) {
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Use the window's current size so centering stays correct whether the
        // overlay is in compact or streaming layout.
        let (width, height) = current_overlay_logical_size(&overlay_window)
            .unwrap_or((OVERLAY_WIDTH, OVERLAY_HEIGHT));

        #[cfg(target_os = "linux")]
        {
            update_gtk_layer_shell_anchors(&overlay_window, width, height);
        }

        #[cfg(target_os = "windows")]
        {
            let state = if WINDOWS_OVERLAY_IS_STREAMING.load(Ordering::Relaxed) {
                "streaming"
            } else {
                "recording"
            };
            let (width, height) = overlay_dimensions(state);
            if let Err(error) = place_windows_overlay(app_handle, &overlay_window, width, height) {
                log::error!("Failed to update recording overlay position: {error}");
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            if let Some((x, y)) = calculate_overlay_position(app_handle, width, height) {
                let _ = overlay_window
                    .set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
            }
        }
    }
}

/// Hides the recording overlay window with fade-out animation
pub fn hide_recording_overlay(app_handle: &AppHandle) {
    // Always hide the overlay regardless of settings - if setting was changed while recording,
    // we still want to hide it properly
    if let Some(overlay_window) = app_handle.get_webview_window("recording_overlay") {
        // Emit event to trigger fade-out animation
        let _ = overlay_window.emit("hide-overlay", ());
        // Hide the window after a short delay to allow animation to complete
        let window_clone = overlay_window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = window_clone.hide();
        });
    }
}

// Cached "overlay is enabled" flag, kept in sync with overlay_style. Avoids
// reading the Tauri store on every audio callback (~24 Hz during recording).
// Defaults to false so the audio path doesn't emit until lib.rs::setup
// populates the cache from initial settings.
static OVERLAY_ENABLED: AtomicBool = AtomicBool::new(false);

/// Update the cached overlay-enabled flag. Called from `lib.rs` at
/// startup after settings load, and from `change_overlay_style_setting`
/// whenever the user changes whether the overlay is shown.
pub fn update_overlay_enabled_cache(enabled: bool) {
    OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn emit_levels(app_handle: &AppHandle, levels: &[f32]) {
    // Skip emission when the overlay is disabled. The recording_overlay
    // window is created at boot regardless of overlay_style, so without this
    // guard a hidden overlay's WebKit subprocess still
    // processes every event. Each event drives some kind of WebKit
    // C++ allocation that accumulates without bound (mechanism not
    // directly characterized; see issue #1279 for the investigation).
    // For users with `overlay_style: none` (the Linux default) this skip
    // eliminates the upstream driver of that accumulation.
    if !OVERLAY_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    // Throttle to ~30 FPS. Even with the overlay enabled, the raw audio
    // callback fires far faster than the UI needs; capping emission rate
    // cuts the per-frame `eval_script`/IPC volume that drives the wry
    // memory growth in issue #1279 (upstream tauri-apps/wry#1489).
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let last = LAST_MIC_LEVEL_EMIT.load(Ordering::Relaxed);
    if now.saturating_sub(last) < EMIT_THROTTLE_MS {
        return;
    }
    LAST_MIC_LEVEL_EMIT.store(now, Ordering::Relaxed);

    // Target only the overlay window. In Tauri 2 both `AppHandle::emit`
    // and `WebviewWindow::emit` broadcast to all webviews; Tauri's
    // listener filter then skips webviews with no registered listener
    // for the event, so the settings webview never received `mic-level`.
    // But the previous dual-call pattern still produced two `eval_script`
    // calls to the overlay per audio callback (one from each .emit()).
    // `emit_to` with the overlay's window label produces a single
    // eval_script call per callback, cutting the per-callback WebKit
    // dispatch work in half.
    #[cfg(target_os = "linux")]
    update_native_linux_levels(app_handle, levels);

    #[cfg(not(target_os = "linux"))]
    let _ = app_handle.emit_to("recording_overlay", "mic-level", levels);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streaming_width_tracks_normal_desktop_monitor() {
        assert_eq!(streaming_overlay_dimensions(1920.0, 1200.0), (672.0, 72.0));
    }

    #[test]
    fn streaming_width_has_readable_minimum() {
        assert_eq!(streaming_overlay_dimensions(800.0, 600.0), (400.0, 72.0));
    }

    #[test]
    fn streaming_width_does_not_become_excessive() {
        assert_eq!(streaming_overlay_dimensions(3840.0, 2160.0), (760.0, 72.0));
    }

    #[test]
    fn monitor_hit_test_uses_half_open_physical_bounds() {
        let position = PhysicalPosition::new(-2560, -200);
        let size = PhysicalSize::new(2560, 1440);

        assert!(is_mouse_within_monitor((-2560, -200), &position, &size));
        assert!(is_mouse_within_monitor((-1, 1239), &position, &size));
        assert!(!is_mouse_within_monitor((0, 0), &position, &size));
        assert!(!is_mouse_within_monitor((-1, 1240), &position, &size));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cursor_hit_test_does_not_scale_physical_monitor_bounds() {
        let position = PhysicalPosition::new(1920, 0);
        let size = PhysicalSize::new(3840, 2160);
        let cursor = (5000, 1000);

        assert!(is_mouse_within_monitor(cursor, &position, &size));

        // This is the old mixed-coordinate comparison. It excludes a cursor
        // that is visibly inside a secondary display running at 150%.
        let scale = 1.5;
        let logical_position = PhysicalPosition::new(
            (position.x as f64 / scale) as i32,
            (position.y as f64 / scale) as i32,
        );
        let logical_size = PhysicalSize::new(
            (size.width as f64 / scale) as u32,
            (size.height as f64 / scale) as u32,
        );
        assert!(!is_mouse_within_monitor(
            cursor,
            &logical_position,
            &logical_size
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_overlay_bounds_use_destination_monitor_scale() {
        let monitor_position = PhysicalPosition::new(1920, 0);
        let monitor_size = PhysicalSize::new(3840, 2160);

        assert_eq!(
            windows_overlay_bounds(
                monitor_position,
                monitor_size,
                1.5,
                OVERLAY_WIDTH,
                OVERLAY_HEIGHT,
                OverlayPosition::Bottom,
            ),
            (3648, 2031, 384, 69)
        );
        assert_eq!(
            windows_overlay_bounds(
                monitor_position,
                monitor_size,
                1.5,
                OVERLAY_WIDTH,
                OVERLAY_HEIGHT,
                OverlayPosition::Top,
            ),
            (3648, 6, 384, 69)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_overlay_bounds_support_negative_monitor_origins() {
        assert_eq!(
            windows_overlay_bounds(
                PhysicalPosition::new(-2560, -200),
                PhysicalSize::new(2560, 1440),
                1.25,
                OVERLAY_STREAM_WIDTH,
                OVERLAY_STREAM_HEIGHT,
                OverlayPosition::Bottom,
            ),
            (-1530, 1040, 500, 150)
        );
    }
}
