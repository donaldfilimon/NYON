//! Creator counterfactuals and portable Living history through public APIs.
use nyon_workshop_core::living::*;

fn edit(
    authority: &LivingGalaxyAuthorityV2,
    mode: LivingCommandModeV2,
    value: i32,
) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    let rows = authority.state().civilizations.as_slice();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Existing { id: rows[0].id },
                to: LivingCommandTargetV2::Existing { id: rows[1].id },
                base_disposition: value,
            }],
        },
    }
}
fn step_to(authority: &mut LivingGalaxyAuthorityV2, tick: u64) {
    while authority.state().tick.0 < tick {
        authority.step().unwrap();
    }
}
#[test]
fn undo_redo_are_same_tick_counterfactuals_and_retain_allocator_marks() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    step_to(&mut authority, 11);
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    let first = authority.bookmark().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -80))
        .unwrap();
    step_to(&mut authority, 15);
    let present = authority.bookmark().unwrap();
    let high_water = authority.accepted_sequence_high_water();
    let undone = authority.undo().unwrap();
    assert_eq!(undone.view.tick, LivingTickV2(15));
    assert_eq!(undone.view.revision, first.view.revision);
    assert_ne!(undone.digest, present.digest);
    assert_eq!(authority.accepted_sequence_high_water(), high_water);
    assert!(matches!(
        authority.apply_paused(edit(&authority, LivingCommandModeV2::Paused, -60)),
        Err(LivingCommandRejectionV2::ViewingHistory)
    ));
    assert_eq!(authority.redo().unwrap(), present);
    assert_eq!(authority.accepted_sequence_high_water(), high_water);
    authority.restore_bookmark(first).unwrap();
    assert_eq!(authority.bookmark().unwrap(), first);
    let mut wrong = first;
    wrong.digest.0[0] ^= 1;
    assert_eq!(
        authority.restore_bookmark(wrong),
        Err(LivingHistoryErrorV2::DigestMismatch)
    );
    assert_eq!(authority.bookmark().unwrap(), first);
}
#[test]
fn portable_round_trip_preserves_modes_siblings_history_and_global_allocators() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    step_to(&mut authority, 9);
    authority
        .submit(edit(&authority, LivingCommandModeV2::Running, -30))
        .unwrap();
    authority.step().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -40))
        .unwrap();
    step_to(&mut authority, 60);
    let root = authority.bookmark().unwrap();
    let cursor = authority.revisions()[1].id;
    let rows = authority.state().civilizations.as_slice();
    let operations = vec![LivingCreatorOperationV2::SetRelationBase {
        from: LivingCommandTargetV2::Existing { id: rows[0].id },
        to: LivingCommandTargetV2::Existing { id: rows[1].id },
        base_disposition: -80,
    }];
    authority.fork_paused(cursor, operations.clone()).unwrap();
    let child = authority.bookmark().unwrap();
    authority.fork_paused(cursor, operations).unwrap();
    let present = authority.bookmark().unwrap();
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let decoded = decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.archive().branches.as_slice().len(), 3);
    assert_eq!(decoded.authority().bookmark().unwrap(), present);
    let mut restored = decoded.into_authority();
    assert_eq!(
        restored.accepted_sequence_high_water(),
        authority.accepted_sequence_high_water()
    );
    assert_eq!(
        restored.branch_sequence_high_water(),
        authority.branch_sequence_high_water()
    );
    restored.restore_bookmark(root).unwrap();
    assert_eq!(restored.bookmark().unwrap(), root);
    restored.restore_bookmark(child).unwrap();
    assert_eq!(restored.bookmark().unwrap(), child);
    restored.restore_bookmark(present).unwrap();
    assert_eq!(
        restored
            .compare_at(root.view.branch, child.view.branch, LivingTickV2(60))
            .unwrap()
            .tick,
        LivingTickV2(60)
    );
    assert_eq!(encode_living_archive_v2(&restored).unwrap(), bytes);
}
#[test]
fn a_saved_undo_view_keeps_its_retained_present_portably_addressable() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -80))
        .unwrap();
    step_to(&mut authority, 60);
    let present = authority.bookmark().unwrap();
    let undone = authority.undo().unwrap();
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let mut restored = decode_living_archive_v2(&bytes, authority.catalog())
        .unwrap()
        .into_authority();
    assert_eq!(restored.bookmark().unwrap(), undone);
    restored.restore_bookmark(present).unwrap();
    assert_eq!(restored.bookmark().unwrap(), present);
}
#[test]
fn incremental_replay_is_bounded_cancellable_and_never_installs_partial_states() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    step_to(&mut authority, 361);
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let mut replay = begin_decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(replay.poll(0), Err(LivingArchiveErrorV2::Budget));
    assert_eq!(replay.poll(1_025), Err(LivingArchiveErrorV2::Budget));
    assert_eq!(
        replay.poll(1).unwrap(),
        LivingArchiveReplayStatusV2::Pending
    );
    replay.cancel();
    assert_eq!(replay.poll(1), Err(LivingArchiveErrorV2::Cancelled));
    assert!(matches!(
        replay.finish(),
        Err(LivingArchiveErrorV2::Cancelled)
    ));
    let mut replay = begin_decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    let mut polls = 0;
    while replay.poll(1).unwrap() == LivingArchiveReplayStatusV2::Pending {
        polls += 1;
    }
    assert!(
        polls >= 722,
        "root future and active view each advance 361 boundaries"
    );
    assert_eq!(
        replay.finish().unwrap().authority().bookmark().unwrap(),
        authority.bookmark().unwrap()
    );
}
fn reseal(document: &mut LivingArchiveV2) -> Vec<u8> {
    let encoded = encode_canonical_v2(document, LIVING_MAX_ARCHIVE_BYTES_V2).unwrap();
    let text = std::str::from_utf8(&encoded).unwrap();
    let marker = text.rfind(",\"integrity_sha256\":").unwrap();
    let mut payload = text.as_bytes()[..marker].to_vec();
    payload.push(b'}');
    document.integrity_sha256 = LivingArchiveIntegrityV2(archive_integrity_v2(&payload));
    encode_canonical_v2(document, LIVING_MAX_ARCHIVE_BYTES_V2).unwrap()
}
#[test]
fn integrity_graph_genesis_final_state_and_wire_corruption_fail_closed() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let original: LivingArchiveV2 =
        decode_canonical_v2(&bytes, LIVING_MAX_ARCHIVE_BYTES_V2).unwrap();
    let mut corrupt = original.clone();
    corrupt.accepted_sequence_high_water = 0;
    assert!(matches!(
        decode_living_archive_v2(&reseal(&mut corrupt), authority.catalog()),
        Err(LivingArchiveErrorV2::HighWater)
    ));
    let mut corrupt = original.clone();
    corrupt.final_digest.0[0] ^= 1;
    assert!(matches!(
        decode_living_archive_v2(&reseal(&mut corrupt), authority.catalog()),
        Err(LivingArchiveErrorV2::FinalDigest)
    ));
    let mut corrupt = original.clone();
    corrupt.genesis_manifest.state.tick = LivingTickV2(1);
    assert!(matches!(
        decode_living_archive_v2(&reseal(&mut corrupt), authority.catalog()),
        Err(LivingArchiveErrorV2::Genesis(_))
    ));
    let mut corrupt = original.clone();
    corrupt.final_tick = LivingTickV2(1);
    assert!(matches!(
        decode_living_archive_v2(&reseal(&mut corrupt), authority.catalog()),
        Err(LivingArchiveErrorV2::HistoryGraph)
    ));
    let mut corrupt = original.clone();
    corrupt.final_digest.0[0] ^= 1;
    let bytes = encode_canonical_v2(&corrupt, LIVING_MAX_ARCHIVE_BYTES_V2).unwrap();
    assert!(matches!(
        decode_living_archive_v2(&bytes, authority.catalog()),
        Err(LivingArchiveErrorV2::Integrity)
    ));
    let mut unknown = String::from_utf8(encode_living_archive_v2(&authority).unwrap()).unwrap();
    unknown.insert_str(1, "\"unknown\":0,");
    assert!(matches!(
        decode_living_archive_v2(unknown.as_bytes(), authority.catalog()),
        Err(LivingArchiveErrorV2::Wire(_))
    ));
    assert_eq!(authority.bookmark().unwrap().digest, original.final_digest);
}
#[test]
fn pending_export_rejects_without_spending_or_dropping_the_queue() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .submit(edit(&authority, LivingCommandModeV2::Running, -30))
        .unwrap();
    let cursor = authority.published_cursor();
    assert_eq!(
        encode_living_archive_v2(&authority),
        Err(LivingArchiveErrorV2::PendingQueue)
    );
    assert_eq!(authority.published_cursor(), cursor);
    authority.step().unwrap();
    let bytes = encode_living_archive_v2(&authority).unwrap();
    assert_eq!(
        decode_living_archive_v2(&bytes, authority.catalog())
            .unwrap()
            .authority()
            .accepted_sequence_high_water(),
        1
    );
}

#[test]
fn bounded_undo_redo_matches_counterfactuals_and_history_cannot_step() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -80))
        .unwrap();
    step_to(&mut authority, 60);
    let present = authority.bookmark().unwrap();
    let mut expected = authority.clone();
    let undo = expected.undo().unwrap();
    let mut job = begin_undo_living_v2(&authority).unwrap();
    while job.poll(3).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    let mut result = job.finish().unwrap().into_authority();
    assert_eq!(result.bookmark().unwrap(), undo);
    let before = result.bookmark().unwrap();
    assert_eq!(
        result.step(),
        Err(LivingDeterministicFaultV2::ViewingHistory)
    );
    assert_eq!(result.bookmark().unwrap(), before);
    assert!(result.fault().is_none());
    let mut job = begin_redo_living_v2(&result).unwrap();
    while job.poll(3).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    assert_eq!(
        job.finish().unwrap().authority().bookmark().unwrap(),
        present
    );
    assert_eq!(authority.bookmark().unwrap(), present);
}
#[test]
fn bounded_running_group_spends_a_work_unit_per_creator_input_before_atomic_commit() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    for value in [-30, -40, -80] {
        authority
            .submit(edit(&authority, LivingCommandModeV2::Running, value))
            .unwrap();
    }
    authority.step().unwrap();
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let mut job = begin_decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    for _ in 0..3 {
        assert_eq!(job.poll(1).unwrap(), LivingArchiveReplayStatusV2::Pending);
    }
    job.cancel();
    assert!(matches!(job.finish(), Err(LivingArchiveErrorV2::Cancelled)));
    let mut job = begin_decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    let mut polls = 0;
    while job.poll(1).unwrap() == LivingArchiveReplayStatusV2::Pending {
        polls += 1;
    }
    assert!(
        polls >= 8,
        "three staged inputs and one atomic boundary per view"
    );
    assert_eq!(
        job.finish().unwrap().authority().bookmark().unwrap(),
        authority.bookmark().unwrap()
    );
}
#[test]
fn undo_inherited_input_selects_parent_branch_and_redo_restores_the_child_cursor() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    let root = authority.branch();
    let cursor = authority.revisions()[0].id;
    let rows = authority.state().civilizations.as_slice();
    authority
        .fork_paused(
            cursor,
            vec![LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Existing { id: rows[0].id },
                to: LivingCommandTargetV2::Existing { id: rows[1].id },
                base_disposition: -80,
            }],
        )
        .unwrap();
    step_to(&mut authority, 60);
    let present = authority.bookmark().unwrap();
    authority.undo().unwrap();
    let inherited = authority.bookmark().unwrap();
    authority.undo().unwrap();
    assert_eq!(authority.branch(), root);
    let bytes = encode_living_archive_v2(&authority).unwrap();
    assert_eq!(
        decode_living_archive_v2(&bytes, authority.catalog())
            .unwrap()
            .authority()
            .bookmark()
            .unwrap(),
        authority.bookmark().unwrap()
    );
    assert_eq!(authority.redo().unwrap(), inherited);
    assert_eq!(authority.redo().unwrap(), present);
}

#[test]
fn bounded_matching_tick_comparison_keeps_both_exact_digests_and_cancels() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    let root = authority.branch();
    let cursor = authority.revisions()[0].id;
    let operations = edit(&authority, LivingCommandModeV2::Paused, -80)
        .command
        .operations()
        .to_vec();
    authority.fork_paused(cursor, operations).unwrap();
    let child = authority.branch();
    step_to(&mut authority, 60);
    let expected = authority.compare_at(root, child, LivingTickV2(60)).unwrap();
    let mut job = begin_compare_living_v2(&authority, root, child, LivingTickV2(60)).unwrap();
    assert_eq!(job.poll(1).unwrap(), LivingArchiveReplayStatusV2::Pending);
    job.cancel();
    assert!(matches!(job.finish(), Err(LivingArchiveErrorV2::Cancelled)));
    let before = authority.bookmark().unwrap();
    let mut job = begin_compare_living_v2(&authority, root, child, LivingTickV2(60)).unwrap();
    while job.poll(7).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    assert_eq!(job.finish().unwrap(), expected);
    assert_eq!(authority.bookmark().unwrap(), before);
}

#[test]
fn genesis_cursor_forks_export_and_reload_retained_sibling_futures() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    step_to(&mut authority, 61);
    let parent = authority.bookmark().unwrap();
    let cursor = parent.view;
    authority.fork_view_paused(vec![]).unwrap();
    let first = authority.bookmark().unwrap();
    step_to(&mut authority, 65);
    let future = authority.bookmark().unwrap();
    authority.restore_view(cursor).unwrap();
    authority.fork_view_paused(vec![]).unwrap();
    let second = authority.bookmark().unwrap();
    assert_ne!(first.view.branch, second.view.branch);
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let decoded = decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(decoded.canonical_bytes(), bytes);
    assert_eq!(decoded.archive().branches.as_slice().len(), 3);
    let mut restored = decoded.into_authority();
    for bookmark in [parent, first, future, second] {
        restored.restore_bookmark(bookmark).unwrap();
        assert_eq!(restored.bookmark().unwrap(), bookmark);
    }
}

#[test]
fn edited_cursor_fork_preserves_selected_later_tick_through_archive_replay() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    step_to(&mut authority, 11);
    authority
        .apply_paused(edit(&authority, LivingCommandModeV2::Paused, -30))
        .unwrap();
    step_to(&mut authority, 61);
    let parent = authority.bookmark().unwrap();
    let sequence = authority.accepted_sequence_high_water();
    let mut expected_state = authority.state().clone();
    authority.fork_view_paused(vec![]).unwrap();
    let child = authority.bookmark().unwrap();
    assert_eq!(child.view.tick, parent.view.tick);
    assert_eq!(child.view.revision, parent.view.revision);
    // A branch allocates its canonical branch_sequence, while every gameplay
    // field at the chosen boundary must remain exactly preserved.
    expected_state.branch_sequence = authority.branch_sequence_high_water();
    assert_eq!(authority.state(), &expected_state);
    assert_eq!(child.digest, expected_state.digest().unwrap());
    assert_eq!(authority.accepted_sequence_high_water(), sequence);
    step_to(&mut authority, 65);
    let future = authority.bookmark().unwrap();
    let decoded = decode_living_archive_v2(
        &encode_living_archive_v2(&authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let mut restored = decoded.into_authority();
    for bookmark in [parent, child, future] {
        restored.restore_bookmark(bookmark).unwrap();
        assert_eq!(restored.bookmark().unwrap(), bookmark);
    }
}
