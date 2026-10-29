//! Web panels: sites docked beside the page and shared by every space of a
//! profile. A panel is a tab item placed at `Placement::Panel`, so it keeps a
//! tab's whole lifecycle; this holds what only a panel has, its width and
//! which of a profile's panels is open.

use std::collections::HashMap;

use crate::ids::{ItemId, ProfileId};

/// A profile keeps at most this many panels.
pub const MAX_PROFILE_PANELS: usize = 24;
/// Panel widths in points. A panel narrower than a phone page is unusable,
/// and one wider than this leaves the page too little room.
pub const MIN_PANEL_WIDTH: u32 = 280;
pub const MAX_PANEL_WIDTH: u32 = 960;
pub const DEFAULT_PANEL_WIDTH: u32 = 400;

pub fn clamp_panel_width(width: f64) -> u32 {
    if !width.is_finite() {
        return DEFAULT_PANEL_WIDTH;
    }
    // In range after the clamp, so the cast cannot truncate meaningfully.
    width
        .round()
        .clamp(f64::from(MIN_PANEL_WIDTH), f64::from(MAX_PANEL_WIDTH)) as u32
}

#[derive(Clone, Debug, Default)]
pub struct Panels {
    widths: HashMap<ItemId, u32>,
    open: HashMap<ProfileId, ItemId>,
}

impl Panels {
    pub fn width(&self, panel: ItemId) -> u32 {
        self.widths
            .get(&panel)
            .copied()
            .unwrap_or(DEFAULT_PANEL_WIDTH)
    }

    /// Stores a width, clamped into range. Returns whether it changed.
    pub fn set_width(&mut self, panel: ItemId, width: f64) -> bool {
        let width = clamp_panel_width(width);
        if width == DEFAULT_PANEL_WIDTH {
            return self.widths.remove(&panel).is_some();
        }
        self.widths.insert(panel, width) != Some(width)
    }

    /// Widths that differ from the default, for persistence.
    pub fn widths(&self) -> impl Iterator<Item = (ItemId, u32)> + '_ {
        self.widths.iter().map(|(id, width)| (*id, *width))
    }

    /// The panel shown beside the page in `profile`'s windows, if any.
    pub fn open(&self, profile: ProfileId) -> Option<ItemId> {
        self.open.get(&profile).copied()
    }

    pub fn opened(&self) -> impl Iterator<Item = (ProfileId, ItemId)> + '_ {
        self.open.iter().map(|(profile, id)| (*profile, *id))
    }

    /// Opens `panel` in `profile`, or closes whatever is open with `None`.
    /// The caller proves the panel is the profile's. Returns whether it changed.
    pub fn set_open(&mut self, profile: ProfileId, panel: Option<ItemId>) -> bool {
        match panel {
            Some(id) => self.open.insert(profile, id) != Some(id),
            None => self.open.remove(&profile).is_some(),
        }
    }

    /// Forgets a removed panel: its width, and being open anywhere.
    pub fn forget(&mut self, panel: ItemId) {
        self.widths.remove(&panel);
        self.open.retain(|_, open| *open != panel);
    }

    /// Forgets everything of a removed profile. Its panels' widths go with
    /// the items through [`Panels::forget`].
    pub fn forget_profile(&mut self, profile: ProfileId) {
        self.open.remove(&profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_are_clamped_and_the_default_is_not_stored() {
        let mut panels = Panels::default();
        let id = ItemId::from(1);
        assert_eq!(panels.width(id), DEFAULT_PANEL_WIDTH);
        assert!(panels.set_width(id, 10.0));
        assert_eq!(panels.width(id), MIN_PANEL_WIDTH);
        assert!(!panels.set_width(id, 100.0));
        assert!(panels.set_width(id, 5000.0));
        assert_eq!(panels.width(id), MAX_PANEL_WIDTH);
        assert!(panels.set_width(id, f64::NAN));
        assert_eq!(panels.widths().count(), 0);
        assert_eq!(clamp_panel_width(512.4), 512);
    }

    #[test]
    fn one_panel_is_open_per_profile_and_forgetting_closes_it() {
        let mut panels = Panels::default();
        let (personal, work) = (ProfileId::from(1), ProfileId::from(2));
        let (chat, music) = (ItemId::from(10), ItemId::from(11));
        assert!(panels.set_open(personal, Some(chat)));
        assert!(!panels.set_open(personal, Some(chat)));
        assert!(panels.set_open(personal, Some(music)));
        assert!(panels.set_open(work, Some(chat)));
        panels.forget(chat);
        assert_eq!(panels.open(personal), Some(music));
        assert_eq!(panels.open(work), None);
        panels.forget_profile(personal);
        assert_eq!(panels.open(personal), None);
        assert!(!panels.set_open(personal, None));
    }
}
