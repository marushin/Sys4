//! Window placement & main/widget switching.
//!
//! The geometry (`resolve_position` and helpers) is pure so it can be unit-tested; the
//! Tauri glue below gathers monitor data, applies the result and persists it.

use tauri::{AppHandle, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::config::{ConfigStore, KEY_MAIN_X, KEY_MAIN_Y, KEY_WIDGET_X, KEY_WIDGET_Y};

pub const MAIN: &str = "main";
pub const WIDGET: &str = "widget";

/// Windows parks minimized windows at (-32000, -32000); anything this far out is bogus.
const INVALID_COORD: i32 = -10000;
/// Logical gap between the widget and the work-area edge.
const WIDGET_PADDING: f64 = 12.0;

/// Initial widget size; the widget then fits its height to its content (2–3 rows).
pub const WIDGET_LOGICAL_SIZE: (f64, f64) = (180.0, 72.0);
const MAIN_LOGICAL_SIZE: (f64, f64) = (260.0, 500.0);

// ───────────────────────────── Pure geometry ─────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn right(&self) -> i32 {
        self.x + self.w
    }
    fn bottom(&self) -> i32 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Screen {
    /// Full monitor bounds.
    pub bounds: Rect,
    /// Bounds minus taskbar/docked bars.
    pub work: Rect,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Center,
    BottomRight,
}

pub fn is_valid_coord((x, y): (i32, i32)) -> bool {
    x > INVALID_COORD && y > INVALID_COORD
}

fn overlap_area(a: Rect, b: Rect) -> i64 {
    let w = (a.right().min(b.right()) - a.x.max(b.x)).max(0) as i64;
    let h = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0) as i64;
    w * h
}

fn clamp_into((x, y): (i32, i32), (w, h): (i32, i32), area: Rect) -> (i32, i32) {
    let max_x = (area.right() - w).max(area.x);
    let max_y = (area.bottom() - h).max(area.y);
    (x.clamp(area.x, max_x), y.clamp(area.y, max_y))
}

pub fn default_position(placement: Placement, (w, h): (i32, i32), screen: &Screen) -> (i32, i32) {
    let work = screen.work;
    match placement {
        Placement::Center => (
            work.x + (work.w - w).max(0) / 2,
            work.y + (work.h - h).max(0) / 2,
        ),
        Placement::BottomRight => {
            let pad = (WIDGET_PADDING * screen.scale).round() as i32;
            (
                (work.right() - w - pad).max(work.x),
                (work.bottom() - h - pad).max(work.y),
            )
        }
    }
}

/// Decide where a window of `size` should go.
///
/// * A valid candidate that overlaps a monitor is clamped into that monitor's work area.
/// * With a single monitor, an off-screen candidate (e.g. after a resolution drop) is clamped in.
/// * Otherwise (no candidate, or saved on a now-disconnected monitor) use the default spot on
///   the primary monitor.
pub fn resolve_position(
    candidate: Option<(i32, i32)>,
    size: (i32, i32),
    screens: &[Screen],
    primary: usize,
    placement: Placement,
) -> (i32, i32) {
    let Some(primary_screen) = screens.get(primary).or_else(|| screens.first()) else {
        return candidate.unwrap_or((0, 0));
    };

    if let Some(pos) = candidate.filter(|&p| is_valid_coord(p)) {
        let rect = Rect {
            x: pos.0,
            y: pos.1,
            w: size.0,
            h: size.1,
        };
        let best = screens
            .iter()
            .map(|s| (overlap_area(rect, s.bounds), s))
            .filter(|(area, _)| *area > 0)
            .max_by_key(|(area, _)| *area)
            .map(|(_, s)| s);

        if let Some(screen) = best {
            return clamp_into(pos, size, screen.work);
        }
        if screens.len() == 1 {
            return clamp_into(pos, size, screens[0].work);
        }
    }

    default_position(placement, size, primary_screen)
}

// ───────────────────────────── Tauri glue ─────────────────────────────

struct Spec {
    placement: Placement,
    keys: (&'static str, &'static str),
    logical_size: (f64, f64),
}

fn spec(label: &str) -> Spec {
    if label == WIDGET {
        Spec {
            placement: Placement::BottomRight,
            keys: (KEY_WIDGET_X, KEY_WIDGET_Y),
            logical_size: WIDGET_LOGICAL_SIZE,
        }
    } else {
        Spec {
            placement: Placement::Center,
            keys: (KEY_MAIN_X, KEY_MAIN_Y),
            logical_size: MAIN_LOGICAL_SIZE,
        }
    }
}

fn to_screen(m: &Monitor) -> Screen {
    let bounds = Rect {
        x: m.position().x,
        y: m.position().y,
        w: m.size().width as i32,
        h: m.size().height as i32,
    };
    let wa = m.work_area();
    let work = Rect {
        x: wa.position.x,
        y: wa.position.y,
        w: wa.size.width as i32,
        h: wa.size.height as i32,
    };
    // Some drivers report an empty work area; fall back to full bounds.
    let work = if work.w > 0 && work.h > 0 { work } else { bounds };
    Screen {
        bounds,
        work,
        scale: m.scale_factor(),
    }
}

/// All screens plus the index of the primary one.
fn screens_of(win: &WebviewWindow) -> (Vec<Screen>, usize) {
    let monitors = win.available_monitors().unwrap_or_default();
    let primary = win.primary_monitor().ok().flatten();
    let mut screens: Vec<Screen> = monitors.iter().map(to_screen).collect();

    let primary_idx = match &primary {
        Some(p) => match monitors.iter().position(|m| m.position() == p.position()) {
            Some(i) => i,
            None => {
                screens.push(to_screen(p));
                screens.len() - 1
            }
        },
        None => 0,
    };
    (screens, primary_idx)
}

fn window_size(win: &WebviewWindow, spec: &Spec, scale: f64) -> (i32, i32) {
    match win.outer_size() {
        Ok(sz) if sz.width > 0 && sz.height > 0 => (sz.width as i32, sz.height as i32),
        _ => (
            (spec.logical_size.0 * scale).round() as i32,
            (spec.logical_size.1 * scale).round() as i32,
        ),
    }
}

fn get_window(app: &AppHandle, label: &str) -> Result<WebviewWindow, String> {
    app.get_webview_window(label)
        .ok_or_else(|| format!("Window '{}' not found", label))
}

/// Move `label` to `candidate` (or its saved/current position) while guaranteeing it is
/// on-screen; `use_saved = false` forces the default placement (used by "reset position").
fn place(
    app: &AppHandle,
    label: &str,
    candidate: Option<(i32, i32)>,
    use_saved: bool,
) -> Result<(i32, i32), String> {
    let win = get_window(app, label)?;
    let spec = spec(label);
    let config = app.state::<ConfigStore>();

    let (screens, primary) = screens_of(&win);
    if screens.is_empty() {
        return Ok(win.outer_position().map(|p| (p.x, p.y)).unwrap_or((0, 0)));
    }

    let candidate = if use_saved {
        candidate
            .filter(|&p| is_valid_coord(p))
            .or_else(|| {
                let saved = (config.get_i32(spec.keys.0)?, config.get_i32(spec.keys.1)?);
                Some(saved).filter(|&p| is_valid_coord(p))
            })
            .or_else(|| {
                win.outer_position()
                    .ok()
                    .map(|p| (p.x, p.y))
                    .filter(|&p| is_valid_coord(p))
            })
    } else {
        None
    };

    let scale = screens[primary.min(screens.len() - 1)].scale;
    let size = window_size(&win, &spec, scale);
    let pos = resolve_position(candidate, size, &screens, primary, spec.placement);

    let _ = win.unminimize();
    win.set_position(tauri::PhysicalPosition { x: pos.0, y: pos.1 })
        .map_err(|e| e.to_string())?;
    config.set_position(spec.keys, pos);
    Ok(pos)
}

pub fn ensure_visible(
    app: &AppHandle,
    label: &str,
    candidate: Option<(i32, i32)>,
) -> Result<(i32, i32), String> {
    place(app, label, candidate, true)
}

pub fn reset_position(app: &AppHandle, label: &str) -> Result<(i32, i32), String> {
    place(app, label, None, false)
}

fn is_visible(app: &AppHandle, label: &str) -> bool {
    app.get_webview_window(label)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false)
}

fn present(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = ensure_visible(app, label, None);
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Get the widget window, creating it (hidden) on first use.
fn ensure_widget(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(w) = app.get_webview_window(WIDGET) {
        return Ok(w);
    }
    WebviewWindowBuilder::new(app, WIDGET, WebviewUrl::App("widget.html".into()))
        .title("Sys4 Widget")
        .inner_size(WIDGET_LOGICAL_SIZE.0, WIDGET_LOGICAL_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.hide();
    }
    present(app, MAIN);
}

pub fn show_widget(app: &AppHandle) -> Result<(), String> {
    ensure_widget(app)?;
    if let Some(m) = app.get_webview_window(MAIN) {
        let _ = m.hide();
    }
    present(app, WIDGET);
    Ok(())
}

pub fn toggle_widget(app: &AppHandle) -> Result<(), String> {
    if is_visible(app, WIDGET) {
        show_main(app);
        Ok(())
    } else {
        show_widget(app)
    }
}

/// Bring the currently active mode (widget or main) to the front — used by single-instance
/// re-launch and the tray icon.
pub fn focus_active(app: &AppHandle) {
    if is_visible(app, WIDGET) {
        present(app, WIDGET);
    } else {
        present(app, MAIN);
    }
}

/// Tray "reset position": snap the active window back to its default spot and show it.
pub fn reset_active(app: &AppHandle) {
    let label = if is_visible(app, WIDGET) { WIDGET } else { MAIN };
    let _ = reset_position(app, label);
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen(x: i32, y: i32, w: i32, h: i32, taskbar: i32) -> Screen {
        Screen {
            bounds: Rect { x, y, w, h },
            work: Rect {
                x,
                y,
                w,
                h: h - taskbar,
            },
            scale: 1.0,
        }
    }

    const SIZE: (i32, i32) = (260, 500);

    #[test]
    fn valid_position_is_kept() {
        let screens = [screen(0, 0, 1920, 1080, 48)];
        let pos = resolve_position(Some((100, 100)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, (100, 100));
    }

    #[test]
    fn minimized_coords_fall_back_to_default() {
        let screens = [screen(0, 0, 1920, 1080, 48)];
        let pos = resolve_position(Some((-32000, -32000)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, ((1920 - 260) / 2, (1032 - 500) / 2));
    }

    #[test]
    fn partially_offscreen_is_clamped_into_work_area() {
        let screens = [screen(0, 0, 1920, 1080, 48)];
        let pos = resolve_position(Some((1800, 900)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, (1920 - 260, 1032 - 500));
    }

    #[test]
    fn single_monitor_resolution_drop_is_clamped() {
        // Saved on a 4K layout, now 1080p: no overlap at all, but only one monitor.
        let screens = [screen(0, 0, 1920, 1080, 48)];
        let pos = resolve_position(Some((3000, 1800)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, (1920 - 260, 1032 - 500));
    }

    #[test]
    fn disconnected_monitor_falls_back_to_primary() {
        let screens = [screen(0, 0, 1920, 1080, 48), screen(1920, 0, 1920, 1080, 0)];
        let pos = resolve_position(Some((5000, 100)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, ((1920 - 260) / 2, (1032 - 500) / 2));
    }

    #[test]
    fn picks_monitor_with_largest_overlap() {
        let screens = [screen(0, 0, 1920, 1080, 48), screen(1920, 0, 1920, 1080, 0)];
        // Mostly on the second monitor, sticking out past its bottom.
        let pos = resolve_position(Some((1900, 700)), SIZE, &screens, 0, Placement::Center);
        assert_eq!(pos, (1920, 1080 - 500));
    }

    #[test]
    fn widget_default_sits_above_taskbar() {
        let s = screen(0, 0, 1920, 1080, 48);
        let pos = default_position(Placement::BottomRight, (180, 70), &s);
        assert_eq!(pos, (1920 - 180 - 12, 1032 - 70 - 12));
    }

    #[test]
    fn no_screens_returns_candidate() {
        let pos = resolve_position(Some((5, 6)), SIZE, &[], 0, Placement::Center);
        assert_eq!(pos, (5, 6));
    }
}
