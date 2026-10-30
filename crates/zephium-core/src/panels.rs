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
/// The icon strip at the window's trailing edge, its padding included.
pub const PANEL_STRIP_WIDTH: f64 = 52.0;

pub fn clamp_panel_width(width: f64) -> u32 {
    if !width.is_finite() {
        return DEFAULT_PANEL_WIDTH;
    }
    // In range after the clamp, so the cast cannot truncate meaningfully.
    width
        .round()
        .clamp(f64::from(MIN_PANEL_WIDTH), f64::from(MAX_PANEL_WIDTH)) as u32
}

pub const MAX_PANEL_NAME_CHARS: usize = 64;
/// An unload timer puts a closed panel to sleep after this many idle minutes.
pub const MIN_UNLOAD_AFTER_MINUTES: u32 = 1;
pub const MAX_UNLOAD_AFTER_MINUTES: u32 = 24 * 60;
/// An auto-reload timer reloads an awake panel this often, in seconds.
pub const MIN_RELOAD_EVERY_SECONDS: u32 = 30;
pub const MAX_RELOAD_EVERY_SECONDS: u32 = 24 * 60 * 60;

/// What a person chose for one panel. The default is a panel that loads
/// when first opened, stays awake until the lifecycle needs its memory,
/// and asks for the desktop site.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PanelSettings {
    /// Shown instead of the page's title; none follows the page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Ask sites for their phone layout, which fits a narrow panel.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mobile: bool,
    /// Load when the browser starts, not only when first opened.
    #[serde(default, skip_serializing_if = "is_false")]
    pub load_on_startup: bool,
    /// Put the page to sleep as soon as the panel is closed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub unload_on_close: bool,
    /// Put a closed panel's page to sleep after this many idle minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unload_after_minutes: Option<u32>,
    /// Reload the page this often while it is awake.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reload_every_seconds: Option<u32>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

impl PanelSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The settings as they are kept: a bounded name stripped of controls
    /// (none when nothing is left), and timers inside their ranges.
    pub fn sanitized(self) -> Self {
        let name = self.name.and_then(|name| {
            let name: String = crate::session::bounded_name(name.trim(), "")
                .chars()
                .take(MAX_PANEL_NAME_CHARS)
                .collect();
            let name = name.trim().to_owned();
            (!name.is_empty()).then_some(name)
        });
        Self {
            name,
            unload_after_minutes: self
                .unload_after_minutes
                .map(|m| m.clamp(MIN_UNLOAD_AFTER_MINUTES, MAX_UNLOAD_AFTER_MINUTES)),
            reload_every_seconds: self
                .reload_every_seconds
                .map(|s| s.clamp(MIN_RELOAD_EVERY_SECONDS, MAX_RELOAD_EVERY_SECONDS)),
            ..self
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Panels {
    widths: HashMap<ItemId, u32>,
    open: HashMap<ProfileId, ItemId>,
    settings: HashMap<ItemId, PanelSettings>,
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

    /// A panel's settings; the default for one never configured.
    pub fn settings(&self, panel: ItemId) -> PanelSettings {
        self.settings.get(&panel).cloned().unwrap_or_default()
    }

    /// Stores settings, sanitized; the default is not kept. Returns whether
    /// they changed.
    pub fn set_settings(&mut self, panel: ItemId, settings: PanelSettings) -> bool {
        let settings = settings.sanitized();
        if settings.is_default() {
            return self.settings.remove(&panel).is_some();
        }
        self.settings.insert(panel, settings.clone()) != Some(settings)
    }

    /// Settings that differ from the default, for persistence.
    pub fn configured(&self) -> impl Iterator<Item = (ItemId, &PanelSettings)> + '_ {
        self.settings.iter().map(|(id, settings)| (*id, settings))
    }

    /// Forgets a removed panel: its width, its settings, and being open.
    pub fn forget(&mut self, panel: ItemId) {
        self.widths.remove(&panel);
        self.settings.remove(&panel);
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
    fn settings_are_sanitized_and_the_default_is_not_stored() {
        let mut panels = Panels::default();
        let id = ItemId::from(1);
        assert!(panels.set_settings(
            id,
            PanelSettings {
                name: Some("  Team\u{202e} Chat \n".into()),
                unload_after_minutes: Some(0),
                reload_every_seconds: Some(5),
                ..PanelSettings::default()
            },
        ));
        let kept = panels.settings(id);
        assert_eq!(kept.name.as_deref(), Some("Team Chat"));
        assert_eq!(kept.unload_after_minutes, Some(MIN_UNLOAD_AFTER_MINUTES));
        assert_eq!(kept.reload_every_seconds, Some(MIN_RELOAD_EVERY_SECONDS));
        assert!(panels.set_settings(
            id,
            PanelSettings {
                name: Some("   ".into()),
                ..PanelSettings::default()
            },
        ));
        assert_eq!(panels.configured().count(), 0);
        panels.set_settings(
            id,
            PanelSettings {
                mobile: true,
                ..PanelSettings::default()
            },
        );
        panels.forget(id);
        assert!(panels.settings(id).is_default());
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
