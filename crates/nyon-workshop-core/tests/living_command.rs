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
    LivingAgreementKindV2, LivingCascadeDispositionV2, LivingCatalogHashV2,
    LivingCivilizationStatusV2, LivingCommandEnvelopeV2, LivingCommandErrorV2, LivingCommandModeV2,
    LivingCommandTargetV2, LivingCommandV2, LivingCreatorOperationV2, LivingEntityIdV2,
    LivingEntityKindV2, LivingFacilityStatusV2, LivingFleetOrderKindV2, LivingInventoryV2,
    LivingNameV2, LivingPolicyV2, LivingResourceV2, LivingRevisionIdV2, LivingRevisionV2,
    LivingRouteKindV2, LivingShipmentDispositionV2, LivingSlugV2, LivingTickV2, LivingWireErrorV2,
    decode_canonical_v2,
};
use serde_json::{Map, Value};

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

fn slug(text: &str) -> LivingSlugV2 {
    LivingSlugV2::new(text).expect("a catalog identifier")
}

fn inventory() -> LivingInventoryV2 {
    LivingInventoryV2 {
        energy: 4,
        ore: 5,
        alloy: 6,
    }
}

/// `living/command.rs`'s own text, so the reference-field contract below is
/// derived from the declaration rather than restated beside it.
const COMMAND_SOURCE: &str = include_str!("../src/living/command.rs");

/// The batch-local identifier no template declares, so pointing any field at
/// it must be refused.
const MISSING_LOCAL: u16 = 7;

/// One field of one declared variant whose type carries a
/// [`LivingCommandTargetV2`], and therefore must reach the reference walk.
#[derive(Debug)]
struct ReferenceField {
    variant: String,
    field: String,
    plural: bool,
}

/// Is this the name of an enum variant, rather than a doc comment, an
/// attribute, or a closing brace at the same indentation?
fn is_variant_name(text: &str) -> bool {
    text.starts_with(|character: char| character.is_ascii_uppercase())
        && text
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
}

/// Read `living/command.rs` and return the operation variants it declares and
/// every reference-bearing field of every enum in the file.
///
/// This is the part that makes the coverage test below total instead of a
/// sample. A twenty-ninth operation, or one more `LivingCommandTargetV2` field
/// on an existing operation, appears here without anyone remembering to say so,
/// and fails the assertions that consume it until the templates are extended.
fn scan_command_schema() -> (Vec<String>, Vec<ReferenceField>) {
    let mut operations = Vec::new();
    let mut fields = Vec::new();
    let mut current_enum: Option<&str> = None;
    let mut current_variant: Option<String> = None;

    for line in COMMAND_SOURCE.lines() {
        if let Some(rest) = line.strip_prefix("pub enum ") {
            current_enum = Some(rest.trim_end_matches(" {"));
            current_variant = None;
            continue;
        }
        if line == "}" {
            current_enum = None;
            current_variant = None;
            continue;
        }
        let Some(enum_name) = current_enum else {
            continue;
        };

        // A variant header sits at one level of indentation, a field at two.
        if let Some(rest) = line.strip_prefix("    ")
            && !rest.starts_with(' ')
        {
            let candidate = rest.trim_end_matches(" {").trim_end_matches(',');
            if is_variant_name(candidate) {
                if enum_name == "LivingCreatorOperationV2" {
                    operations.push(candidate.to_owned());
                }
                current_variant = Some(candidate.to_owned());
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("        ")
            && !rest.starts_with(' ')
            && let Some((field, declared)) = rest.split_once(": ")
            && !field.is_empty()
            && field
                .chars()
                .all(|character| character.is_ascii_lowercase() || character == '_')
            && declared.contains("LivingCommandTargetV2")
        {
            let variant = current_variant
                .clone()
                .unwrap_or_else(|| panic!("{enum_name}.{field} sits outside any variant"));
            fields.push(ReferenceField {
                variant,
                field: field.to_owned(),
                plural: declared.starts_with("Vec<"),
            });
        }
    }

    (operations, fields)
}

/// The serde tag a variant is written under on the wire.
fn snake_case(variant: &str) -> String {
    let mut out = String::new();
    for (index, character) in variant.char_indices() {
        if character.is_ascii_uppercase() {
            if index != 0 {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
        } else {
            out.push(character);
        }
    }
    out
}

/// The object in `document` tagged `"type": tag`, at any depth.
fn find_tagged<'a>(document: &'a Value, tag: &str) -> Option<&'a Map<String, Value>> {
    match document {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some(tag) {
                return Some(object);
            }
            object.values().find_map(|value| find_tagged(value, tag))
        }
        Value::Array(items) => items.iter().find_map(|item| find_tagged(item, tag)),
        _ => None,
    }
}

/// The same search, for the one edit each case makes.
fn find_tagged_mut<'a>(document: &'a mut Value, tag: &str) -> Option<&'a mut Map<String, Value>> {
    match document {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some(tag) {
                return Some(object);
            }
            object
                .values_mut()
                .find_map(|value| find_tagged_mut(value, tag))
        }
        Value::Array(items) => items.iter_mut().find_map(|item| find_tagged_mut(item, tag)),
        _ => None,
    }
}

/// How many batch-local references the document carries, at any depth.
///
/// Each case below asserts this is zero before its edit and one after, so a
/// template that already pointed somewhere local could not make a case pass for
/// a field the walk never reads.
fn count_local_targets(document: &Value) -> usize {
    match document {
        Value::Object(object) => {
            let here = usize::from(object.get("type").and_then(Value::as_str) == Some("local"));
            here + object.values().map(count_local_targets).sum::<usize>()
        }
        Value::Array(items) => items.iter().map(count_local_targets).sum(),
        _ => 0,
    }
}

/// One instance of every declared operation, each valid on its own and each
/// pointing every reference-bearing field it has at an existing entity.
///
/// These are Rust values rather than JSON literals on purpose: a field added to
/// a variant fails to compile here, which is a stronger signal than a test
/// failure and arrives sooner.
fn every_operation_template() -> Vec<LivingCreatorOperationV2> {
    vec![
        LivingCreatorOperationV2::CreateSystem {
            local: 0,
            name: name("Vale"),
        },
        LivingCreatorOperationV2::CreateStar {
            local: 0,
            system: existing(0x01),
            archetype: slug("main_sequence"),
            name: name("Vale Prime"),
        },
        LivingCreatorOperationV2::CreateWorld {
            local: 0,
            system: existing(0x01),
            archetype: slug("terrestrial"),
            name: name("Vale II"),
            owner: Some(existing(0x02)),
            inventory: inventory(),
        },
        LivingCreatorOperationV2::CreateLane {
            local: 0,
            system_a: existing(0x01),
            system_b: existing(0x02),
            distance_units: 101,
        },
        LivingCreatorOperationV2::CreateCivilization {
            local: 0,
            name: name("Concord"),
            policy: LivingPolicyV2(slug("balanced")),
            status: LivingCivilizationStatusV2::Active,
        },
        LivingCreatorOperationV2::CreateDeposit {
            local: 0,
            world: existing(0x01),
            resource: LivingResourceV2::Ore,
            remaining_units: 500,
        },
        LivingCreatorOperationV2::CreateFacility {
            local: 0,
            world: existing(0x01),
            definition: slug("refinery"),
            status: LivingFacilityStatusV2::Operational,
            paid_alloy: 20,
            hit_points: 100,
            next_due: Some(LivingTickV2(9)),
        },
        LivingCreatorOperationV2::CreateConstructionJob {
            local: 0,
            world: existing(0x01),
            owner: existing(0x02),
            definition: slug("refinery"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(100),
            paid_alloy: 20,
        },
        LivingCreatorOperationV2::CreateHullJob {
            local: 0,
            world: existing(0x01),
            owner: existing(0x02),
            definition: slug("escort"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(100),
            paid_alloy: 20,
            target_fleet: Some(existing(0x03)),
        },
        LivingCreatorOperationV2::CreateFleet {
            local: 0,
            owner: existing(0x01),
            world: existing(0x02),
        },
        LivingCreatorOperationV2::CreateHull {
            local: 0,
            fleet: existing(0x01),
            definition: slug("escort"),
            hit_points: 50,
            return_credit: false,
        },
        LivingCreatorOperationV2::CreateRoute {
            local: 0,
            kind: LivingRouteKindV2::Internal,
            source_world: existing(0x01),
            destination_world: existing(0x02),
            resource: LivingResourceV2::Ore,
            batch_size: 10,
            cadence_ticks: 5,
            source_reserve: 2,
            source_owner: existing(0x03),
            receiver_owner: existing(0x04),
            next_due: LivingTickV2(12),
        },
        LivingCreatorOperationV2::CreateShipment {
            local: 0,
            dispatch_owner: existing(0x01),
            intended_receiver: existing(0x02),
            source_world: existing(0x03),
            destination_world: existing(0x04),
            resource: LivingResourceV2::Alloy,
            units: 5,
            departure_tick: LivingTickV2(1),
            arrival_tick: LivingTickV2(9),
            disposition: LivingShipmentDispositionV2::Outbound,
        },
        LivingCreatorOperationV2::CreateHazard {
            local: 0,
            definition: slug("ion_storm"),
            lane: existing(0x01),
            start_tick: LivingTickV2(3),
            end_tick: LivingTickV2(11),
        },
        LivingCreatorOperationV2::SetWorldInventory {
            world: existing(0x01),
            inventory: inventory(),
        },
        LivingCreatorOperationV2::SetWorldOwner {
            world: existing(0x01),
            owner: Some(existing(0x02)),
        },
        LivingCreatorOperationV2::CreateColony {
            world: existing(0x01),
            owner: existing(0x02),
        },
        LivingCreatorOperationV2::RemoveColony {
            world: existing(0x01),
        },
        LivingCreatorOperationV2::SetDepositRemaining {
            deposit: existing(0x01),
            remaining_units: 7,
        },
        LivingCreatorOperationV2::SetCivilizationPolicy {
            civilization: existing(0x01),
            policy: LivingPolicyV2(slug("balanced")),
        },
        LivingCreatorOperationV2::SetRelationBase {
            from: existing(0x01),
            to: existing(0x02),
            base_disposition: 10,
        },
        LivingCreatorOperationV2::SetAgreement {
            kind: LivingAgreementKindV2::Trade,
            participant_a: existing(0x01),
            participant_b: existing(0x02),
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(600),
        },
        LivingCreatorOperationV2::RemoveAgreement {
            kind: LivingAgreementKindV2::Trade,
            participant_a: existing(0x01),
            participant_b: existing(0x02),
        },
        LivingCreatorOperationV2::ForceWar {
            participant_a: existing(0x01),
            participant_b: existing(0x02),
            declarer: existing(0x03),
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(600),
        },
        LivingCreatorOperationV2::ForcePeace {
            participant_a: existing(0x01),
            participant_b: existing(0x02),
        },
        LivingCreatorOperationV2::SetFleetOrder {
            fleet: existing(0x01),
            kind: Some(LivingFleetOrderKindV2::Move),
            target_world: Some(existing(0x02)),
            not_before_tick: LivingTickV2(5),
            lane_path: vec![existing(0x03)],
        },
        LivingCreatorOperationV2::SetShipmentDisposition {
            shipment: existing(0x01),
            disposition: LivingShipmentDispositionV2::Outbound,
        },
        LivingCreatorOperationV2::RemoveEntity {
            target: existing(0x01),
            cascade: LivingCascadeDispositionV2::RemoveListed {
                dependents: vec![existing(0x02)],
            },
        },
    ]
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

/// The reference-field contract is read out of `living/command.rs`, so it
/// cannot drift from the declaration without failing here.
///
/// The two counts are pins, not targets. If one moves, the schema moved:
/// re-read `scan_command_schema` against the declaration and extend
/// `every_operation_template`, rather than editing the number.
#[test]
fn the_command_schema_scan_finds_every_declared_reference_field() {
    let (operations, fields) = scan_command_schema();
    assert_eq!(
        operations.len(),
        28,
        "the declared operation count moved: {operations:?}"
    );
    assert_eq!(
        fields.len(),
        49,
        "the reference-bearing field count moved: {fields:?}"
    );

    // A scan that found nothing would satisfy every assertion below it, so
    // prove it observed both the operation enum and the nested cascade.
    assert!(
        fields
            .iter()
            .any(|field| field.variant == "RemoveListed" && field.field == "dependents"),
        "the scan must reach reference fields nested below an operation"
    );
    assert!(
        fields.iter().any(|field| field.plural),
        "the scan must distinguish a repeated reference field"
    );

    // One template per declared operation, matched by its wire tag rather than
    // by position, so a twenty-ninth operation fails here until it has one.
    let templates: Vec<Value> = every_operation_template()
        .iter()
        .map(|operation| serde_json::to_value(operation).expect("an operation serializes"))
        .collect();
    assert_eq!(templates.len(), operations.len());
    for operation in &operations {
        let tag = snake_case(operation);
        let carrying = templates
            .iter()
            .filter(|template| find_tagged(template, &tag).is_some())
            .count();
        assert_eq!(
            carrying, 1,
            "exactly one template must carry the {tag} operation, found {carrying}"
        );
    }
}

/// Every reference-bearing field must reach validation. A field the walk
/// forgets is a hole an invalid reference travels through, and the only way to
/// see it is to point each field at an undeclared local in turn.
///
/// The field list comes from the declaration itself
/// (`the_command_schema_scan_finds_every_declared_reference_field`), so this is
/// a total scan rather than a hand-enumerated sample: adding a reference field
/// without adding a walk arm fails here, and so does adding one the templates
/// do not populate.
#[test]
fn every_reference_bearing_field_is_validated() {
    let (_, fields) = scan_command_schema();
    let templates = every_operation_template();

    // Nothing below is evidence unless every template is otherwise acceptable,
    // because then a rejection could come from the template rather than from
    // the one field this case redirects.
    for template in &templates {
        let command = LivingCommandV2::CreatorBatch {
            operations: vec![template.clone()],
        };
        assert_eq!(
            command.validate(),
            Ok(()),
            "a template must be valid before its reference is redirected: {template:?}"
        );
    }

    let documents: Vec<Value> = templates
        .iter()
        .map(|operation| serde_json::to_value(operation).expect("an operation serializes"))
        .collect();

    for field in &fields {
        let ReferenceField {
            variant,
            field: name,
            plural,
        } = field;
        let tag = snake_case(variant);
        let mut carrying = documents
            .iter()
            .filter(|document| find_tagged(document, &tag).is_some())
            .cloned();
        let mut document = carrying
            .next()
            .unwrap_or_else(|| panic!("no template declares {tag}, so {tag}.{name} is untested"));
        assert!(
            carrying.next().is_none(),
            "{tag} appears in more than one template, so {tag}.{name} is ambiguous"
        );
        assert_eq!(
            count_local_targets(&document),
            0,
            "a template must start with no batch-local reference: {tag}.{name}"
        );

        let object = find_tagged_mut(&mut document, &tag).expect("the object just located");
        let populated = object
            .get(name)
            .unwrap_or_else(|| panic!("{tag} carries no {name} on the wire"));
        assert!(
            !populated.is_null(),
            "a template must populate {tag}.{name}, so the redirect replaces a real target"
        );
        assert_eq!(
            populated.is_array(),
            *plural,
            "{tag}.{name} is declared repeated but is not written as a list"
        );
        let missing = serde_json::json!({ "type": "local", "local": MISSING_LOCAL });
        let redirected = if *plural {
            Value::Array(vec![missing])
        } else {
            missing
        };
        object.insert(name.clone(), redirected);

        assert_eq!(
            count_local_targets(&document),
            1,
            "exactly one reference may be redirected per case: {tag}.{name}"
        );

        let operation: LivingCreatorOperationV2 = serde_json::from_value(document)
            .unwrap_or_else(|error| panic!("{tag}.{name} is not a wire field: {error}"));
        let command = LivingCommandV2::CreatorBatch {
            operations: vec![operation],
        };
        assert_eq!(
            command.validate(),
            Err(LivingCommandErrorV2::UnknownLocalReference {
                local: MISSING_LOCAL
            }),
            "this field's reference never reached validation: {tag}.{name}"
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

/// A revision's field order reaches archives, and its doc comment declaring it
/// normative pins nothing on its own. Decode a literal in the declared order,
/// check it is the reviewed revision, and refuse the same fields in any other
/// order.
#[test]
fn the_revision_field_order_is_pinned_against_literal_bytes() {
    const DECLARED: &[u8] = br#"{"id":"7a68db6c231f162d4ebee7c5595f78ec1eb34327d6d333cd5eaea5913ccb36c2","parent":null,"tick":7,"ordinal":3,"command":{"type":"creator_batch","operations":[{"type":"create_system","local":0,"name":"Vale"},{"type":"create_system","local":1,"name":"Confluence"},{"type":"create_lane","local":2,"system_a":{"type":"local","local":0},"system_b":{"type":"local","local":1},"distance_units":101}]}}"#;

    let revision: LivingRevisionV2 =
        decode_canonical_v2(DECLARED, LIVING_MAX_ARCHIVE_BYTES_V2).expect("the declared order");
    let corpus = vectors();
    let catalog = LivingCatalogHashV2(hex32(&corpus["creator_command"]["catalog_hash"]));
    assert_eq!(
        revision,
        LivingRevisionV2::seal(catalog, SEED, None, &reviewed_accepted()).expect("seals")
    );

    const REORDERED: &[u8] = br#"{"parent":null,"id":"7a68db6c231f162d4ebee7c5595f78ec1eb34327d6d333cd5eaea5913ccb36c2","tick":7,"ordinal":3,"command":{"type":"creator_batch","operations":[]}}"#;
    assert_eq!(
        decode_canonical_v2::<LivingRevisionV2>(REORDERED, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );
}

/// The accepted envelope's own field order is pinned the same way: the sequence
/// and the boundary precede the envelope they were allocated for.
#[test]
fn the_accepted_envelope_field_order_is_pinned_against_literal_bytes() {
    const DECLARED: &[u8] = br#"{"accepted_sequence":3,"application_tick":7,"envelope":{"expected_committed_revision":null,"expected_tick":6,"expected_pending_sequence":0,"mode":"running","command":{"type":"creator_batch","operations":[]}}}"#;
    let accepted: LivingAcceptedCommandV2 =
        decode_canonical_v2(DECLARED, LIVING_MAX_ARCHIVE_BYTES_V2).expect("the declared order");
    assert_eq!(accepted.accepted_sequence, 3);
    assert_eq!(accepted.application_tick, LivingTickV2(7));

    const REORDERED: &[u8] = br#"{"application_tick":7,"accepted_sequence":3,"envelope":{"expected_committed_revision":null,"expected_tick":6,"expected_pending_sequence":0,"mode":"running","command":{"type":"creator_batch","operations":[]}}}"#;
    assert_eq!(
        decode_canonical_v2::<LivingAcceptedCommandV2>(REORDERED, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );
}
