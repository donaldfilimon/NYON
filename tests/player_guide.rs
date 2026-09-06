use glam::Vec2;
use nyon::{
    app::client_runtime::ClientScreen,
    game::{model::*, simulation::Simulation},
    ui::{
        accessibility::SemanticActionId,
        guide::{GuideAction, GuideLocation, build_guide_frame},
        platform::PlatformUiAction,
    },
};

#[test]
fn classic_launch_teaching_matches_energy_based_rules_v1_behavior() {
    let mut location = GuideLocation::for_screen(ClientScreen::ClassicSector);
    let mut classic_pages = Vec::new();
    for _ in 0..64 {
        let guide = build_guide_frame(location, Vec2::new(640.0, 480.0), false, None);
        if !guide.platform.status_lines[0].starts_with("Classic // First fleet") {
            break;
        }
        classic_pages.extend(guide.lines.iter().cloned());
        match guide.platform.action(&SemanticActionId::new("guide.next")) {
            Some(PlatformUiAction::Guide(GuideAction::Show(next))) => location = *next,
            _ => break,
        }
    }
    assert!(!classic_pages.is_empty());
    let normalize = |text: &str| {
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_ascii_lowercase()
    };
    let guide_text = normalize(&classic_pages.join(" "));
    let manual_text = normalize(include_str!("../docs/PLAYER-MANUAL.md"));

    assert!(guide_text.contains(
        "a launch sends half the source's current energy as fleet strength and consumes that energy"
    ));
    assert!(guide_text.contains("source defense stays unchanged"));
    assert!(guide_text.contains("defense is the force protecting a world"));
    assert!(guide_text.contains("friendly arrivals reinforce defense"));
    assert!(guide_text.contains("the fleet snapshots the source hydrosphere for its travel speed"));

    assert!(manual_text.contains(
        "a launch sends half the source world's current energy as fleet strength and consumes that energy, leaving its defense unchanged"
    ));
    assert!(manual_text.contains("defense protects a world"));
    assert!(manual_text.contains("friendly arrivals reinforce its defense"));
    assert!(manual_text.contains(
        "the fleet snapshots the source's hydrosphere level when launched; that level affects its travel speed"
    ));

    for text in [&guide_text, &manual_text] {
        assert!(!text.contains("source's defense drops"));
        assert!(!text.contains("defense is the strength you can send"));
        assert!(!text.contains("defense protects worlds and supplies fleet strength"));
    }

    let rules = RulesV1::default();
    let mut campaign = Campaign::new(DEFAULT_SEED, rules);
    let source = &mut campaign.worlds[0];
    source.owner = Some(Faction::Union);
    source.energy = Energy(80_000);
    source.defense = Strength(7_000);
    source.base_output_per_second = Energy(0);
    source.base_regeneration_per_second = Strength(0);
    source.fields.hydrosphere = 3;
    source.fields.topology = 0;
    let mut simulation = Simulation::from_campaign(campaign, rules);
    simulation
        .enqueue_next(
            CommandSequence(0),
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(3),
            },
        )
        .unwrap();
    simulation.step();

    let launched = simulation
        .state()
        .fleets
        .iter()
        .find(|fleet| fleet.source == WorldId(0) && fleet.destination == WorldId(3))
        .unwrap();
    assert_eq!(launched.strength, Strength(40_000));
    assert_eq!(launched.hydrosphere_level, 3);
    assert_eq!(simulation.state().worlds[0].energy, Energy(40_000));
    assert_eq!(simulation.state().worlds[0].defense, Strength(7_000));
}

#[test]
fn guide_is_paginated_readable_and_keyboard_reachable_at_small_window_sizes() {
    for viewport in [
        Vec2::new(640.0, 480.0),
        Vec2::new(723.0, 802.0),
        Vec2::new(1280.0, 720.0),
    ] {
        let mut location = GuideLocation::for_screen(ClientScreen::MainMenu);
        let mut pages = 0;
        let mut text = String::new();
        loop {
            let guide = build_guide_frame(location, viewport, false, None);
            guide.platform.semantics.validate().unwrap();
            for (index, line) in guide.lines.iter().enumerate() {
                assert!(line.len() <= guide.columns);
                assert!(134.0 + index as f32 * 22.0 + 12.0 < viewport.y - 74.0);
                text.push_str(line);
                text.push(' ');
            }
            for control in &guide.platform.controls {
                assert!(control.bounds.min.cmpge(Vec2::ZERO).all());
                assert!(control.bounds.max.cmple(viewport).all());
                assert!(control.bounds.max.y - control.bounds.min.y >= 44.0);
                assert_eq!(
                    guide
                        .platform
                        .hit_test((control.bounds.min + control.bounds.max) / 2.0),
                    Some(control)
                );
            }
            assert!(
                guide
                    .platform
                    .action(&SemanticActionId::new("guide.close"))
                    .is_some()
            );
            pages += 1;
            assert!(pages < 100);
            match guide.platform.action(&SemanticActionId::new("guide.next")) {
                Some(PlatformUiAction::Guide(GuideAction::Show(next))) => location = *next,
                _ => break,
            }
        }
        for required in [
            "five of seven",
            "solar-array",
            "4 energy",
            "3 ore",
            "Cmd/Ctrl+S",
            "No native gamepad",
            "ray tracing",
        ] {
            assert!(text.contains(required), "missing {required}");
        }
    }
}

#[test]
fn guide_opens_on_the_active_modes_first_success() {
    for (screen, heading) in [
        (ClientScreen::ClassicSector, "Classic"),
        (ClientScreen::GalaxyWorkshop, "Workshop"),
    ] {
        let guide = build_guide_frame(
            GuideLocation::for_screen(screen),
            Vec2::new(1280.0, 720.0),
            true,
            None,
        );
        assert!(guide.platform.status_lines[0].contains(heading));
        assert!(guide.platform.high_contrast);
    }
}

#[test]
fn classic_start_marker_points_to_a_selectable_union_world_and_vanishes_after_selection() {
    use nyon::{
        app::AppCore,
        game::model::Faction,
        scenario::{ScenarioDraft, store::MemoryScenarioStore},
        ui::start_marker::start_marker,
    };
    for viewport in [
        Vec2::new(640.0, 480.0),
        Vec2::new(723.0, 802.0),
        Vec2::new(1280.0, 720.0),
    ] {
        let mut core = AppCore::new(
            ScenarioDraft::factory_default().validated().unwrap(),
            MemoryScenarioStore::default(),
        );
        core.set_viewport(viewport);
        let marker = start_marker(
            core.onboarding_step(),
            core.simulation().state(),
            &core.scene_frame(),
            &core.ui_frame(),
        )
        .unwrap_or_else(|| panic!("first-run start cue at {viewport:?}"));
        assert_eq!(
            core.simulation().state().worlds[usize::from(marker.world.0)].owner,
            Some(Faction::Union)
        );
        assert_eq!(
            core.pick_scene_world(marker.point, false),
            Some(marker.world)
        );
        let button = core
            .ui_frame()
            .hit_test(marker.label_min + Vec2::new(90.0, 27.0));
        assert_eq!(
            button,
            Some(nyon::presentation::ui::UiAction::SelectGuidanceWorld(
                marker.world
            ))
        );
        assert!(core.ui_frame().onboarding_lines[0].contains("TUTORIAL"));
        assert!(core.handle_ui_action(button.unwrap()));
        assert!(
            start_marker(
                core.onboarding_step(),
                core.simulation().state(),
                &core.scene_frame(),
                &core.ui_frame()
            )
            .is_none()
        );
    }
}

#[test]
fn skip_starts_the_same_classic_match_without_replacing_progress() {
    use nyon::{
        app::{AppCore, GameSpeed},
        scenario::{ScenarioDraft, store::MemoryScenarioStore},
        ui::start_marker::start_marker,
    };
    let mut core = AppCore::new(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
    );
    let before = core.simulation().canonical_fingerprint();
    core.skip_onboarding();
    assert_eq!(core.simulation().canonical_fingerprint(), before);
    assert_eq!(core.game_speed(), GameSpeed::Normal);
    assert!(
        start_marker(
            core.onboarding_step(),
            core.simulation().state(),
            &core.scene_frame(),
            &core.ui_frame()
        )
        .is_none()
    );
    core.advance(std::time::Duration::from_millis(100));
    assert!(core.simulation().state().next_tick.0 > 0);
}

#[test]
fn workshop_manual_defaults_produce_visible_energy_then_alloy() {
    use nyon::{
        ui::{
            platform::build_workshop_platform_frame,
            workshop::{CreatorTool, WorkshopUiContext, WorkshopUiModel, default_creator_batch},
        },
        workshop::{
            WorkshopHistory, decode_catalog_pack,
            session::{WorkshopAction, WorkshopSession},
            store::MemoryWorkshopStore,
        },
    };
    use std::time::Duration;
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    let mut session = WorkshopSession::new(WorkshopHistory::from_seed_u64(catalog, 937));
    let mut store = MemoryWorkshopStore::default();
    for tool in [
        CreatorTool::CreateSystem,
        CreatorTool::CreateStar,
        CreatorTool::CreateWorld,
        CreatorTool::PlaceIndustry,
    ] {
        let batch = default_creator_batch(session.snapshot(), None, tool).unwrap();
        session.enqueue(WorkshopAction::Submit(batch)).unwrap();
        assert_eq!(
            session.update(Duration::ZERO, &mut store).accepted_batches,
            1
        );
    }
    session.enqueue(WorkshopAction::StepOnce).unwrap();
    assert_eq!(
        session.update(Duration::ZERO, &mut store).authority_steps,
        1
    );
    let world = *session.snapshot().state.worlds.keys().next().unwrap();
    let model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            selected_entity: Some(world),
            ..Default::default()
        },
    );
    let frame = build_workshop_platform_frame(&model, Vec2::new(1280.0, 720.0), None);
    assert!(
        frame
            .status_lines
            .iter()
            .any(|line| line == "Inventory: 4 energy")
    );
    assert!(
        frame.hit_test(Vec2::new(30.0, 112.0)).is_none(),
        "inventory must not be painted under the tool palette"
    );
    for tool in [
        CreatorTool::CreateDeposit,
        CreatorTool::PlaceIndustry,
        CreatorTool::PlaceIndustry,
    ] {
        let batch = default_creator_batch(session.snapshot(), Some(world), tool).unwrap();
        session.enqueue(WorkshopAction::Submit(batch)).unwrap();
        assert_eq!(
            session.update(Duration::ZERO, &mut store).accepted_batches,
            1
        );
    }
    for _ in 0..2 {
        session.enqueue(WorkshopAction::StepOnce).unwrap();
        session.update(Duration::ZERO, &mut store);
    }
    assert!(
        session.snapshot().state.worlds[&world]
            .inventory
            .iter()
            .any(|(resource, units)| resource.as_str() == "alloy" && *units >= 1)
    );
}
