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

#[test]
fn covered_edits_and_empty_revisions_keep_old_dormant_actor_without_recovery() {
    let source = builtin_living_sandbox_v2().unwrap();
    let mut state = source.state().clone();
    let actor = LivingEntityIdV2([0xf1; 16]);
    let mut civilizations = state.civilizations.clone().into_vec();
    civilizations.push(LivingCivilizationV2 {
        id: actor,
        name: LivingNameV2::new("Dormant witness").unwrap(),
        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
        status: LivingCivilizationStatusV2::Dormant,
    });
    civilizations.sort_by_key(|row| row.id);
    state.civilizations = LivingSortedVecV2::new(civilizations).unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        source.catalog(),
    )
    .unwrap();
    let mut base = LivingGalaxyAuthorityV2::from_genesis(
        source.catalog().clone(),
        [0x51; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("dormant_witness").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    for operations in [
        vec![],
        vec![LivingCreatorOperationV2::EditCivilization {
            target: t(actor),
            name: LivingNameV2::new("N".repeat(64)).unwrap(),
            policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
        }],
        vec![LivingCreatorOperationV2::CreateCivilization {
            local: 0,
            name: LivingNameV2::new("New Dormant").unwrap(),
            policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
            status: LivingCivilizationStatusV2::Dormant,
        }],
    ] {
        let envelope = request(&base, operations, LivingCommandModeV2::Running);
        base.submit(envelope).unwrap();
    }
    let mut reference = base.clone();
    reference.step().unwrap();
    for budget in [1, 64, 1024] {
        let mut actual = base.clone();
        let mut job = actual.begin_step().unwrap();
        let mut completed = false;
        for _attempt in 0..100_000 {
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            assert_ne!(
                progress,
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            );
            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                completed = true;
                break;
            }
            assert_eq!(progress, LivingStepProgressV2::Pending);
            assert!(
                job.last_poll_visits() > 0,
                "public Step Pending without work"
            );
        }
        assert!(completed, "covered Dormant Step exceeded attempt bound");
        let LivingStepPreparationV2::Ready(prepared) = job.finish_synchronously(&actual).unwrap()
        else {
            panic!("expected ready")
        };
        drop(actual.commit_step(prepared).unwrap());
        assert_eq!(actual.state(), reference.state());
        assert_eq!(actual.last_receipt(), reference.last_receipt());
        assert!(actual.last_creator_recoveries().is_empty());
        assert_eq!(
            actual
                .last_receipt()
                .unwrap()
                .events
                .iter()
                .filter(|event| event.kind == LivingEventKindV2::CreatorIntervention)
                .count(),
            3
        );
        assert_eq!(
            actual
                .state()
                .civilizations
                .as_slice()
                .iter()
                .find(|row| row.id == actor)
                .unwrap()
                .status,
            LivingCivilizationStatusV2::Dormant
        );
    }
}

#[test]
fn colony_public_validated_genesis_admission_recovery_and_reference_partitions() {
    let sandbox = builtin_living_sandbox_v2().unwrap();
    let mut state = sandbox.state().clone();
    let old_actor = state.colonies.as_slice()[0].owner;
    let world = state.colonies.as_slice()[0].world;
    let actor = LivingEntityIdV2([0xf1; 16]);
    let later_actor = LivingEntityIdV2([0xf2; 16]);
    let second_world = state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id != world)
        .unwrap()
        .id;
    let mut civilizations = state.civilizations.clone().into_vec();
    civilizations
        .iter_mut()
        .find(|c| c.id == old_actor)
        .unwrap()
        .status = LivingCivilizationStatusV2::Dormant;
    civilizations.push(LivingCivilizationV2 {
        id: actor,
        name: LivingNameV2::new("Recovery witness").unwrap(),
        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
        status: LivingCivilizationStatusV2::Dormant,
    });
    civilizations.push(LivingCivilizationV2 {
        id: later_actor,
        name: LivingNameV2::new("Later recovery witness").unwrap(),
        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
        status: LivingCivilizationStatusV2::Dormant,
    });
    civilizations.sort_by_key(|c| c.id);
    state.civilizations = LivingSortedVecV2::new(civilizations).unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        sandbox.catalog(),
    )
    .unwrap();
    let base = LivingGalaxyAuthorityV2::from_genesis(
        sandbox.catalog().clone(),
        [0x52; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("colony_recovery").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    for case in 0..9 {
        let mut admitted = base.clone();
        let create = LivingCreatorOperationV2::CreateColony {
            world: t(world),
            owner: t(actor),
        };
        let remove = LivingCreatorOperationV2::RemoveColony { world: t(world) };
        let batches = match case {
            0 => vec![vec![create]],
            1 => vec![vec![LivingCreatorOperationV2::SetWorldOwner {
                world: t(world),
                owner: Some(t(actor)),
            }]],
            2 => vec![vec![LivingCreatorOperationV2::SetWorldOwner {
                world: t(world),
                owner: None,
            }]],
            3 => vec![vec![remove]],
            4 => vec![vec![create, remove]],
            5 => vec![vec![create.clone()], vec![remove], vec![create]],
            6 => {
                let row = base
                    .state()
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|row| row.id == world)
                    .unwrap();
                vec![vec![
                    create,
                    LivingCreatorOperationV2::EditWorld {
                        target: t(world),
                        system: t(row.system),
                        archetype: row.archetype.clone(),
                        name: LivingNameV2::new("Fallback witness").unwrap(),
                    },
                ]]
            }
            7 => vec![vec![
                // Admission orders the larger actor first; recovery orders actors.
                LivingCreatorOperationV2::CreateColony {
                    world: t(second_world),
                    owner: t(later_actor),
                },
                create,
            ]],
            _ => vec![vec![
                LivingCreatorOperationV2::CreateCivilization {
                    local: 0,
                    name: LivingNameV2::new("New local colony owner").unwrap(),
                    policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                    status: LivingCivilizationStatusV2::Dormant,
                },
                LivingCreatorOperationV2::CreateColony {
                    world: t(world),
                    owner: LivingCommandTargetV2::Local { local: 0 },
                },
            ]],
        };
        let revisions = batches.len();
        for batch in batches {
            let request = request(&admitted, batch, LivingCommandModeV2::Running);
            admitted.submit(request).unwrap();
        }
        let mut reference = admitted.clone();
        reference.step().unwrap();
        let expected_actors = match case {
            0 | 1 | 6 => vec![actor],
            5 => vec![actor, actor],
            7 => vec![actor, later_actor],
            _ => vec![],
        };
        assert_eq!(
            reference
                .last_creator_recoveries()
                .iter()
                .map(|p| p.actor())
                .collect::<Vec<_>>(),
            expected_actors
        );
        if case == 8 {
            let created = reference
                .state()
                .civilizations
                .as_slice()
                .iter()
                .find(|row| row.name.as_str() == "New local colony owner")
                .unwrap();
            assert_eq!(created.status, LivingCivilizationStatusV2::Active);
            assert_eq!(
                reference
                    .state()
                    .colonies
                    .as_slice()
                    .iter()
                    .find(|row| row.world == world)
                    .unwrap()
                    .owner,
                created.id
            );
        }
        for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
            let mut actual = admitted.clone();
            let mut job = actual.begin_step().unwrap();
            let mut completed = false;
            for attempt in 0..100_000 {
                let budget = schedule[attempt % schedule.len()];
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
                if progress != LivingStepProgressV2::Pending {
                    completed = true;
                    break;
                }
                assert!(job.last_poll_visits() > 0);
            }
            assert!(completed, "finite public colony Step guard");
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&actual).unwrap()
            else {
                panic!("public colony preparation failed")
            };
            drop(actual.commit_step(prepared).unwrap());
            assert_eq!(
                actual.state().canonical_bytes().unwrap(),
                reference.state().canonical_bytes().unwrap()
            );
            assert_eq!(actual.last_receipt(), reference.last_receipt());
            assert_eq!(actual.last_decision(), reference.last_decision());
            assert_eq!(actual.last_event_facts(), reference.last_event_facts());
            assert_eq!(
                actual.last_creator_recoveries(),
                reference.last_creator_recoveries()
            );
            assert_eq!(actual.revisions(), reference.revisions());
            assert_eq!(actual.published_cursor(), reference.published_cursor());
            assert_eq!(
                actual.accepted_sequence_high_water(),
                reference.accepted_sequence_high_water()
            );
            assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
            assert_eq!(
                actual
                    .last_receipt()
                    .unwrap()
                    .events
                    .iter()
                    .filter(|e| e.kind == LivingEventKindV2::CreatorIntervention)
                    .count(),
                revisions
            );
        }
    }
}

fn diplomacy_public_base() -> LivingGalaxyAuthorityV2 {
    let sandbox = builtin_living_sandbox_v2().unwrap();
    let mut state = sandbox.state().clone();
    let colony = state.colonies.as_slice()[0].clone();
    let origin = state
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.id == colony.world)
        .unwrap()
        .clone();
    let id = |n| LivingEntityIdV2([n; 16]);
    let mut civilizations = state.civilizations.into_vec();
    for n in [0xe0, 0xe1] {
        civilizations.push(LivingCivilizationV2 {
            id: id(n),
            name: LivingNameV2::new("Diplomacy actor").unwrap(),
            policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
            status: LivingCivilizationStatusV2::Dormant,
        });
    }
    civilizations.sort_by_key(|r| r.id);
    state.civilizations = LivingSortedVecV2::new(civilizations).unwrap();
    let mut worlds = state.worlds.into_vec();
    for (n, owner) in [(0xe2, Some(id(0xe0))), (0xe3, None)] {
        let mut row = origin.clone();
        row.id = id(n);
        row.owner = owner;
        worlds.push(row);
    }
    worlds.sort_by_key(|r| r.id);
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut systems = state.systems.into_vec();
    systems.push(LivingSystemV2 {
        id: id(0xe4),
        name: LivingNameV2::new("Remote").unwrap(),
    });
    systems.sort_by_key(|r| r.id);
    state.systems = LivingSortedVecV2::new(systems).unwrap();
    let mut lanes = state.lanes.into_vec();
    lanes.push(LivingLaneV2 {
        id: id(0xe5),
        system_a: origin.system.min(id(0xe4)),
        system_b: origin.system.max(id(0xe4)),
        distance_units: 20,
    });
    lanes.sort_by_key(|r| r.id);
    state.lanes = LivingSortedVecV2::new(lanes).unwrap();
    let a = colony.owner.min(id(0xe0));
    let b = colony.owner.max(id(0xe0));
    state.agreements = LivingSortedVecV2::new(
        [
            LivingAgreementKindV2::Nonaggression,
            LivingAgreementKindV2::Trade,
            LivingAgreementKindV2::Truce,
        ]
        .into_iter()
        .map(|kind| LivingAgreementV2 {
            kind,
            participant_a: a,
            participant_b: b,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(100),
        })
        .collect(),
    )
    .unwrap();
    let mut occupations = vec![
        LivingOccupationV2 {
            world: colony.world,
            claimant: id(0xe0),
            progress_ticks: 7,
        },
        LivingOccupationV2 {
            world: id(0xe2),
            claimant: colony.owner,
            progress_ticks: 8,
        },
        LivingOccupationV2 {
            world: id(0xe3),
            claimant: colony.owner,
            progress_ticks: 9,
        },
    ];
    occupations.sort_by_key(|r| r.world);
    state.occupations = LivingSortedVecV2::new(occupations).unwrap();
    let hull = sandbox.catalog().hull("scout").unwrap();
    let mut fleets = state.fleets.into_vec();
    fleets.push(LivingFleetV2 {
        id: id(0xe6),
        owner: colony.owner,
        location: LivingFleetLocationV2::Docked {
            world: colony.world,
        },
        order: Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(id(0xe2)),
            not_before_tick: LivingTickV2(500),
            lane_path: vec![id(0xe5); 3],
        }),
        hulls: LivingSortedVecV2::new(vec![LivingHullV2 {
            id: id(0xe7),
            definition: hull.id.clone(),
            hit_points: hull.hit_points,
            return_credit: false,
            next_repair: None,
        }])
        .unwrap(),
    });
    fleets.sort_by_key(|r| r.id);
    state.fleets = LivingSortedVecV2::new(fleets).unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        sandbox.catalog(),
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        sandbox.catalog().clone(),
        [0x63; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("diplomacy_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
#[test]
fn diplomacy_public_validated_genesis_admitted_commands_and_running_partitions() {
    let base = diplomacy_public_base();
    let id = |n| LivingEntityIdV2([n; 16]);
    let colony = base.state().colonies.as_slice()[0].clone();
    let owner = colony.owner;
    let war = LivingCreatorOperationV2::ForceWar {
        participant_a: t(id(0xe0)),
        participant_b: t(owner),
        declarer: t(owner),
        start_tick: LivingTickV2(0),
        end_tick: LivingTickV2(100),
    };
    let peace = LivingCreatorOperationV2::ForcePeace {
        participant_a: t(id(0xe0)),
        participant_b: t(owner),
    };
    for case in 0..11 {
        let operations = match case {
            0 => vec![
                LivingCreatorOperationV2::SetRelationBase {
                    from: t(owner),
                    to: t(id(0xe0)),
                    base_disposition: 17,
                },
                LivingCreatorOperationV2::SetRelationBase {
                    from: t(id(0xe0)),
                    to: t(owner),
                    base_disposition: -7,
                },
                LivingCreatorOperationV2::SetRelationBase {
                    from: t(owner),
                    to: t(id(0xe0)),
                    base_disposition: 18,
                },
            ],
            1 => vec![LivingCreatorOperationV2::SetAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: t(id(0xe0)),
                participant_b: t(owner),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(1),
            }],
            2 => vec![LivingCreatorOperationV2::RemoveAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: t(id(0xe0)),
                participant_b: t(owner),
            }],
            3 => vec![war.clone(), war.clone()],
            4 => vec![peace.clone()],
            5 => vec![
                war.clone(),
                LivingCreatorOperationV2::SetWorldOwner {
                    world: t(id(0xe2)),
                    owner: Some(t(id(0xe1))),
                },
                peace.clone(),
            ],
            6 => vec![LivingCreatorOperationV2::SetFleetOrder {
                fleet: t(id(0xe6)),
                kind: Some(LivingFleetOrderKindV2::Return),
                target_world: Some(t(colony.world)),
                not_before_tick: LivingTickV2(700),
                lane_path: vec![t(id(0xe5)); 63],
            }],
            7 => vec![LivingCreatorOperationV2::SetFleetOrder {
                fleet: t(id(0xe6)),
                kind: None,
                target_world: Some(t(id(0xfa))),
                not_before_tick: LivingTickV2(u64::MAX),
                lane_path: vec![t(id(0xfb)); 63],
            }],
            8 => vec![
                LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: LivingNameV2::new("Supplied local path").unwrap(),
                },
                LivingCreatorOperationV2::CreateLane {
                    local: 1,
                    system_a: t(base.state().systems.as_slice()[0].id),
                    system_b: LivingCommandTargetV2::Local { local: 0 },
                    distance_units: 10,
                },
                LivingCreatorOperationV2::SetFleetOrder {
                    fleet: t(id(0xe6)),
                    kind: Some(LivingFleetOrderKindV2::Explore),
                    target_world: None,
                    not_before_tick: LivingTickV2(700),
                    lane_path: vec![
                        LivingCommandTargetV2::Local { local: 1 },
                        t(id(0xe5)),
                        LivingCommandTargetV2::Local { local: 1 },
                    ],
                },
            ],
            9 => vec![
                LivingCreatorOperationV2::CreateColony {
                    world: t(id(0xe3)),
                    owner: t(id(0xe1)),
                },
                war.clone(),
            ],
            _ => {
                let world = base.state().worlds.as_slice()[0].clone();
                vec![
                    war.clone(),
                    LivingCreatorOperationV2::EditWorld {
                        target: t(world.id),
                        system: t(world.system),
                        archetype: world.archetype,
                        name: LivingNameV2::new("After diplomacy prefix").unwrap(),
                    },
                ]
            }
        };
        let mut admitted = base.clone();
        admitted
            .submit(request(&admitted, operations, LivingCommandModeV2::Running))
            .unwrap();
        let mut reference = admitted.clone();
        reference.step().unwrap();
        for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
            let mut actual = admitted.clone();
            let mut job = actual.begin_step().unwrap();
            let mut done = false;
            for poll in 0..100_000 {
                let budget = schedule[poll % schedule.len()];
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
                if progress != LivingStepProgressV2::Pending {
                    done = true;
                    break;
                }
                assert!(job.last_poll_visits() > 0);
            }
            assert!(done, "finite public diplomacy guard");
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&actual).unwrap()
            else {
                panic!("public diplomacy preparation")
            };
            drop(actual.commit_step(prepared).unwrap());
            assert_eq!(
                actual.state().canonical_bytes().unwrap(),
                reference.state().canonical_bytes().unwrap()
            );
            assert_eq!(actual.last_receipt(), reference.last_receipt());
            assert_eq!(actual.last_decision(), reference.last_decision());
            assert_eq!(actual.last_event_facts(), reference.last_event_facts());
            assert_eq!(
                actual.last_creator_recoveries(),
                reference.last_creator_recoveries()
            );
            assert_eq!(actual.revisions(), reference.revisions());
            assert_eq!(actual.published_cursor(), reference.published_cursor());
            assert_eq!(
                actual.accepted_sequence_high_water(),
                reference.accepted_sequence_high_water()
            );
            assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
            assert_eq!(
                actual
                    .last_receipt()
                    .unwrap()
                    .events
                    .iter()
                    .filter(|e| e.kind == LivingEventKindV2::CreatorIntervention)
                    .count(),
                1
            );
            if case == 6 {
                let order = actual
                    .state()
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| f.id == id(0xe6))
                    .unwrap()
                    .order
                    .as_ref()
                    .unwrap();
                assert_eq!(order.lane_path, vec![id(0xe5); 63]);
                assert_eq!(order.not_before_tick, LivingTickV2(700));
            }
            if case == 9 {
                assert!(
                    actual
                        .last_creator_recoveries()
                        .iter()
                        .any(|r| r.actor() == id(0xe1))
                );
            }
        }
    }
}
#[test]
fn diplomacy_public_clear_admission_still_rejects_unknown_locals_and_exact_missing_agreement() {
    let mut authority = diplomacy_public_base();
    let id = |n| LivingEntityIdV2([n; 16]);
    let before = authority.state().canonical_bytes().unwrap();
    let cursor = authority.published_cursor();
    for op in [
        LivingCreatorOperationV2::SetFleetOrder {
            fleet: t(id(0xe6)),
            kind: None,
            target_world: Some(LivingCommandTargetV2::Local { local: 77 }),
            not_before_tick: LivingTickV2(0),
            lane_path: vec![],
        },
        LivingCreatorOperationV2::SetAgreement {
            kind: LivingAgreementKindV2::Trade,
            participant_a: t(id(0xe0)),
            participant_b: t(id(0xe0)),
            start_tick: LivingTickV2(10),
            end_tick: LivingTickV2(9),
        },
    ] {
        assert!(
            authority
                .submit(request(&authority, vec![op], LivingCommandModeV2::Running))
                .is_err()
        );
        assert_eq!(authority.state().canonical_bytes().unwrap(), before);
        assert_eq!(authority.published_cursor(), cursor);
    }
    assert_eq!(
        authority.submit(request(
            &authority,
            vec![LivingCreatorOperationV2::RemoveAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: t(id(0xe0)),
                participant_b: t(id(0xe0))
            }],
            LivingCommandModeV2::Running
        )),
        Err(LivingCommandRejectionV2::MissingTarget {
            operation: 0,
            reference: "agreement"
        })
    );
}

fn cancellation_public_base(alloy: u32, paid: u32) -> LivingGalaxyAuthorityV2 {
    let original = builtin_living_sandbox_v2().unwrap();
    let mut state = original.state().clone();
    let id = |n| LivingEntityIdV2([n; 16]);
    let colony = state.colonies.as_slice()[0].clone();
    let mut worlds = state.worlds.into_vec();
    worlds
        .iter_mut()
        .find(|world| world.id == colony.world)
        .unwrap()
        .inventory
        .alloy = alloy;
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    state.facilities = Default::default();
    state.construction_jobs = LivingSortedVecV2::new(vec![LivingConstructionJobV2 {
        id: id(0xd0),
        world: colony.world,
        owner: colony.owner,
        definition: LivingSlugV2::new("defense_battery").unwrap(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(100),
        paid_alloy: paid,
    }])
    .unwrap();
    let hull = original.catalog().hull("scout").unwrap();
    state.fleets = LivingSortedVecV2::new(vec![LivingFleetV2 {
        id: id(0xd1),
        owner: colony.owner,
        location: LivingFleetLocationV2::Docked {
            world: colony.world,
        },
        order: None,
        hulls: LivingSortedVecV2::new(vec![LivingHullV2 {
            id: id(0xd2),
            definition: hull.id.clone(),
            hit_points: hull.hit_points,
            return_credit: true,
            next_repair: None,
        }])
        .unwrap(),
    }])
    .unwrap();
    state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
        id: id(0xd3),
        world: colony.world,
        owner: colony.owner,
        definition: hull.id.clone(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(100),
        paid_alloy: paid,
        target_fleet: Some(id(0xd1)),
    }])
    .unwrap();
    let first = state.systems.as_slice()[0].id;
    let mut systems = state.systems.into_vec();
    systems.push(LivingSystemV2 {
        id: id(0xd4),
        name: LivingNameV2::new("Remote").unwrap(),
    });
    systems.sort_by_key(|row| row.id);
    state.systems = LivingSortedVecV2::new(systems).unwrap();
    state.lanes = LivingSortedVecV2::new(vec![LivingLaneV2 {
        id: id(0xd5),
        system_a: first.min(id(0xd4)),
        system_b: first.max(id(0xd4)),
        distance_units: 100,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: id(0xd6),
        definition: original.catalog().hazards()[0].id.clone(),
        lane: id(0xd5),
        start_tick: LivingTickV2(20),
        end_tick: LivingTickV2(30),
    }])
    .unwrap();
    cancellation_authored_state(&original, state)
}
fn cancellation_authored_state(
    original: &LivingGalaxyAuthorityV2,
    state: LivingGalaxyStateV2,
) -> LivingGalaxyAuthorityV2 {
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        original.catalog(),
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        original.catalog().clone(),
        [0x64; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("cancellation_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn cancellation_public_step(
    base: &LivingGalaxyAuthorityV2,
    allow_bridge: bool,
) -> LivingGalaxyAuthorityV2 {
    let mut reference = base.clone();
    reference.step().unwrap();
    for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
        let mut actual = base.clone();
        let mut job = actual.begin_step().unwrap();
        let mut done = false;
        for attempt in 0..100_000 {
            let b = schedule[attempt % schedule.len()];
            let progress = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            if !allow_bridge {
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
            }
            if progress != LivingStepProgressV2::Pending {
                done = true;
                break;
            }
            assert!(job.last_poll_visits() > 0);
        }
        assert!(done, "finite public cancellation guard");
        let LivingStepPreparationV2::Ready(prepared) = job.finish_synchronously(&actual).unwrap()
        else {
            panic!("cancellation success")
        };
        drop(actual.commit_step(prepared).unwrap());
        assert_eq!(
            actual.state().canonical_bytes().unwrap(),
            reference.state().canonical_bytes().unwrap()
        );
        assert_eq!(actual.last_receipt(), reference.last_receipt());
        assert_eq!(actual.last_decision(), reference.last_decision());
        assert_eq!(actual.last_event_facts(), reference.last_event_facts());
        assert_eq!(
            actual.last_creator_recoveries(),
            reference.last_creator_recoveries()
        );
        assert_eq!(actual.revisions(), reference.revisions());
        assert_eq!(actual.published_cursor(), reference.published_cursor());
        assert_eq!(
            actual.accepted_sequence_high_water(),
            reference.accepted_sequence_high_water()
        );
        assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
        assert_eq!(actual.fault(), reference.fault());
    }
    reference
}
#[test]
fn cancellation_public_validated_refund_edges_both_jobs_and_reservation_release() {
    let id = |n| LivingEntityIdV2([n; 16]);
    for (alloy, paid) in [
        (0, 0),
        (1, 1),
        (50, 5),
        (9997, 6),
        (9998, 6),
        (10000, u32::MAX),
    ] {
        for hull in [false, true] {
            let mut a = cancellation_public_base(alloy, paid);
            let world = a.state().colonies.as_slice()[0].world;
            let mut operations = vec![if hull {
                LivingCreatorOperationV2::CancelHullJob {
                    target: t(id(0xd3)),
                }
            } else {
                LivingCreatorOperationV2::CancelConstructionJob {
                    target: t(id(0xd0)),
                }
            }];
            if hull {
                operations.push(LivingCreatorOperationV2::SetFleetOrder {
                    fleet: t(id(0xd1)),
                    kind: Some(LivingFleetOrderKindV2::Defend),
                    target_world: None,
                    not_before_tick: LivingTickV2(80),
                    lane_path: vec![],
                });
            }
            a.submit(request(&a, operations, LivingCommandModeV2::Running))
                .unwrap();
            let a = cancellation_public_step(&a, false);
            assert_eq!(
                a.state()
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|r| r.id == world)
                    .unwrap()
                    .inventory
                    .alloy,
                (alloy + paid / 2).min(10000)
            );
            let creator: Vec<_> = a
                .last_receipt()
                .unwrap()
                .events
                .iter()
                .filter(|e| matches!(e.provenance, LivingEventProvenanceV2::Creator { .. }))
                .map(|e| e.kind)
                .collect();
            assert_eq!(
                creator,
                if alloy + paid / 2 > 10000 {
                    vec![
                        LivingEventKindV2::CreatorIntervention,
                        LivingEventKindV2::RefundDiscarded,
                        LivingEventKindV2::ConstructionCancelled,
                    ]
                } else {
                    vec![
                        LivingEventKindV2::CreatorIntervention,
                        LivingEventKindV2::ConstructionCancelled,
                    ]
                }
            );
            assert_eq!(a.state().fleets.as_slice().len(), 1);
            assert_eq!(a.state().fleets.as_slice()[0].hulls.as_slice().len(), 1);
        }
    }
}
#[test]
fn cancellation_public_route_hazard_prefix_bridge_and_multiple_revisions() {
    let id = |n| LivingEntityIdV2([n; 16]);
    for bridge in [false, true] {
        let mut a = cancellation_public_base(10000, 5);
        let route = a.state().routes.as_slice()[0].clone();
        let mut operations = vec![LivingCreatorOperationV2::CancelConstructionJob {
            target: t(id(0xd0)),
        }];
        if bridge {
            operations.push(LivingCreatorOperationV2::RemoveEntity {
                target: t(id(0xd6)),
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            });
        } else {
            operations.push(LivingCreatorOperationV2::CancelHazard {
                target: t(id(0xd6)),
            });
        }
        operations.push(LivingCreatorOperationV2::EditRoute {
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
        });
        a.submit(request(&a, operations, LivingCommandModeV2::Running))
            .unwrap();
        a.submit(request(
            &a,
            vec![
                LivingCreatorOperationV2::CancelHullJob {
                    target: t(id(0xd3)),
                },
                LivingCreatorOperationV2::SetRouteSuspended {
                    target: t(route.id),
                    suspended: true,
                },
                LivingCreatorOperationV2::SetRouteSuspended {
                    target: t(route.id),
                    suspended: false,
                },
            ],
            LivingCommandModeV2::Running,
        ))
        .unwrap();
        let a = cancellation_public_step(&a, bridge);
        let creator: Vec<_> = a
            .last_receipt()
            .unwrap()
            .events
            .iter()
            .filter(|e| matches!(e.provenance, LivingEventProvenanceV2::Creator { .. }))
            .collect();
        assert_eq!(
            creator.iter().map(|e| e.kind).collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled,
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
        assert_eq!(creator[0].provenance, creator[2].provenance);
        assert_eq!(creator[3].provenance, creator[5].provenance);
        assert_ne!(creator[0].provenance, creator[3].provenance);
        assert!(a.state().hazards.as_slice().is_empty());
        reload(&a);
    }
}
#[test]
fn cancellation_public_admission_rejection_is_atomic_and_late_model_guards_stay_late() {
    let mut a = cancellation_public_base(10000, 5);
    let before = a.state().clone();
    let cursor = a.published_cursor();
    let world = a.state().colonies.as_slice()[0].world;
    let overflow = vec![
        LivingCreatorOperationV2::SetWorldInventory {
            world: t(world),
            inventory: LivingInventoryV2 {
                energy: 0,
                ore: 0,
                alloy: u32::MAX,
            },
        },
        LivingCreatorOperationV2::CancelConstructionJob {
            target: t(LivingEntityIdV2([0xd0; 16])),
        },
    ];
    assert_eq!(
        a.submit(request(&a, overflow, LivingCommandModeV2::Running)),
        Err(LivingCommandRejectionV2::Exhausted)
    );
    assert_eq!(a.state(), &before);
    assert_eq!(a.published_cursor(), cursor);
    assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
    let route = a.state().routes.as_slice()[0].clone();
    for (reserve, same, field) in [
        (10001, false, "route source_reserve"),
        (20, true, "route endpoints"),
    ] {
        let op = LivingCreatorOperationV2::EditRoute {
            target: t(route.id),
            kind: LivingRouteKindV2::Internal,
            source_world: t(route.source_world),
            destination_world: t(if same {
                route.source_world
            } else {
                route.destination_world
            }),
            resource: route.resource,
            batch_size: 3,
            cadence_ticks: 10,
            source_reserve: reserve,
            source_owner: t(route.source_owner),
            receiver_owner: t(route.source_owner),
        };
        assert_eq!(
            a.submit(request(&a, vec![op], LivingCommandModeV2::Running)),
            Err(LivingCommandRejectionV2::Projection(
                LivingValidationErrorV2::Range { field }
            ))
        );
        assert_eq!(a.state(), &before);
        assert_eq!(a.published_cursor(), cursor);
    }
}

#[test]
fn cancellation_public_route_permissions_half_open_intervals_and_due_rules() {
    for control in 0..14 {
        for mode in 0..3 {
            let base = cancellation_public_base(100, 5);
            let mut state = base.state().clone();
            let mut route = state.routes.as_slice()[0].clone();
            let owner = route.source_owner;
            let other = state
                .worlds
                .as_slice()
                .iter()
                .find(|w| w.owner.is_some_and(|id| id != owner))
                .unwrap()
                .clone();
            let receiver = other.owner.unwrap();
            let pair = (owner.min(receiver), owner.max(receiver));
            route.kind = match control {
                0 | 1 => LivingRouteKindV2::Internal,
                3..=8 => LivingRouteKindV2::Trade,
                _ => LivingRouteKindV2::Aid,
            };
            if control != 0 {
                route.destination_world = other.id;
                route.receiver_owner = receiver;
            }
            route.next_due = LivingTickV2([0, 1, 50][control % 3]);
            route.suspended = mode == 0;
            state.wars = Default::default();
            state.agreements = Default::default();
            if (4..=8).contains(&control) {
                state.agreements = LivingSortedVecV2::new(vec![LivingAgreementV2 {
                    kind: match control {
                        4 => LivingAgreementKindV2::Nonaggression,
                        5 => LivingAgreementKindV2::Truce,
                        _ => LivingAgreementKindV2::Trade,
                    },
                    participant_a: pair.0,
                    participant_b: pair.1,
                    start_tick: LivingTickV2(if control == 8 { 2 } else { 0 }),
                    end_tick: LivingTickV2(if control == 7 { 1 } else { 100 }),
                }])
                .unwrap();
            }
            if (9..=11).contains(&control) {
                state.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
                    participant_a: pair.0,
                    participant_b: pair.1,
                    declarer: owner,
                    start_tick: LivingTickV2(if control == 11 { 2 } else { 0 }),
                    end_tick: LivingTickV2(if control == 10 { 1 } else { 100 }),
                }])
                .unwrap();
            }
            if control == 12 {
                route.source_owner = receiver;
            }
            if control == 13 {
                route.receiver_owner = owner;
            }
            state.routes = LivingSortedVecV2::new(vec![route.clone()]).unwrap();
            let mut a = cancellation_authored_state(&base, state);
            let before = a.state().clone();
            let cursor = a.published_cursor();
            let op = if mode == 0 {
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
                }
            } else {
                LivingCreatorOperationV2::SetRouteSuspended {
                    target: t(route.id),
                    suspended: mode == 2,
                }
            };
            let result = a.submit(request(&a, vec![op], LivingCommandModeV2::Running));
            let permitted = mode == 2 || matches!(control, 0 | 2 | 6 | 10 | 11);
            if permitted {
                result.unwrap();
                let a = cancellation_public_step(&a, false);
                let actual = a
                    .state()
                    .routes
                    .as_slice()
                    .iter()
                    .find(|r| r.id == route.id)
                    .unwrap();
                assert_eq!(actual.suspended, mode == 2);
                assert_eq!(
                    actual.next_due,
                    if route.next_due.0 <= 1 {
                        if mode == 2 {
                            // Creator preserves suspended due. Later Dispatch still
                            // advances a due route once from its original deadline.
                            LivingTickV2(route.next_due.0 + 10)
                        } else {
                            LivingTickV2(11)
                        }
                    } else {
                        route.next_due
                    }
                );
            } else {
                assert_eq!(
                    result,
                    Err(LivingCommandRejectionV2::Projection(
                        LivingValidationErrorV2::Range {
                            field: "route permission"
                        }
                    ))
                );
                assert_eq!(a.state(), &before);
                assert_eq!(a.published_cursor(), cursor);
                assert_eq!(a.accepted_sequence_high_water(), 0);
                assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
            }
        }
    }
}
#[test]
fn cancellation_public_route_local_guards_preceding_owner_and_diplomacy() {
    let base = cancellation_public_base(100, 5);
    let route = base.state().routes.as_slice()[0].clone();
    let other = base
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.owner.is_some_and(|id| id != route.source_owner))
        .unwrap()
        .clone();
    for control in 0..7 {
        let mut a = base.clone();
        let edit = LivingCreatorOperationV2::EditRoute {
            target: t(if control == 0 {
                LivingEntityIdV2([0xff; 16])
            } else {
                route.id
            }),
            kind: LivingRouteKindV2::Trade,
            source_world: t(route.source_world),
            destination_world: t(other.id),
            resource: route.resource,
            batch_size: if control < 2 {
                0
            } else if control == 2 {
                11
            } else {
                3
            },
            cadence_ticks: if control == 3 { 9 } else { 10 },
            source_reserve: if control == 4 { 19 } else { 20 },
            source_owner: t(route.source_owner),
            receiver_owner: t(if control == 5 {
                route.source_owner
            } else {
                other.owner.unwrap()
            }),
        };
        let operations = if control == 5 {
            vec![
                LivingCreatorOperationV2::SetWorldOwner {
                    world: t(other.id),
                    owner: Some(t(route.source_owner)),
                },
                LivingCreatorOperationV2::EditRoute {
                    target: t(route.id),
                    kind: LivingRouteKindV2::Internal,
                    source_world: t(route.source_world),
                    destination_world: t(other.id),
                    resource: route.resource,
                    batch_size: 3,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: t(route.source_owner),
                    receiver_owner: t(route.source_owner),
                },
            ]
        } else if control == 6 {
            vec![
                LivingCreatorOperationV2::SetAgreement {
                    kind: LivingAgreementKindV2::Trade,
                    participant_a: t(route.source_owner),
                    participant_b: t(other.owner.unwrap()),
                    start_tick: LivingTickV2(1),
                    end_tick: LivingTickV2(100),
                },
                edit,
            ]
        } else {
            vec![edit]
        };
        let result = a.submit(request(&a, operations, LivingCommandModeV2::Running));
        if control >= 5 {
            result.unwrap();
            cancellation_public_step(&a, false);
        } else {
            assert_eq!(
                result,
                Err(if control == 0 {
                    LivingCommandRejectionV2::MissingTarget {
                        operation: 0,
                        reference: "route",
                    }
                } else {
                    LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                        field: "route bounds",
                    })
                })
            );
        }
    }
}

#[test]
fn cancellation_public_local_job_targets_missing_targets_and_generic_removal() {
    let id = |n| LivingEntityIdV2([n; 16]);
    for hull in [false, true] {
        let mut a = cancellation_public_base(100, 0);
        let colony = a.state().colonies.as_slice()[0].clone();
        let local = LivingCommandTargetV2::Local { local: 0 };
        let operations = if hull {
            vec![
                LivingCreatorOperationV2::CancelHullJob {
                    target: t(id(0xd3)),
                },
                LivingCreatorOperationV2::CreateHullJob {
                    local: 0,
                    world: t(colony.world),
                    owner: t(colony.owner),
                    definition: LivingSlugV2::new("scout").unwrap(),
                    accepted_tick: LivingTickV2(1),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 5,
                    target_fleet: Some(t(id(0xd1))),
                },
                LivingCreatorOperationV2::CancelHullJob { target: local },
            ]
        } else {
            vec![
                LivingCreatorOperationV2::CancelConstructionJob {
                    target: t(id(0xd0)),
                },
                LivingCreatorOperationV2::CreateConstructionJob {
                    local: 0,
                    world: t(colony.world),
                    owner: t(colony.owner),
                    definition: LivingSlugV2::new("defense_battery").unwrap(),
                    accepted_tick: LivingTickV2(1),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 5,
                },
                LivingCreatorOperationV2::CancelConstructionJob { target: local },
            ]
        };
        a.submit(request(&a, operations, LivingCommandModeV2::Running))
            .unwrap();
        let a = cancellation_public_step(&a, false);
        assert_eq!(
            a.last_receipt()
                .unwrap()
                .events
                .iter()
                .filter(|e| matches!(e.provenance, LivingEventProvenanceV2::Creator { .. }))
                .map(|e| e.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::ConstructionCancelled,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
    }
    for control in 0..3 {
        let mut a = cancellation_public_base(100, 5);
        let before = a.state().clone();
        let cursor = a.published_cursor();
        let target = t(id(0xff));
        let (op, reference) = match control {
            0 => (
                LivingCreatorOperationV2::CancelConstructionJob { target },
                "construction job",
            ),
            1 => (
                LivingCreatorOperationV2::CancelHullJob { target },
                "hull job",
            ),
            _ => (LivingCreatorOperationV2::CancelHazard { target }, "hazard"),
        };
        assert_eq!(
            a.submit(request(&a, vec![op], LivingCommandModeV2::Running)),
            Err(LivingCommandRejectionV2::MissingTarget {
                operation: 0,
                reference
            })
        );
        assert_eq!(a.state(), &before);
        assert_eq!(a.published_cursor(), cursor);
        assert_eq!(a.accepted_sequence_high_water(), 0);
    }
    let mut a = cancellation_public_base(100, 5);
    let world = a.state().colonies.as_slice()[0].world;
    a.submit(request(
        &a,
        vec![LivingCreatorOperationV2::RemoveEntity {
            target: t(id(0xd0)),
            cascade: LivingCascadeDispositionV2::RejectIfReferenced,
        }],
        LivingCommandModeV2::Running,
    ))
    .unwrap();
    let a = cancellation_public_step(&a, true);
    assert_eq!(
        a.state()
            .worlds
            .as_slice()
            .iter()
            .find(|w| w.id == world)
            .unwrap()
            .inventory
            .alloy,
        100
    );
    assert_eq!(
        a.last_receipt()
            .unwrap()
            .events
            .iter()
            .filter(|e| matches!(e.provenance, LivingEventProvenanceV2::Creator { .. }))
            .map(|e| e.kind)
            .collect::<Vec<_>>(),
        vec![LivingEventKindV2::CreatorIntervention]
    );
}

#[test]
fn cancellation_public_hazard_active_future_and_expiration_boundary_have_only_marker() {
    for (start, end) in [(0, 30), (20, 30), (0, 1)] {
        let base = cancellation_public_base(100, 5);
        let mut state = base.state().clone();
        let target = state.hazards.as_slice()[0].id;
        let mut hazards = state.hazards.into_vec();
        hazards[0].start_tick = LivingTickV2(start);
        hazards[0].end_tick = LivingTickV2(end);
        state.hazards = LivingSortedVecV2::new(hazards).unwrap();
        let mut a = cancellation_authored_state(&base, state);
        a.submit(request(
            &a,
            vec![LivingCreatorOperationV2::CancelHazard { target: t(target) }],
            LivingCommandModeV2::Running,
        ))
        .unwrap();
        let a = cancellation_public_step(&a, false);
        assert!(a.state().hazards.as_slice().is_empty());
        assert_eq!(
            a.last_receipt()
                .unwrap()
                .events
                .iter()
                .filter(|e| matches!(e.provenance, LivingEventProvenanceV2::Creator { .. }))
                .map(|e| e.kind)
                .collect::<Vec<_>>(),
            vec![LivingEventKindV2::CreatorIntervention]
        );
    }
}

fn geometry_public_fixture(
    control: usize,
) -> (
    LivingGalaxyAuthorityV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
) {
    let sandbox = builtin_living_sandbox_v2().unwrap();
    let mut state = sandbox.state().clone();
    let system = state.systems.as_slice()[0].id;
    let new_system = LivingEntityIdV2([0xe9; 16]);
    let lane = LivingEntityIdV2([0xea; 16]);
    let other_lane = LivingEntityIdV2([0xeb; 16]);
    let world = state.worlds.as_slice()[0].id;
    let other = state.worlds.as_slice()[1].id;
    let actor = state.civilizations.as_slice()[0].id;
    let mut systems = state.systems.into_vec();
    systems.push(LivingSystemV2 {
        id: new_system,
        name: LivingNameV2::new("Geometry").unwrap(),
    });
    systems.sort_by_key(|r| r.id);
    state.systems = LivingSortedVecV2::new(systems).unwrap();
    state.lanes = LivingSortedVecV2::new(vec![
        LivingLaneV2 {
            id: lane,
            system_a: system.min(new_system),
            system_b: system.max(new_system),
            distance_units: 10,
        },
        LivingLaneV2 {
            id: other_lane,
            system_a: system.min(new_system),
            system_b: system.max(new_system),
            distance_units: 11,
        },
    ])
    .unwrap();
    let mut fleets = state.fleets.into_vec();
    fleets[1].order = Some(LivingFleetOrderV2 {
        kind: LivingFleetOrderKindV2::Move,
        target_world: Some(world),
        not_before_tick: LivingTickV2(99),
        lane_path: vec![other_lane; 3],
    });
    match control {
        3 | 4 => {
            fleets[0].location = LivingFleetLocationV2::Travelling {
                origin_world: if control == 3 { world } else { other },
                destination_world: if control == 4 { world } else { other },
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(99),
            };
        }
        5 => {
            fleets[0].order = Some(LivingFleetOrderV2 {
                kind: LivingFleetOrderKindV2::Move,
                target_world: Some(other),
                not_before_tick: LivingTickV2(99),
                lane_path: Vec::new(),
            })
        }
        6..=8 => fleets[1].order.as_mut().unwrap().lane_path[control - 6] = lane,
        9 => {
            fleets[0].location = LivingFleetLocationV2::Travelling {
                origin_world: other,
                destination_world: world,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(99),
            }
        }
        _ => {}
    }
    state.fleets = LivingSortedVecV2::new(fleets).unwrap();
    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
        id: LivingEntityIdV2([0xec; 16]),
        dispatch_owner: actor,
        intended_receiver: actor,
        source_world: if control == 1 { world } else { other },
        destination_world: if control == 2 { world } else { other },
        resource: LivingResourceV2::Ore,
        units: 1,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(99),
        disposition: LivingShipmentDispositionV2::Outbound,
        dispatch_kind: Some(LivingRouteKindV2::Internal),
        returning_to_source: false,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: LivingEntityIdV2([0xed; 16]),
        lane,
        definition: LivingSlugV2::new("ion_storm").unwrap(),
        start_tick: LivingTickV2(0),
        end_tick: LivingTickV2(100),
    }])
    .unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        sandbox.catalog(),
    )
    .unwrap();
    let base = LivingGalaxyAuthorityV2::from_genesis(
        sandbox.catalog().clone(),
        [0x59; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("geometry_guards").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    (base, world, lane, system, new_system)
}

#[test]
fn geometry_public_validated_guards_and_complete_partition_parity() {
    for control in 0..10 {
        for kind in 0..4 {
            let (base, world, lane, system, new_system) = geometry_public_fixture(control);
            let operation = if kind < 2 {
                LivingCreatorOperationV2::EditWorld {
                    target: t(world),
                    system: t(if kind == 0 { new_system } else { system }),
                    archetype: base.state().worlds.as_slice()[0].archetype.clone(),
                    name: LivingNameV2::new("Geometry changed").unwrap(),
                }
            } else {
                LivingCreatorOperationV2::EditLane {
                    target: t(lane),
                    system_a: t(new_system),
                    system_b: t(system),
                    distance_units: if kind == 2 { 20 } else { 10 },
                }
            };
            let blocked = (kind == 0 && matches!(control, 1..=5 | 9))
                || (kind == 2 && matches!(control, 6..=8));
            let mut admitted = base.clone();
            let original = admitted.bookmark().unwrap();
            let result = admitted.submit(request(
                &admitted,
                vec![operation.clone()],
                LivingCommandModeV2::Running,
            ));
            if blocked {
                assert_eq!(
                    result.unwrap_err(),
                    LivingCommandRejectionV2::ReferencedTarget
                );
                assert_eq!(admitted.bookmark().unwrap(), original);
                assert_eq!(admitted.capture_boundary().unwrap().pending_count(), 0);
                continue;
            }
            result.unwrap();
            let mut reference = admitted.clone();
            reference.step().unwrap();
            for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                let mut actual = admitted.clone();
                let mut job = actual.begin_step().unwrap();
                let mut ready = false;
                for attempt in 0..100000 {
                    let budget = schedule[attempt % schedule.len()];
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    assert_ne!(
                        progress,
                        LivingStepProgressV2::NeedsSynchronousCreatorOperation
                    );
                    if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                        ready = true;
                        break;
                    }
                    assert!(job.last_poll_visits() > 0);
                }
                assert!(ready);
                let LivingStepPreparationV2::Ready(token) =
                    job.finish_synchronously(&actual).unwrap()
                else {
                    panic!("geometry preparation")
                };
                drop(actual.commit_step(token).unwrap());
                assert_eq!(
                    actual.state().canonical_bytes().unwrap(),
                    reference.state().canonical_bytes().unwrap()
                );
                assert_eq!(actual.last_receipt(), reference.last_receipt());
                assert_eq!(actual.last_decision(), reference.last_decision());
                assert_eq!(actual.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    actual.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(actual.revisions(), reference.revisions());
                assert_eq!(actual.published_cursor(), reference.published_cursor());
                assert_eq!(
                    actual.accepted_sequence_high_water(),
                    reference.accepted_sequence_high_water()
                );
                assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
                reload(&actual);
            }
        }
    }
}

fn fleet_hull_public_fixture(
    control: usize,
) -> (
    LivingGalaxyAuthorityV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
    LivingEntityIdV2,
) {
    let source = builtin_first_expansion_v2().unwrap();
    let mut state = source.state().clone();
    let mut fleets = state.fleets.clone().into_vec();
    let selected = &mut fleets[0];
    let fid = selected.id;
    let hid = selected.hulls.as_slice()[0].id;
    let owner = selected.owner;
    let LivingFleetLocationV2::Docked { world } = selected.location else {
        panic!("Docked genesis")
    };
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.id != world)
        .unwrap()
        .id;
    let lane = state.lanes.as_slice()[0].id;
    selected.order = match control {
        0 => Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(destination),
            not_before_tick: LivingTickV2(100),
            lane_path: vec![lane; 33],
        }),
        1 => Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(destination),
            not_before_tick: LivingTickV2(100),
            lane_path: Vec::with_capacity(7),
        }),
        _ => None,
    };
    if control == 2 {
        selected.location = LivingFleetLocationV2::Travelling {
            origin_world: world,
            destination_world: destination,
            departure_tick: LivingTickV2(0),
            arrival_tick: LivingTickV2(20),
        };
    }
    let mut hulls = selected.hulls.clone().into_vec();
    hulls[0].hit_points = 1;
    hulls[0].next_repair = Some(LivingTickV2(40));
    selected.hulls = LivingSortedVecV2::new(hulls).unwrap();
    state.fleets = LivingSortedVecV2::new(fleets).unwrap();
    if control >= 3 {
        let mut jobs = state.hull_jobs.clone().into_vec();
        jobs.push(LivingHullJobV2 {
            id: LivingEntityIdV2([0xf8; 16]),
            world,
            owner,
            definition: LivingSlugV2::new("scout").unwrap(),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(50),
            paid_alloy: 10,
            target_fleet: Some(fid),
        });
        jobs.sort_by_key(|r| r.id);
        state.hull_jobs = LivingSortedVecV2::new(jobs).unwrap();
    }
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
    let a = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("fleet_hull_metadata").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    (a, fid, hid, owner, world)
}

#[test]
fn fleet_hull_public_validated_guards_health_and_complete_partition_parity() {
    for control in 0..4 {
        for kind in 0..4 {
            let (base, fleet, hull, owner, world) = fleet_hull_public_fixture(control);
            let operation = if kind == 0 {
                LivingCreatorOperationV2::EditFleet {
                    target: t(fleet),
                    owner: t(owner),
                    world: t(world),
                }
            } else {
                LivingCreatorOperationV2::EditHull {
                    target: t(hull),
                    definition: LivingSlugV2::new(["colony_ark", "escort", "scout"][kind - 1])
                        .unwrap(),
                    hit_points: 1,
                    return_credit: true,
                }
            };
            let mut admitted = base.clone();
            let original = admitted.bookmark().unwrap();
            let result = admitted.submit(request(
                &admitted,
                vec![operation],
                LivingCommandModeV2::Running,
            ));
            if control >= 2 {
                assert_eq!(
                    result.unwrap_err(),
                    LivingCommandRejectionV2::ReferencedTarget
                );
                assert_eq!(admitted.bookmark().unwrap(), original);
                assert_eq!(admitted.capture_boundary().unwrap().pending_count(), 0);
                continue;
            }
            result.unwrap();
            let mut reference = admitted.clone();
            reference.step().unwrap();
            for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                let mut actual = admitted.clone();
                let mut job = actual.begin_step().unwrap();
                let mut done = false;
                for attempt in 0..100000 {
                    let b = schedule[attempt % schedule.len()];
                    let p = job.poll(b).unwrap();
                    assert!(job.last_poll_visits() <= b);
                    assert_ne!(p, LivingStepProgressV2::NeedsSynchronousCreatorOperation);
                    if p == LivingStepProgressV2::NeedsSynchronousCompletion {
                        done = true;
                        break;
                    }
                }
                assert!(done);
                let LivingStepPreparationV2::Ready(token) =
                    job.finish_synchronously(&actual).unwrap()
                else {
                    panic!("metadata ready")
                };
                drop(actual.commit_step(token).unwrap());
                assert_eq!(
                    actual.state().canonical_bytes().unwrap(),
                    reference.state().canonical_bytes().unwrap()
                );
                assert_eq!(actual.last_receipt(), reference.last_receipt());
                assert_eq!(actual.last_decision(), reference.last_decision());
                assert_eq!(actual.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    actual.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(actual.revisions(), reference.revisions());
                assert_eq!(actual.published_cursor(), reference.published_cursor());
                assert_eq!(
                    actual.accepted_sequence_high_water(),
                    reference.accepted_sequence_high_water()
                );
                assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
                let row = actual
                    .state()
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| f.id == fleet)
                    .unwrap();
                assert!(row.order.is_none());
                assert_eq!(
                    row.hulls
                        .as_slice()
                        .iter()
                        .find(|h| h.id == hull)
                        .unwrap()
                        .next_repair,
                    Some(LivingTickV2(40))
                );
                reload(&actual);
            }
        }
    }
}

// Authentic authored structural controls: every caller validates before admission.
fn structural_public_authority(state: LivingGalaxyStateV2) -> LivingGalaxyAuthorityV2 {
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
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0x67; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("fleet_structural").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn structural_public_fixture(
    source_hulls: usize,
    target_hulls: usize,
    control: usize,
) -> LivingGalaxyAuthorityV2 {
    let mut state = builtin_first_expansion_v2().unwrap().state().clone();
    let owner = state.colonies.as_slice()[0].owner;
    let world = state.colonies.as_slice()[0].world;
    let other_world = state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id != world)
        .unwrap()
        .id;
    let lane = state.lanes.as_slice()[0].id;
    let mut fleets: Vec<_> = (0..3)
        .map(|i| LivingFleetV2 {
            id: LivingEntityIdV2([0xd0 + i; 16]),
            owner,
            location: LivingFleetLocationV2::Docked { world },
            order: (i != 2).then(|| LivingFleetOrderV2 {
                kind: LivingFleetOrderKindV2::Move,
                target_world: Some(other_world),
                not_before_tick: LivingTickV2(100),
                lane_path: if i == 0 {
                    vec![lane; 33]
                } else {
                    Vec::with_capacity(7)
                },
            }),
            hulls: LivingSortedVecV2::new(
                (0..if i == 0 {
                    source_hulls
                } else if i == 1 {
                    target_hulls
                } else {
                    1
                })
                    .map(|n| LivingHullV2 {
                        id: LivingEntityIdV2([20 + n as u8 * 3 + i; 16]),
                        definition: LivingSlugV2::new("scout").unwrap(),
                        hit_points: 7,
                        return_credit: i != 0,
                        next_repair: Some(LivingTickV2(40)),
                    })
                    .collect(),
            )
            .unwrap(),
        })
        .collect();
    if control == 1 || control == 2 {
        let index = control - 1;
        fleets[index].location = LivingFleetLocationV2::Travelling {
            origin_world: world,
            destination_world: other_world,
            departure_tick: LivingTickV2(0),
            arrival_tick: LivingTickV2(20),
        };
    }
    if control == 3 {
        fleets[1].location = LivingFleetLocationV2::Docked { world: other_world };
    }
    if control == 4 {
        let other = LivingEntityIdV2([0xcf; 16]);
        let mut civilizations = state.civilizations.into_vec();
        civilizations.push(LivingCivilizationV2 {
            id: other,
            name: LivingNameV2::new("Other").unwrap(),
            policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
            status: LivingCivilizationStatusV2::Dormant,
        });
        civilizations.sort_by_key(|row| row.id);
        state.civilizations = LivingSortedVecV2::new(civilizations).unwrap();
        fleets[1].owner = other;
    }
    if control == 5 || control == 6 {
        let index = control - 5;
        fleets[index].order = None;
        state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
            id: LivingEntityIdV2([0xef; 16]),
            world,
            owner,
            definition: LivingSlugV2::new("scout").unwrap(),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(50),
            paid_alloy: 10,
            target_fleet: Some(fleets[index].id),
        }])
        .unwrap();
    }
    if control == 7 || control == 8 {
        for i in 3..if control == 7 { 256 } else { 255 } {
            let mut id = [0xf0; 16];
            id[15] = i as u8;
            fleets.push(LivingFleetV2 {
                id: LivingEntityIdV2(id),
                owner,
                location: LivingFleetLocationV2::Docked { world },
                order: None,
                hulls: LivingSortedVecV2::default(),
            });
        }
        if control == 8 {
            state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
                id: LivingEntityIdV2([0xef; 16]),
                world,
                owner,
                definition: LivingSlugV2::new("scout").unwrap(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(50),
                paid_alloy: 10,
                target_fleet: None,
            }])
            .unwrap();
        }
    }
    fleets.sort_by_key(|row| row.id);
    state.fleets = LivingSortedVecV2::new(fleets).unwrap();
    structural_public_authority(state)
}
fn structural_assert_complete(
    actual: &LivingGalaxyAuthorityV2,
    reference: &LivingGalaxyAuthorityV2,
) {
    assert_eq!(actual.state(), reference.state());
    assert_eq!(actual.last_receipt(), reference.last_receipt());
    assert_eq!(actual.last_event_facts(), reference.last_event_facts());
    assert_eq!(actual.last_decision(), reference.last_decision());
    assert_eq!(
        actual.last_creator_recoveries(),
        reference.last_creator_recoveries()
    );
    assert_eq!(actual.revisions(), reference.revisions());
    assert_eq!(actual.published_cursor(), reference.published_cursor());
    assert_eq!(
        actual.accepted_sequence_high_water(),
        reference.accepted_sequence_high_water()
    );
    assert_eq!(actual.fault(), reference.fault());
    assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
    assert_eq!(
        encode_living_archive_v2(actual).unwrap(),
        encode_living_archive_v2(reference).unwrap()
    );
    reload(actual);
}
#[test]
fn structural_public_validated_split_merge_complete_partition_parity() {
    for (source_count, target_count) in [(4, 3), (8, 8), (0, 0), (0, 4), (4, 0)] {
        for source_index in 0..3 {
            for target_index in 0..3 {
                if source_index == target_index {
                    continue;
                }
                let base = structural_public_fixture(source_count, target_count, 0);
                let source = &base.state().fleets.as_slice()[source_index];
                let target = &base.state().fleets.as_slice()[target_index];
                let mut batches = vec![vec![LivingCreatorOperationV2::MergeFleets {
                    source: t(source.id),
                    target: t(target.id),
                }]];
                if source.hulls.as_slice().len() > 1 {
                    for index in [
                        0,
                        source.hulls.as_slice().len() / 2,
                        source.hulls.as_slice().len() - 1,
                    ] {
                        batches.push(vec![LivingCreatorOperationV2::SplitFleet {
                            local: 0,
                            source: t(source.id),
                            hulls: vec![t(source.hulls.as_slice()[index].id)],
                        }]);
                        batches.push(vec![
                            LivingCreatorOperationV2::SplitFleet {
                                local: 0,
                                source: t(source.id),
                                hulls: vec![t(source.hulls.as_slice()[index].id)],
                            },
                            LivingCreatorOperationV2::MergeFleets {
                                target: t(source.id),
                                source: LivingCommandTargetV2::Local { local: 0 },
                            },
                        ]);
                    }
                }
                for operations in batches {
                    let mut admitted = base.clone();
                    admitted
                        .submit(request(&admitted, operations, LivingCommandModeV2::Running))
                        .unwrap();
                    let mut reference = admitted.clone();
                    reference.step().unwrap();
                    for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                        let mut actual = admitted.clone();
                        let mut job = actual.begin_step().unwrap();
                        let mut ready = false;
                        for attempt in 0..100000 {
                            let budget = schedule[attempt % schedule.len()];
                            let progress = job.poll(budget).unwrap();
                            assert!(job.last_poll_visits() <= budget);
                            assert_ne!(
                                progress,
                                LivingStepProgressV2::NeedsSynchronousCreatorOperation
                            );
                            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                                ready = true;
                                break;
                            }
                        }
                        assert!(ready);
                        let LivingStepPreparationV2::Ready(token) =
                            job.take_synchronous_preparation(&actual).unwrap()
                        else {
                            panic!("structural success");
                        };
                        assert!(job.take_synchronous_preparation(&actual).is_err());
                        drop(actual.commit_step(token).unwrap());
                        structural_assert_complete(&actual, &reference);
                        for row in actual.state().fleets.as_slice() {
                            for hull in row.hulls.as_slice() {
                                assert_eq!(hull.next_repair, Some(LivingTickV2(40)));
                            }
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn structural_public_validated_membership_capacity_and_guard_precedence() {
    for control in 0..9 {
        let base = structural_public_fixture(9, 8, control);
        let source = &base.state().fleets.as_slice()[0];
        let target = &base.state().fleets.as_slice()[1];
        let missing = LivingEntityIdV2([0xfe; 16]);
        let first = source.hulls.as_slice()[0].id;
        let last = source.hulls.as_slice()[8].id;
        let selections = [
            Vec::new(),
            source
                .hulls
                .as_slice()
                .iter()
                .map(|row| t(row.id))
                .collect(),
            vec![t(first), t(first)],
            vec![t(last), t(first)],
            vec![t(missing)],
            vec![t(first)],
        ];
        for (case, hulls) in selections.into_iter().enumerate() {
            let mut actual = base.clone();
            let before = actual.bookmark().unwrap();
            let result = actual.submit(request(
                &actual,
                vec![LivingCreatorOperationV2::SplitFleet {
                    local: 0,
                    source: t(source.id),
                    hulls,
                }],
                LivingCommandModeV2::Running,
            ));
            let expected = if control == 1 || control == 5 {
                Some(LivingCommandRejectionV2::ReferencedTarget)
            } else if case < 4 {
                Some(LivingCommandRejectionV2::Projection(
                    LivingValidationErrorV2::Range {
                        field: "split hull membership",
                    },
                ))
            } else if control >= 7 {
                Some(LivingCommandRejectionV2::Projection(
                    LivingValidationErrorV2::Capacity {
                        collection: "fleets",
                        limit: 256,
                        found: 257,
                    },
                ))
            } else if case == 4 {
                Some(LivingCommandRejectionV2::MissingTarget {
                    operation: 0,
                    reference: "split hull",
                })
            } else {
                None
            };
            if let Some(expected) = expected {
                assert_eq!(result.unwrap_err(), expected);
                assert_eq!(actual.bookmark().unwrap(), before);
                assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
            } else {
                result.unwrap();
            }
        }
        for (target_id, source_id) in [
            (missing, missing),
            (missing, source.id),
            (target.id, missing),
            (target.id, source.id),
        ] {
            let mut actual = base.clone();
            let before = actual.bookmark().unwrap();
            let error = actual
                .submit(request(
                    &actual,
                    vec![LivingCreatorOperationV2::MergeFleets {
                        target: t(target_id),
                        source: t(source_id),
                    }],
                    LivingCommandModeV2::Running,
                ))
                .unwrap_err();
            let expected = if target_id == source_id {
                LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                    field: "merge fleet identity",
                })
            } else if target_id == missing {
                LivingCommandRejectionV2::MissingTarget {
                    operation: 0,
                    reference: "fleet",
                }
            } else if control == 2 || control == 6 {
                LivingCommandRejectionV2::ReferencedTarget
            } else if source_id == missing {
                LivingCommandRejectionV2::MissingTarget {
                    operation: 0,
                    reference: "fleet",
                }
            } else if control == 1 || control == 5 {
                LivingCommandRejectionV2::ReferencedTarget
            } else if control == 3 || control == 4 {
                LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                    field: "merge fleet colocation",
                })
            } else {
                LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Capacity {
                    collection: "fleet hulls",
                    limit: 16,
                    found: 17,
                })
            };
            assert_eq!(error, expected);
            assert_eq!(actual.bookmark().unwrap(), before);
        }
    }
}
#[test]
fn structural_public_preseeded_split_identity_and_cross_parent_duplicate_attempts() {
    for duplicate in [false, true] {
        let base = structural_public_fixture(4, 3, 0);
        let source = base.state().fleets.as_slice()[0].id;
        let target = base.state().fleets.as_slice()[1].id;
        let operations = if duplicate {
            vec![
                LivingCreatorOperationV2::CreateHull {
                    local: 0,
                    fleet: t(target),
                    definition: LivingSlugV2::new("scout").unwrap(),
                    hit_points: 7,
                    return_credit: false,
                },
                LivingCreatorOperationV2::MergeFleets {
                    target: t(target),
                    source: t(source),
                },
            ]
        } else {
            vec![LivingCreatorOperationV2::SplitFleet {
                local: 0,
                source: t(source),
                hulls: vec![t(base.state().fleets.as_slice()[0].hulls.as_slice()[0].id)],
            }]
        };
        let envelope = request(&base, operations, LivingCommandModeV2::Running);
        let accepted = LivingAcceptedCommandV2 {
            envelope: envelope.clone(),
            application_tick: LivingTickV2(1),
            accepted_sequence: 1,
        };
        let revision =
            LivingRevisionV2::seal(base.catalog().catalog_hash(), [0x67; 32], None, &accepted)
                .unwrap();
        let collision = revision.created_entities()[0].2;
        let mut state = base.state().clone();
        let mut fleets = state.fleets.into_vec();
        if duplicate {
            let mut hulls = fleets[0].hulls.clone().into_vec();
            hulls[0].id = collision;
            hulls.sort_by_key(|row| row.id);
            fleets[0].hulls = LivingSortedVecV2::new(hulls).unwrap();
        } else {
            fleets[2].id = collision;
        }
        fleets.sort_by_key(|row| row.id);
        state.fleets = LivingSortedVecV2::new(fleets).unwrap();
        let mut actual = structural_public_authority(state);
        let before = actual.bookmark().unwrap();
        let expected = LivingCommandRejectionV2::Projection(if duplicate {
            LivingWireErrorV2::KeyedOrder.into()
        } else {
            LivingValidationErrorV2::Range {
                field: "split fleet identity",
            }
        });
        assert_eq!(actual.submit(envelope).unwrap_err(), expected);
        assert_eq!(actual.bookmark().unwrap(), before);
        assert_eq!(actual.accepted_sequence_high_water(), 0);
        assert_eq!(actual.capture_boundary().unwrap().pending_count(), 0);
    }
}

fn removal_public_fixture() -> LivingGalaxyAuthorityV2 {
    let original = structural_public_fixture(2, 2, 0);
    let mut state = original.state().clone();
    let world = state.colonies.as_slice()[0].world;
    let owner = state.colonies.as_slice()[0].owner;
    let other = state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id != world)
        .unwrap()
        .id;
    let id = |n| LivingEntityIdV2([n; 16]);
    state.stars = LivingSortedVecV2::new(vec![LivingStarV2 {
        id: id(0xb0),
        system: state.systems.as_slice()[0].id,
        archetype: LivingSlugV2::new("main_sequence").unwrap(),
        name: LivingNameV2::new("Removal Star").unwrap(),
    }])
    .unwrap();
    state.construction_jobs = LivingSortedVecV2::new(vec![LivingConstructionJobV2 {
        id: id(0xb1),
        world,
        owner,
        definition: LivingSlugV2::new("defense_battery").unwrap(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(100),
        paid_alloy: 40,
    }])
    .unwrap();
    state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
        id: id(0xb2),
        world,
        owner,
        definition: LivingSlugV2::new("scout").unwrap(),
        accepted_tick: LivingTickV2(0),
        completion_tick: LivingTickV2(100),
        paid_alloy: 40,
        target_fleet: None,
    }])
    .unwrap();
    state.routes = LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
        id: id(0xb3),
        kind: LivingRouteKindV2::Internal,
        source_world: world,
        destination_world: other,
        resource: LivingResourceV2::Ore,
        batch_size: 3,
        cadence_ticks: 10,
        source_reserve: 20,
        source_owner: owner,
        receiver_owner: owner,
        next_due: LivingTickV2(10),
        suspended: true,
    }])
    .unwrap();
    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
        id: id(0xb4),
        dispatch_owner: owner,
        intended_receiver: owner,
        source_world: world,
        destination_world: other,
        resource: LivingResourceV2::Ore,
        units: 3,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(100),
        disposition: LivingShipmentDispositionV2::Outbound,
        dispatch_kind: Some(LivingRouteKindV2::Internal),
        returning_to_source: false,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: id(0xb5),
        definition: original.catalog().hazards()[0].id.clone(),
        lane: state.lanes.as_slice()[0].id,
        start_tick: LivingTickV2(20),
        end_tick: LivingTickV2(30),
    }])
    .unwrap();
    structural_public_authority(state)
}
#[test]
fn removal_public_all_stable_kinds_positions_and_nested_hulls_match_full_boundary() {
    let base = removal_public_fixture();
    let state = base.state();
    let mut groups = vec![
        state
            .systems
            .as_slice()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        state.stars.as_slice().iter().map(|r| r.id).collect(),
        state.worlds.as_slice().iter().map(|r| r.id).collect(),
        state.lanes.as_slice().iter().map(|r| r.id).collect(),
        state
            .civilizations
            .as_slice()
            .iter()
            .map(|r| r.id)
            .collect(),
        state.deposits.as_slice().iter().map(|r| r.id).collect(),
        state.facilities.as_slice().iter().map(|r| r.id).collect(),
        state
            .construction_jobs
            .as_slice()
            .iter()
            .map(|r| r.id)
            .collect(),
        state.hull_jobs.as_slice().iter().map(|r| r.id).collect(),
        state.fleets.as_slice().iter().map(|r| r.id).collect(),
        state.routes.as_slice().iter().map(|r| r.id).collect(),
        state.shipments.as_slice().iter().map(|r| r.id).collect(),
        state.hazards.as_slice().iter().map(|r| r.id).collect(),
    ];
    groups.push(
        state
            .fleets
            .as_slice()
            .iter()
            .flat_map(|r| r.hulls.as_slice().iter().map(|h| h.id))
            .collect(),
    );
    assert_eq!(groups.len(), 14);
    for ids in groups {
        assert!(!ids.is_empty());
        for target in [ids[0], ids[ids.len() / 2], ids[ids.len() - 1]] {
            let preview = preview_living_removal_v2(base.state(), target).unwrap();
            for reverse in [false, true] {
                let cascade = if preview.dependents.is_empty() {
                    LivingCascadeDispositionV2::RejectIfReferenced
                } else {
                    let mut dependents = preview
                        .dependents
                        .iter()
                        .map(|id| t(*id))
                        .collect::<Vec<_>>();
                    if reverse {
                        dependents.reverse();
                    }
                    LivingCascadeDispositionV2::RemoveListed { dependents }
                };
                let mut admitted = base.clone();
                admitted
                    .submit(request(
                        &admitted,
                        vec![LivingCreatorOperationV2::RemoveEntity {
                            target: t(target),
                            cascade,
                        }],
                        LivingCommandModeV2::Running,
                    ))
                    .unwrap();
                let actual = cancellation_public_step(&admitted, false);
                assert!(preview_living_removal_v2(actual.state(), target).is_none());
            }
        }
    }
}
#[test]
fn removal_public_scrap_transient_world_reference_and_atomic_later_failure() {
    let base = builtin_first_expansion_v2().unwrap();
    let state = base.state().clone();
    let base = structural_public_authority(state);
    let facility = base.state().facilities.as_slice()[0].id;
    let world = base
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.owner.is_none())
        .unwrap()
        .clone();
    let edit = LivingCreatorOperationV2::EditWorld {
        target: t(world.id),
        system: t(facility),
        archetype: world.archetype.clone(),
        name: world.name.clone(),
    };
    let mut admitted = base.clone();
    admitted
        .submit(request(
            &admitted,
            vec![
                edit,
                LivingCreatorOperationV2::ScrapFacility {
                    target: t(facility),
                    cascade: LivingCascadeDispositionV2::RemoveListed {
                        dependents: vec![t(world.id)],
                    },
                },
            ],
            LivingCommandModeV2::Running,
        ))
        .unwrap();
    let result = cancellation_public_step(&admitted, false);
    assert!(
        !result
            .state()
            .worlds
            .as_slice()
            .iter()
            .any(|r| r.id == world.id)
    );
    assert!(
        !result
            .state()
            .facilities
            .as_slice()
            .iter()
            .any(|r| r.id == facility)
    );
    for scrap in [false, true] {
        let op = if scrap {
            LivingCreatorOperationV2::ScrapFacility {
                target: t(facility),
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            }
        } else {
            LivingCreatorOperationV2::RemoveEntity {
                target: t(facility),
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            }
        };
        let mut admitted = base.clone();
        admitted
            .submit(request(
                &admitted,
                vec![op.clone()],
                LivingCommandModeV2::Running,
            ))
            .unwrap();
        cancellation_public_step(&admitted, false);
        let mut rejected = base.clone();
        let before = encode_living_archive_v2(&rejected).unwrap();
        assert_eq!(
            rejected.submit(request(
                &rejected,
                vec![
                    op,
                    LivingCreatorOperationV2::ScrapFacility {
                        target: t(facility),
                        cascade: LivingCascadeDispositionV2::RejectIfReferenced
                    }
                ],
                LivingCommandModeV2::Running
            )),
            Err(LivingCommandRejectionV2::MissingTarget {
                operation: 1,
                reference: "facility"
            })
        );
        assert_eq!(encode_living_archive_v2(&rejected).unwrap(), before);
    }
}
#[test]
fn removal_public_paid_jobs_have_no_refund_and_observed_owner_is_deferred_projection() {
    let base = cancellation_public_base(9_999, 4);
    for target in [
        base.state().construction_jobs.as_slice()[0].id,
        base.state().hull_jobs.as_slice()[0].id,
    ] {
        let mut admitted = base.clone();
        admitted
            .submit(request(
                &admitted,
                vec![LivingCreatorOperationV2::RemoveEntity {
                    target: t(target),
                    cascade: LivingCascadeDispositionV2::RejectIfReferenced,
                }],
                LivingCommandModeV2::Running,
            ))
            .unwrap();
        let result = cancellation_public_step(&admitted, false);
        assert!(
            !result
                .last_receipt()
                .unwrap()
                .events
                .iter()
                .any(|e| matches!(
                    e.kind,
                    LivingEventKindV2::RefundDiscarded | LivingEventKindV2::ConstructionCancelled
                ))
        );
        assert_eq!(result.state().worlds, base.state().worlds);
    }
    let mut state = builtin_first_expansion_v2().unwrap().state().clone();
    let remembered = LivingEntityIdV2([0xc0; 16]);
    let mut actors = state.civilizations.into_vec();
    actors.push(LivingCivilizationV2 {
        id: remembered,
        name: LivingNameV2::new("Remembered").unwrap(),
        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
        status: LivingCivilizationStatusV2::Dormant,
    });
    actors.sort_by_key(|r| r.id);
    state.civilizations = LivingSortedVecV2::new(actors).unwrap();
    let observer = state.colonies.as_slice()[0].owner;
    let world = state.colonies.as_slice()[0].world;
    state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
        civilization: observer,
        world,
        observed_tick: LivingTickV2(0),
        owner: Some(remembered),
        remaining_deposit: 0,
        facilities: LivingSortedVecV2::new(vec![LivingObservedFacilityV2 {
            definition: LivingSlugV2::new("solar_array").unwrap(),
            count: 1,
        }])
        .unwrap(),
        stationed_hulls: LivingSortedVecV2::new(vec![LivingObservedHullV2 {
            definition: LivingSlugV2::new("scout").unwrap(),
            count: 1,
        }])
        .unwrap(),
        operational_defense_batteries: None,
    }])
    .unwrap();
    let mut authority = structural_public_authority(state);
    let before = encode_living_archive_v2(&authority).unwrap();
    let remove = LivingCreatorOperationV2::RemoveEntity {
        target: t(remembered),
        cascade: LivingCascadeDispositionV2::RejectIfReferenced,
    };
    assert_eq!(
        authority.submit(request(
            &authority,
            vec![remove.clone()],
            LivingCommandModeV2::Running
        )),
        Err(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::DanglingReference {
                reference: "observed owner"
            }
        ))
    );
    assert_eq!(encode_living_archive_v2(&authority).unwrap(), before);
    let preview = preview_living_removal_v2(authority.state(), world).unwrap();
    authority
        .submit(request(
            &authority,
            vec![
                remove,
                LivingCreatorOperationV2::RemoveEntity {
                    target: t(world),
                    cascade: LivingCascadeDispositionV2::RemoveListed {
                        dependents: preview.dependents.into_iter().map(t).collect(),
                    },
                },
            ],
            LivingCommandModeV2::Running,
        ))
        .unwrap();
    cancellation_public_step(&authority, false);
}

#[test]
fn removal_public_status_recovery_and_preowned_dormant_exclusion() {
    for preowned in [false, true] {
        for remove_again in [false, true] {
            let mut state = builtin_first_expansion_v2().unwrap().state().clone();
            let frontier = state
                .worlds
                .as_slice()
                .iter()
                .find(|w| w.owner.is_none())
                .unwrap()
                .id;
            let actor = LivingEntityIdV2([0xe0; 16]);
            let unrelated = LivingEntityIdV2([0xe1; 16]);
            let mut actors = state.civilizations.into_vec();
            actors.push(LivingCivilizationV2 {
                id: actor,
                name: LivingNameV2::new("Recoverable").unwrap(),
                policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                status: LivingCivilizationStatusV2::Dormant,
            });
            actors.push(LivingCivilizationV2 {
                id: unrelated,
                name: LivingNameV2::new("Unrelated status").unwrap(),
                policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                status: LivingCivilizationStatusV2::Active,
            });
            actors.sort_by_key(|r| r.id);
            state.civilizations = LivingSortedVecV2::new(actors).unwrap();
            if preowned {
                let mut worlds = state.worlds.into_vec();
                worlds.iter_mut().find(|r| r.id == frontier).unwrap().owner = Some(actor);
                state.worlds = LivingSortedVecV2::new(worlds).unwrap();
                let mut colony = state.colonies.as_slice()[0].clone();
                colony.world = frontier;
                colony.owner = actor;
                let mut colonies = state.colonies.into_vec();
                colonies.push(colony);
                colonies.sort_by_key(|r| r.world);
                state.colonies = LivingSortedVecV2::new(colonies).unwrap();
            }
            let mut authority = structural_public_authority(state);
            let facility = authority.state().facilities.as_slice()[0].id;
            let mut ops = Vec::new();
            if !preowned {
                ops.push(LivingCreatorOperationV2::CreateColony {
                    world: t(frontier),
                    owner: t(actor),
                });
            }
            ops.push(LivingCreatorOperationV2::ScrapFacility {
                target: t(facility),
                cascade: LivingCascadeDispositionV2::RejectIfReferenced,
            });
            if remove_again {
                ops.push(LivingCreatorOperationV2::RemoveColony { world: t(frontier) });
            }
            authority
                .submit(request(&authority, ops, LivingCommandModeV2::Running))
                .unwrap();
            let actual = cancellation_public_step(&authority, false);
            assert_eq!(
                actual
                    .state()
                    .civilizations
                    .as_slice()
                    .iter()
                    .find(|r| r.id == unrelated)
                    .unwrap()
                    .status,
                LivingCivilizationStatusV2::Dormant
            );
            assert_eq!(
                actual
                    .last_creator_recoveries()
                    .iter()
                    .filter(|r| r.actor() == actor)
                    .count(),
                usize::from(!preowned && !remove_again)
            );
        }
    }
}
