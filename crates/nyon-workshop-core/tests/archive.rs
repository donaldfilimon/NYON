mod common;

use nyon_workshop_core::{
    ArchiveError, CreatorBatchV1, CreatorOpV1, WorkshopHistory, archive_catalog_hash,
    decode_archive, encode_archive,
};

const TEST_REPLAY_BUDGET: u64 = 1_000_000;

#[test]
fn canonical_archive_round_trip_replays_graph_branches_tick_and_digest() {
    let pack = common::pack();
    let mut history = WorkshopHistory::from_seed_u64(pack.clone(), 41);
    let forge = common::create_forge(&mut history);
    history.advance_ticks(4).unwrap();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(forge.world_b),
                name: common::name("Archive World"),
            }],
        })
        .unwrap();

    let archive = encode_archive(&history).unwrap();
    assert!(
        !archive
            .bytes
            .windows(10)
            .any(|window| window == b"checkpoint")
    );
    let restored = decode_archive(&pack, &archive.bytes, TEST_REPLAY_BUDGET).unwrap();
    assert_eq!(restored.state(), history.state());
    assert_eq!(restored.active_view(), history.active_view());
    assert_eq!(restored.revisions(), history.revisions());
    assert_eq!(restored.branches(), history.branches());
    assert_eq!(encode_archive(&restored).unwrap(), archive);
}

#[test]
fn archive_rejects_noncanonical_tampered_and_wrong_catalog_bytes() {
    let pack = common::pack();
    let mut history = WorkshopHistory::from_seed_u64(pack.clone(), 42);
    common::create_forge(&mut history);
    history.step().unwrap();
    let archive = encode_archive(&history).unwrap();

    let value: serde_json::Value = serde_json::from_slice(&archive.bytes).unwrap();
    let pretty = serde_json::to_vec_pretty(&value).unwrap();
    assert!(matches!(
        decode_archive(&pack, &pretty, TEST_REPLAY_BUDGET),
        Err(ArchiveError::NonCanonical)
    ));

    let mut tampered = archive.bytes.clone();
    let position = tampered
        .windows(b"\"integrity_sha256\":[".len())
        .position(|window| window == b"\"integrity_sha256\":[")
        .unwrap()
        + b"\"integrity_sha256\":[".len();
    tampered[position] = if tampered[position] == b'1' {
        b'2'
    } else {
        b'1'
    };
    assert!(decode_archive(&pack, &tampered, TEST_REPLAY_BUDGET).is_err());

    let mut other_value: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    other_value["title"] = serde_json::json!("Other Core");
    let other_pack =
        nyon_workshop_core::decode_catalog_pack(&serde_json::to_vec(&other_value).unwrap())
            .unwrap();
    assert!(matches!(
        decode_archive(&other_pack, &archive.bytes, TEST_REPLAY_BUDGET),
        Err(ArchiveError::CatalogMismatch)
    ));
}

#[test]
fn archive_catalog_inspection_authenticates_only_a_canonical_supported_envelope() {
    let pack = common::pack();
    let archive = encode_archive(&WorkshopHistory::from_seed_u64(pack.clone(), 43)).unwrap();
    assert_eq!(
        archive_catalog_hash(&archive.bytes).unwrap(),
        pack.catalog_hash()
    );

    let value: serde_json::Value = serde_json::from_slice(&archive.bytes).unwrap();
    assert!(matches!(
        archive_catalog_hash(&serde_json::to_vec_pretty(&value).unwrap()),
        Err(ArchiveError::NonCanonical)
    ));

    let mut unsupported = archive.bytes.clone();
    replace_once(
        &mut unsupported,
        b"\"format_version\":1",
        b"\"format_version\":2",
    );
    assert!(matches!(
        archive_catalog_hash(&unsupported),
        Err(ArchiveError::Format)
    ));

    let mut bad_integrity = archive.bytes.clone();
    mutate_first_integrity_byte(&mut bad_integrity);
    assert!(matches!(
        archive_catalog_hash(&bad_integrity),
        Err(ArchiveError::Integrity)
    ));

    let oversized = vec![b' '; nyon_workshop_core::archive::MAX_ARCHIVE_BYTES + 1];
    assert!(matches!(
        archive_catalog_hash(&oversized),
        Err(ArchiveError::TooLarge { actual, limit })
            if actual == limit + 1 && limit == nyon_workshop_core::archive::MAX_ARCHIVE_BYTES
    ));
}

fn replace_once(bytes: &mut [u8], before: &[u8], after: &[u8]) {
    assert_eq!(before.len(), after.len());
    let start = bytes
        .windows(before.len())
        .position(|window| window == before)
        .expect("archive field is present");
    bytes[start..start + before.len()].copy_from_slice(after);
}

fn mutate_first_integrity_byte(bytes: &mut [u8]) {
    let prefix = b"\"integrity_sha256\":[";
    let start = bytes
        .windows(prefix.len())
        .position(|window| window == prefix)
        .expect("integrity field is present")
        + prefix.len();
    let end = bytes[start..]
        .iter()
        .position(|byte| *byte == b',' || *byte == b']')
        .map(|offset| start + offset)
        .unwrap();
    let original: u8 = std::str::from_utf8(&bytes[start..end])
        .unwrap()
        .parse()
        .unwrap();
    let replacement = match original {
        0..=8 => original + 1,
        9 => 8,
        10..=98 => original + 1,
        99 => 98,
        100..=254 => original + 1,
        255 => 254,
    }
    .to_string();
    assert_eq!(replacement.len(), end - start);
    bytes[start..end].copy_from_slice(replacement.as_bytes());
}
