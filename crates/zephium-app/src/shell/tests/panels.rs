use super::*;
use crate::{PanelCommand, PanelNavigation};
use zephium_core::item::Placement;

fn browsed() -> (Shell, Arc<FakeEngine>, Screen, Arc<FakeStore>) {
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    (shell, engine, screen, store)
}

fn panel_op(shell: &mut Shell, command: PanelCommand) -> OperationDisposition {
    shell.handle_operation(Command::Panel(Box::new(command)))
}

fn profile(shell: &Shell) -> ProfileId {
    shell.windows.focused().unwrap().profile
}

/// Adds a panel and commits its first page, so it is on screen.
fn add(shell: &mut Shell, input: &str) -> ItemId {
    let added = panel_op(
        shell,
        PanelCommand::Add {
            url: input.into(),
            container: None,
        },
    );
    assert_ne!(added.outcome, OperationOutcome::Rejected);
    let profile = profile(shell);
    let id = *shell.profile_panels(profile).last().unwrap();
    let url = navigation::classify(input).unwrap().to_string();
    commit_url(shell, id, &url);
    present_committed(shell, id, &url);
    id
}

/// Applied, or deferred while the native layout it queued is settling.
fn done(disposition: OperationDisposition) -> bool {
    matches!(
        disposition.outcome,
        OperationOutcome::Applied | OperationOutcome::Deferred
    )
}

fn layouts(engine: &FakeEngine) -> Vec<String> {
    engine
        .calls()
        .into_iter()
        .filter(|call| call.starts_with("layout@"))
        .collect()
}

#[test]
fn an_added_panel_opens_beside_the_page_and_never_joins_the_tabs() {
    let (mut shell, engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");

    assert!(engine
        .calls()
        .iter()
        .any(|call| call.starts_with(&format!("create {panel} https://chat.example/"))));
    let state = last(&screen);
    assert_eq!(state.web_panels.len(), 1);
    assert!(state.web_panels[0].open);
    assert_eq!(state.web_panels[0].tab.id, panel.to_string());
    assert!(state.tabs.iter().all(|view| view.id != panel.to_string()));
    assert_eq!(state.active, Some(tab.to_string()));
    assert!(matches!(
        shell.items.get(panel).unwrap().placement,
        Placement::Panel { .. }
    ));
    let last_layout = layouts(&engine).pop().unwrap();
    assert!(
        last_layout.ends_with(&format!("{tab},{panel}")),
        "{last_layout}"
    );
}

#[test]
fn closing_and_toggling_keep_the_panel_and_bring_it_back() {
    let (mut shell, engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");

    assert!(done(panel_op(&mut shell, PanelCommand::Toggle)));
    assert_eq!(shell.panels.open(profile(&shell)), None);
    assert!(layouts(&engine).pop().unwrap().ends_with(&tab.to_string()));
    assert!(!last(&screen).web_panels[0].open);

    assert_ne!(
        panel_op(&mut shell, PanelCommand::Toggle).outcome,
        OperationOutcome::Rejected
    );
    assert_eq!(shell.panels.open(profile(&shell)), Some(panel));
    assert!(layouts(&engine)
        .pop()
        .unwrap()
        .ends_with(&panel.to_string()));
}

#[test]
fn panels_reorder_resize_and_go_away_cleanly() {
    let (mut shell, _engine, screen, _store) = browsed();
    let first = add(&mut shell, "chat.example");
    let second = add(&mut shell, "music.example");
    let profile = profile(&shell);
    assert_eq!(shell.profile_panels(profile), vec![first, second]);

    assert!(done(panel_op(
        &mut shell,
        PanelCommand::Move {
            id: second,
            before: Some(first),
        },
    )));
    assert_eq!(shell.profile_panels(profile), vec![second, first]);

    assert!(done(panel_op(
        &mut shell,
        PanelCommand::Resize {
            id: second,
            width: 520.0,
        },
    )));
    assert_eq!(shell.panels.width(second), 520);

    assert!(done(panel_op(&mut shell, PanelCommand::Remove(second))));
    assert!(shell.items.get(second).is_none());
    assert_eq!(shell.panels.open(profile), None);
    assert_eq!(last(&screen).web_panels.len(), 1);
}

#[test]
fn panels_only_answer_to_panel_commands_of_their_profile() {
    let (mut shell, _engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");
    // A tab command never treats a panel as a tab, and a panel command never
    // treats a tab as a panel.
    assert_eq!(
        shell.handle_operation(Command::Close(panel)).outcome,
        OperationOutcome::Rejected
    );
    for command in [
        PanelCommand::Remove(tab),
        PanelCommand::Open(Some(tab)),
        PanelCommand::Resize {
            id: tab,
            width: 500.0,
        },
        PanelCommand::Add {
            url: "javascript:alert(1)".into(),
            container: None,
        },
        PanelCommand::Add {
            url: "https://chat.example/".into(),
            container: Some(zephium_core::ids::ContainerId::generate()),
        },
    ] {
        assert_eq!(
            panel_op(&mut shell, command).outcome,
            OperationOutcome::Rejected
        );
    }
    // The panel's own toolbar reaches it.
    assert_ne!(
        panel_op(
            &mut shell,
            PanelCommand::Navigate {
                id: panel,
                action: PanelNavigation::Reload,
            },
        )
        .outcome,
        OperationOutcome::Rejected
    );
}

#[test]
fn a_tab_becomes_a_panel_in_its_own_container_and_stays_a_tab() {
    let (mut shell, engine, screen, _store) = browsed();
    let profile = profile(&shell);
    shell.handle_operation(Command::Container(Box::new(
        crate::ContainerCommand::Create {
            name: "Work".into(),
            color: 0,
            symbol: 0,
        },
    )));
    let work = shell.containers.of_profile(profile).next().unwrap().id;
    shell.handle_operation(Command::Container(Box::new(
        crate::ContainerCommand::OpenTab(Some(work)),
    )));
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "mail.example");

    assert_ne!(
        panel_op(&mut shell, PanelCommand::AddTab(tab)).outcome,
        OperationOutcome::Rejected
    );
    let panel = *shell.profile_panels(profile).last().unwrap();
    assert_eq!(shell.items.tab(panel).unwrap().container(), Some(work));
    assert!(shell.items.tab(tab).is_some());
    assert!(engine.calls().iter().any(|call| {
        call.starts_with(&format!("create {panel} "))
            && call.ends_with(&format!("[container {work}]"))
    }));
}

#[test]
fn a_link_opened_in_a_panel_lands_as_a_tab_of_the_current_space() {
    let (mut shell, _engine, screen, _store) = browsed();
    let panel = add(&mut shell, "chat.example");
    let space = shell.windows.focused().unwrap().space;
    shell.open_linked_tab(panel, "https://docs.example/");
    let opened = active_id(&screen);
    assert_ne!(opened, panel);
    assert!(matches!(
        shell.items.get(opened).unwrap().placement,
        Placement::Space { space: s, .. } if s == space
    ));
}

#[test]
fn panels_and_the_open_one_survive_a_restart() {
    let (mut shell, engine, _screen, store) = browsed();
    let panel = add(&mut shell, "chat.example");
    panel_op(
        &mut shell,
        PanelCommand::Resize {
            id: panel,
            width: 480.0,
        },
    );
    shell.persist();
    let saved = store.saved.lock().unwrap().clone().unwrap();
    drop(engine);

    let restarted = Arc::new(FakeStore::default());
    *restarted.saved.lock().unwrap() = Some(saved);
    let (mut shell, engine, screen) = setup_with(restarted);
    shell.handle(Command::Bootstrap);
    let profile = profile(&shell);
    assert_eq!(shell.profile_panels(profile), vec![panel]);
    assert_eq!(shell.panels.open(profile), Some(panel));
    assert_eq!(shell.panels.width(panel), 480);
    assert!(engine
        .calls()
        .iter()
        .any(|call| call.starts_with(&format!("create {panel} "))));
    assert_eq!(last(&screen).web_panels.len(), 1);
}

#[test]
fn a_hidden_sidebar_puts_the_panels_away_until_it_comes_back() {
    let (mut shell, engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");
    shell.windows.focused_mut().unwrap().metrics.sidebar_hidden = true;
    assert_eq!(shell.shown_panel(), None);
    let _ = shell.relayout();
    assert!(layouts(&engine).pop().unwrap().ends_with(&tab.to_string()));
    shell.windows.focused_mut().unwrap().metrics.sidebar_hidden = false;
    let _ = shell.relayout();
    assert!(layouts(&engine)
        .pop()
        .unwrap()
        .ends_with(&panel.to_string()));
}
