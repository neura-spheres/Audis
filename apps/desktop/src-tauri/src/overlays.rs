//! The floating windows that appear during a session.

use tauri::{AppHandle, Manager};

/// One of the floating session windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// The caption bar along the bottom of the screen.
    Captions,
    /// The small controller chip.
    Controller,
    /// The assistant's answer panel, beside the controller.
    Assistant,
}

impl Overlay {
    /// The Tauri window label.
    fn label(self) -> &'static str {
        match self {
            Self::Captions => "captions",
            Self::Controller => "controller",
            Self::Assistant => "assistant",
        }
    }

    /// Every overlay, for operations that apply to all of them.
    const ALL: [Self; 3] = [Self::Captions, Self::Controller, Self::Assistant];
}

/// Show an overlay, positioned for the current monitor.
pub fn show(app: &AppHandle, overlay: Overlay) {
    let Some(window) = app.get_webview_window(overlay.label()) else {
        tracing::warn!(
            overlay = overlay.label(),
            "overlay window missing from this build"
        );
        return;
    };

    position(app, &window, overlay);

    window.show().ok();
    window.set_always_on_top(true).ok();

    if overlay == Overlay::Captions {
        let click_through = app
            .state::<crate::commands::AppState>()
            .settings
            .get()
            .captions
            .click_through;
        crate::commands::apply_caption_click_through(app, click_through);
    }
}

/// Place an overlay for the current monitor without showing it.
///
/// The assistant panel owns its own visibility from the frontend, appearing
/// only when it has an answer to show. This positions it beside the controller
/// ahead of time, so that when it does show itself it lands in the right place.
pub fn place(app: &AppHandle, overlay: Overlay) {
    if let Some(window) = app.get_webview_window(overlay.label()) {
        position(app, &window, overlay);
    }
}

/// Work out where an overlay belongs and move it there.
fn position(app: &AppHandle, window: &tauri::WebviewWindow, overlay: Overlay) {
    let (Ok(Some(monitor)), Ok(size)) = (window.primary_monitor(), window.outer_size()) else {
        return;
    };
    let screen = monitor.size();
    let scale = monitor.scale_factor();

    let centre_x = (i64::from(screen.width) - i64::from(size.width)) / 2;

    let (x, y) = match overlay {
        Overlay::Captions => {
            let margin = (72.0 * scale).round() as i64;
            let y = i64::from(screen.height) - i64::from(size.height) - margin;
            (centre_x, y)
        }
        Overlay::Controller => {
            let margin = (24.0 * scale).round() as i64;
            (centre_x, margin)
        }
        Overlay::Assistant => assistant_position(app, window, screen, scale),
    };

    let x = u32::try_from(x.max(0)).unwrap_or(0);
    let y = u32::try_from(y.max(0)).unwrap_or(0);
    window.set_position(tauri::PhysicalPosition::new(x, y)).ok();
}

/// Place the assistant panel beside the controller, on whichever side has room.
fn assistant_position(
    app: &AppHandle,
    window: &tauri::WebviewWindow,
    screen: &tauri::PhysicalSize<u32>,
    scale: f64,
) -> (i64, i64) {
    let size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(380, 300));
    let gap = (12.0 * scale).round() as i64;

    // Anchor to the controller if it exists, so the panel follows it around; a
    // sensible top-centre fallback otherwise.
    let (anchor_x, anchor_right, top) = app
        .get_webview_window("controller")
        .and_then(|controller| {
            let position = controller.outer_position().ok()?;
            let controller_size = controller.outer_size().ok()?;
            Some((
                i64::from(position.x),
                i64::from(position.x) + i64::from(controller_size.width),
                i64::from(position.y),
            ))
        })
        .unwrap_or_else(|| {
            let margin = (24.0 * scale).round() as i64;
            let centre = (i64::from(screen.width) - i64::from(size.width)) / 2;
            (centre, centre + i64::from(size.width), margin)
        });

    // Prefer the right of the controller, fall back to the left when the right
    // would run off the screen — that is the "depends on the clear space" rule.
    let right_x = anchor_right + gap;
    let x = if right_x + i64::from(size.width) <= i64::from(screen.width) {
        right_x
    } else {
        anchor_x - gap - i64::from(size.width)
    };

    (x, top)
}

/// Hide one overlay without ending the session.
pub fn hide(app: &AppHandle, overlay: Overlay) {
    if let Some(window) = app.get_webview_window(overlay.label()) {
        window.hide().ok();
    }
}

/// Hide every overlay. Used when a session ends or the app is put away.
pub fn hide_all(app: &AppHandle) {
    for overlay in Overlay::ALL {
        hide(app, overlay);
    }
}

/// Flip an overlay's visibility.
pub fn toggle(app: &AppHandle, overlay: Overlay) {
    let visible = app
        .get_webview_window(overlay.label())
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);

    if visible {
        hide(app, overlay);
    } else {
        show(app, overlay);
    }
}
