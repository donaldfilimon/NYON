//! Creator counterfactuals and portable Living history through public APIs.
use nyon_workshop_core::living::*;

#[test]
fn paid_freight_return_rejects_authentic_pre_correction_archive() {
    let baseline = builtin_living_sandbox_v2().unwrap();
    let old = baseline.state();
    let source = old.worlds.as_slice()[0].clone();
    let destination = old.worlds.as_slice()[1].clone();
    let origin_system = old.systems.as_slice()[0].clone();
    let destination_system = LivingSystemV2 {
        id: LivingEntityIdV2([0xf0; 16]),
        name: LivingNameV2::new("Freight destination").unwrap(),
    };
    let lane = LivingLaneV2 {
        id: LivingEntityIdV2([0xf1; 16]),
        system_a: origin_system.id,
        system_b: destination_system.id,
        distance_units: 100,
    };
    let mut source = source;
    source.inventory = LivingInventoryV2 {
        energy: 0,
        ore: 30,
        alloy: 0,
    };
    let mut destination = destination;
    destination.system = destination_system.id;
    destination.inventory = LivingInventoryV2::default();
    let civ_a = source.owner.unwrap();
    let civ_b = destination.owner.unwrap();
    let state = LivingGalaxyStateV2 {
        systems: LivingSortedVecV2::new(vec![origin_system, destination_system]).unwrap(),
        stars: old.stars.clone(),
        worlds: LivingSortedVecV2::new(vec![source.clone(), destination.clone()]).unwrap(),
        lanes: LivingSortedVecV2::new(vec![lane.clone()]).unwrap(),
        civilizations: old.civilizations.clone(),
        colonies: LivingSortedVecV2::new(old.colonies.as_slice()[..2].to_vec()).unwrap(),
        routes: LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
            id: LivingEntityIdV2([0xf2; 16]),
            kind: LivingRouteKindV2::Aid,
            source_world: source.id,
            destination_world: destination.id,
            resource: LivingResourceV2::Ore,
            batch_size: 10,
            cadence_ticks: 10,
            source_reserve: 20,
            source_owner: civ_a,
            receiver_owner: civ_b,
            next_due: LivingTickV2(10),
            suspended: false,
        }])
        .unwrap(),
        ..Default::default()
    };
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        &catalog,
    )
    .unwrap();
    let mut authority = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa3; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("freight_return_lane_edit_capture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    step_to(&mut authority, 10);
    let outbound = &authority.state().shipments.as_slice()[0];
    assert_eq!(
        (
            outbound.departure_tick,
            outbound.arrival_tick,
            outbound.units
        ),
        (LivingTickV2(10), LivingTickV2(20), 10)
    );
    assert_eq!(authority.state().worlds.as_slice()[0].inventory.ore, 20);
    let cursor = authority.published_cursor();
    authority
        .submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![
                    LivingCreatorOperationV2::EditLane {
                        target: LivingCommandTargetV2::Existing { id: lane.id },
                        system_a: LivingCommandTargetV2::Existing { id: lane.system_a },
                        system_b: LivingCommandTargetV2::Existing { id: lane.system_b },
                        distance_units: 200,
                    },
                    LivingCreatorOperationV2::ForceWar {
                        participant_a: LivingCommandTargetV2::Existing { id: civ_a },
                        participant_b: LivingCommandTargetV2::Existing { id: civ_b },
                        declarer: LivingCommandTargetV2::Existing { id: civ_a },
                        start_tick: LivingTickV2(11),
                        end_tick: LivingTickV2(100),
                    },
                ],
            },
        })
        .unwrap();
    step_to(&mut authority, 20);
    let returning = &authority.state().shipments.as_slice()[0];
    assert_eq!(
        (
            returning.departure_tick,
            returning.arrival_tick,
            returning.units
        ),
        (LivingTickV2(20), LivingTickV2(30), 10)
    );
    assert_eq!(
        returning.disposition,
        LivingShipmentDispositionV2::Returning
    );
    assert_eq!(authority.state().worlds.as_slice()[0].inventory.ore, 20);
    let bytes = encode_living_archive_v2(&authority).unwrap();
    assert_eq!(
        decode_living_archive_v2(&bytes, authority.catalog())
            .unwrap()
            .canonical_bytes(),
        bytes
    );
    let old =
        include_bytes!("fixtures/living-v2-history/freight-return-lane-edit-pre-correction.json");
    assert_eq!(old.len(), 3886);
    use sha2::{Digest, Sha256};
    assert_eq!(
        format!("{:x}", Sha256::digest(old)),
        "f2f3a6022d57d36fef31a616611115ac25ce4aa3608267c030aeb3fc0f5e5482"
    );
    assert_ne!(old.as_slice(), bytes.as_slice());
    assert!(matches!(
        decode_living_archive_v2(old, authority.catalog()),
        Err(LivingArchiveErrorV2::FinalDigest)
    ));
}

fn admitted_freight_control_genesis() -> (
    LivingGalaxyAuthorityV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
) {
    let baseline = builtin_living_sandbox_v2().unwrap();
    let old = baseline.state();
    let mut source = old.worlds.as_slice()[0].clone();
    let mut destination = old.worlds.as_slice()[1].clone();
    let source_system = old.systems.as_slice()[0].clone();
    let destination_system = LivingSystemV2 {
        id: LivingEntityIdV2([0xe0; 16]),
        name: LivingNameV2::new("Control destination").unwrap(),
    };
    source.inventory = LivingInventoryV2 {
        energy: 0,
        ore: 30,
        alloy: 0,
    };
    destination.system = destination_system.id;
    destination.inventory = LivingInventoryV2::default();
    let civ_a = source.owner.unwrap();
    let civ_b = destination.owner.unwrap();
    let route_id = LivingEntityIdV2([0xe2; 16]);
    let colonies = old.colonies.as_slice()[..2].to_vec();
    let state = LivingGalaxyStateV2 {
        systems: LivingSortedVecV2::new(vec![source_system.clone(), destination_system.clone()])
            .unwrap(),
        stars: old.stars.clone(),
        worlds: LivingSortedVecV2::new(vec![source.clone(), destination.clone()]).unwrap(),
        lanes: LivingSortedVecV2::new(vec![LivingLaneV2 {
            id: LivingEntityIdV2([0xe1; 16]),
            system_a: source_system.id,
            system_b: destination_system.id,
            distance_units: 100,
        }])
        .unwrap(),
        civilizations: old.civilizations.clone(),
        colonies: LivingSortedVecV2::new(colonies).unwrap(),
        routes: LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
            id: route_id,
            kind: LivingRouteKindV2::Aid,
            source_world: source.id,
            destination_world: destination.id,
            resource: LivingResourceV2::Ore,
            batch_size: 10,
            cadence_ticks: 10,
            source_reserve: 20,
            source_owner: civ_a,
            receiver_owner: civ_b,
            next_due: LivingTickV2(10),
            suspended: false,
        }])
        .unwrap(),
        ..Default::default()
    };
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        &catalog,
    )
    .unwrap();
    let authority = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa4; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("freight_public_controls").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    (authority, source.id, destination.id, route_id, civ_a, civ_b)
}

fn submit_running_freight_controls(
    authority: &mut LivingGalaxyAuthorityV2,
    operations: Vec<LivingCreatorOperationV2>,
) {
    let cursor = authority.published_cursor();
    authority
        .submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch { operations },
        })
        .unwrap();
}

fn existing(id: LivingEntityIdV2) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Existing { id }
}

fn world_at(state: &LivingGalaxyStateV2, id: LivingEntityIdV2) -> &LivingWorldV2 {
    state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == id)
        .unwrap()
}

fn route_at(state: &LivingGalaxyStateV2, id: LivingEntityIdV2) -> &LivingFreightRouteV2 {
    state
        .routes
        .as_slice()
        .iter()
        .find(|row| row.id == id)
        .unwrap()
}

fn colony_at(state: &LivingGalaxyStateV2, id: LivingEntityIdV2) -> &LivingColonyV2 {
    state
        .colonies
        .as_slice()
        .iter()
        .find(|row| row.world == id)
        .unwrap()
}

#[test]
fn admitted_freight_capacity_wait_then_one_credit_preserves_paid_manifest() {
    let (mut authority, source, destination, route, _, _) = admitted_freight_control_genesis();
    step_to(&mut authority, 10);
    let paid = authority.state().shipments.as_slice()[0].clone();
    assert_eq!(
        (paid.departure_tick, paid.arrival_tick, paid.units),
        (LivingTickV2(10), LivingTickV2(20), 10)
    );
    assert_eq!(world_at(authority.state(), source).inventory.ore, 20);
    submit_running_freight_controls(
        &mut authority,
        vec![
            LivingCreatorOperationV2::SetRouteSuspended {
                target: existing(route),
                suspended: true,
            },
            LivingCreatorOperationV2::SetWorldInventory {
                world: existing(destination),
                inventory: LivingInventoryV2 {
                    energy: 0,
                    ore: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                    alloy: 0,
                },
            },
        ],
    );
    step_to(&mut authority, 11);
    assert!(route_at(authority.state(), route).suspended);
    assert_eq!(
        world_at(authority.state(), destination).inventory.ore,
        LIVING_RESOURCE_STORAGE_LIMIT_V2
    );
    step_to(&mut authority, 20);
    let waiting = &authority.state().shipments.as_slice()[0];
    assert_eq!(waiting.id, paid.id);
    assert_eq!(
        waiting.disposition,
        LivingShipmentDispositionV2::WaitingForStorage
    );
    assert!(!waiting.returning_to_source);
    assert_eq!(
        (waiting.departure_tick, waiting.arrival_tick, waiting.units),
        (LivingTickV2(10), LivingTickV2(20), 10)
    );
    assert_eq!(world_at(authority.state(), source).inventory.ore, 20);
    submit_running_freight_controls(
        &mut authority,
        vec![LivingCreatorOperationV2::SetWorldInventory {
            world: existing(destination),
            inventory: LivingInventoryV2 {
                energy: 0,
                ore: LIVING_RESOURCE_STORAGE_LIMIT_V2 - 10,
                alloy: 0,
            },
        }],
    );
    step_to(&mut authority, 21);
    assert!(authority.state().shipments.as_slice().is_empty());
    assert_eq!(
        world_at(authority.state(), destination).inventory.ore,
        LIVING_RESOURCE_STORAGE_LIMIT_V2
    );
    assert_eq!(world_at(authority.state(), source).inventory.ore, 20);
    assert_eq!(
        authority
            .last_receipt()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.kind == LivingEventKindV2::ShipmentDelivered)
            .count(),
        1
    );
}

#[test]
fn admitted_owner_transfer_updates_colony_and_captures_paid_cargo() {
    let (mut authority, source, destination, route, civ_a, _) = admitted_freight_control_genesis();
    step_to(&mut authority, 10);
    let paid = authority.state().shipments.as_slice()[0].clone();
    submit_running_freight_controls(
        &mut authority,
        vec![
            LivingCreatorOperationV2::SetRouteSuspended {
                target: existing(route),
                suspended: true,
            },
            LivingCreatorOperationV2::SetWorldOwner {
                world: existing(destination),
                owner: Some(existing(civ_a)),
            },
        ],
    );
    step_to(&mut authority, 11);
    assert_eq!(world_at(authority.state(), destination).owner, Some(civ_a));
    assert_eq!(colony_at(authority.state(), destination).owner, civ_a);
    step_to(&mut authority, 20);
    assert!(authority.state().shipments.as_slice().is_empty());
    assert_eq!(world_at(authority.state(), destination).inventory.ore, 10);
    assert_eq!(world_at(authority.state(), source).inventory.ore, 20);
    assert_eq!(paid.units, 10);
    assert_eq!(
        authority
            .last_receipt()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.kind == LivingEventKindV2::ShipmentCaptured)
            .count(),
        1
    );
}

#[test]
fn admitted_war_return_waits_at_full_source_then_credits_once() {
    let (mut authority, source, destination, route, civ_a, civ_b) =
        admitted_freight_control_genesis();
    step_to(&mut authority, 10);
    let paid = authority.state().shipments.as_slice()[0].clone();
    submit_running_freight_controls(
        &mut authority,
        vec![
            LivingCreatorOperationV2::SetRouteSuspended {
                target: existing(route),
                suspended: true,
            },
            LivingCreatorOperationV2::SetWorldInventory {
                world: existing(source),
                inventory: LivingInventoryV2 {
                    energy: 0,
                    ore: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                    alloy: 0,
                },
            },
            LivingCreatorOperationV2::ForceWar {
                participant_a: existing(civ_a),
                participant_b: existing(civ_b),
                declarer: existing(civ_a),
                start_tick: LivingTickV2(11),
                end_tick: LivingTickV2(100),
            },
        ],
    );
    step_to(&mut authority, 20);
    let returning = &authority.state().shipments.as_slice()[0];
    assert_eq!(returning.id, paid.id);
    assert_eq!(
        (
            returning.departure_tick,
            returning.arrival_tick,
            returning.units
        ),
        (LivingTickV2(20), LivingTickV2(30), 10)
    );
    assert_eq!(
        returning.disposition,
        LivingShipmentDispositionV2::Returning
    );
    assert_eq!(world_at(authority.state(), destination).inventory.ore, 0);
    step_to(&mut authority, 30);
    let waiting = &authority.state().shipments.as_slice()[0];
    assert_eq!(waiting.id, paid.id);
    assert_eq!(
        waiting.disposition,
        LivingShipmentDispositionV2::WaitingForStorage
    );
    assert!(waiting.returning_to_source);
    assert_eq!(
        world_at(authority.state(), source).inventory.ore,
        LIVING_RESOURCE_STORAGE_LIMIT_V2
    );
    submit_running_freight_controls(
        &mut authority,
        vec![LivingCreatorOperationV2::SetWorldInventory {
            world: existing(source),
            inventory: LivingInventoryV2 {
                energy: 0,
                ore: LIVING_RESOURCE_STORAGE_LIMIT_V2 - 10,
                alloy: 0,
            },
        }],
    );
    step_to(&mut authority, 31);
    assert!(authority.state().shipments.as_slice().is_empty());
    assert_eq!(
        world_at(authority.state(), source).inventory.ore,
        LIVING_RESOURCE_STORAGE_LIMIT_V2
    );
    assert_eq!(
        authority
            .last_receipt()
            .unwrap()
            .events
            .iter()
            .filter(|e| e.kind == LivingEventKindV2::ShipmentReturned)
            .count(),
        1
    );
}

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

#[test]
fn authentic_pre_correction_repair_archive_rejects_final_digest_without_installation() {
    use sha2::{Digest, Sha256};
    let original = include_bytes!("fixtures/living-v2-history/overdue-repair-pre-correction.json");
    let hash = Sha256::digest(original);
    assert_eq!(
        format!("{hash:x}"),
        "76e2ab0287864f3ed5c2f72260dc127df0f1a4ce53fac9a10ae5451fcda9fe86"
    );
    let mut active = builtin_living_sandbox_v2().unwrap();
    active
        .submit(edit(&active, LivingCommandModeV2::Running, -30))
        .unwrap();
    let mut control = active.clone();
    let capture = active.capture_boundary().unwrap();
    let before = active.state().canonical_bytes().unwrap();
    let cursor = active.published_cursor();
    let high = active.accepted_sequence_high_water();
    for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
        let mut replay = begin_decode_living_archive_v2(original, active.catalog()).unwrap();
        assert!(matches!(
            replay.take_ready_result(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        assert_eq!(replay.poll(0), Err(LivingArchiveErrorV2::Budget));
        assert_eq!(replay.poll(1025), Err(LivingArchiveErrorV2::Budget));
        let mut n = 0;
        loop {
            let result = replay.poll(schedule[n % schedule.len()]);
            n += 1;
            match result {
                Ok(LivingArchiveReplayStatusV2::Pending) => {}
                Err(LivingArchiveErrorV2::FinalDigest) => break,
                other => panic!("authentic old archive unexpectedly admitted: {other:?}"),
            }
            assert!(n < 100000);
        }
        assert_eq!(replay.poll(1), Err(LivingArchiveErrorV2::FinalDigest));
        assert!(matches!(
            replay.take_ready_result(),
            Err(LivingArchiveErrorV2::FinalDigest)
        ));
        replay.cancel();
        assert!(matches!(
            replay.take_ready_result(),
            Err(LivingArchiveErrorV2::Cancelled)
        ));
        let mut retired =
            replay.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        let mut cleanup = 0;
        loop {
            let budget = schedule[cleanup % schedule.len()];
            cleanup += 1;
            let progress = retired.poll(budget).unwrap();
            assert!(retired.last_poll_visits() <= budget);
            if progress == LivingPublicationRetirementProgressV2::Ready {
                break;
            }
            assert_eq!(progress, LivingPublicationRetirementProgressV2::Pending);
            assert!(cleanup < 100000);
        }
        assert_eq!(
            retired.poll(1).unwrap(),
            LivingPublicationRetirementProgressV2::Ready
        );
        assert_eq!(retired.last_poll_visits(), 0);
        assert_eq!(Sha256::digest(original), hash);
        assert_eq!(active.state().canonical_bytes().unwrap(), before);
        assert_eq!(active.published_cursor(), cursor);
        assert_eq!(active.accepted_sequence_high_water(), high);
        assert_eq!(active.capture_boundary().unwrap().pending_count(), 1);
        assert!(active.boundary_capture_is_current(&capture));
    }
    active.step().unwrap();
    control.step().unwrap();
    assert_eq!(active.state(), control.state());
    assert_eq!(active.last_receipt(), control.last_receipt());
}
