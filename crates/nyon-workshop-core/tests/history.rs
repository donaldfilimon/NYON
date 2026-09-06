mod common;

use nyon_workshop_core::{
    CheckpointPolicy, CreatorBatchV1, CreatorOpV1, HistoryError, WorkshopHistory, encode_archive,
};

#[test]
fn undo_redo_and_alternate_branch_preserve_both_exact_states() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 31);
    let forge = common::create_forge(&mut history);
    history.advance_ticks(3).unwrap();
    let parent = history.active_revision();
    let main_branch = history.active_view().selected_branch;
    let original = history
        .submit(CreatorBatchV1 {
            expected_cursor: parent,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Relay Forge"),
            }],
        })
        .unwrap();
    let original_digest = history.active_state_digest();
    assert_eq!(history.revision_count(), 2);

    assert_eq!(history.undo().unwrap(), parent);
    assert_eq!(history.state().tick, original.tick);
    assert_eq!(history.redo().unwrap(), original.revision);
    assert_eq!(history.active_state_digest(), original_digest);

    history.undo().unwrap();
    let alternate = history
        .submit(CreatorBatchV1 {
            expected_cursor: parent,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Relay Alternate"),
            }],
        })
        .unwrap();
    let alternate_branch = history.active_view().selected_branch;
    let alternate_digest = history.active_state_digest();
    assert_ne!(alternate_branch, main_branch);
    assert_eq!(history.branches()[&alternate_branch].name, "Branch 1");
    assert_eq!(history.branches().len(), 2);

    history.undo().unwrap();
    let choices = history.redo_choices();
    assert_eq!(choices.len(), 2);
    assert!(choices.contains(&original.revision));
    assert!(choices.contains(&alternate.revision));
    assert_eq!(
        history.redo(),
        Err(HistoryError::AmbiguousRedo {
            choices: choices.clone()
        })
    );
    history.redo_to(original.revision).unwrap();
    assert_eq!(history.active_view().selected_branch, main_branch);
    assert_eq!(history.active_state_digest(), original_digest);

    history.switch_branch(alternate_branch).unwrap();
    assert_eq!(history.active_state_digest(), alternate_digest);
}

#[test]
fn undo_requires_pause_and_checkpoint_cache_is_bounded_and_derived() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 32);
    common::create_forge(&mut history);
    history.set_paused(false);
    assert_eq!(history.undo(), Err(HistoryError::NotPaused));
    history.set_paused(true);
    history.set_checkpoint_policy(CheckpointPolicy {
        tick_interval: 1,
        revision_interval: 1,
        maximum_entries: 2,
    });
    for _ in 0..5 {
        history.step().unwrap();
    }
    assert_eq!(history.checkpoint_count(), 2);
}

#[test]
fn advancing_an_ancestor_keeps_the_original_branch_tick_and_recovery_intact() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 33);
    let forge = common::create_forge(&mut history);
    history.advance_ticks(2).unwrap();
    let first = history.active_revision();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: first,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Original Head"),
            }],
        })
        .unwrap();
    let original_branch = history.active_view().selected_branch;
    let original_tick = history.branches()[&original_branch].last_tick;
    let original_digest = history.active_state_digest();

    history.undo().unwrap();
    history.step().unwrap();
    assert_eq!(
        history.branches()[&original_branch].last_tick,
        original_tick
    );
    assert!(history.state().tick > original_tick);

    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Forked Head"),
            }],
        })
        .unwrap();
    let fork = history.active_view().selected_branch;
    assert_ne!(fork, original_branch);
    assert_eq!(history.branches()[&fork].last_tick, history.state().tick);
    assert_eq!(
        history.branches()[&original_branch].last_tick,
        original_tick
    );

    history.switch_branch(original_branch).unwrap();
    assert_eq!(history.state().tick, original_tick);
    assert_eq!(history.active_state_digest(), original_digest);
}

#[test]
fn redo_can_select_a_revision_from_a_branch_that_later_advanced() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 35);
    let forge = common::create_forge(&mut history);
    history.advance_ticks(3).unwrap();
    let parent = history.active_revision();
    let main_branch = history.active_view().selected_branch;
    let original = history
        .submit(CreatorBatchV1 {
            expected_cursor: parent,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Original Child"),
            }],
        })
        .unwrap();

    history.undo().unwrap();
    let alternate = history
        .submit(CreatorBatchV1 {
            expected_cursor: parent,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Alternate Child"),
            }],
        })
        .unwrap();
    let alternate_branch = history.active_view().selected_branch;

    history.switch_branch(main_branch).unwrap();
    history.advance_ticks(5).unwrap();
    assert!(history.branches()[&main_branch].last_tick > original.tick);

    history.switch_branch(alternate_branch).unwrap();
    history.undo().unwrap();
    assert_eq!(history.state().tick, alternate.tick);
    let choices = history.redo_choices();
    assert_eq!(choices.len(), 2);
    assert!(choices.contains(&original.revision));
    assert!(choices.contains(&alternate.revision));

    history.redo_to(original.revision).unwrap();
    assert_eq!(history.active_view().selected_branch, main_branch);
    assert_eq!(history.active_view().tick, original.tick);
    assert_eq!(history.active_revision(), Some(original.revision));
}

#[test]
fn checkpoint_cadence_cannot_change_replay_digest_or_archive_bytes() {
    let mut cached = WorkshopHistory::from_seed_u64(common::pack(), 34);
    let mut uncached = WorkshopHistory::from_seed_u64(common::pack(), 34);
    cached.set_checkpoint_policy(CheckpointPolicy {
        tick_interval: 600,
        revision_interval: 1,
        maximum_entries: 4,
    });
    uncached.set_checkpoint_policy(CheckpointPolicy {
        tick_interval: 0,
        revision_interval: 0,
        maximum_entries: 0,
    });
    let cached_forge = common::create_forge(&mut cached);
    let uncached_forge = common::create_forge(&mut uncached);
    cached.advance_ticks(2).unwrap();
    uncached.advance_ticks(2).unwrap();
    cached.set_checkpoint_policy(CheckpointPolicy {
        tick_interval: 600,
        revision_interval: 0,
        maximum_entries: 4,
    });

    for (history, world) in [
        (&mut cached, cached_forge.world_b),
        (&mut uncached, uncached_forge.world_b),
    ] {
        history
            .submit(CreatorBatchV1 {
                expected_cursor: history.active_revision(),
                expected_tick: history.state().tick,
                operations: vec![CreatorOpV1::RenameObject {
                    target: common::existing(world),
                    name: common::name("Checkpoint Neutral"),
                }],
            })
            .unwrap();
        history.undo().unwrap();
        history.redo().unwrap();
    }

    assert!(cached.checkpoint_count() > 0);
    assert_eq!(uncached.checkpoint_count(), 0);
    assert_eq!(cached.state(), uncached.state());
    assert_eq!(cached.active_state_digest(), uncached.active_state_digest());
    assert_eq!(
        encode_archive(&cached).unwrap(),
        encode_archive(&uncached).unwrap()
    );
}

#[test]
fn checkpoint_cadence_is_independent_for_forked_branch_lineages() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 36);
    history.set_checkpoint_policy(CheckpointPolicy {
        tick_interval: 2,
        revision_interval: 0,
        maximum_entries: 8,
    });
    let forge = common::create_forge(&mut history);
    history.advance_ticks(2).unwrap();
    let main = history.active_view().selected_branch;
    let at_fork = history.active_revision();
    let checkpoints_on_main = history.checkpoint_count();

    history
        .submit(CreatorBatchV1 {
            expected_cursor: at_fork,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Main Future"),
            }],
        })
        .unwrap();
    history.undo().unwrap();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: at_fork,
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Fork Future"),
            }],
        })
        .unwrap();
    let fork = history.active_view().selected_branch;
    assert_ne!(fork, main);
    assert!(history.checkpoint_count() > checkpoints_on_main);

    history.switch_branch(main).unwrap();
    let main_digest = history.active_state_digest();
    history.switch_branch(fork).unwrap();
    let fork_digest = history.active_state_digest();
    assert_ne!(main_digest, fork_digest);
}
