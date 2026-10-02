//! Additive creator transactions preserve legacy wire and atomic authority.
use nyon_workshop_core::living::*;
fn t(id: LivingEntityIdV2) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Existing { id }
}
fn request(
    a: &LivingGalaxyAuthorityV2,
    operations: Vec<LivingCreatorOperationV2>,
    mode: LivingCommandModeV2,
) -> LivingCommandEnvelopeV2 {
    let cursor = a.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch { operations },
    }
}
fn apply(a: &mut LivingGalaxyAuthorityV2, operations: Vec<LivingCreatorOperationV2>) {
    let e = request(a, operations, LivingCommandModeV2::Paused);
    a.apply_paused(e).unwrap();
}
fn reload(a: &LivingGalaxyAuthorityV2) {
    let bytes = encode_living_archive_v2(a).unwrap();
    let loaded = decode_living_archive_v2(&bytes, a.catalog())
        .unwrap()
        .into_authority();
    assert_eq!(loaded.bookmark().unwrap(), a.bookmark().unwrap());
    assert_eq!(loaded.state(), a.state());
}
#[test]
fn edits_preserve_ids_and_replay_after_nonzero_boundary() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    for _ in 0..5 {
        a.step().unwrap();
    }
    let system = a.state().systems.as_slice()[0].clone();
    let star = a.state().stars.as_slice()[0].clone();
    let world = a.state().worlds.as_slice()[0].clone();
    let deposit = a.state().deposits.as_slice()[0].clone();
    let civ = a.state().civilizations.as_slice()[0].clone();
    let before = a.bookmark().unwrap();
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::EditSystem {
                target: t(system.id),
                name: LivingNameV2::new("Renamed System").unwrap(),
            },
            LivingCreatorOperationV2::EditStar {
                target: t(star.id),
                system: t(star.system),
                archetype: star.archetype,
                name: LivingNameV2::new("Renamed Star").unwrap(),
            },
            LivingCreatorOperationV2::EditWorld {
                target: t(world.id),
                system: t(world.system),
                archetype: world.archetype,
                name: LivingNameV2::new("Renamed World").unwrap(),
            },
            LivingCreatorOperationV2::EditDeposit {
                target: t(deposit.id),
                resource: deposit.resource,
                remaining_units: deposit.remaining_units + 1,
            },
            LivingCreatorOperationV2::EditCivilization {
                target: t(civ.id),
                name: LivingNameV2::new("Renamed Civilization").unwrap(),
                policy: civ.policy,
            },
        ],
    );
    assert_eq!(a.state().tick.0, 5);
    assert_eq!(a.state().systems.as_slice()[0].id, system.id);
    reload(&a);
    let after = a.bookmark().unwrap();
    a.undo().unwrap();
    assert_eq!(a.bookmark().unwrap().digest, before.digest);
    a.redo().unwrap();
    assert_eq!(a.bookmark().unwrap(), after);
    let state = a.state().clone();
    let high = a.accepted_sequence_high_water();
    let e = request(
        &a,
        vec![
            LivingCreatorOperationV2::EditSystem {
                target: t(system.id),
                name: LivingNameV2::new("must rollback").unwrap(),
            },
            LivingCreatorOperationV2::EditWorld {
                target: t(world.id),
                system: t(world.system),
                archetype: LivingSlugV2::new("absent_archetype").unwrap(),
                name: world.name,
            },
        ],
        LivingCommandModeV2::Paused,
    );
    assert!(a.apply_paused(e).is_err());
    assert_eq!(a.state(), &state);
    assert_eq!(a.accepted_sequence_high_water(), high);
}
#[test]
fn disable_due_clock_no_catchup_and_scrap_preserve_atomic_history() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    let facility = a
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|f| f.next_due.is_some())
        .unwrap()
        .clone();
    let next = facility.next_due.unwrap();
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::SetFacilityEnabled {
            target: t(facility.id),
            enabled: false,
        }],
    );
    let encoded = a.state().canonical_bytes().unwrap();
    assert!(
        std::str::from_utf8(&encoded)
            .unwrap()
            .contains("\"disabled\":true")
    );
    while a.state().tick < next {
        a.step().unwrap();
    }
    let disabled = a
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|f| f.id == facility.id)
        .unwrap();
    assert!(disabled.disabled);
    assert!(disabled.next_due.unwrap() > next);
    assert!(
        disabled
            .blocked_reasons
            .as_slice()
            .iter()
            .any(|r| matches!(r, LivingBlockedReasonV2::Disabled))
    );
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::SetFacilityEnabled {
            target: t(facility.id),
            enabled: true,
        }],
    );
    reload(&a);
    let before = a.bookmark().unwrap();
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::ScrapFacility {
            target: t(facility.id),
            cascade: LivingCascadeDispositionV2::RejectIfReferenced,
        }],
    );
    assert!(
        !a.state()
            .facilities
            .as_slice()
            .iter()
            .any(|f| f.id == facility.id)
    );
    reload(&a);
    a.undo().unwrap();
    assert_eq!(a.bookmark().unwrap().digest, before.digest);
}
#[test]
fn fleet_split_merge_preserves_hull_ids_hp_credits_and_rejects_bad_membership() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    let world = a
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.owner.is_some())
        .unwrap()
        .clone();
    let owner = world.owner.unwrap();
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::CreateFleet {
                local: 0,
                owner: t(owner),
                world: t(world.id),
            },
            LivingCreatorOperationV2::CreateHull {
                local: 1,
                fleet: LivingCommandTargetV2::Local { local: 0 },
                definition: LivingSlugV2::new("scout").unwrap(),
                hit_points: 1,
                return_credit: true,
            },
            LivingCreatorOperationV2::CreateHull {
                local: 2,
                fleet: LivingCommandTargetV2::Local { local: 0 },
                definition: LivingSlugV2::new("scout").unwrap(),
                hit_points: 1,
                return_credit: false,
            },
        ],
    );
    let fleet = a
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|f| f.owner == owner && f.hulls.as_slice().len() == 2)
        .unwrap()
        .clone();
    let hulls = fleet.hulls.clone();
    let high = a.accepted_sequence_high_water();
    let state = a.state().clone();
    let bad = request(
        &a,
        vec![LivingCreatorOperationV2::SplitFleet {
            local: 0,
            source: t(fleet.id),
            hulls: vec![t(hulls.as_slice()[0].id), t(hulls.as_slice()[0].id)],
        }],
        LivingCommandModeV2::Paused,
    );
    assert!(a.apply_paused(bad).is_err());
    assert_eq!(a.state(), &state);
    assert_eq!(a.accepted_sequence_high_water(), high);
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::SplitFleet {
            local: 0,
            source: t(fleet.id),
            hulls: vec![t(hulls.as_slice()[0].id)],
        }],
    );
    let split = a
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|f| {
            f.id != fleet.id
                && f.hulls
                    .as_slice()
                    .iter()
                    .any(|h| h.id == hulls.as_slice()[0].id)
        })
        .unwrap()
        .id;
    reload(&a);
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::MergeFleets {
            target: t(fleet.id),
            source: t(split),
        }],
    );
    assert_eq!(
        a.state()
            .fleets
            .as_slice()
            .iter()
            .find(|f| f.id == fleet.id)
            .unwrap()
            .hulls,
        hulls
    );
    reload(&a);
}
#[test]
fn exact_reviewed_cascade_is_explicit_and_rejects_incomplete_keys() {
    let mut a = builtin_first_expansion_v2().unwrap();
    let target = a.state().civilizations.as_slice()[0].id;
    let preview = preview_living_removal_v2(a.state(), target).unwrap();
    let derived = preview_living_derived_removal_v2(a.state(), target).unwrap();
    assert!(!derived.is_empty());
    let before = a.state().clone();
    let bad = request(
        &a,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: t(target),
            cascade: LivingCascadeDispositionV2::RemoveReviewed {
                dependents: preview.dependents.iter().copied().map(t).collect(),
                derived: vec![],
            },
        }],
        LivingCommandModeV2::Paused,
    );
    assert_eq!(
        a.apply_paused(bad),
        Err(LivingCommandRejectionV2::CascadeMismatch)
    );
    assert_eq!(a.state(), &before);
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: t(target),
            cascade: LivingCascadeDispositionV2::RemoveReviewed {
                dependents: preview.dependents.into_iter().map(t).collect(),
                derived,
            },
        }],
    );
    assert!(a.state().civilizations.as_slice().is_empty());
    reload(&a);
    for _ in 0..10 {
        a.step().unwrap();
    }
}

#[test]
fn cancellation_refunds_overflow_and_hull_reservations_are_atomic_and_replayable() {
    let mut a = builtin_first_expansion_v2().unwrap();
    let colony = a.state().colonies.as_slice()[0].clone();
    let build = a.catalog().industry("defense_battery").unwrap().clone();
    let hull = a.catalog().hull("scout").unwrap().clone();
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::SetWorldInventory {
                world: t(colony.world),
                inventory: LivingInventoryV2 {
                    energy: 0,
                    ore: 0,
                    alloy: 10000,
                },
            },
            LivingCreatorOperationV2::CreateConstructionJob {
                local: 0,
                world: t(colony.world),
                owner: t(colony.owner),
                definition: build.id,
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(u64::from(build.build_ticks)),
                paid_alloy: build.alloy_cost,
            },
            LivingCreatorOperationV2::CreateHullJob {
                local: 1,
                world: t(colony.world),
                owner: t(colony.owner),
                definition: hull.id,
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(u64::from(hull.build_ticks)),
                paid_alloy: hull.alloy_cost,
                target_fleet: None,
            },
        ],
    );
    let construction = a.state().construction_jobs.as_slice()[0].id;
    let hulljob = a.state().hull_jobs.as_slice()[0].id;
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::CancelConstructionJob {
                target: t(construction),
            },
            LivingCreatorOperationV2::CancelHullJob { target: t(hulljob) },
        ],
    );
    assert!(a.state().construction_jobs.as_slice().is_empty());
    assert!(a.state().hull_jobs.as_slice().is_empty());
    assert_eq!(
        a.state()
            .worlds
            .as_slice()
            .iter()
            .find(|w| w.id == colony.world)
            .unwrap()
            .inventory
            .alloy,
        10000
    );
    reload(&a);
    let state = a.state().clone();
    let e = request(
        &a,
        vec![LivingCreatorOperationV2::CancelHullJob { target: t(hulljob) }],
        LivingCommandModeV2::Paused,
    );
    assert!(a.apply_paused(e).is_err());
    assert_eq!(a.state(), &state);
}
#[test]
fn route_suspend_adjustment_and_hazard_edits_replay_in_running_queue() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    let route = a.state().routes.as_slice()[0].clone();
    let system = a.state().systems.as_slice()[0].id;
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::CreateSystem {
                local: 0,
                name: LivingNameV2::new("New System").unwrap(),
            },
            LivingCreatorOperationV2::CreateLane {
                local: 1,
                system_a: t(system),
                system_b: LivingCommandTargetV2::Local { local: 0 },
                distance_units: 100,
            },
        ],
    );
    let lane = a.state().lanes.as_slice()[0].clone();
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::EditLane {
                target: t(lane.id),
                system_a: t(lane.system_a),
                system_b: t(lane.system_b),
                distance_units: lane.distance_units,
            },
            LivingCreatorOperationV2::SetRouteSuspended {
                target: t(route.id),
                suspended: true,
            },
        ],
    );
    for _ in 0..10 {
        a.step().unwrap();
    }
    assert!(
        a.state()
            .routes
            .as_slice()
            .iter()
            .find(|r| r.id == route.id)
            .unwrap()
            .suspended
    );
    let hazard = a.catalog().hazards()[0].id.clone();
    let e = request(
        &a,
        vec![
            LivingCreatorOperationV2::EditRoute {
                target: t(route.id),
                kind: route.kind,
                source_world: t(route.source_world),
                destination_world: t(route.destination_world),
                resource: route.resource,
                batch_size: route.batch_size,
                cadence_ticks: route.cadence_ticks,
                source_reserve: route.source_reserve,
                source_owner: t(route.source_owner),
                receiver_owner: t(route.receiver_owner),
            },
            LivingCreatorOperationV2::CreateHazard {
                local: 0,
                definition: hazard.clone(),
                lane: t(lane.id),
                start_tick: LivingTickV2(12),
                end_tick: LivingTickV2(100),
            },
        ],
        LivingCommandModeV2::Running,
    );
    a.submit(e).unwrap();
    a.step().unwrap();
    assert!(
        !a.state()
            .routes
            .as_slice()
            .iter()
            .find(|r| r.id == route.id)
            .unwrap()
            .suspended
    );
    let hazardid = a.state().hazards.as_slice()[0].id;
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::EditHazard {
            target: t(hazardid),
            lane: t(lane.id),
            definition: hazard,
            start_tick: LivingTickV2(13),
            end_tick: LivingTickV2(101),
        }],
    );
    reload(&a);
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::CancelHazard {
            target: t(hazardid),
        }],
    );
    assert!(a.state().hazards.as_slice().is_empty());
    reload(&a);
}

#[test]
fn derived_only_cascade_with_no_stable_dependents_is_representable() {
    let mut a = builtin_first_expansion_v2().unwrap();
    let peer = a.state().civilizations.as_slice()[0].clone();
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::CreateCivilization {
                local: 0,
                name: LivingNameV2::new("Only Derived").unwrap(),
                policy: peer.policy,
                status: LivingCivilizationStatusV2::Dormant,
            },
            LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Local { local: 0 },
                to: t(peer.id),
                base_disposition: 5,
            },
        ],
    );
    let target = a
        .state()
        .civilizations
        .as_slice()
        .iter()
        .find(|c| c.name.as_str() == "Only Derived")
        .unwrap()
        .id;
    let preview = preview_living_removal_v2(a.state(), target).unwrap();
    assert!(preview.dependents.is_empty());
    let derived = preview_living_derived_removal_v2(a.state(), target).unwrap();
    assert_eq!(derived.len(), 1);
    let before = a.bookmark().unwrap();
    let legacy = request(
        &a,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: t(target),
            cascade: LivingCascadeDispositionV2::RemoveListed { dependents: vec![] },
        }],
        LivingCommandModeV2::Paused,
    );
    assert!(a.apply_paused(legacy).is_err());
    assert_eq!(a.bookmark().unwrap(), before);
    apply(
        &mut a,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: t(target),
            cascade: LivingCascadeDispositionV2::RemoveReviewed {
                dependents: vec![],
                derived,
            },
        }],
    );
    assert!(
        !a.state()
            .civilizations
            .as_slice()
            .iter()
            .any(|c| c.id == target)
    );
    reload(&a);
}

#[test]
fn additive_command_tags_match_independent_full_bytes_and_revision_formula() {
    let bytes = include_bytes!("fixtures/living-v2-additions/creator-command.json");
    let command: LivingCommandV2 = decode_canonical_v2(bytes, 65536).unwrap();
    command.validate().unwrap();
    assert_eq!(command.canonical_bytes().unwrap(), bytes);
    assert_eq!(command.operations().len(), 19);
    let revision = revision_id_v2(
        LivingCatalogHashV2([0x11; 32]),
        [0; 32],
        None,
        LivingTickV2(50),
        1,
        bytes,
    );
    assert_eq!(
        serde_json::to_string(&revision).unwrap(),
        "\"d1567b31c8cb29246a57b2a642ba4237ef92fa913144e17c7be6c83b7351bcb8\""
    );
    let created = command.created_entities(LivingRevisionIdV2([0; 32]));
    assert_eq!(created.len(), 1);
    assert_eq!(created[0].1, LivingEntityKindV2::Fleet);
}

#[test]
fn fleet_and_hull_edits_validate_health_and_preserve_transit_boundaries() {
    let mut a = builtin_first_expansion_v2().unwrap();
    let fleet = a.state().fleets.as_slice()[0].clone();
    let hull = fleet.hulls.as_slice()[0].clone();
    let home = match fleet.location {
        LivingFleetLocationV2::Docked { world } => world,
        _ => panic!("fixture docked"),
    };
    let before = a.state().clone();
    let bad = request(
        &a,
        vec![LivingCreatorOperationV2::EditHull {
            target: t(hull.id),
            definition: hull.definition.clone(),
            hit_points: 0,
            return_credit: true,
        }],
        LivingCommandModeV2::Paused,
    );
    assert!(a.apply_paused(bad).is_err());
    assert_eq!(a.state(), &before);
    apply(
        &mut a,
        vec![
            LivingCreatorOperationV2::EditHull {
                target: t(hull.id),
                definition: hull.definition,
                hit_points: 1,
                return_credit: true,
            },
            LivingCreatorOperationV2::EditFleet {
                target: t(fleet.id),
                owner: t(fleet.owner),
                world: t(home),
            },
        ],
    );
    let edited = a.state().fleets.as_slice()[0]
        .hulls
        .as_slice()
        .iter()
        .find(|h| h.id == hull.id)
        .unwrap();
    assert_eq!(edited.hit_points, 1);
    assert!(edited.return_credit);
    reload(&a);
    let destination = a
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id != home)
        .unwrap()
        .id;
    let mut state = a.state().clone();
    state.accepted_sequence = 0;
    state.tick = LivingTickV2(0);
    let mut fleets = state.fleets.as_slice().to_vec();
    fleets
        .iter_mut()
        .find(|row| row.id == fleet.id)
        .unwrap()
        .location = LivingFleetLocationV2::Travelling {
        origin_world: home,
        destination_world: destination,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(20),
    };
    state.fleets = LivingSortedVecV2::new(fleets).unwrap();
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
    let mut transit = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("transit_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    let before = transit.state().clone();
    let e = request(
        &transit,
        vec![LivingCreatorOperationV2::EditFleet {
            target: t(fleet.id),
            owner: t(fleet.owner),
            world: t(home),
        }],
        LivingCommandModeV2::Paused,
    );
    assert_eq!(
        transit.apply_paused(e),
        Err(LivingCommandRejectionV2::ReferencedTarget)
    );
    assert_eq!(transit.state(), &before);
}
