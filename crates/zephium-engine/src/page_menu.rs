//! The browser's own items in a page's menu take their titles from the
//! interface language, which the desktop hands over at launch and whenever
//! it changes. Until then they read in English.

use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageMenuLabels {
    pub open_link_in_panel: String,
    pub search_in_panel: String,
}

static LABELS: RwLock<Option<PageMenuLabels>> = RwLock::new(None);

/// Sets the titles of the browser's page-menu items.
pub fn set_page_menu_labels(labels: PageMenuLabels) {
    *LABELS
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(labels);
}

#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub(crate) fn labels() -> PageMenuLabels {
    LABELS
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
        .unwrap_or_else(|| PageMenuLabels {
            open_link_in_panel: "Open Link in Web Panel".into(),
            search_in_panel: "Search in Web Panel".into(),
        })
}
