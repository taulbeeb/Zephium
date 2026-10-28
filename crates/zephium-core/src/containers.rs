//! Containers are separate cookie jars inside one persistent profile. A tab
//! opened in a container keeps its sign-ins and site storage apart from the
//! profile's own tabs and from every other container, while history,
//! bookmarks, extensions and spaces stay the profile's.

use std::collections::HashMap;

use crate::ids::{ContainerId, ProfileId};

/// A profile keeps at most this many containers. Each is one native store.
pub const MAX_PROFILE_CONTAINERS: usize = 16;
/// Every container across all profiles; bounds restore and the native stores
/// the engine may be asked to bind.
pub const MAX_CONTAINERS: usize = 64;
/// Colours and symbols are indices into the frame's curated sets, so a
/// stored container never names a colour or glyph the interface cannot draw.
pub const CONTAINER_COLORS: u8 = 8;
pub const CONTAINER_SYMBOLS: u8 = 12;
pub const MAX_CONTAINER_NAME_CHARS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub id: ContainerId,
    pub profile: ProfileId,
    pub name: String,
    pub color: u8,
    pub symbol: u8,
}

impl Container {
    pub fn look_is_valid(color: u8, symbol: u8) -> bool {
        color < CONTAINER_COLORS && symbol < CONTAINER_SYMBOLS
    }
}

/// A container name as it is kept: bounded, stripped of controls, never empty.
pub fn container_name(name: &str) -> Option<String> {
    let name: String = crate::session::bounded_name(name.trim(), "")
        .chars()
        .take(MAX_CONTAINER_NAME_CHARS)
        .collect();
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// The containers of every persistent profile, in creation order per profile.
#[derive(Clone, Debug, Default)]
pub struct Containers {
    order: Vec<ContainerId>,
    map: HashMap<ContainerId, Container>,
}

impl Containers {
    /// Admits one container. The caller decides whether its profile may own
    /// containers; this refuses a duplicate id, an invalid look or name, and
    /// anything over the per-profile and total bounds.
    pub fn insert(&mut self, container: Container) -> bool {
        if self.map.len() >= MAX_CONTAINERS
            || self.map.contains_key(&container.id)
            || self.of_profile(container.profile).count() >= MAX_PROFILE_CONTAINERS
            || !Container::look_is_valid(container.color, container.symbol)
        {
            return false;
        }
        let Some(name) = container_name(&container.name) else {
            return false;
        };
        self.order.push(container.id);
        self.map
            .insert(container.id, Container { name, ..container });
        true
    }

    pub fn get(&self, id: ContainerId) -> Option<&Container> {
        self.map.get(&id)
    }

    /// Whether `id` names a container of exactly `profile`.
    pub fn belongs_to(&self, id: ContainerId, profile: ProfileId) -> bool {
        self.map.get(&id).is_some_and(|c| c.profile == profile)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Container> {
        self.order.iter().filter_map(|id| self.map.get(id))
    }

    pub fn of_profile(&self, profile: ProfileId) -> impl Iterator<Item = &Container> {
        self.iter().filter(move |c| c.profile == profile)
    }

    /// Renames and restyles in one step. A name with nothing left, or a look
    /// outside the curated sets, is refused and changes nothing.
    pub fn update(&mut self, id: ContainerId, name: &str, color: u8, symbol: u8) -> bool {
        let Some(name) = container_name(name) else {
            return false;
        };
        if !Container::look_is_valid(color, symbol) {
            return false;
        }
        match self.map.get_mut(&id) {
            Some(container) => {
                container.name = name;
                container.color = color;
                container.symbol = symbol;
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, id: ContainerId) -> Option<Container> {
        let container = self.map.remove(&id)?;
        self.order.retain(|candidate| *candidate != id);
        Some(container)
    }

    /// Drops every container of a removed profile and returns them, so the
    /// caller can account for their native stores.
    pub fn remove_profile(&mut self, profile: ProfileId) -> Vec<Container> {
        let ids: Vec<_> = self.of_profile(profile).map(|c| c.id).collect();
        ids.into_iter().filter_map(|id| self.remove(id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn container(id: u128, profile: u128) -> Container {
        Container {
            id: ContainerId::from(id),
            profile: ProfileId::from(profile),
            name: format!("Container {id}"),
            color: 0,
            symbol: 0,
        }
    }

    #[test]
    fn names_are_bounded_and_never_empty() {
        assert_eq!(container_name("  Work \n").as_deref(), Some("Work"));
        assert_eq!(container_name(" \u{200f} "), None);
        assert_eq!(
            container_name(&"a".repeat(500)).map(|n| n.chars().count()),
            Some(MAX_CONTAINER_NAME_CHARS)
        );
    }

    #[test]
    fn insertion_is_bounded_per_profile_and_validates_the_look() {
        let mut containers = Containers::default();
        for id in 0..MAX_PROFILE_CONTAINERS as u128 {
            assert!(containers.insert(container(id + 1, 1)));
        }
        assert!(!containers.insert(container(100, 1)));
        assert!(containers.insert(container(100, 2)));
        assert!(!containers.insert(container(100, 2)), "duplicate id");
        assert!(!containers.insert(Container {
            color: CONTAINER_COLORS,
            ..container(101, 2)
        }));
        assert!(!containers.insert(Container {
            name: " \n".into(),
            ..container(102, 2)
        }));
    }

    #[test]
    fn membership_is_exact_to_the_profile() {
        let mut containers = Containers::default();
        assert!(containers.insert(container(1, 1)));
        assert!(containers.belongs_to(ContainerId::from(1), ProfileId::from(1)));
        assert!(!containers.belongs_to(ContainerId::from(1), ProfileId::from(2)));
        assert!(!containers.belongs_to(ContainerId::from(2), ProfileId::from(1)));
    }

    #[test]
    fn update_refuses_an_empty_name_or_invalid_look() {
        let mut containers = Containers::default();
        assert!(containers.insert(container(1, 1)));
        let id = ContainerId::from(1);
        assert!(containers.update(id, " Banking ", 3, 4));
        let kept = containers.get(id).unwrap().clone();
        assert_eq!(
            (kept.name.as_str(), kept.color, kept.symbol),
            ("Banking", 3, 4)
        );
        assert!(!containers.update(id, "", 1, 1));
        assert!(!containers.update(id, "Shopping", 1, CONTAINER_SYMBOLS));
        assert_eq!(containers.get(id), Some(&kept));
        assert!(!containers.update(ContainerId::from(9), "Missing", 0, 0));
    }

    #[test]
    fn removing_a_profile_removes_only_its_containers_in_order() {
        let mut containers = Containers::default();
        for (id, profile) in [(1, 1), (2, 2), (3, 1)] {
            assert!(containers.insert(container(id, profile)));
        }
        let removed: Vec<_> = containers
            .remove_profile(ProfileId::from(1))
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(removed, vec![ContainerId::from(1), ContainerId::from(3)]);
        assert_eq!(
            containers.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![ContainerId::from(2)]
        );
    }
}
