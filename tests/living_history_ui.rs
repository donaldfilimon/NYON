use std::collections::BTreeSet;

use glam::Vec2;
use nyon::{
    living_creator::LivingCreatorTool,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
    ui::{accessibility::SemanticRole, platform::PlatformUiAction},
};

#[test]
fn requested_history_controls_and_full_commands_are_paginated_and_semantically_equivalent() {
    let mut session = SandboxSession::open().unwrap();
    session
        .apply_action(A::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    session
        .edit_creator_field("0.name", "Readable history command")
        .unwrap();
    session.preview_creator().unwrap();
    session.apply_creator().unwrap();
    let revision = session.authority().revisions().last().unwrap().id;
    let branch = session.authority().branch();
    let digest = session.state().digest().unwrap();
    for scale in [0.85, 1.0, 1.3, 1.5] {
        session.apply_action(A::HistoryPicker).unwrap();
        let mut actions = BTreeSet::new();
        for _ in 0..200 {
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            frame.semantics.validate().unwrap();
            for control in frame.controls.iter().filter(|control| control.enabled) {
                assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
                let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
                assert_eq!(node.bounds, Some(control.bounds.into()));
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    control.action_id
                );
                actions.insert(control.action_id.as_str().to_owned());
                if control.action_id.as_str().ends_with("field.history_tick")
                    || control.action_id.as_str().ends_with("field.compare_tick")
                {
                    assert_eq!(node.role, SemanticRole::TextInput);
                    assert_eq!(node.value.as_deref(), Some("0"));
                }
            }
            if !frame.controls.iter().any(|control| {
                control.action_id.as_str() == "sandbox.modal.page.next" && control.enabled
            }) {
                break;
            }
            session.apply_action(A::InspectorPage(false)).unwrap();
        }
        assert!(actions.contains("sandbox.creator.field.history_tick"));
        assert!(actions.contains("sandbox.creator.field.compare_tick"));
        assert!(actions.contains("sandbox.history.requested.restore"));
        assert!(actions.contains("sandbox.history.requested.compare"));
        assert!(
            actions
                .iter()
                .any(|id| id.starts_with("sandbox.history.choose.left."))
        );
        assert!(
            actions
                .iter()
                .any(|id| id.starts_with("sandbox.history.choose.right."))
        );
        assert!(
            actions
                .iter()
                .any(|id| id.starts_with("sandbox.history.cursor."))
        );
        session
            .apply_action(A::HistoryRevisionDetails(branch, revision))
            .unwrap();
        let mut command = String::new();
        for _ in 0..200 {
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            frame.semantics.validate().unwrap();
            for control in &frame.controls {
                if control
                    .action_id
                    .as_str()
                    .starts_with("sandbox.history.detail.line.")
                {
                    command.push_str(&control.label);
                    assert!(!control.enabled);
                    assert_eq!(
                        frame
                            .semantics
                            .node(control.semantic_id.as_str())
                            .unwrap()
                            .role,
                        SemanticRole::Status
                    );
                }
            }
            if !frame.controls.iter().any(|control| {
                control.action_id.as_str() == "sandbox.modal.page.next" && control.enabled
            }) {
                break;
            }
            session.apply_action(A::InspectorPage(false)).unwrap();
        }
        assert!(command.contains("Readable history command"));
        assert!(command.contains("create_system"));
        assert!(command.contains("Parent"));
        session.apply_action(A::HistoryDetailsClose).unwrap();
        assert_eq!(session.state().digest().unwrap(), digest);
    }
}

#[test]
fn all_four_header_lines_stay_above_tools_at_supported_scales() {
    let session = SandboxSession::open().unwrap();
    for viewport in [Vec2::new(800.0, 480.0), Vec2::new(1280.0, 720.0)] {
        for scale in [0.85, 1.0, 1.3, 1.5] {
            let frame = session.frame_with_scale(viewport, scale, false);
            let origin = frame
                .layout
                .left_panel
                .map_or(frame.layout.top_bar.min, |panel| panel.min)
                .y
                + 16.0;
            let tools = frame.controls.iter().filter(|control| {
                matches!(
                    control.action,
                    PlatformUiAction::Sandbox(
                        A::ChroniclePicker | A::OpenCreator(_) | A::HistoryPicker | A::CameraPicker
                    )
                )
            });
            for tool in tools {
                assert!(
                    tool.bounds.min.y >= origin + 84.0,
                    "{} overlaps the four-line header at scale {scale}",
                    tool.label
                );
            }
        }
    }
}
