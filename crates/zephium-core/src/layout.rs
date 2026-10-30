//! Pure window layout in top-left logical coordinates. Chrome and content tile
//! without overlap, so the two webviews never contend for the cursor, except
//! while the sidebar is hidden: then the page takes the window and the chrome
//! keeps the column's width beneath it, ready to come over it.

use crate::geometry::{Rect, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sidebar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub sidebar_width: f64,
    pub padding: f64,
    pub gap: f64,
    /// Optional reserved title band. Native overlay controls reserve no space.
    pub caption: f64,
    /// The column is put away until the pointer asks for it.
    pub sidebar_hidden: bool,
    /// The web panel icon strip at the window's trailing edge, padding
    /// included; zero while the profile has no panels.
    pub panel_strip: f64,
    /// The open web panel's width; zero while none is open.
    pub panel: f64,
    /// The open panel floats over the page instead of narrowing it.
    pub panel_floats: bool,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            sidebar_width: 240.0,
            padding: 8.0,
            gap: 8.0,
            caption: 0.0,
            sidebar_hidden: false,
            panel_strip: 0.0,
            panel: 0.0,
            panel_floats: false,
        }
    }
}

/// The sidebar's admitted width range, spanning the compact rail through the
/// widest expanded column.
///
/// This is the single authority. It used to be restated in the shell actor and
/// again in the privileged IPC guard, which let the rail pass the boundary
/// check and then get silently widened back by the actor, so the chrome and
/// the content region disagreed about where the sidebar ended.
pub const MIN_SIDEBAR_WIDTH: f64 = 56.0;
pub const MAX_SIDEBAR_WIDTH: f64 = 460.0;

pub fn clamp_sidebar_width(width: f64) -> f64 {
    if !width.is_finite() {
        return MIN_SIDEBAR_WIDTH;
    }
    width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH)
}

/// The Work browser pane never shrinks below a usable page; the frame around
/// it is drawn by chrome from the applied rect, so clamping is safe here.
pub const MIN_WORK_PANE_WIDTH: f64 = 480.0;
pub const MIN_WORK_PANE_HEIGHT: f64 = 320.0;

/// Fits a requested Work pane rect into the window. `None` when the request
/// is not finite or the window cannot host the minimum pane.
pub fn clamp_work_pane_rect(window: Size, rect: Rect) -> Option<Rect> {
    if ![
        rect.x,
        rect.y,
        rect.width,
        rect.height,
        window.width,
        window.height,
    ]
    .iter()
    .all(|value| value.is_finite())
        || window.width < MIN_WORK_PANE_WIDTH
        || window.height < MIN_WORK_PANE_HEIGHT
    {
        return None;
    }
    let width = rect.width.clamp(MIN_WORK_PANE_WIDTH, window.width);
    let height = rect.height.clamp(MIN_WORK_PANE_HEIGHT, window.height);
    Some(Rect::new(
        rect.x.clamp(0.0, window.width - width),
        rect.y.clamp(0.0, window.height - height),
        width,
        height,
    ))
}

/// The page keeps at least this much width beside an open panel; a window
/// too narrow for both does not show the panel.
pub const MIN_CONTENT_BESIDE_PANEL: f64 = 360.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub chrome: Rect,
    // None on an empty tab: chrome fills the inner area and renders the NTP.
    pub content: Option<Rect>,
    /// The open web panel, docked at the trailing edge beside the content,
    /// or beside the New Tab page chrome draws.
    pub panel: Option<Rect>,
}

/// How far past the column a hidden sidebar's chrome reaches, so the shadow
/// it casts over the page while it is shown is not cut off.
pub const HIDDEN_SHADOW_REACH: f64 = 64.0;

/// The gap a hidden sidebar's card keeps from the window's left, top and
/// bottom, and the padding it keeps inside its own edge.
pub const HIDDEN_CARD_INSET: f64 = 8.0;

/// How far from the window's left the card ends, given the width of the
/// chrome a hidden sidebar keeps beneath the page: the column and its
/// gutter, the shadow's reach, and the card's padding beyond the column.
pub fn hidden_card_edge(chrome_width: f64) -> f64 {
    (chrome_width - HIDDEN_SHADOW_REACH + 2.0 * HIDDEN_CARD_INSET).max(0.0)
}

/// Where a hidden sidebar's card stands while it is over the page: the column
/// with the inset all round it, as the frame draws it.
pub fn hidden_card(window: Size, m: Metrics) -> Rect {
    let inset = HIDDEN_CARD_INSET;
    Rect::new(
        inset,
        inset,
        (m.sidebar_width + 2.0 * inset).min((window.width - inset).max(0.0)),
        (window.height - 2.0 * inset).max(0.0),
    )
}

pub fn compute(window: Size, mode: Mode, m: Metrics, content_present: bool) -> Layout {
    match mode {
        Mode::Sidebar => sidebar(window, m, content_present),
    }
}

/// Chrome is anchored at the window's corner and draws the gutter around the
/// column itself, so its origin is the same in every shape and only its width
/// changes. Moving the origin between shapes made the whole column jump for
/// the frames between native moving the view and the frame repadding it.
fn sidebar(win: Size, m: Metrics, content_present: bool) -> Layout {
    let p = m.padding;
    let inner_h = win.height - 2.0 * p;
    let full = Rect::new(0.0, 0.0, win.width, win.height);
    let content_x = if m.sidebar_hidden {
        p
    } else {
        p + m.sidebar_width + m.gap
    };
    // The strip carries its own padding; without it the window's padding
    // ends the content as before.
    let strip = m.panel_strip.max(0.0);
    let right = win.width - if strip > 0.0 { strip } else { p };
    let top = p + m.caption;
    let height = inner_h - m.caption;
    let panel = docked_panel(content_x, right, top, height, m);
    let content_right = match panel {
        Some(panel) if !m.panel_floats => panel.x - m.gap,
        _ => right,
    };

    if !content_present {
        return Layout {
            chrome: full,
            content: None,
            panel,
        };
    }

    // Hidden, the column keeps its width beneath the page, so it can come
    // over it without laying out again, and the page starts at the gutter.
    let chrome_width = if m.sidebar_hidden {
        (p + m.sidebar_width + HIDDEN_SHADOW_REACH).min(win.width)
    } else {
        p + m.sidebar_width
    };
    // The strip is chrome at the window's far edge, so chrome spans the
    // window beneath the page; the page views stay above it.
    let chrome = if strip > 0.0 {
        full
    } else {
        Rect::new(0.0, 0.0, chrome_width, win.height)
    };
    let content = Rect::new(content_x, top, content_right - content_x, height);
    Layout {
        chrome,
        content: Some(content),
        panel,
    }
}

/// The open panel at the trailing edge of `[left, right]`, narrowed so the
/// page keeps its minimum, or none when the window is too narrow for both.
fn docked_panel(left: f64, right: f64, top: f64, height: f64, m: Metrics) -> Option<Rect> {
    if !(m.panel.is_finite() && m.panel > 0.0) {
        return None;
    }
    // Floating, the page keeps its width beneath, so the panel needs only
    // to leave some of it in sight.
    let keep = if m.panel_floats {
        MIN_CONTENT_BESIDE_PANEL / 2.0
    } else {
        MIN_CONTENT_BESIDE_PANEL
    };
    let room = right - left - m.gap - keep;
    let width = m.panel.min(room);
    (width >= f64::from(crate::panels::MIN_PANEL_WIDTH) && height > 0.0)
        .then(|| Rect::new(right - width, top, width, height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sidebar_clamp_admits_the_rail_and_rejects_nonsense() {
        assert_eq!(clamp_sidebar_width(MIN_SIDEBAR_WIDTH), MIN_SIDEBAR_WIDTH);
        assert_eq!(clamp_sidebar_width(MAX_SIDEBAR_WIDTH), MAX_SIDEBAR_WIDTH);
        assert_eq!(clamp_sidebar_width(0.0), MIN_SIDEBAR_WIDTH);
        assert_eq!(clamp_sidebar_width(10_000.0), MAX_SIDEBAR_WIDTH);
        assert_eq!(clamp_sidebar_width(f64::NAN), MIN_SIDEBAR_WIDTH);
        assert_eq!(clamp_sidebar_width(240.0), 240.0);
    }

    const M: Metrics = Metrics {
        panel: 0.0,
        panel_strip: 0.0,
        sidebar_width: 240.0,
        padding: 8.0,
        gap: 8.0,
        caption: 0.0,
        sidebar_hidden: false,
        panel_floats: false,
    };

    #[test]
    fn the_panel_strip_takes_the_trailing_edge_and_chrome_spans_the_window() {
        let m = Metrics {
            panel_strip: 48.0,
            ..M
        };
        let l = compute(Size::new(1200.0, 800.0), Mode::Sidebar, m, true);
        assert_eq!(l.chrome, Rect::new(0.0, 0.0, 1200.0, 800.0));
        assert_eq!(l.content, Some(Rect::new(256.0, 8.0, 896.0, 784.0)));
        assert_eq!(l.panel, None);
    }

    #[test]
    fn an_open_panel_docks_beside_the_page_and_beside_the_new_tab() {
        let m = Metrics {
            panel_strip: 48.0,
            panel: 400.0,
            ..M
        };
        let window = Size::new(1200.0, 800.0);
        let l = compute(window, Mode::Sidebar, m, true);
        assert_eq!(l.panel, Some(Rect::new(752.0, 8.0, 400.0, 784.0)));
        assert_eq!(l.content, Some(Rect::new(256.0, 8.0, 488.0, 784.0)));
        let blank = compute(window, Mode::Sidebar, m, false);
        assert_eq!(blank.content, None);
        assert_eq!(blank.panel, l.panel);
    }

    #[test]
    fn a_floating_panel_leaves_the_page_its_width() {
        let docked = Metrics {
            panel_strip: 48.0,
            panel: 400.0,
            ..M
        };
        let floats = Metrics {
            panel_floats: true,
            ..docked
        };
        let size = Size::new(1200.0, 800.0);
        let beside = sidebar(size, docked, true);
        let over = sidebar(size, floats, true);
        assert_eq!(over.panel, beside.panel);
        let content = over.content.unwrap();
        assert_eq!(content.x + content.width, 1200.0 - 48.0);
        assert!(beside.content.unwrap().width < content.width);
    }

    #[test]
    fn a_panel_narrows_for_the_page_and_goes_when_both_cannot_fit() {
        let m = Metrics {
            panel_strip: 48.0,
            panel: 600.0,
            ..M
        };
        let l = compute(Size::new(1100.0, 800.0), Mode::Sidebar, m, true);
        let panel = l.panel.unwrap();
        let content = l.content.unwrap();
        assert_eq!(content.width, MIN_CONTENT_BESIDE_PANEL);
        assert_eq!(panel.x, content.x + content.width + M.gap);
        assert_eq!(panel.x + panel.width, 1100.0 - 48.0);
        let narrow = compute(Size::new(900.0, 800.0), Mode::Sidebar, m, true);
        assert_eq!(narrow.panel, None);
        assert_eq!(
            narrow.content.unwrap().x + narrow.content.unwrap().width,
            900.0 - 48.0
        );
    }

    #[test]
    fn a_hidden_sidebar_gives_the_page_the_window_and_waits_beneath_it() {
        let m = Metrics {
            sidebar_hidden: true,
            ..M
        };
        let l = compute(Size::new(1200.0, 800.0), Mode::Sidebar, m, true);
        assert_eq!(l.content, Some(Rect::new(8.0, 8.0, 1184.0, 784.0)));
        assert_eq!(
            l.chrome,
            Rect::new(0.0, 0.0, 248.0 + HIDDEN_SHADOW_REACH, 800.0)
        );
        // Without a page there is nothing to hide beneath.
        let empty = compute(Size::new(1200.0, 800.0), Mode::Sidebar, m, false);
        assert_eq!(empty.chrome, Rect::new(0.0, 0.0, 1200.0, 800.0));
    }

    #[test]
    fn the_hidden_card_is_the_column_with_its_inset_and_stays_in_the_window() {
        assert_eq!(
            hidden_card(Size::new(1200.0, 800.0), M),
            Rect::new(8.0, 8.0, 256.0, 784.0)
        );
        assert_eq!(
            hidden_card(Size::new(200.0, 100.0), M),
            Rect::new(8.0, 8.0, 192.0, 84.0)
        );
        // The chrome beneath a hidden 240 column is 8 + 240 + reach wide.
        let chrome = compute(
            Size::new(1200.0, 800.0),
            Mode::Sidebar,
            Metrics {
                sidebar_hidden: true,
                ..M
            },
            true,
        )
        .chrome;
        let card = hidden_card(Size::new(1200.0, 800.0), M);
        assert_eq!(hidden_card_edge(chrome.width), card.x + card.width);
        let tiny = hidden_card(Size::new(4.0, 4.0), M);
        assert_eq!((tiny.width, tiny.height), (0.0, 0.0));
    }

    #[test]
    fn a_title_band_lowers_the_page_and_leaves_the_sidebar_full_height() {
        let m = Metrics { caption: 32.0, ..M };
        let l = compute(Size::new(1200.0, 800.0), Mode::Sidebar, m, true);
        assert_eq!(l.chrome, Rect::new(0.0, 0.0, 248.0, 800.0));
        assert_eq!(l.content, Some(Rect::new(256.0, 40.0, 936.0, 752.0)));
    }

    #[test]
    fn native_overlay_controls_reserve_no_page_space() {
        let l = compute(
            Size::new(1200.0, 800.0),
            Mode::Sidebar,
            Metrics::default(),
            true,
        );
        assert_eq!(l.content, Some(Rect::new(256.0, 8.0, 936.0, 784.0)));
    }

    #[test]
    fn sidebar_with_content_tiles_without_overlap() {
        let l = compute(Size::new(1200.0, 800.0), Mode::Sidebar, M, true);
        assert_eq!(l.chrome, Rect::new(0.0, 0.0, 248.0, 800.0));
        let content = l.content.unwrap();
        assert_eq!(content, Rect::new(256.0, 8.0, 936.0, 784.0));
        // chrome right edge + gap == content left edge: no overlap.
        assert_eq!(l.chrome.x + l.chrome.width + M.gap, content.x);
    }

    #[test]
    fn empty_tab_lets_chrome_fill_the_window() {
        let l = compute(Size::new(1200.0, 800.0), Mode::Sidebar, M, false);
        assert_eq!(l.chrome, Rect::new(0.0, 0.0, 1200.0, 800.0));
        assert!(l.content.is_none());
    }

    #[test]
    fn the_chrome_origin_is_the_same_in_every_shape() {
        let window = Size::new(1200.0, 800.0);
        let with_page = compute(window, Mode::Sidebar, M, true).chrome;
        let without = compute(window, Mode::Sidebar, M, false).chrome;
        assert_eq!((with_page.x, with_page.y), (without.x, without.y));
    }

    #[test]
    fn work_pane_rect_is_fitted_into_the_window_or_refused() {
        let window = Size::new(1200.0, 800.0);
        assert_eq!(
            clamp_work_pane_rect(window, Rect::new(100.0, 80.0, 700.0, 500.0)),
            Some(Rect::new(100.0, 80.0, 700.0, 500.0))
        );
        assert_eq!(
            clamp_work_pane_rect(window, Rect::new(900.0, 700.0, 100.0, 100.0)),
            Some(Rect::new(720.0, 480.0, 480.0, 320.0))
        );
        assert_eq!(
            clamp_work_pane_rect(window, Rect::new(-50.0, -50.0, 5000.0, 5000.0)),
            Some(Rect::new(0.0, 0.0, 1200.0, 800.0))
        );
        assert_eq!(
            clamp_work_pane_rect(window, Rect::new(f64::NAN, 0.0, 600.0, 400.0)),
            None
        );
        assert_eq!(
            clamp_work_pane_rect(Size::new(400.0, 300.0), Rect::new(0.0, 0.0, 600.0, 400.0)),
            None
        );
    }

    #[test]
    fn degenerate_window_clamps_to_nonnegative() {
        let l = compute(Size::new(10.0, 10.0), Mode::Sidebar, M, true);
        let content = l.content.unwrap();
        assert!(content.width >= 0.0 && content.height >= 0.0);
        assert!(l.chrome.height >= 0.0);
    }
}
