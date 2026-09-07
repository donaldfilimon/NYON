//! Reviewed exact-vector tests for the Living Galaxy V2 canonical wire.
//!
//! Every expected digest comes from `tests/fixtures/living-v2/vectors.json`,
//! which is derived from
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` section 10
//! by an independent SHA-256 computation, never from this crate's output. The
//! fixture is compiled in with `include_str!` so a relocated checkout cannot
//! resolve a stale absolute path.

use nyon_workshop_core::living::{
    LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2, LIVING_AUTO_ENTITY_DOMAIN_V2, LIVING_CLAIM_DOMAIN_V2,
    LIVING_CREATOR_ENTITY_DOMAIN_V2, LIVING_EVENT_DOMAIN_V2, LIVING_FORK_BRANCH_DOMAIN_V2,
    LIVING_MAX_CANONICAL_DEPTH_V2, LIVING_MAX_PACK_BYTES_V2, LIVING_PACK_DOMAIN_V2,
    LIVING_RECEIPT_DOMAIN_V2, LIVING_REVISION_DOMAIN_V2, LIVING_ROOT_BRANCH_DOMAIN_V2,
    LIVING_RULES_VERSION, LIVING_STATE_DOMAIN_V2, LIVING_TICK_HZ, LivingAutonomousEntityInputsV2,
    LivingBranchIdV2, LivingCatalogHashV2, LivingEntityIdV2, LivingEventIdV2, LivingKeyedV2,
    LivingReceiptDigestV2, LivingRevisionIdV2, LivingSortedVecV2, LivingStateDigestV2,
    LivingTickV2, LivingWireErrorV2, archive_integrity_v2, autonomous_entity_digest_v2,
    autonomous_entity_id_v2, catalog_hash_v2, claim_rank_v2, creator_entity_digest_v2,
    creator_entity_id_v2, decode_canonical_v2, encode_canonical_v2, event_digest_v2, event_id_v2,
    fork_branch_digest_v2, fork_branch_id_v2, receipt_digest_v2, revision_id_v2,
    root_branch_digest_v2, root_branch_id_v2, state_digest_v2,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const VECTORS: &str = include_str!("fixtures/living-v2/vectors.json");

const EMPTY_OBJECT: &[u8] = b"{}";
const SEED: [u8; 32] = [0x11; 32];

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("reviewed Living V2 vectors parse")
}

fn hex_bytes(value: &str) -> Vec<u8> {
    assert!(value.len().is_multiple_of(2), "hex vector has odd width");
    (0..value.len() / 2)
        .map(|index| {
            u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).expect("hex vector is valid")
        })
        .collect()
}

fn hex32(value: &Value) -> [u8; 32] {
    hex_bytes(value.as_str().expect("hex vector is a string"))
        .try_into()
        .expect("32-byte vector")
}

fn hex16(value: &Value) -> [u8; 16] {
    hex_bytes(value.as_str().expect("hex vector is a string"))
        .try_into()
        .expect("16-byte vector")
}

fn hex_string(value: &impl Serialize) -> String {
    let encoded = serde_json::to_string(value).expect("wrapper serializes");
    encoded.trim_matches('"').to_owned()
}

fn domain(vectors: &Value, name: &str) -> Value {
    vectors["domains"]
        .as_array()
        .expect("domain table")
        .iter()
        .find(|entry| entry["name"] == name)
        .cloned()
        .unwrap_or_else(|| panic!("domain {name} is present in the reviewed vectors"))
}

// ---------------------------------------------------------------- domain tags

#[test]
fn the_eleven_spec_domain_literals_are_frozen_byte_for_byte() {
    let vectors = vectors();
    let frozen: [(&str, &[u8]); 11] = [
        ("pack", LIVING_PACK_DOMAIN_V2),
        ("state", LIVING_STATE_DOMAIN_V2),
        ("archive_integrity", LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2),
        ("receipt", LIVING_RECEIPT_DOMAIN_V2),
        ("revision", LIVING_REVISION_DOMAIN_V2),
        ("creator_entity", LIVING_CREATOR_ENTITY_DOMAIN_V2),
        ("auto_entity", LIVING_AUTO_ENTITY_DOMAIN_V2),
        ("root_branch", LIVING_ROOT_BRANCH_DOMAIN_V2),
        ("fork_branch", LIVING_FORK_BRANCH_DOMAIN_V2),
        ("event", LIVING_EVENT_DOMAIN_V2),
        ("claim", LIVING_CLAIM_DOMAIN_V2),
    ];

    for (name, literal) in frozen {
        let entry = domain(&vectors, name);
        assert_eq!(
            literal,
            hex_bytes(entry["bytes_hex"].as_str().expect("domain bytes")),
            "domain {name} bytes are frozen including the terminating NUL"
        );
        assert_eq!(
            *literal.last().expect("domain literal is not empty"),
            0,
            "domain {name} ends with the terminating NUL"
        );
        assert_eq!(
            &literal[..literal.len() - 1],
            entry["literal"]
                .as_str()
                .expect("domain literal")
                .as_bytes(),
            "domain {name} is the exact ASCII literal"
        );
    }

    assert_eq!(LIVING_RULES_VERSION, 2);
    assert_eq!(LIVING_TICK_HZ, 10);
}

#[test]
fn payload_hash_formulas_match_the_published_spec_goldens() {
    let vectors = vectors();
    let payload = &vectors["payload_hashes"];
    assert_eq!(
        hex_bytes(payload["canonical_bytes_hex"].as_str().expect("payload")),
        EMPTY_OBJECT
    );

    assert_eq!(
        catalog_hash_v2(EMPTY_OBJECT).0,
        hex32(&payload["catalog_hash"])
    );
    assert_eq!(
        state_digest_v2(EMPTY_OBJECT).0,
        hex32(&payload["state_digest"])
    );
    assert_eq!(
        archive_integrity_v2(EMPTY_OBJECT),
        hex32(&payload["archive_integrity"])
    );
    assert_eq!(
        receipt_digest_v2(EMPTY_OBJECT).0,
        hex32(&payload["receipt_digest"])
    );

    // The three values the rules spec publishes inline, restated here so a
    // reviewer sees them without opening the fixture.
    assert_eq!(
        hex_string(&catalog_hash_v2(EMPTY_OBJECT)),
        "fb53ddfe7525550e3eda1e6eed938928e6350c7b6bb5b66ca16f3d74350e020c"
    );
    assert_eq!(
        hex_string(&state_digest_v2(EMPTY_OBJECT)),
        "23aea47d6aa816e31f3f5a65dfe7d6d9b6c8889cf6442826f030ab1800bfd683"
    );
    assert_eq!(
        hex_bytes("dcf8ed382ce041b714305622a8c800cc6ae19bb1753b6f9369e62a14871e2714"),
        archive_integrity_v2(EMPTY_OBJECT)
    );
}

// ------------------------------------------------------------------ revisions

#[test]
fn revision_framing_and_optional_parent_tags_match_reviewed_vectors() {
    let vectors = vectors();
    let rows = vectors["revisions"].as_array().expect("revision vectors");
    let root = &rows[0];
    let child = &rows[1];

    let catalog = LivingCatalogHashV2(hex32(&root["catalog_hash"]));
    let root_id = revision_id_v2(
        catalog,
        SEED,
        None,
        LivingTickV2(root["tick"].as_u64().expect("tick")),
        root["ordinal"].as_u64().expect("ordinal"),
        EMPTY_OBJECT,
    );
    assert!(root["parent"].is_null(), "the first vector has no parent");
    assert_eq!(root_id.0, hex32(&root["revision_id"]));

    let child_id = revision_id_v2(
        catalog,
        SEED,
        Some(root_id),
        LivingTickV2(child["tick"].as_u64().expect("tick")),
        child["ordinal"].as_u64().expect("ordinal"),
        EMPTY_OBJECT,
    );
    assert_eq!(hex32(&child["parent"]), root_id.0);
    assert_eq!(child_id.0, hex32(&child["revision_id"]));
    assert_ne!(root_id, child_id);

    // Absent and present optional tags are exactly 0x00 and 0x01 || value.
    let tag = &vectors["optional_tag"];
    assert_eq!(hex_bytes(tag["absent_hex"].as_str().expect("tag")), [0x00]);
    assert_eq!(
        hex_bytes(tag["present_prefix_hex"].as_str().expect("tag")),
        [0x01]
    );
    let present = hex_bytes(tag["present_example_hex"].as_str().expect("tag"));
    assert_eq!(present.len(), 33);
    assert_eq!(present[0], 0x01);
    assert_eq!(present[1..], root_id.0);
}

// ------------------------------------------------------------------ entities

#[test]
fn creator_entity_framing_matches_reviewed_vectors() {
    let vectors = vectors();
    let revision = LivingRevisionIdV2(hex32(&vectors["revisions"][0]["revision_id"]));

    for row in vectors["creator_entities"]
        .as_array()
        .expect("creator rows")
    {
        let kind = u16::try_from(row["entity_kind"].as_u64().expect("kind")).expect("u16 kind");
        let local = u16::try_from(row["batch_local_id"].as_u64().expect("local")).expect("u16 id");
        let digest = creator_entity_digest_v2(revision, kind, local);
        assert_eq!(digest, hex32(&row["entity_digest"]));
        let id = creator_entity_id_v2(revision, kind, local);
        assert_eq!(id.0, hex16(&row["entity_id"]));
        assert_eq!(id.0, digest[..16], "entity_id is the first 16 digest bytes");
    }
}

fn autonomous_inputs(catalog: LivingCatalogHashV2, row: &Value) -> LivingAutonomousEntityInputsV2 {
    LivingAutonomousEntityInputsV2 {
        catalog_hash: catalog,
        genesis_seed: SEED,
        branch: LivingBranchIdV2(hex16(&row["branch_id"])),
        tick: LivingTickV2(row["tick"].as_u64().expect("tick")),
        phase: u16::try_from(row["phase"].as_u64().expect("phase")).expect("u16 phase"),
        actor: LivingEntityIdV2(hex16(&row["actor_id"])),
        intent_ordinal: u16::try_from(row["intent_ordinal"].as_u64().expect("intent"))
            .expect("u16 intent"),
        entity_kind: u16::try_from(row["entity_kind"].as_u64().expect("kind")).expect("u16 kind"),
        local_id: u16::try_from(row["local_id"].as_u64().expect("local")).expect("u16 local"),
    }
}

#[test]
fn autonomous_entity_framing_including_a_shipment_matches_reviewed_vectors() {
    let vectors = vectors();
    let catalog = LivingCatalogHashV2(hex32(&vectors["payload_hashes"]["catalog_hash"]));
    let rows = vectors["autonomous_entities"]
        .as_array()
        .expect("autonomous rows");
    assert!(
        rows.iter()
            .any(|row| row["label"] == "route_shipment_phase_9"),
        "the corpus includes a shipment identity"
    );

    for row in rows {
        let inputs = autonomous_inputs(catalog, row);
        assert_eq!(
            autonomous_entity_digest_v2(&inputs),
            hex32(&row["entity_digest"])
        );
        assert_eq!(autonomous_entity_id_v2(&inputs).0, hex16(&row["entity_id"]));
    }
}

#[test]
fn identity_tuple_reuse_is_deterministic_and_every_field_is_load_bearing() {
    let vectors = vectors();
    let catalog = LivingCatalogHashV2(hex32(&vectors["payload_hashes"]["catalog_hash"]));
    let rows = vectors["autonomous_entities"]
        .as_array()
        .expect("autonomous rows");
    let base = autonomous_inputs(catalog, &rows[0]);

    // Reusing the identical tuple reproduces the identical identity.
    assert_eq!(
        autonomous_entity_id_v2(&base),
        autonomous_entity_id_v2(&autonomous_inputs(catalog, &rows[0]))
    );

    // The reviewed corpus pins that a different phase/kind and a bumped intent
    // ordinal are distinct identities from the same actor and boundary.
    let shipment = autonomous_inputs(catalog, &rows[1]);
    let next_intent = autonomous_inputs(catalog, &rows[2]);
    assert_ne!(
        autonomous_entity_id_v2(&base),
        autonomous_entity_id_v2(&shipment)
    );
    assert_ne!(
        autonomous_entity_id_v2(&base),
        autonomous_entity_id_v2(&next_intent)
    );
    assert_eq!(next_intent.intent_ordinal, base.intent_ordinal + 1);

    let mut perturbed = base;
    perturbed.local_id += 1;
    assert_ne!(
        autonomous_entity_id_v2(&base),
        autonomous_entity_id_v2(&perturbed)
    );

    let mut other_seed = base;
    other_seed.genesis_seed = [0x12; 32];
    assert_ne!(
        autonomous_entity_id_v2(&base),
        autonomous_entity_id_v2(&other_seed)
    );
}

// ------------------------------------------------------- branches and events

#[test]
fn root_and_fork_branch_framing_match_reviewed_vectors() {
    let vectors = vectors();
    let root = &vectors["branches"]["root"];
    let catalog = LivingCatalogHashV2(hex32(&root["catalog_hash"]));
    let manifest = LivingStateDigestV2(hex32(&root["genesis_manifest_digest"]));

    let root_digest = root_branch_digest_v2(catalog, SEED, manifest);
    assert_eq!(root_digest, hex32(&root["branch_digest"]));
    let root_id = root_branch_id_v2(catalog, SEED, manifest);
    assert_eq!(root_id.0, hex16(&root["branch_id"]));
    assert_eq!(root_id.0, root_digest[..16]);

    let fork = &vectors["branches"]["fork"];
    let fork_revision = LivingRevisionIdV2(hex32(&fork["fork_revision"]));
    let fork_tick = LivingTickV2(fork["fork_tick"].as_u64().expect("fork tick"));
    let ordinal = fork["branch_ordinal"].as_u64().expect("branch ordinal");
    let fork_digest = fork_branch_digest_v2(root_id, fork_revision, fork_tick, ordinal);
    assert_eq!(fork_digest, hex32(&fork["branch_digest"]));
    assert_eq!(
        fork_branch_id_v2(root_id, fork_revision, fork_tick, ordinal).0,
        hex16(&fork["branch_id"])
    );
    assert_ne!(
        root_id,
        fork_branch_id_v2(root_id, fork_revision, fork_tick, ordinal)
    );
}

#[test]
fn event_framing_matches_reviewed_vectors() {
    let vectors = vectors();
    for row in vectors["events"].as_array().expect("event rows") {
        let receipt = LivingReceiptDigestV2(hex32(&row["receipt_digest"]));
        let ordinal = u16::try_from(row["event_ordinal"].as_u64().expect("ordinal")).expect("u16");
        assert_eq!(
            event_digest_v2(receipt, ordinal),
            hex32(&row["event_digest"])
        );
        let id: LivingEventIdV2 = event_id_v2(receipt, ordinal);
        assert_eq!(id.0, hex16(&row["event_id"]));
    }
}

#[test]
fn claim_rank_framing_and_full_thirty_two_byte_ordering_match_reviewed_vectors() {
    let vectors = vectors();
    let rows = vectors["claims"].as_array().expect("claim rows");
    let mut ranked = Vec::new();
    for row in rows {
        let rank = claim_rank_v2(
            SEED,
            LivingEntityIdV2(hex16(&row["world_id"])),
            LivingTickV2(row["claim_close_tick"].as_u64().expect("close tick")),
            LivingEntityIdV2(hex16(&row["civilization_id"])),
        );
        assert_eq!(rank, hex32(&row["claim_rank"]));
        ranked.push((rank, row["label"].as_str().expect("label").to_owned()));
    }
    ranked.sort();
    assert_eq!(
        ranked[0].1,
        vectors["claim_winner"].as_str().expect("winner"),
        "arbitration chooses the lexicographically smallest full 32-byte rank"
    );
}

// ---------------------------------------------------------- wrapper contract

#[test]
fn fixed_width_v2_wrappers_render_lowercase_hex_at_their_declared_widths() {
    let bytes32 = [0xab_u8; 32];
    let bytes16 = [0xcd_u8; 16];
    let hex64 = "ab".repeat(32);
    let hex32_text = "cd".repeat(16);

    assert_eq!(hex_string(&LivingRevisionIdV2(bytes32)), hex64);
    assert_eq!(hex_string(&LivingCatalogHashV2(bytes32)), hex64);
    assert_eq!(hex_string(&LivingStateDigestV2(bytes32)), hex64);
    assert_eq!(hex_string(&LivingReceiptDigestV2(bytes32)), hex64);
    assert_eq!(hex_string(&LivingEntityIdV2(bytes16)), hex32_text);
    assert_eq!(hex_string(&LivingBranchIdV2(bytes16)), hex32_text);
    assert_eq!(hex_string(&LivingEventIdV2(bytes16)), hex32_text);

    // Ticks are base-10 integers, not hex: V2 canonical numbers are base 10.
    assert_eq!(
        serde_json::to_string(&LivingTickV2(150)).expect("tick serializes"),
        "150"
    );
    assert_eq!(
        serde_json::from_str::<LivingTickV2>("150").expect("tick parses"),
        LivingTickV2(150)
    );
}

#[test]
fn v2_wrapper_hex_rejects_uppercase_short_long_and_non_hex_text() {
    for text in [
        "\"AB00000000000000000000000000000000000000000000000000000000000000\"",
        "\"ab0000000000000000000000000000000000000000000000000000000000000\"",
        "\"ab000000000000000000000000000000000000000000000000000000000000000\"",
        "\"zz00000000000000000000000000000000000000000000000000000000000000\"",
        "\"cd00000000000000000000000000000\"",
    ] {
        assert!(
            serde_json::from_str::<LivingRevisionIdV2>(text).is_err(),
            "{text} must not decode as a 32-byte V2 identity"
        );
    }
    assert!(
        serde_json::from_str::<LivingEntityIdV2>("\"CD00000000000000000000000000000000\"").is_err()
    );
    assert!(
        serde_json::from_str::<LivingEntityIdV2>("\"cd000000000000000000000000000000\"").is_ok()
    );
}

#[test]
fn living_v2_sources_declare_no_conversion_to_or_from_workshop_v1() {
    let living = [
        ("living/mod.rs", include_str!("../src/living/mod.rs")),
        ("living/ids.rs", include_str!("../src/living/ids.rs")),
        ("living/wire.rs", include_str!("../src/living/wire.rs")),
    ];
    for (name, source) in living {
        for forbidden in [
            "crate::ids",
            "super::ids",
            "crate::model",
            "crate::command",
            "crate::history",
            "WorkshopTick",
            "WORKSHOP_RULES_VERSION",
            "impl From<",
            "impl Into<",
            "unsafe",
        ] {
            assert!(
                !source.contains(forbidden),
                "{name} must not reference the WorkshopV1 identity surface: {forbidden}"
            );
        }
    }
    let v1_ids = include_str!("../src/ids.rs");
    assert!(
        !v1_ids.contains("Living"),
        "the WorkshopV1 identity module must not know about Living V2"
    );
}

// ------------------------------------------------------- canonical JSON wire

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct WireProbe {
    kind: String,
    format_version: u32,
    tick: LivingTickV2,
    parent: Option<LivingRevisionIdV2>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct KeyedRow {
    id: u32,
    name: String,
}

impl LivingKeyedV2 for KeyedRow {
    type Key = u32;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

const PROBE: &[u8] = br#"{"kind":"probe","format_version":2,"tick":7,"parent":null}"#;

#[test]
fn canonical_decode_round_trips_the_exact_input_bytes() {
    let decoded: WireProbe = decode_canonical_v2(PROBE, LIVING_MAX_PACK_BYTES_V2).expect("decodes");
    assert_eq!(decoded.tick, LivingTickV2(7));
    assert_eq!(decoded.parent, None);
    let encoded = encode_canonical_v2(&decoded, LIVING_MAX_PACK_BYTES_V2).expect("encodes");
    assert_eq!(encoded, PROBE);
}

#[test]
fn canonical_decode_rejects_unknown_duplicate_and_reordered_fields() {
    let unknown = br#"{"kind":"probe","format_version":2,"tick":7,"parent":null,"extra":1}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(unknown, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    ));

    let duplicate = br#"{"kind":"probe","kind":"probe","format_version":2,"tick":7,"parent":null}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(duplicate, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    ));

    let reordered = br#"{"format_version":2,"kind":"probe","tick":7,"parent":null}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(reordered, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    ));
}

#[test]
fn canonical_decode_rejects_a_byte_order_mark_and_invalid_utf8() {
    let mut with_bom = vec![0xef, 0xbb, 0xbf];
    with_bom.extend_from_slice(PROBE);
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(&with_bom, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::ByteOrderMark)
    ));

    let invalid = [b'{', b'"', 0xff, 0xfe, b'"', b':', b'1', b'}'];
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(&invalid, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::Utf8)
    ));
}

#[test]
fn canonical_decode_rejects_noncanonical_escapes_and_insignificant_whitespace() {
    // The kind starts with an escaped `p` (\u0070): it parses, but re-encoding
    // emits the plain character, so the input bytes are not canonical.
    let escaped = br#"{"kind":"\u0070robe","format_version":2,"tick":7,"parent":null}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(escaped, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    ));

    let spaced = br#"{"kind": "probe", "format_version": 2, "tick": 7, "parent": null}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(spaced, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    ));

    let newline = b"{\"kind\":\"probe\",\"format_version\":2,\"tick\":7,\"parent\":null}\n";
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(newline, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    ));
}

#[test]
fn canonical_decode_rejects_floating_point_values() {
    for bytes in [
        br#"{"a":1.0}"#.as_slice(),
        br#"{"a":1e3}"#.as_slice(),
        br#"{"a":-2.5}"#.as_slice(),
    ] {
        assert!(
            matches!(
                decode_canonical_v2::<Value>(bytes, LIVING_MAX_PACK_BYTES_V2),
                Err(LivingWireErrorV2::Float)
            ),
            "V2 canonical data contains no floating-point values"
        );
    }
    assert!(decode_canonical_v2::<Value>(br#"{"a":-25}"#, LIVING_MAX_PACK_BYTES_V2).is_ok());
}

#[test]
fn canonical_decode_accepts_depth_thirty_two_and_rejects_depth_thirty_three() {
    let nest = |depth: usize| {
        let mut text = String::new();
        for _ in 0..depth {
            text.push('[');
        }
        text.push('0');
        for _ in 0..depth {
            text.push(']');
        }
        text
    };
    let limit = usize::try_from(LIVING_MAX_CANONICAL_DEPTH_V2).expect("depth fits usize");
    assert_eq!(limit, 32);

    let accepted = nest(limit);
    assert!(decode_canonical_v2::<Value>(accepted.as_bytes(), LIVING_MAX_PACK_BYTES_V2).is_ok());

    let rejected = nest(limit + 1);
    assert!(matches!(
        decode_canonical_v2::<Value>(rejected.as_bytes(), LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::DepthExceeded { limit: 32 })
    ));
}

#[test]
fn canonical_decode_rejects_unsorted_and_duplicate_keyed_arrays() {
    let sorted = br#"[{"id":1,"name":"a"},{"id":2,"name":"b"}]"#;
    let rows: LivingSortedVecV2<KeyedRow> =
        decode_canonical_v2(sorted, LIVING_MAX_PACK_BYTES_V2).expect("sorted rows decode");
    assert_eq!(rows.as_slice().len(), 2);
    assert_eq!(
        encode_canonical_v2(&rows, LIVING_MAX_PACK_BYTES_V2).expect("re-encodes"),
        sorted
    );

    let unsorted = br#"[{"id":2,"name":"b"},{"id":1,"name":"a"}]"#;
    assert!(matches!(
        decode_canonical_v2::<LivingSortedVecV2<KeyedRow>>(unsorted, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    ));

    let duplicated = br#"[{"id":1,"name":"a"},{"id":1,"name":"b"}]"#;
    assert!(matches!(
        decode_canonical_v2::<LivingSortedVecV2<KeyedRow>>(duplicated, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    ));
}

#[test]
fn canonical_decode_rejects_oversize_input_before_parsing() {
    let mut oversize = b"{".to_vec();
    oversize.resize(64, b' ');
    match decode_canonical_v2::<Value>(&oversize, 16) {
        Err(LivingWireErrorV2::TooLarge { actual, limit }) => {
            assert_eq!(actual, 64);
            assert_eq!(limit, 16);
        }
        other => panic!("oversize input must reject before parsing, got {other:?}"),
    }
    assert!(encode_canonical_v2(&LivingTickV2(1_000_000), 3).is_err());
}

#[test]
fn optional_values_are_explicit_null_and_never_omitted() {
    let omitted = br#"{"kind":"probe","format_version":2,"tick":7}"#;
    assert!(matches!(
        decode_canonical_v2::<WireProbe>(omitted, LIVING_MAX_PACK_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    ));

    let revision = LivingRevisionIdV2([0x01; 32]);
    let present = WireProbe {
        kind: "probe".to_owned(),
        format_version: 2,
        tick: LivingTickV2(7),
        parent: Some(revision),
    };
    let bytes = encode_canonical_v2(&present, LIVING_MAX_PACK_BYTES_V2).expect("encodes");
    let decoded: WireProbe =
        decode_canonical_v2(&bytes, LIVING_MAX_PACK_BYTES_V2).expect("round trips");
    assert_eq!(decoded, present);
    assert!(
        String::from_utf8(bytes)
            .expect("utf8")
            .contains(&"01".repeat(32))
    );
}
