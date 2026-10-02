//! Linked Trade proof derives from replayed observations, never a persisted completion bit.
use nyon::living_content::LivingContent;
use nyon_workshop_core::living::*;

#[derive(Clone)]
struct Witness {
    bookmark: LivingBookmarkV2,
    receipt: LivingTickReceiptV2,
    event: LivingEventV2,
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
fn step_until(
    authority: &mut LivingGalaxyAuthorityV2,
    kind: LivingEventKindV2,
    after: u64,
    units: Option<u32>,
) -> Witness {
    for _ in 0..1500 {
        let receipt = authority.step().unwrap();
        for event in &receipt.events {
            if event.kind == kind && receipt.tick.0 > after {
                let facts = &authority
                    .last_event_facts()
                    .unwrap()
                    .events
                    .iter()
                    .find(|entry| entry.ordinal == event.ordinal)
                    .unwrap()
                    .facts;
                if units.is_none_or(|units| {
                    facts
                        .delivery
                        .as_ref()
                        .is_some_and(|delivery| delivery.units == units)
                }) {
                    return capture(authority, &receipt, event);
                }
            }
        }
    }
    panic!("fixture did not emit requested witness");
}
fn validate(
    authority: &LivingGalaxyAuthorityV2,
    witnesses: &[Witness],
    budget: usize,
) -> ValidatedLivingNotebookV2 {
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
            label: LivingNameV2::new(format!("Trade witness {}", index + 1)).unwrap(),
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
            experiment: LivingNotebookExperimentKindV2::TradeDelivery,
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
    let mut job = begin_decode_living_notebook_v2(&bytes, archive).unwrap();
    while job.poll(budget).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    let validated = job.finish().unwrap();
    assert_eq!(
        validated.notebook().canonical_bytes().unwrap(),
        bytes,
        "derived proof does not enter canonical bytes"
    );
    validated
}
fn formation(authority: &mut LivingGalaxyAuthorityV2) -> Witness {
    step_until(authority, LivingEventKindV2::AgreementFormed, 0, None)
}
fn fixture(arrival: u64, third_party: bool) -> LivingGalaxyAuthorityV2 {
    fixture_with_direction(arrival, third_party, false)
}
fn fixture_with_direction(
    arrival: u64,
    third_party: bool,
    reverse: bool,
) -> LivingGalaxyAuthorityV2 {
    let baseline = LivingContent::Commerce.open().unwrap();
    let mut state = baseline.state().clone();
    let mut route = state.routes.as_slice()[0].clone();
    if reverse {
        std::mem::swap(&mut route.source_owner, &mut route.receiver_owner);
        std::mem::swap(&mut route.source_world, &mut route.destination_world);
    }
    let sender = if third_party {
        state
            .civilizations
            .as_slice()
            .iter()
            .find(|actor| actor.id != route.source_owner && actor.id != route.receiver_owner)
            .unwrap()
            .id
    } else {
        route.source_owner
    };
    let source = if third_party {
        state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner == Some(sender))
            .unwrap()
            .id
    } else {
        route.source_world
    };
    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
        id: LivingEntityIdV2([0xf7; 16]),
        dispatch_owner: sender,
        intended_receiver: route.receiver_owner,
        source_world: source,
        destination_world: route.destination_world,
        resource: LivingResourceV2::Ore,
        units: 1,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(arrival),
        disposition: LivingShipmentDispositionV2::Outbound,
        dispatch_kind: Some(LivingRouteKindV2::Trade),
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
        [0x7f; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("trade_completion_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn edit_name(
    authority: &LivingGalaxyAuthorityV2,
    mode: LivingCommandModeV2,
) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::EditSystem {
                target: LivingCommandTargetV2::Existing {
                    id: authority.state().systems.as_slice()[0].id,
                },
                name: LivingNameV2::new("Observed system").unwrap(),
            }],
        },
    }
}

#[test]
fn commerce_links_formation_and_delivery_without_a_mandatory_baseline() {
    let mut authority = LivingContent::Commerce.open().unwrap();
    let f = formation(&mut authority);
    let d = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        f.receipt.tick.0,
        None,
    );
    for budget in [1, 128] {
        let validated = validate(&authority, &[f.clone(), d.clone()], budget);
        let proof = &validated.trade_completions()[0];
        assert_eq!(validated.trade_completions().len(), 1);
        assert_eq!(proof.formation_record(), 1);
        assert_eq!(proof.delivery_record(), 2);
        assert_eq!(proof.formation_bookmark(), 1);
        assert_eq!(proof.delivery_bookmark(), 2);
        assert_eq!(proof.formation_view(), f.bookmark.view);
        assert_eq!(proof.delivery_view(), d.bookmark.view);
        assert_eq!(proof.formation_event().id, f.event.id);
        assert_eq!(proof.delivery_event().id, d.event.id);
        assert_eq!(proof.agreement().kind, LivingAgreementKindV2::Trade);
        assert_eq!(proof.delivery().units, 10);
    }
    for witness in [f, d] {
        assert!(
            validate(&authority, &[witness], 128)
                .trade_completions()
                .is_empty()
        );
    }
}

#[test]
fn preexisting_cargo_can_support_observed_stages_without_a_dispatch_causation_claim() {
    let mut authority = fixture(150, false);
    let f = formation(&mut authority);
    let d = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        f.receipt.tick.0.max(149),
        Some(1),
    );
    assert_eq!(
        validate(&authority, &[f, d], 128).trade_completions().len(),
        1
    );
}

#[test]
fn same_tick_wrong_party_and_expired_agreement_remain_individual_observations() {
    for (arrival, third_party) in [(100, false), (150, true), (1300, false)] {
        let mut authority = fixture(arrival, third_party);
        let f = formation(&mut authority);
        let d =
            if arrival == 100 {
                let event =
                    f.receipt
                        .events
                        .iter()
                        .find(|event| {
                            event.kind == LivingEventKindV2::ShipmentDelivered
                                && authority.last_event_facts().unwrap().events.iter().any(
                                    |entry| {
                                        entry.ordinal == event.ordinal
                                            && entry
                                                .facts
                                                .delivery
                                                .as_ref()
                                                .is_some_and(|delivery| delivery.units == 1)
                                    },
                                )
                        })
                        .unwrap();
                capture(&authority, &f.receipt, event)
            } else {
                step_until(
                    &mut authority,
                    LivingEventKindV2::ShipmentDelivered,
                    f.receipt.tick.0.max(arrival - 1),
                    Some(1),
                )
            };
        assert!(
            validate(&authority, &[f, d], 128)
                .trade_completions()
                .is_empty(),
            "arrival={arrival}, third={third_party}"
        );
    }
}

#[test]
fn sibling_branches_do_not_join_and_forward_creator_prefix_does() {
    let mut authority = LivingContent::Commerce.open().unwrap();
    let f = formation(&mut authority);
    authority
        .apply_paused(edit_name(&authority, LivingCommandModeV2::Paused))
        .unwrap();
    let d = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        f.receipt.tick.0,
        None,
    );
    assert_eq!(
        validate(&authority, &[f.clone(), d], 128)
            .trade_completions()
            .len(),
        1
    );
    authority.fork_view_paused(vec![]).unwrap();
    let sibling = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        f.receipt.tick.0,
        None,
    );
    assert!(
        validate(&authority, &[f, sibling], 128)
            .trade_completions()
            .is_empty()
    );
}

#[test]
fn reversing_the_actual_creator_prefix_does_not_join_matching_ticks_and_parties() {
    let mut authority = LivingContent::Commerce.open().unwrap();
    while authority.state().tick.0 < 99 {
        authority.step().unwrap();
    }
    authority
        .submit(edit_name(&authority, LivingCommandModeV2::Running))
        .unwrap();
    let receipt = authority.step().unwrap();
    let event = receipt
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::AgreementFormed)
        .unwrap();
    let f = capture(&authority, &receipt, event);
    let direct = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        100,
        None,
    );
    let view = LivingHistoryViewV2 {
        revision: None,
        ..direct.bookmark.view
    };
    let mut job = begin_restore_living_view_v2(&authority, view).unwrap();
    while job.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    let alternate = job.finish().unwrap().into_authority();
    assert!(alternate.revisions().is_empty());
    let receipt = alternate.last_receipt().unwrap();
    let event = receipt
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
        .unwrap();
    let d = capture(&alternate, receipt, event);
    assert_eq!(f.bookmark.view.branch, d.bookmark.view.branch);
    assert!(
        validate(&authority, &[f, d], 128)
            .trade_completions()
            .is_empty()
    );
}

#[test]
fn reverse_direction_matches_the_same_bilateral_parties() {
    let mut authority = fixture_with_direction(150, false, true);
    let f = formation(&mut authority);
    let d = step_until(
        &mut authority,
        LivingEventKindV2::ShipmentDelivered,
        149,
        Some(1),
    );
    let validated = validate(&authority, &[f, d], 128);
    let proof = &validated.trade_completions()[0];
    assert_eq!(
        proof
            .delivery()
            .dispatch_owner
            .min(proof.delivery().recipient),
        proof.agreement().participant_a
    );
    assert_eq!(
        proof
            .delivery()
            .dispatch_owner
            .max(proof.delivery().recipient),
        proof.agreement().participant_b
    );
}

#[test]
fn removed_and_replaced_agreements_do_not_join_a_valid_later_delivery() {
    for replace in [false, true] {
        let mut authority = fixture(150, false);
        let f = formation(&mut authority);
        let agreement = *authority
            .last_event_facts()
            .unwrap()
            .events
            .iter()
            .find_map(|entry| entry.facts.agreement.as_ref())
            .unwrap();
        let target = |id| LivingCommandTargetV2::Existing { id };
        let operation = if replace {
            LivingCreatorOperationV2::SetAgreement {
                kind: agreement.kind,
                participant_a: target(agreement.participant_a),
                participant_b: target(agreement.participant_b),
                start_tick: f.receipt.tick,
                end_tick: LivingTickV2(agreement.end_tick.0 + 1),
            }
        } else {
            LivingCreatorOperationV2::RemoveAgreement {
                kind: agreement.kind,
                participant_a: target(agreement.participant_a),
                participant_b: target(agreement.participant_b),
            }
        };
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![operation],
                },
            })
            .unwrap();
        let d = step_until(
            &mut authority,
            LivingEventKindV2::ShipmentDelivered,
            149,
            Some(1),
        );
        assert!(
            validate(&authority, &[f, d], 128)
                .trade_completions()
                .is_empty()
        );
    }
}
