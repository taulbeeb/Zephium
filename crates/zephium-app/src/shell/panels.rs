//! Web panels of the focused profile: sites docked beside the page, shared by
//! every space. A panel is a tab item at `Placement::Panel`, so its view,
//! title, icon, audio, container and persistence are a tab's; this module
//! owns only what a panel adds. Every command is checked against the focused
//! persistent profile.

use super::*;
use crate::api::{PanelCommand, PanelNavigation, PanelZoom};
use zephium_core::panels::{PanelSettings, MAX_PROFILE_PANELS, PANEL_STRIP_WIDTH};

/// When each panel was last closed and last reloaded, for its timers.
/// Runtime only: a relaunch starts every timer afresh.
#[derive(Default)]
pub(super) struct PanelTimers {
    closed_at: std::collections::HashMap<ItemId, std::time::Instant>,
    reloaded_at: std::collections::HashMap<ItemId, std::time::Instant>,
}

#[cfg(test)]
impl PanelTimers {
    /// Moves one panel's close and another's last reload into the past.
    pub(super) fn age_for_test(
        &mut self,
        closed: ItemId,
        reloaded: ItemId,
        at: std::time::Instant,
    ) {
        self.closed_at.insert(closed, at);
        self.reloaded_at.insert(reloaded, at);
    }
}

/// The style that shows only what a panel's selector finds.
fn isolation_css(selector: Option<&str>) -> Option<std::sync::Arc<str>> {
    selector
        .and_then(zephium_core::panels::isolation_style)
        .map(std::sync::Arc::from)
}

fn rejected(reason: OperationReason) -> OperationDisposition {
    operation_result(OperationOutcome::Rejected, reason)
}

fn unchanged() -> OperationDisposition {
    operation_result(OperationOutcome::NoOp, OperationReason::StateUnchanged)
}

impl Shell {
    pub(super) fn operation_panel(&mut self, command: PanelCommand) -> OperationDisposition {
        let Some(profile) = self.panel_scope() else {
            return rejected(OperationReason::InvalidScope);
        };
        match command {
            PanelCommand::Add { url, container } => self.add_panel(profile, &url, container),
            PanelCommand::AddTab(tab) => {
                let Some(space) = self.windows.focused().map(|window| window.space) else {
                    return rejected(OperationReason::NoFocusedWindow);
                };
                if !self.item_in_scope(tab, profile, space) {
                    return rejected(OperationReason::InvalidScope);
                }
                let Some(state) = self
                    .items
                    .tab(tab)
                    .filter(|state| state.content == zephium_core::item::TabContent::Web)
                else {
                    return rejected(OperationReason::InvalidInput);
                };
                let Some(url) = state.url.as_ref().map(ToString::to_string) else {
                    return rejected(OperationReason::InvalidInput);
                };
                let container = state.container();
                self.add_panel(profile, &url, container)
            }
            PanelCommand::Remove(id) => self.remove_panel(profile, id),
            PanelCommand::Move { id, before } => self.move_panel(profile, id, before),
            PanelCommand::Open(id) => self.open_panel(profile, id),
            PanelCommand::Toggle => {
                let open = self.panels.open(profile);
                match open {
                    Some(_) => self.open_panel(profile, None),
                    None => {
                        let next = self
                            .panel_last
                            .get(&profile)
                            .copied()
                            .filter(|id| self.panel_in_scope(*id, profile))
                            .or_else(|| self.profile_panels(profile).first().copied());
                        match next {
                            Some(id) => self.open_panel(profile, Some(id)),
                            None => unchanged(),
                        }
                    }
                }
            }
            PanelCommand::Resize { id, width } => {
                if !self.panel_in_scope(id, profile) {
                    return rejected(OperationReason::InvalidScope);
                }
                if !self.panels.set_width(id, width) {
                    return unchanged();
                }
                self.panels_changed()
            }
            PanelCommand::SetContainer { id, container } => {
                self.set_panel_container(profile, id, container)
            }
            PanelCommand::Load { id, input } => {
                if !self.panel_in_scope(id, profile) {
                    return rejected(OperationReason::InvalidScope);
                }
                self.operation_navigate(id, input)
            }
            PanelCommand::Navigate { id, action } => {
                if !self.panel_in_scope(id, profile) {
                    return rejected(OperationReason::InvalidScope);
                }
                if action == PanelNavigation::Reload {
                    self.panel_timers
                        .reloaded_at
                        .insert(id, std::time::Instant::now());
                }
                match action {
                    PanelNavigation::Back => self.operation_history(id, false),
                    PanelNavigation::Forward => self.operation_history(id, true),
                    PanelNavigation::Reload => self.operation_reload(id),
                }
            }
            PanelCommand::Configure { id, settings } => self.configure_panel(profile, id, settings),
            PanelCommand::Zoom { id, step } => {
                if !self.panel_in_scope(id, profile) {
                    return rejected(OperationReason::InvalidScope);
                }
                self.adjust_zoom_of(
                    id,
                    match step {
                        PanelZoom::In => ZoomStep::In,
                        PanelZoom::Out => ZoomStep::Out,
                        PanelZoom::Reset => ZoomStep::Reset,
                    },
                )
            }
            PanelCommand::MoveTab(tab) => {
                let added = self.operation_panel(PanelCommand::AddTab(tab));
                if added.outcome == OperationOutcome::Rejected {
                    return added;
                }
                // The page reloads in the panel; the tab it came from goes.
                let closed = self.operation_close(tab);
                if closed.outcome == OperationOutcome::Rejected {
                    return closed;
                }
                added
            }
            PanelCommand::Select(position) => {
                let Some(id) = position
                    .checked_sub(1)
                    .and_then(|at| self.profile_panels(profile).get(at).copied())
                else {
                    return unchanged();
                };
                let target = (self.panels.open(profile) != Some(id)).then_some(id);
                self.open_panel(profile, target)
            }
        }
    }

    /// The focused profile when it may have panels: persistent and not being
    /// deleted.
    fn panel_scope(&self) -> Option<ProfileId> {
        let window = self.windows.focused()?;
        (self
            .profiles
            .get(window.profile)
            .is_some_and(|profile| profile.kind != ProfileKind::Incognito)
            && !self.profile_deletion_quarantines(window.profile))
        .then_some(window.profile)
    }

    /// A panel of the focused window's profile.
    pub(super) fn panel_in_focused_scope(&self, id: ItemId) -> bool {
        self.panel_scope()
            .is_some_and(|profile| self.panel_in_scope(id, profile))
    }

    pub(super) fn profile_panels(&self, profile: ProfileId) -> Vec<ItemId> {
        self.items.roots(Placement::Panel { profile }).to_vec()
    }

    fn panels_changed(&mut self) -> OperationDisposition {
        self.schedule_persist();
        self.project_items();
        let mut native = NativeWork::default();
        native.record(self.relayout_with(true));
        mutation_result(native)
    }

    fn add_panel(
        &mut self,
        profile: ProfileId,
        input: &str,
        container: Option<zephium_core::ids::ContainerId>,
    ) -> OperationDisposition {
        if self.profile_panels(profile).len() >= MAX_PROFILE_PANELS {
            return rejected(OperationReason::ItemLimitReached);
        }
        if container.is_some_and(|id| !self.containers.belongs_to(id, profile)) {
            return rejected(OperationReason::InvalidScope);
        }
        let Some(url) = self
            .search
            .engine
            .configured_classify(input, &self.search.custom_url)
            .filter(|url| matches!(url.scheme(), "http" | "https"))
        else {
            return rejected(OperationReason::InvalidInput);
        };
        let placement = Placement::Panel { profile };
        let Some(id) = (0..8).find_map(|_| {
            let candidate = ItemId::generate();
            self.items
                .insert_unloaded_tab(candidate, placement, url.clone(), url.as_str())
                .then_some(candidate)
        }) else {
            return rejected(OperationReason::ItemLimitReached);
        };
        if !self.items.assign_container(id, container) {
            let _ = self.items.remove(id);
            return rejected(OperationReason::InvalidInput);
        }
        self.engine.set_mobile_user_agent(id, false);
        self.open_panel(profile, Some(id))
    }

    fn remove_panel(&mut self, profile: ProfileId, id: ItemId) -> OperationDisposition {
        if !self.panel_in_scope(id, profile) {
            return rejected(OperationReason::InvalidScope);
        }
        self.panels.forget(id);
        self.panel_last.retain(|_, last| *last != id);
        self.panel_timers.closed_at.remove(&id);
        self.panel_timers.reloaded_at.remove(&id);
        self.engine.set_mobile_user_agent(id, false);
        self.engine.set_item_style(id, None);
        self.cancel_page_permission_for_item(id);
        self.forget_page_permission_documents_of_item(id);
        self.cancel_pending_presentation(id);
        self.cancel_favicon_attempt(id);
        self.cancel_discard_probe(id);
        let effects = self.items.remove(id);
        let mut native = self.commit(effects);
        self.schedule_persist();
        self.project_items();
        native.record(self.relayout_with(true));
        mutation_result(native)
    }

    fn move_panel(
        &mut self,
        profile: ProfileId,
        id: ItemId,
        before: Option<ItemId>,
    ) -> OperationDisposition {
        if !self.panel_in_scope(id, profile)
            || before.is_some_and(|before| before == id || !self.panel_in_scope(before, profile))
        {
            return rejected(OperationReason::InvalidScope);
        }
        let placement = Placement::Panel { profile };
        let order = self.profile_panels(profile);
        let at = order.iter().position(|candidate| *candidate == id);
        let next = at.and_then(|at| order.get(at + 1)).copied();
        if next == before {
            return unchanged();
        }
        self.items.move_tab_to_root(id, placement, before);
        self.panels_changed()
    }

    /// Shows `id` beside the page, or closes the open panel with `None`. A
    /// panel with no view yet makes one now; the view stays when it closes,
    /// so it comes back as it was left until the lifecycle puts it to sleep.
    fn open_panel(&mut self, profile: ProfileId, id: Option<ItemId>) -> OperationDisposition {
        if id.is_some_and(|id| !self.panel_in_scope(id, profile)) {
            return rejected(OperationReason::InvalidScope);
        }
        let previous = self.panels.open(profile);
        if let Some(previous) = previous {
            self.panel_last.insert(profile, previous);
        }
        if !self.panels.set_open(profile, id) {
            return unchanged();
        }
        let now = std::time::Instant::now();
        let mut effects = Vec::new();
        if let Some(previous) = previous.filter(|previous| Some(*previous) != id) {
            self.panel_timers.closed_at.insert(previous, now);
            // A panel set to unload on close sleeps as soon as it goes.
            if self.panels.settings(previous).unload_on_close {
                self.cancel_discard_probe(previous);
                effects.extend(self.items.hibernate(previous));
            }
        }
        if let Some(id) = id {
            self.panel_timers.closed_at.remove(&id);
            if !self.items.tab(id).is_some_and(TabState::has_view) {
                self.panel_timers.reloaded_at.insert(id, now);
            }
            effects.extend(self.items.ensure_view(id));
        }
        let mut native = self.commit(effects);
        self.schedule_persist();
        self.project_items();
        native.record(self.relayout_with(true));
        self.reschedule_panel_timers();
        mutation_result(native)
    }

    /// Replaces a panel's settings. A change of layout rebuilds a live page
    /// so the site sees the new user agent from its first request.
    fn configure_panel(
        &mut self,
        profile: ProfileId,
        id: ItemId,
        settings: PanelSettings,
    ) -> OperationDisposition {
        if !self.panel_in_scope(id, profile) {
            return rejected(OperationReason::InvalidScope);
        }
        let before = self.panels.settings(id);
        let was_mobile = before.mobile;
        if !self.panels.set_settings(id, settings) {
            return unchanged();
        }
        let after = self.panels.settings(id);
        let mobile = after.mobile;
        if after.selector != before.selector {
            self.engine
                .set_item_style(id, isolation_css(after.selector.as_deref()));
        }
        let mut effects = Vec::new();
        if mobile != was_mobile {
            self.engine.set_mobile_user_agent(id, mobile);
            if self.items.tab(id).is_some_and(TabState::has_view) {
                self.cancel_discard_probe(id);
                effects.extend(self.items.hibernate(id));
                if self.panels.open(profile) == Some(id) {
                    effects.extend(self.items.ensure_view(id));
                }
            }
        }
        let mut native = self.commit(effects);
        self.schedule_persist();
        self.project_items();
        native.record(self.relayout_with(false));
        self.reschedule_panel_timers();
        mutation_result(native)
    }

    /// A link a page's menu asked to open as a panel: a new panel in the
    /// page's own container, shown beside it.
    pub(super) fn open_link_in_panel(&mut self, source: ItemId, url: &str) {
        let Some((profile, space)) = self.windows.focused().map(|win| (win.profile, win.space))
        else {
            return;
        };
        if self.panel_scope() != Some(profile)
            || !(self.item_in_scope(source, profile, space) || self.panel_in_scope(source, profile))
        {
            return;
        }
        let container = self.items.tab(source).and_then(|tab| tab.container());
        let _ = self.add_panel(profile, url, container);
    }

    pub(super) fn panel_wake(&mut self) {
        let now = std::time::Instant::now();
        let shown = self.shown_panel();
        let mut effects = Vec::new();
        let mut reloads = Vec::new();
        for id in self.all_panels() {
            let settings = self.panels.settings(id);
            if !self.items.tab(id).is_some_and(TabState::has_view) {
                continue;
            }
            if let (Some(minutes), Some(closed)) = (
                settings.unload_after_minutes,
                self.panel_timers.closed_at.get(&id).copied(),
            ) {
                let idle = std::time::Duration::from_secs(u64::from(minutes) * 60);
                // A page making sound keeps playing in a closed panel.
                let audible = self.items.tab(id).is_some_and(|tab| tab.audio.shown());
                if shown != Some(id) && !audible && now >= closed + idle {
                    self.panel_timers.closed_at.remove(&id);
                    self.cancel_discard_probe(id);
                    effects.extend(self.items.hibernate(id));
                    continue;
                }
            }
            if let Some(seconds) = settings.reload_every_seconds {
                let every = std::time::Duration::from_secs(u64::from(seconds));
                let last = *self.panel_timers.reloaded_at.entry(id).or_insert(now);
                if now >= last + every {
                    self.panel_timers.reloaded_at.insert(id, now);
                    reloads.push(id);
                }
            }
        }
        let _ = self.commit(effects);
        for id in reloads {
            let _ = self.engine.reload(id);
        }
        self.reschedule_panel_timers();
    }

    /// Every panel of every persistent profile, which keep their timers
    /// while another profile is in front.
    fn all_panels(&self) -> Vec<ItemId> {
        self.profiles
            .iter()
            .filter(|profile| profile.kind != ProfileKind::Incognito)
            .flat_map(|profile| self.profile_panels(profile.id))
            .collect()
    }

    /// Asks to be woken at the earliest panel timer due, or not at all.
    pub(super) fn reschedule_panel_timers(&mut self) {
        let now = std::time::Instant::now();
        let shown = self.shown_panel();
        let mut next: Option<std::time::Instant> = None;
        let mut sooner = |at: std::time::Instant| {
            next = Some(next.map_or(at, |next: std::time::Instant| next.min(at)));
        };
        for id in self.all_panels() {
            if !self.items.tab(id).is_some_and(TabState::has_view) {
                continue;
            }
            let settings = self.panels.settings(id);
            if let (Some(minutes), Some(closed), false) = (
                settings.unload_after_minutes,
                self.panel_timers.closed_at.get(&id),
                shown == Some(id),
            ) {
                sooner(*closed + std::time::Duration::from_secs(u64::from(minutes) * 60));
            }
            if let Some(seconds) = settings.reload_every_seconds {
                let last = *self.panel_timers.reloaded_at.entry(id).or_insert(now);
                sooner(last + std::time::Duration::from_secs(u64::from(seconds)));
            }
        }
        if let Some(queue) = &self.self_queue {
            queue.schedule_panels(next);
        }
    }

    /// Moves a panel to another store by rebuilding its view: a live view is
    /// never retargeted.
    fn set_panel_container(
        &mut self,
        profile: ProfileId,
        id: ItemId,
        container: Option<zephium_core::ids::ContainerId>,
    ) -> OperationDisposition {
        if !self.panel_in_scope(id, profile)
            || container.is_some_and(|container| !self.containers.belongs_to(container, profile))
        {
            return rejected(OperationReason::InvalidScope);
        }
        if self.items.tab(id).and_then(TabState::container) == container {
            return unchanged();
        }
        self.cancel_discard_probe(id);
        let mut effects = self.items.hibernate(id);
        if !self.items.assign_container(id, container) {
            return mutation_result(self.commit(effects));
        }
        if self.panels.open(profile) == Some(id) {
            effects.extend(self.items.ensure_view(id));
        }
        let mut native = self.commit(effects);
        self.schedule_persist();
        self.project_items();
        native.record(self.relayout_with(true));
        mutation_result(native)
    }

    /// The panel shown beside the page in the focused window right now.
    pub(super) fn shown_panel(&self) -> Option<ItemId> {
        let window = self.windows.focused()?;
        if self.active_browser_page().is_some() || self.window_filling_fullscreen().is_some() {
            return None;
        }
        self.panels
            .open(window.profile)
            .filter(|id| self.panel_in_scope(*id, window.profile))
    }

    /// The window's metrics with its panel strip and open panel, which every
    /// layout of the window must use alike.
    pub(super) fn layout_metrics(&self, window: &zephium_core::windows::Window) -> layout::Metrics {
        let mut metrics = window.metrics;
        // A hidden sidebar comes over the page as the chrome itself, and
        // chrome spanning the window for the strip would cover the whole
        // page; the open panel stays docked, and the strip returns with the
        // sidebar.
        let strip = !metrics.sidebar_hidden
            && self.active_browser_page().is_none()
            && self.panel_scope() == Some(window.profile)
            && !self.profile_panels(window.profile).is_empty();
        if strip {
            metrics.panel_strip = PANEL_STRIP_WIDTH;
        }
        if let Some(id) = self.shown_panel() {
            metrics.panel = f64::from(self.panels.width(id));
            metrics.panel_floats = self.panels.floats(id);
        }
        metrics
    }

    pub(super) fn web_panels_view(&self, profile: ProfileId) -> Vec<zephium_ipc::WebPanelView> {
        if self.panel_scope() != Some(profile) {
            return Vec::new();
        }
        let open = self.panels.open(profile);
        self.profile_panels(profile)
            .into_iter()
            .filter_map(|id| {
                let tab = self.items.tab(id)?;
                let settings = self.panels.settings(id);
                Some(zephium_ipc::WebPanelView {
                    tab: self.generic_tab_view(id, tab, Some(profile)),
                    width: self.panels.width(id),
                    open: open == Some(id),
                    settings: zephium_ipc::WebPanelSettingsView {
                        name: settings.name,
                        mobile: settings.mobile,
                        load_on_startup: settings.load_on_startup,
                        unload_on_close: settings.unload_on_close,
                        unload_after_minutes: settings.unload_after_minutes,
                        reload_every_seconds: settings.reload_every_seconds,
                        floating: settings.floating,
                        selector: settings.selector,
                        icon: settings.icon.as_ref().map(super::spaces::icon_view),
                    },
                })
            })
            .collect()
    }

    /// Wakes the panel a profile had open when the session was saved, and
    /// the panels set to load at startup; tells the engine which panels ask
    /// for phone layouts before any of their views is built.
    pub(super) fn restore_open_panel(&mut self) {
        let panels = self.all_panels();
        for id in &panels {
            let settings = self.panels.settings(*id);
            if settings.mobile {
                self.engine.set_mobile_user_agent(*id, true);
            }
            if settings.selector.is_some() {
                self.engine
                    .set_item_style(*id, isolation_css(settings.selector.as_deref()));
            }
        }
        let focused = self.panel_scope();
        let mut effects = Vec::new();
        for id in panels {
            let eager = self.panels.settings(id).load_on_startup
                && focused.is_some_and(|profile| self.panel_in_scope(id, profile));
            if eager || self.shown_panel() == Some(id) {
                effects.extend(self.items.ensure_view(id));
                if self.shown_panel() != Some(id) {
                    self.panel_timers
                        .closed_at
                        .insert(id, std::time::Instant::now());
                }
            }
        }
        let _ = self.commit(effects);
        let _ = self.relayout();
        self.reschedule_panel_timers();
    }

    /// Forgets the panels of a deleted profile; the items go with its spaces.
    pub(super) fn forget_profile_panels(&mut self, profile: ProfileId) {
        for id in self.profile_panels(profile) {
            self.panels.forget(id);
            self.panel_timers.closed_at.remove(&id);
            self.panel_timers.reloaded_at.remove(&id);
        }
        self.panels.forget_profile(profile);
        self.panel_last.remove(&profile);
    }
}
