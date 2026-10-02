use glam::Vec2;
use nyon::{
    app::client_runtime::ClientScreen,
    ui::{
        accessibility::SemanticActionId,
        guide::{GuideAction, GuideLocation, build_guide_frame},
        platform::PlatformUiAction,
    },
};
#[test]
fn living_guide_three_item_journey_is_readable_and_reachable_on_small_screens() {
    for viewport in [
        Vec2::new(640.0, 480.0),
        Vec2::new(800.0, 480.0),
        Vec2::new(1280.0, 720.0),
    ] {
        let mut location = GuideLocation::for_screen(ClientScreen::GalaxySandbox);
        let mut text = String::new();
        for _ in 0..64 {
            let frame = build_guide_frame(location, viewport, false, None);
            if !frame.platform.status_lines[0].starts_with("Living Galaxy") {
                break;
            }
            frame.platform.semantics.validate().unwrap();
            for line in &frame.lines {
                assert!(line.len() <= frame.columns);
                text.push_str(line);
                text.push(' ');
            }
            assert!(
                frame
                    .platform
                    .action(&SemanticActionId::new("guide.close"))
                    .is_some()
            );
            match frame.platform.action(&SemanticActionId::new("guide.next")) {
                Some(PlatformUiAction::Guide(GuideAction::Show(next))) => location = *next,
                _ => break,
            }
        }
        for term in [
            "1 / SELECTION",
            "2 / TIME",
            "3 / CREATION",
            "Preview",
            "Chronicle",
            "unknown links",
            "Before the intervention",
            "Save & leave",
            "immutable export snapshot",
        ] {
            assert!(text.contains(term), "missing {term}");
        }
        assert!(!text.contains("Previous world / Next world"));
    }
}

#[test]
fn vale_intro_names_the_real_bottleneck_and_solar_draft_without_timed_success() {
    use nyon::{
        living_navigation::LivingEntityKind,
        sandbox_session::{SandboxSession, SandboxUiAction},
    };
    let mut session = SandboxSession::open().unwrap();
    let original = session.state().digest().unwrap();
    session.present_intro().unwrap();
    let lines = session.starter_guidance_lines().join(" ");
    for term in [
        "Aster Vale",
        "0 energy",
        "no Solar Array",
        "Recipe",
        "Input",
        "Output",
    ] {
        assert!(lines.contains(term), "missing actual briefing {term}");
    }
    assert_eq!(
        session.selected_entity().unwrap().kind,
        LivingEntityKind::World
    );
    assert_eq!(session.state().digest().unwrap(), original);
    assert!(!session.starter_guidance_complete());
    session
        .apply_action(SandboxUiAction::IntroStart(false))
        .unwrap();
    for _ in 0..20 {
        session.step_once().unwrap();
    }
    assert!(
        session
            .starter_guidance_lines()
            .join(" ")
            .contains("Authenticated energy blockage")
    );
    assert!(!session.starter_guidance_complete());
    let blocked_state = session.state().digest().unwrap();
    session.apply_action(SandboxUiAction::IntroPicker).unwrap();
    session.apply_action(SandboxUiAction::IntroCreate).unwrap();
    assert_eq!(
        session.creator_field_value("0.definition"),
        Some("solar_array")
    );
    let aster = session
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Aster Vale")
        .unwrap();
    let world = aster
        .id
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        session.creator_field_value("0.world.id"),
        Some(world.as_str())
    );
    session.preview_creator().unwrap();
    assert_eq!(session.state().digest().unwrap(), blocked_state);
    assert!(!session.starter_guidance_complete());
    session
        .apply_action(SandboxUiAction::CreatorCancel)
        .unwrap();
    assert!(!session.starter_guidance_complete());
    session.apply_action(SandboxUiAction::IntroPicker).unwrap();
    for viewport in [Vec2::new(800.0, 480.0), Vec2::new(1280.0, 720.0)] {
        let frame = session.frame(viewport, true);
        frame.semantics.validate().unwrap();
        for control in frame.controls.iter().filter(|control| control.enabled) {
            assert!(control.bounds.max.y - control.bounds.min.y >= 44.0);
        }
    }
}

#[test]
fn actual_solar_intervention_records_foundry_production_after_authenticated_blockage() {
    use nyon::sandbox_session::{SandboxSession, SandboxUiAction};
    use nyon_workshop_core::living::{
        LivingEventKindV2, LivingEventResourceRoleV2, LivingResourceV2,
    };
    let mut session = SandboxSession::open().unwrap();
    session.present_intro().unwrap();
    session
        .apply_action(SandboxUiAction::IntroStart(false))
        .unwrap();
    for _ in 0..20 {
        session.step_once().unwrap();
    }
    assert!(!session.starter_guidance_complete());
    session.apply_action(SandboxUiAction::IntroCreate).unwrap();
    session.preview_creator().unwrap();
    session.apply_creator().unwrap();
    for _ in 0..40 {
        session.step_once().unwrap();
        if session.starter_guidance_complete() {
            break;
        }
    }
    assert!(
        session.starter_guidance_complete(),
        "{}",
        session.starter_guidance_lines().join(" ")
    );
    let world_id = session.selected_world().unwrap();
    let industry = session
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|facility| facility.world == world_id && facility.definition.as_str() == "foundry")
        .unwrap()
        .id;
    let receipt = session.receipts().back().unwrap();
    let facts = session.authority().last_event_facts().unwrap();
    facts.verify_receipt(receipt).unwrap();
    let explained = facts
        .events
        .iter()
        .find(|event| {
            event.facts.entity == Some(industry)
                && receipt.events.iter().any(|ordinary| {
                    ordinary.id == event.event_id
                        && ordinary.kind == LivingEventKindV2::FacilityProduced
                })
                && event.facts.resources.iter().any(|fact| {
                    fact.role == LivingEventResourceRoleV2::Input
                        && fact.resource == LivingResourceV2::Energy
                })
                && event.facts.resources.iter().any(|fact| {
                    fact.role == LivingEventResourceRoleV2::Output
                        && fact.resource == LivingResourceV2::Alloy
                })
        })
        .unwrap();
    let inventory = session
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Aster Vale")
        .unwrap()
        .inventory;
    let lines = session.starter_guidance_lines().join(" ");
    assert!(lines.contains("Guidance production witness recorded"));
    assert!(lines.contains(&format!("alloy {}", inventory.alloy)));
    assert!(
        lines.contains(
            &explained
                .event_id
                .0
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<String>()
        )
    );
}
