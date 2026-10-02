//! Real content produces ephemeral, replay-regenerated restriction witnesses.
use nyon::living_content::LivingContent;
use nyon_workshop_core::living::*;

fn restricted_boundary() -> LivingGalaxyAuthorityV2 {
    let mut authority = LivingContent::InterruptedCorridor.open().unwrap();
    for _ in 0..149 {
        authority.step().unwrap();
        if !authority.last_corridor_restrictions().is_empty() {
            return authority;
        }
    }
    panic!("approved Interrupted Corridor produced no actual restriction witness");
}

#[test]
fn actual_corridor_proofs_bind_and_regenerate_without_exporting_ephemeral_records() {
    let authority = restricted_boundary();
    let receipt = authority.last_receipt().unwrap();
    for witness in authority.last_corridor_restrictions() {
        assert_eq!(witness.tick(), receipt.tick);
        assert_eq!(witness.receipt_digest(), receipt.digest);
        assert!(witness.eligible_batch() > witness.restricted_batch());
        assert_eq!(
            witness.actual_dispatched_units(),
            witness.restricted_batch()
        );
        assert_ne!(witness.source_system(), witness.destination_system());
        assert!(witness.hazard().start_tick <= receipt.tick);
        assert!(receipt.tick < witness.hazard().end_tick);
    }
    let bytes = encode_living_archive_v2(&authority).unwrap();
    assert!(
        !std::str::from_utf8(&bytes)
            .unwrap()
            .contains("corridor_restrictions")
    );
    let replay = decode_living_archive_v2(&bytes, authority.catalog())
        .unwrap()
        .into_authority();
    assert_eq!(replay.bookmark().unwrap(), authority.bookmark().unwrap());
    assert_eq!(
        replay.last_corridor_restrictions(),
        authority.last_corridor_restrictions()
    );
}

#[test]
fn successful_paused_edits_clear_ephemeral_restrictions_and_rejections_preserve_them() {
    let mut authority = restricted_boundary();
    let proof = authority.last_corridor_restrictions().to_vec();
    let cursor = authority.published_cursor();
    let civs = authority.state().civilizations.as_slice();
    let envelope = LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Paused,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Existing { id: civs[0].id },
                to: LivingCommandTargetV2::Existing { id: civs[1].id },
                base_disposition: 20,
            }],
        },
    };
    let mut rejected = envelope.clone();
    rejected.expected_tick = LivingTickV2(cursor.tick.0 - 1);
    assert!(authority.apply_paused(rejected).is_err());
    assert_eq!(authority.last_corridor_restrictions(), proof);
    authority.apply_paused(envelope).unwrap();
    assert!(authority.last_corridor_restrictions().is_empty());
    assert_eq!(authority.state().tick, cursor.tick);
}

#[test]
fn staged_publication_preserves_restriction_parity_and_atomic_visibility() {
    let mut authority = LivingContent::InterruptedCorridor.open().unwrap();
    for _ in 0..99 {
        authority.step().unwrap();
    }
    let before = authority.bookmark().unwrap();
    let previous_proofs = authority.last_corridor_restrictions().to_vec();
    let mut reference = authority.clone();
    let expected = reference.step().unwrap();
    let mut job = authority.begin_step().unwrap();
    while job.poll(1024).unwrap() == LivingStepProgressV2::Pending {
        assert_eq!(authority.bookmark().unwrap(), before);
        assert_eq!(authority.last_corridor_restrictions(), previous_proofs);
    }
    let prepared = match job.finish_synchronously(&authority).unwrap() {
        LivingStepPreparationV2::Ready(prepared) => prepared,
        LivingStepPreparationV2::Failed(_) => panic!("valid staged content faulted"),
    };
    assert_eq!(authority.bookmark().unwrap(), before);
    let retired = authority.commit_step(prepared).unwrap();
    assert_eq!(authority.last_receipt(), Some(&expected));
    assert!(!reference.last_corridor_restrictions().is_empty());
    assert_eq!(
        authority.last_corridor_restrictions(),
        reference.last_corridor_restrictions()
    );
    drop(retired);
}
