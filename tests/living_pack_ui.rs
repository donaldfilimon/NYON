use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;
use nyon::{
    living_store::{
        LivingGenerationV2, LivingPackDescriptorV2, LivingPackReferenceV2, LivingSlotIdV2,
    },
    sandbox_session::{SandboxSession, SandboxUiAction as A},
    ui::{accessibility::SemanticRole, platform::PlatformUiAction},
};
use nyon_workshop_core::living::LivingCatalogHashV2;

#[test]
fn all_256_catalog_packs_are_reachable_without_authority_mutation() {
    let mut session = SandboxSession::open().unwrap();
    let rows = (0..=255)
        .map(|index| LivingPackDescriptorV2 {
            hash: LivingCatalogHashV2([index; 32]),
            generation: LivingGenerationV2(1),
            archived: index.is_multiple_of(2),
            diagnostic: None,
        })
        .collect::<Vec<_>>();
    let digest = session.state().digest().unwrap();
    session.set_pack_projection(&rows, &BTreeMap::new(), false);
    for scale in [0.85, 1.5] {
        session.apply_action(A::OpenPackLibrary).unwrap();
        let mut reached = BTreeSet::new();
        for _ in 0..500 {
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            frame.semantics.validate().unwrap();
            for control in &frame.controls {
                assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
                assert!(
                    control.bounds.min.min_element() >= 0.0
                        && control.bounds.max.x <= 800.0
                        && control.bounds.max.y <= 480.0
                );
                let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
                assert_eq!(node.bounds, Some(control.bounds.into()));
                if control.enabled {
                    assert_eq!(
                        frame.hit_test(control.bounds.center()).unwrap().action_id,
                        control.action_id
                    );
                }
                if let PlatformUiAction::Sandbox(A::SelectPack(hash)) = control.action {
                    reached.insert(hash);
                }
            }
            if !frame.controls.iter().any(|control| {
                control.action_id.as_str() == "sandbox.packs.next" && control.enabled
            }) {
                break;
            }
            session.apply_action(A::InspectorPage(false)).unwrap();
        }
        assert_eq!(reached.len(), 256);
        assert_eq!(session.state().digest().unwrap(), digest);
        session.apply_action(A::CreatorCancel).unwrap();
    }
}

#[test]
fn pack_deletion_review_discloses_real_archived_blockers_and_requires_typed_hash() {
    let mut session = SandboxSession::open().unwrap();
    let hash = LivingCatalogHashV2([42; 32]);
    let row = LivingPackDescriptorV2 {
        hash,
        generation: LivingGenerationV2(3),
        archived: true,
        diagnostic: None,
    };
    let mut refs = BTreeMap::from([(
        hash,
        vec![LivingPackReferenceV2 {
            slot: LivingSlotIdV2(7),
            name: "Archived dependency".into(),
            archived: true,
            retained_generation: LivingGenerationV2(2),
        }],
    )]);
    session.set_pack_projection(std::slice::from_ref(&row), &refs, false);
    session.apply_action(A::OpenPackLibrary).unwrap();
    session
        .apply_action(A::ReviewDeletePack {
            hash,
            generation: row.generation,
        })
        .unwrap();
    let typed = hash
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    session
        .edit_creator_field("pack_delete_hash", &typed)
        .unwrap();
    assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
    let mut facts = String::new();
    let mut saw_field = false;
    for _ in 0..100 {
        let frame = session.frame(Vec2::new(800.0, 480.0), false);
        for control in &frame.controls {
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.packs.fact.")
            {
                facts.push_str(&control.label);
                facts.push(' ');
            }
            if control.action_id.as_str() == "sandbox.creator.field.pack_delete_hash" {
                saw_field = true;
                let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
                assert_eq!(node.role, SemanticRole::TextInput);
                assert_eq!(node.value.as_deref(), Some(typed.as_str()));
            }
            if matches!(
                control.action,
                PlatformUiAction::Sandbox(A::ConfirmDeletePack { .. })
            ) {
                assert!(!control.enabled);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|control| control.action_id.as_str() == "sandbox.packs.next" && control.enabled)
        {
            break;
        }
        session.apply_action(A::InspectorPage(false)).unwrap();
    }
    assert!(facts.contains("Archived dependency"));
    assert!(facts.contains("retained generation 2"));
    assert!(facts.contains("archived galaxy"));
    assert!(facts.contains("irreversible"));
    assert!(saw_field);
    refs.insert(hash, Vec::new());
    session.set_pack_projection(std::slice::from_ref(&row), &refs, false);
    assert!(session.pack_delete_confirmation_matches(hash, row.generation));
    session
        .edit_creator_field("pack_delete_hash", "wrong")
        .unwrap();
    assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
    session.apply_action(A::PackLibraryBack).unwrap();
    assert_eq!(session.creator_field_value("pack_delete_hash"), None);
}

#[test]
fn pending_pack_work_preserves_unknown_catalog_and_back_retires_delete_review() {
    let mut session = SandboxSession::open().unwrap();
    let digest = session.state().digest().unwrap();
    session.set_pack_pending(true);
    session.apply_action(A::OpenPackLibrary).unwrap();
    let frame = session.frame(Vec2::new(800.0, 480.0), false);
    assert!(
        frame
            .controls
            .iter()
            .any(|control| control.label.contains("has not been loaded"))
    );
    session.set_pack_pending(false);
    let hash = LivingCatalogHashV2([42; 32]);
    let row = LivingPackDescriptorV2 {
        hash,
        generation: LivingGenerationV2(3),
        archived: true,
        diagnostic: None,
    };
    session.set_pack_projection(
        std::slice::from_ref(&row),
        &BTreeMap::from([(hash, vec![])]),
        false,
    );
    session
        .apply_action(A::ReviewDeletePack {
            hash,
            generation: row.generation,
        })
        .unwrap();
    let typed = hash
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    session
        .edit_creator_field("pack_delete_hash", &typed)
        .unwrap();
    assert!(session.pack_delete_confirmation_matches(hash, row.generation));
    session.set_pack_pending(true);
    assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
    session.set_pack_pending(false);
    assert!(session.pack_delete_confirmation_matches(hash, row.generation));
    session.apply_action(A::PackLibraryBack).unwrap();
    assert!(!session.pack_delete_confirmation_matches(hash, row.generation));
    session.apply_action(A::CreatorCancel).unwrap();
    assert!(!session.creator_modal_open());
    assert_eq!(session.state().digest().unwrap(), digest);
}
