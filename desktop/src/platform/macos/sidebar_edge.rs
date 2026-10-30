//! A hidden sidebar waits beneath the page, so the page's own view takes
//! every pointer move along the window's left edge and the chrome never sees
//! the pointer arrive. Reading the moves app-wide tells the chrome the moment
//! the pointer reaches the edge, or is plainly about to.

use std::cell::RefCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSEvent, NSEventMask, NSWindow};
use objc2_foundation::MainThreadMarker;
use tauri::{Manager, WebviewWindow};

use crate::edge_reach::at_edge;

struct Edge {
    window: WebviewWindow,
    number: isize,
    /// The trailing edge, where a waiting panel strip is, not the leading.
    right: bool,
    near: bool,
    last: Option<f64>,
}

/// The event an edge reports on.
fn edge_event(right: bool) -> &'static str {
    if right {
        "zephium:panel-edge"
    } else {
        "zephium:sidebar-edge"
    }
}

thread_local! {
    static EDGES: RefCell<Vec<Edge>> = const { RefCell::new(Vec::new()) };
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

fn observe(event: &NSEvent) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(window) = event.window(mtm) else {
        return;
    };
    let number = window.windowNumber();
    let x = event.locationInWindow().x;
    let width = window.frame().size.width;
    let changed = EDGES.with(|edges| {
        let Ok(mut edges) = edges.try_borrow_mut() else {
            return Vec::new();
        };
        let mut changed = Vec::new();
        for edge in edges.iter_mut().filter(|edge| edge.number == number) {
            // Distance in from the edge watched, so both sides ask alike.
            let x = if edge.right { width - x } else { x };
            let step = edge.last.map_or(0.0, |last| x - last);
            edge.last = Some(x);
            let near = at_edge(x, step, edge.near);
            if near != edge.near {
                edge.near = near;
                changed.push((edge.window.clone(), edge.right, near));
            }
        }
        changed
    });
    for (window, right, near) in changed {
        crate::emit_to_privileged(
            window.app_handle(),
            window.label(),
            edge_event(right),
            &serde_json::json!({ "near": near }),
        );
    }
}

/// Starts or stops watching `window`'s left edge. Runs on the main thread.
pub fn watch(window: &WebviewWindow, on: bool) {
    watch_side(window, false, on);
}

/// Starts or stops watching one of `window`'s side edges. Runs on the main
/// thread.
pub fn watch_side(window: &WebviewWindow, right: bool, on: bool) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    let Ok(pointer) = window.ns_window() else {
        return;
    };
    // SAFETY: Tao's live NSWindow, borrowed for this main-thread call only.
    let number = unsafe { &*(pointer as *const NSWindow) }.windowNumber();
    let watching = EDGES.with(|edges| {
        let mut edges = edges.borrow_mut();
        edges.retain(|edge| edge.number != number || edge.right != right);
        if on {
            edges.push(Edge {
                window: window.clone(),
                number,
                right,
                near: false,
                last: None,
            });
        }
        !edges.is_empty()
    });
    MONITOR.with(|monitor| {
        let mut monitor = monitor.borrow_mut();
        match (watching, monitor.is_some()) {
            (true, false) => {
                let handler = RcBlock::new(|event: NonNull<NSEvent>| -> *mut NSEvent {
                    // SAFETY: the local monitor lends its event for this call.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        observe(unsafe { event.as_ref() })
                    }));
                    event.as_ptr()
                });
                // SAFETY: an app-local main-thread monitor that hands every
                // event back untouched.
                *monitor = unsafe {
                    NSEvent::addLocalMonitorForEventsMatchingMask_handler(
                        NSEventMask::MouseMoved,
                        &handler,
                    )
                };
            }
            (false, true) => {
                if let Some(old) = monitor.take() {
                    // SAFETY: this exact registration is removed once.
                    unsafe { NSEvent::removeMonitor(&old) };
                }
            }
            _ => {}
        }
    });
}
