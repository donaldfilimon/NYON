//! Trade evidence observes exact producer facts, never present inventory deltas.
use nyon::living_content::LivingContent;
use nyon_workshop_core::living::*;
use serde_json::{Value, json};

fn id(value: LivingEntityIdV2) -> Value {
    serde_json::to_value(value).unwrap()
}
fn explained(
    authority: &LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    ordinal: u16,
) -> Value {
    let facts = authority
        .last_event_facts()
        .expect("committed event facts sidecar");
    facts.verify_receipt(receipt).unwrap();
    let entry = facts
        .events
        .iter()
        .find(|entry| entry.ordinal == ordinal)
        .expect("ordinary event has producer-supplied facts");
    assert_eq!(entry.event_id, receipt.events[usize::from(ordinal)].id);
    serde_json::to_value(&entry.facts).unwrap()
}

#[test]
fn commerce_formation_and_real_delivery_expose_exact_parties_and_accepted_transfer() {
    let mut authority = LivingContent::Commerce.open().unwrap();
    let route = authority.state().routes.as_slice()[0].clone();
    let source_system = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == route.source_world)
        .unwrap()
        .system;
    let destination_system = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == route.destination_world)
        .unwrap()
        .system;
    assert_ne!(source_system, destination_system);
    let mut formation = None;
    let mut delivery = None;
    for _ in 0..200 {
        let receipt = authority.step().unwrap();
        for event in &receipt.events {
            match event.kind {
                LivingEventKindV2::AgreementFormed => {
                    let facts = explained(&authority, &receipt, event.ordinal);
                    let agreement = &facts["agreement"];
                    if agreement["kind"] == "trade"
                        && agreement["participant_a"]
                            == id(route.source_owner.min(route.receiver_owner))
                        && agreement["participant_b"]
                            == id(route.source_owner.max(route.receiver_owner))
                    {
                        assert_eq!(agreement["start_tick"], json!(receipt.tick.0));
                        assert_eq!(agreement["end_tick"], json!(receipt.tick.0 + 1200));
                        formation.get_or_insert(receipt.tick.0);
                    }
                }
                LivingEventKindV2::ShipmentDelivered => {
                    let facts = explained(&authority, &receipt, event.ordinal);
                    let transfer = &facts["delivery"];
                    if transfer["source_world"] == id(route.source_world)
                        && transfer["destination_world"] == id(route.destination_world)
                        && transfer["resource"] == "ore"
                    {
                        assert_eq!(transfer["dispatch_owner"], id(route.source_owner));
                        assert_eq!(transfer["intended_receiver"], id(route.receiver_owner));
                        assert_eq!(transfer["recipient"], id(route.receiver_owner));
                        assert_eq!(transfer["units"], json!(10));
                        assert_eq!(transfer["dispatch_kind"], "trade");
                        assert_eq!(transfer["source_system"], id(source_system));
                        assert_eq!(transfer["destination_system"], id(destination_system));
                        assert!(matches!(
                            event.provenance,
                            LivingEventProvenanceV2::Autonomous { .. }
                        ));
                        delivery.get_or_insert(receipt.tick.0);
                    }
                }
                _ => {}
            }
        }
    }
    assert!(
        formation.is_some_and(|tick| tick <= 100),
        "matching bilateral Trade formation missing"
    );
    assert!(
        delivery.is_some_and(|tick| tick <= 200),
        "matching actual Trade delivery missing"
    );
}

fn cargo_fixture(
    count: usize,
    full: bool,
    returning: bool,
    captured: bool,
    legacy: bool,
    same_world: bool,
) -> (LivingGalaxyAuthorityV2, LivingEntityIdV2) {
    let baseline = LivingContent::Commerce.open().unwrap();
    let mut state = baseline.state().clone();
    let route = state.routes.as_slice()[0].clone();
    state.routes = LivingSortedVecV2::default();
    state.facilities = LivingSortedVecV2::default();
    state.construction_jobs = LivingSortedVecV2::default();
    let recipient_world = if returning {
        route.source_world
    } else {
        route.destination_world
    };
    let mut worlds = state.worlds.as_slice().to_vec();
    worlds
        .iter_mut()
        .find(|row| row.id == recipient_world)
        .unwrap()
        .inventory
        .ore = if full { 10000 } else { 0 };
    if captured {
        let conqueror = state
            .civilizations
            .as_slice()
            .iter()
            .find(|row| row.id != route.source_owner && row.id != route.receiver_owner)
            .unwrap()
            .id;
        worlds
            .iter_mut()
            .find(|row| row.id == recipient_world)
            .unwrap()
            .owner = Some(conqueror);
        let mut colonies = state.colonies.as_slice().to_vec();
        colonies
            .iter_mut()
            .filter(|row| row.world == recipient_world)
            .for_each(|row| row.owner = conqueror);
        state.colonies = LivingSortedVecV2::new(colonies).unwrap();
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    state.shipments = LivingSortedVecV2::new(
        (0..count)
            .map(|index| {
                let mut bytes = [0xf1; 16];
                bytes[0..8].copy_from_slice(&(index as u64).to_be_bytes());
                LivingShipmentV2 {
                    id: LivingEntityIdV2(bytes),
                    dispatch_owner: route.source_owner,
                    intended_receiver: route.receiver_owner,
                    source_world: if same_world {
                        route.destination_world
                    } else {
                        route.source_world
                    },
                    destination_world: route.destination_world,
                    resource: LivingResourceV2::Ore,
                    units: 1,
                    departure_tick: LivingTickV2(0),
                    arrival_tick: LivingTickV2(1),
                    disposition: if returning {
                        LivingShipmentDispositionV2::Returning
                    } else {
                        LivingShipmentDispositionV2::Outbound
                    },
                    dispatch_kind: (!legacy).then_some(LivingRouteKindV2::Trade),
                    returning_to_source: false,
                }
            })
            .collect(),
    )
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
    (
        LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            [0x74; 32],
            LivingGenesisGeneratorV2 {
                generator_id: LivingSlugV2::new("trade_evidence_fixture").unwrap(),
                generator_version: 1,
            },
            genesis,
        )
        .unwrap(),
        recipient_world,
    )
}

#[test]
fn full_4096_actual_deliveries_commit_with_explicit_capacity_unknowns() {
    let (mut authority, destination) = cargo_fixture(4096, false, false, false, false, false);
    let receipt = authority.step().unwrap();
    let events: Vec<_> = receipt
        .events
        .iter()
        .filter(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
        .collect();
    assert_eq!(events.len(), 4096);
    assert!(authority.state().shipments.as_slice().is_empty());
    assert_eq!(
        authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == destination)
            .unwrap()
            .inventory
            .ore,
        4096
    );
    let facts = authority.last_event_facts().unwrap();
    facts.verify_receipt(&receipt).unwrap();
    assert!(facts.omitted.is_some());
    assert!(facts.events.len() < 4096);
    assert!(facts.canonical_bytes().unwrap().len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
    for entry in &facts.events {
        let delivery = entry.facts.delivery.as_ref().unwrap();
        assert_eq!(delivery.units, 1);
        assert_eq!(delivery.destination_world, destination);
    }
}

#[test]
fn storage_wait_has_no_delivery_fact_and_later_complete_acceptance_has_one() {
    let (mut authority, destination) = cargo_fixture(1, true, false, false, false, false);
    let waiting = authority.step().unwrap();
    assert!(
        waiting
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::ShipmentWaiting)
    );
    assert!(
        authority
            .last_event_facts()
            .unwrap()
            .events
            .iter()
            .all(|event| event.facts.delivery.is_none())
    );
    let mut inventory = authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == destination)
        .unwrap()
        .inventory;
    inventory.ore = 9999;
    let cursor = authority.published_cursor();
    authority
        .apply_paused(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![LivingCreatorOperationV2::SetWorldInventory {
                    world: LivingCommandTargetV2::Existing { id: destination },
                    inventory,
                }],
            },
        })
        .unwrap();
    let accepted = authority.step().unwrap();
    let event = accepted
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
        .unwrap();
    let facts = explained(&authority, &accepted, event.ordinal);
    assert_eq!(facts["delivery"]["units"], 1);
    assert_eq!(
        authority
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == destination)
            .unwrap()
            .inventory
            .ore,
        10000
    );
}

#[test]
fn captured_returned_and_legacy_manifests_do_not_invent_trade_delivery() {
    for (returning, captured, legacy, kind) in [
        (false, true, false, LivingEventKindV2::ShipmentCaptured),
        (true, false, false, LivingEventKindV2::ShipmentReturned),
        (false, false, true, LivingEventKindV2::ShipmentDelivered),
    ] {
        let (mut authority, _) = cargo_fixture(1, false, returning, captured, legacy, false);
        let receipt = authority.step().unwrap();
        let event = receipt
            .events
            .iter()
            .find(|event| event.kind == kind)
            .unwrap();
        if legacy {
            assert!(
                explained(&authority, &receipt, event.ordinal)["delivery"]["dispatch_kind"]
                    .is_null()
            );
        } else {
            assert!(
                authority
                    .last_event_facts()
                    .unwrap()
                    .events
                    .iter()
                    .all(|event| event.facts.delivery.is_none())
            );
        }
    }
}

fn validate_trade_reference(
    authority: &LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    event: &LivingEventV2,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let bookmark = authority.bookmark().unwrap();
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let notebook = LivingNotebookV2::seal_with_evidence(
        &archive,
        vec![LivingNamedBookmarkV2 {
            ordinal: 1,
            label: LivingNameV2::new("Trade observation").unwrap(),
            view: bookmark.view,
            digest: bookmark.digest,
        }],
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 2,
            records: vec![LivingNotebookEvidenceRecordV2 {
                ordinal: 1,
                bookmark: 1,
                experiment: LivingNotebookExperimentKindV2::TradeDelivery,
                receipt_digest: receipt.digest,
                route_attempts: None,
                events: vec![LivingNotebookEventReferenceV2 {
                    ordinal: event.ordinal,
                    id: event.id,
                    kind: event.kind,
                }],
            }],
        }),
    )?;
    let mut replay = begin_decode_living_notebook_v2(&notebook.canonical_bytes()?, archive)?;
    while replay.poll(128)? == LivingArchiveReplayStatusV2::Pending {}
    replay.finish()
}

#[test]
fn notebook_trade_reference_replays_typed_fact_and_rejects_legacy_or_same_world() {
    for (legacy, same_world, accepted) in [
        (false, false, true),
        (true, false, false),
        (false, true, false),
    ] {
        let (mut authority, _) = cargo_fixture(1, false, false, false, legacy, same_world);
        let receipt = authority.step().unwrap();
        let event = receipt
            .events
            .iter()
            .find(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
            .unwrap();
        let result = validate_trade_reference(&authority, &receipt, event);
        assert_eq!(
            result.is_ok(),
            accepted,
            "legacy={legacy} same_world={same_world}"
        );
        if !accepted {
            assert!(matches!(result, Err(LivingNotebookErrorV2::Evidence)));
        }
    }
}

#[test]
fn notebook_trade_reference_rejects_an_explicit_capacity_omission() {
    let (mut authority, _) = cargo_fixture(4096, false, false, false, false, false);
    let receipt = authority.step().unwrap();
    let omitted = authority
        .last_event_facts()
        .unwrap()
        .omitted
        .as_ref()
        .unwrap()
        .ranges[0][0];
    let event = &receipt.events[usize::from(omitted)];
    assert!(matches!(
        validate_trade_reference(&authority, &receipt, event),
        Err(LivingNotebookErrorV2::Evidence)
    ));
}

#[test]
fn arrival_details_and_capacity_omissions_match_across_executor_slice_budgets() {
    let (mut direct, _) = cargo_fixture(4096, false, false, false, false, false);
    let original = direct.clone();
    let receipt = direct.step().unwrap();
    let facts = direct.last_event_facts().unwrap().clone();
    for budget in [1, 17, 1024] {
        let mut sliced = original.clone();
        let mut job = sliced.begin_step().unwrap();
        while job.poll(budget).unwrap() == LivingStepProgressV2::Pending {}
        let prepared = match job.finish_synchronously(&sliced).unwrap() {
            LivingStepPreparationV2::Ready(prepared) => prepared,
            LivingStepPreparationV2::Failed(_) => panic!("valid fixture failed preparation"),
        };
        let retired = sliced.commit_step(prepared).unwrap();
        drop(retired);
        assert_eq!(sliced.last_receipt().unwrap(), &receipt);
        assert_eq!(sliced.last_event_facts().unwrap(), &facts);
        assert_eq!(sliced.state(), direct.state());
    }
}
