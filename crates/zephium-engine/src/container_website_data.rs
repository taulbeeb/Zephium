//! A container's own website data: one persistent store per container,
//! apart from its profile's store, Work's and every other container's.
//!
//! The identifier is derived, never stored. Its first half depends only on
//! the profile, so erasing a profile finds every container store it owns by
//! enumeration, including one whose container record was already lost; the
//! second half names the container.
use sha2::{Digest, Sha256};
use zephium_core::ids::{ContainerId, ProfileId};
use zephium_core::ports::engine::Partition;

const PREFIX_DOMAIN: &[u8] = b"zephium.container-website-data.profile.v1\0";
const CONTAINER_DOMAIN: &[u8] = b"zephium.container-website-data.container.v1\0";

/// The half every container store of `profile` shares, version bits set.
fn prefix(profile: ProfileId) -> [u8; 8] {
    let digest = Sha256::new()
        .chain_update(PREFIX_DOMAIN)
        .chain_update(profile.bytes())
        .finalize();
    let mut half = [0_u8; 8];
    half.copy_from_slice(&digest[..8]);
    // WebKit takes the identifier as an RFC 9562 UUID; version 8 is custom.
    half[6] = (half[6] & 0x0f) | 0x80;
    half
}

/// Stable native identifier of one container's store; never a profile's own
/// or Work's.
pub(crate) fn identifier(profile: ProfileId, container: ContainerId) -> [u8; 16] {
    let digest = Sha256::new()
        .chain_update(CONTAINER_DOMAIN)
        .chain_update(profile.bytes())
        .chain_update(container.bytes())
        .finalize();
    let mut id = [0_u8; 16];
    id[..8].copy_from_slice(&prefix(profile));
    id[8..].copy_from_slice(&digest[..8]);
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}

/// Whether `identifier` names a container store of `profile`.
pub(crate) fn belongs_to(identifier: &[u8; 16], profile: ProfileId) -> bool {
    identifier[..8] == prefix(profile) && identifier[8] >> 6 == 2
}

/// The durable store a partition's pages use, or none for a private one.
pub(crate) fn store_identifier(partition: Partition) -> Option<[u8; 16]> {
    match partition {
        Partition::Default(profile) | Partition::Persistent(profile) => Some(profile.bytes()),
        Partition::Container(profile, container) => Some(identifier(profile, container)),
        Partition::Ephemeral(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_stable_distinct_and_owned_by_their_profile() {
        let (first, second) = (ProfileId::generate(), ProfileId::generate());
        let (a, b) = (ContainerId::generate(), ContainerId::generate());
        let id = identifier(first, a);
        assert_eq!(id, identifier(first, a));
        assert_ne!(id, identifier(first, b));
        assert_ne!(id, identifier(second, a));
        assert_ne!(id, first.bytes());
        assert_ne!(id, crate::work_website_data::identifier(first));
        assert_eq!(id[6] >> 4, 8);
        assert_eq!(id[8] >> 6, 2);
        assert!(belongs_to(&id, first));
        assert!(belongs_to(&identifier(first, b), first));
        assert!(!belongs_to(&id, second));
        assert!(!belongs_to(&first.bytes(), first));
        assert!(!belongs_to(
            &crate::work_website_data::identifier(first),
            first
        ));
    }

    #[test]
    fn each_partition_names_its_own_store() {
        let profile = ProfileId::generate();
        let container = ContainerId::generate();
        assert_eq!(
            store_identifier(Partition::Persistent(profile)),
            Some(profile.bytes())
        );
        assert_eq!(
            store_identifier(Partition::Container(profile, container)),
            Some(identifier(profile, container))
        );
        assert_eq!(store_identifier(Partition::Ephemeral(profile)), None);
        // A profile's own store is never Work's or another profile's.
        let other = ProfileId::generate();
        let own = store_identifier(Partition::Default(profile)).unwrap();
        assert_ne!(own, crate::work_website_data::identifier(profile));
        assert_ne!(own, crate::work_website_data::identifier(other));
        assert_ne!(Some(own), store_identifier(Partition::Default(other)));
    }
}
