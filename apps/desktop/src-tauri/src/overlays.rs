//! The floating windows that appear during a session.

use tauri::{AppHandle, Manager};

/// One of the floating session windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// The caption bar along the bottom of the screen.
    Captions,
    /// The small controller chip.
    Controller,
}

impl Overlay {
    /// The Tauri window label.
    fn label(self) -> &'static str {
        match self {
            Self::Captions => "captions",
            Self::Controller => "controller",
        }
    }

    /// Every overlay, for operations that apply to all of them.
    const ALL: [Self; 2] = [Self::Captions, Self::Controller];
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

    if let (Ok(Some(monitor)), Ok(size)) = (window.primary_monitor(), window.outer_size()) {
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
        };

        let x = u32::try_from(x.max(0)).unwrap_or(0);
        let y = u32::try_from(y.max(0)).unwrap_or(0);
        window.set_position(tauri::PhysicalPosition::new(x, y)).ok();
    }

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
