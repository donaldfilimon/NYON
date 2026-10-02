//! Replay-derived observed alloy production and actual inter-system supply transfer.
use nyon_workshop_core::living::notebook::LivingNotebookSupplyTopologyV2;
use nyon_workshop_core::living::*;
#[derive(Clone)]
struct Witness {
    bookmark: LivingBookmarkV2,
    receipt: LivingTickReceiptV2,
    event: LivingEventV2,
}
fn fixture(
    resource: LivingResourceV2,
    arrival: u64,
    due: u64,
    wrong_endpoint: bool,
    same_system: bool,
) -> LivingGalaxyAuthorityV2 {
    let baseline = builtin_first_expansion_v2().unwrap();
    let mut state = baseline.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let source = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.owner == Some(actor))
        .unwrap()
        .id;
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.owner.is_none())
        .unwrap()
        .id;
    let source_system = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == source)
        .unwrap()
        .system;
    let mut worlds = state.worlds.as_slice().to_vec();
    for world in &mut worlds {
        world.owner = Some(actor);
        world.inventory = LivingInventoryV2 {
            energy: 100,
            ore: 100,
            alloy: 0,
        };
        if same_system {
            world.system = source_system;
        }
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut frontier_colony = state.colonies.as_slice()[0].clone();
    frontier_colony.world = destination;
    state.colonies =
        LivingSortedVecV2::new(vec![state.colonies.as_slice()[0].clone(), frontier_colony])
            .unwrap();
    let other = builtin_living_sandbox_v2().unwrap();
    let mut foundry = other
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|facility| facility.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    foundry.id = LivingEntityIdV2([0xa3; 16]);
    foundry.world = if (resource == LivingResourceV2::Ore) != wrong_endpoint {
        destination
    } else {
        source
    };
    foundry.next_due = Some(LivingTickV2(due));
    state.facilities = LivingSortedVecV2::new(vec![foundry]).unwrap();
    state.fleets = LivingSortedVecV2::default();
    state.routes = LivingSortedVecV2::default();
    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
        id: LivingEntityIdV2([0xa4; 16]),
        dispatch_owner: actor,
        intended_receiver: actor,
        source_world: source,
        destination_world: destination,
        resource,
        units: 1,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(arrival),
        disposition: LivingShipmentDispositionV2::Outbound,
        dispatch_kind: Some(LivingRouteKindV2::Internal),
        returning_to_source: false,
    }])
    .unwrap();
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
        [0xa7; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("supply_completion_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn capture(
    authority: &LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    event: &LivingEventV2,
) -> Witness {
    Witness {
        bookmark: authority.bookmark().unwrap(),
        receipt: receipt.clone(),
        event: *event,
    }
}
fn observe(authority: &mut LivingGalaxyAuthorityV2, end: u64) -> (Witness, Witness) {
    let mut production = None;
    let mut delivery = None;
    while authority.state().tick.0 < end {
        let receipt = authority.step().unwrap();
        for event in &receipt.events {
            if event.kind == LivingEventKindV2::FacilityProduced && production.is_none() {
                production = Some(capture(authority, &receipt, event));
            }
            if event.kind == LivingEventKindV2::ShipmentDelivered && delivery.is_none() {
                delivery = Some(capture(authority, &receipt, event));
            }
        }
    }
    (production.unwrap(), delivery.unwrap())
}
fn validate(
    authority: &LivingGalaxyAuthorityV2,
    witnesses: &[Witness],
    budget: usize,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let bookmarks = witnesses
        .iter()
        .enumerate()
        .map(|(index, w)| LivingNamedBookmarkV2 {
            ordinal: index as u64 + 1,
            label: LivingNameV2::new(format!("Supply witness {}", index + 1)).unwrap(),
            view: w.bookmark.view,
            digest: w.bookmark.digest,
        })
        .collect();
    let records = witnesses
        .iter()
        .enumerate()
        .map(|(index, w)| LivingNotebookEvidenceRecordV2 {
            ordinal: index as u64 + 1,
            bookmark: index as u64 + 1,
            experiment: LivingNotebookExperimentKindV2::SupplyChain,
            receipt_digest: w.receipt.digest,
            route_attempts: None,
            events: vec![LivingNotebookEventReferenceV2 {
                ordinal: w.event.ordinal,
                id: w.event.id,
                kind: w.event.kind,
            }],
        })
        .collect();
    let notebook = LivingNotebookV2::seal_with_evidence(
        &archive,
        bookmarks,
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: witnesses.len() as u64 + 1,
            records,
        }),
    )
    .unwrap();
    let bytes = notebook.canonical_bytes().unwrap();
    let mut replay = begin_decode_living_notebook_v2(&bytes, archive)?;
    while replay.poll(budget)? == LivingArchiveReplayStatusV2::Pending {}
    let result = replay.finish()?;
    assert_eq!(result.notebook().canonical_bytes().unwrap(), bytes);
    Ok(result)
}
fn edit(authority: &LivingGalaxyAuthorityV2) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Running,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::EditSystem {
                target: LivingCommandTargetV2::Existing {
                    id: authority.state().systems.as_slice()[0].id,
                },
                name: LivingNameV2::new("Observed supply system").unwrap(),
            }],
        },
    }
}

#[test]
fn both_observed_topologies_match_exact_actors_worlds_systems_and_quantities() {
    for (resource, arrival, due, topology) in [
        (
            LivingResourceV2::Alloy,
            21,
            20,
            LivingNotebookSupplyTopologyV2::AlloyDelivered,
        ),
        (
            LivingResourceV2::Ore,
            20,
            20,
            LivingNotebookSupplyTopologyV2::OreDeliveredToAlloyProducer,
        ),
    ] {
        let mut authority = fixture(resource, arrival, due, false, false);
        let (p, d) = observe(&mut authority, arrival.max(due));
        for budget in [1, 128] {
            let validated = validate(&authority, &[p.clone(), d.clone()], budget).unwrap();
            let proof = &validated.supply_completions()[0];
            assert_eq!(validated.supply_completions().len(), 1);
            assert_eq!(proof.topology(), topology);
            assert_eq!(proof.production_record(), 1);
            assert_eq!(proof.delivery_record(), 2);
            assert_eq!(proof.production_view(), p.bookmark.view);
            assert_eq!(proof.delivery_view(), d.bookmark.view);
            assert_eq!(proof.production_event().id, p.event.id);
            assert_eq!(proof.delivery_event().id, d.event.id);
            assert!(proof.ore_input() > 0 && proof.alloy_output() > 0);
            assert_eq!(proof.delivery().units, 1);
            assert_ne!(
                proof.delivery().source_system,
                proof.delivery().destination_system
            );
        }
        for single in [p, d] {
            assert!(
                validate(&authority, &[single], 128)
                    .unwrap()
                    .supply_completions()
                    .is_empty()
            );
        }
    }
}

#[test]
fn wrong_endpoint_and_wrong_temporal_order_do_not_join_valid_observations() {
    for (resource, arrival, due, wrong_endpoint) in [
        (LivingResourceV2::Alloy, 21, 20, true),
        (LivingResourceV2::Ore, 20, 20, true),
        (LivingResourceV2::Alloy, 20, 20, false),
        (LivingResourceV2::Ore, 21, 20, false),
    ] {
        let mut authority = fixture(resource, arrival, due, wrong_endpoint, false);
        let (p, d) = observe(&mut authority, arrival.max(due));
        assert!(
            validate(&authority, &[p, d], 128)
                .unwrap()
                .supply_completions()
                .is_empty()
        );
    }
}

#[test]
fn same_system_and_incorrect_resource_reject_supply_reference() {
    for (resource, same_system) in [
        (LivingResourceV2::Alloy, true),
        (LivingResourceV2::Energy, false),
    ] {
        let mut authority = fixture(resource, 21, 20, false, same_system);
        let (p, d) = observe(&mut authority, 21);
        assert!(matches!(
            validate(&authority, &[p, d], 128),
            Err(LivingNotebookErrorV2::Evidence)
        ));
    }
}

#[test]
fn sibling_branches_and_reversed_creator_prefix_do_not_join() {
    let mut authority = fixture(LivingResourceV2::Alloy, 40, 20, false, false);
    while authority.state().tick.0 < 19 {
        authority.step().unwrap();
    }
    authority.submit(edit(&authority)).unwrap();
    let receipt = authority.step().unwrap();
    let p = capture(
        &authority,
        &receipt,
        receipt
            .events
            .iter()
            .find(|event| event.kind == LivingEventKindV2::FacilityProduced)
            .unwrap(),
    );
    let (_, direct) = observe(&mut authority, 40);
    let view = LivingHistoryViewV2 {
        revision: None,
        ..direct.bookmark.view
    };
    let mut restore = begin_restore_living_view_v2(&authority, view).unwrap();
    while restore.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    let alternate = restore.finish().unwrap().into_authority();
    let receipt = alternate.last_receipt().unwrap();
    let d = capture(
        &alternate,
        receipt,
        receipt
            .events
            .iter()
            .find(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
            .unwrap(),
    );
    assert!(
        validate(&authority, &[p.clone(), d], 128)
            .unwrap()
            .supply_completions()
            .is_empty()
    );
    authority.restore_bookmark(p.bookmark).unwrap();
    authority.fork_view_paused(vec![]).unwrap();
    let (_, sibling) = observe(&mut authority, 40);
    assert!(
        validate(&authority, &[p, sibling], 128)
            .unwrap()
            .supply_completions()
            .is_empty()
    );
}

#[test]
fn shared_observation_predicate_rejects_stale_and_forged_receipt_or_event() {
    use nyon_workshop_core::living::notebook::supports_living_supply_observation_v2;
    let mut authority = fixture(LivingResourceV2::Alloy, 21, 20, false, false);
    while authority.state().tick.0 < 19 {
        authority.step().unwrap();
    }
    let production = authority.step().unwrap();
    let event = production
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::FacilityProduced)
        .unwrap();
    assert!(supports_living_supply_observation_v2(
        &authority,
        &production,
        event
    ));
    let mut wrong_receipt = production.clone();
    wrong_receipt.digest = LivingReceiptDigestV2([0; 32]);
    assert!(!supports_living_supply_observation_v2(
        &authority,
        &wrong_receipt,
        event
    ));
    let mut wrong_event = *event;
    wrong_event.kind = LivingEventKindV2::FacilityBlocked;
    assert!(!supports_living_supply_observation_v2(
        &authority,
        &production,
        &wrong_event
    ));
    let delivery = authority.step().unwrap();
    let delivered = delivery
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
        .unwrap();
    assert!(supports_living_supply_observation_v2(
        &authority, &delivery, delivered
    ));
    assert!(!supports_living_supply_observation_v2(
        &authority,
        &production,
        event
    ));
}
