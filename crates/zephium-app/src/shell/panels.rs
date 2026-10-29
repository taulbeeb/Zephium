//! Web panels of the focused profile: sites docked beside the page, shared by
//! every space. A panel is a tab item at `Placement::Panel`, so its view,
//! title, icon, audio, container and persistence are a tab's; this module
//! owns only what a panel adds. Every command is checked against the focused
//! persistent profile.

use super::*;
use crate::api::{PanelCommand, PanelNavigation};
use zephium_core::panels::{MAX_PROFILE_PANELS, PANEL_STRIP_WIDTH};

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
                match action {
                    PanelNavigation::Back => self.operation_history(id, false),
                    PanelNavigation::Forward => self.operation_history(id, true),
                    PanelNavigation::Reload => self.operation_reload(id),
                }
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
        self.open_panel(profile, Some(id))
    }

    fn remove_panel(&mut self, profile: ProfileId, id: ItemId) -> OperationDisposition {
        if !self.panel_in_scope(id, profile) {
            return rejected(OperationReason::InvalidScope);
        }
        self.panels.forget(id);
        self.panel_last.retain(|_, last| *last != id);
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
        if let Some(previous) = self.panels.open(profile) {
            self.panel_last.insert(profile, previous);
        }
        if !self.panels.set_open(profile, id) {
            return unchanged();
        }
        let effects = id.map(|id| self.items.ensure_view(id)).unwrap_or_default();
        let mut native = self.commit(effects);
        self.schedule_persist();
        self.project_items();
        native.record(self.relayout_with(true));
        mutation_result(native)
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
        let strip = self.active_browser_page().is_none()
            && self.panel_scope() == Some(window.profile)
            && !self.profile_panels(window.profile).is_empty();
        if strip {
            metrics.panel_strip = PANEL_STRIP_WIDTH;
        }
        if let Some(id) = self.shown_panel() {
            metrics.panel = f64::from(self.panels.width(id));
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
                Some(zephium_ipc::WebPanelView {
                    tab: self.generic_tab_view(id, tab, Some(profile)),
                    width: self.panels.width(id),
                    open: open == Some(id),
                })
            })
            .collect()
    }

    /// Wakes the panel a profile had open when the session was saved.
    pub(super) fn restore_open_panel(&mut self) {
        let Some(id) = self.shown_panel() else {
            return;
        };
        let effects = self.items.ensure_view(id);
        let _ = self.commit(effects);
        let _ = self.relayout();
    }

    /// Forgets the panels of a deleted profile; the items go with its spaces.
    pub(super) fn forget_profile_panels(&mut self, profile: ProfileId) {
        for id in self.profile_panels(profile) {
            self.panels.forget(id);
        }
        self.panels.forget_profile(profile);
        self.panel_last.remove(&profile);
    }
}
