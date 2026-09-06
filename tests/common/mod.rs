//! Shared fixtures and assertions for the workshop UI integration suites.
#![allow(dead_code)]

use glam::Vec2;
use nyon::engine::primitives::PrimitiveBatch;
use nyon::ui::AtlasMetrics;
use nyon::ui::UiIcon;
use nyon::ui::platform::PlatformUiAction;
use nyon::ui::platform::WorkshopMarkerLayer;
use nyon::ui::platform::WorkshopMarkerWitness;
use nyon::ui::platform_sdf::PlatformPanelRole;
use nyon::ui::platform_sdf::PlatformTextOverflow;
use nyon::ui::platform_sdf::PlatformTextRole;
use nyon::ui::platform_sdf::build_platform_ui_batch;
use nyon::ui::workshop::WorkshopUiModel;
use nyon::ui::workshop::removal_blockers;
use nyon::ui::workshop_layout::WorkshopLayoutMode;
use nyon::ui::workshop_view::WorkshopViewAction;
use nyon::workshop::ActiveView;
use nyon::workshop::BatchLocalId;
use nyon::workshop::BranchId;
use nyon::workshop::BranchRef;
use nyon::workshop::CatalogId;
use nyon::workshop::CreatorOpV1;
use nyon::workshop::EntityId;
use nyon::workshop::GalaxyPointV1;
use nyon::workshop::ObjectName;
use nyon::workshop::ObjectRefV1;
use nyon::workshop::RevisionId;
use nyon::workshop::WorkshopStateV1;
use nyon::workshop::WorkshopTick;
use nyon::workshop::session::WorkshopSessionSnapshot;
use nyon::workshop::session::WorkshopSpeed;
use nyon::workshop::session::WorkshopStoreSnapshot;
use nyon::workshop::store::SaveGeneration;
use nyon::workshop::store::SlotId;
use nyon_workshop_core::model::DepositV1;
use nyon_workshop_core::model::FactionV1;
use nyon_workshop_core::model::HazardV1;
use nyon_workshop_core::model::IndustryV1;
use nyon_workshop_core::model::LaneV1;
use nyon_workshop_core::model::RouteV1;
use nyon_workshop_core::model::ShipmentV1;
use nyon_workshop_core::model::StarV1;
use nyon_workshop_core::model::SystemV1;
use nyon_workshop_core::model::WorldV1;
use std::collections::BTreeMap;

pub const SDF_QUALIFICATION_MATRIX: [(f32, f32, f32); 11] = [
    (723.0, 802.0, 1.0),
    (723.0, 802.0, 1.15),
    (723.0, 802.0, 1.3),
    (1280.0, 480.0, 1.0),
    (1280.0, 480.0, 1.3),
    (1280.0, 720.0, 1.0),
    (1280.0, 720.0, 1.3),
    (1440.0, 900.0, 1.0),
    (1440.0, 900.0, 1.3),
    (1920.0, 1080.0, 1.0),
    (1920.0, 1080.0, 1.3),
];

pub fn core_catalog() -> nyon_workshop_core::ValidatedCatalogPackV1 {
    nyon_workshop_core::decode_catalog_pack(include_bytes!(
        "../../assets/workshop/core-pack-v1.json"
    ))
    .unwrap()
}

pub fn catalog_with_maximum_industry_description() -> nyon_workshop_core::ValidatedCatalogPackV1 {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../assets/workshop/core-pack-v1.json")).unwrap();
    let definition = value["industry_definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "extractor")
        .unwrap();
    definition["description"] = serde_json::json!(format!("FIRST{}LAST", "D".repeat(503)));
    nyon_workshop_core::decode_catalog_pack(&serde_json::to_vec(&value).unwrap()).unwrap()
}

pub fn entity(value: u8) -> EntityId {
    EntityId([value; 16])
}

pub fn revision(value: u8) -> RevisionId {
    RevisionId([value; 32])
}

pub fn branch(value: u8) -> BranchId {
    BranchId([value; 16])
}

pub fn catalog(value: &str) -> CatalogId {
    CatalogId::new(value).unwrap()
}

pub fn name(value: &str) -> ObjectName {
    ObjectName::new(value).unwrap()
}

pub fn populated_state() -> WorkshopStateV1 {
    let faction = entity(1);
    let system_a = entity(2);
    let star_a = entity(3);
    let world_a = entity(4);
    let deposit = entity(5);
    let industry = entity(6);
    let lane = entity(7);
    let system_b = entity(8);
    let route = entity(9);
    let world_b = entity(10);
    let shipment = entity(11);
    let hazard = entity(12);
    let star_b = entity(13);

    let mut state = WorkshopStateV1 {
        tick: WorkshopTick(15),
        ..WorkshopStateV1::default()
    };
    state.factions.insert(
        faction,
        FactionV1 {
            name: name("Forge Guild"),
            color_rgb: [20, 120, 240],
        },
    );
    state.systems.insert(
        system_a,
        SystemV1 {
            name: name("Anvil"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        },
    );
    state.systems.insert(
        system_b,
        SystemV1 {
            name: name("Crucible"),
            position: GalaxyPointV1::new(1_024, 0).unwrap(),
        },
    );
    state.stars.insert(
        star_a,
        StarV1 {
            system: system_a,
            name: name("Anvil Prime"),
            archetype_id: catalog("yellow-dwarf"),
        },
    );
    state.stars.insert(
        star_b,
        StarV1 {
            system: system_b,
            name: name("Crucible Prime"),
            archetype_id: catalog("yellow-dwarf"),
        },
    );
    state.worlds.insert(
        world_a,
        WorldV1 {
            system: system_a,
            primary: star_a,
            name: name("Hammerfall"),
            archetype_id: catalog("rocky-world"),
            orbit_radius_milli_au: 1_000,
            orbit_period_ticks: 100,
            phase_millidegrees: 0,
            inventory: BTreeMap::from([(catalog("energy"), 8), (catalog("ore"), 3)]),
        },
    );
    state.worlds.insert(
        world_b,
        WorldV1 {
            system: system_b,
            primary: star_b,
            name: name("Bellows"),
            archetype_id: catalog("rocky-world"),
            orbit_radius_milli_au: 1_500,
            orbit_period_ticks: 150,
            phase_millidegrees: 180_000,
            inventory: BTreeMap::new(),
        },
    );
    state.deposits.insert(
        deposit,
        DepositV1 {
            world: world_a,
            resource_id: catalog("ore"),
            reserve_units: 80,
        },
    );
    state.industries.insert(
        industry,
        IndustryV1 {
            world: world_a,
            definition_id: catalog("extractor"),
            linked_deposit: Some(deposit),
            enabled: true,
        },
    );
    state.lanes.insert(
        lane,
        LaneV1 {
            a: system_a,
            b: system_b,
            distance_units: 1_024,
        },
    );
    state.routes.insert(
        route,
        RouteV1 {
            source: world_a,
            destination: world_b,
            resource_id: catalog("ore"),
            batch_units: 2,
        },
    );
    state.shipments.insert(
        shipment,
        ShipmentV1 {
            route,
            source: world_a,
            destination: world_b,
            resource_id: catalog("ore"),
            units: 2,
            dispatched_tick: WorkshopTick(14),
            arrival_tick: WorkshopTick(18),
        },
    );
    state.hazards.insert(
        hazard,
        HazardV1 {
            lane,
            hazard_id: catalog("ion-storm"),
            start_tick: WorkshopTick(14),
            duration_ticks: 10,
            cancelled: false,
        },
    );
    state.owners.insert(world_a, faction);
    state
}

pub fn snapshot() -> WorkshopSessionSnapshot {
    let state = populated_state();
    let main = branch(1);
    WorkshopSessionSnapshot {
        state_digest: state.digest(),
        active_view: ActiveView {
            selected_branch: main,
            view_cursor: Some(revision(2)),
            tick: state.tick,
            digest: state.digest(),
        },
        state,
        branches: vec![
            BranchRef {
                id: main,
                name: "Main".to_owned(),
                head: Some(revision(2)),
                last_tick: WorkshopTick(15),
            },
            BranchRef {
                id: branch(3),
                name: "Branch 1".to_owned(),
                head: Some(revision(3)),
                last_tick: WorkshopTick(12),
            },
        ],
        revision_count: 2,
        speed: WorkshopSpeed::Paused,
        diagnostics: Vec::new(),
        store: WorkshopStoreSnapshot {
            slot: Some(SlotId(4)),
            generation: Some(SaveGeneration(7)),
            head_generation: Some(SaveGeneration(7)),
            commit_pending: false,
            load_pending: false,
            dirty: true,
            status: "Saved".to_owned(),
        },
    }
}

pub fn operation_examples() -> Vec<CreatorOpV1> {
    let existing = ObjectRefV1::Existing(entity(1));
    vec![
        CreatorOpV1::CreateFaction {
            local: BatchLocalId(1),
            name: name("Faction"),
            color_rgb: [1, 2, 3],
        },
        CreatorOpV1::CreateSystem {
            local: BatchLocalId(2),
            name: name("System"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        },
        CreatorOpV1::CreateStar {
            local: BatchLocalId(3),
            system: existing,
            name: name("Star"),
            archetype_id: catalog("yellow-dwarf"),
        },
        CreatorOpV1::CreateWorld {
            local: BatchLocalId(4),
            system: existing,
            primary: existing,
            name: name("World"),
            archetype_id: catalog("rocky-world"),
            orbit_radius_milli_au: 1,
            orbit_period_ticks: 1,
            phase_millidegrees: 0,
        },
        CreatorOpV1::ConnectLane {
            local: BatchLocalId(5),
            a: existing,
            b: existing,
        },
        CreatorOpV1::CreateDeposit {
            local: BatchLocalId(6),
            world: existing,
            resource_id: catalog("ore"),
            reserve_units: 1,
        },
        CreatorOpV1::SetOwner {
            target: existing,
            faction: None,
        },
        CreatorOpV1::PlaceIndustry {
            local: BatchLocalId(7),
            world: existing,
            definition_id: catalog("foundry"),
            linked_deposit: None,
        },
        CreatorOpV1::ConnectRoute {
            local: BatchLocalId(8),
            source: existing,
            destination: existing,
            resource_id: catalog("ore"),
            batch_units: 1,
        },
        CreatorOpV1::SetIndustryEnabled {
            industry: existing,
            enabled: false,
        },
        CreatorOpV1::RenameObject {
            target: existing,
            name: name("Renamed"),
        },
        CreatorOpV1::RemoveObject { target: existing },
        CreatorOpV1::ScheduleHazard {
            local: BatchLocalId(9),
            lane: existing,
            hazard_id: catalog("ion-storm"),
            start_tick: WorkshopTick(0),
            duration_ticks: 1,
        },
        CreatorOpV1::CancelHazard { hazard: existing },
    ]
}

pub fn assert_full_text_ink(
    run: &nyon::ui::platform_sdf::PlatformTextRun,
    batch: &nyon::ui::UiBatch,
) {
    let mut uncropped = nyon::ui::UiBatch::default();
    uncropped
        .push_text_clipped(
            &AtlasMetrics::embedded().unwrap(),
            [
                run.bounds.min.x,
                run.bounds.min.y
                    + run.style.font_size
                        * if run.style.role == PlatformTextRole::Control
                            && run.style.overflow == PlatformTextOverflow::Wrap
                        {
                            0.8
                        } else {
                            1.0
                        },
            ],
            run.style.font_size,
            run.style.weight,
            [1.0; 4],
            &run.exact_text,
            None,
        )
        .unwrap();
    let emitted = &batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]];
    assert_eq!(emitted.len(), uncropped.glyphs().len());
    for (actual, full) in emitted.iter().zip(uncropped.glyphs()) {
        let atlas_x = (full.uv_rect[0] * 1024.0).round() as usize;
        let atlas_y = (full.uv_rect[1] * 1024.0).round() as usize;
        for y in 0..64 {
            for x in 0..64 {
                if nyon::ui::UI_ATLAS_BYTES[(atlas_y + y) * 1024 + atlas_x + x] < 96 {
                    continue;
                }
                let ink = nyon::ui::platform::PlatformRect::from_xywh(
                    full.rect[0] + x as f32 * full.rect[2] / 64.0,
                    full.rect[1] + y as f32 * full.rect[3] / 64.0,
                    full.rect[2] / 64.0,
                    full.rect[3] / 64.0,
                );
                let actual = nyon::ui::platform::PlatformRect::from_xywh(
                    actual.rect[0],
                    actual.rect[1],
                    actual.rect[2],
                    actual.rect[3],
                );
                assert!(
                    actual.contains_rect(ink),
                    "clipped ink for {} at scale {}: {:?} outside {:?}",
                    run.semantic_id,
                    run.style.font_size,
                    ink,
                    actual
                );
            }
        }
    }
}

pub fn assert_action_witnesses(
    model: &WorkshopUiModel,
    frame: &nyon::ui::platform::PlatformUiFrame,
) {
    use nyon::ui::platform::{PlatformIconAction, icon_for_action};
    let output = build_platform_ui_batch(frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for node in frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| node.visible && node.action_id.is_some())
    {
        assert!(
            frame
                .controls
                .iter()
                .any(|control| control.semantic_id == node.id
                    && Some(&control.action_id) == node.action_id.as_ref()),
            "visible semantic action {} has no source control",
            node.id
        );
        assert!(
            frame
                .visible_nodes
                .iter()
                .any(|record| record.semantic_id == node.id),
            "visible semantic action {} has no presented record",
            node.id
        );
    }
    for control in &frame.controls {
        let intent;
        let source = match &control.action {
            PlatformUiAction::Workshop(action) => {
                intent = model
                    .controls()
                    .find(|item| &item.action_id == action)
                    .unwrap();
                PlatformIconAction::Workshop(&intent.intent)
            }
            PlatformUiAction::WorkshopView(action) => PlatformIconAction::WorkshopView(*action),
            PlatformUiAction::Shell(action) => PlatformIconAction::Shell(*action),
            PlatformUiAction::Guide(action) => PlatformIconAction::Guide(*action),
        };
        let expected = icon_for_action(source);
        assert_eq!(
            control.icon, expected,
            "wrong typed icon for {}",
            control.action_id
        );
        let presented = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == control.semantic_id);
        let icons = output
            .icon_runs
            .iter()
            .filter(|run| run.semantic_id == control.semantic_id)
            .collect::<Vec<_>>();
        let Some(record) = presented else {
            assert!(icons.is_empty());
            continue;
        };
        assert_eq!(record.icon, expected);
        assert_eq!(
            icons.iter().map(|run| run.icon).collect::<Vec<_>>(),
            expected.into_iter().collect::<Vec<_>>()
        );
        for icon in icons {
            assert!(icon.emitted_glyph_range[0] < icon.emitted_glyph_range[1]);
        }
        let rail = frame.layout.mode == WorkshopLayoutMode::Medium
            && matches!(
                control.action,
                PlatformUiAction::WorkshopView(WorkshopViewAction::OpenNavigator)
            )
            && frame
                .layout
                .left_panel
                .is_some_and(|rail| rail.contains_rect(control.bounds));
        let runs = output
            .text_runs
            .iter()
            .filter(|run| run.semantic_id == control.semantic_id)
            .collect::<Vec<_>>();
        if record.overflow == PlatformTextOverflow::Wrap {
            for run in &runs {
                assert_eq!(run.style.font_size, 13.0 * frame.layout.ui_scale);
                assert!(
                    run.style.line_height >= run.style.font_size,
                    "unreadable line spacing for {}",
                    control.action_id
                );
                assert!(
                    run.clip.contains_rect(run.bounds),
                    "fixed label {} at scale {} has clipped line {:?} in {:?}",
                    control.action_id,
                    frame.layout.ui_scale,
                    run.bounds,
                    run.clip
                );
                assert!(run.emitted_glyph_range[0] < run.emitted_glyph_range[1]);
                assert_full_text_ink(run, &output.batch);
            }
        }
        if rail {
            assert_eq!(expected, Some(UiIcon::Zoom));
            assert!(
                runs.iter().any(|run| run.exact_text == control.label
                    && frame.layout.top_bar.contains_rect(run.clip)),
                "rail has no persistent exact help label"
            );
        } else {
            let label = runs
                .iter()
                .map(|run| run.exact_text.as_str())
                .collect::<String>();
            if record.overflow == PlatformTextOverflow::Wrap {
                assert_eq!(
                    label, control.label,
                    "missing exact supplemental label for {}",
                    control.action_id
                );
            } else {
                assert!(
                    label == control.label
                        || (label.ends_with('.')
                            && control.label.starts_with(label.trim_end_matches('.'))),
                    "variable control label is neither exact nor a bounded prefix: {}",
                    control.action_id
                );
            }
        }
        assert_eq!(
            frame
                .semantics
                .node(control.semantic_id.as_str())
                .unwrap()
                .name,
            control.label
        );
    }
}

pub fn maximum_reference_snapshot() -> WorkshopSessionSnapshot {
    use nyon_workshop_core::{
        CreatorBatchV1, WorkshopHistory,
        command::MAX_BATCH_OPERATIONS,
        model::{MAX_STARS, MAX_SYSTEMS, MAX_WORLDS},
    };
    fn submit(history: &mut WorkshopHistory, operations: Vec<CreatorOpV1>) {
        for chunk in operations.chunks(MAX_BATCH_OPERATIONS) {
            let active = history.active_view();
            history
                .submit(CreatorBatchV1 {
                    expected_cursor: active.view_cursor,
                    expected_tick: active.tick,
                    operations: chunk.to_vec(),
                })
                .unwrap();
        }
    }
    let mut history =
        WorkshopHistory::from_seed_u64(catalog_with_maximum_industry_description(), 45);
    submit(
        &mut history,
        (0..MAX_SYSTEMS)
            .map(|index| CreatorOpV1::CreateSystem {
                local: BatchLocalId(index as u16),
                name: name(&"S".repeat(64)),
                position: GalaxyPointV1::new(index as i64, 0).unwrap(),
            })
            .collect(),
    );
    let system = *history.active_state().systems.keys().next().unwrap();
    submit(
        &mut history,
        (0..MAX_STARS)
            .map(|index| CreatorOpV1::CreateStar {
                local: BatchLocalId(index as u16),
                system: ObjectRefV1::Existing(system),
                name: name(&"T".repeat(64)),
                archetype_id: catalog("yellow-dwarf"),
            })
            .collect(),
    );
    let star = *history.active_state().stars.keys().next().unwrap();
    submit(
        &mut history,
        (0..MAX_WORLDS)
            .map(|index| CreatorOpV1::CreateWorld {
                local: BatchLocalId(index as u16),
                system: ObjectRefV1::Existing(system),
                primary: ObjectRefV1::Existing(star),
                name: name(&"W".repeat(64)),
                archetype_id: catalog("rocky-world"),
                orbit_radius_milli_au: 1000,
                orbit_period_ticks: 100,
                phase_millidegrees: 0,
            })
            .collect(),
    );
    assert_eq!(history.active_state().systems.len(), MAX_SYSTEMS);
    assert_eq!(history.active_state().stars.len(), MAX_STARS);
    assert_eq!(history.active_state().worlds.len(), MAX_WORLDS);
    assert_eq!(
        removal_blockers(history.active_state(), system).len(),
        MAX_STARS + MAX_WORLDS
    );
    nyon::workshop::session::WorkshopSession::new(history)
        .snapshot()
        .clone()
}

pub fn sixty_four_system_snapshot() -> WorkshopSessionSnapshot {
    use nyon_workshop_core::{CreatorBatchV1, WorkshopHistory};

    let mut history = WorkshopHistory::from_seed_u64(core_catalog(), 72);
    history
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: (0..64)
                .map(|index| CreatorOpV1::CreateSystem {
                    local: BatchLocalId(index),
                    name: name(&format!("Reveal system {index:02}")),
                    position: GalaxyPointV1::new(i64::from(index), 0).unwrap(),
                })
                .collect(),
        })
        .unwrap();
    nyon::workshop::session::WorkshopSession::new(history)
        .snapshot()
        .clone()
}

pub fn qualify_sdf_frame(model: &WorkshopUiModel, frame: &nyon::ui::platform::PlatformUiFrame) {
    use nyon::ui::platform::PlatformRect;
    if build_platform_ui_batch(frame, &AtlasMetrics::embedded().unwrap()).is_err() {
        for record in &frame.visible_nodes {
            let mut isolated = frame.clone();
            isolated.visible_nodes = vec![record.clone()];
            if let Err(error) =
                build_platform_ui_batch(&isolated, &AtlasMetrics::embedded().unwrap())
            {
                eprintln!(
                    "Invalid record {} {:?} {:?}: {error:?}",
                    record.semantic_id, record.bounds, record.display_text
                );
            }
        }
    }
    assert_action_witnesses(model, frame);
    let output = build_platform_ui_batch(frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for run in &output.text_runs {
        assert_full_text_ink(run, &output.batch);
        assert!(
            run.clip.contains_rect(run.bounds),
            "{} line {:?} escapes {:?} at {:?} / {}",
            run.semantic_id,
            run.bounds,
            run.clip,
            frame.viewport,
            frame.layout.ui_scale
        );
        assert!(frame.layout.viewport.contains_rect(run.bounds));
        assert!(
            frame
                .controls
                .iter()
                .filter(|control| frame
                    .visible_nodes
                    .iter()
                    .any(|record| record.semantic_id == control.semantic_id))
                .all(|control| {
                    control.semantic_id == run.semantic_id || !run.bounds.overlaps(control.bounds)
                }),
            "{} intersects another control at {:?} / {}",
            run.semantic_id,
            frame.viewport,
            frame.layout.ui_scale
        );
        for glyph in &output.batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
        {
            let rect =
                PlatformRect::from_xywh(glyph.rect[0], glyph.rect[1], glyph.rect[2], glyph.rect[3]);
            assert!(run.clip.contains_rect(rect));
            assert!(frame.layout.viewport.contains_rect(rect));
        }
    }
    for panel in &output.panel_witnesses {
        assert!(frame.layout.viewport.contains_rect(panel.bounds));
        if panel.role == PlatformPanelRole::PersistentChrome {
            assert!(!panel.bounds.overlaps(frame.layout.canvas));
        }
        for emitted in
            &output.batch.panels()[panel.emitted_panel_range[0]..panel.emitted_panel_range[1]]
        {
            assert!(panel.bounds.contains_rect(PlatformRect::from_xywh(
                emitted.rect[0],
                emitted.rect[1],
                emitted.rect[2],
                emitted.rect[3]
            )));
        }
    }
    if frame.high_contrast {
        for control in frame
            .controls
            .iter()
            .filter(|control| control.focused && control.enabled)
        {
            let border = output
                .panel_witnesses
                .iter()
                .find(|panel| {
                    panel.role == PlatformPanelRole::ContrastBorder
                        && panel.owning_node.as_ref() == Some(&control.semantic_id)
                })
                .unwrap();
            assert_eq!(
                border.emitted_panel_range[1] - border.emitted_panel_range[0],
                4
            );
            assert!(
                output.batch.panels()[border.emitted_panel_range[0]..border.emitted_panel_range[1]]
                    .iter()
                    .all(|panel| panel.rect[2] <= 2.0 || panel.rect[3] <= 2.0)
            );
        }
    }
    assert!(output.batch.glyphs().len() < 8192);
    assert!(output.batch.panels().len() < 512);
}

pub fn control<'a>(
    model: &'a WorkshopUiModel,
    id: &str,
) -> &'a nyon::ui::workshop::WorkshopControl {
    model
        .controls()
        .find(|control| control.action_id.as_str() == id)
        .unwrap_or_else(|| panic!("missing control {id}"))
}

pub fn marker_layer_rank(layer: WorkshopMarkerLayer) -> u8 {
    match layer {
        WorkshopMarkerLayer::SystemRing => 0,
        WorkshopMarkerLayer::StarHalo => 1,
        WorkshopMarkerLayer::StarCore => 2,
        WorkshopMarkerLayer::StarSelectionRing => 3,
        WorkshopMarkerLayer::World => 4,
    }
}

pub fn luminance(color: [f32; 4]) -> f32 {
    color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722
}

pub fn matching_witness<'a>(
    witnesses: &'a [WorkshopMarkerWitness],
    expected: &WorkshopMarkerWitness,
) -> &'a WorkshopMarkerWitness {
    witnesses
        .iter()
        .find(|witness| {
            witness.layer == expected.layer && witness.source_index == expected.source_index
        })
        .expect("normal and high-contrast marker layers must correspond")
}

pub fn assert_marker_witness(batch: &PrimitiveBatch, witness: &WorkshopMarkerWitness) {
    let vertices = &batch.vertices()[witness.vertex_span[0]..witness.vertex_span[1]];
    assert_eq!(vertices.len(), 6);
    let expected_shape = match witness.layer {
        WorkshopMarkerLayer::SystemRing
        | WorkshopMarkerLayer::StarHalo
        | WorkshopMarkerLayer::StarCore
        | WorkshopMarkerLayer::StarSelectionRing
        | WorkshopMarkerLayer::World => 2,
    };
    assert!(vertices.iter().all(|vertex| {
        vertex.shape == witness.packed_shape
            && vertex.shape & 0xff == expected_shape
            && vertex.color == witness.color
    }));
    match witness.ring_thickness {
        Some(thickness) => {
            let packed_width = witness.packed_shape >> 8;
            let decoded = packed_width as f32 / ((1_u32 << 24) - 1) as f32 * witness.radius;
            let quantization_tolerance = witness.radius / ((1_u32 << 24) - 1) as f32;
            assert!((decoded - thickness).abs() <= quantization_tolerance);
        }
        None => panic!("every Workshop marker must use the cross-backend ring path"),
    }
    let min = vertices
        .iter()
        .fold(Vec2::splat(f32::INFINITY), |min, vertex| {
            min.min(Vec2::from_array(vertex.position))
        });
    let max = vertices
        .iter()
        .fold(Vec2::splat(f32::NEG_INFINITY), |max, vertex| {
            max.max(Vec2::from_array(vertex.position))
        });
    assert_eq!(min, witness.center - Vec2::splat(witness.radius));
    assert_eq!(max, witness.center + Vec2::splat(witness.radius));
}

pub fn contains_full_viewport_quad(batch: &PrimitiveBatch, viewport: Vec2) -> bool {
    batch.vertices().as_chunks::<6>().0.iter().any(|vertices| {
        let min = vertices
            .iter()
            .fold(Vec2::splat(f32::INFINITY), |min, vertex| {
                min.min(Vec2::from_array(vertex.position))
            });
        let max = vertices
            .iter()
            .fold(Vec2::splat(f32::NEG_INFINITY), |max, vertex| {
                max.max(Vec2::from_array(vertex.position))
            });
        min == Vec2::ZERO && max == viewport
    })
}

pub fn opaque_quad_covers(batch: &PrimitiveBatch, point: Vec2) -> bool {
    batch.vertices().as_chunks::<6>().0.iter().any(|vertices| {
        let alpha = vertices[0].color[3];
        let shape = vertices[0].shape & 0xff;
        if alpha < 0.90 || shape != 0 {
            return false;
        }
        let min = vertices
            .iter()
            .fold(Vec2::splat(f32::INFINITY), |min, vertex| {
                min.min(Vec2::from_array(vertex.position))
            });
        let max = vertices
            .iter()
            .fold(Vec2::splat(f32::NEG_INFINITY), |max, vertex| {
                max.max(Vec2::from_array(vertex.position))
            });
        point.cmpge(min).all() && point.cmple(max).all()
    })
}
