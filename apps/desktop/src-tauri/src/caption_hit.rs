//! Cursor-aware interactivity for the caption overlay.
//!
//! The caption window floats over the user's other apps. It must let clicks
//! reach whatever is behind it, yet still be grabbable when the pointer is
//! actually over the caption. A single OS flag, `set_ignore_cursor_events`,
//! decides this for the whole window, so this module drives that flag from the
//! real cursor position: interactive while the pointer is over a region the
//! frontend reports as hot, click-through everywhere else.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use audis_common::events;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

/// How often the cursor is sampled while idle.
const POLL: Duration = Duration::from_millis(50);

/// How often the window follows the cursor while being dragged.
const DRAG_POLL: Duration = Duration::from_millis(8);

/// A hot region, in CSS pixels relative to the caption window's viewport.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Shared caption interactivity state.
#[derive(Default)]
pub struct CaptionHot {
    /// Bumped to stop the running loop; the loop exits when it no longer matches.
    generation: AtomicU64,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    rects: Vec<HotRect>,
    last: Option<bool>,
    dragging: bool,
    offset: (f64, f64),
}

impl CaptionHot {
    /// Replace the regions the caption reports as interactive.
    pub fn set_rects(&self, rects: Vec<HotRect>) {
        self.lock().rects = rects;
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Begin tracking the cursor and toggling the caption's click-through.
pub fn start(app: &AppHandle) {
    let state = app.state::<CaptionHot>();
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let mut inner = state.lock();
        inner.last = None;
        inner.rects.clear();
    }

    let handle = app.clone();
    std::thread::Builder::new()
        .name("audis-caption-hit".to_owned())
        .spawn(move || run(handle, generation))
        .ok();
}

/// Stop tracking and hand the caption back to a plain, interactive state.
pub fn stop(app: &AppHandle) {
    let state = app.state::<CaptionHot>();
    state.generation.fetch_add(1, Ordering::SeqCst);
    if let Some(window) = app.get_webview_window("captions") {
        window.set_ignore_cursor_events(false).ok();
    }
    let mut inner = state.lock();
    inner.last = None;
    inner.dragging = false;
    inner.rects.clear();
}

/// Begin dragging the caption: remember where on the window it was grabbed, and
/// keep it interactive so the drag is not cut off by the cursor loop.
pub fn begin_drag(app: &AppHandle) {
    let Some(window) = app.get_webview_window("captions") else {
        return;
    };
    let (Ok(cursor), Ok(position)) = (window.cursor_position(), window.outer_position()) else {
        return;
    };
    window.set_ignore_cursor_events(false).ok();

    let state = app.state::<CaptionHot>();
    let mut inner = state.lock();
    inner.dragging = true;
    inner.offset = (
        cursor.x - f64::from(position.x),
        cursor.y - f64::from(position.y),
    );
    inner.last = Some(true);
}

/// Stop dragging the caption.
pub fn end_drag(app: &AppHandle) {
    let state = app.state::<CaptionHot>();
    let mut inner = state.lock();
    inner.dragging = false;
    inner.last = None;
}

fn run(app: AppHandle, generation: u64) {
    let mut nap = POLL;
    loop {
        std::thread::sleep(nap);
        nap = POLL;

        let state = app.state::<CaptionHot>();
        if state.generation.load(Ordering::SeqCst) != generation {
            break;
        }

        let Some(window) = app.get_webview_window("captions") else {
            break;
        };
        if !window.is_visible().unwrap_or(false) {
            continue;
        }

        let (rects, last, dragging, offset) = {
            let inner = state.lock();
            (
                inner.rects.clone(),
                inner.last,
                inner.dragging,
                inner.offset,
            )
        };

        if dragging {
            if let Ok(cursor) = window.cursor_position() {
                let x = (cursor.x - offset.0).round() as i32;
                let y = (cursor.y - offset.1).round() as i32;
                window.set_position(tauri::PhysicalPosition::new(x, y)).ok();
            }
            nap = DRAG_POLL;
            continue;
        }

        let interactive = cursor_over(&window, &rects);
        if Some(interactive) == last {
            continue;
        }

        window.set_ignore_cursor_events(!interactive).ok();
        app.emit(events::CAPTION_ACTIVE, interactive).ok();
        state.lock().last = Some(interactive);
    }
}

/// True when the cursor is over any hot region. Empty regions fail safe to
/// interactive, so a caption that never reports its bounds is never dead to the
/// mouse.
fn cursor_over(window: &tauri::WebviewWindow, rects: &[HotRect]) -> bool {
    if rects.is_empty() {
        return true;
    }

    let (Ok(cursor), Ok(origin)) = (window.cursor_position(), window.inner_position()) else {
        return true;
    };
    let scale = window.scale_factor().unwrap_or(1.0);

    rects.iter().any(|rect| {
        let x0 = f64::from(origin.x) + rect.x * scale;
        let y0 = f64::from(origin.y) + rect.y * scale;
        cursor.x >= x0
            && cursor.x < x0 + rect.w * scale
            && cursor.y >= y0
            && cursor.y < y0 + rect.h * scale
    })
}
