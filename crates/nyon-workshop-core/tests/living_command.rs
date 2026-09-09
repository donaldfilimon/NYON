//! Reviewed exact-byte tests for Living Galaxy V2 creator commands and the
//! revisions they become.
//!
//! The expected command bytes, revision identity and created entity identities
//! come from the `creator_command` section of
//! `tests/fixtures/living-v2/vectors.json`. That section was written from the
//! operation schema declared in `living/command.rs` and hashed by
//! `tools/living-v2-vectors.py`, never read back out of this crate's encoder,
//! so a change to the encoding fails here instead of quietly redefining what
//! "canonical" means.

use nyon_workshop_core::living::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2, LivingAcceptedCommandV2,
    LivingCascadeDispositionV2, LivingCatalogHashV2, LivingCommandEnvelopeV2, LivingCommandErrorV2,
    LivingCommandModeV2, LivingCommandTargetV2, LivingCommandV2, LivingCreatorOperationV2,
    LivingEntityIdV2, LivingEntityKindV2, LivingNameV2, LivingRevisionIdV2, LivingRevisionV2,
    LivingTickV2, LivingWireErrorV2, decode_canonical_v2,
};
use serde_json::Value;

const VECTORS: &str = include_str!("fixtures/living-v2/vectors.json");
const SEED: [u8; 32] = [0x11; 32];

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("reviewed Living V2 vectors parse")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "hexadecimal is byte aligned");
    (0..text.len() / 2)
        .map(|index| {
            u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("lowercase hexadecimal")
        })
        .collect()
}

fn hex32(value: &Value) -> [u8; 32] {
    let bytes = hex_bytes(value.as_str().expect("a hexadecimal string"));
    <[u8; 32]>::try_from(bytes.as_slice()).expect("32 bytes")
}

fn hex16(value: &Value) -> [u8; 16] {
    let bytes = hex_bytes(value.as_str().expect("a hexadecimal string"));
    <[u8; 16]>::try_from(bytes.as_slice()).expect("16 bytes")
}

fn name(text: &str) -> LivingNameV2 {
    LivingNameV2::new(text).expect("a printable display name")
}

fn system(local: u16, text: &str) -> LivingCreatorOperationV2 {
    LivingCreatorOperationV2::CreateSystem {
        local,
        name: name(text),
    }
}

fn existing(byte: u8) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Existing {
        id: LivingEntityIdV2([byte; 16]),
    }
}

/// The exact batch the reviewed `creator_command` vector describes: two
/// systems, then a lane that reaches back to both of them by batch-local
/// identifier.
fn reviewed_batch() -> LivingCommandV2 {
    LivingCommandV2::CreatorBatch {
        operations: vec![
            system(0, "Vale"),
            system(1, "Confluence"),
            LivingCreatorOperationV2::CreateLane {
                local: 2,
                system_a: LivingCommandTargetV2::Local { local: 0 },
                system_b: LivingCommandTargetV2::Local { local: 1 },
                distance_units: 101,
            },
        ],
    }
}

fn reviewed_accepted() -> LivingAcceptedCommandV2 {
    let corpus = vectors();
    LivingAcceptedCommandV2 {
        accepted_sequence: corpus["creator_command"]["ordinal"]
            .as_u64()
            .expect("ordinal"),
        application_tick: LivingTickV2(corpus["creator_command"]["tick"].as_u64().expect("tick")),
        envelope: LivingCommandEnvelopeV2 {
            expected_committed_revision: None,
            expected_tick: LivingTickV2(6),
            expected_pending_sequence: 0,
            mode: LivingCommandModeV2::Running,
            command: reviewed_batch(),
        },
    }
}

// ------------------------------------------------------------ exact vectors

/// The batch must encode to the exact bytes the corpus carries. Those bytes are
/// hashed into every revision identity, so this single assertion pins the
/// discriminant spelling, the `type`-first tagged operations, the field order
/// of each operation, and the shape of a batch-local reference.
#[test]
fn the_reviewed_batch_encodes_to_the_reviewed_command_bytes() {
    let expected = hex_bytes(
        vectors()["creator_command"]["command_bytes_hex"]
            .as_str()
            .expect("command bytes"),
    );
    let encoded = reviewed_batch()
        .canonical_bytes()
        .expect("the batch encodes");
    assert_eq!(
        encoded,
        expected,
        "encoded as {}",
        String::from_utf8_lossy(&encoded)
    );
}

/// Sealing must produce the reviewed revision identity, which is the framing of
/// section 10 applied to those exact command bytes.
#[test]
fn sealing_the_reviewed_batch_produces_the_reviewed_revision() {
    let corpus = vectors();
    let catalog = LivingCatalogHashV2(hex32(&corpus["creator_command"]["catalog_hash"]));
    let revision = LivingRevisionV2::seal(catalog, SEED, None, &reviewed_accepted())
        .expect("the reviewed batch seals");

    assert_eq!(
        revision.id.0,
        hex32(&corpus["creator_command"]["revision_id"])
    );
    assert_eq!(revision.parent, None);
    assert_eq!(revision.tick, LivingTickV2(7));
    assert_eq!(revision.ordinal, 3);
    assert_eq!(revision.command, reviewed_batch());
}

/// Every created entity's identity must be the one the corpus derives from that
/// revision and the published registry kind. This is the end-to-end tie: an
/// operation chooses its kind, the registry chooses that kind's ordinal, and
/// the ordinal is hashed.
#[test]
fn the_batch_creates_the_reviewed_entity_identities() {
    let corpus = vectors();
    let catalog = LivingCatalogHashV2(hex32(&corpus["creator_command"]["catalog_hash"]));
    let revision = LivingRevisionV2::seal(catalog, SEED, None, &reviewed_accepted())
        .expect("the reviewed batch seals");

    let created = revision.created_entities();
    let rows = corpus["creator_command"]["created_entities"]
        .as_array()
        .expect("created rows");
    assert_eq!(created.len(), rows.len());
    for ((local, kind, id), row) in created.iter().zip(rows) {
        assert_eq!(
            u64::from(*local),
            row["batch_local_id"].as_u64().expect("local")
        );
        assert_eq!(
            u64::from(kind.ordinal()),
            row["entity_kind"].as_u64().expect("kind")
        );
        assert_eq!(id.0, hex16(&row["entity_id"]));
    }
    assert_eq!(created[0].1, LivingEntityKindV2::System);
    assert_eq!(created[2].1, LivingEntityKindV2::Lane);
}

/// An operation is bound to one registry kind, and the fourteen creating
/// variants cover the registry exactly. A fifteenth entity kind added without a
/// creating operation, or a creating operation reusing another's kind, fails
/// here.
#[test]
fn the_creating_operations_cover_the_entity_kind_registry_exactly() {
    let inventory = serde_json::from_str(r#"{"energy":0,"ore":0,"alloy":0}"#).expect("inventory");
    let archetype = serde_json::from_str::<Value>("\"rocky\"").expect("slug json");
    let slug = serde_json::from_value(archetype).expect("slug");
    let operations = vec![
        system(0, "Vale"),
        LivingCreatorOperationV2::CreateStar {
            local: 1,
            system: existing(0x01),
            archetype: serde_json::from_str("\"main_sequence\"").expect("slug"),
            name: name("Vale Prime"),
        },
        LivingCreatorOperationV2::CreateWorld {
            local: 2,
            system: existing(0x01),
            archetype: slug,
            name: name("Vale I"),
            owner: None,
            inventory,
        },
        LivingCreatorOperationV2::CreateLane {
            local: 3,
            system_a: existing(0x01),
            system_b: existing(0x02),
            distance_units: 10,
        },
        LivingCreatorOperationV2::CreateCivilization {
            local: 4,
            name: name("Kestrel Compact"),
            policy: serde_json::from_str("\"balanced\"").expect("policy"),
            status: serde_json::from_str("\"active\"").expect("status"),
        },
        LivingCreatorOperationV2::CreateDeposit {
            local: 5,
            world: existing(0x03),
            resource: serde_json::from_str("\"ore\"").expect("resource"),
            remaining_units: 20_000,
        },
        LivingCreatorOperationV2::CreateFacility {
            local: 6,
            world: existing(0x03),
            definition: serde_json::from_str("\"foundry\"").expect("slug"),
            status: serde_json::from_str("\"operational\"").expect("status"),
            paid_alloy: 40,
            hit_points: 0,
            next_due: Some(LivingTickV2(20)),
        },
        LivingCreatorOperationV2::CreateConstructionJob {
            local: 7,
            world: existing(0x03),
            owner: existing(0x04),
            definition: serde_json::from_str("\"solar_array\"").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(100),
            paid_alloy: 20,
        },
        LivingCreatorOperationV2::CreateHullJob {
            local: 8,
            world: existing(0x03),
            owner: existing(0x04),
            definition: serde_json::from_str("\"escort\"").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(100),
            paid_alloy: 20,
            target_fleet: None,
        },
        LivingCreatorOperationV2::CreateFleet {
            local: 9,
            owner: existing(0x04),
            world: existing(0x03),
        },
        LivingCreatorOperationV2::CreateHull {
            local: 10,
            fleet: LivingCommandTargetV2::Local { local: 9 },
            definition: serde_json::from_str("\"scout\"").expect("slug"),
            hit_points: 10,
            return_credit: false,
        },
        LivingCreatorOperationV2::CreateRoute {
            local: 11,
            kind: serde_json::from_str("\"internal\"").expect("route kind"),
            source_world: existing(0x03),
            destination_world: existing(0x05),
            resource: serde_json::from_str("\"alloy\"").expect("resource"),
            batch_size: 10,
            cadence_ticks: 50,
            source_reserve: 20,
            source_owner: existing(0x04),
            receiver_owner: existing(0x04),
            next_due: LivingTickV2(50),
        },
        LivingCreatorOperationV2::CreateShipment {
            local: 12,
            dispatch_owner: existing(0x04),
            intended_receiver: existing(0x04),
            source_world: existing(0x03),
            destination_world: existing(0x05),
            resource: serde_json::from_str("\"alloy\"").expect("resource"),
            units: 10,
            departure_tick: LivingTickV2(50),
            arrival_tick: LivingTickV2(61),
            disposition: serde_json::from_str("\"outbound\"").expect("disposition"),
        },
        LivingCreatorOperationV2::CreateHazard {
            local: 13,
            definition: serde_json::from_str("\"ion_storm\"").expect("slug"),
            lane: existing(0x06),
            start_tick: LivingTickV2(100),
            end_tick: LivingTickV2(200),
        },
    ];

    let command = LivingCommandV2::CreatorBatch { operations };
    command.validate().expect("the survey batch validates");

    let kinds: Vec<LivingEntityKindV2> = command
        .operations()
        .iter()
        .filter_map(LivingCreatorOperationV2::created_kind)
        .collect();
    let ordinals: Vec<u16> = kinds.iter().map(|kind| kind.ordinal()).collect();
    assert_eq!(
        ordinals,
        (1..=14).collect::<Vec<u16>>(),
        "one creating operation per published kind, in registry order"
    );

    // The identities are distinct because the kind is part of the hash, even
    // where two operations share a batch-local identifier space position.
    let revision = LivingRevisionIdV2([0x7a; 32]);
    let created = command.created_entities(revision);
    let mut identities: Vec<LivingEntityIdV2> = created.iter().map(|(_, _, id)| *id).collect();
    identities.sort_unstable();
    identities.dedup();
    assert_eq!(identities.len(), 14);
}

// -------------------------------------------------- structural rejections

#[test]
fn a_batch_over_the_declared_bound_is_refused() {
    let operations: Vec<LivingCreatorOperationV2> = (0..=LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2)
        .map(|index| system(u16::try_from(index).expect("small index"), "Vale"))
        .collect();
    let over = LivingCommandV2::CreatorBatch { operations };
    assert_eq!(
        over.validate(),
        Err(LivingCommandErrorV2::BatchTooLarge {
            operations: LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2 + 1,
            limit: LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2,
        })
    );

    // Exactly at the bound is accepted, so the check is `>` and not `>=`.
    let operations: Vec<LivingCreatorOperationV2> = (0..LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2)
        .map(|index| system(u16::try_from(index).expect("small index"), "Vale"))
        .collect();
    LivingCommandV2::CreatorBatch { operations }
        .validate()
        .expect("a full batch validates");
}

#[test]
fn batch_local_identifiers_must_start_at_zero_and_run_consecutively() {
    for (operations, expected, found) in [
        (vec![system(1, "Vale")], 0_u16, 1_u16),
        (vec![system(0, "Vale"), system(0, "Confluence")], 1, 0),
        (vec![system(0, "Vale"), system(2, "Confluence")], 1, 2),
    ] {
        assert_eq!(
            LivingCommandV2::CreatorBatch { operations }.validate(),
            Err(LivingCommandErrorV2::LocalIdentifierOutOfOrder { expected, found })
        );
    }
}

/// A reference may only reach backwards. A forward reference and an undeclared
/// one are distinct refusals, because one is a reordering and the other is a
/// typo.
#[test]
fn a_reference_may_only_reach_an_earlier_operation() {
    let forward = LivingCommandV2::CreatorBatch {
        operations: vec![
            LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: LivingCommandTargetV2::Local { local: 1 },
                archetype: serde_json::from_str("\"main_sequence\"").expect("slug"),
                name: name("Vale Prime"),
            },
            system(1, "Vale"),
        ],
    };
    assert_eq!(
        forward.validate(),
        Err(LivingCommandErrorV2::ForwardLocalReference { local: 1 })
    );

    let unknown = LivingCommandV2::CreatorBatch {
        operations: vec![LivingCreatorOperationV2::CreateStar {
            local: 0,
            system: LivingCommandTargetV2::Local { local: 9 },
            archetype: serde_json::from_str("\"main_sequence\"").expect("slug"),
            name: name("Vale Prime"),
        }],
    };
    assert_eq!(
        unknown.validate(),
        Err(LivingCommandErrorV2::UnknownLocalReference { local: 9 })
    );
}

/// An operation may not reference itself, even though its own identifier is
/// "declared" by that operation: the identity does not exist until the
/// operation applies.
#[test]
fn an_operation_may_not_reference_its_own_local_identifier() {
    let recursive = LivingCommandV2::CreatorBatch {
        operations: vec![LivingCreatorOperationV2::CreateStar {
            local: 0,
            system: LivingCommandTargetV2::Local { local: 0 },
            archetype: serde_json::from_str("\"main_sequence\"").expect("slug"),
            name: name("Vale Prime"),
        }],
    };
    assert_eq!(
        recursive.validate(),
        Err(LivingCommandErrorV2::ForwardLocalReference { local: 0 })
    );
}

/// Every reference-bearing field must reach validation. A field the walk
/// forgets is a hole an invalid reference travels through, and the only way to
/// see it is to point each field at an undeclared local in turn.
#[test]
fn every_reference_bearing_field_is_validated() {
    let missing = LivingCommandTargetV2::Local { local: 7 };
    let operations = vec![
        LivingCreatorOperationV2::SetWorldOwner {
            world: existing(0x01),
            owner: Some(missing),
        },
        LivingCreatorOperationV2::SetRelationBase {
            from: existing(0x01),
            to: missing,
            base_disposition: 10,
        },
        LivingCreatorOperationV2::SetFleetOrder {
            fleet: existing(0x01),
            kind: serde_json::from_str("\"move\"").expect("order kind"),
            target_world: None,
            not_before_tick: LivingTickV2(5),
            lane_path: vec![existing(0x02), missing],
        },
        LivingCreatorOperationV2::ForceWar {
            participant_a: existing(0x01),
            participant_b: existing(0x02),
            declarer: missing,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(600),
        },
        LivingCreatorOperationV2::RemoveEntity {
            target: existing(0x01),
            cascade: LivingCascadeDispositionV2::RemoveListed {
                dependents: vec![missing],
            },
        },
        LivingCreatorOperationV2::CreateHullJob {
            local: 0,
            world: existing(0x01),
            owner: existing(0x02),
            definition: serde_json::from_str("\"escort\"").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(100),
            paid_alloy: 20,
            target_fleet: Some(missing),
        },
    ];

    for operation in operations {
        let command = LivingCommandV2::CreatorBatch {
            operations: vec![operation.clone()],
        };
        assert_eq!(
            command.validate(),
            Err(LivingCommandErrorV2::UnknownLocalReference { local: 7 }),
            "this operation's reference never reached validation: {operation:?}"
        );
    }
}

#[test]
fn a_cascade_must_name_its_dependents_exactly_once() {
    let empty = LivingCommandV2::CreatorBatch {
        operations: vec![LivingCreatorOperationV2::RemoveEntity {
            target: existing(0x01),
            cascade: LivingCascadeDispositionV2::RemoveListed { dependents: vec![] },
        }],
    };
    assert_eq!(empty.validate(), Err(LivingCommandErrorV2::EmptyCascade));

    let repeated = LivingCommandV2::CreatorBatch {
        operations: vec![LivingCreatorOperationV2::RemoveEntity {
            target: existing(0x01),
            cascade: LivingCascadeDispositionV2::RemoveListed {
                dependents: vec![existing(0x02), existing(0x02)],
            },
        }],
    };
    assert_eq!(
        repeated.validate(),
        Err(LivingCommandErrorV2::RepeatedCascadeDependent)
    );

    let reviewed = LivingCommandV2::CreatorBatch {
        operations: vec![LivingCreatorOperationV2::RemoveEntity {
            target: existing(0x01),
            cascade: LivingCascadeDispositionV2::RemoveListed {
                dependents: vec![existing(0x02), existing(0x03)],
            },
        }],
    };
    reviewed.validate().expect("a reviewed cascade validates");
}

/// An invalid batch must never reach an identity: section 1 requires the
/// refusal before any entity identity is consumed.
#[test]
fn an_invalid_batch_is_refused_before_it_is_hashed() {
    let mut accepted = reviewed_accepted();
    accepted.envelope.command = LivingCommandV2::CreatorBatch {
        operations: vec![system(3, "Vale")],
    };
    assert_eq!(
        LivingRevisionV2::seal(LivingCatalogHashV2([0; 32]), SEED, None, &accepted),
        Err(LivingCommandErrorV2::LocalIdentifierOutOfOrder {
            expected: 0,
            found: 3
        })
    );
}

// --------------------------------------------------------------- wire rules

#[test]
fn commands_round_trip_through_the_canonical_wire() {
    let bytes = reviewed_batch().canonical_bytes().expect("encodes");
    let decoded: LivingCommandV2 =
        decode_canonical_v2(&bytes, LIVING_MAX_ARCHIVE_BYTES_V2).expect("decodes");
    assert_eq!(decoded, reviewed_batch());
}

#[test]
fn commands_reject_unknown_variants_reordered_and_extra_fields() {
    let unknown_variant =
        br#"{"type":"creator_batch","operations":[{"type":"create_galaxy","local":0}]}"#;
    assert_eq!(
        decode_canonical_v2::<LivingCommandV2>(unknown_variant, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    );

    let extra_field = br#"{"type":"creator_batch","operations":[{"type":"create_system","local":0,"name":"Vale","colour":"blue"}]}"#;
    assert_eq!(
        decode_canonical_v2::<LivingCommandV2>(extra_field, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    );

    let reordered =
        br#"{"type":"creator_batch","operations":[{"type":"create_system","name":"Vale","local":0}]}"#;
    assert_eq!(
        decode_canonical_v2::<LivingCommandV2>(reordered, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );

    let omitted_optional = br#"{"type":"creator_batch","operations":[{"type":"create_hull_job","local":0,"world":{"type":"existing","id":"01010101010101010101010101010101"},"owner":{"type":"existing","id":"02020202020202020202020202020202"},"definition":"escort","accepted_tick":0,"completion_tick":100,"paid_alloy":20}]}"#;
    assert_eq!(
        decode_canonical_v2::<LivingCommandV2>(omitted_optional, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical),
        "an optional value is encoded as null, never omitted; serde reads a missing \
         option as absent, so the exact byte comparison is what refuses this one"
    );
}

/// The envelope carries the three expectations section 2 requires, and a
/// document missing one of them is not an envelope.
#[test]
fn an_envelope_without_all_three_expectations_is_refused() {
    let complete = br#"{"expected_committed_revision":null,"expected_tick":6,"expected_pending_sequence":0,"mode":"running","command":{"type":"creator_batch","operations":[]}}"#;
    let envelope: LivingCommandEnvelopeV2 =
        decode_canonical_v2(complete, LIVING_MAX_ARCHIVE_BYTES_V2).expect("the envelope decodes");
    assert_eq!(envelope.expected_tick, LivingTickV2(6));
    assert_eq!(envelope.expected_pending_sequence, 0);
    assert_eq!(envelope.mode, LivingCommandModeV2::Running);

    let missing_tail = br#"{"expected_committed_revision":null,"expected_tick":6,"mode":"running","command":{"type":"creator_batch","operations":[]}}"#;
    assert_eq!(
        decode_canonical_v2::<LivingCommandEnvelopeV2>(missing_tail, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    );
}

/// Two envelopes that differ only in their accepted sequence, or only in their
/// application boundary, are different revisions. Both are framed inputs of the
/// identity, and section 2 says their meanings never change after replay.
#[test]
fn the_sequence_and_the_boundary_both_reach_the_revision_identity() {
    let catalog = LivingCatalogHashV2([0x5a; 32]);
    let base = reviewed_accepted();
    let sealed = LivingRevisionV2::seal(catalog, SEED, None, &base).expect("seals");

    let mut other_sequence = reviewed_accepted();
    other_sequence.accepted_sequence += 1;
    let mut other_tick = reviewed_accepted();
    other_tick.application_tick = LivingTickV2(8);
    let mut other_parent = reviewed_accepted();
    other_parent.envelope.expected_tick = LivingTickV2(7);

    for (label, accepted, parent) in [
        ("sequence", other_sequence, None),
        ("boundary", other_tick, None),
        ("parent", other_parent, Some(LivingRevisionIdV2([0x99; 32]))),
    ] {
        let changed = LivingRevisionV2::seal(catalog, SEED, parent, &accepted).expect("seals");
        assert_ne!(
            changed.id, sealed.id,
            "changing the {label} must change the revision identity"
        );
    }
}
