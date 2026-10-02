//! Explicit removal previews, atomic rejection, and portable creator history.
use nyon_workshop_core::living::*;
fn envelope(
    authority: &LivingGalaxyAuthorityV2,
    operations: Vec<LivingCreatorOperationV2>,
) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Paused,
        command: LivingCommandV2::CreatorBatch { operations },
    }
}
fn remove(target: LivingEntityIdV2, dependents: &[LivingEntityIdV2]) -> LivingCreatorOperationV2 {
    LivingCreatorOperationV2::RemoveEntity {
        target: LivingCommandTargetV2::Existing { id: target },
        cascade: LivingCascadeDispositionV2::RemoveListed {
            dependents: dependents
                .iter()
                .map(|id| LivingCommandTargetV2::Existing { id: *id })
                .collect(),
        },
    }
}
#[test]
fn a_world_cascade_rejects_ambiguity_and_omissions_before_spending_then_replays_exactly() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    for _ in 0..60 {
        authority.step().unwrap();
    }
    let target = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Home A")
        .unwrap()
        .id;
    let preview = preview_living_removal_v2(authority.state(), target).unwrap();
    assert!(preview.dependents.len() >= 10);
    assert!(preview.derived_records > 0);
    let before = authority.bookmark().unwrap();
    let state = authority.state().clone();
    let high_water = authority.accepted_sequence_high_water();
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: LivingCommandTargetV2::Existing { id: target },
            cascade: LivingCascadeDispositionV2::RejectIfReferenced,
        }],
    );
    assert_eq!(
        authority.apply_paused(request),
        Err(LivingCommandRejectionV2::ReferencedTarget)
    );
    assert_eq!(authority.state(), &state);
    let request = envelope(&authority, vec![remove(target, &preview.dependents[1..])]);
    assert_eq!(
        authority.apply_paused(request),
        Err(LivingCommandRejectionV2::CascadeMismatch)
    );
    assert_eq!(authority.bookmark().unwrap(), before);
    assert_eq!(authority.accepted_sequence_high_water(), high_water);
    let request = envelope(&authority, vec![remove(target, &preview.dependents)]);
    let receipt = authority.apply_paused(request).unwrap();
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::CreatorIntervention)
    );
    assert!(
        !authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .any(|world| world.id == target)
    );
    assert!(
        !authority
            .state()
            .colonies
            .as_slice()
            .iter()
            .any(|colony| colony.world == target)
    );
    authority.state().validate(authority.catalog()).unwrap();
    let after = authority.bookmark().unwrap();
    assert_eq!(authority.replay_digest().unwrap(), after.digest);
    assert_eq!(authority.undo().unwrap(), before);
    assert_eq!(authority.redo().unwrap(), after);
    let bytes = encode_living_archive_v2(&authority).unwrap();
    assert_eq!(
        decode_living_archive_v2(&bytes, authority.catalog())
            .unwrap()
            .authority()
            .bookmark()
            .unwrap(),
        after
    );
}
#[test]
fn unrelated_or_unknown_cascade_members_leave_every_identity_and_resource_unchanged() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let target = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Home A")
        .unwrap()
        .id;
    let preview = preview_living_removal_v2(authority.state(), target).unwrap();
    let unrelated = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Home B")
        .unwrap()
        .id;
    let before = authority.state().clone();
    let cursor = authority.published_cursor();
    for extra in [unrelated, LivingEntityIdV2([0xfe; 16])] {
        let mut declared = preview.dependents.clone();
        declared.push(extra);
        let request = envelope(&authority, vec![remove(target, &declared)]);
        assert_eq!(
            authority.apply_paused(request),
            Err(LivingCommandRejectionV2::CascadeMismatch)
        );
        assert_eq!(authority.state(), &before);
        assert_eq!(authority.published_cursor(), cursor);
        assert_eq!(authority.accepted_sequence_high_water(), 0);
    }
}
#[test]
fn independent_entity_removal_and_colony_removal_have_deterministic_history() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let star = authority.state().stars.as_slice()[0].id;
    assert!(
        preview_living_removal_v2(authority.state(), star)
            .unwrap()
            .dependents
            .is_empty()
    );
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: LivingCommandTargetV2::Existing { id: star },
            cascade: LivingCascadeDispositionV2::RejectIfReferenced,
        }],
    );
    authority.apply_paused(request).unwrap();
    assert!(authority.state().stars.as_slice().is_empty());
    let home = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Home B")
        .unwrap()
        .id;
    let owner = authority
        .state()
        .colonies
        .as_slice()
        .iter()
        .find(|colony| colony.world == home)
        .unwrap()
        .owner;
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::RemoveColony {
            world: LivingCommandTargetV2::Existing { id: home },
        }],
    );
    authority.apply_paused(request).unwrap();
    assert!(
        !authority
            .state()
            .colonies
            .as_slice()
            .iter()
            .any(|colony| colony.world == home)
    );
    assert_eq!(
        authority
            .state()
            .civilizations
            .as_slice()
            .iter()
            .find(|row| row.id == owner)
            .unwrap()
            .status,
        LivingCivilizationStatusV2::Dormant
    );
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}

#[test]
fn cancelling_a_job_refunds_half_and_creator_owner_changes_require_explicit_cancellation() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let job = authority.state().hull_jobs.as_slice()[0].clone();
    let before = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == job.world)
        .unwrap()
        .inventory
        .alloy;
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::CancelHullJob {
            target: LivingCommandTargetV2::Existing { id: job.id },
        }],
    );
    authority.apply_paused(request).unwrap();
    assert!(authority.state().hull_jobs.as_slice().is_empty());
    assert_eq!(
        authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == job.world)
            .unwrap()
            .inventory
            .alloy,
        before + job.paid_alloy / 2
    );
    authority.undo().unwrap();
    assert!(!authority.state().hull_jobs.as_slice().is_empty());
    authority.return_to_present().unwrap();
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let state_before = authority.state().clone();
    let archive_before = encode_living_archive_v2(&authority).unwrap();
    let owner_change = LivingCreatorOperationV2::SetWorldOwner {
        world: LivingCommandTargetV2::Existing { id: job.world },
        owner: None,
    };
    let request = envelope(&authority, vec![owner_change.clone()]);
    assert!(matches!(
        authority.apply_paused(request),
        Err(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::Range {
                field: "foreign_jobs_on_world",
            }
        ))
    ));
    assert_eq!(*authority.state(), state_before);
    assert_eq!(
        encode_living_archive_v2(&authority).unwrap(),
        archive_before,
        "rejection preserves identities, queue and every retained history input"
    );
    let before = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == job.world)
        .unwrap()
        .inventory
        .alloy;
    let mut refund = 0;
    let mut operations = Vec::new();
    for queued in authority.state().construction_jobs.as_slice() {
        if queued.world == job.world {
            refund += queued.paid_alloy / 2;
            operations.push(LivingCreatorOperationV2::CancelConstructionJob {
                target: LivingCommandTargetV2::Existing { id: queued.id },
            });
        }
    }
    for queued in authority.state().hull_jobs.as_slice() {
        if queued.world == job.world {
            refund += queued.paid_alloy / 2;
            operations.push(LivingCreatorOperationV2::CancelHullJob {
                target: LivingCommandTargetV2::Existing { id: queued.id },
            });
        }
    }
    operations.push(owner_change);
    let request = envelope(&authority, operations);
    authority.apply_paused(request).unwrap();
    assert!(authority.state().construction_jobs.as_slice().is_empty());
    assert!(authority.state().hull_jobs.as_slice().is_empty());
    let world = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == job.world)
        .unwrap();
    assert_eq!(world.owner, None);
    assert_eq!(world.inventory.alloy, before + refund);
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let reloaded = decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(reloaded.authority().state(), authority.state());
}

#[test]
fn outstanding_world_queue_reservations_reject_new_work_atomically() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..50 {
        authority.step().unwrap();
    }
    let job = authority.state().construction_jobs.as_slice()[0].clone();
    let before = authority.state().clone();
    let cursor = authority.published_cursor();
    let sequence = authority.accepted_sequence_high_water();
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::CreateConstructionJob {
            local: 0,
            world: LivingCommandTargetV2::Existing { id: job.world },
            owner: LivingCommandTargetV2::Existing { id: job.owner },
            definition: job.definition,
            accepted_tick: LivingTickV2(50),
            completion_tick: LivingTickV2(250),
            paid_alloy: 40,
        }],
    );
    assert!(matches!(
        authority.apply_paused(request),
        Err(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::Range {
                field: "world construction queue"
            }
        ))
    ));
    assert_eq!(authority.state(), &before);
    assert_eq!(authority.published_cursor(), cursor);
    assert_eq!(authority.accepted_sequence_high_water(), sequence);
}

#[test]
fn shipyard_and_battery_uniqueness_count_existing_and_reserved_assets() {
    for definition in ["shipyard", "defense_battery"] {
        let mut authority = builtin_first_expansion_v2().unwrap();
        let world = authority.state().colonies.as_slice()[0].world;
        let owner = authority.state().colonies.as_slice()[0].owner;
        if definition == "defense_battery" {
            let request = envelope(
                &authority,
                vec![LivingCreatorOperationV2::CreateConstructionJob {
                    local: 0,
                    world: LivingCommandTargetV2::Existing { id: world },
                    owner: LivingCommandTargetV2::Existing { id: owner },
                    definition: LivingSlugV2::new(definition).unwrap(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(200),
                    paid_alloy: 40,
                }],
            );
            authority.apply_paused(request).unwrap();
        }
        let before = authority.state().clone();
        let cursor = authority.published_cursor();
        let sequence = authority.accepted_sequence_high_water();
        let request = envelope(
            &authority,
            vec![LivingCreatorOperationV2::CreateFacility {
                local: 0,
                world: LivingCommandTargetV2::Existing { id: world },
                definition: LivingSlugV2::new(definition).unwrap(),
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 0,
                hit_points: 0,
                next_due: None,
            }],
        );
        assert!(matches!(
            authority.apply_paused(request),
            Err(LivingCommandRejectionV2::Projection(
                LivingValidationErrorV2::Range {
                    field: "unique world industry"
                }
            ))
        ));
        assert_eq!(authority.state(), &before);
        assert_eq!(authority.published_cursor(), cursor);
        assert_eq!(authority.accepted_sequence_high_water(), sequence);
    }
}

#[test]
fn generic_creator_job_removal_is_an_override_without_economic_refund() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let job = authority.state().hull_jobs.as_slice()[0].clone();
    let before = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == job.world)
        .unwrap()
        .inventory;
    let request = envelope(
        &authority,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: LivingCommandTargetV2::Existing { id: job.id },
            cascade: LivingCascadeDispositionV2::RejectIfReferenced,
        }],
    );
    let result = authority.apply_paused(request).unwrap();
    assert!(authority.state().hull_jobs.as_slice().is_empty());
    assert_eq!(
        authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == job.world)
            .unwrap()
            .inventory,
        before
    );
    assert!(
        !result
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::RefundDiscarded)
    );
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
    let bytes = encode_living_archive_v2(&authority).unwrap();
    let reloaded = decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(reloaded.authority().state(), authority.state());
}
