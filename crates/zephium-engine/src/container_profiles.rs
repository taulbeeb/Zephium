//! A container's own WebView2 profile within its Zephium profile's user data
//! folder: same browser process, separate cookies and storage. The name
//! carries the container's identity, so a sweep can tell which container
//! each profile on disk belongs to.

use zephium_core::ids::ContainerId;

const PREFIX: &str = "container-";

/// The WebView2 profile name of `container`.
pub(crate) fn profile_name(container: ContainerId) -> String {
    format!("{PREFIX}{container}")
}

/// The container a WebView2 profile name belongs to, read back exactly.
pub(crate) fn container_of(name: &str) -> Option<ContainerId> {
    let id = name.strip_prefix(PREFIX)?;
    ContainerId::parse(id).filter(|container| container.to_string() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_and_nothing_else_reads_as_a_container() {
        let container = ContainerId::generate();
        let name = profile_name(container);
        assert!(name.len() <= 64 && name.is_ascii());
        assert!(name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'));
        assert_eq!(container_of(&name), Some(container));
        for other in [
            "Default",
            "work-0123",
            "agent-x",
            "container-",
            "container-nope",
        ] {
            assert_eq!(container_of(other), None, "{other}");
        }
        assert_eq!(container_of(&name.to_lowercase()), None);
    }
}
