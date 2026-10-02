use glam::Vec2;
use nyon::{
    living_content::LivingContent,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
    ui::platform::PlatformUiAction,
};
use nyon_workshop_core::living::LivingEventKindV2;
use std::collections::BTreeSet;

fn visible_pages(session: &mut SandboxSession, scale: f32) -> String {
    let mut visible = String::new();
    for _ in 0..500 {
        let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
            assert!(control.bounds.min.min_element() >= 0.0);
            assert!(control.bounds.max.x <= 800.0 && control.bounds.max.y <= 480.0);
            assert_eq!(
                frame
                    .semantics
                    .node(control.semantic_id.as_str())
                    .unwrap()
                    .bounds,
                Some(control.bounds.into())
            );
            if control.enabled {
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    control.action_id
                );
            }
            if (control.action_id.as_str().starts_with("sandbox.notebook.")
                || control.action_id.as_str() == "sandbox.creator.field.bookmark_label")
                && !matches!(
                    control.action_id.as_str(),
                    "sandbox.notebook.previous"
                        | "sandbox.notebook.next"
                        | "sandbox.notebook.close"
                )
            {
                visible.push_str(&control.label);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|control| control.action_id.as_str() == "sandbox.notebook.next" && control.enabled)
        {
            break;
        }
        session.apply_action(A::InspectorPage(false)).unwrap();
    }
    visible
}

#[test]
fn receipt_actions_precede_detail_pages_without_mutating_authority() {
    let mut session = SandboxSession::open().unwrap();
    let before = session.bookmark().unwrap();
    session.apply_action(A::NotebookPicker).unwrap();
    for (experiment, verify) in [(0, "supply.verify"), (2, "trade.verify")] {
        session
            .apply_action(A::ExperimentDetails(Some(experiment)))
            .unwrap();
        let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), 1.5, false);
        frame.semantics.validate().unwrap();
        for suffix in ["back", "record", verify] {
            assert!(
                frame.controls.iter().any(|control| {
                    control.action_id.as_str() == format!("sandbox.notebook.{suffix}")
                }),
                "missing first-page action {suffix}"
            );
        }
    }
    assert_eq!(session.bookmark().unwrap(), before);
    assert!(session.bookmarks().is_empty());
}

#[test]
fn all_seven_optional_experiments_are_reachable_and_uncertified_outcomes_fail_closed() {
    let mut session = SandboxSession::open().unwrap();
    let before = session.bookmark().unwrap();
    let generation = session.notebook_generation();
    session.set_paused(false);
    for scale in [0.85, 1.5] {
        session.apply_action(A::NotebookPicker).unwrap();
        session.update(std::time::Duration::from_secs(1)).unwrap();
        assert_eq!(session.tick(), 0);
        let mut reached = BTreeSet::new();
        for _ in 0..500 {
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            for control in &frame.controls {
                if let PlatformUiAction::Sandbox(A::ExperimentDetails(Some(index))) = control.action
                {
                    reached.insert(index);
                }
            }
            if !frame.controls.iter().any(|control| {
                control.action_id.as_str() == "sandbox.notebook.next" && control.enabled
            }) {
                break;
            }
            session.apply_action(A::InspectorPage(false)).unwrap();
        }
        assert_eq!(reached, BTreeSet::from([0, 1, 2, 3, 4, 5, 6]));
        for index in 0..7 {
            session
                .apply_action(A::ExperimentDetails(Some(index)))
                .unwrap();
            let text = visible_pages(&mut session, scale);
            assert!(text.contains("Not certified:"));
            assert!(session.apply_action(A::RecordExperiment(index)).is_err());
        }
        session.apply_action(A::CreatorCancel).unwrap();
        assert!(!session.creator_modal_open());
    }
    assert_eq!(session.bookmark().unwrap(), before);
    assert_eq!(session.notebook_generation(), generation);
    assert!(session.bookmarks().is_empty());
}

#[test]
fn autonomous_settlement_notebook_preserves_exact_receipt_and_bookmark_then_hides() {
    let mut session = SandboxSession::from_authority(LivingContent::FirstExpansion.open().unwrap());
    for _ in 0..361 {
        session.step_once().unwrap();
    }
    let before = session.bookmark().unwrap();
    session.apply_action(A::NotebookPicker).unwrap();
    session
        .edit_creator_field("bookmark_label", "First autonomous settlement")
        .unwrap();
    session.apply_action(A::ExperimentDetails(Some(1))).unwrap();
    assert!(visible_pages(&mut session, 0.85).contains("Available exact witness:"));
    session.apply_action(A::RecordExperiment(1)).unwrap();
    assert_eq!(session.bookmarks(), &[before]);
    let receipt = session.bookmark_receipt(0).unwrap();
    receipt.verify().unwrap();
    assert_eq!(receipt.tick, before.view.tick);
    assert_eq!(receipt.state_digest, before.digest);
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::SettlementClaimResolved)
    );
    let receipt_id = receipt
        .digest
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    session.apply_action(A::ExperimentDetails(None)).unwrap();
    let text = visible_pages(&mut session, 1.5);
    assert!(text.contains("First autonomous settlement"));
    assert!(text.contains(&receipt_id));
    assert!(
        text.contains("No broader completion or causal claim"),
        "visible notebook text: {text}"
    );
    session.apply_action(A::CreatorCancel).unwrap();
    assert!(!session.creator_modal_open());
    assert_eq!(session.bookmark().unwrap(), before);
}
