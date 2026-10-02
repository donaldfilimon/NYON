//! The shell's galaxy sandbox is a session of its own: opening it leaves
//! Classic and Galaxy Workshop, and a stepped boundary shows up in the frame
//! the shell paints.

use glam::Vec2;
use nyon::{
    app::{
        AppCore,
        client_runtime::{ActiveSession, ClientRuntime, ClientScreen, MainMenuRoute},
    },
    preferences::store::MemoryPreferencesStore,
    sandbox_session::SandboxSession,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    ui::platform::PlatformUiFrame,
    workshop::store::MemoryWorkshopStore,
};

type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;

fn runtime() -> Runtime {
    let classic = AppCore::new_with_preferences(
        ScenarioDraft::factory_default()
            .validated()
            .expect("factory"),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    ClientRuntime::new(classic, MemoryWorkshopStore::default())
}

fn home_line(frame: &PlatformUiFrame) -> String {
    frame
        .status_lines
        .iter()
        .find(|line| line.to_ascii_lowercase().contains("aster vale"))
        .cloned()
        .unwrap_or_default()
}

#[test]
fn the_shell_steps_a_galaxy_sandbox_and_paints_the_changed_inventory() {
    let opened = SandboxSession::open().expect("built-in galaxy opens");
    assert!(opened.paused());
    assert_eq!(opened.tick(), 0);
    let genesis = opened.frame(Vec2::new(1280.0, 720.0), false);
    let genesis_inventory = home_line(&genesis);
    assert!(
        genesis_inventory
            .to_ascii_lowercase()
            .contains("aster vale")
    );

    let mut runtime = runtime();
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .expect("menu opens the sandbox");
    assert_eq!(runtime.screen(), ClientScreen::GalaxySandbox);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert!(runtime.workshop_snapshot().is_none());
    assert_eq!(runtime.sandbox_tick(), Some(0));
    assert_eq!(runtime.sandbox_paused(), Some(true));

    runtime.update(std::time::Duration::ZERO);
    assert_eq!(
        runtime.sandbox_tick(),
        Some(0),
        "a paused sandbox stays on its boundary across an update"
    );

    runtime.step_sandbox(10).expect("ten boundaries");
    assert_eq!(runtime.sandbox_tick(), Some(10));
    assert_eq!(runtime.sandbox_paused(), Some(true));

    let frame = runtime
        .sandbox_frame(Vec2::new(1280.0, 720.0), false)
        .expect("sandbox frame");
    assert_eq!(frame.semantics.root.id.as_str(), "sandbox.application");
    assert_ne!(frame.semantics.root.id.as_str(), "workshop.application");
    assert_eq!(frame.semantics.root.name.as_str(), "Living Galaxy V2");
    assert!(
        frame
            .status_lines
            .iter()
            .any(|line| line.starts_with("Living Galaxy / "))
    );
    assert!(
        frame
            .visible_nodes
            .iter()
            .any(|node| { node.display_text.starts_with("Living Galaxy / ") })
    );
    let stepped = home_line(&frame);
    assert!(stepped.to_ascii_lowercase().contains("aster vale"));
    assert_ne!(stepped, genesis_inventory);
    assert!(frame.visible_nodes.iter().any(|node| {
        node.display_text
            .to_ascii_lowercase()
            .contains("aster vale")
            && node.display_text != genesis_inventory
    }));
    assert!(
        frame
            .status_lines
            .iter()
            .any(|line| line.contains("tick 10"))
    );
    eprintln!(
        "LivingGalaxyAuthorityV2 screen {:?} tick {} entity {stepped}",
        runtime.screen(),
        runtime.sandbox_tick().expect("tick")
    );
}

#[test]
fn transport_depends_on_elapsed_time_instead_of_update_count() {
    use nyon::sandbox_session::SandboxSpeed;
    use std::time::Duration;
    for speed in [
        SandboxSpeed::Normal,
        SandboxSpeed::Fast,
        SandboxSpeed::Fastest,
    ] {
        let mut fine = SandboxSession::open().unwrap();
        let mut coarse = SandboxSession::open().unwrap();
        fine.set_speed(speed);
        coarse.set_speed(speed);
        fine.toggle_pause();
        coarse.toggle_pause();
        for _ in 0..100 {
            fine.set_suspended(false);
            fine.update(Duration::from_millis(10)).unwrap();
            drain_steps(&mut fine);
        }
        for _ in 0..10 {
            coarse.update(Duration::from_millis(100)).unwrap();
            drain_steps(&mut coarse);
        }
        assert_eq!(fine.tick(), 10 * u64::from(speed.multiplier()));
        assert_eq!(
            fine.state().digest().unwrap(),
            coarse.state().digest().unwrap()
        );
    }
}

#[test]
fn bounded_catch_up_discards_whole_debt_and_pause_discards_fractional_debt() {
    use std::time::Duration;
    let mut session = SandboxSession::open().unwrap();
    session.toggle_pause();
    session.update(Duration::from_secs(600)).unwrap();
    let frame = session.frame(Vec2::new(723.0, 802.0), false);
    assert!(
        frame
            .status_lines
            .iter()
            .any(|line| line.contains("effective") && line.contains("capped"))
    );
    drain_steps(&mut session);
    assert_eq!(session.tick(), 20);
    assert_eq!(session.update(Duration::ZERO).unwrap(), 0);
    session.update(Duration::from_millis(90)).unwrap();
    session.set_paused(true);
    session.update(Duration::from_secs(600)).unwrap();
    session.set_paused(false);
    assert_eq!(session.update(Duration::from_millis(10)).unwrap(), 0);
    session.set_suspended(true);
    session.update(Duration::from_secs(600)).unwrap();
    session.set_suspended(false);
    assert_eq!(session.update(Duration::from_millis(90)).unwrap(), 0);
    session.update(Duration::from_millis(10)).unwrap();
    drain_steps(&mut session);
    assert_eq!(session.tick(), 21);
}

#[test]
fn nested_holds_are_session_scoped_and_preserve_explicit_pause() {
    use std::time::Duration;
    let mut session = SandboxSession::open().unwrap();
    session.toggle_pause();
    let outer = session.begin_hold().unwrap();
    let inner = session.begin_hold().unwrap();
    assert!(session.paused());
    assert_eq!(session.update(Duration::from_secs(600)).unwrap(), 0);
    assert!(session.end_hold(inner));
    assert!(session.paused());
    session.set_paused(true);
    assert!(session.end_hold(outer));
    assert!(session.paused());
    let mut previous = SandboxSession::open().unwrap();
    let stale = previous.begin_hold().unwrap();
    let mut replacement = SandboxSession::open().unwrap();
    assert!(!replacement.end_hold(stale));
    assert!(replacement.paused());
    session.set_paused(false);
    let hold = session.begin_hold().unwrap();
    assert!(session.end_hold(hold));
    assert!(!session.paused());
    assert_eq!(session.update(Duration::from_millis(99)).unwrap(), 0);
}

#[test]
fn transport_actions_have_matching_semantics_and_logical_hit_bounds() {
    use nyon::sandbox_session::{SandboxSpeed, SandboxUiAction};
    let mut session = SandboxSession::open().unwrap();
    for viewport in [Vec2::new(723.0, 802.0), Vec2::new(1280.0, 720.0)] {
        let frame = session.frame(viewport, true);
        frame.semantics.validate().unwrap();
        assert!(frame.controls.len() >= 5);
        for control in &frame.controls {
            assert!(control.bounds.width() >= 44.0);
            assert!(control.bounds.height() >= 44.0);
            assert!(control.bounds.max.x <= viewport.x);
            assert!(control.bounds.max.y <= viewport.y);
            let semantic = frame.semantics.node(control.semantic_id.as_str()).unwrap();
            assert_eq!(semantic.action_id.as_ref(), Some(&control.action_id));
            assert_eq!(semantic.bounds, Some(control.bounds.into()));
            if control.enabled {
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    control.action_id
                );
            }
        }
    }
    session.apply_action(SandboxUiAction::Step).unwrap();
    drain_steps(&mut session);
    assert_eq!(session.tick(), 1);
    session
        .apply_action(SandboxUiAction::Speed(SandboxSpeed::Fast))
        .unwrap();
    session.apply_action(SandboxUiAction::TogglePause).unwrap();
    assert!(session.apply_action(SandboxUiAction::Step).is_err());
    assert_eq!(session.tick(), 1);
    assert_eq!(session.speed(), SandboxSpeed::Fast);
}

#[test]
fn selection_and_inspector_paging_never_change_authority() {
    use nyon::sandbox_session::SandboxUiAction;
    let mut session = SandboxSession::open().unwrap();
    let world = session.state().worlds.as_slice().last().unwrap().id;
    let digest = session.state().digest().unwrap();
    session
        .apply_action(SandboxUiAction::SelectWorld(world))
        .unwrap();
    assert_eq!(session.selected_world(), Some(world));
    for scale in [1.0, 1.15, 1.3] {
        let viewport = Vec2::new(723.0, 802.0);
        let frame = session.frame_with_scale(viewport, scale, false);
        frame.semantics.validate().unwrap();
        assert!(!frame.sighted_text.is_empty());
        for text in &frame.sighted_text {
            assert!(text.clip.contains_rect(text.bounds));
            assert!(text.bounds.max.y < viewport.y - 44.0 * scale);
            assert!(
                frame
                    .visible_nodes
                    .iter()
                    .any(|node| node.semantic_id == text.semantic_id)
            );
        }
    }
    session
        .apply_action(SandboxUiAction::InspectorPage(false))
        .unwrap();
    assert_eq!(session.state().digest().unwrap(), digest);
    assert_eq!(session.tick(), 0);
    assert!(session.receipts().is_empty());
    assert!(
        session
            .select_world(nyon_workshop_core::living::LivingEntityIdV2([255; 16]))
            .is_err()
    );
    assert_eq!(session.selected_world(), Some(world));
    assert_eq!(session.state().digest().unwrap(), digest);
}

#[test]
fn chronicle_retains_only_committed_receipts_and_preserves_provenance() {
    use nyon_workshop_core::living::LivingEventProvenanceV2;
    let mut session = SandboxSession::open().unwrap();
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    assert_eq!(session.receipts().len(), 16);
    assert_eq!(session.receipts().front().unwrap().tick.0, 85);
    let last = session.receipts().back().unwrap();
    last.verify().unwrap();
    assert_eq!(last.tick.0, session.tick());
    assert_eq!(last.state_digest, session.state().digest().unwrap());
    assert!(!last.events.is_empty());
    assert!(
        last.events
            .iter()
            .all(|event| matches!(event.provenance, LivingEventProvenanceV2::Autonomous { .. }))
    );
}

#[test]
fn decision_inspector_pairs_real_intents_with_their_ordinary_receipt_only() {
    use nyon::sandbox_session::SandboxUiAction;
    use nyon::{living_content::LivingContent, living_creator::LivingCreatorTool};
    let mut session = SandboxSession::from_authority(LivingContent::FirstExpansion.open().unwrap());
    for _ in 0..50 {
        session.step_once().unwrap();
    }
    let (ordinary, decision) = session.decision_receipts().back().unwrap();
    decision.verify_receipt(ordinary).unwrap();
    assert!(!decision.intents.is_empty());
    assert_eq!(decision.receipt_digest, ordinary.digest);
    assert_eq!(decision.state_after, ordinary.state_digest);
    let actor = decision.intents[0]
        .actor
        .0
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<String>();
    let rank = decision.intents[0].score.0;
    let mut visible = String::new();
    for _ in 0..32 {
        let frame = session.frame(Vec2::new(1280.0, 720.0), false);
        visible.extend(
            frame
                .sighted_text
                .iter()
                .flat_map(|text| text.lines.iter())
                .map(String::as_str),
        );
        let next = frame.controls.iter().find(|control| {
            matches!(
                control.action,
                nyon::ui::platform::PlatformUiAction::Sandbox(SandboxUiAction::InspectorPage(
                    false
                ))
            )
        });
        if !next.is_some_and(|control| control.enabled) {
            break;
        }
        session
            .apply_action(SandboxUiAction::InspectorPage(false))
            .unwrap();
    }
    assert!(visible.to_uppercase().contains(&actor));
    assert!(visible.contains(&format!("rank {rank}")));
    let (ordinary, _) = session.decision_receipts().back().unwrap();
    let paired_digest = ordinary.digest;
    let cached_count = session.decision_receipts().len();
    let state_digest = session.state().digest().unwrap();
    let frame = session.frame(Vec2::new(1280.0, 720.0), false);
    assert!(frame.controls.iter().any(|control| matches!(
        control.action,
        nyon::ui::platform::PlatformUiAction::Sandbox(SandboxUiAction::DecisionPage(false))
    )));
    session
        .apply_action(SandboxUiAction::DecisionPage(false))
        .unwrap();
    session
        .apply_action(SandboxUiAction::InspectorPage(false))
        .unwrap();
    assert_eq!(session.state().digest().unwrap(), state_digest);
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    assert_eq!(session.tick(), 50);
    assert_ne!(session.receipts().back().unwrap().digest, paired_digest);
    assert_eq!(session.decision_receipts().len(), cached_count);
    let creator_rows = session
        .chronicle_rows()
        .into_iter()
        .filter(|row| row.category == nyon::living_chronicle::ChronicleCategory::Creator)
        .collect::<Vec<_>>();
    assert!(!creator_rows.is_empty());
    assert!(creator_rows.iter().all(|row| row.actor.is_none()
        && row.decision_ordinal.is_none()
        && row.receipt_digest != paired_digest));
    let (ordinary, decision) = session.decision_receipts().back().unwrap();
    assert_eq!(ordinary.digest, paired_digest);
    decision.verify_receipt(ordinary).unwrap();
}

#[test]
fn every_nonmodal_control_keeps_minimum_targets_at_supported_scales() {
    for scale in [0.85, 1.0, 1.15, 1.3] {
        for viewport in [Vec2::new(800.0, 480.0), Vec2::new(1280.0, 720.0)] {
            let mut session = SandboxSession::open().unwrap();
            for _ in 0..50 {
                session.step_once().unwrap();
            }
            let digest = session.state().digest().unwrap();
            let frame = session.frame_with_scale(viewport, scale, false);
            frame.semantics.validate().unwrap();
            for control in &frame.controls {
                assert!(
                    control.bounds.width() >= 44.0,
                    "{} at {scale}: {:?}",
                    control.label,
                    control.bounds
                );
                assert!(
                    control.bounds.height() >= 44.0,
                    "{} at {scale}: {:?}",
                    control.label,
                    control.bounds
                );
                assert!(control.bounds.min.min_element() >= 0.0, "{}", control.label);
                assert!(
                    control.bounds.max.x <= viewport.x && control.bounds.max.y <= viewport.y,
                    "{}",
                    control.label
                );
                let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
                assert_eq!(node.bounds, Some(control.bounds.into()));
                if control.enabled {
                    assert_eq!(
                        frame.hit_test(control.bounds.center()).unwrap().action_id,
                        control.action_id
                    );
                }
            }
            for (index, a) in frame.controls.iter().enumerate() {
                for b in frame.controls.iter().skip(index + 1) {
                    let overlap = (a.bounds.max.min(b.bounds.max) - a.bounds.min.max(b.bounds.min))
                        .min_element();
                    assert!(
                        overlap <= 0.0,
                        "{} overlaps {} at {scale}",
                        a.label,
                        b.label
                    );
                }
            }
            for text in &frame.sighted_text {
                assert!(text.clip.contains_rect(text.bounds));
            }
            assert_eq!(session.state().digest().unwrap(), digest);
        }
    }
}

#[test]
fn chronicle_pause_stops_at_the_committed_boundary_and_drops_catchup() {
    use nyon::{
        living_chronicle::ChronicleCategory,
        living_content::LivingContent,
        sandbox_session::{SandboxSpeed, SandboxUiAction},
    };
    use std::time::Duration;
    let mut session = SandboxSession::from_authority(LivingContent::FirstExpansion.open().unwrap());
    session
        .apply_action(SandboxUiAction::ChroniclePause(ChronicleCategory::Decision))
        .unwrap();
    session
        .apply_action(SandboxUiAction::Speed(SandboxSpeed::Fastest))
        .unwrap();
    session.set_paused(false);
    let mut committed = 0;
    while !session.paused() && committed < 100 {
        committed += session.update(Duration::from_secs(1)).unwrap();
        while session.processing() && !session.paused() {
            committed += session.update(Duration::ZERO).unwrap();
        }
    }
    assert!(session.paused());
    assert_eq!(committed, session.tick());
    let decision = session
        .chronicle_rows()
        .into_iter()
        .find(|row| row.category == ChronicleCategory::Decision)
        .unwrap();
    assert_eq!(decision.tick_last, session.tick());
    assert!(decision.actor.is_some());
    assert!(decision.decision_ordinal.is_some());
    session
        .apply_action(SandboxUiAction::ChroniclePause(ChronicleCategory::Decision))
        .unwrap();
    session.set_paused(false);
    let tick = session.tick();
    assert_eq!(session.update(Duration::ZERO).unwrap(), 0);
    assert_eq!(session.tick(), tick);
}

#[test]
fn chronicle_filters_use_actual_decision_actor_and_modal_paging_stays_reachable() {
    use nyon::{
        living_chronicle::ChronicleCategory,
        living_content::LivingContent,
        living_navigation::{LivingEntityKind, LivingEntitySelection},
        sandbox_session::SandboxUiAction,
    };
    let mut session = SandboxSession::from_authority(LivingContent::FirstExpansion.open().unwrap());
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    let actor = session
        .chronicle_rows()
        .into_iter()
        .find_map(|row| row.actor)
        .unwrap();
    let bookmark = session.bookmark();
    session
        .select_entity(LivingEntitySelection::new(
            LivingEntityKind::Civilization,
            actor,
        ))
        .unwrap();
    session
        .apply_action(SandboxUiAction::ChronicleFollowActor(true))
        .unwrap();
    assert!(!session.chronicle_rows().is_empty());
    assert!(
        session
            .chronicle_rows()
            .iter()
            .all(|row| row.actor == Some(actor))
    );
    session
        .apply_action(SandboxUiAction::ChronicleCategory(Some(
            ChronicleCategory::Decision,
        )))
        .unwrap();
    session
        .apply_action(SandboxUiAction::ChroniclePicker)
        .unwrap();
    assert!(session.creator_modal_open());
    for action in [
        SandboxUiAction::ChronicleOptions(false),
        SandboxUiAction::ChronicleOptions(true),
        SandboxUiAction::ChronicleInspect(Some(0)),
    ] {
        session.apply_action(action).unwrap();
        for scale in [0.85, 1.0, 1.15, 1.3] {
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            frame.semantics.validate().unwrap();
            let modal = frame.modal.unwrap();
            for control in frame.controls.iter().filter(|control| control.enabled) {
                assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
                assert!(modal.contains_rect(control.bounds));
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    control.action_id
                );
            }
        }
    }
    assert_eq!(session.bookmark(), bookmark);
}

#[test]
fn introduction_is_dismissible_read_only_and_releases_time_without_catchup() {
    use nyon::sandbox_session::SandboxUiAction;
    use std::time::Duration;
    let mut session = SandboxSession::open().unwrap();
    let bookmark = session.bookmark();
    let dirty = session.dirty();
    let generation = session.notebook_generation();
    session.set_paused(false);
    session.present_intro().unwrap();
    assert!(session.creator_modal_open());
    assert_eq!(session.update(Duration::from_secs(10)).unwrap(), 0);
    let mut observed = String::new();
    for _ in 0..16 {
        let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), 1.3, false);
        frame.semantics.validate().unwrap();
        let modal = frame.modal.unwrap();
        for control in frame.controls.iter().filter(|control| control.enabled) {
            assert!(modal.contains_rect(control.bounds));
            assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
            assert_eq!(
                frame.hit_test(control.bounds.center()).unwrap().action_id,
                control.action_id
            );
        }
        observed.extend(
            frame
                .sighted_text
                .iter()
                .flat_map(|text| text.lines.iter())
                .map(String::as_str),
        );
        if !frame
            .controls
            .iter()
            .any(|control| control.action_id.as_str() == "sandbox.intro.next" && control.enabled)
        {
            break;
        }
        session
            .apply_action(SandboxUiAction::InspectorPage(false))
            .unwrap();
    }
    for concept in ["1. Select", "2. Time", "3. Create", "Preview", "Chronicle"] {
        assert!(observed.contains(concept), "missing {concept}");
    }
    assert_eq!(session.bookmark(), bookmark);
    assert_eq!(session.dirty(), dirty);
    assert_eq!(session.notebook_generation(), generation);
    session
        .apply_action(SandboxUiAction::IntroStart(true))
        .unwrap();
    assert!(!session.creator_modal_open());
    assert!(session.intro_dismissed());
    assert_eq!(session.update(Duration::ZERO).unwrap(), 0);
    session.present_intro().unwrap();
    assert!(!session.creator_modal_open());
    session.apply_action(SandboxUiAction::IntroPicker).unwrap();
    session.apply_action(SandboxUiAction::IntroCreate).unwrap();
    assert!(session.creator_modal_open());
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    assert_eq!(session.bookmark(), bookmark);
}

#[test]
fn storage_status_scope_and_readout_do_not_overlap_interactive_geometry() {
    use nyon::sandbox_session::SandboxUiAction;
    use nyon::ui::platform::PlatformUiAction;
    let mut session = SandboxSession::open().unwrap();
    for viewport in [Vec2::new(800.0, 480.0), Vec2::new(1280.0, 720.0)] {
        for scale in [0.85, 1.0, 1.5] {
            let frame = session.frame_with_scale(viewport, scale, false);
            let storage = frame
                .sighted_text
                .iter()
                .find(|text| text.semantic_id.as_str() == "sandbox.storage.status")
                .unwrap();
            for control in &frame.controls {
                assert!(
                    storage.bounds.intersection(control.bounds).is_none(),
                    "storage collides with {} at {viewport:?} scale{scale}",
                    control.label
                );
            }
            for record in frame
                .visible_nodes
                .iter()
                .filter(|record| record.semantic_id.as_str().starts_with("sandbox.status."))
            {
                assert!(record.bounds.intersection(storage.bounds).is_none());
                for control in &frame.controls {
                    assert!(
                        record.bounds.intersection(control.bounds).is_none(),
                        "readout collides with {}",
                        control.label
                    );
                }
            }
        }
    }
    let digest = session.state().digest().unwrap();
    for (action, label) in [
        (SandboxUiAction::NavigateGalaxy, "Galaxy"),
        (SandboxUiAction::NavigateSystem, "System"),
        (SandboxUiAction::NavigateWorld, "World"),
    ] {
        session.apply_action(action).unwrap();
        let frame = session.frame(Vec2::new(1280.0, 720.0), false);
        assert!(frame.status_lines[0].contains(label));
        let selected = frame
            .controls
            .iter()
            .filter(|control| {
                matches!(
                    control.action,
                    PlatformUiAction::Sandbox(
                        SandboxUiAction::NavigateGalaxy
                            | SandboxUiAction::NavigateSystem
                            | SandboxUiAction::NavigateWorld
                    )
                ) && control.selected
            })
            .collect::<Vec<_>>();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].action, PlatformUiAction::Sandbox(action));
        assert!(selected[0].description.contains(label));
    }
    assert_eq!(session.state().digest().unwrap(), digest);
}

fn drain_steps(session: &mut SandboxSession) {
    for _ in 0..10000 {
        if !session.processing() {
            return;
        }
        session.update(std::time::Duration::ZERO).unwrap();
    }
    panic!("resident step did not finish");
}
