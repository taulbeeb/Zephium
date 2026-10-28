//! Containers from privileged chrome, the File menu and the tab menu.
use std::sync::Mutex;

use tauri::menu::{IsMenuItem, MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::menu_text;
use zephium_app::{Command, ContainerCommand, Handle};
use zephium_core::ids::{ContainerId, ItemId, SpaceId};
use zephium_ipc::{ItemsState, OperationAdmission};

/// "Open in Container ▸" on the tab menu: `tabmenu.container=<id>` or
/// `tabmenu.container=none` for the profile's own store.
pub(crate) const TAB_MENU_REOPEN: &str = "container=";
/// "New Container Tab ▸" in the File menu.
const NEW_TAB: &str = "container.newTab=";
const NONE: &str = "none";

#[derive(Clone, Default, PartialEq)]
struct Shown {
    /// The focused profile's containers, in order, with their names.
    containers: Vec<(String, String)>,
    private: bool,
}

static SHOWN: Mutex<Option<Shown>> = Mutex::new(None);

fn shown() -> Shown {
    SHOWN
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default()
}

/// Follows each sidebar projection so the menus list the focused profile's
/// containers, rebuilding the menu bar when they change.
pub(crate) fn follow(app: &AppHandle, state: &ItemsState) {
    let next = Shown {
        containers: state
            .containers
            .iter()
            .map(|container| (container.id.clone(), container.name.clone()))
            .collect(),
        private: state
            .profile
            .as_ref()
            .is_some_and(|profile| profile.kind == zephium_ipc::ProfileKindView::Incognito),
    };
    let changed = {
        let mut shown = SHOWN.lock().unwrap_or_else(|e| e.into_inner());
        let changed = shown.as_ref() != Some(&next);
        *shown = Some(next);
        changed
    };
    #[cfg(target_os = "macos")]
    if changed {
        let handle = app.clone();
        let queued = app.run_on_main_thread(move || {
            let Some(keymap) = handle.try_state::<super::keymap::Keymap>() else {
                return;
            };
            if super::install_menu_bar(&handle, &keymap.overrides(), keymap.work_pane_shown())
                .is_err()
            {
                super::write_diagnostic(format_args!("menu: containers were not applied"));
            }
        });
        if queued.is_err() {
            super::write_diagnostic(format_args!("menu: containers were not applied"));
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, changed);
}

fn container_of(value: &str) -> Option<ContainerId> {
    ContainerId::parse(value).filter(|id| id.to_string() == value)
}

/// `None` names the profile's own store; anything unparsable is refused.
fn target_of(value: Option<&str>) -> Result<Option<ContainerId>, ()> {
    match value {
        None => Ok(None),
        Some(value) => container_of(value).map(Some).ok_or(()),
    }
}

fn admitted(caller: &WebviewWindow, command: &str) -> bool {
    super::authorize(caller, super::CallerPolicy::Main, command)
        && !super::shutdown_started(caller.app_handle())
}

fn dispatch(app: &AppHandle, shell: &Handle, command: ContainerCommand) -> OperationAdmission {
    super::dispatch_operation(app, shell, Command::Container(Box::new(command)))
}

#[tauri::command]
#[specta::specta]
pub(crate) fn containers_create(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    name: String,
    color: u8,
    symbol: u8,
) -> OperationAdmission {
    if !admitted(&caller, "containers_create")
        || !super::bounded(&name, super::MAX_PROFILE_NAME_BYTES)
    {
        return super::rejected_operation();
    }
    dispatch(
        caller.app_handle(),
        &shell,
        ContainerCommand::Create {
            name,
            color,
            symbol,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub(crate) fn containers_update(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    container: String,
    name: String,
    color: u8,
    symbol: u8,
) -> OperationAdmission {
    let Some(id) = container_of(&container) else {
        return super::rejected_operation();
    };
    if !admitted(&caller, "containers_update")
        || !super::bounded(&name, super::MAX_PROFILE_NAME_BYTES)
    {
        return super::rejected_operation();
    }
    dispatch(
        caller.app_handle(),
        &shell,
        ContainerCommand::Update {
            id,
            name,
            color,
            symbol,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub(crate) fn containers_delete(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    container: String,
) -> OperationAdmission {
    match container_of(&container).filter(|_| admitted(&caller, "containers_delete")) {
        Some(id) => dispatch(caller.app_handle(), &shell, ContainerCommand::Delete(id)),
        None => super::rejected_operation(),
    }
}

/// A new tab in the container, or in the profile's own store without one.
#[tauri::command]
#[specta::specta]
pub(crate) fn containers_open_tab(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    container: Option<String>,
) -> OperationAdmission {
    match target_of(container.as_deref()) {
        Ok(target) if admitted(&caller, "containers_open_tab") => dispatch(
            caller.app_handle(),
            &shell,
            ContainerCommand::OpenTab(target),
        ),
        _ => super::rejected_operation(),
    }
}

/// The tab's page again in a new tab in another container.
#[tauri::command]
#[specta::specta]
pub(crate) fn containers_reopen_tab(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    tab: String,
    container: Option<String>,
) -> OperationAdmission {
    let tab = ItemId::parse(&tab).filter(|_| tab.len() <= super::MAX_ITEM_ID_BYTES);
    match (tab, target_of(container.as_deref())) {
        (Some(tab), Ok(container)) if admitted(&caller, "containers_reopen_tab") => dispatch(
            caller.app_handle(),
            &shell,
            ContainerCommand::ReopenIn { tab, container },
        ),
        _ => super::rejected_operation(),
    }
}

/// The container a space's new tabs open in, or none.
#[tauri::command]
#[specta::specta]
pub(crate) fn containers_set_space_default(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    space: String,
    container: Option<String>,
) -> OperationAdmission {
    let space = SpaceId::parse(&space).filter(|id| id.to_string() == space);
    match (space, target_of(container.as_deref())) {
        (Some(space), Ok(container)) if admitted(&caller, "containers_set_space_default") => {
            dispatch(
                caller.app_handle(),
                &shell,
                ContainerCommand::SetSpaceDefault { space, container },
            )
        }
        _ => super::rejected_operation(),
    }
}

/// Menu ids this module owns; anything else is not a container action.
pub(crate) fn execute(app: &AppHandle, id: &str) -> Option<OperationAdmission> {
    let target = id.strip_prefix(NEW_TAB)?;
    let (Some(shell), Ok(target)) = (
        app.try_state::<Handle>(),
        target_of((target != NONE).then_some(target)),
    ) else {
        return Some(super::rejected_operation());
    };
    Some(dispatch(app, &shell, ContainerCommand::OpenTab(target)))
}

/// A tab menu action naming the container to open its page in again.
pub(crate) fn reopen_tab(app: &AppHandle, tab: ItemId, target: &str) -> OperationAdmission {
    match (
        app.try_state::<Handle>(),
        target_of((target != NONE).then_some(target)),
    ) {
        (Some(shell), Ok(container)) => {
            dispatch(app, &shell, ContainerCommand::ReopenIn { tab, container })
        }
        _ => super::rejected_operation(),
    }
}

type Items = Vec<Box<dyn IsMenuItem<tauri::Wry>>>;

/// "No Container", a separator, then each container, under `prefix`.
fn target_items(handle: &AppHandle, prefix: &str, shown: &Shown) -> tauri::Result<Items> {
    let mut items: Items = vec![Box::new(
        MenuItemBuilder::with_id(format!("{prefix}{NONE}"), menu_text::label("No Container"))
            .build(handle)?,
    )];
    if !shown.containers.is_empty() {
        items.push(Box::new(PredefinedMenuItem::separator(handle)?));
    }
    for (id, name) in &shown.containers {
        items.push(Box::new(
            MenuItemBuilder::with_id(format!("{prefix}{id}"), super::extension_menu_label(name))
                .build(handle)?,
        ));
    }
    Ok(items)
}

fn refs(items: &Items) -> Vec<&dyn IsMenuItem<tauri::Wry>> {
    items.iter().map(|item| item.as_ref()).collect()
}

/// "New Container Tab ▸" for the File menu, or nothing while the platform
/// cannot bind container stores, the window is private or none exist.
pub(crate) fn file_menu_section(handle: &AppHandle) -> tauri::Result<Option<Submenu<tauri::Wry>>> {
    let shown = shown();
    if !zephium_engine::container_storage_supported()
        || shown.private
        || shown.containers.is_empty()
    {
        return Ok(None);
    }
    let items = target_items(handle, NEW_TAB, &shown)?;
    SubmenuBuilder::new(handle, menu_text::label("New Container Tab"))
        .items(&refs(&items))
        .build()
        .map(Some)
}

/// "Open in Container ▸" for the tab menu, or nothing where containers are
/// unavailable.
pub(crate) fn tab_menu_section(
    handle: &AppHandle,
    prefix: &str,
    page: bool,
) -> tauri::Result<Option<Submenu<tauri::Wry>>> {
    let shown = shown();
    if !zephium_engine::container_storage_supported()
        || shown.private
        || shown.containers.is_empty()
    {
        return Ok(None);
    }
    let items = target_items(handle, prefix, &shown)?;
    SubmenuBuilder::new(handle, menu_text::label("Open in Container"))
        .items(&refs(&items))
        .enabled(page)
        .build()
        .map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_are_exact_container_ids_or_none() {
        let id = ContainerId::generate();
        assert_eq!(target_of(None), Ok(None));
        assert_eq!(target_of(Some(&id.to_string())), Ok(Some(id)));
        assert_eq!(target_of(Some("not-an-id")), Err(()));
        assert_eq!(target_of(Some(&id.to_string().to_lowercase())), Err(()));
    }
}
