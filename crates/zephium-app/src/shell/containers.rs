//! Containers of the focused profile: separate cookie jars its tabs can be
//! opened in. Every mutation is checked against the focused persistent
//! profile, as spaces are.

use super::*;
use crate::api::ContainerCommand;
use zephium_core::containers::{Container, MAX_PROFILE_CONTAINERS};
use zephium_core::ids::ContainerId;

fn rejected(reason: OperationReason) -> OperationDisposition {
    operation_result(OperationOutcome::Rejected, reason)
}

impl Shell {
    pub(super) fn operation_container(
        &mut self,
        command: ContainerCommand,
    ) -> OperationDisposition {
        let Some(profile) = self.container_scope() else {
            return rejected(OperationReason::InvalidScope);
        };
        match command {
            ContainerCommand::Create {
                name,
                color,
                symbol,
            } => self.create_container(profile, &name, color, symbol),
            ContainerCommand::Update {
                id,
                name,
                color,
                symbol,
            } => {
                if !self.containers.belongs_to(id, profile) {
                    return rejected(OperationReason::InvalidScope);
                }
                if !self.containers.update(id, &name, color, symbol) {
                    return rejected(OperationReason::InvalidInput);
                }
                self.containers_changed()
            }
            ContainerCommand::Delete(id) => self.delete_container(profile, id),
            ContainerCommand::OpenTab(container) => self.open_container_tab(profile, container),
            ContainerCommand::ReopenIn { tab, container } => {
                self.reopen_in_container(profile, tab, container)
            }
        }
    }

    /// The focused profile when it may own containers: persistent, not being
    /// deleted, and on a platform that binds container stores.
    fn container_scope(&self) -> Option<ProfileId> {
        let window = self.windows.focused()?;
        (zephium_engine_supports_containers()
            && self
                .profiles
                .get(window.profile)
                .is_some_and(|profile| profile.kind != ProfileKind::Incognito)
            && !self.profile_deletion_quarantines(window.profile))
        .then_some(window.profile)
    }

    fn containers_changed(&mut self) -> OperationDisposition {
        self.schedule_persist();
        self.project_items();
        operation_result(OperationOutcome::Applied, OperationReason::MutationApplied)
    }

    fn create_container(
        &mut self,
        profile: ProfileId,
        name: &str,
        color: u8,
        symbol: u8,
    ) -> OperationDisposition {
        let count = self.containers.of_profile(profile).count();
        if count >= MAX_PROFILE_CONTAINERS {
            return rejected(OperationReason::ItemLimitReached);
        }
        let name = zephium_core::containers::container_name(name)
            .unwrap_or_else(|| format!("Container {}", count + 1));
        let inserted = self.containers.insert(Container {
            id: ContainerId::generate(),
            profile,
            name,
            color,
            symbol,
        });
        if !inserted {
            return rejected(OperationReason::InvalidInput);
        }
        self.containers_changed()
    }

    /// Closes every tab in the container, forgets it, then asks the engine to
    /// erase its store. A store that cannot be erased yet is swept at the
    /// next launch: the container no longer exists to keep it.
    fn delete_container(&mut self, profile: ProfileId, id: ContainerId) -> OperationDisposition {
        if !self.containers.belongs_to(id, profile) {
            return rejected(OperationReason::InvalidScope);
        }
        for tab in self.items.container_tabs(id) {
            self.close_in_any_space(tab);
        }
        self.recently_closed
            .retain(|entry| entry.container != Some(id));
        self.project_recently_closed_if_changed();
        self.containers.remove(id);
        self.sweep_container_stores(profile);
        self.containers_changed()
    }

    /// A new tab in the focused window whose pages use `container`'s store,
    /// or the profile's own with `None`.
    fn open_container_tab(
        &mut self,
        profile: ProfileId,
        container: Option<ContainerId>,
    ) -> OperationDisposition {
        if container.is_some_and(|id| !self.containers.belongs_to(id, profile)) {
            return rejected(OperationReason::InvalidScope);
        }
        let Some((id, effects)) = self.open_tab_with_id() else {
            return rejected(OperationReason::ItemLimitReached);
        };
        if !self.items.assign_container(id, container) {
            let native = self.commit(effects);
            self.close_in_any_space(id);
            let _ = native;
            return rejected(OperationReason::InvalidInput);
        }
        let native = self.commit(effects);
        self.project_tab(id);
        mutation_result(native)
    }

    /// Opens a tab's page again in another container, right after it. The
    /// original stays: a live page is never moved between stores.
    fn reopen_in_container(
        &mut self,
        profile: ProfileId,
        tab: ItemId,
        container: Option<ContainerId>,
    ) -> OperationDisposition {
        let Some(space) = self.windows.focused().map(|window| window.space) else {
            return rejected(OperationReason::NoFocusedWindow);
        };
        if !self.item_in_scope(tab, profile, space)
            || container.is_some_and(|id| !self.containers.belongs_to(id, profile))
        {
            return rejected(OperationReason::InvalidScope);
        }
        let Some(url) = self
            .items
            .tab(tab)
            .filter(|state| state.content == zephium_core::item::TabContent::Web)
            .and_then(|state| state.url.as_ref())
            .map(ToString::to_string)
        else {
            return operation_result(OperationOutcome::NoOp, OperationReason::InvalidInput);
        };
        if self.items.tab(tab).and_then(|state| state.container) == container {
            return operation_result(OperationOutcome::NoOp, OperationReason::StateUnchanged);
        }
        let Some((copy, mut effects)) = self.open_tab_with_id() else {
            return rejected(OperationReason::ItemLimitReached);
        };
        let today = Placement::Space {
            space,
            section: SpaceSection::Today,
        };
        let roots = self.items.roots(today);
        if let Some(at) = roots.iter().position(|root| *root == tab) {
            let before = roots[at + 1..].iter().copied().find(|root| *root != copy);
            self.items.move_tab_to_root(copy, today, before);
        }
        if !self.items.assign_container(copy, container) {
            let native = self.commit(effects);
            self.close_in_any_space(copy);
            let _ = native;
            return rejected(OperationReason::InvalidInput);
        }
        effects.extend(self.items.navigate(copy, &url));
        mutation_result(self.commit(effects))
    }

    /// Asks the engine to erase every container store of `profile` that no
    /// container of it names any more.
    pub(super) fn sweep_container_stores(&self, profile: ProfileId) {
        let keep: Vec<ContainerId> = self.containers.of_profile(profile).map(|c| c.id).collect();
        self.engine.retain_container_data(
            profile,
            keep,
            Box::new(|swept| {
                if !swept {
                    crate::diagnostic!(
                        "containers: a deleted container's store is still in use; it is swept at the next launch"
                    );
                }
            }),
        );
    }

    /// Erases the stores of containers deleted in an earlier run whose
    /// erasure did not finish.
    pub(super) fn sweep_all_container_stores(&self) {
        if !zephium_engine_supports_containers() {
            return;
        }
        let profiles: Vec<ProfileId> = self
            .profiles
            .iter()
            .filter(|profile| profile.kind != ProfileKind::Incognito)
            .map(|profile| profile.id)
            .collect();
        for profile in profiles {
            self.sweep_container_stores(profile);
        }
    }

    pub(super) fn containers_view(&self, profile: ProfileId) -> Vec<zephium_ipc::ContainerView> {
        self.containers
            .of_profile(profile)
            .map(|container| zephium_ipc::ContainerView {
                id: container.id.to_string(),
                name: container.name.clone(),
                color: container.color,
                symbol: container.symbol,
                tabs: self.items.container_tabs(container.id).len() as u32,
            })
            .collect()
    }
}

/// Whether container tabs can be opened here at all.
fn zephium_engine_supports_containers() -> bool {
    cfg!(target_os = "macos")
}
