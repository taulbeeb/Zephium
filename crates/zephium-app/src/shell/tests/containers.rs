use super::*;
use crate::ContainerCommand;
use zephium_core::ids::ContainerId;

fn browsed() -> (Shell, Arc<FakeEngine>, Screen, Arc<FakeStore>) {
    let store = Arc::new(FakeStore::default());
    let (mut shell, engine, screen) = setup_with(store.clone());
    shell.handle(Command::Bootstrap);
    (shell, engine, screen, store)
}

fn container_op(shell: &mut Shell, command: ContainerCommand) -> OperationDisposition {
    shell.handle_operation(Command::Container(Box::new(command)))
}

fn create(shell: &mut Shell, screen: &Screen, name: &str) -> ContainerId {
    let made = container_op(
        shell,
        ContainerCommand::Create {
            name: name.into(),
            color: 2,
            symbol: 3,
        },
    );
    assert_eq!(made.outcome, OperationOutcome::Applied);
    let id = last(screen).containers.last().unwrap().id.clone();
    ContainerId::parse(&id).unwrap()
}

fn open_in(shell: &mut Shell, screen: &Screen, container: ContainerId, input: &str) -> ItemId {
    let opened = container_op(shell, ContainerCommand::OpenTab(Some(container)));
    assert_ne!(opened.outcome, OperationOutcome::Rejected);
    let id = active_id(screen);
    navigate_and_commit(shell, id, input);
    id
}

#[test]
fn a_container_tab_loads_in_its_containers_store_and_is_drawn_as_such() {
    let (mut shell, engine, screen, _store) = browsed();
    let work = create(&mut shell, &screen, "  Work ");
    let state = last(&screen);
    assert_eq!(state.containers.len(), 1);
    assert_eq!(state.containers[0].name, "Work");
    assert_eq!(
        (state.containers[0].color, state.containers[0].symbol),
        (2, 3)
    );

    let tab = open_in(&mut shell, &screen, work, "mail.example");
    assert!(engine
        .calls()
        .iter()
        .any(|call| call.starts_with(&format!("create {tab} "))
            && call.ends_with(&format!("[container {work}]"))));
    let view = last(&screen)
        .tabs
        .into_iter()
        .find(|view| view.id == tab.to_string())
        .unwrap();
    assert_eq!(view.container, Some(work.to_string()));
    assert_eq!(last(&screen).containers[0].tabs, 1);
}

#[test]
fn duplicates_and_pages_opened_from_a_container_tab_stay_in_it() {
    let (mut shell, engine, screen, _store) = browsed();
    let work = create(&mut shell, &screen, "Work");
    let tab = open_in(&mut shell, &screen, work, "mail.example");

    shell.handle(Command::TabAction {
        id: tab,
        action: TabAction::Duplicate,
    });
    let copy = active_id(&screen);
    assert_ne!(copy, tab);
    assert_eq!(shell.items.tab(copy).unwrap().container(), Some(work));

    shell.open_linked_tab(tab, "https://docs.example/");
    let linked = active_id(&screen);
    assert_eq!(shell.items.tab(linked).unwrap().container(), Some(work));
    assert!(engine
        .calls()
        .iter()
        .any(|call| call.starts_with(&format!("create {linked} "))
            && call.ends_with(&format!("[container {work}]"))));
}

#[test]
fn reopening_in_another_container_makes_a_new_tab_and_keeps_the_original() {
    let (mut shell, _engine, screen, _store) = browsed();
    let work = create(&mut shell, &screen, "Work");
    let home = create(&mut shell, &screen, "Home");
    let tab = open_in(&mut shell, &screen, work, "mail.example");

    let same = container_op(
        &mut shell,
        ContainerCommand::ReopenIn {
            tab,
            container: Some(work),
        },
    );
    assert_eq!(same.outcome, OperationOutcome::NoOp);
    let moved = container_op(
        &mut shell,
        ContainerCommand::ReopenIn {
            tab,
            container: Some(home),
        },
    );
    assert_eq!(moved.outcome, OperationOutcome::Applied);
    let copy = active_id(&screen);
    assert_ne!(copy, tab);
    assert_eq!(shell.items.tab(copy).unwrap().container(), Some(home));
    assert_eq!(shell.items.tab(tab).unwrap().container(), Some(work));
    let plain = container_op(
        &mut shell,
        ContainerCommand::ReopenIn {
            tab,
            container: None,
        },
    );
    assert_eq!(plain.outcome, OperationOutcome::Applied);
    assert_eq!(
        shell.items.tab(active_id(&screen)).unwrap().container(),
        None
    );
}

#[test]
fn a_container_of_another_profile_or_an_invalid_look_is_refused() {
    let (mut shell, _engine, screen, _store) = browsed();
    let stranger = ContainerId::generate();
    for command in [
        ContainerCommand::OpenTab(Some(stranger)),
        ContainerCommand::Delete(stranger),
        ContainerCommand::Update {
            id: stranger,
            name: "X".into(),
            color: 0,
            symbol: 0,
        },
        ContainerCommand::Create {
            name: "Bad".into(),
            color: zephium_core::containers::CONTAINER_COLORS,
            symbol: 0,
        },
    ] {
        assert_eq!(
            container_op(&mut shell, command).outcome,
            OperationOutcome::Rejected
        );
    }
    let work = create(&mut shell, &screen, "Work");
    let renamed = container_op(
        &mut shell,
        ContainerCommand::Update {
            id: work,
            name: "".into(),
            color: 1,
            symbol: 1,
        },
    );
    assert_eq!(renamed.outcome, OperationOutcome::Rejected);
    assert_eq!(last(&screen).containers[0].name, "Work");
}

#[test]
fn deleting_a_container_closes_its_tabs_forgets_their_history_and_erases_its_store() {
    let (mut shell, engine, screen, _store) = browsed();
    let profile = shell.windows.focused().unwrap().profile;
    let work = create(&mut shell, &screen, "Work");
    let keep = create(&mut shell, &screen, "Keep");
    let first = open_in(&mut shell, &screen, work, "mail.example");
    let second = open_in(&mut shell, &screen, work, "calendar.example");
    shell.handle(Command::Close(second));
    assert!(shell
        .recently_closed
        .iter()
        .any(|entry| entry.container == Some(work)));

    let deleted = container_op(&mut shell, ContainerCommand::Delete(work));
    assert_eq!(deleted.outcome, OperationOutcome::Applied);
    assert!(shell.items.get(first).is_none());
    assert!(shell
        .recently_closed
        .iter()
        .all(|entry| entry.container != Some(work)));
    let state = last(&screen);
    assert_eq!(
        state
            .containers
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Keep"]
    );
    assert!(engine
        .calls()
        .contains(&format!("retain containers {profile} [{keep}]")));
}

#[test]
fn containers_and_their_tabs_survive_a_restart_and_closed_tabs_reopen_in_them() {
    let (mut shell, _engine, screen, store) = browsed();
    let work = create(&mut shell, &screen, "Work");
    let tab = open_in(&mut shell, &screen, work, "mail.example");
    let closed = open_in(&mut shell, &screen, work, "calendar.example");
    shell.handle(Command::Close(closed));
    shell.persist();
    let saved = store.saved.lock().unwrap().clone().unwrap();
    assert_eq!(saved.containers.containers.len(), 1);

    let restarted = Arc::new(FakeStore::default());
    *restarted.saved.lock().unwrap() = Some(saved);
    let (mut shell, engine, screen) = setup_with(restarted);
    shell.handle(Command::Bootstrap);
    let profile = shell.windows.focused().unwrap().profile;
    assert_eq!(shell.items.tab(tab).unwrap().container(), Some(work));
    assert_eq!(last(&screen).containers[0].name, "Work");
    // The launch sweep keeps the stores of containers that still exist.
    assert!(engine
        .calls()
        .contains(&format!("retain containers {profile} [{work}]")));

    let (reopened, _) = shell
        .restore_recently_closed_tab(profile, None)
        .expect("the closed container tab comes back");
    assert_eq!(shell.items.tab(reopened).unwrap().container(), Some(work));
}
