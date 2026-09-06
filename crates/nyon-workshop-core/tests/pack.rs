mod common;

use nyon_workshop_core::{
    PackValidationError, decode_catalog_pack, encode_catalog_pack, pack::MAX_PACK_BYTES,
};

#[test]
fn core_pack_is_strict_canonical_and_round_trips() {
    let first = decode_catalog_pack(common::CORE_PACK).unwrap();
    assert_eq!(
        first.catalog_hash().0,
        [
            17, 230, 139, 74, 75, 41, 15, 107, 241, 65, 38, 248, 204, 243, 55, 142, 30, 195, 50,
            150, 44, 105, 125, 111, 149, 142, 118, 254, 211, 239, 72, 66,
        ]
    );
    assert_eq!(first.pack_id(), "nyon.core");
    assert_eq!(first.resource_ids().count(), 3);
    let canonical = encode_catalog_pack(&first).unwrap();
    let second = decode_catalog_pack(&canonical).unwrap();
    assert_eq!(first.catalog_hash(), second.catalog_hash());
    assert_eq!(canonical, encode_catalog_pack(&second).unwrap());
}

#[test]
fn validated_display_metadata_is_read_only_and_preserves_canonical_identity() {
    let pack = decode_catalog_pack(common::CORE_PACK).unwrap();
    let canonical = encode_catalog_pack(&pack).unwrap();
    let hash = pack.catalog_hash();

    assert_eq!(
        pack.star_archetype_metadata(&common::catalog("yellow-dwarf")),
        Some(("Yellow Dwarf", "A stable main-sequence star."))
    );
    assert_eq!(
        pack.world_archetype_metadata(&common::catalog("rocky-world")),
        Some(("Rocky World", "A solid world suitable for industry."))
    );
    assert_eq!(
        pack.resource_metadata(&common::catalog("alloy")),
        Some(("Alloy", "Refined structural material."))
    );
    let industry = pack.industry(&common::catalog("foundry")).unwrap();
    assert_eq!(industry.name(), "Foundry");
    assert_eq!(
        industry.description(),
        "Converts energy and ore into alloy."
    );
    let hazard = pack.hazard(&common::catalog("ion-storm")).unwrap();
    assert_eq!(hazard.name(), "Ion Storm");
    assert_eq!(
        hazard.description(),
        "Halves route capacity on its lane using integer floor division."
    );

    assert_eq!(pack.catalog_hash(), hash);
    assert_eq!(encode_catalog_pack(&pack).unwrap(), canonical);
    assert_eq!(
        decode_catalog_pack(&canonical).unwrap().catalog_hash(),
        hash
    );
}

#[test]
fn pack_hash_ignores_source_collection_order() {
    let mut value: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    value["resources"].as_array_mut().unwrap().reverse();
    value["industry_definitions"][2]["inputs"]
        .as_array_mut()
        .unwrap()
        .reverse();
    value["industry_definitions"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let reordered = serde_json::to_vec(&value).unwrap();
    let first = decode_catalog_pack(common::CORE_PACK).unwrap();
    let second = decode_catalog_pack(&reordered).unwrap();
    assert_eq!(first.catalog_hash(), second.catalog_hash());
    assert_eq!(
        encode_catalog_pack(&first).unwrap(),
        encode_catalog_pack(&second).unwrap()
    );
}

#[test]
fn pack_rejects_oversized_bom_invalid_utf8_trailing_and_depth() {
    let oversized = vec![b' '; MAX_PACK_BYTES + 1];
    assert_eq!(
        decode_catalog_pack(&oversized),
        Err(PackValidationError::PackTooLarge {
            actual: MAX_PACK_BYTES + 1,
            limit: MAX_PACK_BYTES,
        })
    );
    assert_eq!(
        decode_catalog_pack(b"\xef\xbb\xbf{}"),
        Err(PackValidationError::Bom)
    );
    assert_eq!(
        decode_catalog_pack(&[0xff]),
        Err(PackValidationError::InvalidUtf8)
    );
    let mut trailing = common::CORE_PACK.to_vec();
    trailing.extend_from_slice(b" null");
    assert_eq!(
        decode_catalog_pack(&trailing),
        Err(PackValidationError::Json)
    );
    let too_deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
    assert_eq!(
        decode_catalog_pack(too_deep.as_bytes()),
        Err(PackValidationError::DepthExceeded { limit: 32 })
    );
}

#[test]
fn pack_rejects_duplicate_keys_at_root_and_recipe_depth() {
    let root = String::from_utf8(common::CORE_PACK.to_vec())
        .unwrap()
        .replacen(
            "\"format_version\": 1,",
            "\"format_version\":1,\"format_version\":1,",
            1,
        );
    assert_eq!(
        decode_catalog_pack(root.as_bytes()),
        Err(PackValidationError::DuplicateKey)
    );
    let nested = String::from_utf8(common::CORE_PACK.to_vec())
        .unwrap()
        .replacen("\"quantity\": 4", "\"quantity\":4,\"quantity\":4", 1);
    assert_eq!(
        decode_catalog_pack(nested.as_bytes()),
        Err(PackValidationError::DuplicateKey)
    );
}

#[test]
fn pack_rejects_unknown_fields_floats_and_authority_content() {
    let mut unknown: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    unknown["scenarios"] = serde_json::json!([]);
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&unknown).unwrap()),
        Err(PackValidationError::Json)
    );

    let floating = String::from_utf8(common::CORE_PACK.to_vec())
        .unwrap()
        .replacen("\"pack_version\": 1", "\"pack_version\": 1.0", 1);
    assert_eq!(
        decode_catalog_pack(floating.as_bytes()),
        Err(PackValidationError::Float)
    );

    let mut forbidden: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    forbidden["tick_hz"] = serde_json::json!(10);
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&forbidden).unwrap()),
        Err(PackValidationError::Json)
    );
}

#[test]
fn pack_rejects_bad_ids_names_counts_quantities_and_references() {
    let mut invalid_id: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    invalid_id["pack_id"] = serde_json::json!("Bad Pack");
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&invalid_id).unwrap()),
        Err(PackValidationError::InvalidField { field: "pack_id" })
    );

    let mut invalid_name: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    invalid_name["resources"][0]["name"] = serde_json::json!("Bad  Name");
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&invalid_name).unwrap()),
        Err(PackValidationError::InvalidField { field: "name" })
    );

    let mut too_many: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    let prototype = too_many["star_archetypes"][0].clone();
    let stars = too_many["star_archetypes"].as_array_mut().unwrap();
    for index in 1..33 {
        let mut star = prototype.clone();
        star["id"] = serde_json::json!(format!("star-{index}"));
        stars.push(star);
    }
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&too_many).unwrap()),
        Err(PackValidationError::DefinitionLimit { kind: "star" })
    );

    let mut zero_quantity: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    zero_quantity["industry_definitions"][0]["outputs"][0]["quantity"] = serde_json::json!(0);
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&zero_quantity).unwrap()),
        Err(PackValidationError::InvalidField { field: "quantity" })
    );

    let mut oversized_hazard_ratio: serde_json::Value =
        serde_json::from_slice(common::CORE_PACK).unwrap();
    oversized_hazard_ratio["hazard_definitions"][0]["route_capacity_numerator"] =
        serde_json::json!(1_000_000_001_u64);
    oversized_hazard_ratio["hazard_definitions"][0]["route_capacity_denominator"] =
        serde_json::json!(1_000_000_001_u64);
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&oversized_hazard_ratio).unwrap()),
        Err(PackValidationError::InvalidField { field: "hazard" })
    );

    let mut unknown_resource: serde_json::Value =
        serde_json::from_slice(common::CORE_PACK).unwrap();
    unknown_resource["industry_definitions"][0]["outputs"][0]["resource_id"] =
        serde_json::json!("water");
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&unknown_resource).unwrap()),
        Err(PackValidationError::UnknownResource)
    );

    let mut duplicate_id: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    duplicate_id["resources"][1]["id"] = duplicate_id["resources"][0]["id"].clone();
    assert_eq!(
        decode_catalog_pack(&serde_json::to_vec(&duplicate_id).unwrap()),
        Err(PackValidationError::DuplicateCatalogId)
    );
}

#[test]
fn deterministic_arbitrary_byte_inputs_never_panic() {
    let mut generator = 0xd1b5_4a32_d192_ed03_u64;
    for case in 0..512_usize {
        generator = xorshift64(generator);
        let length = usize::try_from(generator % 4_097).unwrap();
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            generator = xorshift64(generator);
            bytes.push(generator.to_le_bytes()[0]);
        }
        let result = std::panic::catch_unwind(|| decode_catalog_pack(&bytes));
        assert!(
            result.is_ok(),
            "decoder panicked for deterministic case {case}"
        );
    }
}

#[test]
fn deterministic_mutated_inputs_round_trip_if_they_validate() {
    let mut candidates = vec![
        common::CORE_PACK.to_vec(),
        [b" \n\t".as_slice(), common::CORE_PACK, b"\r\n".as_slice()].concat(),
    ];
    let mut generator = 0xa076_1d64_78bd_642f_u64;
    for case in 0..256_usize {
        let mut bytes = common::CORE_PACK.to_vec();
        generator = xorshift64(generator);
        let index = usize::try_from(generator % u64::try_from(bytes.len()).unwrap()).unwrap();
        generator = xorshift64(generator);
        bytes[index] = generator.to_le_bytes()[0];
        if case.is_multiple_of(4) {
            generator = xorshift64(generator);
            let length =
                usize::try_from(generator % u64::try_from(bytes.len().saturating_add(1)).unwrap())
                    .unwrap();
            bytes.truncate(length);
        }
        candidates.push(bytes);
    }

    let mut accepted = 0_usize;
    for bytes in candidates {
        let Ok(pack) = decode_catalog_pack(&bytes) else {
            continue;
        };
        accepted += 1;
        let canonical = encode_catalog_pack(&pack).unwrap();
        let decoded = decode_catalog_pack(&canonical).unwrap();
        assert_eq!(decoded.catalog_hash(), pack.catalog_hash());
        assert_eq!(encode_catalog_pack(&decoded).unwrap(), canonical);
    }
    assert!(
        accepted >= 2,
        "the valid seed corpus must exercise round trips"
    );
}

fn xorshift64(mut value: u64) -> u64 {
    value ^= value << 13;
    value ^= value >> 7;
    value ^= value << 17;
    value
}
