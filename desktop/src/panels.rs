//! Web panels from privileged chrome and the tab menu.
use tauri::{AppHandle, Manager, State, WebviewWindow};

use zephium_app::{Command, Handle, PanelCommand, PanelNavigation, PanelZoom};
use zephium_core::ids::{ContainerId, ItemId};
use zephium_ipc::OperationAdmission;

/// "Add Page to Panels" on the tab menu.
pub(crate) const TAB_MENU_ADD: &str = "panel.add";
/// "Move Tab to Panels" on the tab menu.
pub(crate) const TAB_MENU_MOVE: &str = "panel.move";

#[derive(Clone, Copy, Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PanelZoomV1 {
    In,
    Out,
    Reset,
}

/// What the person chose for one panel, as chrome sends it. The shell
/// sanitizes the name and timers.
#[derive(Clone, Debug, serde::Deserialize, specta::Type)]
pub(crate) struct PanelSettingsInputV1 {
    name: Option<String>,
    mobile: bool,
    load_on_startup: bool,
    unload_on_close: bool,
    unload_after_minutes: Option<u32>,
    reload_every_seconds: Option<u32>,
    floating: bool,
    selector: Option<String>,
    icon: Option<zephium_ipc::SpaceIconView>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PanelNavigationV1 {
    Back,
    Forward,
    Reload,
    Home,
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
        PanelNavigationV1::Home => PanelNavigation::Home,
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

/// Replaces a panel's name, lifecycle timers and layout choice.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_configure(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    settings: PanelSettingsInputV1,
) -> OperationAdmission {
    let name_ok = settings
        .name
        .as_deref()
        .is_none_or(|name| super::bounded(name, super::MAX_PROFILE_NAME_BYTES))
        && settings.selector.as_deref().is_none_or(|selector| {
            super::bounded(selector, zephium_core::panels::MAX_PANEL_SELECTOR_BYTES)
        });
    let icon = settings.icon.map(|icon| match icon {
        zephium_ipc::SpaceIconView::Symbol { index } => {
            zephium_core::spaces::SpaceIcon::Symbol(index)
        }
        zephium_ipc::SpaceIconView::Emoji { text } => zephium_core::spaces::SpaceIcon::Emoji(text),
    });
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_configure") && name_ok) {
        Some(id) => dispatch(
            caller.app_handle(),
            &shell,
            PanelCommand::Configure {
                id,
                settings: zephium_core::panels::PanelSettings {
                    name: settings.name,
                    mobile: settings.mobile,
                    load_on_startup: settings.load_on_startup,
                    unload_on_close: settings.unload_on_close,
                    unload_after_minutes: settings.unload_after_minutes,
                    reload_every_seconds: settings.reload_every_seconds,
                    floating: settings.floating,
                    selector: settings.selector,
                    icon,
                    // The shell keeps Home; chrome never sets it.
                    home: None,
                },
            },
        ),
        None => super::rejected_operation(),
    }
}

#[tauri::command]
#[specta::specta]
pub(crate) fn panels_zoom(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    panel: String,
    step: PanelZoomV1,
) -> OperationAdmission {
    let step = match step {
        PanelZoomV1::In => PanelZoom::In,
        PanelZoomV1::Out => PanelZoom::Out,
        PanelZoomV1::Reset => PanelZoom::Reset,
    };
    match panel_of(&panel).filter(|_| admitted(&caller, "panels_zoom")) {
        Some(id) => dispatch(caller.app_handle(), &shell, PanelCommand::Zoom { id, step }),
        None => super::rejected_operation(),
    }
}

/// Brings a strip waiting at the window's edge out, or puts it back.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_peek_strip(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    peek: bool,
) -> OperationAdmission {
    if !admitted(&caller, "panels_peek_strip") {
        return super::rejected_operation();
    }
    dispatch(caller.app_handle(), &shell, PanelCommand::PeekStrip(peek))
}

/// The panel strip began or stopped waiting at the window's edge; the edge
/// is watched natively while it waits, since the page takes the pointer.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_watch_edge(caller: WebviewWindow, waiting: bool) {
    if admitted(&caller, "panels_watch_edge") {
        super::platform::imp::panel_edge(&caller, waiting);
    }
}

/// The panel sheet opened or closed; pages step aside while it is open,
/// since the interface draws it beneath them.
#[tauri::command]
#[specta::specta]
pub(crate) fn panels_sheet(
    caller: WebviewWindow,
    shell: State<'_, Handle>,
    open: bool,
) -> OperationAdmission {
    if !admitted(&caller, "panels_sheet") {
        return super::rejected_operation();
    }
    dispatch(caller.app_handle(), &shell, PanelCommand::Sheet(open))
}

/// The browser's items in a page's own menu, in the interface language.
pub(crate) fn publish_page_menu_labels() {
    zephium_engine::page_menu::set_page_menu_labels(zephium_engine::page_menu::PageMenuLabels {
        open_link_in_panel: super::menu_text::label("Open Link in Web Panel"),
        search_in_panel: super::menu_text::label("Search in Web Panel"),
    });
}

/// The tab menu's "Move Tab to Panels": the page moves, the tab closes.
pub(crate) fn move_tab(app: &AppHandle, tab: ItemId) -> OperationAdmission {
    match app.try_state::<Handle>() {
        Some(shell) => dispatch(app, &shell, PanelCommand::MoveTab(tab)),
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
