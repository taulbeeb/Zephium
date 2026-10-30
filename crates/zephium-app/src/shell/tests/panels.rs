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
fn a_hidden_sidebar_keeps_the_panel_docked_and_puts_the_strip_away() {
    let (mut shell, engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");
    shell.windows.focused_mut().unwrap().metrics.sidebar_hidden = true;
    let metrics = shell.layout_metrics(shell.windows.focused().unwrap());
    assert_eq!(metrics.panel_strip, 0.0);
    assert!(metrics.panel > 0.0);
    let _ = shell.relayout();
    assert!(layouts(&engine)
        .pop()
        .unwrap()
        .ends_with(&format!("{tab},{panel}")));
}

#[test]
fn settings_rename_and_a_phone_layout_rebuilds_the_page() {
    let (mut shell, engine, screen, _store) = browsed();
    let panel = add(&mut shell, "chat.example");
    let creates = |engine: &FakeEngine| {
        engine
            .calls()
            .iter()
            .filter(|call| call.starts_with(&format!("create {panel} ")))
            .count()
    };
    let before = creates(&engine);
    assert!(done(panel_op(
        &mut shell,
        PanelCommand::Configure {
            id: panel,
            settings: zephium_core::panels::PanelSettings {
                name: Some(" Team ".into()),
                mobile: true,
                ..Default::default()
            },
        },
    )));
    assert!(engine.calls().contains(&format!("mobile {panel} true")));
    assert_eq!(creates(&engine), before + 1);
    let view = &last(&screen).web_panels[0];
    assert_eq!(view.settings.name.as_deref(), Some("Team"));
    assert!(view.settings.mobile);
    // The same settings again change nothing.
    assert_eq!(
        panel_op(
            &mut shell,
            PanelCommand::Configure {
                id: panel,
                settings: zephium_core::panels::PanelSettings {
                    name: Some("Team".into()),
                    mobile: true,
                    ..Default::default()
                },
            },
        )
        .outcome,
        OperationOutcome::NoOp
    );
}

#[test]
fn a_panel_zooms_and_answers_its_number_key() {
    let (mut shell, engine, _screen, _store) = browsed();
    let first = add(&mut shell, "chat.example");
    let second = add(&mut shell, "music.example");
    assert_ne!(
        panel_op(
            &mut shell,
            PanelCommand::Zoom {
                id: second,
                step: crate::PanelZoom::In,
            },
        )
        .outcome,
        OperationOutcome::Rejected
    );
    assert!(engine
        .calls()
        .iter()
        .any(|call| call.starts_with(&format!("zoom {second}"))));
    let profile = profile(&shell);
    assert!(done(panel_op(&mut shell, PanelCommand::Select(1))));
    assert_eq!(shell.panels.open(profile), Some(first));
    assert!(done(panel_op(&mut shell, PanelCommand::Select(1))));
    assert_eq!(shell.panels.open(profile), None);
    assert_eq!(
        panel_op(&mut shell, PanelCommand::Select(9)).outcome,
        OperationOutcome::NoOp
    );
}

#[test]
fn moving_a_tab_to_the_panels_closes_the_tab() {
    let (mut shell, _engine, screen, _store) = browsed();
    let first = active_id(&screen);
    navigate_and_commit(&mut shell, first, "news.example");
    shell.handle(Command::Open);
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "mail.example");
    assert_ne!(
        panel_op(&mut shell, PanelCommand::MoveTab(tab)).outcome,
        OperationOutcome::Rejected
    );
    assert!(shell.items.get(tab).is_none());
    let panel = *shell.profile_panels(profile(&shell)).last().unwrap();
    assert_eq!(
        shell
            .items
            .tab(panel)
            .unwrap()
            .url
            .as_ref()
            .map(ToString::to_string),
        Some("https://mail.example/".into())
    );
}

#[test]
fn a_panel_set_to_unload_on_close_sleeps_when_it_closes() {
    let (mut shell, _engine, _screen, _store) = browsed();
    let panel = add(&mut shell, "chat.example");
    panel_op(
        &mut shell,
        PanelCommand::Configure {
            id: panel,
            settings: zephium_core::panels::PanelSettings {
                unload_on_close: true,
                ..Default::default()
            },
        },
    );
    assert!(shell.items.tab(panel).unwrap().has_view());
    assert!(done(panel_op(&mut shell, PanelCommand::Open(None))));
    assert!(!shell.items.tab(panel).unwrap().has_view());
}

#[test]
fn timers_put_an_idle_closed_panel_to_sleep_and_reload_an_awake_one() {
    let (mut shell, engine, _screen, _store) = browsed();
    let idle = add(&mut shell, "chat.example");
    let busy = add(&mut shell, "news.example");
    for (id, settings) in [
        (
            idle,
            zephium_core::panels::PanelSettings {
                unload_after_minutes: Some(5),
                ..Default::default()
            },
        ),
        (
            busy,
            zephium_core::panels::PanelSettings {
                reload_every_seconds: Some(60),
                ..Default::default()
            },
        ),
    ] {
        panel_op(&mut shell, PanelCommand::Configure { id, settings });
    }
    // `busy` is open, so `idle` was closed when `busy` opened.
    assert!(shell.items.tab(idle).unwrap().has_view());
    let long_ago = std::time::Instant::now() - std::time::Duration::from_secs(10 * 60);
    shell.panel_timers.age_for_test(idle, busy, long_ago);
    shell.handle(Command::PanelWake);
    assert!(!shell.items.tab(idle).unwrap().has_view());
    assert!(shell.items.tab(busy).unwrap().has_view());
    assert!(engine.calls().contains(&format!("reload {busy}")));
}

#[test]
fn a_floating_panel_leaves_the_page_its_width_and_isolates_its_selector() {
    let (mut shell, engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let panel = add(&mut shell, "chat.example");
    assert!(done(panel_op(
        &mut shell,
        PanelCommand::Configure {
            id: panel,
            settings: zephium_core::panels::PanelSettings {
                floating: true,
                selector: Some("#messages".into()),
                icon: Some(zephium_core::spaces::SpaceIcon::Emoji("💬".into())),
                ..Default::default()
            },
        },
    )));
    let window = shell.windows.focused().unwrap();
    let metrics = shell.layout_metrics(window);
    assert!(metrics.panel_floats);
    let page = layout::compute(window.size, window.mode, metrics, true)
        .content
        .unwrap();
    // The page keeps its own region; the panel is drawn over its edge.
    let floating = engine
        .layout_regions
        .lock()
        .unwrap()
        .last()
        .copied()
        .flatten();
    assert_eq!(floating, Some(page));
    let beside = layout::Metrics {
        panel_floats: false,
        ..metrics
    };
    let narrowed = layout::compute(window.size, window.mode, beside, true)
        .content
        .unwrap();
    assert!(narrowed.width + metrics.panel <= page.width);
    assert!(layouts(&engine)
        .pop()
        .unwrap()
        .ends_with(&format!("{tab},{panel}")));
    assert!(engine.calls().contains(&format!("style {panel} true")));
    let view = &last(&screen).web_panels[0].settings;
    assert!(view.floating);
    assert_eq!(view.selector.as_deref(), Some("#messages"));
    assert_eq!(
        view.icon,
        Some(zephium_ipc::SpaceIconView::Emoji {
            text: "💬".into()
        })
    );
    assert!(done(panel_op(&mut shell, PanelCommand::Remove(panel))));
    assert!(engine.calls().contains(&format!("style {panel} false")));
}

#[test]
fn a_link_from_the_page_menu_becomes_a_panel_in_the_page_container() {
    let (mut shell, _engine, screen, _store) = browsed();
    let tab = active_id(&screen);
    navigate_and_commit(&mut shell, tab, "news.example");
    let profile = profile(&shell);
    shell.handle(Command::Engine(EngineEvent::OpenInPanelRequested {
        id: tab,
        url: "https://chat.example/".into(),
    }));
    let panels = shell.profile_panels(profile);
    assert_eq!(panels.len(), 1);
    assert_eq!(shell.panels.open(profile), Some(panels[0]));
    assert_eq!(active_id(&screen), tab);
    // A page out of scope, or a link that is not a web page, adds nothing.
    shell.handle(Command::Engine(EngineEvent::OpenInPanelRequested {
        id: ItemId::generate(),
        url: "https://other.example/".into(),
    }));
    shell.handle(Command::Engine(EngineEvent::OpenInPanelRequested {
        id: tab,
        url: "file:///etc/passwd".into(),
    }));
    assert_eq!(shell.profile_panels(profile).len(), 1);
}

#[test]
fn a_waiting_strip_keeps_only_the_edge_until_the_pointer_brings_it_out() {
    let (mut shell, _engine, _screen, _store) = browsed();
    let _panel = add(&mut shell, "chat.example");
    let strip = |shell: &Shell| {
        shell
            .layout_metrics(shell.windows.focused().unwrap())
            .panel_strip
    };
    assert_eq!(strip(&shell), zephium_core::panels::PANEL_STRIP_WIDTH);
    // Peeking means nothing while the strip always shows.
    assert_eq!(
        panel_op(&mut shell, PanelCommand::PeekStrip(true)).outcome,
        OperationOutcome::NoOp
    );
    let _ = shell.handle_operation(Command::SetAppSetting {
        key: "panels.strip".into(),
        value: "auto".into(),
    });
    let padding = shell.windows.focused().unwrap().metrics.padding;
    assert_eq!(strip(&shell), padding);
    assert!(done(panel_op(&mut shell, PanelCommand::PeekStrip(true))));
    assert_eq!(strip(&shell), zephium_core::panels::PANEL_STRIP_WIDTH);
    assert!(done(panel_op(&mut shell, PanelCommand::PeekStrip(false))));
    assert_eq!(strip(&shell), padding);
}
