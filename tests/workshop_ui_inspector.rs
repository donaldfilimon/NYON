mod common;
use common::*;

use glam::Vec2;
use nyon::engine::backend::BackendKind;
use nyon::ui::AtlasMetrics;
use nyon::ui::platform::build_workshop_platform_frame_for_view;
use nyon::ui::platform_sdf::build_platform_ui_batch;
use nyon::ui::workshop::WorkshopUiContext;
use nyon::ui::workshop::WorkshopUiModel;
use nyon::ui::workshop_layout::WorkshopLayout;
use nyon::ui::workshop_layout::WorkshopLayoutMode;
use nyon::ui::workshop_view::WorkshopViewAction;
use nyon::ui::workshop_view::WorkshopViewState;
use nyon::workshop::CatalogHash;
use nyon::workshop::store::SaveGeneration;
use nyon_workshop_core::model::DepositV1;
use nyon_workshop_core::model::IndustryV1;
use std::collections::BTreeSet;

#[test]
fn required_summary_sources_materialize_in_medium_and_compact_status_drawers() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    for viewport in [Vec2::new(900.0, 720.0), Vec2::new(640.0, 720.0)] {
        let layout = WorkshopLayout::resolve(viewport, 1.0).unwrap();
        let mut view = WorkshopViewState::default();
        view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
        view.apply(WorkshopViewAction::ShowStatus, &model, &layout);
        let frame = build_workshop_platform_frame_for_view(&model, layout, &view, None);
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        for id in [
            "save.status",
            "diagnostics.backend",
            "diagnostics.catalog",
            "diagnostics.digest",
            "diagnostics.tick",
        ] {
            assert!(frame.semantics.node(id).is_some_and(|node| node.visible));
            assert!(
                frame
                    .visible_nodes
                    .iter()
                    .any(|record| record.semantic_id.as_str() == id)
            );
            assert!(
                output
                    .text_runs
                    .iter()
                    .any(|run| run.semantic_id.as_str() == id)
            );
        }
    }
}

#[test]
fn status_pages_preserve_full_wrapped_sources_and_independent_inspector_cursor() {
    let mut current = snapshot();
    current.store.status = "Recovered previous valid generation".to_owned();
    current.store.generation = Some(SaveGeneration(u64::MAX));
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        for high_contrast in [false, true] {
            let model = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    high_contrast,
                    backend: Some(BackendKind::WebGl2Low),
                    catalog_hash: Some(CatalogHash([0xAB; 32])),
                    ..WorkshopUiContext::default()
                },
            );
            let initial = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &WorkshopViewState::default(),
                None,
            );
            if layout.mode == WorkshopLayoutMode::Wide {
                let output =
                    build_platform_ui_batch(&initial, &AtlasMetrics::embedded().unwrap()).unwrap();
                if !output
                    .text_runs
                    .iter()
                    .any(|run| run.semantic_id.as_str() == "diagnostics.tick")
                {
                    assert!(
                        initial.controls.iter().any(|control| control.action_id
                            == WorkshopViewAction::ShowStatus.action_id()),
                        "overflow lacks visible Status route at {width}x{height}@{scale}"
                    );
                }
            }
            for id in [
                "save.status",
                "diagnostics.backend",
                "diagnostics.catalog",
                "diagnostics.digest",
                "diagnostics.tick",
            ] {
                let source = model.semantics.node(id).unwrap();
                let mut expected = source.name.clone();
                if let Some(value) = source.value.as_deref().filter(|value| !value.is_empty()) {
                    expected.push_str(": ");
                    expected.push_str(value);
                }
                if !source.description.is_empty()
                    && source.description != source.value.as_deref().unwrap_or_default()
                {
                    expected.push_str(" - ");
                    expected.push_str(&source.description);
                }
                let mut view = WorkshopViewState::default();
                view.inspector_start = 2;
                assert!(view.reveal_semantic(&model, &layout, &source.id));
                let mut rendered = String::new();
                for _ in 0..30 {
                    let frame =
                        build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                    qualify_sdf_frame(&model, &frame);
                    let output =
                        build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap())
                            .unwrap();
                    let Some(run) = output
                        .text_runs
                        .iter()
                        .find(|run| run.semantic_id == source.id)
                    else {
                        break;
                    };
                    rendered.push_str(&run.exact_text);
                    let next = frame
                        .controls
                        .iter()
                        .find(|control| {
                            control.action_id == WorkshopViewAction::ScrollNext.action_id()
                        })
                        .unwrap();
                    assert_eq!(
                        frame.hit_test(next.bounds.center()).unwrap().action,
                        next.action
                    );
                    view.apply(WorkshopViewAction::ScrollNext, &model, &layout);
                    assert_eq!(view.inspector_start, 2);
                    if rendered == expected {
                        break;
                    }
                }
                assert_eq!(
                    rendered, expected,
                    "missing full status {id} at {width}x{height}@{scale}"
                );
            }
        }
    }
}

#[test]
fn outliner_and_inspector_expose_the_complete_workshop_without_canvas_input() {
    let catalog = core_catalog();
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );

    assert_eq!(model.outliner.len(), 13);
    let world = model
        .outliner
        .iter()
        .find(|entry| entry.entity == entity(4))
        .unwrap();
    assert_eq!(world.parent, Some(entity(2)));
    assert_eq!(world.depth, 1);
    let deposit = model
        .outliner
        .iter()
        .find(|entry| entry.entity == entity(5))
        .unwrap();
    assert_eq!(deposit.parent, Some(entity(4)));
    assert_eq!(deposit.depth, 2);

    assert_eq!(model.inspector.title, "Hammerfall");
    assert_eq!(
        model
            .inspector
            .sections
            .iter()
            .map(|section| section.heading)
            .collect::<Vec<_>>(),
        vec![
            "Status",
            "Inventory",
            "Deposits",
            "Industries",
            "Routes",
            "Shipments",
            "Hazards",
        ]
    );
    assert_eq!(model.inspector.sections[1].rows.len(), 2);
    assert_eq!(model.inspector.sections[2].rows.len(), 1);
    assert_eq!(model.inspector.sections[3].rows.len(), 1);
    assert_eq!(model.inspector.sections[4].rows.len(), 1);
    assert_eq!(model.inspector.sections[5].rows.len(), 1);
    assert!(model.semantics.validate().is_ok());
}

#[test]
fn inspector_exposes_complete_catalog_backed_facts_for_overview_and_every_entity_kind() {
    let current = snapshot();
    let canonical_state = current.state.canonical_bytes();
    let state_digest = current.state_digest;
    let catalog = core_catalog();
    let cases = [
        (
            None,
            &[
                "Factions",
                "Systems",
                "Stars",
                "Worlds",
                "Lanes",
                "Deposits",
                "Industries",
                "Routes",
                "Shipments",
                "Hazards",
                "Catalog",
                "Catalog ID",
                "Catalog version",
            ][..],
        ),
        (
            Some(entity(1)),
            &["Entity ID", "Kind", "Name", "Color", "Owned objects"],
        ),
        (
            Some(entity(2)),
            &[
                "Entity ID",
                "Kind",
                "Name",
                "Position",
                "Stars",
                "Worlds",
                "Connected lanes",
            ],
        ),
        (
            Some(entity(3)),
            &[
                "Entity ID",
                "Kind",
                "Name",
                "System",
                "Archetype",
                "Archetype ID",
                "Catalog detail",
            ],
        ),
        (
            Some(entity(4)),
            &[
                "Entity ID",
                "Kind",
                "Name",
                "System",
                "Primary",
                "Archetype",
                "Archetype ID",
                "Catalog detail",
                "Orbit radius",
                "Orbit period",
                "Orbit phase",
                "Owner",
            ],
        ),
        (
            Some(entity(7)),
            &["Entity ID", "Kind", "Endpoints", "Distance", "Hazards"],
        ),
        (
            Some(entity(5)),
            &[
                "Entity ID",
                "Kind",
                "World",
                "Resource",
                "Resource ID",
                "Catalog detail",
                "Reserve",
            ],
        ),
        (
            Some(entity(6)),
            &[
                "Entity ID",
                "Kind",
                "World",
                "Definition",
                "Definition ID",
                "Catalog detail",
                "Industry type",
                "Duration",
                "Inputs",
                "Outputs",
                "Linked deposit",
                "Operation",
            ],
        ),
        (
            Some(entity(9)),
            &[
                "Entity ID",
                "Kind",
                "Source",
                "Destination",
                "Resource",
                "Resource ID",
                "Catalog detail",
                "Batch capacity",
                "Effective capacity",
                "Active hazards",
                "In-flight shipments",
            ],
        ),
        (
            Some(entity(11)),
            &[
                "Entity ID",
                "Kind",
                "Route",
                "Source",
                "Destination",
                "Resource",
                "Resource ID",
                "Units",
                "Dispatched",
                "Arrival",
                "Ticks remaining",
            ],
        ),
        (
            Some(entity(12)),
            &[
                "Entity ID",
                "Kind",
                "Lane",
                "Hazard",
                "Hazard ID",
                "Catalog detail",
                "Start",
                "Duration",
                "End",
                "Status",
                "Route capacity",
            ],
        ),
    ];

    for (selected_entity, expected_labels) in cases {
        let model = WorkshopUiModel::build(
            &current,
            WorkshopUiContext {
                catalog: Some(&catalog),
                selected_entity,
                ..WorkshopUiContext::default()
            },
        );
        let labels = model
            .inspector
            .sections
            .iter()
            .flat_map(|section| section.rows.iter().map(|row| row.label.as_str()))
            .collect::<BTreeSet<_>>();
        for expected in expected_labels {
            assert!(
                labels.contains(expected),
                "selection {selected_entity:?} omitted {expected}: {labels:?}"
            );
        }
    }
    assert_eq!(current.state.canonical_bytes(), canonical_state);
    assert_eq!(current.state.digest(), state_digest);
}

#[test]
fn inspector_visible_records_and_semantic_rows_share_stable_ids_and_values() {
    let catalog = core_catalog();
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(6)),
            ..WorkshopUiContext::default()
        },
    );
    assert_eq!(
        model.diagnostics.catalog_hash,
        catalog
            .catalog_hash()
            .0
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let layout = WorkshopLayout::resolve(Vec2::new(1440.0, 900.0), 1.0).unwrap();
    let frame =
        build_workshop_platform_frame_for_view(&model, layout, &WorkshopViewState::default(), None);

    assert!(!frame.sighted_text.is_empty());
    for visible in &frame.sighted_text {
        let semantic = frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.id == visible.semantic_id)
            .unwrap_or_else(|| panic!("missing semantic row {}", visible.semantic_id));
        assert_eq!(visible.label, semantic.name);
        assert_eq!(visible.value.as_deref().unwrap_or(""), semantic.description);
        let expected_display = visible.value.as_ref().map_or_else(
            || visible.label.clone(),
            |value| format!("{}: {value}", visible.label),
        );
        assert_eq!(
            visible.lines.concat(),
            expected_display.to_ascii_uppercase()
        );
        assert!(visible.clip.contains_rect(visible.bounds));
    }
    assert!(
        frame
            .sighted_text
            .iter()
            .any(|text| text.value.as_deref() == Some("Extractor"))
    );
    assert!(
        frame.sighted_text.iter().any(
            |text| text.value.as_deref() == Some("Uses energy to extract linked ore reserves.")
        )
    );
}

#[test]
fn inspector_fact_ids_survive_inventory_and_related_entity_insertions() {
    let catalog_pack = core_catalog();
    let mut first = snapshot();
    let first_model = WorkshopUiModel::build(
        &first,
        WorkshopUiContext {
            catalog: Some(&catalog_pack),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );
    let fact = |model: &WorkshopUiModel, section: &str, label: &str| {
        model
            .inspector
            .sections
            .iter()
            .find(|candidate| candidate.heading == section)
            .unwrap()
            .rows
            .iter()
            .find(|candidate| candidate.label == label)
            .unwrap()
            .clone()
    };
    let energy_before = fact(&first_model, "Inventory", "Energy");
    let ore_before = fact(&first_model, "Inventory", "Ore");
    let deposit_before = first_model
        .inspector
        .sections
        .iter()
        .find(|section| section.heading == "Deposits")
        .unwrap()
        .rows[0]
        .clone();

    first
        .state
        .worlds
        .get_mut(&entity(4))
        .unwrap()
        .inventory
        .insert(catalog("alloy"), 1);
    first
        .state
        .worlds
        .get_mut(&entity(4))
        .unwrap()
        .inventory
        .insert(catalog("energy"), 9);
    first.state.deposits.insert(
        entity(0),
        DepositV1 {
            world: entity(4),
            resource_id: catalog("energy"),
            reserve_units: 4,
        },
    );
    first.state_digest = first.state.digest();
    first.active_view.digest = first.state_digest;
    let second_model = WorkshopUiModel::build(
        &first,
        WorkshopUiContext {
            catalog: Some(&catalog_pack),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );
    let energy_after = fact(&second_model, "Inventory", "Energy");
    let ore_after = fact(&second_model, "Inventory", "Ore");
    let deposit_after = second_model
        .inspector
        .sections
        .iter()
        .find(|section| section.heading == "Deposits")
        .unwrap()
        .rows
        .iter()
        .find(|row| row.label == deposit_before.label)
        .unwrap();

    assert_eq!(energy_before.semantic_id, energy_after.semantic_id);
    assert_eq!(energy_after.value, "9");
    assert_eq!(ore_before.semantic_id, ore_after.semantic_id);
    assert_eq!(ore_before.value, ore_after.value);
    assert_eq!(deposit_before.semantic_id, deposit_after.semantic_id);
    assert_ne!(energy_before.value, energy_after.value);
}

#[test]
fn short_height_inspector_keeps_title_fixed_and_every_line_inside_body_clip() {
    let catalog = catalog_with_maximum_industry_description();
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(6)),
            ..WorkshopUiContext::default()
        },
    );
    let detail_id = model
        .inspector
        .sections
        .iter()
        .flat_map(|section| &section.rows)
        .find(|row| row.label == "Catalog detail")
        .unwrap()
        .semantic_id
        .clone();

    for (scale, expected_mode) in [
        (1.0, WorkshopLayoutMode::Wide),
        (1.3, WorkshopLayoutMode::Medium),
    ] {
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), scale).unwrap();
        assert_eq!(layout.mode, expected_mode);
        let mut view = WorkshopViewState::default();
        assert!(view.reveal_semantic(&model, &layout, &detail_id));
        let frame = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);

        let title = frame
            .sighted_text
            .iter()
            .find(|text| text.kind == nyon::ui::workshop::InspectorTextKind::Title)
            .expect("the fixed inspector title must remain visible after reveal");
        assert!(title.label.contains("Extractor"));
        assert!(title.clip.contains_rect(title.bounds));

        for text in &frame.sighted_text {
            assert!(
                text.clip.contains_rect(text.bounds),
                "{} escaped its clip at scale {scale}: {:?} outside {:?}",
                text.semantic_id,
                text.bounds,
                text.clip
            );
            assert!(
                frame
                    .controls
                    .iter()
                    .all(|control| !text.bounds.overlaps(control.bounds)),
                "{} overlapped a control at scale {scale}",
                text.semantic_id
            );
        }
        assert!(
            frame
                .sighted_text
                .iter()
                .any(|text| text.semantic_id == detail_id),
            "revealed maximum description is absent at scale {scale}"
        );

        let last_id = model.inspector.rows().last().unwrap().semantic_id.clone();
        assert!(view.reveal_semantic(&model, &layout, &last_id));
        let last = build_workshop_platform_frame_for_view(&model, layout, &view, None);
        assert!(last.sighted_text.iter().any(|text| {
            text.kind == nyon::ui::workshop::InspectorTextKind::Title
                && text.label.contains("Extractor")
        }));
        assert!(
            last.sighted_text
                .iter()
                .any(|text| text.semantic_id == last_id)
        );
    }
}

#[test]
fn inspector_cursor_reconciles_selection_width_and_wrapped_line_identity() {
    let catalog = catalog_with_maximum_industry_description();
    let snapshot = snapshot();
    let industry_model = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(6)),
            ..WorkshopUiContext::default()
        },
    );
    let wide = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), 1.0).unwrap();
    let detail_id = industry_model
        .inspector
        .rows()
        .find(|row| row.label == "Catalog detail")
        .unwrap()
        .semantic_id
        .clone();
    let mut view = WorkshopViewState::default();
    assert!(view.reveal_semantic(&industry_model, &wide, &detail_id));
    view.apply(
        WorkshopViewAction::ScrollInspectorNext,
        &industry_model,
        &wide,
    );
    assert_eq!(
        view.inspector_start,
        industry_model
            .inspector
            .text_record_index(&detail_id)
            .unwrap()
    );
    assert!(view.inspector_line_offset > 0);
    view.apply(
        WorkshopViewAction::ScrollInspectorPrevious,
        &industry_model,
        &wide,
    );
    assert_eq!(view.inspector_line_offset, 0);

    for _ in 0..256 {
        let prior = (view.inspector_start, view.inspector_line_offset);
        view.apply(
            WorkshopViewAction::ScrollInspectorNext,
            &industry_model,
            &wide,
        );
        if view.inspector_start != prior.0 {
            break;
        }
        assert!(view.inspector_line_offset > prior.1);
    }
    assert!(
        view.inspector_start
            > industry_model
                .inspector
                .text_record_index(&detail_id)
                .unwrap(),
        "every wrapped detail line must be traversed before advancing"
    );

    let medium = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), 1.3).unwrap();
    let resized =
        build_workshop_platform_frame_for_view(&industry_model, medium.clone(), &view, None);
    assert!(resized.sighted_text.iter().any(|text| {
        text.semantic_id == industry_model.inspector.text_records()[0].semantic_id
    }));

    let world_model = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );
    let reselected =
        build_workshop_platform_frame_for_view(&world_model, medium.clone(), &view, None);
    assert!(reselected.sighted_text.iter().any(|text| {
        text.semantic_id == world_model.inspector.text_records()[0].semantic_id
            && text.label.contains("Status")
    }));
    let inventory_id = world_model
        .inspector
        .rows()
        .find(|row| row.label == "Energy")
        .unwrap()
        .semantic_id
        .clone();
    assert!(view.reveal_semantic(&world_model, &medium, &inventory_id));
    assert_eq!(view.inspector_line_offset, 0);
    let revealed = build_workshop_platform_frame_for_view(&world_model, medium, &view, None);
    assert!(
        revealed
            .sighted_text
            .iter()
            .any(|text| text.semantic_id == inventory_id)
    );
}

#[test]
fn two_system_forge_inspector_explains_foundry_recipe_and_active_ion_storm_capacity() {
    let mut current = snapshot();
    current.state.industries.insert(
        entity(14),
        IndustryV1 {
            world: entity(10),
            definition_id: catalog("foundry"),
            linked_deposit: None,
            enabled: true,
        },
    );
    current.state_digest = current.state.digest();
    current.active_view.digest = current.state_digest;
    let catalog = core_catalog();

    let foundry = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(14)),
            ..WorkshopUiContext::default()
        },
    );
    assert!(
        foundry
            .inspector
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| { row.label == "Inputs" && row.value == "2 Energy + 3 Ore" })
    );
    assert!(
        foundry
            .inspector
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| { row.label == "Outputs" && row.value == "1 Alloy" })
    );
    assert!(
        foundry
            .inspector
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| {
                row.label == "Operation" && row.value == "Enabled; waiting for 2 Energy + 3 Ore"
            })
    );

    let route = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(9)),
            ..WorkshopUiContext::default()
        },
    );
    assert!(
        route
            .inspector
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| { row.label == "Active hazards" && row.value.contains("Ion Storm") })
    );
    assert!(
        route
            .inspector
            .sections
            .iter()
            .flat_map(|section| &section.rows)
            .any(|row| { row.label == "Effective capacity" && row.value == "1 Ore per dispatch" })
    );
}

#[test]
fn inspector_rows_are_windowed_and_reachable_in_compact_navigator_drawer() {
    let catalog = core_catalog();
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );
    let layout = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), 1.30).unwrap();
    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::ShowInspector, &model, &layout);
    let first = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
    assert!(!first.sighted_text.is_empty());
    assert!(
        first
            .sighted_text
            .iter()
            .all(|text| layout.drawer_sheet.contains_rect(text.bounds))
    );
    assert!(first.sighted_text.iter().all(|text| {
        first
            .controls
            .iter()
            .all(|control| !text.bounds.overlaps(control.bounds))
    }));
    let first_ids = first
        .sighted_text
        .iter()
        .map(|text| text.semantic_id.clone())
        .collect::<BTreeSet<_>>();

    for _ in 0..model.inspector.text_record_count() {
        view.apply(WorkshopViewAction::ScrollNext, &model, &layout);
    }
    let last = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
    let last_ids = last
        .sighted_text
        .iter()
        .map(|text| text.semantic_id.clone())
        .collect::<BTreeSet<_>>();
    assert_ne!(first_ids, last_ids);
    assert!(view.inspector_start > 0);
    assert!(
        last.sighted_text
            .iter()
            .all(|text| text.clip == layout.drawer_sheet)
    );
    let inspector_region = last.semantics.node("workshop.inspector").unwrap();
    assert!(inspector_region.visible);
    assert_eq!(inspector_region.bounds, Some(layout.drawer_sheet.into()));

    for row in model.inspector.rows() {
        if !first
            .sighted_text
            .iter()
            .any(|text| text.semantic_id == row.semantic_id)
        {
            let first_output =
                build_platform_ui_batch(&first, &AtlasMetrics::embedded().unwrap()).unwrap();
            assert!(
                first_output
                    .text_runs
                    .iter()
                    .all(|run| run.semantic_id != row.semantic_id)
            );
            assert!(
                first_output
                    .icon_runs
                    .iter()
                    .all(|run| run.semantic_id != row.semantic_id)
            );
            assert!(
                first
                    .semantics
                    .node(row.semantic_id.as_str())
                    .is_none_or(|node| node.action_id.is_none() && !node.visible)
            );
        }
        let mut revealed = WorkshopViewState::default();
        assert!(revealed.reveal_semantic(&model, &layout, &row.semantic_id));
        let frame = build_workshop_platform_frame_for_view(&model, layout.clone(), &revealed, None);
        assert!(
            frame
                .sighted_text
                .iter()
                .any(|text| text.semantic_id == row.semantic_id)
        );
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == row.semantic_id)
        );
        assert!(
            frame
                .semantics
                .node(row.semantic_id.as_str())
                .is_some_and(|node| node.visible && node.id == row.semantic_id)
        );
    }
}

#[test]
fn docked_inspector_scroll_is_independent_from_an_open_navigator_section() {
    let catalog = core_catalog();
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            catalog: Some(&catalog),
            selected_entity: Some(entity(4)),
            ..WorkshopUiContext::default()
        },
    );
    let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.30).unwrap();
    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
    assert_eq!(
        view.navigator_section,
        nyon::ui::workshop_view::NavigatorSection::Hierarchy
    );

    view.apply(WorkshopViewAction::ScrollInspectorNext, &model, &layout);
    assert_eq!(view.outliner_start, 0);
    assert_eq!(view.inspector_start, 1);

    view.apply(WorkshopViewAction::ScrollNext, &model, &layout);
    assert_eq!(view.outliner_start, 1);
    assert_eq!(view.inspector_start, 1);
}
