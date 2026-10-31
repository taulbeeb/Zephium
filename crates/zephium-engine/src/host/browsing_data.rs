//! Clearing and listing the website data of one persistent profile.
//!
//! The call is admitted for the exact profile it names, only while that
//! profile is durable and not being erased, and reaches only that profile's
//! own engine store. A private partition, another profile and Work's own
//! store are refused or simply never addressed.

use std::collections::{HashMap, HashSet};

use zephium_core::browsing_data::{
    EngineDataCall, EngineDataCompletion, EngineDataFailure, EngineDataReply,
};
use zephium_core::ids::ProfileId;
use zephium_core::ports::engine::Partition;

use super::profiles::{bind_profile_persistence_class, ProfilePersistenceClass};
use super::EngineHost;

/// The profile a data call may act on, or why none.
pub(super) fn admitted_profile(
    partition: Partition,
    classes: &mut HashMap<ProfileId, ProfilePersistenceClass>,
    erasing: &HashSet<ProfileId>,
) -> Result<ProfileId, EngineDataFailure> {
    let profile = match partition {
        Partition::Default(profile) | Partition::Persistent(profile) => profile,
        // Container stores exist only where the engine binds them.
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        Partition::Container(profile, _) => profile,
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        Partition::Container(..) => return Err(EngineDataFailure::Unsupported),
        Partition::Ephemeral(_) => return Err(EngineDataFailure::Unsupported),
    };
    if erasing.contains(&profile) {
        return Err(EngineDataFailure::Unavailable);
    }
    // A profile already bound private never gains a durable store here.
    if !bind_profile_persistence_class(classes, partition) {
        return Err(EngineDataFailure::Unsupported);
    }
    Ok(profile)
}

impl EngineHost {
    pub(crate) fn browsing_data_call(
        &mut self,
        partition: Partition,
        call: EngineDataCall,
        done: EngineDataCompletion,
    ) {
        let profile = match admitted_profile(
            partition,
            &mut self.profile_persistence_classes,
            &self.erasure_tombstones,
        ) {
            Ok(profile) => profile,
            Err(failure) => {
                done.finish(EngineDataReply::Failed(failure));
                return;
            }
        };
        #[cfg(target_os = "macos")]
        {
            use crate::platform::imp::browsing_data as native;
            // A container's partition reaches its own store, never the
            // profile's; admission already refused a private one.
            let Some(store) = crate::container_website_data::store_identifier(partition) else {
                done.finish(EngineDataReply::Failed(EngineDataFailure::Unsupported));
                return;
            };
            let _ = profile;
            match call {
                EngineDataCall::Clear { kinds, since } => native::clear(store, kinds, since, done),
                EngineDataCall::Sites => native::sites(store, done),
                EngineDataCall::ForgetSite { site, .. } => native::forget_site(store, site, done),
            }
        }
        #[cfg(target_os = "windows")]
        self.windows_browsing_data(partition, profile, call, done);
    }
}

#[cfg(target_os = "macos")]
impl EngineHost {
    /// Erases the stores of `profile`'s containers outside `keep`. A page
    /// still open in one of them keeps its store, and the answer is `false`.
    pub(crate) fn retain_container_data(
        &mut self,
        profile: ProfileId,
        keep: Vec<zephium_core::ids::ContainerId>,
        done: Box<dyn FnOnce(bool) + Send>,
    ) {
        if self.erasure_tombstones.contains(&profile) {
            done(false);
            return;
        }
        let in_use = self
            .partitions
            .values()
            .chain(self.spare.iter().map(|spare| &spare.partition))
            .any(|partition| match *partition {
                Partition::Container(owner, container) => {
                    owner == profile && !keep.contains(&container)
                }
                _ => false,
            });
        if in_use {
            done(false);
            return;
        }
        crate::platform::imp::retain_container_stores(profile, keep, done);
    }
}

#[cfg(target_os = "windows")]
impl EngineHost {
    /// Clears the data of `profile`'s containers outside `keep`: each is a
    /// WebView2 profile in the profile's folder, found by name. A page still
    /// open in one of them keeps it, and the answer is `false`.
    pub(crate) fn retain_container_data(
        &mut self,
        profile: ProfileId,
        keep: Vec<zephium_core::ids::ContainerId>,
        done: Box<dyn FnOnce(bool) + Send>,
    ) {
        if self.erasure_tombstones.contains(&profile) {
            done(false);
            return;
        }
        let in_use = self.partitions.values().any(|partition| match *partition {
            Partition::Container(owner, container) => {
                owner == profile && !keep.contains(&container)
            }
            _ => false,
        });
        if in_use {
            done(false);
            return;
        }
        let Ok(folder) = crate::erasure::prepare_profile_directory(&self.profiles_root, profile)
        else {
            done(false);
            return;
        };
        let mut stale: Vec<_> = std::fs::read_dir(folder.join("EBWebView"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| {
                crate::container_profiles::container_of(&entry.file_name().to_string_lossy())
            })
            .filter(|container| !keep.contains(container))
            .collect();
        stale.sort();
        stale.dedup();
        self.clear_stale_containers(profile, stale, true, done);
    }

    /// Clears one stale container's data, then the next.
    fn clear_stale_containers(
        &mut self,
        profile: ProfileId,
        mut stale: Vec<zephium_core::ids::ContainerId>,
        cleared: bool,
        done: Box<dyn FnOnce(bool) + Send>,
    ) {
        let Some(container) = stale.pop() else {
            done(cleared);
            return;
        };
        let everything = zephium_core::browsing_data::EngineDataKinds {
            history: true,
            site_data: true,
            cache: true,
        };
        self.windows_browsing_data(
            Partition::Container(profile, container),
            profile,
            EngineDataCall::Clear {
                kinds: everything,
                since: None,
            },
            EngineDataCompletion::new(move |reply| {
                let ok = cleared && matches!(reply, EngineDataReply::Cleared);
                let _ = super::try_with(move |host| {
                    host.clear_stale_containers(profile, stale, ok, done)
                });
            }),
        );
    }

    fn windows_browsing_data(
        &mut self,
        partition: Partition,
        profile: ProfileId,
        call: EngineDataCall,
        done: EngineDataCompletion,
    ) {
        use crate::platform::imp::browsing_data as native;
        let Some((target, temporary)) = self.windows_data_target(partition) else {
            done.finish(EngineDataReply::Failed(EngineDataFailure::Unavailable));
            return;
        };
        // A page opened only to reach the profile goes again once it answers.
        let done = if temporary {
            EngineDataCompletion::new(move |reply| {
                done.finish(reply);
                let _ = super::try_with(move |host| host.close_idle_spare(profile));
            })
        } else {
            done
        };
        match call {
            EngineDataCall::Clear { kinds, since } => native::clear(&target, kinds, since, done),
            EngineDataCall::Sites => native::sites(&target, done),
            EngineDataCall::ForgetSite { site, origin } => {
                native::forget_site(&target, site, origin, done)
            }
        }
    }

    /// A live view of the profile, which is how WebView2 reaches its data. A
    /// profile with no page open gets a hidden one for the call, and the flag
    /// says so.
    fn windows_data_target(
        &mut self,
        partition: Partition,
    ) -> Option<(crate::platform::imp::browsing_data::DataTarget, bool)> {
        use crate::platform::imp::browsing_data::DataTarget;
        let profile = partition.profile();
        // The data is the store's: a container's page reaches only its own
        // container, and a profile's page never a container's.
        let same_store = |owner: &Partition| match (*owner, partition) {
            (Partition::Container(a, x), Partition::Container(b, y)) => a == b && x == y,
            (Partition::Container(..), _) | (_, Partition::Container(..)) => false,
            (owner, _) => owner.profile() == profile,
        };
        let open = self
            .views
            .iter()
            .find(|(id, _)| self.partitions.get(id).is_some_and(same_store))
            .map(|(_, view)| view)
            .or_else(|| {
                self.spare
                    .as_ref()
                    .filter(|spare| same_store(&spare.partition))
                    .map(|spare| &spare.view)
            });
        if let Some(view) = open {
            return DataTarget::of(view).ok().map(|target| (target, false));
        }
        self.applied_content_policy(profile)?;
        // One warm page exists at a time; another profile's is only a
        // latency optimisation.
        self.spare = None;
        let id = std::rc::Rc::new(std::cell::Cell::new(zephium_core::ids::ItemId::generate()));
        let view = self.build_view(
            id.clone(),
            partition,
            "about:blank",
            zephium_core::geometry::Rect::default(),
            super::permits::EventPermit::inactive(),
            super::construction::NativeViewPurpose::WarmSpare,
        )?;
        let target = DataTarget::of(&view).ok();
        self.spare = Some(super::Spare {
            partition,
            view,
            id,
        });
        if target.is_none() {
            self.close_idle_spare(profile);
        }
        target.map(|target| (target, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound(
        profile: ProfileId,
        class: ProfilePersistenceClass,
    ) -> HashMap<ProfileId, ProfilePersistenceClass> {
        HashMap::from([(profile, class)])
    }

    #[test]
    fn a_durable_profile_is_admitted_as_itself_and_no_other() {
        let profile = ProfileId::generate();
        let other = ProfileId::generate();
        let mut classes = bound(other, ProfilePersistenceClass::Durable);
        let erasing = HashSet::new();
        for partition in [Partition::Persistent(profile), Partition::Default(profile)] {
            assert_eq!(
                admitted_profile(partition, &mut classes, &erasing),
                Ok(profile)
            );
        }
        assert_eq!(
            classes.get(&profile),
            Some(&ProfilePersistenceClass::Durable)
        );
        assert_eq!(classes.get(&other), Some(&ProfilePersistenceClass::Durable));
    }

    #[test]
    fn a_private_partition_is_refused_and_binds_nothing() {
        let profile = ProfileId::generate();
        let mut classes = HashMap::new();
        assert_eq!(
            admitted_profile(Partition::Ephemeral(profile), &mut classes, &HashSet::new()),
            Err(EngineDataFailure::Unsupported)
        );
        assert!(classes.is_empty());
    }

    #[test]
    fn a_profile_bound_private_never_gets_a_durable_store() {
        let profile = ProfileId::generate();
        let mut classes = bound(profile, ProfilePersistenceClass::Ephemeral);
        assert_eq!(
            admitted_profile(
                Partition::Persistent(profile),
                &mut classes,
                &HashSet::new()
            ),
            Err(EngineDataFailure::Unsupported)
        );
        assert_eq!(
            classes.get(&profile),
            Some(&ProfilePersistenceClass::Ephemeral)
        );
    }

    #[test]
    fn an_erasing_profile_is_refused() {
        let profile = ProfileId::generate();
        let mut classes = HashMap::new();
        let erasing = HashSet::from([profile]);
        assert_eq!(
            admitted_profile(Partition::Persistent(profile), &mut classes, &erasing),
            Err(EngineDataFailure::Unavailable)
        );
        assert!(classes.is_empty());
    }
}
