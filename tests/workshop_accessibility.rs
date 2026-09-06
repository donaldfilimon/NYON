use nyon::ui::accessibility::{
    AnnouncementKind, FocusError, FocusManager, SemanticActionId, SemanticAnnouncement,
    SemanticNode, SemanticRect, SemanticRole, SemanticTree, SemanticTreeError,
};

#[cfg(not(target_arch = "wasm32"))]
use accesskit::{Action, Live, Role};
#[cfg(not(target_arch = "wasm32"))]
use nyon::ui::platform_native::NativeSemanticAdapter;

fn action(value: &str) -> SemanticActionId {
    SemanticActionId::new(value)
}

#[test]
fn focus_order_is_stable_cyclic_and_deduplicated() {
    let mut focus = FocusManager::new([
        action("tool.create-system"),
        action("tool.create-world"),
        action("tool.create-system"),
        action("timeline.pause"),
    ]);

    assert_eq!(
        focus.base_order(),
        &[
            action("tool.create-system"),
            action("tool.create-world"),
            action("timeline.pause"),
        ]
    );
    assert_eq!(focus.focused(), Some(&action("tool.create-system")));
    assert_eq!(focus.move_next(), Some(&action("tool.create-world")));
    assert_eq!(focus.move_next(), Some(&action("timeline.pause")));
    assert_eq!(focus.move_next(), Some(&action("tool.create-system")));
    assert_eq!(focus.move_previous(), Some(&action("timeline.pause")));
}

#[test]
fn modal_focus_is_trapped_then_restored_to_the_prior_control() {
    let mut focus = FocusManager::new([
        action("tool.create-system"),
        action("remove.open"),
        action("save.commit"),
    ]);
    assert!(focus.request_focus(&action("remove.open")));

    assert_eq!(
        focus
            .open_modal([action("remove.cancel"), action("remove.confirm")])
            .unwrap(),
        &action("remove.cancel")
    );
    assert!(focus.modal_is_open());
    assert!(!focus.request_focus(&action("save.commit")));
    assert_eq!(focus.move_next(), Some(&action("remove.confirm")));
    assert_eq!(focus.move_next(), Some(&action("remove.cancel")));

    assert_eq!(focus.close_modal(), Some(&action("remove.open")));
    assert!(!focus.modal_is_open());
    assert_eq!(focus.move_next(), Some(&action("save.commit")));
}

#[test]
fn modal_focus_rejects_empty_and_nested_scopes() {
    let mut focus = FocusManager::new([action("base")]);
    assert_eq!(
        focus.open_modal(std::iter::empty()),
        Err(FocusError::EmptyModal)
    );
    focus.open_modal([action("cancel")]).unwrap();
    assert_eq!(
        focus.open_modal([action("second")]),
        Err(FocusError::ModalAlreadyOpen)
    );
}

#[test]
fn modal_order_refresh_is_atomic_and_preserves_focus_and_restore_target() {
    let mut focus = FocusManager::new([action("base"), action("restore")]);
    assert!(focus.request_focus(&action("restore")));
    focus
        .open_modal([action("field"), action("cancel")])
        .unwrap();
    assert!(focus.request_focus(&action("cancel")));
    focus
        .replace_active_order([
            action("field"),
            action("cancel"),
            action("next"),
            action("next"),
        ])
        .unwrap();
    assert_eq!(focus.focused(), Some(&action("cancel")));
    assert_eq!(focus.move_next(), Some(&action("next")));
    assert_eq!(focus.move_next(), Some(&action("field")));
    let previous = focus.clone();
    assert_eq!(focus.replace_active_order([]), Err(FocusError::EmptyModal));
    assert_eq!(focus, previous);
    focus
        .replace_active_order([action("cancel"), action("previous")])
        .unwrap();
    assert_eq!(focus.focused(), Some(&action("cancel")));
    assert!(!focus.request_focus(&action("base")));
    assert_eq!(focus.close_modal(), Some(&action("restore")));
    focus.replace_active_order([action("new-base")]).unwrap();
    assert_eq!(focus.focused(), Some(&action("new-base")));
}

#[test]
fn semantic_tree_validation_rejects_ambiguous_adapter_contracts() {
    let control = SemanticNode::control(
        "control.one",
        SemanticRole::Button,
        "One",
        "First action",
        true,
        false,
        action("same-action"),
    );
    let duplicate_action = SemanticNode::control(
        "control.two",
        SemanticRole::Button,
        "Two",
        "Second action",
        true,
        false,
        action("same-action"),
    );
    let tree = SemanticTree {
        root: SemanticNode::container(
            "root",
            SemanticRole::Application,
            "NYON",
            vec![control, duplicate_action],
        ),
        announcements: Vec::new(),
    };

    assert_eq!(
        tree.validate(),
        Err(SemanticTreeError::DuplicateActionId(action("same-action")))
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_accesskit_tree_keeps_stable_ids_and_routes_the_shared_action() {
    let action_id = action("save.commit");
    let tree = SemanticTree {
        root: SemanticNode::container(
            "root",
            SemanticRole::Application,
            "NYON",
            vec![SemanticNode::control(
                "save",
                SemanticRole::Button,
                "Save Workshop",
                "Commit the current archive.",
                true,
                false,
                action_id.clone(),
            )],
        ),
        announcements: vec![SemanticAnnouncement {
            kind: AnnouncementKind::Error,
            code: "archive-rejected",
            message: "The Workshop archive could not be validated.".to_owned(),
        }],
    };
    let mut adapter = NativeSemanticAdapter::default();
    adapter.sync(&tree, Some(&action_id));
    let first = adapter.full_tree_update();
    let node_id = adapter.action_node(&action_id).unwrap();
    assert_eq!(first.focus, node_id);
    let node = first
        .nodes
        .iter()
        .find_map(|(id, node)| (*id == node_id).then_some(node))
        .unwrap();
    assert_eq!(node.role(), Role::Button);
    assert!(node.supports_action(Action::Click));
    assert!(node.supports_action(Action::Focus));
    let alert = first
        .nodes
        .iter()
        .map(|(_, node)| node)
        .find(|node| node.role() == Role::Alert)
        .unwrap();
    assert_eq!(alert.live(), Some(Live::Assertive));
    assert_eq!(
        alert.label(),
        Some("The Workshop archive could not be validated.")
    );

    adapter.sync(&tree, Some(&action_id));
    let second = adapter.full_tree_update();
    assert_eq!(adapter.action_node(&action_id), Some(node_id));
    assert_eq!(second.focus, node_id);
    assert!(adapter.request_node_action(node_id));
    assert_eq!(adapter.drain_actions().collect::<Vec<_>>(), vec![action_id]);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_accesskit_tree_traps_actions_inside_the_active_dialog() {
    let underlying = action("save.commit");
    let cancel = action("creator.cancel");
    let submit = action("creator.submit");
    let tree = SemanticTree {
        root: SemanticNode::container(
            "root",
            SemanticRole::Application,
            "NYON",
            vec![
                SemanticNode::control(
                    "save",
                    SemanticRole::Button,
                    "Save Workshop",
                    "Commit the current archive.",
                    true,
                    false,
                    underlying.clone(),
                ),
                SemanticNode::container(
                    "creator.dialog",
                    SemanticRole::Dialog,
                    "Create system",
                    vec![
                        SemanticNode::control(
                            "creator.cancel",
                            SemanticRole::Button,
                            "Cancel",
                            "Close the creator form.",
                            true,
                            false,
                            cancel.clone(),
                        ),
                        SemanticNode::control(
                            "creator.submit",
                            SemanticRole::Button,
                            "Apply",
                            "Submit the creator batch.",
                            true,
                            false,
                            submit.clone(),
                        ),
                    ],
                ),
            ],
        ),
        announcements: Vec::new(),
    };

    let mut adapter = NativeSemanticAdapter::default();
    adapter.sync(&tree, Some(&underlying));
    let update = adapter.full_tree_update();
    assert_eq!(adapter.action_node(&underlying), None);
    assert!(adapter.action_node(&cancel).is_some());
    assert!(adapter.action_node(&submit).is_some());
    let (underlying_node_id, underlying_node) = update
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Save Workshop"))
        .unwrap();
    assert!(underlying_node.is_disabled());
    assert!(!underlying_node.supports_action(Action::Click));
    assert!(!adapter.request_node_action(*underlying_node_id));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_accesskit_creator_fields_expose_editable_roles_and_values() {
    let action_id = action("creator.field.name");
    let mut field = SemanticNode::control(
        "creator.name",
        SemanticRole::TextInput,
        "Name",
        "Type a Workshop object name.",
        true,
        false,
        action_id.clone(),
    );
    field.value = Some("Ember Reach".to_owned());
    let tree = SemanticTree {
        root: SemanticNode::container("root", SemanticRole::Application, "NYON", vec![field]),
        announcements: Vec::new(),
    };

    let mut adapter = NativeSemanticAdapter::default();
    adapter.sync(&tree, Some(&action_id));
    let update = adapter.full_tree_update();
    let node_id = adapter.action_node(&action_id).unwrap();
    let node = update
        .nodes
        .iter()
        .find_map(|(id, node)| (*id == node_id).then_some(node))
        .unwrap();
    assert_eq!(node.role(), Role::TextInput);
    assert_eq!(node.value(), Some("Ember Reach"));
    assert!(node.supports_action(Action::SetValue));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_accesskit_uses_shared_visible_control_bounds() {
    let action_id = action("view.open-navigator");
    let mut control = SemanticNode::control(
        "view.open-navigator.node",
        SemanticRole::Button,
        "Navigator",
        "Open the measured navigator drawer.",
        true,
        false,
        action_id.clone(),
    );
    control.bounds = Some(SemanticRect {
        min: [14.0, 72.0],
        max: [58.0, 116.0],
    });
    let tree = SemanticTree {
        root: SemanticNode::container("root", SemanticRole::Application, "NYON", vec![control]),
        announcements: Vec::new(),
    };
    let mut adapter = NativeSemanticAdapter::default();
    adapter.sync(&tree, Some(&action_id));
    let update = adapter.full_tree_update();
    let node_id = adapter.action_node(&action_id).unwrap();
    let node = update
        .nodes
        .iter()
        .find_map(|(id, node)| (*id == node_id).then_some(node))
        .unwrap();
    let bounds = node.bounds().unwrap();
    assert_eq!(
        [bounds.x0, bounds.y0, bounds.x1, bounds.y1],
        [14.0, 72.0, 58.0, 116.0]
    );
    assert!(node.supports_action(Action::Focus));
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn actual_sdf_frames_deliver_stable_native_nodes_and_virtual_action_scope() {
    use nyon::ui::{
        AtlasMetrics,
        platform::build_workshop_platform_frame_for_view,
        platform_sdf::build_platform_ui_batch,
        workshop::{CreatorTool, WorkshopUiContext, WorkshopUiModel},
        workshop_layout::WorkshopLayout,
        workshop_view::WorkshopViewState,
    };
    use nyon_workshop_core::{
        BatchLocalId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopHistory,
    };
    let catalog = nyon_workshop_core::decode_catalog_pack(include_bytes!(
        "../assets/workshop/core-pack-v1.json"
    ))
    .unwrap();
    let mut history = WorkshopHistory::from_seed_u64(catalog, 44);
    history
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: nyon_workshop_core::WorkshopTick(0),
            operations: (0..64)
                .map(|index| CreatorOpV1::CreateSystem {
                    local: BatchLocalId(index),
                    name: ObjectName::new(format!("System {index:02}")).unwrap(),
                    position: GalaxyPointV1::new(i64::from(index), 0).unwrap(),
                })
                .collect(),
        })
        .unwrap();
    let system = *history.active_state().systems.keys().next().unwrap();
    let active = history.active_view();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: active.view_cursor,
            expected_tick: active.tick,
            operations: vec![CreatorOpV1::CreateStar {
                local: BatchLocalId(0),
                system: nyon_workshop_core::ObjectRefV1::Existing(system),
                name: ObjectName::new("Star").unwrap(),
                archetype_id: nyon_workshop_core::CatalogId::new("yellow-dwarf").unwrap(),
            }],
        })
        .unwrap();
    let session = nyon::workshop::session::WorkshopSession::new(history);
    let model = WorkshopUiModel::build(session.snapshot(), WorkshopUiContext::default());
    let layout = WorkshopLayout::resolve(glam::Vec2::new(723.0, 802.0), 1.3).unwrap();
    let mut view = WorkshopViewState::default();
    let mut adapter = NativeSemanticAdapter::default();
    let mut identities = std::collections::BTreeMap::new();
    let first = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
    check_actual_native_frame(&first, &mut adapter, &mut identities);
    let hidden = model.outliner.last().unwrap();
    assert!(
        !first
            .semantics
            .node(&format!("outliner.{}", hidden.control.action_id))
            .is_some_and(|node| node.visible)
    );
    assert!(adapter.action_node(&hidden.control.action_id).is_none());
    assert!(view.reveal_action(&model, &layout, &hidden.control.action_id));
    let revealed = build_workshop_platform_frame_for_view(
        &model,
        layout.clone(),
        &view,
        Some(&hidden.control.action_id),
    );
    check_actual_native_frame(&revealed, &mut adapter, &mut identities);
    let output = build_platform_ui_batch(&revealed, &AtlasMetrics::embedded().unwrap()).unwrap();
    let control = revealed
        .controls
        .iter()
        .find(|control| control.action_id == hidden.control.action_id)
        .unwrap();
    assert!(
        output
            .text_runs
            .iter()
            .any(|run| run.semantic_id == control.semantic_id)
    );
    assert!(adapter.action_node(&hidden.control.action_id).is_some());
    let modal_model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            ..WorkshopUiContext::default()
        },
    );
    let modal = build_workshop_platform_frame_for_view(&modal_model, layout, &view, None);
    check_actual_native_frame(&modal, &mut adapter, &mut identities);
    assert!(adapter.action_node(&hidden.control.action_id).is_none());
    for action in modal.focus_order() {
        assert!(adapter.action_node(&action).is_some());
    }
    let paged_model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateWorld),
            ..WorkshopUiContext::default()
        },
    );
    let short = WorkshopLayout::resolve(glam::Vec2::new(1280.0, 480.0), 1.3).unwrap();
    let fields = &paged_model.creator_form.as_ref().unwrap().fields;
    assert_eq!(fields.len(), 7);
    for field in fields {
        assert!(view.reveal_action(&paged_model, &short, &field.control.action_id));
        let frame = build_workshop_platform_frame_for_view(
            &paged_model,
            short.clone(),
            &view,
            Some(&field.control.action_id),
        );
        check_actual_native_frame(&frame, &mut adapter, &mut identities);
        assert!(adapter.action_node(&field.control.action_id).is_some());
        assert!(adapter.action_node(&hidden.control.action_id).is_none());
    }
    assert!(adapter.action_node(&fields[0].control.action_id).is_none());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn actual_creator_and_removal_frames_keep_visible_enabled_modal_ancestry() {
    use nyon::ui::{
        platform::build_workshop_platform_frame_for_view,
        workshop::{CreatorTool, WorkshopUiContext, WorkshopUiModel},
        workshop_layout::WorkshopLayout,
        workshop_view::WorkshopViewState,
    };
    use nyon_workshop_core::{
        BatchLocalId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopHistory,
        WorkshopTick,
    };

    fn assert_modal_path(
        node: &SemanticNode,
        target: &str,
        ancestors_visible: bool,
        ancestors_enabled: bool,
    ) -> bool {
        let path_visible = ancestors_visible && node.visible;
        let path_enabled = ancestors_enabled && node.enabled;
        if node.id.as_str() == target {
            assert!(path_visible, "{target} inherits hidden semantic ancestry");
            assert!(path_enabled, "{target} inherits disabled semantic ancestry");
            return true;
        }
        node.children
            .iter()
            .any(|child| assert_modal_path(child, target, path_visible, path_enabled))
    }

    let catalog = nyon_workshop_core::decode_catalog_pack(include_bytes!(
        "../assets/workshop/core-pack-v1.json"
    ))
    .unwrap();
    let mut history = WorkshopHistory::from_seed_u64(catalog, 71);
    history
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(0),
                name: ObjectName::new("Modal ancestry system").unwrap(),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    let target = *history.active_state().systems.keys().next().unwrap();
    let session = nyon::workshop::session::WorkshopSession::new(history);
    let layout = WorkshopLayout::resolve(glam::Vec2::new(1280.0, 480.0), 1.0).unwrap();

    for (context, dialog_id) in [
        (
            WorkshopUiContext {
                creator_form: Some(CreatorTool::CreateWorld),
                ..WorkshopUiContext::default()
            },
            "workshop.creator-dialog",
        ),
        (
            WorkshopUiContext {
                pending_removal: Some(target),
                ..WorkshopUiContext::default()
            },
            "workshop.removal-dialog",
        ),
    ] {
        let model = WorkshopUiModel::build(session.snapshot(), context);
        let frame = build_workshop_platform_frame_for_view(
            &model,
            layout.clone(),
            &WorkshopViewState::default(),
            None,
        );
        assert!(
            assert_modal_path(&frame.semantics.root, dialog_id, true, true),
            "missing active dialog {dialog_id}"
        );

        let mut adapter = NativeSemanticAdapter::default();
        adapter.sync(&frame.semantics, None);
        let update = adapter.full_tree_update();
        let root = update
            .nodes
            .iter()
            .find(|(id, _)| *id == update.tree.as_ref().unwrap().root)
            .map(|(_, node)| node)
            .unwrap();
        assert!(!root.is_hidden(), "native root hides {dialog_id}");
        assert!(!root.is_disabled(), "native root disables {dialog_id}");
        for action in frame.focus_order() {
            assert!(
                adapter.action_node(&action).is_some(),
                "native adapter omitted modal action {action} for {dialog_id}"
            );
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn check_actual_native_frame(
    frame: &nyon::ui::platform::PlatformUiFrame,
    adapter: &mut NativeSemanticAdapter,
    identities: &mut std::collections::BTreeMap<String, accesskit::NodeId>,
) {
    fn visit(
        source: &SemanticNode,
        native_id: accesskit::NodeId,
        update: &accesskit::TreeUpdate,
        identities: &mut std::collections::BTreeMap<String, accesskit::NodeId>,
    ) {
        let native = &update
            .nodes
            .iter()
            .find(|(id, _)| *id == native_id)
            .unwrap()
            .1;
        if let Some(prior) = identities.insert(source.id.to_string(), native_id) {
            assert_eq!(native_id, prior);
        }
        if !source.name.is_empty() {
            assert_eq!(native.label(), Some(source.name.as_str()));
        }
        assert_eq!(native.value(), source.value.as_deref());
        assert_eq!(native.is_hidden(), !source.visible);
        if !source.visible {
            assert!(!native.supports_action(Action::Click));
            assert!(!native.supports_action(Action::Focus));
        }
        assert!(native.children().len() >= source.children.len());
        for (child, id) in source.children.iter().zip(native.children()) {
            visit(child, *id, update, identities);
        }
    }
    adapter.sync(&frame.semantics, None);
    let update = adapter.full_tree_update();
    visit(
        &frame.semantics.root,
        update.tree.as_ref().unwrap().root,
        &update,
        identities,
    );
    let output = nyon::ui::platform_sdf::build_platform_ui_batch(
        frame,
        &nyon::ui::AtlasMetrics::embedded().unwrap(),
    )
    .unwrap();
    for id in output
        .text_runs
        .iter()
        .map(|run| &run.semantic_id)
        .chain(output.icon_runs.iter().map(|run| &run.semantic_id))
        .chain(
            output
                .panel_witnesses
                .iter()
                .filter_map(|panel| panel.owning_node.as_ref()),
        )
    {
        let source = frame.semantics.node(id.as_str()).unwrap();
        assert!(source.visible);
        assert!(identities.contains_key(id.as_str()));
    }
    for source in frame.semantics.nodes_depth_first() {
        if !source.visible {
            assert!(
                output
                    .text_runs
                    .iter()
                    .all(|run| run.semantic_id != source.id)
            );
            assert!(
                output
                    .icon_runs
                    .iter()
                    .all(|run| run.semantic_id != source.id)
            );
            if let Some(action) = &source.action_id {
                assert!(adapter.action_node(action).is_none());
            }
        }
    }
}
