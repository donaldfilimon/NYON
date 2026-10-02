//! Creator qualification uses public client forms and canonical reload, not test authority mutation.
use nyon::{
    living_content::LivingContent,
    living_creator::LivingCreatorTool as Tool,
    sandbox_session::{SandboxSession, SandboxUiAction},
};
use nyon_workshop_core::living::{decode_living_archive_v2, encode_living_archive_v2};
fn apply(session: &mut SandboxSession, tool: Tool) {
    session
        .apply_action(SandboxUiAction::OpenCreator(tool))
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap_or_else(|error| panic!("{} preview: {error}", tool.label()));
    session
        .apply_action(SandboxUiAction::CreatorApply)
        .unwrap_or_else(|error| panic!("{} apply: {error}", tool.label()));
}
fn prepared(tool: Tool) -> SandboxSession {
    let mut session = match tool {
        Tool::EditRoute | Tool::RouteSuspended => {
            SandboxSession::from_authority(LivingContent::Commerce.open().unwrap())
        }
        Tool::EditHazard | Tool::CancelHazard => {
            SandboxSession::from_authority(LivingContent::InterruptedCorridor.open().unwrap())
        }
        _ => SandboxSession::open().unwrap(),
    };
    if matches!(tool, Tool::EditRoute | Tool::RouteSuspended) {
        for _ in 0..100 {
            session.step_once().unwrap();
        }
    }
    match tool {
        Tool::CancelConstruction => apply(&mut session, Tool::Construction),
        Tool::CancelHullJob => apply(&mut session, Tool::HullJob),
        Tool::MergeFleets => apply(&mut session, Tool::SplitFleet),
        Tool::RemoveAgreement => apply(&mut session, Tool::Agreement),
        Tool::Peace => apply(&mut session, Tool::War),
        Tool::ShipmentDisposition => apply(&mut session, Tool::Shipment),
        Tool::Colony => apply(&mut session, Tool::RemoveColony),
        _ => {}
    }
    session
}
#[test]
fn every_creator_form_valid_transaction_has_exact_undo_redo_and_canonical_reload() {
    for tool in Tool::ALL {
        let mut session = prepared(tool);
        let before = session.bookmark().unwrap();
        let state = session.state().clone();
        apply(&mut session, tool);
        let after = session.bookmark().unwrap();
        assert_ne!(before, after, "{}", tool.label());
        let bytes = encode_living_archive_v2(session.authority()).unwrap();
        let loaded = decode_living_archive_v2(&bytes, session.authority().catalog()).unwrap();
        assert_eq!(
            loaded.authority().bookmark().unwrap(),
            after,
            "{} reload",
            tool.label()
        );
        session.apply_action(SandboxUiAction::Undo).unwrap();
        for _ in 0..10000 {
            session.update(std::time::Duration::ZERO).unwrap();
            if session.bookmark().unwrap() != after {
                break;
            }
        }
        assert_eq!(
            session.bookmark().unwrap().digest,
            before.digest,
            "{} undo",
            tool.label()
        );
        assert_eq!(session.state(), &state, "{} undo state", tool.label());
        session.apply_action(SandboxUiAction::Redo).unwrap();
        for _ in 0..10000 {
            session.update(std::time::Duration::ZERO).unwrap();
            if session.bookmark().unwrap() == after {
                break;
            }
        }
        assert_eq!(session.bookmark().unwrap(), after, "{} redo", tool.label());
    }
}

fn invalidate_reference(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("type").and_then(|v| v.as_str()) == Some("existing") {
                object.insert("id".into(), serde_json::json!("ff".repeat(16)));
                return true;
            }
            for child in object.values_mut() {
                if invalidate_reference(child) {
                    return true;
                }
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                if invalidate_reference(child) {
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}
#[test]
fn every_creator_form_missing_reference_or_invalid_local_rejects_without_history_or_identity_changes()
 {
    use nyon::living_creator::LivingCreatorDraft;
    use nyon_workshop_core::living::{
        LivingCommandEnvelopeV2, LivingCommandModeV2, LivingCommandV2, LivingCreatorOperationV2,
    };
    let mut accepted = Vec::new();
    for tool in Tool::ALL {
        let session = prepared(tool);
        let mut authority = session.authority().clone();
        let operations = LivingCreatorDraft::new(tool, authority.state(), None)
            .operations()
            .unwrap();
        let mut value = serde_json::to_value(&operations).unwrap();
        if !invalidate_reference(&mut value) {
            value[0]["local"] = serde_json::json!(9);
        }
        let operations: Vec<LivingCreatorOperationV2> = serde_json::from_value(value).unwrap();
        let cursor = authority.published_cursor();
        let before = encode_living_archive_v2(&authority).unwrap();
        let state = authority.state().clone();
        let envelope = LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: LivingCommandV2::CreatorBatch { operations },
        };
        if authority.apply_paused(envelope).is_ok() {
            accepted.push(tool.label());
            continue;
        }
        assert_eq!(authority.state(), &state, "{} state", tool.label());
        assert_eq!(
            encode_living_archive_v2(&authority).unwrap(),
            before,
            "{} archive includes all high-water/history identities",
            tool.label()
        );
    }
    assert!(
        accepted.is_empty(),
        "missing reference accepted: {accepted:?}"
    );
}

#[test]
fn system_and_per_fleet_hull_capacity_rejection_preserves_canonical_history() {
    use nyon::living_creator::LivingCreatorDraft;
    use nyon_workshop_core::living::{
        LIVING_MAX_HULLS_PER_FLEET_V2, LIVING_MAX_SYSTEMS_V2, LivingCommandEnvelopeV2,
        LivingCommandModeV2, LivingCommandV2, LivingCreatorOperationV2,
    };
    assert_eq!(LIVING_MAX_SYSTEMS_V2, 64);
    assert_eq!(LIVING_MAX_HULLS_PER_FLEET_V2, 16);
    for tool in [Tool::System, Tool::Hull] {
        let session = prepared(tool);
        let mut authority = session.authority().clone();
        let prototype = LivingCreatorDraft::new(tool, authority.state(), None)
            .operations()
            .unwrap()
            .remove(0);
        let count = match &prototype {
            LivingCreatorOperationV2::CreateSystem { .. } => {
                LIVING_MAX_SYSTEMS_V2 - authority.state().systems.as_slice().len()
            }
            LivingCreatorOperationV2::CreateHull {
                fleet: nyon_workshop_core::living::LivingCommandTargetV2::Existing { id },
                ..
            } => {
                LIVING_MAX_HULLS_PER_FLEET_V2
                    - authority
                        .state()
                        .fleets
                        .as_slice()
                        .iter()
                        .find(|row| row.id == *id)
                        .unwrap()
                        .hulls
                        .as_slice()
                        .len()
            }
            _ => panic!("unexpected form"),
        };
        let operations = (0..count)
            .map(|index| {
                let mut operation = prototype.clone();
                match &mut operation {
                    LivingCreatorOperationV2::CreateSystem { local, .. }
                    | LivingCreatorOperationV2::CreateHull { local, .. } => {
                        *local = index.try_into().unwrap()
                    }
                    _ => unreachable!(),
                };
                operation
            })
            .collect();
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch { operations },
            })
            .unwrap();
        let before = encode_living_archive_v2(&authority).unwrap();
        let cursor = authority.published_cursor();
        assert!(
            authority
                .apply_paused(LivingCommandEnvelopeV2 {
                    expected_committed_revision: cursor.committed_revision,
                    expected_tick: cursor.tick,
                    expected_pending_sequence: cursor.pending_sequence,
                    mode: LivingCommandModeV2::Paused,
                    command: LivingCommandV2::CreatorBatch {
                        operations: vec![prototype]
                    }
                })
                .is_err()
        );
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), before);
        authority
            .step()
            .expect("capacity refusal does not fault existing simulation");
    }
}

#[test]
fn generic_removal_is_free_explicit_cancellation_refunds_and_owner_change_requires_paid_job_disposition()
 {
    use nyon_workshop_core::living::{
        LivingCascadeDispositionV2, LivingCommandEnvelopeV2, LivingCommandModeV2,
        LivingCommandTargetV2 as Target, LivingCommandV2, LivingCreatorOperationV2 as Op,
    };
    fn transaction(
        authority: &mut nyon_workshop_core::living::LivingGalaxyAuthorityV2,
        operations: Vec<Op>,
    ) -> bool {
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch { operations },
            })
            .is_ok()
    }
    let session = prepared(Tool::CancelConstruction);
    let original = session.authority();
    let job = original.state().construction_jobs.as_slice()[0].clone();
    let owner = original
        .state()
        .civilizations
        .as_slice()
        .iter()
        .find(|row| row.id != job.owner)
        .unwrap()
        .id;
    let inventory = |a: &nyon_workshop_core::living::LivingGalaxyAuthorityV2| {
        a.state()
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.id == job.world)
            .unwrap()
            .inventory
            .alloy
    };
    let transfer = Op::SetWorldOwner {
        world: Target::Existing { id: job.world },
        owner: Some(Target::Existing { id: owner }),
    };
    let mut rejected = original.clone();
    let before = encode_living_archive_v2(&rejected).unwrap();
    assert!(!transaction(&mut rejected, vec![transfer.clone()]));
    assert_eq!(encode_living_archive_v2(&rejected).unwrap(), before);
    let mut generic = original.clone();
    assert!(transaction(
        &mut generic,
        vec![
            Op::RemoveEntity {
                target: Target::Existing { id: job.id },
                cascade: LivingCascadeDispositionV2::RejectIfReferenced
            },
            transfer.clone()
        ]
    ));
    assert_eq!(inventory(&generic), inventory(original));
    let mut economic = original.clone();
    assert!(transaction(
        &mut economic,
        vec![
            Op::CancelConstructionJob {
                target: Target::Existing { id: job.id }
            },
            transfer
        ]
    ));
    assert_eq!(
        inventory(&economic),
        inventory(original) + job.paid_alloy / 2
    );
    assert!(generic.state().construction_jobs.as_slice().is_empty());
    assert!(economic.state().construction_jobs.as_slice().is_empty());
    for authority in [&generic, &economic] {
        let bytes = encode_living_archive_v2(authority).unwrap();
        assert_eq!(
            decode_living_archive_v2(&bytes, authority.catalog())
                .unwrap()
                .authority()
                .state(),
            authority.state()
        );
    }
}

fn public_fixture_authority(
    state: nyon_workshop_core::living::LivingGalaxyStateV2,
    tag: &str,
) -> nyon_workshop_core::living::LivingGalaxyAuthorityV2 {
    use nyon_workshop_core::living::*;
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [93; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new(tag).unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn fixture_id(kind: u8, ordinal: usize) -> nyon_workshop_core::living::LivingEntityIdV2 {
    let mut bytes = [0xfd; 16];
    bytes[1] = kind;
    bytes[8..].copy_from_slice(&(ordinal as u64).to_be_bytes());
    nyon_workshop_core::living::LivingEntityIdV2(bytes)
}
fn ordered_rows<T: nyon_workshop_core::living::LivingKeyedV2>(
    mut rows: Vec<T>,
) -> nyon_workshop_core::living::LivingSortedVecV2<T> {
    rows.sort_by_key(|row| row.living_key());
    nyon_workshop_core::living::LivingSortedVecV2::new(rows).unwrap()
}
fn creator_transaction(
    authority: &mut nyon_workshop_core::living::LivingGalaxyAuthorityV2,
    operations: Vec<nyon_workshop_core::living::LivingCreatorOperationV2>,
) -> Result<
    nyon_workshop_core::living::LivingTickReceiptV2,
    nyon_workshop_core::living::LivingCommandRejectionV2,
> {
    use nyon_workshop_core::living::*;
    let cursor = authority.published_cursor();
    authority.apply_paused(LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Paused,
        command: LivingCommandV2::CreatorBatch { operations },
    })
}
fn exact_undo_redo_reload(
    authority: &mut nyon_workshop_core::living::LivingGalaxyAuthorityV2,
    before: &nyon_workshop_core::living::LivingGalaxyStateV2,
) {
    let after = authority.bookmark().unwrap();
    let after_state = authority.state().clone();
    let bytes = encode_living_archive_v2(authority).unwrap();
    let loaded = decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
    assert_eq!(loaded.authority().bookmark().unwrap(), after);
    assert_eq!(loaded.authority().state(), &after_state);
    authority.undo().unwrap();
    assert_eq!(authority.state(), before);
    authority.redo().unwrap();
    assert_eq!(authority.bookmark().unwrap(), after);
    assert_eq!(authority.state(), &after_state);
}

#[test]
fn star_civilization_deposit_and_hazard_caps_reject_whole_batches_then_reclaim_and_replay() {
    use nyon::living_creator::LivingCreatorDraft;
    use nyon_workshop_core::living::*;
    // Pin the accepted rules table independently of implementation constants.
    assert_eq!(LIVING_MAX_STARS_V2, 128);
    assert_eq!(LIVING_MAX_CIVILIZATIONS_V2, 16);
    assert_eq!(LIVING_MAX_DEPOSITS_V2, 1024);
    assert_eq!(LIVING_MAX_HAZARDS_V2, 128);
    for tool in [Tool::Star, Tool::Civilization, Tool::Deposit, Tool::Hazard] {
        let baseline = builtin_first_expansion_v2().unwrap();
        let mut state = baseline.state().clone();
        let mut operation = LivingCreatorDraft::new(tool, baseline.state(), None)
            .operations()
            .unwrap()
            .remove(0);
        let removed = match tool {
            Tool::Star => {
                let LivingCreatorOperationV2::CreateStar {
                    system: LivingCommandTargetV2::Existing { id: system },
                    archetype,
                    name,
                    ..
                } = &operation
                else {
                    panic!("star form");
                };
                let prototype = LivingStarV2 {
                    id: fixture_id(1, 0),
                    system: *system,
                    archetype: archetype.clone(),
                    name: name.clone(),
                };
                let mut rows = state.stars.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_STARS_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(1, index);
                    rows.push(row);
                }
                state.stars = ordered_rows(rows);
                fixture_id(1, LIVING_MAX_STARS_V2 - 1)
            }
            Tool::Civilization => {
                let prototype = state.civilizations.as_slice()[0].clone();
                let mut rows = state.civilizations.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_CIVILIZATIONS_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(2, index);
                    row.status = LivingCivilizationStatusV2::Dormant;
                    rows.push(row);
                }
                state.civilizations = ordered_rows(rows);
                fixture_id(2, LIVING_MAX_CIVILIZATIONS_V2 - 1)
            }
            Tool::Deposit => {
                let prototype = state.deposits.as_slice()[0].clone();
                let mut rows = state.deposits.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_DEPOSITS_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(3, index);
                    row.remaining_units = 1;
                    rows.push(row);
                }
                state.deposits = ordered_rows(rows);
                fixture_id(3, LIVING_MAX_DEPOSITS_V2 - 1)
            }
            Tool::Hazard => {
                let LivingCreatorOperationV2::CreateHazard {
                    definition,
                    lane: LivingCommandTargetV2::Existing { id: lane },
                    start_tick,
                    end_tick,
                    ..
                } = &mut operation
                else {
                    panic!("hazard form");
                };
                *start_tick = LivingTickV2(1000);
                *end_tick = LivingTickV2(2000);
                state.hazards = ordered_rows(
                    (0..LIVING_MAX_HAZARDS_V2)
                        .map(|index| LivingHazardV2 {
                            id: fixture_id(4, index),
                            definition: definition.clone(),
                            lane: *lane,
                            start_tick: *start_tick,
                            end_tick: *end_tick,
                        })
                        .collect(),
                );
                fixture_id(4, LIVING_MAX_HAZARDS_V2 - 1)
            }
            _ => unreachable!(),
        };
        let mut authority = public_fixture_authority(state, "creator_capacity_fixture");
        let before = authority.state().clone();
        let archive = encode_living_archive_v2(&authority).unwrap();
        let world = &before.worlds.as_slice()[0];
        let mut inventory = world.inventory;
        inventory.energy += 1;
        assert!(
            creator_transaction(
                &mut authority,
                vec![
                    LivingCreatorOperationV2::SetWorldInventory {
                        world: LivingCommandTargetV2::Existing { id: world.id },
                        inventory
                    },
                    operation.clone(),
                ]
            )
            .is_err(),
            "{} overflow",
            tool.label()
        );
        assert_eq!(
            authority.state(),
            &before,
            "{} batch rollback",
            tool.label()
        );
        assert_eq!(
            encode_living_archive_v2(&authority).unwrap(),
            archive,
            "{} history/high-water rollback",
            tool.label()
        );
        creator_transaction(
            &mut authority,
            vec![LivingCreatorOperationV2::RemoveEntity {
                target: LivingCommandTargetV2::Existing { id: removed },
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            }],
        )
        .unwrap();
        let reclaimed = authority.state().clone();
        creator_transaction(&mut authority, vec![operation]).unwrap();
        exact_undo_redo_reload(&mut authority, &reclaimed);
        authority
            .step()
            .expect("reclaimed maximum remains a valid runnable boundary");
    }
}

#[test]
fn world_lane_and_fleet_caps_reject_atomically_and_reclaim_through_exact_history() {
    use nyon::living_creator::LivingCreatorDraft;
    use nyon_workshop_core::living::*;
    assert_eq!(LIVING_MAX_WORLDS_V2, 512);
    assert_eq!(LIVING_MAX_LANES_V2, 256);
    assert_eq!(LIVING_MAX_FLEETS_V2, 256);
    for tool in [Tool::World, Tool::Lane, Tool::Fleet] {
        let baseline = builtin_first_expansion_v2().unwrap();
        let mut state = baseline.state().clone();
        let operation = LivingCreatorDraft::new(tool, baseline.state(), None)
            .operations()
            .unwrap()
            .remove(0);
        let removed = match tool {
            Tool::World => {
                let mut prototype = state.worlds.as_slice()[0].clone();
                prototype.owner = None;
                prototype.inventory = LivingInventoryV2::default();
                let mut rows = state.worlds.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_WORLDS_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(10, index);
                    rows.push(row);
                }
                state.worlds = ordered_rows(rows);
                fixture_id(10, LIVING_MAX_WORLDS_V2 - 1)
            }
            Tool::Lane => {
                let prototype = state.lanes.as_slice()[0].clone();
                let mut rows = state.lanes.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_LANES_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(11, index);
                    rows.push(row);
                }
                state.lanes = ordered_rows(rows);
                fixture_id(11, LIVING_MAX_LANES_V2 - 1)
            }
            Tool::Fleet => {
                let mut prototype = state.fleets.as_slice()[0].clone();
                prototype.order = None;
                prototype.hulls = LivingSortedVecV2::new(Vec::new()).unwrap();
                let mut rows = state.fleets.as_slice().to_vec();
                for index in rows.len()..LIVING_MAX_FLEETS_V2 {
                    let mut row = prototype.clone();
                    row.id = fixture_id(12, index);
                    rows.push(row);
                }
                state.fleets = ordered_rows(rows);
                fixture_id(12, LIVING_MAX_FLEETS_V2 - 1)
            }
            _ => unreachable!(),
        };
        let mut authority = public_fixture_authority(state, "creator_topology_capacity");
        let before = authority.state().clone();
        let archive = encode_living_archive_v2(&authority).unwrap();
        let world = &before.worlds.as_slice()[0];
        let mut inventory = world.inventory;
        inventory.energy += 1;
        assert!(
            creator_transaction(
                &mut authority,
                vec![
                    LivingCreatorOperationV2::SetWorldInventory {
                        world: LivingCommandTargetV2::Existing { id: world.id },
                        inventory,
                    },
                    operation.clone(),
                ],
            )
            .is_err(),
            "{} overflow",
            tool.label()
        );
        assert_eq!(authority.state(), &before, "{} rollback", tool.label());
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), archive);
        creator_transaction(
            &mut authority,
            vec![LivingCreatorOperationV2::RemoveEntity {
                target: LivingCommandTargetV2::Existing { id: removed },
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            }],
        )
        .unwrap();
        let reclaimed = authority.state().clone();
        creator_transaction(&mut authority, vec![operation]).unwrap();
        exact_undo_redo_reload(&mut authority, &reclaimed);
        authority
            .step()
            .expect("maximum remains runnable after reclaim");
    }
}

#[test]
fn six_industrial_slots_include_reserved_jobs_and_same_batch_cancel_releases_capacity() {
    use nyon_workshop_core::living::*;
    let mut authority = builtin_first_expansion_v2().unwrap();
    let home = authority.state().colonies.as_slice()[0].world;
    let owner = authority.state().colonies.as_slice()[0].owner;
    let existing = authority
        .state()
        .facilities
        .as_slice()
        .iter()
        .filter(|row| row.world == home)
        .count();
    let facility = |local| LivingCreatorOperationV2::CreateFacility {
        local,
        world: LivingCommandTargetV2::Existing { id: home },
        definition: LivingSlugV2::new("solar_array").unwrap(),
        status: LivingFacilityStatusV2::Operational,
        paid_alloy: 0,
        hit_points: 0,
        next_due: Some(LivingTickV2(10)),
    };
    creator_transaction(
        &mut authority,
        (0..5 - existing)
            .map(|index| facility(index as u16))
            .collect(),
    )
    .unwrap();
    let construction = LivingCreatorOperationV2::CreateConstructionJob {
        local: 0,
        world: LivingCommandTargetV2::Existing { id: home },
        owner: LivingCommandTargetV2::Existing { id: owner },
        definition: LivingSlugV2::new("foundry").unwrap(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(200),
        paid_alloy: 20,
    };
    creator_transaction(&mut authority, vec![construction.clone()]).unwrap();
    let reserved = authority.state().clone();
    let archive = encode_living_archive_v2(&authority).unwrap();
    for operation in [facility(0), construction] {
        let mut inventory = reserved
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == home)
            .unwrap()
            .inventory;
        inventory.ore += 1;
        assert!(
            creator_transaction(
                &mut authority,
                vec![
                    LivingCreatorOperationV2::SetWorldInventory {
                        world: LivingCommandTargetV2::Existing { id: home },
                        inventory
                    },
                    operation,
                ]
            )
            .is_err()
        );
        assert_eq!(authority.state(), &reserved);
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), archive);
    }
    let job = reserved.construction_jobs.as_slice()[0].id;
    creator_transaction(
        &mut authority,
        vec![
            LivingCreatorOperationV2::CancelConstructionJob {
                target: LivingCommandTargetV2::Existing { id: job },
            },
            facility(0),
        ],
    )
    .unwrap();
    assert!(authority.state().construction_jobs.as_slice().is_empty());
    assert_eq!(
        authority
            .state()
            .facilities
            .as_slice()
            .iter()
            .filter(|row| row.world == home)
            .count(),
        6
    );
    exact_undo_redo_reload(&mut authority, &reserved);
}

#[test]
fn reviewed_system_cascade_rejects_omitted_and_unrelated_keys_then_restores_exact_history() {
    use nyon_workshop_core::living::*;
    use std::collections::BTreeSet;
    let mut authority = LivingContent::ValeConfluence.open().unwrap();
    let state = authority.state().clone();
    let system = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == "Aster Vale")
        .unwrap()
        .system;
    let worlds = state
        .worlds
        .as_slice()
        .iter()
        .filter(|world| world.system == system)
        .map(|world| world.id)
        .collect::<BTreeSet<_>>();
    let expected = state
        .stars
        .as_slice()
        .iter()
        .filter(|star| star.system == system)
        .map(|star| star.id)
        .chain(worlds.iter().copied())
        .chain(
            state
                .lanes
                .as_slice()
                .iter()
                .filter(|lane| lane.system_a == system || lane.system_b == system)
                .map(|lane| lane.id),
        )
        .chain(
            state
                .deposits
                .as_slice()
                .iter()
                .filter(|deposit| worlds.contains(&deposit.world))
                .map(|deposit| deposit.id),
        )
        .chain(
            state
                .facilities
                .as_slice()
                .iter()
                .filter(|facility| worlds.contains(&facility.world))
                .map(|facility| facility.id),
        )
        .collect::<BTreeSet<_>>();
    let preview = preview_living_removal_v2(&state, system).unwrap();
    let dependent_ids = preview.dependents.iter().copied().collect::<BTreeSet<_>>();
    assert!(
        expected.is_subset(&dependent_ids),
        "worlds, stars, connected lanes, deposits and industries must all be reviewed"
    );
    let derived = preview_living_derived_removal_v2(&state, system).unwrap();
    assert!(
        !derived.is_empty(),
        "owned colony/intelligence references are explicitly reviewed"
    );
    let reviewed = LivingCascadeDispositionV2::RemoveReviewed {
        dependents: preview
            .dependents
            .iter()
            .map(|id| LivingCommandTargetV2::Existing { id: *id })
            .collect(),
        derived: derived.clone(),
    };
    let archive = encode_living_archive_v2(&authority).unwrap();
    let mut omitted = reviewed.clone();
    if let LivingCascadeDispositionV2::RemoveReviewed { dependents, .. } = &mut omitted {
        dependents.pop();
    }
    let mut unrelated = reviewed.clone();
    if let LivingCascadeDispositionV2::RemoveReviewed { dependents, .. } = &mut unrelated {
        let unrelated = state
            .systems
            .as_slice()
            .iter()
            .find(|row| row.id != system)
            .unwrap()
            .id;
        dependents.push(LivingCommandTargetV2::Existing { id: unrelated });
        dependents.sort_by_key(|target| match target {
            LivingCommandTargetV2::Existing { id } => *id,
            _ => unreachable!(),
        });
    }
    let mut missing_derived = reviewed.clone();
    if let LivingCascadeDispositionV2::RemoveReviewed { derived, .. } = &mut missing_derived {
        derived.pop();
    }
    let mut unrelated_derived = reviewed.clone();
    if let LivingCascadeDispositionV2::RemoveReviewed { derived, .. } = &mut unrelated_derived {
        let retained = state
            .colonies
            .as_slice()
            .iter()
            .find(|row| !worlds.contains(&row.world))
            .unwrap();
        let key = LivingDerivedRemovalV2::Colony {
            world: retained.world,
            owner: retained.owner,
        };
        assert!(!derived.contains(&key));
        derived.push(key);
        derived.sort();
    }
    for cascade in [
        LivingCascadeDispositionV2::RejectIfReferenced,
        omitted,
        unrelated,
        missing_derived,
        unrelated_derived,
    ] {
        let retained_world = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| !worlds.contains(&world.id))
            .unwrap();
        let mut inventory = retained_world.inventory;
        inventory.energy += 1;
        assert!(
            creator_transaction(
                &mut authority,
                vec![
                    LivingCreatorOperationV2::SetWorldInventory {
                        world: LivingCommandTargetV2::Existing {
                            id: retained_world.id
                        },
                        inventory
                    },
                    LivingCreatorOperationV2::RemoveEntity {
                        target: LivingCommandTargetV2::Existing { id: system },
                        cascade
                    },
                ]
            )
            .is_err()
        );
        assert_eq!(authority.state(), &state);
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), archive);
    }
    creator_transaction(
        &mut authority,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: LivingCommandTargetV2::Existing { id: system },
            cascade: reviewed,
        }],
    )
    .unwrap();
    assert!(
        !authority
            .state()
            .systems
            .as_slice()
            .iter()
            .any(|row| row.id == system)
    );
    assert!(
        !authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .any(|row| worlds.contains(&row.id))
    );
    assert!(
        !authority
            .state()
            .lanes
            .as_slice()
            .iter()
            .any(|lane| lane.system_a == system || lane.system_b == system)
    );
    for world in state
        .worlds
        .as_slice()
        .iter()
        .filter(|world| !worlds.contains(&world.id))
    {
        assert_eq!(
            authority
                .state()
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == world.id),
            Some(world)
        );
    }
    authority.state().validate(authority.catalog()).unwrap();
    exact_undo_redo_reload(&mut authority, &state);
}

#[test]
fn global_hull_capacity_counts_paid_reservations_and_releases_only_after_cancel() {
    use nyon_workshop_core::living::*;
    assert_eq!(LIVING_MAX_HULLS_V2, 2048);
    assert_eq!(LIVING_MAX_HULLS_PER_FLEET_V2, 16);
    let baseline = builtin_first_expansion_v2().unwrap();
    let mut state = baseline.state().clone();
    let prototype = state.fleets.as_slice()[0].clone();
    let hull = prototype.hulls.as_slice()[0].clone();
    let reserved_fleet = prototype.id;
    let empty_fleet = fixture_id(6, 128);
    let mut next_hull = 0;
    let mut fleets = Vec::new();
    for index in 0..128 {
        let mut fleet = prototype.clone();
        if index != 0 {
            fleet.id = fixture_id(6, index);
        }
        fleet.hulls = ordered_rows(
            (0..if index == 0 { 15 } else { 16 })
                .map(|_| {
                    let mut row = hull.clone();
                    row.id = fixture_id(5, next_hull);
                    next_hull += 1;
                    row
                })
                .collect(),
        );
        fleets.push(fleet);
    }
    let mut empty = prototype.clone();
    empty.id = empty_fleet;
    empty.hulls = LivingSortedVecV2::default();
    fleets.push(empty);
    state.fleets = ordered_rows(fleets);
    let home = state.colonies.as_slice()[0].world;
    let job = fixture_id(7, 0);
    state.hull_jobs = ordered_rows(vec![LivingHullJobV2 {
        id: job,
        world: home,
        owner: prototype.owner,
        definition: LivingSlugV2::new("escort").unwrap(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(200),
        paid_alloy: 20,
        target_fleet: Some(reserved_fleet),
    }]);
    assert_eq!(
        next_hull + state.hull_jobs.as_slice().len(),
        LIVING_MAX_HULLS_V2
    );
    let mut authority = public_fixture_authority(state, "creator_global_hull_fixture");
    let before = authority.state().clone();
    let archive = encode_living_archive_v2(&authority).unwrap();
    let create = LivingCreatorOperationV2::CreateHull {
        local: 0,
        fleet: LivingCommandTargetV2::Existing { id: empty_fleet },
        definition: LivingSlugV2::new("escort").unwrap(),
        hit_points: 10,
        return_credit: false,
    };
    let source_hull = before
        .fleets
        .as_slice()
        .iter()
        .find(|row| row.id == reserved_fleet)
        .unwrap()
        .hulls
        .as_slice()[0]
        .id;
    for operation in [
        create.clone(),
        LivingCreatorOperationV2::SplitFleet {
            local: 0,
            source: LivingCommandTargetV2::Existing { id: reserved_fleet },
            hulls: vec![LivingCommandTargetV2::Existing { id: source_hull }],
        },
    ] {
        assert!(creator_transaction(&mut authority, vec![operation]).is_err());
        assert_eq!(authority.state(), &before);
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), archive);
    }
    creator_transaction(
        &mut authority,
        vec![
            LivingCreatorOperationV2::CancelHullJob {
                target: LivingCommandTargetV2::Existing { id: job },
            },
            create,
        ],
    )
    .unwrap();
    assert!(authority.state().hull_jobs.as_slice().is_empty());
    assert_eq!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .map(|row| row.hulls.as_slice().len())
            .sum::<usize>(),
        LIVING_MAX_HULLS_V2
    );
    assert_eq!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .find(|row| row.id == empty_fleet)
            .unwrap()
            .hulls
            .as_slice()
            .len(),
        1
    );
    exact_undo_redo_reload(&mut authority, &before);
    authority
        .step()
        .expect("maximum actual hull population remains valid after rejection/reclamation");
}
