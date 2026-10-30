//! Web panels from privileged chrome and the tab menu.
use tauri::{AppHandle, Manager, State, WebviewWindow};

use zephium_app::{Command, Handle, PanelCommand, PanelNavigation};
use zephium_core::ids::{ContainerId, ItemId};
use zephium_ipc::OperationAdmission;

/// "Add Page to Panels" on the tab menu.
pub(crate) const TAB_MENU_ADD: &str = "panel.add";

#[derive(Clone, Copy, Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PanelNavigationV1 {
    Back,
    Forward,
    Reload,
}

fn panel_of(value: &str) -> Option<ItemId> {
    ItemId::parse(value)
        .filter(|id| value.len() <= super::MAX_ITEM_ID_BYTES && id.to_string() == value)
}

/// `None` names the profile's own store; anything unparsable is refused.
fn container_of(value: Option<&str>) -> Result<Option<ContainerId>, ()> {
    match value {
        None => Ok(None),
        Some(value) => ContainerId::parse(value)
            .filter(|id| id.to_string() == value)
            .map(Some)
            .ok_or(()),
    }
}

fn admitted(caller: &WebviewWindow, command: &str) -> bool {
    super::authorize(caller, super::CallerPolicy::Main, command)
        && !super::shutdown_started(caller.app_handle())
}

fn dispatch(app: &AppHandle, shell: &Handle, command: PanelCommand) -> OperationAdmission {
    super::dispatch_operation(app, shell, Command::Panel(Box::new(command)))
}

/// A new panel for an address, opened beside the page.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_add(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    url: String,
    container: Option<String>,
) -> OperationAdmission {
    match container_of(container.as_deref()) {
        Ok(container)
            if admitted(&caller, "panels_add")
                && super::bounded(&url, super::MAX_NAVIGATION_INPUT_BYTES) =>
        {
            dispatch(
                caller.app_handle(),
                &shell,
                PanelCommand::Add { url, container },
            )
        }
        _ => super::rejected_operation(),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn panels_remove(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
) -> OperationAdmission {
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_remove")) {
        Some(id) => dispatch(caller.app_handle(), &shell, PanelCommand::Remove(id)),
        None => super::rejected_operation(),
    }
}

/// Before another panel, or last with none.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_move(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    before: Option<String>,
) -> OperationAdmission {
    let before = match before.as_deref().map(panel_of) {
        None => None,
        Some(Some(id)) => Some(id),
        Some(None) => return super::rejected_operation(),
    };
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_move")) {
        Some(id) => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::Move { id, before },
        ),
        None => super::rejected_operation(),
    }
}

/// Shows a panel beside the page, or closes the open one with none.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_open(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: Option<String>,
) -> OperationAdmission {
    let target = match panel.as_deref().map(panel_of) {
        None => None,
        Some(Some(id)) => Some(id),
        Some(None) => return super::rejected_operation(),
    };
    if !admitted(&caller, "panels_open") {
        return super::rejected_operation();
    }
    dispatch(caller.app_handle(), &shell, PanelCommand::Open(target))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn panels_resize(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    width: f64,
) -> OperationAdmission {
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_resize") && width.is_finite()) {
        Some(id) => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::Resize { id, width },
        ),
        None => super::rejected_operation(),
    }
}

/// Moves the panel to another container's store, rebuilding its page.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_set_container(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    container: Option<String>,
) -> OperationAdmission {
    match (panel_of(&panel), container_of(container.as_deref())) {
        (Some(id), Ok(container)) if admitted(&caller, "panels_set_container") => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::SetContainer { id, container },
        ),
        _ => super::rejected_operation(),
    }
}

/// Goes to an address in the panel.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_load(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    input: String,
) -> OperationAdmission {
    match panel_of(&panel).filter(|_| {
        admitted(&caller, "panels_load")
            && super::bounded(&input, super::MAX_NAVIGATION_INPUT_BYTES)
    }) {
        Some(id) => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::Load { id, input },
        ),
        None => super::rejected_operation(),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn panels_navigate(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    action: PanelNavigationV1,
) -> OperationAdmission {
    let action = match action {
        PanelNavigationV1::Back => PanelNavigation::Back,
        PanelNavigationV1::Forward => PanelNavigation::Forward,
        PanelNavigationV1::Reload => PanelNavigation::Reload,
    };
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_navigate")) {
        Some(id) => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::Navigate { id, action },
        ),
        None => super::rejected_operation(),
    }
}

/// The tab menu's "Add Page to Panels": the tab stays, a panel joins it.
pub(crate) fn add_tab(app: &AppHandle, tab: ItemId) -> OperationAdmission {
    match app.try_state::<Handle>() {
        Some(shell) => dispatch(app, &shell, PanelCommand::AddTab(tab)),
        None => super::rejected_operation(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_must_be_exact() {
        let id = ItemId::generate();
        assert_eq!(panel_of(&id.to_string()), Some(id));
        assert_eq!(panel_of(&id.to_string().to_lowercase()), None);
        assert_eq!(panel_of("nope"), None);
        let container = ContainerId::generate();
        assert_eq!(container_of(None), Ok(None));
        assert_eq!(
            container_of(Some(&container.to_string())),
            Ok(Some(container))
        );
        assert_eq!(container_of(Some("nope")), Err(()));
    }
}
