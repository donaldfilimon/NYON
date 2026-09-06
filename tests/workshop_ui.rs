use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;
use nyon::ui::{
    AtlasMetrics, UiBatchError, UiIcon,
    platform_sdf::{
        PlatformPanelRole, PlatformTextOverflow, PlatformTextRole, build_platform_ui_batch,
    },
};
use nyon::{
    engine::{backend::BackendKind, primitives::PrimitiveBatch},
    ui::{
        accessibility::{AnnouncementKind, FocusManager, InputModality, SemanticRole},
        creator::{CreatorDraft, CreatorFieldKind},
        platform::{
            PlatformBackground, PlatformUiAction, WorkshopMarkerLayer, WorkshopMarkerWitness,
            build_workshop_platform_frame, build_workshop_platform_frame_for_view,
            build_workshop_platform_frame_with_layout, draw_workshop_scene,
        },
        workshop::{
            CreatorTool, WorkshopUiContext, WorkshopUiIntent, WorkshopUiModel,
            default_creator_batch, removal_blockers,
        },
        workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
        workshop_view::{WorkshopDrawer, WorkshopViewAction, WorkshopViewState},
    },
    workshop::{
        ActiveView, BatchLocalId, BranchId, BranchRef, CatalogHash, CatalogId, CreatorOpV1,
        EntityId, GalaxyPointV1, ObjectName, ObjectRefV1, RevisionId, WorkshopStateV1,
        WorkshopTick,
        session::{
            WorkshopDiagnostic, WorkshopDiagnosticCode, WorkshopSessionSnapshot, WorkshopSpeed,
            WorkshopStoreSnapshot,
        },
        store::{SaveGeneration, SlotId},
    },
};
use nyon_workshop_core::model::{
    DepositV1, FactionV1, HazardV1, IndustryV1, LaneV1, RouteV1, ShipmentV1, StarV1, SystemV1,
    WorldV1,
};

fn core_catalog() -> nyon_workshop_core::ValidatedCatalogPackV1 {
    nyon_workshop_core::decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json"))
        .unwrap()
}

fn catalog_with_maximum_industry_description() -> nyon_workshop_core::ValidatedCatalogPackV1 {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    let definition = value["industry_definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == "extractor")
        .unwrap();
    definition["description"] = serde_json::json!(format!("FIRST{}LAST", "D".repeat(503)));
    nyon_workshop_core::decode_catalog_pack(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn entity(value: u8) -> EntityId {
    EntityId([value; 16])
}

fn revision(value: u8) -> RevisionId {
    RevisionId([value; 32])
}

fn branch(value: u8) -> BranchId {
    BranchId([value; 16])
}

fn catalog(value: &str) -> CatalogId {
    CatalogId::new(value).unwrap()
}

fn name(value: &str) -> ObjectName {
    ObjectName::new(value).unwrap()
}

fn populated_state() -> WorkshopStateV1 {
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

fn snapshot() -> WorkshopSessionSnapshot {
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

fn operation_examples() -> Vec<CreatorOpV1> {
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

#[test]
fn platform_sdf_uses_source_records_for_semantic_text_and_icons() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let save_id = frame
        .controls
        .iter()
        .find(|control| control.label == "Save Workshop")
        .unwrap()
        .action_id
        .clone();
    frame.append_status_line("Waiting for durable save");
    frame.reconcile_focused(Some(&save_id));
    let metrics = AtlasMetrics::embedded().unwrap();
    let output = build_platform_ui_batch(&frame, &metrics).unwrap();
    for run in &output.text_runs {
        assert_eq!(
            run.logical_advance,
            metrics
                .measure_text(run.style.font_size, run.style.weight, &run.exact_text)
                .unwrap()
                .advance
        );
        assert!(frame.visible_nodes.iter().any(|record| {
            record.semantic_id == run.semantic_id && record.bounds.contains_rect(run.clip)
        }));
    }
    for run in &output.icon_runs {
        let owner = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == run.semantic_id)
            .unwrap();
        assert_eq!(owner.icon, Some(run.icon));
        assert!(owner.bounds.contains_rect(run.bounds));
        assert!(run.clip.contains_rect(run.bounds));
    }
    for run in &output.text_runs {
        for glyph in &output.batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
        {
            let ink = nyon::ui::platform::PlatformRect::from_xywh(
                glyph.rect[0],
                glyph.rect[1],
                glyph.rect[2],
                glyph.rect[3],
            );
            assert!(run.clip.contains_rect(ink));
        }
    }
    for run in &output.icon_runs {
        for glyph in &output.batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
        {
            let ink = nyon::ui::platform::PlatformRect::from_xywh(
                glyph.rect[0],
                glyph.rect[1],
                glyph.rect[2],
                glyph.rect[3],
            );
            assert!(run.clip.contains_rect(ink));
            assert!(run.bounds.contains_rect(ink));
        }
    }
    for node in frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.visible
                && matches!(
                    node.role,
                    SemanticRole::Button
                        | SemanticRole::TreeItem
                        | SemanticRole::Heading
                        | SemanticRole::Text
                        | SemanticRole::Status
                        | SemanticRole::Alert
                        | SemanticRole::ListItem
                        | SemanticRole::Option
                        | SemanticRole::Checkbox
                        | SemanticRole::TextInput
                        | SemanticRole::SpinButton
                )
                && (!node.name.is_empty() || !node.description.is_empty() || node.value.is_some())
        })
    {
        let record = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("semantic node {} lacks a visible record", node.id));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
                || output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == node.id),
            "semantic node {} lacks SDF output",
            node.id
        );
    }
    assert_action_witnesses(&model, &frame);
    assert!(!frame.visible_nodes.is_empty());
    assert!(frame.visible_nodes.iter().any(|record| {
        record.role == PlatformTextRole::Status
            && record.overflow == PlatformTextOverflow::Wrap
            && record.semantic_id.as_str() == "save.status"
    }));
    assert!(frame.visible_nodes.iter().any(|record| {
        record.role == PlatformTextRole::Status && record.display_text == "Waiting for durable save"
    }));
    for record in frame
        .visible_nodes
        .iter()
        .filter(|record| record.informative())
    {
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| { run.semantic_id == record.semantic_id && !run.exact_text.is_empty() })
                || output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == record.semantic_id)
        );
    }
    let save = frame
        .controls
        .iter()
        .find(|control| control.label == "Save Workshop")
        .unwrap();
    assert_eq!(save.icon, Some(UiIcon::Save));
    assert!(save.focused);
    assert!(
        output
            .icon_runs
            .iter()
            .any(|run| run.semantic_id == save.semantic_id && run.icon == UiIcon::Save)
    );
}

fn assert_full_text_ink(run: &nyon::ui::platform_sdf::PlatformTextRun, batch: &nyon::ui::UiBatch) {
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

fn assert_action_witnesses(model: &WorkshopUiModel, frame: &nyon::ui::platform::PlatformUiFrame) {
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

#[test]
fn public_visible_record_remains_constructible_outside_the_crate() {
    use nyon::ui::{
        accessibility::SemanticNodeId,
        platform::PlatformRect,
        platform_sdf::{PlatformVisibleNodeRecord, PlatformVisibleNodeState},
    };
    let record = PlatformVisibleNodeRecord {
        semantic_id: SemanticNodeId::new("external.status"),
        action_id: None,
        display_text: "Ready".to_owned(),
        semantic_name: "Ready".to_owned(),
        semantic_description: String::new(),
        semantic_value: None,
        role: PlatformTextRole::Status,
        overflow: PlatformTextOverflow::Wrap,
        bounds: PlatformRect::from_xywh(0.0, 0.0, 100.0, 40.0),
        clip: None,
        state: PlatformVisibleNodeState::default(),
        icon: None,
        prewrapped_lines: None,
    };
    assert!(record.informative());
}

#[test]
fn typed_icons_freeze_allowed_and_forbidden_meanings() {
    use nyon::{
        app::client_runtime::MainMenuRoute,
        ui::{
            guide::{GuideAction, GuideLocation},
            platform::{PlatformIconAction as Source, ShellUiAction, icon_for_action},
        },
        workshop::session::WorkshopAction,
    };
    for (action, expected) in [
        (WorkshopViewAction::OpenCreator, Some(UiIcon::Crosshair)),
        (WorkshopViewAction::OpenNavigator, Some(UiIcon::Zoom)),
        (WorkshopViewAction::CloseDrawer, Some(UiIcon::Close)),
        (WorkshopViewAction::ShowInspector, Some(UiIcon::Zoom)),
        (WorkshopViewAction::ScrollPrevious, None),
        (WorkshopViewAction::ScrollNext, None),
        (WorkshopViewAction::ScrollInspectorPrevious, None),
        (WorkshopViewAction::ScrollInspectorNext, None),
        (WorkshopViewAction::ShowHierarchy, None),
        (WorkshopViewAction::ShowBranches, None),
    ] {
        assert_eq!(icon_for_action(Source::WorkshopView(action)), expected);
    }
    for (action, expected) in [
        (
            ShellUiAction::Menu(MainMenuRoute::Continue),
            Some(UiIcon::Load),
        ),
        (
            ShellUiAction::Menu(MainMenuRoute::Settings),
            Some(UiIcon::Settings),
        ),
        (ShellUiAction::Menu(MainMenuRoute::NewWorkshop), None),
        (ShellUiAction::Menu(MainMenuRoute::ClassicSector), None),
        (ShellUiAction::Menu(MainMenuRoute::Credits), None),
        (ShellUiAction::CloseSettings, Some(UiIcon::Close)),
        (ShellUiAction::DismissCredits, Some(UiIcon::Close)),
        (ShellUiAction::DismissRecovery, Some(UiIcon::Close)),
        (ShellUiAction::ContinueRecovery, Some(UiIcon::Check)),
        (ShellUiAction::CycleUiScale, Some(UiIcon::Settings)),
        (ShellUiAction::ToggleReducedMotion, Some(UiIcon::Settings)),
        (ShellUiAction::ToggleHighContrast, Some(UiIcon::Settings)),
        (ShellUiAction::ReturnToMainMenu, None),
    ] {
        assert_eq!(icon_for_action(Source::Shell(action)), expected);
    }
    for (action, expected) in [
        (GuideAction::Open, Some(UiIcon::Help)),
        (GuideAction::Close, Some(UiIcon::Close)),
        (GuideAction::MainMenu, None),
        (
            GuideAction::Show(GuideLocation::for_screen(
                nyon::app::client_runtime::ClientScreen::GalaxyWorkshop,
            )),
            None,
        ),
    ] {
        assert_eq!(icon_for_action(Source::Guide(action)), expected);
    }
    for (intent, expected) in [
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Pause),
            Some(UiIcon::Pause),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Resume(WorkshopSpeed::One)),
            Some(UiIcon::Speed),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::StepOnce),
            Some(UiIcon::Speed),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestSave),
            Some(UiIcon::Save),
        ),
        (WorkshopUiIntent::Dispatch(WorkshopAction::Undo), None),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Redo(revision(2))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::SelectBranch(branch(1))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestExport),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestImport(Box::new([]))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestLoad(SlotId(4))),
            Some(UiIcon::Load),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Submit(
                default_creator_batch(&snapshot(), None, CreatorTool::CreateSystem).unwrap(),
            )),
            Some(UiIcon::Check),
        ),
        (
            WorkshopUiIntent::OpenCreatorForm {
                tool: CreatorTool::CreateSystem,
                subject: None,
            },
            Some(UiIcon::Crosshair),
        ),
        (WorkshopUiIntent::CloseCreatorForm, Some(UiIcon::Close)),
        (
            WorkshopUiIntent::CloseRemovalConfirmation,
            Some(UiIcon::Close),
        ),
        (WorkshopUiIntent::ReturnToMainMenu, None),
        (
            WorkshopUiIntent::SelectEntity(entity(2)),
            Some(UiIcon::Crosshair),
        ),
        (
            WorkshopUiIntent::SetReducedMotion(true),
            Some(UiIcon::Settings),
        ),
        (
            WorkshopUiIntent::SetHighContrast(true),
            Some(UiIcon::Settings),
        ),
        (WorkshopUiIntent::OpenRemovalConfirmation(entity(2)), None),
        (
            WorkshopUiIntent::EditCreatorField {
                field_id: "name".to_owned(),
                cycle: false,
            },
            None,
        ),
    ] {
        assert_eq!(icon_for_action(Source::Workshop(&intent)), expected);
    }
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        icon_for_action(Source::Shell(ShellUiAction::Menu(MainMenuRoute::Quit))),
        None
    );
}

#[test]
fn fixed_action_labels_supplement_icons_across_responsive_presentations() {
    for (viewport, scale) in [
        Vec2::new(1600.0, 900.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(1024.0, 768.0),
        Vec2::new(723.0, 802.0),
        Vec2::new(640.0, 480.0),
        Vec2::new(320.0, 450.0),
    ]
    .into_iter()
    .flat_map(|viewport| [0.85, 1.0, 1.15, 1.3].map(|scale| (viewport, scale)))
    {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        for context in [
            WorkshopUiContext::default(),
            WorkshopUiContext {
                pending_removal: Some(entity(2)),
                ..WorkshopUiContext::default()
            },
        ]
        .into_iter()
        .chain(CreatorTool::ALL.map(|tool| WorkshopUiContext {
            creator_form: Some(tool),
            ..WorkshopUiContext::default()
        })) {
            let model = WorkshopUiModel::build(&snapshot(), context);
            for open_drawer in [
                None,
                Some(WorkshopDrawer::Creator),
                Some(WorkshopDrawer::Navigator),
            ] {
                let mut view = WorkshopViewState::default();
                view.open_drawer = open_drawer;
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                if let Some(form) = &model.creator_form {
                    let cancel = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == form.cancel_control.action_id)
                        .unwrap();
                    let submit = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == form.submit_control.action_id)
                        .unwrap();
                    assert!(!cancel.bounds.overlaps(submit.bounds));
                    assert!(cancel.bounds.width() >= 44.0 && cancel.bounds.height() >= 39.99);
                    assert!(submit.bounds.width() >= 44.0 && submit.bounds.height() >= 39.99);
                }
                assert!(
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).is_ok(),
                    "fixed layout failed at {viewport:?}, scale {scale}, form {:?}, drawer {open_drawer:?}",
                    model.creator_form.as_ref().map(|form| form.tool)
                );
                assert_action_witnesses(&model, &frame);
            }
        }
    }
}

#[test]
fn fixed_action_layout_rejects_boxes_that_cannot_fit_readable_lines() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1280.0, 720.0), None);
    let record = frame
        .visible_nodes
        .iter_mut()
        .find(|record| {
            record.role == PlatformTextRole::Control
                && record.overflow == PlatformTextOverflow::Wrap
        })
        .unwrap();
    record.bounds.max = record.bounds.min + Vec2::new(40.0, 12.0);
    record.clip = Some(record.bounds);
    assert!(matches!(
        build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()),
        Err(UiBatchError::NonFiniteGeometry),
    ));
}

const SDF_QUALIFICATION_MATRIX: [(f32, f32, f32); 11] = [
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

fn maximum_reference_snapshot() -> WorkshopSessionSnapshot {
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

fn sixty_four_system_snapshot() -> WorkshopSessionSnapshot {
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

#[test]
fn short_modal_pages_reveal_every_field_and_maximum_reference_blockers() {
    let current = maximum_reference_snapshot();
    let system = *current.state.systems.keys().next().unwrap();
    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), scale).unwrap();
        for high_contrast in [false, true] {
            let model = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    pending_removal: Some(system),
                    high_contrast,
                    ..WorkshopUiContext::default()
                },
            );
            let mut view = WorkshopViewState::default();
            let mut seen = BTreeSet::new();
            let expected = model
                .semantics
                .nodes_depth_first()
                .into_iter()
                .filter(|node| node.id.as_str().starts_with("removal.blocker."))
                .map(|node| {
                    (
                        node.id.clone(),
                        format!("{} - {}", node.name, node.description),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut rendered = BTreeMap::<_, String>::new();
            let mut max_glyphs = 0;
            let mut max_panels = 0;
            let mut first = None;
            let mut forward_pages = Vec::new();
            for _ in 0..4000 {
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                let output =
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
                forward_pages.push(frame.visible_nodes.clone());
                max_glyphs = max_glyphs.max(output.batch.glyphs().len());
                max_panels = max_panels.max(output.batch.panels().len());
                let ids = output
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id.as_str().starts_with("removal.blocker."))
                    .map(|run| run.semantic_id.clone())
                    .collect::<BTreeSet<_>>();
                seen.extend(ids);
                for run in output
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id.as_str().starts_with("removal.blocker."))
                {
                    rendered
                        .entry(run.semantic_id.clone())
                        .or_default()
                        .push_str(&run.exact_text);
                }
                for (id, text) in &rendered {
                    assert!(
                        expected[id].starts_with(text),
                        "paging repeated or skipped {id}"
                    );
                }
                if first.is_none() {
                    first = Some(frame.clone());
                }
                for id in [
                    "remove.cancel",
                    "remove.confirm",
                    "view.scroll-previous",
                    "view.scroll-next",
                ] {
                    let control = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id.as_str() == id)
                        .unwrap();
                    assert!(frame.modal.unwrap().contains_rect(control.bounds));
                    if control.enabled {
                        assert_eq!(
                            frame.hit_test(control.bounds.center()).unwrap().action,
                            control.action
                        );
                    }
                }
                if rendered == expected {
                    qualify_sdf_frame(&model, &frame);
                    break;
                }
                view.apply(WorkshopViewAction::ScrollNext, &model, &layout);
            }
            assert_eq!(seen.len(), 640);
            assert_eq!(rendered, expected);
            for expected_page in forward_pages.iter().rev().skip(1) {
                view.apply(WorkshopViewAction::ScrollPrevious, &model, &layout);
                let reverse =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                assert_eq!(
                    &reverse.visible_nodes, expected_page,
                    "Previous must traverse one complete measured page without overlap or skips"
                );
                qualify_sdf_frame(&model, &reverse);
            }
            assert!(max_glyphs < 8192 && max_panels < 512);
            eprintln!(
                "CAPACITY removal640 @ {scale} contrast={high_contrast}: glyphs={max_glyphs} panels={max_panels}"
            );
            let first = first.unwrap();
            qualify_sdf_frame(&model, &first);
            let first_id = nyon::ui::accessibility::SemanticNodeId::new("removal.blocker.0");
            assert!(view.reveal_semantic(&model, &layout, &first_id));
            let revealed =
                build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
            assert_eq!(first.visible_nodes, revealed.visible_nodes);
            let creator = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    creator_form: Some(CreatorTool::CreateWorld),
                    high_contrast,
                    ..WorkshopUiContext::default()
                },
            );
            let form = creator.creator_form.as_ref().unwrap();
            for field in &form.fields {
                assert!(view.reveal_action(&creator, &layout, &field.control.action_id));
                let frame = build_workshop_platform_frame_for_view(
                    &creator,
                    layout.clone(),
                    &view,
                    Some(&field.control.action_id),
                );
                qualify_sdf_frame(&creator, &frame);
                let control = frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == field.control.action_id)
                    .unwrap();
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    field.control.action_id
                );
                assert_eq!(
                    frame
                        .semantics
                        .node(control.semantic_id.as_str())
                        .unwrap()
                        .value
                        .as_deref(),
                    Some(field.value.as_str())
                );
                assert_eq!(
                    creator.activate(&field.control.action_id, InputModality::Keyboard),
                    creator.activate(&field.control.action_id, InputModality::Pointer)
                );
            }
        }
    }
}

#[test]
fn short_height_maximum_description_and_navigation_remain_reachable_in_both_motion_modes() {
    let catalog = catalog_with_maximum_industry_description();
    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), scale).unwrap();
        for high_contrast in [false, true] {
            let current = snapshot();
            let models = [false, true].map(|reduced_motion| {
                WorkshopUiModel::build(
                    &current,
                    WorkshopUiContext {
                        catalog: Some(&catalog),
                        selected_entity: Some(entity(6)),
                        high_contrast,
                        reduced_motion,
                        ..WorkshopUiContext::default()
                    },
                )
            });
            let detail = models[0]
                .inspector
                .rows()
                .find(|row| row.label == "Catalog detail")
                .unwrap();
            assert_eq!(detail.value.len(), 512);
            assert!(detail.value.starts_with("FIRST") && detail.value.ends_with("LAST"));
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_semantic(&models[0], &layout, &detail.semantic_id));
            let mut saw_first = false;
            let mut saw_last = false;
            for _ in 0..80 {
                let frames = models.each_ref().map(|model| {
                    build_workshop_platform_frame_for_view(model, layout.clone(), &view, None)
                });
                let outputs = frames.each_ref().map(|frame| {
                    build_platform_ui_batch(frame, &AtlasMetrics::embedded().unwrap()).unwrap()
                });
                assert_eq!(outputs[0].text_runs, outputs[1].text_runs);
                assert_eq!(outputs[0].icon_runs, outputs[1].icon_runs);
                assert_eq!(outputs[0].batch.glyphs(), outputs[1].batch.glyphs());
                let reduced_motion_id = frames[1]
                    .controls
                    .iter()
                    .find(|control| {
                        control.action_id == models[1].preferences.reduced_motion_control.action_id
                    })
                    .map(|control| control.semantic_id.clone())
                    .unwrap();
                let reduced_motion_outline = outputs[1].panel_witnesses.iter().find(|witness| {
                    witness.role == PlatformPanelRole::ContrastBorder
                        && witness.owning_node.as_ref() == Some(&reduced_motion_id)
                });
                assert_eq!(reduced_motion_outline.is_some(), high_contrast);
                assert!(outputs[0].panel_witnesses.iter().all(|witness| {
                    witness.role != PlatformPanelRole::ContrastBorder
                        || witness.owning_node.as_ref() != Some(&reduced_motion_id)
                }));
                let selection_range = reduced_motion_outline
                    .map(|witness| witness.emitted_panel_range)
                    .unwrap_or([0, 0]);
                assert_eq!(
                    outputs[0]
                        .batch
                        .panels()
                        .iter()
                        .map(|panel| panel.rect)
                        .collect::<Vec<_>>(),
                    outputs[1]
                        .batch
                        .panels()
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| {
                            *index < selection_range[0] || *index >= selection_range[1]
                        })
                        .map(|(_, panel)| panel.rect)
                        .collect::<Vec<_>>()
                );
                let text = outputs[0]
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id == detail.semantic_id)
                    .map(|run| run.exact_text.as_str())
                    .collect::<String>();
                saw_first |= text.contains("FIRST");
                saw_last |= text.contains("LAST");
                assert!(
                    frames[0]
                        .sighted_text
                        .iter()
                        .any(|text| text.kind == nyon::ui::workshop::InspectorTextKind::Title)
                );
                for action in [
                    &models[0].preferences.reduced_motion_control.action_id,
                    &models[0].preferences.high_contrast_control.action_id,
                    &models[0]
                        .inspector
                        .remove_control
                        .as_ref()
                        .unwrap()
                        .action_id,
                    &WorkshopViewAction::ScrollInspectorNext.action_id(),
                ] {
                    assert!(
                        frames[0]
                            .controls
                            .iter()
                            .any(|control| &control.action_id == action),
                        "missing persistent footer {action}"
                    );
                }
                qualify_sdf_frame(&models[0], &frames[0]);
                if saw_last {
                    break;
                }
                view.apply(WorkshopViewAction::ScrollInspectorNext, &models[0], &layout);
            }
            assert!(saw_first && saw_last);
            for model in &models {
                let mut view = WorkshopViewState::default();
                let default =
                    build_workshop_platform_frame_for_view(model, layout.clone(), &view, None);
                assert!(default.controls.iter().any(
                    |control| control.action_id == WorkshopViewAction::OpenCreator.action_id()
                ));
                for tool in &model.tool_palette {
                    assert!(view.reveal_action(model, &layout, &tool.control.action_id));
                    let frame = build_workshop_platform_frame_for_view(
                        model,
                        layout.clone(),
                        &view,
                        Some(&tool.control.action_id),
                    );
                    qualify_sdf_frame(model, &frame);
                    let control = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == tool.control.action_id)
                        .unwrap();
                    assert_eq!(
                        frame.hit_test(control.bounds.center()).unwrap().action_id,
                        control.action_id
                    );
                }
            }
        }
    }
}

fn qualify_sdf_frame(model: &WorkshopUiModel, frame: &nyon::ui::platform::PlatformUiFrame) {
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

#[test]
fn sdf_required_matrix_crosses_drawers_modal_validation_and_contrast() {
    let mut glyph_maximum = (0, String::new());
    let mut panel_maximum = (0, String::new());
    let current = snapshot();
    let catalog = catalog_with_maximum_industry_description();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut invalid = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    invalid.set_text("name", "").unwrap();
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        for high_contrast in [false, true] {
            for state in 0..6 {
                let context = WorkshopUiContext {
                    catalog: Some(&catalog),
                    selected_entity: Some(entity(6)),
                    high_contrast,
                    creator_form: (state == 2 || state == 3).then_some(CreatorTool::CreateSystem),
                    creator_draft: (state == 3).then_some(&invalid),
                    pending_removal: (state == 1).then_some(entity(2)),
                    backend: (state == 5).then_some(BackendKind::WebGl2Low),
                    ..WorkshopUiContext::default()
                };
                let mut state_snapshot = current.clone();
                if state == 4 {
                    state_snapshot.state = WorkshopStateV1::default();
                    state_snapshot.state_digest = state_snapshot.state.digest();
                    state_snapshot.active_view.digest = state_snapshot.state_digest;
                    state_snapshot.active_view.tick = WorkshopTick(0);
                }
                if state == 5 {
                    state_snapshot.store.commit_pending = true;
                    state_snapshot.diagnostics.push(WorkshopDiagnostic {
                        code: WorkshopDiagnosticCode::ArchiveRejected,
                        message: "Untrusted imported error".repeat(30),
                    });
                }
                let model = WorkshopUiModel::build(&state_snapshot, context);
                for open_drawer in [
                    None,
                    Some(WorkshopDrawer::Navigator),
                    Some(WorkshopDrawer::Creator),
                ] {
                    let mut view = WorkshopViewState::default();
                    view.open_drawer = open_drawer;
                    let mut frame =
                        build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                    let focus = frame
                        .controls
                        .iter()
                        .find(|control| {
                            control.enabled
                                && frame
                                    .semantics
                                    .node(control.semantic_id.as_str())
                                    .is_some_and(|node| node.visible)
                        })
                        .map(|control| control.action_id.clone());
                    frame.reconcile_focused(focus.as_ref());
                    qualify_sdf_frame(&model, &frame);
                    let output =
                        build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap())
                            .unwrap();
                    let state = format!(
                        "{width}x{height}@{scale} contrast={high_contrast} state={state} drawer={open_drawer:?}"
                    );
                    if output.batch.glyphs().len() > glyph_maximum.0 {
                        glyph_maximum = (output.batch.glyphs().len(), state.clone());
                    }
                    if output.batch.panels().len() > panel_maximum.0 {
                        panel_maximum = (output.batch.panels().len(), state);
                    }
                }
            }
        }
    }
    eprintln!("MATRIX MAX glyphs={glyph_maximum:?} panels={panel_maximum:?}");
}

#[test]
fn validated_capacity_navigation_frames_stay_below_renderer_limits() {
    let current = maximum_reference_snapshot();
    let mut maxima = (0, 0, String::new(), String::new());
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        for high_contrast in [false, true] {
            let model = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    high_contrast,
                    selected_entity: current.state.worlds.keys().next().copied(),
                    ..WorkshopUiContext::default()
                },
            );
            for action in [
                None,
                Some(WorkshopViewAction::OpenCreator),
                Some(WorkshopViewAction::OpenNavigator),
                Some(WorkshopViewAction::ShowStatus),
            ] {
                let mut view = WorkshopViewState::default();
                if let Some(action) = action {
                    view.apply(action, &model, &layout);
                }
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                qualify_sdf_frame(&model, &frame);
                let output =
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
                let state = format!(
                    "64 systems/128 stars/512 worlds,64char names,{width}x{height}@{scale},contrast={high_contrast},action={action:?}"
                );
                if output.batch.glyphs().len() > maxima.0 {
                    maxima.0 = output.batch.glyphs().len();
                    maxima.2.clone_from(&state);
                }
                if output.batch.panels().len() > maxima.1 {
                    maxima.1 = output.batch.panels().len();
                    maxima.3 = state;
                }
            }
        }
    }
    eprintln!("VALIDATED CAPACITY MAX {maxima:?}");
}

#[test]
fn shell_chrome_qualifies_at_every_required_viewport_and_scale() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability, MainMenuRoute},
        preferences::{UiScale, UserPreferencesV1},
        ui::platform::{ShellPlatformInput, build_shell_platform_frame},
    };
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let capabilities = [
        MainMenuRoute::NewWorkshop,
        MainMenuRoute::Continue,
        MainMenuRoute::ClassicSector,
        MainMenuRoute::Settings,
        MainMenuRoute::Credits,
    ]
    .map(|route| MainMenuCapability {
        route,
        enabled: true,
    });
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        for high_contrast in [false, true] {
            for (screen, credits_visible) in [
                (ClientScreen::MainMenu, false),
                (ClientScreen::Settings, false),
                (ClientScreen::MainMenu, true),
                (ClientScreen::RecoverableError, false),
            ] {
                let frame = build_shell_platform_frame(ShellPlatformInput {
                    screen,
                    capabilities: &capabilities,
                    credits_visible,
                    recovery_message: Some("Recovery available"),
                    continue_available: true,
                    backend: Some(BackendKind::WebGl2Low),
                    preferences: UserPreferencesV1 {
                        ui_scale: UiScale::try_from((scale * 100.0).round() as u16).unwrap(),
                        high_contrast,
                        ..UserPreferencesV1::default()
                    },
                    viewport: Vec2::new(width, height),
                    focused: None,
                });
                qualify_sdf_frame(&model, &frame);
            }
        }
    }
}

#[test]
fn settings_ui_scale_control_is_exact_semantic_and_ink_qualified_on_short_viewports() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability},
        preferences::{UiScale, UserPreferencesV1},
        ui::{
            accessibility::SemanticActionId,
            platform::{
                PlatformUiAction, ShellPlatformInput, ShellUiAction, build_shell_platform_frame,
            },
        },
    };

    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let action_id = SemanticActionId::new("shell.settings.scale");
    for viewport in [Vec2::new(1280.0, 480.0), Vec2::new(723.0, 802.0)] {
        let frame = build_shell_platform_frame(ShellPlatformInput {
            screen: ClientScreen::Settings,
            capabilities: &[] as &[MainMenuCapability],
            credits_visible: false,
            recovery_message: None,
            continue_available: false,
            backend: Some(BackendKind::WebGl2Low),
            preferences: UserPreferencesV1 {
                ui_scale: UiScale::Percent130,
                ..UserPreferencesV1::default()
            },
            viewport,
            focused: Some(&action_id),
        });
        let control = frame
            .controls
            .iter()
            .find(|control| control.action_id == action_id)
            .unwrap_or_else(|| panic!("missing rendered UI scale action at {viewport:?}"));
        assert_eq!(control.label, "UI scale: 130%");
        assert_eq!(
            control.action,
            PlatformUiAction::Shell(ShellUiAction::CycleUiScale)
        );
        assert_eq!(
            control.description,
            "Cycle UI scale through 85%, 100%, 115%, and 130%."
        );
        assert!(control.focused);
        assert_eq!(
            frame
                .hit_test(control.bounds.center())
                .map(|hit| &hit.action_id),
            Some(&action_id)
        );
        let semantic = frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.action_id.as_ref() == Some(&action_id))
            .expect("UI scale action must have a semantic control");
        assert_eq!(semantic.name, control.label);
        assert_eq!(semantic.description, control.description);
        assert_eq!(semantic.bounds, Some(control.bounds.into()));
        qualify_sdf_frame(&model, &frame);
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        let exact_ink = output
            .text_runs
            .iter()
            .filter(|run| run.semantic_id == control.semantic_id)
            .map(|run| run.exact_text.as_str())
            .collect::<String>();
        assert_eq!(exact_ink, control.label);
    }
}

#[test]
fn accepted_branch_history_and_overflow_timeline_actions_are_reachable() {
    use nyon_workshop_core::{CreatorBatchV1, WorkshopHistory};
    let mut history = WorkshopHistory::from_seed_u64(core_catalog(), 46);
    for index in 0..24 {
        history
            .submit(CreatorBatchV1 {
                expected_cursor: None,
                expected_tick: WorkshopTick(0),
                operations: vec![CreatorOpV1::CreateSystem {
                    local: BatchLocalId(0),
                    name: name(&format!("Alternative {index:02}")),
                    position: GalaxyPointV1::new(0, 0).unwrap(),
                }],
            })
            .unwrap();
        history.undo().unwrap();
    }
    let redo = history.redo_choices();
    assert_eq!(redo.len(), 24);
    let session = nyon::workshop::session::WorkshopSession::new(history);
    let model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            redo_children: &redo,
            ..WorkshopUiContext::default()
        },
    );
    assert!(model.branches.len() >= 24);
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        let mut view = WorkshopViewState::default();
        for target in model
            .branches
            .iter()
            .map(|branch| &branch.control)
            .chain(&model.timeline.controls)
        {
            assert!(view.reveal_action(&model, &layout, &target.action_id));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(&target.action_id),
            );
            qualify_sdf_frame(&model, &frame);
            let control = frame
                .controls
                .iter()
                .find(|control| control.action_id == target.action_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{} not revealed at {width}x{height}@{scale}",
                        target.action_id
                    )
                });
            if control.enabled {
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    target.action_id
                );
                assert_eq!(
                    model.activate(&target.action_id, InputModality::Keyboard),
                    model.activate(&target.action_id, InputModality::Pointer)
                );
            }
        }
    }
}

#[test]
fn wide_outliner_reveal_materializes_actual_first_middle_and_last_systems() {
    let current = sixty_four_system_snapshot();
    let model = WorkshopUiModel::build(&current, WorkshopUiContext::default());
    assert_eq!(model.outliner.len(), 64);
    let metrics = AtlasMetrics::embedded().unwrap();

    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1920.0, 1080.0), scale).unwrap();
        for index in [0, model.outliner.len() / 2, model.outliner.len() - 1] {
            let target = &model.outliner[index].control.action_id;
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, target));

            for rebuild in 0..2 {
                assert!(view.reveal_action(&model, &layout, target));
                let frame = build_workshop_platform_frame_for_view(
                    &model,
                    layout.clone(),
                    &view,
                    Some(target),
                );
                let control = frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == target)
                    .unwrap_or_else(|| {
                        panic!(
                            "wide outliner index {index} disappeared at {scale} on rebuild {rebuild}; start={}",
                            view.outliner_start
                        )
                    });
                assert!(frame.action(target).is_some());
                assert_eq!(
                    frame
                        .hit_test(control.bounds.center())
                        .map(|hit| &hit.action_id),
                    Some(target)
                );
                assert!(
                    frame
                        .semantics
                        .node(control.semantic_id.as_str())
                        .is_some_and(|node| node.visible
                            && node.enabled
                            && node.action_id.as_ref() == Some(target))
                );
                let output = build_platform_ui_batch(&frame, &metrics).unwrap();
                assert!(output.text_runs.iter().any(|run| {
                    run.semantic_id == control.semantic_id
                        && run.emitted_glyph_range[1] > run.emitted_glyph_range[0]
                }));
            }
        }
    }
}

#[test]
fn shell_actions_use_the_same_exact_label_and_typed_icon_contract() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability, MainMenuRoute},
        preferences::UserPreferencesV1,
        ui::platform::{ShellPlatformInput, build_shell_platform_frame},
    };
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let capabilities = [
        MainMenuRoute::NewWorkshop,
        MainMenuRoute::Continue,
        MainMenuRoute::ClassicSector,
        MainMenuRoute::Settings,
        MainMenuRoute::Credits,
    ]
    .map(|route| MainMenuCapability {
        route,
        enabled: true,
    });
    for screen in [
        ClientScreen::MainMenu,
        ClientScreen::Settings,
        ClientScreen::RecoverableError,
    ] {
        for credits_visible in [false, true] {
            let frame = build_shell_platform_frame(ShellPlatformInput {
                screen,
                capabilities: &capabilities,
                credits_visible,
                recovery_message: Some("Recovery available"),
                continue_available: true,
                backend: None,
                preferences: UserPreferencesV1::default(),
                viewport: Vec2::new(800.0, 600.0),
                focused: None,
            });
            assert_action_witnesses(&model, &frame);
        }
    }
}

#[test]
fn platform_sdf_preserves_semantic_sources_layers_and_measured_text_contracts() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let metrics = AtlasMetrics::embedded().unwrap();

    for id in [
        "save.status",
        "diagnostics.backend",
        "diagnostics.catalog",
        "diagnostics.digest",
        "diagnostics.tick",
    ] {
        let node = frame.semantics.node(id).unwrap();
        let record = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("missing visible record for {id}"));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
    }

    let inspector_id = model.inspector.text_records()[1].semantic_id.clone();
    let output = build_platform_ui_batch(&frame, &metrics).unwrap();
    assert!(output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::InspectorRowFill
            && witness.owning_node.as_ref() == Some(&inspector_id)
    }));

    frame.visible_nodes[0].display_text = "A  B".to_owned();
    frame.visible_nodes[0].overflow = PlatformTextOverflow::Wrap;
    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 200.0, 80.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    frame.visible_nodes[0].prewrapped_lines = None;
    let whitespace = build_platform_ui_batch(&frame, &metrics).unwrap();
    let exact = whitespace
        .text_runs
        .iter()
        .filter(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .map(|run| run.exact_text.as_str())
        .collect::<String>();
    assert_eq!(exact, "A  B");

    frame.visible_nodes[0].overflow = PlatformTextOverflow::SingleLineEllipsis;
    frame.visible_nodes[0].display_text = "A very long measured platform title".to_owned();
    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 60.0, 40.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    let ellipsized = build_platform_ui_batch(&frame, &metrics).unwrap();
    let run = ellipsized
        .text_runs
        .iter()
        .find(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .unwrap();
    assert!(run.exact_text.ends_with('.'));
    assert!(run.logical_advance <= run.bounds.width());

    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 1.0, 40.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    let narrow = build_platform_ui_batch(&frame, &metrics).unwrap();
    let run = narrow
        .text_runs
        .iter()
        .find(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .unwrap();
    assert!(run.logical_advance <= run.bounds.width());
}

#[test]
fn platform_sdf_modal_suppresses_nonmodal_records_and_full_surface_is_typed() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            pending_removal: Some(entity(2)),
            ..WorkshopUiContext::default()
        },
    );
    let frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for node in frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| node.id.as_str().starts_with("removal.blocker."))
    {
        assert!(
            frame
                .visible_nodes
                .iter()
                .any(|record| record.semantic_id == node.id)
        );
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
        );
    }
    assert!(
        output
            .panel_witnesses
            .iter()
            .all(|witness| witness.role != PlatformPanelRole::FullSurface)
    );
    let modal = frame.modal.unwrap();
    let dialog = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)
        .unwrap();
    let mut dialog_ids = BTreeSet::new();
    let mut pending = vec![dialog];
    while let Some(node) = pending.pop() {
        dialog_ids.insert(node.id.clone());
        pending.extend(&node.children);
    }
    let records = frame
        .visible_nodes
        .iter()
        .map(|record| record.semantic_id.clone())
        .collect::<BTreeSet<_>>();
    assert!(records.is_subset(&dialog_ids));
    let rendered = output
        .text_runs
        .iter()
        .map(|run| run.semantic_id.clone())
        .chain(output.icon_runs.iter().map(|run| run.semantic_id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(records, rendered);
    let mut rebuilt = frame.clone();
    rebuilt.append_status_line("Saving before exit");
    rebuilt.reconcile_focused(frame.focus_order().first());
    assert_eq!(
        records,
        rebuilt
            .visible_nodes
            .iter()
            .map(|record| record.semantic_id.clone())
            .collect::<BTreeSet<_>>()
    );
    assert_action_witnesses(&model, &rebuilt);
    assert!(frame.semantics.nodes_depth_first().into_iter().all(|node| {
        node.role == SemanticRole::Application
            || dialog_ids.contains(&node.id)
            || (!node.visible && !node.enabled)
    }));
    assert!(
        output
            .text_runs
            .iter()
            .all(|run| modal.contains_rect(run.bounds))
    );
    assert!(
        output
            .icon_runs
            .iter()
            .all(|run| modal.contains_rect(run.bounds))
    );
    let modal_panel_end = output
        .panel_witnesses
        .iter()
        .find(|witness| witness.role == PlatformPanelRole::Modal)
        .unwrap()
        .emitted_panel_range[1];
    assert!(output.panel_witnesses.iter().all(|witness| {
        witness.role != PlatformPanelRole::ControlFill
            || (modal.contains_rect(witness.bounds)
                && witness.emitted_panel_range[0] >= modal_panel_end)
    }));

    let guide = nyon::ui::guide::build_guide_frame(
        nyon::ui::guide::GuideLocation::for_screen(
            nyon::app::client_runtime::ClientScreen::GalaxyWorkshop,
        ),
        Vec2::new(800.0, 600.0),
        false,
        None,
    );
    let guide_output =
        build_platform_ui_batch(&guide.platform, &AtlasMetrics::embedded().unwrap()).unwrap();
    assert_action_witnesses(&model, &guide.platform);
    assert!(guide_output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::FullSurface
            && witness.bounds == guide.platform.layout.viewport
    }));
}

#[test]
fn platform_sdf_capacity_failures_return_no_partial_output() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut glyph_frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let template = glyph_frame.visible_nodes[0].clone();
    glyph_frame.controls.clear();
    glyph_frame.sighted_text.clear();
    glyph_frame.visible_nodes = (0..8_300)
        .map(|index| {
            let mut record = template.clone();
            record.semantic_id =
                nyon::ui::accessibility::SemanticNodeId::new(format!("capacity.glyph.{index}"));
            record.display_text = "X".to_owned();
            record.prewrapped_lines = None;
            record
        })
        .collect();
    assert!(matches!(
        build_platform_ui_batch(&glyph_frame, &metrics),
        Err(UiBatchError::Capacity { requested: 8_193 })
    ));

    let mut panel_frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    panel_frame.visible_nodes.clear();
    let template = panel_frame.controls[0].clone();
    panel_frame.controls = (0..520).map(|_| template.clone()).collect();
    assert!(matches!(
        build_platform_ui_batch(&panel_frame, &metrics),
        Err(UiBatchError::PanelCapacity { .. })
    ));

    let mut exact_glyph_frame =
        build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    exact_glyph_frame.controls.clear();
    exact_glyph_frame.visible_nodes.truncate(1);
    exact_glyph_frame.visible_nodes[0].display_text = "X".repeat(8_192);
    exact_glyph_frame.visible_nodes[0].overflow = PlatformTextOverflow::Wrap;
    exact_glyph_frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 1_000_000.0, 40.0);
    exact_glyph_frame.visible_nodes[0].clip = Some(exact_glyph_frame.visible_nodes[0].bounds);
    exact_glyph_frame.viewport = Vec2::new(1_000_000.0, 720.0);
    let exact_glyphs = build_platform_ui_batch(&exact_glyph_frame, &metrics).unwrap();
    assert_eq!(exact_glyphs.batch.glyphs().len(), 8_192);

    let mut exact_panel_frame =
        build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    exact_panel_frame.visible_nodes.clear();
    let control = exact_panel_frame.controls[0].clone();
    exact_panel_frame.controls.clear();
    let base_panels = build_platform_ui_batch(&exact_panel_frame, &metrics)
        .unwrap()
        .batch
        .panels()
        .len();
    exact_panel_frame.controls = (base_panels..512).map(|_| control.clone()).collect();
    let exact_panels = build_platform_ui_batch(&exact_panel_frame, &metrics).unwrap();
    assert_eq!(exact_panels.batch.panels().len(), 512);
}

#[test]
fn platform_sdf_identity_and_exact_text_survive_presentation_preferences() {
    let snapshot = snapshot();
    let normal = WorkshopUiModel::build(&snapshot, WorkshopUiContext::default());
    let altered = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            reduced_motion: true,
            high_contrast: true,
            ..WorkshopUiContext::default()
        },
    );
    let viewport = Vec2::new(1_280.0, 720.0);
    let normal_frame = build_workshop_platform_frame(&normal, viewport, None);
    let focus = altered.save.save_control.action_id.clone();
    let mut altered_frame = build_workshop_platform_frame(&altered, viewport, Some(&focus));
    altered_frame.reconcile_focused(Some(&focus));
    let normal_records = normal_frame
        .visible_nodes
        .iter()
        .map(|record| (record.semantic_id.clone(), record.display_text.clone()))
        .collect::<BTreeSet<_>>();
    let altered_records = altered_frame
        .visible_nodes
        .iter()
        .map(|record| (record.semantic_id.clone(), record.display_text.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(normal_records, altered_records);

    let output =
        build_platform_ui_batch(&altered_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    assert!(output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::ContrastBorder
            && witness.owning_node.as_ref()
                == altered_frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == focus)
                    .map(|control| &control.semantic_id)
    }));
    let border = output
        .panel_witnesses
        .iter()
        .find(|witness| witness.role == PlatformPanelRole::ContrastBorder)
        .unwrap();
    assert_eq!(
        border.emitted_panel_range[1] - border.emitted_panel_range[0],
        4
    );
    let strips =
        &output.batch.panels()[border.emitted_panel_range[0]..border.emitted_panel_range[1]];
    assert!(
        strips
            .iter()
            .all(|strip| strip.rect[2] <= 2.0 || strip.rect[3] <= 2.0)
    );

    let normal_output =
        build_platform_ui_batch(&normal_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let normal_identity = normal_output
        .text_runs
        .iter()
        .map(|run| (run.semantic_id.clone(), run.exact_text.clone()))
        .chain(
            normal_output
                .icon_runs
                .iter()
                .map(|run| (run.semantic_id.clone(), format!("icon:{:?}", run.icon))),
        )
        .collect::<BTreeSet<_>>();
    let altered_identity = output
        .text_runs
        .iter()
        .map(|run| (run.semantic_id.clone(), run.exact_text.clone()))
        .chain(
            output
                .icon_runs
                .iter()
                .map(|run| (run.semantic_id.clone(), format!("icon:{:?}", run.icon))),
        )
        .collect::<BTreeSet<_>>();
    assert_eq!(normal_identity, altered_identity);
}

#[test]
fn high_contrast_selection_has_owner_linked_outline_with_and_without_focus() {
    let mut current = snapshot();
    current.speed = WorkshopSpeed::One;
    let selected_entity = current.state.systems.keys().next().copied();
    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            reduced_motion: true,
            high_contrast: true,
            selected_entity,
            ..WorkshopUiContext::default()
        },
    );
    let selected_actions = [
        model.preferences.reduced_motion_control.action_id.clone(),
        model
            .timeline
            .controls
            .iter()
            .find(|control| control.selected)
            .expect("active speed control")
            .action_id
            .clone(),
        model
            .outliner
            .iter()
            .find(|entry| entry.control.selected)
            .expect("selected outliner row")
            .control
            .action_id
            .clone(),
    ];
    let metrics = AtlasMetrics::embedded().unwrap();

    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1920.0, 1080.0), scale).unwrap();
        for action in &selected_actions {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, action));
            for focused in [false, true] {
                let focus = focused.then_some(action);
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, focus);
                let control = frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == action)
                    .unwrap();
                assert!(control.selected);
                assert!(control.enabled, "selected cue fixture disabled {action}");
                assert_eq!(control.focused, focused);
                let output = build_platform_ui_batch(&frame, &metrics).unwrap();
                let outline = output
                    .panel_witnesses
                    .iter()
                    .find(|witness| {
                        witness.role == PlatformPanelRole::ContrastBorder
                            && witness.owning_node.as_ref() == Some(&control.semantic_id)
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "selected control {action} has no non-color outline at {scale}, focused={focused}"
                        )
                    });
                assert_eq!(outline.bounds, control.bounds);
                assert_eq!(
                    outline.emitted_panel_range[1] - outline.emitted_panel_range[0],
                    4
                );

                let mut overlay = nyon::ui::platform::PlatformPrimitiveOverlay::default();
                frame.append_primitive_overlay(&mut overlay).unwrap();
                assert_eq!(
                    overlay.witnesses.iter().any(|witness| {
                        witness.kind == nyon::ui::platform::PlatformDecorationKind::Focus
                            && witness.owning_node.as_ref() == Some(&control.semantic_id)
                    }),
                    focused,
                    "focus remains a separate owned primitive cue for {action}: {:?}",
                    overlay.witnesses
                );
            }
        }
    }
}

#[test]
fn platform_sdf_materializes_empty_and_creator_diagnostic_semantics() {
    let mut empty_snapshot = snapshot();
    empty_snapshot.state = WorkshopStateV1::default();
    let empty = WorkshopUiModel::build(&empty_snapshot, WorkshopUiContext::default());
    let empty_frame = build_workshop_platform_frame(&empty, Vec2::new(1_280.0, 720.0), None);
    let empty_output =
        build_platform_ui_batch(&empty_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let empty_node = empty_frame.semantics.node("outliner.empty").unwrap();
    let empty_record = empty_frame
        .visible_nodes
        .iter()
        .find(|record| record.semantic_id == empty_node.id)
        .unwrap();
    assert_eq!(empty_record.semantic_name, empty_node.name);
    assert_eq!(empty_record.semantic_description, empty_node.description);
    assert!(
        empty_output
            .text_runs
            .iter()
            .any(|run| run.semantic_id == empty_node.id)
    );

    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    draft.set_text("name", "").unwrap();
    let creator = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let creator_frame = build_workshop_platform_frame(&creator, Vec2::new(1_280.0, 720.0), None);
    let creator_output =
        build_platform_ui_batch(&creator_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for node in creator_frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.id.as_str().starts_with("creator.preview.")
                || node.id.as_str() == "creator.validation"
                || node.id.as_str().starts_with("creator-field.")
        })
    {
        let record = creator_frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("missing creator record {}", node.id));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
        assert!(
            creator_output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
                || creator_output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == node.id)
        );
    }
}

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
fn bundled_nonfixed_ascii_ink_fits_owned_records_at_required_scales() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let text = (b'!'..=b'~').map(char::from).collect::<String>();
    for (_, _, scale) in SDF_QUALIFICATION_MATRIX {
        let mut frame = build_workshop_platform_frame(&model, Vec2::new(1280.0, 720.0), None);
        frame.layout.ui_scale = scale;
        frame.visible_nodes.truncate(1);
        let record = &mut frame.visible_nodes[0];
        record.display_text = text.clone();
        record.prewrapped_lines = None;
        record.role = PlatformTextRole::Body;
        record.overflow = PlatformTextOverflow::Wrap;
        record.bounds = nyon::ui::platform::PlatformRect::from_xywh(20.0, 100.0, 240.0, 500.0);
        record.clip = Some(record.bounds);
        let owned_bounds = record.bounds;
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        assert_eq!(
            output
                .text_runs
                .iter()
                .map(|run| run.exact_text.as_str())
                .collect::<String>(),
            text
        );
        for run in &output.text_runs {
            assert_full_text_ink(run, &output.batch);
            assert!(run.clip.contains_rect(run.bounds));
            assert!(owned_bounds.contains_rect(run.clip));
        }
    }
}

#[test]
fn every_creator_operation_has_a_non_spatial_semantic_tool() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mapped = operation_examples()
        .iter()
        .map(CreatorTool::for_operation)
        .collect::<BTreeSet<_>>();

    assert_eq!(mapped, CreatorTool::ALL.into_iter().collect());
    assert_eq!(model.tool_palette.len(), CreatorTool::ALL.len());
    for tool in CreatorTool::ALL {
        let control = &model
            .tool_palette
            .iter()
            .find(|candidate| candidate.tool == tool)
            .unwrap()
            .control;
        assert!(control.enabled);
        assert!(model.semantics.nodes_depth_first().iter().any(|node| {
            node.role == SemanticRole::Button && node.action_id.as_ref() == Some(&control.action_id)
        }));
        assert_eq!(
            model.activate(&control.action_id, InputModality::Keyboard),
            Some(WorkshopUiIntent::OpenCreatorForm {
                tool,
                subject: None,
            })
        );
    }
}

#[test]
fn pointer_and_keyboard_activation_produce_identical_typed_intents() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(4)),
            redo_children: &[revision(9)],
            pending_removal: Some(entity(12)),
            ..WorkshopUiContext::default()
        },
    );

    for control in model.controls() {
        assert_eq!(
            model.activate(&control.action_id, InputModality::Pointer),
            model.activate(&control.action_id, InputModality::Keyboard),
            "input paths diverged for {}",
            control.action_id
        );
    }
    let focus = FocusManager::new(model.focus_order().iter().cloned());
    assert_eq!(focus.base_order(), model.focus_order());
    assert_eq!(
        model.focus_order().iter().collect::<BTreeSet<_>>().len(),
        model.focus_order().len()
    );
}

#[test]
fn every_creator_form_exposes_a_concrete_typed_batch_or_fixed_prerequisite() {
    let mut current = snapshot();
    current.state.systems.insert(
        entity(14),
        SystemV1 {
            name: name("Quench"),
            position: GalaxyPointV1::new(2_048, 1_024).unwrap(),
        },
    );
    current.state_digest = current.state.digest();
    current.active_view.digest = current.state_digest;

    let selections = BTreeMap::from([
        (CreatorTool::CreateStar, entity(2)),
        (CreatorTool::CreateWorld, entity(3)),
        (CreatorTool::ConnectLane, entity(14)),
        (CreatorTool::CreateDeposit, entity(10)),
        (CreatorTool::SetOwner, entity(10)),
        (CreatorTool::PlaceIndustry, entity(10)),
        (CreatorTool::ConnectRoute, entity(4)),
        (CreatorTool::SetIndustryEnabled, entity(6)),
        (CreatorTool::RenameObject, entity(4)),
        (CreatorTool::RemoveObject, entity(12)),
        (CreatorTool::ScheduleHazard, entity(7)),
        (CreatorTool::CancelHazard, entity(12)),
    ]);

    for tool in CreatorTool::ALL {
        let selected = selections.get(&tool).copied();
        let batch = default_creator_batch(&current, selected, tool)
            .unwrap_or_else(|error| panic!("{tool:?} did not produce a batch: {error}"));
        assert_eq!(batch.expected_cursor, current.active_view.view_cursor);
        assert_eq!(batch.expected_tick, current.active_view.tick);
        assert_eq!(batch.operations.len(), 1);
        assert_eq!(CreatorTool::for_operation(&batch.operations[0]), tool);

        let model = WorkshopUiModel::build(
            &current,
            WorkshopUiContext {
                selected_entity: selected,
                creator_form: Some(tool),
                ..WorkshopUiContext::default()
            },
        );
        let form = model.creator_form.as_ref().unwrap();
        assert!(form.submit_control.enabled, "{tool:?}");
        let keyboard = model.activate(&form.submit_control.action_id, InputModality::Keyboard);
        let pointer = model.activate(&form.submit_control.action_id, InputModality::Pointer);
        assert_eq!(keyboard, pointer);
        let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(
            submitted,
        ))) = keyboard
        else {
            panic!("{tool:?} form did not submit a typed creator batch");
        };
        assert_eq!(submitted, batch);
        assert!(model.semantics.node("workshop.creator-dialog").is_some());
    }
}

#[test]
fn creator_form_focus_controls_are_unique_and_menu_exit_is_typed() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(4)),
            creator_form: Some(CreatorTool::CreateDeposit),
            ..WorkshopUiContext::default()
        },
    );
    assert_eq!(
        model.focus_order().iter().collect::<BTreeSet<_>>().len(),
        model.focus_order().len()
    );
    assert!(model.focus_order().contains(&model.menu_control.action_id));
    assert!(
        model.focus_order().contains(
            &model
                .creator_form
                .as_ref()
                .unwrap()
                .cancel_control
                .action_id
        )
    );
    assert!(
        model.focus_order().contains(
            &model
                .creator_form
                .as_ref()
                .unwrap()
                .submit_control
                .action_id
        )
    );
    assert_eq!(
        model.activate(&model.menu_control.action_id, InputModality::Keyboard),
        Some(WorkshopUiIntent::ReturnToMainMenu)
    );
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

#[test]
fn undo_and_redo_disable_while_running_or_during_store_replacement() {
    let mut running = snapshot();
    running.speed = WorkshopSpeed::Four;
    let running_model = WorkshopUiModel::build(
        &running,
        WorkshopUiContext {
            redo_children: &[revision(9)],
            ..WorkshopUiContext::default()
        },
    );
    assert!(!control(&running_model, "timeline.undo").enabled);
    assert!(
        !control(
            &running_model,
            &format!("timeline.redo.{}", "09".repeat(32))
        )
        .enabled
    );

    let mut replacing = snapshot();
    replacing.store.commit_pending = true;
    let replacing_model = WorkshopUiModel::build(
        &replacing,
        WorkshopUiContext {
            redo_children: &[revision(9)],
            ..WorkshopUiContext::default()
        },
    );
    assert!(!control(&replacing_model, "timeline.undo").enabled);
    assert!(
        !control(
            &replacing_model,
            &format!("timeline.redo.{}", "09".repeat(32))
        )
        .enabled
    );
    assert_eq!(
        replacing_model.activate(
            &control(&replacing_model, "timeline.undo").action_id,
            InputModality::Keyboard,
        ),
        None
    );
}

#[test]
fn branch_chooser_distinguishes_the_immutable_head_from_the_view_cursor() {
    let mut snapshot = snapshot();
    snapshot.active_view.view_cursor = Some(revision(1));
    let model = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            redo_children: &[revision(2)],
            ..WorkshopUiContext::default()
        },
    );
    let main = model
        .branches
        .iter()
        .find(|choice| choice.selected)
        .unwrap();

    assert_eq!(main.head, Some(revision(2)));
    assert_eq!(main.selected_cursor, Some(revision(1)));
    assert!(main.browsing_history);
    assert!(main.control.description.contains("behind head"));
    assert_eq!(model.timeline.redo_children, vec![revision(2)]);
}

#[test]
fn removal_confirmation_lists_ordered_blockers_and_never_cascades() {
    let state = populated_state();
    assert_eq!(
        removal_blockers(&state, entity(2)),
        vec![entity(3), entity(4), entity(7)]
    );
    let blocked = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(2)),
            pending_removal: Some(entity(2)),
            ..WorkshopUiContext::default()
        },
    );
    let dialog = blocked.removal_confirmation.as_ref().unwrap();
    assert_eq!(
        dialog
            .blockers
            .iter()
            .map(|blocker| blocker.entity)
            .collect::<Vec<_>>(),
        vec![entity(3), entity(4), entity(7)]
    );
    assert!(!dialog.confirm_control.enabled);
    assert_eq!(
        blocked.activate(&dialog.confirm_control.action_id, InputModality::Keyboard),
        None
    );

    let removable = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(12)),
            pending_removal: Some(entity(12)),
            ..WorkshopUiContext::default()
        },
    );
    let dialog = removable.removal_confirmation.as_ref().unwrap();
    assert!(dialog.blockers.is_empty());
    assert!(dialog.confirm_control.enabled);
    let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(batch))) =
        removable.activate(&dialog.confirm_control.action_id, InputModality::Keyboard)
    else {
        panic!("removal confirmation did not create a typed creator batch");
    };
    assert_eq!(batch.expected_cursor, Some(revision(2)));
    assert_eq!(batch.expected_tick, WorkshopTick(15));
    assert_eq!(
        batch.operations,
        vec![CreatorOpV1::RemoveObject {
            target: ObjectRefV1::Existing(entity(12)),
        }]
    );
}

#[test]
fn backend_catalog_save_and_accessibility_preferences_are_diagnostic_only() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            backend: Some(BackendKind::WebGl2Low),
            catalog_hash: Some(CatalogHash([0xAB; 32])),
            reduced_motion: true,
            high_contrast: true,
            ..WorkshopUiContext::default()
        },
    );

    assert_eq!(model.diagnostics.backend, "WEBGL2 LOW");
    assert_eq!(model.diagnostics.catalog_hash, "ab".repeat(32));
    assert_eq!(model.save.slot, Some(4));
    assert_eq!(model.save.generation, Some(7));
    assert!(model.save.dirty);
    assert!(model.preferences.reduced_motion);
    assert!(model.preferences.high_contrast);
    assert!(model.preferences.reduced_motion_control.selected);
    assert!(model.preferences.high_contrast_control.selected);
    assert_eq!(
        model.activate(
            &model.preferences.reduced_motion_control.action_id,
            InputModality::Keyboard,
        ),
        Some(WorkshopUiIntent::SetReducedMotion(false))
    );
    assert_eq!(
        model
            .semantics
            .node("diagnostics.backend")
            .unwrap()
            .description,
        "WEBGL2 LOW"
    );
    assert_eq!(
        model
            .semantics
            .node("diagnostics.catalog")
            .unwrap()
            .description,
        "ab".repeat(32)
    );
}

#[test]
fn status_announcements_use_codes_without_echoing_untrusted_error_content() {
    const RAW: &str = "UNTRUSTED IMPORTED NAME <script>alert(1)</script>";
    let mut snapshot = snapshot();
    snapshot.diagnostics = vec![
        WorkshopDiagnostic {
            code: WorkshopDiagnosticCode::ArchiveRejected,
            message: RAW.to_owned(),
        },
        WorkshopDiagnostic {
            code: WorkshopDiagnosticCode::StoreRejected,
            message: RAW.to_owned(),
        },
    ];
    snapshot.store.status = RAW.to_owned();
    let model = WorkshopUiModel::build(&snapshot, WorkshopUiContext::default());

    assert!(
        model
            .semantics
            .announcements
            .iter()
            .all(|announcement| !announcement.message.contains(RAW))
    );
    assert!(model.semantics.announcements.iter().any(|announcement| {
        announcement.kind == AnnouncementKind::Error
            && announcement.code == "archive-rejected"
            && announcement.message == "The Workshop archive could not be validated."
    }));
    assert!(model.semantics.announcements.iter().any(|announcement| {
        announcement.kind == AnnouncementKind::Status
            && announcement.code == "workshop-store-status"
            && announcement.message == "Workshop storage status changed"
    }));
    assert_eq!(model.save.status, "Workshop storage status changed");
}

#[test]
fn creator_dialog_disables_pointer_controls_outside_its_modal_scope() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            ..WorkshopUiContext::default()
        },
    );
    let frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let enabled = frame
        .controls
        .iter()
        .filter(|control| control.enabled)
        .map(|control| control.action_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        enabled,
        BTreeSet::from([
            "creator.cancel",
            "creator.field.name",
            "creator.field.x",
            "creator.field.y",
            "creator.submit",
        ])
    );

    let menu = frame
        .controls
        .iter()
        .find(|control| control.action_id.as_str() == "workshop.main-menu")
        .unwrap();
    assert!(!menu.enabled);
    assert!(
        frame
            .hit_test((menu.bounds.min + menu.bounds.max) * 0.5)
            .is_none()
    );
    assert_eq!(
        frame
            .focus_order()
            .iter()
            .map(|action| action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "creator.field.name",
            "creator.field.x",
            "creator.field.y",
            "creator.cancel",
            "creator.submit",
        ]
    );
    assert!(frame.action(&menu.action_id).is_none());
    let dialog = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)
        .unwrap();
    fn subtree_ids(
        node: &nyon::ui::accessibility::SemanticNode,
        ids: &mut BTreeSet<nyon::ui::accessibility::SemanticNodeId>,
    ) {
        ids.insert(node.id.clone());
        for child in &node.children {
            subtree_ids(child, ids);
        }
    }
    let mut modal_ids = BTreeSet::new();
    subtree_ids(dialog, &mut modal_ids);
    let records = frame
        .visible_nodes
        .iter()
        .map(|record| record.semantic_id.clone())
        .collect::<BTreeSet<_>>();
    assert!(
        records.is_subset(&modal_ids),
        "nonmodal visible records: {:?}",
        records.difference(&modal_ids).collect::<Vec<_>>()
    );
    let visible = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.visible
                && node.role != SemanticRole::Application
                && node.role != SemanticRole::Group
                && node.role != SemanticRole::Dialog
        })
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(records, visible);
    let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let rendered = output
        .text_runs
        .iter()
        .map(|run| run.semantic_id.clone())
        .chain(output.icon_runs.iter().map(|run| run.semantic_id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(records, rendered);
    for control in frame.controls.iter().filter(|control| control.enabled) {
        assert!(records.contains(&control.semantic_id));
        assert!(modal_ids.contains(&control.semantic_id));
    }
}

#[test]
fn creator_draft_edits_are_typed_client_state_until_submit() {
    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    assert_eq!(draft.field("name").unwrap().kind, CreatorFieldKind::Text);
    assert_eq!(draft.field("x").unwrap().kind, CreatorFieldKind::Integer);

    draft.set_text("name", "Ember Reach").unwrap();
    draft.set_text("x", "32768").unwrap();
    draft.set_text("y", "-4096").unwrap();
    let submitted = draft.to_batch().unwrap();

    assert_eq!(
        current.state,
        snapshot().state,
        "draft editing mutated authority"
    );
    assert_eq!(submitted.expected_cursor, defaults.expected_cursor);
    assert_eq!(submitted.expected_tick, defaults.expected_tick);
    assert!(matches!(
        &submitted.operations[0],
        CreatorOpV1::CreateSystem { name, position, .. }
            if name.as_str() == "Ember Reach"
                && position.x.get() == 32_768
                && position.y.get() == -4_096
    ));

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let form = model.creator_form.as_ref().unwrap();
    assert_eq!(form.fields.len(), 3);
    assert!(form.submit_control.enabled);
    assert_eq!(
        model
            .semantics
            .node("creator-field.control.creator.field.name")
            .unwrap()
            .role,
        SemanticRole::TextInput
    );
    let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(actual))) =
        model.activate(&form.submit_control.action_id, InputModality::Keyboard)
    else {
        panic!("edited draft did not produce a typed submit intent");
    };
    assert_eq!(actual, submitted);
}

#[test]
fn creator_draft_validation_keeps_invalid_text_out_of_the_mailbox() {
    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    draft.set_text("name", "").unwrap();

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let form = model.creator_form.as_ref().unwrap();
    assert!(!form.submit_control.enabled);
    assert!(form.validation_message.as_deref().unwrap().contains("Name"));
    assert_eq!(
        model.activate(&form.submit_control.action_id, InputModality::Pointer),
        None
    );
}

#[test]
fn creator_choice_fields_cycle_through_typed_available_objects() {
    let mut current = snapshot();
    current.state.systems.insert(
        entity(14),
        SystemV1 {
            name: name("Quench"),
            position: GalaxyPointV1::new(2_048, 1_024).unwrap(),
        },
    );
    current.state_digest = current.state.digest();
    current.active_view.digest = current.state_digest;
    let defaults =
        default_creator_batch(&current, Some(entity(14)), CreatorTool::ConnectLane).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    let before = draft.field("a").unwrap().value.clone();
    assert_eq!(draft.field("a").unwrap().kind, CreatorFieldKind::Choice);
    assert!(draft.field("a").unwrap().choices.len() >= 3);
    draft.cycle("a", 1).unwrap();
    assert_ne!(draft.field("a").unwrap().value, before);
    assert_ne!(draft.to_batch().unwrap(), defaults);

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::ConnectLane),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let control = &model.creator_form.as_ref().unwrap().fields[0].control;
    let keyboard = model.activate(&control.action_id, InputModality::Keyboard);
    let pointer = model.activate(&control.action_id, InputModality::Pointer);
    assert_eq!(keyboard, pointer);
    assert!(matches!(
        keyboard,
        Some(WorkshopUiIntent::EditCreatorField { cycle: true, .. })
    ));
}

fn control<'a>(model: &'a WorkshopUiModel, id: &str) -> &'a nyon::ui::workshop::WorkshopControl {
    model
        .controls()
        .find(|control| control.action_id.as_str() == id)
        .unwrap_or_else(|| panic!("missing control {id}"))
}

#[test]
fn workshop_layout_preserves_a_real_canvas_at_supported_sizes() {
    for (viewport, scale, expected) in [
        (Vec2::new(723.0, 802.0), 1.15, WorkshopLayoutMode::Compact),
        (Vec2::new(1280.0, 720.0), 1.30, WorkshopLayoutMode::Medium),
        (Vec2::new(1280.0, 720.0), 1.00, WorkshopLayoutMode::Wide),
        (Vec2::new(1440.0, 900.0), 1.00, WorkshopLayoutMode::Wide),
    ] {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        assert_eq!(layout.mode, expected);
        assert!(layout.canvas.width() >= 320.0, "{layout:?}");
        assert!(layout.canvas.height() >= 300.0, "{layout:?}");
        assert!(
            layout
                .chrome
                .iter()
                .all(|rect| !rect.overlaps(layout.canvas)),
            "{layout:?}"
        );
        assert!(
            layout
                .chrome
                .iter()
                .all(|rect| layout.viewport.contains_rect(*rect)),
            "{layout:?}"
        );
    }
}

#[test]
fn workshop_layout_uses_the_exact_effective_width_boundaries() {
    for (effective_width, expected) in [
        (899.0, WorkshopLayoutMode::Compact),
        (900.0, WorkshopLayoutMode::Medium),
        (1199.0, WorkshopLayoutMode::Medium),
        (1200.0, WorkshopLayoutMode::Wide),
    ] {
        let layout = WorkshopLayout::resolve(Vec2::new(effective_width, 900.0), 1.0).unwrap();
        assert_eq!(layout.mode, expected, "effective width {effective_width}");
    }

    let compact = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), 1.30).unwrap();
    assert_eq!(compact.canvas.min.x, 0.0);
    assert_eq!(compact.canvas.max.x, 723.0);
    assert_eq!(compact.left_panel, None);
    assert_eq!(compact.right_panel, None);

    let medium = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.30).unwrap();
    assert_eq!(medium.mode, WorkshopLayoutMode::Medium);
    assert!(medium.left_panel.unwrap().width() >= 44.0);
    assert!(medium.left_panel.unwrap().width() < 100.0);
}

#[test]
fn medium_layout_uses_a_collapsed_rail_and_one_click_navigator_overlay() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.30).unwrap();
    assert_eq!(layout.mode, WorkshopLayoutMode::Medium);
    let rail = layout.left_panel.unwrap();

    let closed = build_workshop_platform_frame_for_view(
        &model,
        layout.clone(),
        &WorkshopViewState::default(),
        None,
    );
    assert_eq!(closed.drawer, None);
    let open = closed
        .controls
        .iter()
        .find(|control| control.action_id == WorkshopViewAction::OpenNavigator.action_id())
        .expect("medium navigator rail needs one persistent open action");
    assert!(rail.contains_rect(open.bounds));
    assert!(open.bounds.width() >= 44.0 && open.bounds.height() >= 44.0);
    let navigator_label = closed
        .visible_nodes
        .iter()
        .find(|record| {
            record.action_id.as_ref() == Some(&open.action_id)
                && record.icon.is_none()
                && record.display_text == "Navigator"
        })
        .expect("the icon rail needs a persistent sighted focus/help label");
    assert!(closed.layout.top_bar.contains_rect(navigator_label.bounds));
    assert!(navigator_label.bounds.width() >= 100.0);

    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
    let expanded = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
    assert_eq!(expanded.drawer, Some(layout.drawer_sheet));
    assert!(expanded.drawer.unwrap().overlaps(layout.canvas));
    assert!(expanded.controls.iter().any(|control| {
        control.action_id == WorkshopViewAction::CloseDrawer.action_id()
            && control.bounds.width() >= 44.0
            && control.bounds.height() >= 44.0
    }));
}

#[test]
fn workshop_frame_never_draws_a_full_viewport_overlay() {
    for high_contrast in [false, true] {
        let mut model = WorkshopUiModel::build(
            &snapshot(),
            WorkshopUiContext {
                high_contrast,
                ..WorkshopUiContext::default()
            },
        );
        model.preferences.high_contrast = high_contrast;
        let viewport = Vec2::new(1280.0, 720.0);
        let layout = WorkshopLayout::resolve(viewport, 1.0).unwrap();
        let frame = build_workshop_platform_frame_with_layout(&model, layout, None);
        assert_eq!(frame.background, PlatformBackground::ChromeOnly);
        assert!(
            frame
                .layout
                .chrome
                .iter()
                .all(|rect| *rect != frame.layout.viewport && !rect.overlaps(frame.layout.canvas))
        );

        let mut batch = PrimitiveBatch::default();
        frame.draw(&mut batch);
        assert!(!contains_full_viewport_quad(&batch, viewport));
    }
}

#[test]
fn responsive_workshop_chrome_reserves_controls_from_sighted_text() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    for (viewport, scale, expected_mode) in [
        (Vec2::new(723.0, 802.0), 1.15, WorkshopLayoutMode::Compact),
        (Vec2::new(723.0, 802.0), 1.30, WorkshopLayoutMode::Compact),
        (Vec2::new(816.0, 802.0), 1.00, WorkshopLayoutMode::Compact),
        (Vec2::new(1280.0, 720.0), 1.30, WorkshopLayoutMode::Medium),
    ] {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        assert_eq!(layout.mode, expected_mode);
        let frame = build_workshop_platform_frame_for_view(
            &model,
            layout,
            &WorkshopViewState::default(),
            None,
        );
        let expected_actions = [
            WorkshopViewAction::OpenCreator.action_id(),
            WorkshopViewAction::OpenNavigator.action_id(),
            model.menu_control.action_id.clone(),
            nyon::ui::accessibility::SemanticActionId::new("guide.open"),
        ];
        let reserved_controls = expected_actions
            .iter()
            .map(|action| {
                frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == action)
                    .unwrap_or_else(|| panic!("missing responsive control {action}"))
            })
            .collect::<Vec<_>>();

        assert!(frame.title_bounds().is_none());
        if expected_mode == WorkshopLayoutMode::Compact {
            assert!(frame.sighted_text_bounds().is_empty());
        } else {
            let right = frame.layout.right_panel.unwrap();
            assert!(!frame.sighted_text.is_empty());
            assert!(
                frame
                    .sighted_text
                    .iter()
                    .all(|text| right.contains_rect(text.bounds))
            );
        }
        for (index, control) in reserved_controls.iter().enumerate() {
            assert!(frame.layout.viewport.contains_rect(control.bounds));
            assert!(
                reserved_controls[index + 1..]
                    .iter()
                    .all(|other| !control.bounds.overlaps(other.bounds))
            );
        }
    }

    let wide_layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
    let wide = build_workshop_platform_frame_for_view(
        &model,
        wide_layout,
        &WorkshopViewState::default(),
        None,
    );
    let title = wide
        .title_bounds()
        .expect("Wide Workshop title remains sighted");
    assert!(title.width() > 0.0 && wide.layout.top_bar.contains_rect(title));
    assert!(
        wide.controls
            .iter()
            .filter(|control| {
                control.action_id == model.menu_control.action_id
                    || control.action_id.as_str() == "guide.open"
            })
            .all(|control| !title.overlaps(control.bounds))
    );
}

#[test]
fn compact_drawers_reveal_every_creator_tool_and_populated_outliner_row() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    for scale in [1.15, 1.30] {
        let layout = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), scale).unwrap();
        assert_eq!(layout.mode, WorkshopLayoutMode::Compact);

        let closed = build_workshop_platform_frame_for_view(
            &model,
            layout.clone(),
            &WorkshopViewState::default(),
            None,
        );
        assert!(
            closed.controls.iter().all(|control| {
                control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0
            })
        );
        for action in [
            WorkshopViewAction::OpenCreator,
            WorkshopViewAction::OpenNavigator,
        ] {
            let id = action.action_id();
            let control = closed
                .controls
                .iter()
                .find(|control| control.action_id == id)
                .unwrap_or_else(|| panic!("missing persistent action {id}"));
            assert!(control.bounds.width() >= 44.0);
            assert!(control.bounds.height() >= 44.0);
        }

        for expected in model
            .tool_palette
            .iter()
            .map(|tool| &tool.control.action_id)
        {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, expected));
            assert_eq!(view.open_drawer, Some(WorkshopDrawer::Creator));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(expected),
            );
            let control = frame
                .controls
                .iter()
                .find(|control| &control.action_id == expected)
                .unwrap_or_else(|| panic!("creator action {expected} was not materialized"));
            assert!(control.bounds.width() >= 44.0);
            assert!(control.bounds.height() >= 44.0);
        }

        assert!(!model.outliner.is_empty());
        for expected in model.outliner.iter().map(|entry| &entry.control.action_id) {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, expected));
            assert_eq!(view.open_drawer, Some(WorkshopDrawer::Navigator));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(expected),
            );
            assert!(
                frame
                    .controls
                    .iter()
                    .any(|control| &control.action_id == expected),
                "outliner action {expected} was not materialized"
            );
        }

        let mut view = WorkshopViewState::default();
        view.apply(WorkshopViewAction::OpenCreator, &model, &layout);
        let creator = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
        assert!(creator.drawer.is_some());
        assert!(creator.controls.iter().any(|control| {
            control.action_id == WorkshopViewAction::CloseDrawer.action_id()
                && control.bounds.width() >= 44.0
                && control.bounds.height() >= 44.0
        }));
        assert!(creator.controls.iter().any(|control| {
            control.action_id == WorkshopViewAction::OpenNavigator.action_id()
                && control.bounds.width() >= 44.0
                && control.bounds.height() >= 44.0
        }));
        view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
        assert_eq!(view.open_drawer, Some(WorkshopDrawer::Navigator));

        let navigator = build_workshop_platform_frame_for_view(&model, layout, &view, None);
        for control in &navigator.controls {
            assert!(
                navigator.layout.viewport.contains_rect(control.bounds),
                "{} is outside the viewport at {scale}: {:?}",
                control.action_id.as_str(),
                control.bounds
            );
        }
    }
}

#[test]
fn visible_controls_share_draw_hit_focus_and_semantic_geometry() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let layout = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), 1.30).unwrap();
    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
    let frame = build_workshop_platform_frame_for_view(&model, layout, &view, None);

    for control in &frame.controls {
        if control.enabled {
            assert_eq!(
                frame
                    .hit_test(control.bounds.center())
                    .map(|hit| &hit.action_id),
                Some(&control.action_id)
            );
        }
        assert!(frame.focus_order().contains(&control.action_id));
        let semantic = frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.action_id.as_ref() == Some(&control.action_id))
            .unwrap_or_else(|| panic!("missing semantic action {}", control.action_id));
        assert!(semantic.visible);
        assert_eq!(semantic.bounds, Some(control.bounds.into()));
    }
    for expected in model.focus_order() {
        assert!(frame.focus_order().contains(expected));

        let mut revealed = WorkshopViewState::default();
        assert!(
            revealed.reveal_action(&model, &frame.layout, expected),
            "logical focus action {expected} has no reveal behavior"
        );
        let revealed_frame = build_workshop_platform_frame_for_view(
            &model,
            frame.layout.clone(),
            &revealed,
            Some(expected),
        );
        assert!(
            revealed_frame
                .controls
                .iter()
                .any(|control| &control.action_id == expected),
            "logical focus action {expected} was not materialized"
        );
    }
}

#[test]
fn populated_workshop_markers_are_not_covered_by_opaque_chrome() {
    for high_contrast in [false, true] {
        let model = WorkshopUiModel::build(
            &snapshot(),
            WorkshopUiContext {
                high_contrast,
                ..WorkshopUiContext::default()
            },
        );
        let scene = nyon::presentation::workshop::build_workshop_scene_frame(
            &snapshot().state,
            None,
            high_contrast,
        );
        for (viewport, scale) in [
            (Vec2::new(1280.0, 720.0), 1.0),
            (Vec2::new(723.0, 802.0), 1.15),
            (Vec2::new(723.0, 802.0), 1.30),
        ] {
            let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
            let mut underlay = PrimitiveBatch::default();
            let coverage = draw_workshop_scene(&scene, &layout, &mut underlay, high_contrast);
            assert!(!coverage.world_markers.is_empty());
            assert!(!underlay.vertices().is_empty());

            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout,
                &WorkshopViewState::default(),
                None,
            );
            let mut overlay = PrimitiveBatch::default();
            frame.draw(&mut overlay);
            for marker in coverage.world_markers {
                assert!(
                    !opaque_quad_covers(&overlay, marker),
                    "opaque Workshop chrome covered world marker {marker:?} at {viewport:?}"
                );
            }
        }
    }
}

#[test]
fn workshop_scene_draws_distinct_system_star_world_layers_and_selected_star_ring() {
    let state = populated_state();
    let selected_star = entity(3);
    let mut contrast_witnesses = Vec::new();
    for high_contrast in [false, true] {
        let scene = nyon::presentation::workshop::build_workshop_scene_frame(
            &state,
            Some(selected_star),
            high_contrast,
        );
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
        let mut batch = PrimitiveBatch::default();
        let coverage = draw_workshop_scene(&scene, &layout, &mut batch, high_contrast);

        assert_eq!(coverage.system_markers.len(), state.systems.len());
        assert_eq!(coverage.star_markers.len(), state.stars.len());
        assert_eq!(coverage.world_markers.len(), state.worlds.len());
        assert!(
            coverage
                .marker_witnesses
                .windows(2)
                .all(|pair| marker_layer_rank(pair[0].layer) <= marker_layer_rank(pair[1].layer))
        );
        for witness in &coverage.marker_witnesses {
            assert_marker_witness(&batch, witness);
        }
        for witness in coverage.marker_witnesses.iter().filter(|witness| {
            matches!(
                witness.layer,
                WorkshopMarkerLayer::StarHalo
                    | WorkshopMarkerLayer::StarCore
                    | WorkshopMarkerLayer::World
            )
        }) {
            assert_eq!(witness.packed_shape, 0xffff_ff02);
            assert_eq!(witness.ring_thickness, Some(witness.radius));
        }

        for (source_index, star) in scene.stars.iter().enumerate() {
            let star_witnesses = coverage
                .marker_witnesses
                .iter()
                .filter(|witness| {
                    witness.source_index == source_index
                        && matches!(
                            witness.layer,
                            WorkshopMarkerLayer::StarHalo
                                | WorkshopMarkerLayer::StarCore
                                | WorkshopMarkerLayer::StarSelectionRing
                        )
                })
                .collect::<Vec<_>>();
            let expected_count = if star.flags[0] == 1 { 3 } else { 2 };
            assert_eq!(star_witnesses.len(), expected_count);
            assert_eq!(star_witnesses[0].layer, WorkshopMarkerLayer::StarHalo);
            assert_eq!(star_witnesses[1].layer, WorkshopMarkerLayer::StarCore);
            assert!(star_witnesses[0].radius > star_witnesses[1].radius);
            assert!(star_witnesses.iter().all(|witness| {
                witness.center.is_finite()
                    && witness.radius.is_finite()
                    && witness.radius > 0.0
                    && witness.color.iter().all(|value| value.is_finite())
            }));
            assert_eq!(star_witnesses[0].color[3], 0.28);
            assert_eq!(star_witnesses[1].color[3], 1.0);
            assert!(luminance(star_witnesses[1].color) > 0.20);
            if expected_count == 3 {
                assert_eq!(
                    star_witnesses[2].layer,
                    WorkshopMarkerLayer::StarSelectionRing
                );
                assert!(star_witnesses[2].radius > star_witnesses[0].radius);
            }
            let system_ring = coverage
                .marker_witnesses
                .iter()
                .find(|witness| {
                    witness.layer == WorkshopMarkerLayer::SystemRing
                        && witness.center == star_witnesses[0].center
                })
                .expect("each star must retain its enclosing system ring");
            assert!(
                system_ring.radius
                    > star_witnesses
                        .iter()
                        .map(|witness| witness.radius)
                        .fold(0.0_f32, f32::max)
            );
        }

        let selected_witnesses = coverage
            .marker_witnesses
            .iter()
            .filter(|witness| witness.layer == WorkshopMarkerLayer::StarSelectionRing)
            .count();
        assert_eq!(selected_witnesses, 1);
        assert!(
            batch
                .vertices()
                .iter()
                .all(|vertex| Vec2::from_array(vertex.position).is_finite())
        );
        contrast_witnesses.push(coverage.marker_witnesses);
    }

    let [normal_witnesses, high_contrast_witnesses] = contrast_witnesses.as_slice() else {
        panic!("both presentation modes must be retained for comparison");
    };
    for layer in [
        WorkshopMarkerLayer::StarHalo,
        WorkshopMarkerLayer::StarCore,
        WorkshopMarkerLayer::StarSelectionRing,
    ] {
        for normal in normal_witnesses
            .iter()
            .filter(|witness| witness.layer == layer)
        {
            let high_contrast = matching_witness(high_contrast_witnesses, normal);
            assert_eq!(high_contrast.center, normal.center);
            assert!(
                high_contrast.radius > normal.radius,
                "high contrast must enlarge {layer:?} geometry"
            );
        }
    }
    for layer in [
        WorkshopMarkerLayer::SystemRing,
        WorkshopMarkerLayer::StarSelectionRing,
    ] {
        for normal in normal_witnesses
            .iter()
            .filter(|witness| witness.layer == layer)
        {
            let high_contrast = matching_witness(high_contrast_witnesses, normal);
            assert!(high_contrast.ring_thickness.unwrap() > normal.ring_thickness.unwrap());
            assert_ne!(high_contrast.packed_shape, normal.packed_shape);
        }
    }

    let unselected = nyon::presentation::workshop::build_workshop_scene_frame(&state, None, false);
    let selected = nyon::presentation::workshop::build_workshop_scene_frame(
        &state,
        Some(selected_star),
        false,
    );
    let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
    let mut unselected_batch = PrimitiveBatch::default();
    draw_workshop_scene(&unselected, &layout, &mut unselected_batch, false);
    let mut selected_batch = PrimitiveBatch::default();
    draw_workshop_scene(&selected, &layout, &mut selected_batch, false);
    assert_eq!(
        selected_batch.vertices().len(),
        unselected_batch.vertices().len() + 6,
        "selection must add one geometric ring rather than recolor the star"
    );
}

fn marker_layer_rank(layer: WorkshopMarkerLayer) -> u8 {
    match layer {
        WorkshopMarkerLayer::SystemRing => 0,
        WorkshopMarkerLayer::StarHalo => 1,
        WorkshopMarkerLayer::StarCore => 2,
        WorkshopMarkerLayer::StarSelectionRing => 3,
        WorkshopMarkerLayer::World => 4,
    }
}

fn luminance(color: [f32; 4]) -> f32 {
    color[0] * 0.2126 + color[1] * 0.7152 + color[2] * 0.0722
}

fn matching_witness<'a>(
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

fn assert_marker_witness(batch: &PrimitiveBatch, witness: &WorkshopMarkerWitness) {
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

fn contains_full_viewport_quad(batch: &PrimitiveBatch, viewport: Vec2) -> bool {
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

fn opaque_quad_covers(batch: &PrimitiveBatch, point: Vec2) -> bool {
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
