use glam::Vec2;
use nyon::{
    living_content::LivingContent,
    living_navigation::{
        LivingEntityKind as K, LivingEntitySelection, entity_details, entity_id, navigator_rows,
        visible_navigator_rows,
    },
    sandbox_session::{SandboxSession, SandboxUiAction},
    ui::platform::PlatformUiAction,
};
use std::collections::BTreeSet;

#[test]
fn every_stable_record_is_reachable_with_full_identity_and_read_only_facts() {
    for content in [
        LivingContent::ValeConfluence,
        LivingContent::Commerce,
        LivingContent::InterruptedCorridor,
        LivingContent::FirstExpansion,
    ] {
        let authority = content.open().unwrap();
        let state = authority.state();
        let digest = state.digest().unwrap();
        let rows = navigator_rows(state);
        let expected = state.systems.as_slice().len()
            + state.stars.as_slice().len()
            + state.worlds.as_slice().len()
            + state.colonies.as_slice().len()
            + state.civilizations.as_slice().len()
            + state.lanes.as_slice().len()
            + state.deposits.as_slice().len()
            + state.facilities.as_slice().len()
            + state.construction_jobs.as_slice().len()
            + state.hull_jobs.as_slice().len()
            + state.routes.as_slice().len()
            + state.shipments.as_slice().len()
            + state.hazards.as_slice().len()
            + state.fleets.as_slice().len()
            + state
                .fleets
                .as_slice()
                .iter()
                .map(|fleet| fleet.hulls.as_slice().len())
                .sum::<usize>();
        assert_eq!(rows.len(), expected);
        assert_eq!(
            rows.iter()
                .map(|row| row.selection)
                .collect::<BTreeSet<_>>()
                .len(),
            expected
        );
        for row in &rows {
            let facts = entity_details(state, authority.catalog(), row.selection).unwrap();
            assert!(
                facts
                    .iter()
                    .any(|line| line.contains(&entity_id(row.selection.id)))
            );
            assert!(
                row.parent
                    .is_none_or(|parent| rows.iter().any(|row| row.selection == parent))
            );
        }
        assert_eq!(state.digest().unwrap(), digest);
        let world = state.worlds.as_slice()[0].id;
        assert!(
            rows.iter()
                .any(|row| row.selection == LivingEntitySelection::new(K::World, world))
        );
        if state
            .colonies
            .as_slice()
            .iter()
            .any(|colony| colony.world == world)
        {
            assert!(
                rows.iter()
                    .any(|row| row.selection == LivingEntitySelection::new(K::Colony, world))
            );
        }
    }
}

#[test]
fn navigator_search_filter_and_reveal_materialize_deep_rows_without_commands() {
    let mut session = SandboxSession::open().unwrap();
    let rows = navigator_rows(session.state());
    let target = rows
        .iter()
        .find(|row| row.selection.kind == K::Hull)
        .unwrap()
        .selection;
    let digest = session.state().digest().unwrap();
    let empty = BTreeSet::new();
    assert!(
        !visible_navigator_rows(session.state(), "", None, &empty)
            .iter()
            .any(|row| row.selection == target)
    );
    let filtered = visible_navigator_rows(
        session.state(),
        &entity_id(target.id),
        Some(K::Hull),
        &empty,
    );
    assert!(filtered.iter().any(|row| row.selection == target));
    assert!(
        filtered
            .iter()
            .all(|row| row.selection == target || row.has_children)
    );
    session.select_entity(target).unwrap();
    session
        .apply_action(SandboxUiAction::RevealSelection)
        .unwrap();
    let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), 0.85, false);
    frame.semantics.validate().unwrap();
    let control = frame
        .controls
        .iter()
        .find(|control| {
            control.action == PlatformUiAction::Sandbox(SandboxUiAction::SelectEntity(target))
        })
        .unwrap();
    assert!(control.selected && control.focused);
    assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
    assert_eq!(
        frame.hit_test(control.bounds.center()).unwrap().action,
        control.action
    );
    let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
    assert!(node.hierarchy_level.unwrap() > 1);
    assert!(node.expanded.is_none());
    session
        .apply_action(SandboxUiAction::SelectEntity(target))
        .unwrap();
    let inspector = session.frame(Vec2::new(800.0, 480.0), false);
    assert!(inspector.modal.is_some());
    assert!(
        inspector
            .sighted_text
            .iter()
            .any(|text| text.lines.iter().any(|line| line.contains("Hull /")))
    );
    assert_eq!(session.state().digest().unwrap(), digest);
    assert!(session.receipts().is_empty());
}

#[test]
fn facility_inspection_reports_simultaneous_current_causes_and_actual_last_attempt() {
    let authority = LivingContent::ValeConfluence.open().unwrap();
    let mut state = authority.state().clone();
    let facility = state
        .facilities
        .as_slice()
        .iter()
        .find(|row| row.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    let mut worlds = state.worlds.as_slice().to_vec();
    let world = worlds
        .iter_mut()
        .find(|row| row.id == facility.world)
        .unwrap();
    world.inventory.energy = 0;
    world.inventory.ore = 0;
    world.inventory.alloy = 10000;
    state.worlds = nyon_workshop_core::living::LivingSortedVecV2::new(worlds).unwrap();
    let mut facilities = state.facilities.as_slice().to_vec();
    facilities
        .iter_mut()
        .find(|row| row.id == facility.id)
        .unwrap()
        .disabled = true;
    state.facilities = nyon_workshop_core::living::LivingSortedVecV2::new(facilities).unwrap();
    let digest = state.digest().unwrap();
    let facts = entity_details(
        &state,
        authority.catalog(),
        LivingEntitySelection::new(K::Facility, facility.id),
    )
    .unwrap()
    .join("\n");
    for cause in [
        "Disabled",
        "Waiting for input: energy",
        "Waiting for input: ore",
        "Output full: alloy",
        "Last due attempt:",
        "capacity 10000",
        "Input 4",
        "Output 2",
    ] {
        assert!(facts.contains(cause), "missing {cause}: {facts}");
    }
    assert_eq!(state.digest().unwrap(), digest);
}

#[test]
fn navigator_and_inspector_modal_controls_stay_visible_at_every_supported_scale() {
    for scale in [0.85, 1.0, 1.15, 1.3] {
        for action in [
            SandboxUiAction::NavigatorPicker,
            SandboxUiAction::InspectorPicker,
            SandboxUiAction::CameraPicker,
        ] {
            let mut session = SandboxSession::open().unwrap();
            session.apply_action(action).unwrap();
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            let modal = frame.modal.unwrap();
            frame.semantics.validate().unwrap();
            let controls = frame
                .controls
                .iter()
                .filter(|control| control.enabled)
                .collect::<Vec<_>>();
            for control in &controls {
                assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
                assert!(modal.contains_rect(control.bounds));
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    control.action_id
                );
            }
            for (index, a) in controls.iter().enumerate() {
                for b in controls.iter().skip(index + 1) {
                    assert!(
                        (a.bounds.max.min(b.bounds.max) - a.bounds.min.max(b.bounds.min))
                            .min_element()
                            <= 0.0,
                        "{} / {}",
                        a.label,
                        b.label
                    );
                }
            }
        }
    }
}

#[test]
fn all_navigator_pages_and_type_choices_remain_reachable_and_reveal_focus_is_once() {
    let mut session = SandboxSession::from_authority(LivingContent::Seeded(7).open().unwrap());
    session
        .apply_action(SandboxUiAction::NavigatorPicker)
        .unwrap();
    session
        .edit_creator_field("navigator_query", "World")
        .unwrap();
    let before = session
        .creator_field_value("navigator_query")
        .unwrap()
        .to_owned();
    assert!(
        session
            .edit_creator_field("navigator_query", "World\né")
            .is_err()
    );
    assert_eq!(
        session.creator_field_value("navigator_query"),
        Some(before.as_str())
    );
    session.edit_creator_field("navigator_query", "").unwrap();
    session
        .apply_action(SandboxUiAction::NavigatorTypes)
        .unwrap();
    let mut kinds = BTreeSet::new();
    for _ in 0..32 {
        let frame = session.frame(Vec2::new(800.0, 480.0), false);
        for control in &frame.controls {
            if let PlatformUiAction::Sandbox(SandboxUiAction::NavigatorFilter(kind)) =
                control.action
            {
                kinds.insert(kind);
            }
        }
        let next = frame
            .controls
            .iter()
            .find(|control| control.action_id.as_str() == "sandbox.entity.next")
            .unwrap();
        if !next.enabled {
            break;
        }
        session
            .apply_action(SandboxUiAction::InspectorPage(false))
            .unwrap();
    }
    assert_eq!(kinds.len(), K::ALL.len() + 1);
    session
        .apply_action(SandboxUiAction::NavigatorFilter(None))
        .unwrap();
    session
        .edit_creator_field("navigator_query", "World")
        .unwrap();
    let mut found = BTreeSet::new();
    for _ in 0..128 {
        let frame = session.frame(Vec2::new(800.0, 480.0), false);
        for control in &frame.controls {
            if let PlatformUiAction::Sandbox(SandboxUiAction::SelectEntity(selection)) =
                control.action
            {
                found.insert(selection);
            }
        }
        let next = frame
            .controls
            .iter()
            .find(|control| control.action_id.as_str() == "sandbox.entity.next")
            .unwrap();
        if !next.enabled {
            break;
        }
        session
            .apply_action(SandboxUiAction::InspectorPage(false))
            .unwrap();
    }
    for world in session.state().worlds.as_slice() {
        assert!(found.contains(&LivingEntitySelection::new(K::World, world.id)));
    }
    let selection = LivingEntitySelection::new(
        K::World,
        session.state().worlds.as_slice().last().unwrap().id,
    );
    session.select_entity(selection).unwrap();
    session
        .apply_action(SandboxUiAction::RevealSelection)
        .unwrap();
    let request = session.take_requested_focus().unwrap();
    assert!(session.take_requested_focus().is_none());
    let frame = session.frame(Vec2::new(800.0, 480.0), false);
    assert!(
        frame
            .controls
            .iter()
            .any(|control| control.action_id == request && control.selected)
    );
}
