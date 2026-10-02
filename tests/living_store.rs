#![cfg(not(target_arch = "wasm32"))]
use nyon::living_store::{native::NativeLivingStoreV2, *};
use nyon_workshop_core::living::{
    LIVING_CORE_PACK_BYTES_V2, builtin_first_expansion_v2, encode_living_archive_v2,
};
fn request(
    store: &mut NativeLivingStoreV2,
    r: LivingStoreRequestV2,
) -> Result<LivingStoreResultV2, LivingStoreErrorV2> {
    let id = store.begin(7, r)?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match store.poll(7, id, 1024) {
            LivingStorePollV2::Pending { .. } => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            LivingStorePollV2::Ready(result) => {
                assert!(store.release(7, id));
                return Ok(result);
            }
            LivingStorePollV2::Failed(error) => {
                assert!(store.release(7, id));
                return Err(error);
            }
        }
    }
}
fn root() -> std::path::PathBuf {
    static NEXT_ROOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "nyon-living-store-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ))
}
#[test]
fn native_exact_archive_reopen_cas_lifecycle_and_pack_reference_protection() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let pack = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let LivingStoreResultV2::Pack(pack) = pack else {
        panic!("pack")
    };
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..61 {
        authority.step().unwrap();
    }
    let digest = authority.state().digest().unwrap();
    let archive = encode_living_archive_v2(&authority).unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "First Expansion".into(),
            archive: archive.clone(),
        },
    )
    .unwrap() else {
        panic!("slot")
    };
    let mut reopened = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Loaded { authority, .. } = request(
        &mut reopened,
        LivingStoreRequestV2::LoadSlot { id: slot.id },
    )
    .unwrap() else {
        panic!("load")
    };
    assert_eq!(authority.state().digest().unwrap(), digest);
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::SaveSlot {
                id: slot.id,
                expected_generation: LivingGenerationV2(0),
                archive: archive.clone()
            }
        ),
        Err(LivingStoreErrorV2::StaleGeneration)
    ));
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::SetPackArchived {
                hash: pack.hash,
                expected_generation: pack.generation,
                archived: true
            }
        ),
        Err(LivingStoreErrorV2::Referenced)
    ));
    let LivingStoreResultV2::Slot(archived) = request(
        &mut reopened,
        LivingStoreRequestV2::SetArchived {
            id: slot.id,
            expected_generation: slot.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!("archive")
    };
    let LivingStoreResultV2::Pack(archived_pack) = request(
        &mut reopened,
        LivingStoreRequestV2::SetPackArchived {
            hash: pack.hash,
            expected_generation: pack.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!("archive pack")
    };
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::DeletePack {
                hash: pack.hash,
                expected_generation: archived_pack.generation
            }
        ),
        Err(LivingStoreErrorV2::Referenced)
    ));
    request(
        &mut reopened,
        LivingStoreRequestV2::DeleteArchivedSlot {
            id: slot.id,
            expected_generation: archived.generation,
        },
    )
    .unwrap();
    request(
        &mut reopened,
        LivingStoreRequestV2::DeletePack {
            hash: pack.hash,
            expected_generation: archived_pack.generation,
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn begin_is_nonmutating_cancel_and_wrong_owner_are_explicit() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let job = store
        .begin(
            7,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
            },
        )
        .unwrap();
    assert!(!root.exists());
    assert!(matches!(
        store.poll(8, job, 1024),
        LivingStorePollV2::Failed(LivingStoreErrorV2::WrongOwner)
    ));
    assert!(matches!(
        store.cancel(7, job).unwrap(),
        LivingStoreCancelV2::Cancelled
    ));
    assert!(!root.exists());
    assert!(matches!(
        store.begin(
            7,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: b"{\"kind\":\"NYON_WORKSHOP_DATA\"}".to_vec()
            }
        ),
        Err(LivingStoreErrorV2::WrongAuthority)
    ));
}

#[test]
fn invalid_newest_payload_recovers_prior_and_saves_using_recovered_generation() {
    use sha2::{Digest, Sha256};
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let mut authority = builtin_first_expansion_v2().unwrap();
    let first = encode_living_archive_v2(&authority).unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Recovery".into(),
            archive: first,
        },
    )
    .unwrap() else {
        panic!()
    };
    authority.step().unwrap();
    let second = encode_living_archive_v2(&authority).unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::SaveSlot {
            id: slot.id,
            expected_generation: slot.generation,
            archive: second.clone(),
        },
    )
    .unwrap();
    let file = Sha256::digest(&second)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(root.join("living-v2").join(file), b"broken").unwrap();
    let LivingStoreResultV2::Loaded {
        descriptor,
        authority: loaded,
    } = request(&mut store, LivingStoreRequestV2::LoadSlot { id: slot.id }).unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.generation, LivingGenerationV2(1));
    assert_eq!(loaded.state().tick.0, 0);
    authority.step().unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::SaveSlot {
            id: slot.id,
            expected_generation: descriptor.generation,
            archive: encode_living_archive_v2(&authority).unwrap(),
        },
    )
    .unwrap();
    let LivingStoreResultV2::Loaded {
        authority: loaded, ..
    } = request(&mut store, LivingStoreRequestV2::LoadSlot { id: slot.id }).unwrap()
    else {
        panic!()
    };
    assert_eq!(loaded.state().tick.0, 2);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn sixty_four_archived_slots_still_consume_capacity_until_deleted() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let bytes = encode_living_archive_v2(&builtin_first_expansion_v2().unwrap()).unwrap();
    let mut first = None;
    for n in 0..64 {
        let LivingStoreResultV2::Slot(slot) = request(
            &mut store,
            LivingStoreRequestV2::CreateSlot {
                name: format!("Galaxy {n}"),
                archive: bytes.clone(),
            },
        )
        .unwrap() else {
            panic!()
        };
        if n == 0 {
            first = Some(slot);
        }
    }
    let first = first.unwrap();
    let LivingStoreResultV2::Slot(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetArchived {
            id: first.id,
            expected_generation: first.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::CreateSlot {
                name: "Overflow".into(),
                archive: bytes.clone()
            }
        ),
        Err(LivingStoreErrorV2::Limit)
    ));
    request(
        &mut store,
        LivingStoreRequestV2::DeleteArchivedSlot {
            id: archived.id,
            expected_generation: archived.generation,
        },
    )
    .unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Replacement".into(),
            archive: bytes.clone(),
        },
    )
    .unwrap();
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::CreateSlot {
                name: "Overflow".into(),
                archive: bytes
            }
        ),
        Err(LivingStoreErrorV2::Limit)
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn job_ids_cannot_be_replayed_against_another_adapter() {
    let one = root();
    let two = root();
    let mut a = NativeLivingStoreV2::new(&one);
    let mut b = NativeLivingStoreV2::new(&two);
    let job = a.begin(7, LivingStoreRequestV2::ListSlots).unwrap();
    let foreign = b.begin(7, LivingStoreRequestV2::ListSlots).unwrap();
    assert_ne!(job, foreign);
    assert!(matches!(
        b.poll(7, job, 1024),
        LivingStorePollV2::Failed(LivingStoreErrorV2::NotFound)
    ));
    a.cancel(7, job).unwrap();
    b.cancel(7, foreign).unwrap();
}

#[test]
fn registered_pack_capacity_requires_archived_unreferenced_deletion() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let original = std::str::from_utf8(LIVING_CORE_PACK_BYTES_V2).unwrap();
    let make = |n| {
        original
            .replace(
                "\"pack_id\":\"nyon_living_core\"",
                &format!("\"pack_id\":\"capacity_{n}\""),
            )
            .into_bytes()
    };
    let mut first = None;
    for n in 0..256 {
        let LivingStoreResultV2::Pack(pack) = request(
            &mut store,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: make(n),
            },
        )
        .unwrap() else {
            panic!()
        };
        if n == 0 {
            first = Some(pack);
        }
    }
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: make(256)
            }
        ),
        Err(LivingStoreErrorV2::Limit)
    ));
    let first = first.unwrap();
    let LivingStoreResultV2::Pack(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: first.hash,
            expected_generation: first.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::DeletePack {
                hash: first.hash,
                expected_generation: first.generation
            }
        ),
        Err(LivingStoreErrorV2::StaleGeneration)
    ));
    request(
        &mut store,
        LivingStoreRequestV2::DeletePack {
            hash: first.hash,
            expected_generation: archived.generation,
        },
    )
    .unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: make(256),
        },
    )
    .unwrap();
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: make(257)
            }
        ),
        Err(LivingStoreErrorV2::Limit)
    ));
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn missing_payload_is_a_per_item_listing_diagnostic() {
    use sha2::{Digest, Sha256};
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let archive = encode_living_archive_v2(&builtin_first_expansion_v2().unwrap()).unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Diagnose".into(),
            archive: archive.clone(),
        },
    )
    .unwrap();
    let file = Sha256::digest(&archive)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::remove_file(root.join("living-v2").join(file)).unwrap();
    let LivingStoreResultV2::Slots(slots) =
        request(&mut store, LivingStoreRequestV2::ListSlots).unwrap()
    else {
        panic!()
    };
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].diagnostic, Some(LivingStoreErrorV2::NotFound));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_manifest_commit_preserves_the_confirmed_archive() {
    use nyon::living_store::native::NativeLivingFaultPoint;
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let mut authority = builtin_first_expansion_v2().unwrap();
    let first = encode_living_archive_v2(&authority).unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Interrupted".into(),
            archive: first,
        },
    )
    .unwrap() else {
        panic!()
    };
    authority.step().unwrap();
    store.inject_next_failure(NativeLivingFaultPoint::BeforeHead);
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::SaveSlot {
                id: slot.id,
                expected_generation: slot.generation,
                archive: encode_living_archive_v2(&authority).unwrap()
            }
        ),
        Err(LivingStoreErrorV2::Storage(_))
    ));
    let mut reopened = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Loaded {
        descriptor,
        authority: loaded,
    } = request(
        &mut reopened,
        LivingStoreRequestV2::LoadSlot { id: slot.id },
    )
    .unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.generation, slot.generation);
    assert_eq!(loaded.state().tick.0, 0);
    request(
        &mut reopened,
        LivingStoreRequestV2::SaveSlot {
            id: slot.id,
            expected_generation: slot.generation,
            archive: encode_living_archive_v2(&authority).unwrap(),
        },
    )
    .unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn cancellation_wins_while_native_worker_waits_for_commit_lock() {
    let root = root();
    std::fs::create_dir_all(root.join("living-v2")).unwrap();
    let blocker = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("living-v2/lock"))
        .unwrap();
    blocker.lock().unwrap();
    let mut store = NativeLivingStoreV2::new(&root);
    let job = store
        .begin(
            7,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
            },
        )
        .unwrap();
    assert!(matches!(
        store.poll(7, job, 1024),
        LivingStorePollV2::Pending { .. }
    ));
    assert!(matches!(
        store.cancel(7, job).unwrap(),
        LivingStoreCancelV2::Cancelled
    ));
    drop(blocker);
    let LivingStoreResultV2::Packs(packs) =
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap()
    else {
        panic!()
    };
    assert!(packs.is_empty());
    assert!(!root.join("living-v2/head").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn retired_payloads_are_pruned_only_after_both_recovery_manifests_release_them() {
    use sha2::{Digest, Sha256};
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Pack(pack) = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let file = Sha256::digest(LIVING_CORE_PACK_BYTES_V2)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let unknown = root.join("living-v2").join("0".repeat(64));
    std::fs::write(&unknown, b"unregistered preserved bytes").unwrap();
    let LivingStoreResultV2::Pack(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: pack.hash,
            expected_generation: pack.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    request(
        &mut store,
        LivingStoreRequestV2::DeletePack {
            hash: pack.hash,
            expected_generation: archived.generation,
        },
    )
    .unwrap();
    assert!(root.join("living-v2").join(&file).exists());
    let replacement = std::str::from_utf8(LIVING_CORE_PACK_BYTES_V2)
        .unwrap()
        .replace(
            "\"pack_id\":\"nyon_living_core\"",
            "\"pack_id\":\"replacement\"",
        )
        .into_bytes();
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: replacement,
        },
    )
    .unwrap();
    assert!(!root.join("living-v2").join(file).exists());
    assert_eq!(
        std::fs::read(unknown).unwrap(),
        b"unregistered preserved bytes"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn notebooks_are_exact_generation_cas_attachments_preserved_on_metadata_and_detached_on_save() {
    use nyon_workshop_core::living::{
        LivingNotebookV2, decode_living_archive_v2, decode_living_catalog_pack_v2,
    };
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let authority = builtin_first_expansion_v2().unwrap();
    let archive = encode_living_archive_v2(&authority).unwrap();
    let catalog = decode_living_catalog_pack_v2(LIVING_CORE_PACK_BYTES_V2).unwrap();
    let validated = decode_living_archive_v2(&archive, &catalog).unwrap();
    let bytes = LivingNotebookV2::seal(&validated, vec![], vec![])
        .unwrap()
        .canonical_bytes()
        .unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Notebook".into(),
            archive: archive.clone(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let LivingStoreResultV2::NotebookBytes(None) = request(
        &mut store,
        LivingStoreRequestV2::ReadNotebook {
            id: slot.id,
            generation: slot.generation,
        },
    )
    .unwrap() else {
        panic!()
    };
    let mut other_authority = builtin_first_expansion_v2().unwrap();
    other_authority.step().unwrap();
    let other_archive = decode_living_archive_v2(
        &encode_living_archive_v2(&other_authority).unwrap(),
        &catalog,
    )
    .unwrap();
    let wrong_binding = LivingNotebookV2::seal(&other_archive, vec![], vec![])
        .unwrap()
        .canonical_bytes()
        .unwrap();
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::WriteNotebook {
                id: slot.id,
                expected_generation: slot.generation,
                canonical_bytes: wrong_binding
            }
        ),
        Err(LivingStoreErrorV2::Integrity)
    ));
    let LivingStoreResultV2::Slot(written) = request(
        &mut store,
        LivingStoreRequestV2::WriteNotebook {
            id: slot.id,
            expected_generation: slot.generation,
            canonical_bytes: bytes.clone(),
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::WriteNotebook {
                id: slot.id,
                expected_generation: slot.generation,
                canonical_bytes: bytes.clone()
            }
        ),
        Err(LivingStoreErrorV2::StaleGeneration)
    ));
    let mut reopened = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::NotebookBytes(Some(actual)) = request(
        &mut reopened,
        LivingStoreRequestV2::ReadNotebook {
            id: slot.id,
            generation: written.generation,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(actual, bytes);
    let LivingStoreResultV2::Slot(renamed) = request(
        &mut reopened,
        LivingStoreRequestV2::RenameSlot {
            id: slot.id,
            expected_generation: written.generation,
            name: "Renamed".into(),
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: renamed.generation
            }
        )
        .unwrap(),
        LivingStoreResultV2::NotebookBytes(Some(_))
    ));
    let LivingStoreResultV2::Slot(saved) = request(
        &mut reopened,
        LivingStoreRequestV2::SaveSlot {
            id: slot.id,
            expected_generation: renamed.generation,
            archive,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: saved.generation
            }
        )
        .unwrap(),
        LivingStoreResultV2::NotebookBytes(None)
    ));
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: renamed.generation
            }
        )
        .unwrap(),
        LivingStoreResultV2::NotebookBytes(Some(_))
    ));
    let mut invalid = bytes;
    invalid.push(b' ');
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::WriteNotebook {
                id: slot.id,
                expected_generation: saved.generation,
                canonical_bytes: invalid
            }
        ),
        Err(LivingStoreErrorV2::Integrity)
    ));
    let mut too_large = vec![b' '; 1024 * 1024 + 1];
    too_large[..30].copy_from_slice(b"{\"kind\":\"NYON_LIVING_NOTEBOOK\"");
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::WriteNotebook {
                id: slot.id,
                expected_generation: saved.generation,
                canonical_bytes: too_large
            }
        ),
        Err(LivingStoreErrorV2::TooLarge)
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_notebook_write_preserves_prior_generation_and_missing_attachment() {
    use nyon::living_store::native::NativeLivingFaultPoint;
    use nyon_workshop_core::living::{
        LivingNotebookV2, decode_living_archive_v2, decode_living_catalog_pack_v2,
    };
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let archive = encode_living_archive_v2(&builtin_first_expansion_v2().unwrap()).unwrap();
    let catalog = decode_living_catalog_pack_v2(LIVING_CORE_PACK_BYTES_V2).unwrap();
    let validated = decode_living_archive_v2(&archive, &catalog).unwrap();
    let bytes = LivingNotebookV2::seal(&validated, vec![], vec![])
        .unwrap()
        .canonical_bytes()
        .unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Interrupted notebook".into(),
            archive,
        },
    )
    .unwrap() else {
        panic!()
    };
    let mut interrupted_orphan = None;
    for fault in [
        NativeLivingFaultPoint::BeforeManifest,
        NativeLivingFaultPoint::BeforeHead,
    ] {
        if let Some(orphan) = interrupted_orphan.take() {
            request(&mut store, LivingStoreRequestV2::ListSlots).unwrap();
            assert!(
                !root.join("living-v2").join(orphan).exists(),
                "known pre-manifest orphan reclaimed"
            );
            assert!(!root.join("living-v2/staging").exists());
        }
        store.inject_next_failure(fault);
        assert!(matches!(
            request(
                &mut store,
                LivingStoreRequestV2::WriteNotebook {
                    id: slot.id,
                    expected_generation: slot.generation,
                    canonical_bytes: bytes.clone()
                }
            ),
            Err(LivingStoreErrorV2::Storage(_))
        ));
        let journal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("living-v2/staging")).unwrap())
                .unwrap();
        let file = journal["references"].as_array().unwrap().last().unwrap()["file"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(root.join("living-v2").join(&file).exists());
        if fault == NativeLivingFaultPoint::BeforeManifest {
            interrupted_orphan = Some(file);
        }
    }
    let mut reopened = NativeLivingStoreV2::new(&root);
    assert!(matches!(
        request(
            &mut reopened,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: slot.generation
            }
        )
        .unwrap(),
        LivingStoreResultV2::NotebookBytes(None)
    ));
    let LivingStoreResultV2::Slot(written) = request(
        &mut reopened,
        LivingStoreRequestV2::WriteNotebook {
            id: slot.id,
            expected_generation: slot.generation,
            canonical_bytes: bytes,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(written.generation.0, slot.generation.0 + 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn notebook_cancellation_after_worker_start_preserves_archive_generation_and_selection_boundary() {
    use nyon_workshop_core::living::{
        LivingNotebookV2, decode_living_archive_v2, decode_living_catalog_pack_v2,
    };
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let archive = encode_living_archive_v2(&builtin_first_expansion_v2().unwrap()).unwrap();
    let catalog = decode_living_catalog_pack_v2(LIVING_CORE_PACK_BYTES_V2).unwrap();
    let validated = decode_living_archive_v2(&archive, &catalog).unwrap();
    let bytes = LivingNotebookV2::seal(&validated, vec![], vec![])
        .unwrap()
        .canonical_bytes()
        .unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Cancel notebook".into(),
            archive,
        },
    )
    .unwrap() else {
        panic!()
    };
    let blocker = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("living-v2/lock"))
        .unwrap();
    blocker.lock().unwrap();
    let job = store
        .begin(
            7,
            LivingStoreRequestV2::WriteNotebook {
                id: slot.id,
                expected_generation: slot.generation,
                canonical_bytes: bytes.clone(),
            },
        )
        .unwrap();
    assert!(matches!(
        store.poll(7, job, 1024),
        LivingStorePollV2::Pending { .. }
    ));
    assert!(matches!(
        store.cancel(7, job).unwrap(),
        LivingStoreCancelV2::Cancelled
    ));
    drop(blocker);
    let LivingStoreResultV2::Loaded { descriptor, .. } =
        request(&mut store, LivingStoreRequestV2::LoadSlot { id: slot.id }).unwrap()
    else {
        panic!()
    };
    assert_eq!(descriptor.generation, slot.generation);
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: slot.generation
            }
        )
        .unwrap(),
        LivingStoreResultV2::NotebookBytes(None)
    ));
    let LivingStoreResultV2::Slot(written) = request(
        &mut store,
        LivingStoreRequestV2::WriteNotebook {
            id: slot.id,
            expected_generation: slot.generation,
            canonical_bytes: bytes,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(written.generation.0, slot.generation.0 + 1);
    std::fs::remove_dir_all(root).unwrap();
}

fn legacy_manifest_bytes(bytes: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let source = std::str::from_utf8(bytes).unwrap();
    let payload_start = source.find("\"payload\":").unwrap() + "\"payload\":".len();
    let integrity_start = source.rfind(",\"integrity\":").unwrap();
    let integrity: [u8; 32] =
        Sha256::digest(&source.as_bytes()[payload_start..integrity_start]).into();
    format!(
        "{},\"integrity\":{}}}",
        source[..integrity_start].replace(",\"format_version\":2", ""),
        serde_json::to_string(&integrity).unwrap()
    )
    .into_bytes()
}
#[test]
fn private_manifest_generation_tampering_cannot_reorder_recovery() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let file = root.join("living-v2/generation-a");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    value["generation"] = serde_json::json!(99);
    std::fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    std::fs::remove_file(root.join("living-v2/head")).unwrap();
    assert_eq!(
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap_err(),
        LivingStoreErrorV2::CorruptRecovery
    );
    assert!(file.exists());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn legacy_manifest_requires_intact_exact_head_and_upgrades_only_on_write() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap();
    let file = root.join("living-v2/generation-a");
    let legacy = legacy_manifest_bytes(&std::fs::read(&file).unwrap());
    std::fs::write(&file, &legacy).unwrap();
    let LivingStoreResultV2::Packs(packs) =
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap()
    else {
        panic!()
    };
    assert_eq!(packs.len(), 1);
    assert_eq!(std::fs::read(&file).unwrap(), legacy);
    let tampered = String::from_utf8(legacy.clone()).unwrap().replace(
        "\"generation\":1,\"payload\":",
        "\"generation\":99,\"payload\":",
    );
    assert_ne!(tampered.as_bytes(), legacy.as_slice());
    std::fs::write(&file, tampered).unwrap();
    assert_eq!(
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap_err(),
        LivingStoreErrorV2::CorruptRecovery
    );
    std::fs::write(&file, &legacy).unwrap();
    std::fs::write(root.join("living-v2/head"), b"invalid").unwrap();
    assert_eq!(
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap_err(),
        LivingStoreErrorV2::CorruptRecovery
    );
    assert_eq!(std::fs::read(&file).unwrap(), legacy);
    std::fs::remove_file(root.join("living-v2/head")).unwrap();
    assert_eq!(
        request(&mut store, LivingStoreRequestV2::ListPacks).unwrap_err(),
        LivingStoreErrorV2::CorruptRecovery
    );
    std::fs::write(root.join("living-v2/head"), b"1").unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: packs[0].hash,
            expected_generation: packs[0].generation,
            archived: true,
        },
    )
    .unwrap();
    let next: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("living-v2/generation-b")).unwrap())
            .unwrap();
    assert_eq!(next["format_version"], 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn oversized_archive_pack_and_notebook_refuse_before_io_or_job_lane_admission() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    for request in [
        LivingStoreRequestV2::CreateSlot {
            name: "Oversize".into(),
            archive: vec![b' '; 32 * 1024 * 1024 + 1],
        },
        LivingStoreRequestV2::PutPack {
            canonical_bytes: vec![b' '; 2 * 1024 * 1024 + 1],
        },
        LivingStoreRequestV2::WriteNotebook {
            id: LivingSlotIdV2(1),
            expected_generation: LivingGenerationV2(1),
            canonical_bytes: vec![b' '; 1024 * 1024 + 1],
        },
    ] {
        assert_eq!(store.begin(7, request), Err(LivingStoreErrorV2::TooLarge));
        assert!(!root.exists());
    }
    // Rejections consume no lane and cannot poison a subsequent valid job.
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec()
            }
        ),
        Ok(LivingStoreResultV2::Pack(_))
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pack_reference_metadata_retains_old_archived_generations_without_mutating_head() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Pack(pack) = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let mut authority = builtin_first_expansion_v2().unwrap();
    let LivingStoreResultV2::Slot(first) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Exact retained pack references".into(),
            archive: encode_living_archive_v2(&authority).unwrap(),
        },
    )
    .unwrap() else {
        panic!()
    };
    authority.step().unwrap();
    let LivingStoreResultV2::Slot(saved) = request(
        &mut store,
        LivingStoreRequestV2::SaveSlot {
            id: first.id,
            expected_generation: first.generation,
            archive: encode_living_archive_v2(&authority).unwrap(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let LivingStoreResultV2::Slot(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetArchived {
            id: saved.id,
            expected_generation: saved.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    let before = std::fs::read(root.join("living-v2/head")).unwrap();
    let LivingStoreResultV2::PackReferences { hash, references } = request(
        &mut store,
        LivingStoreRequestV2::ListPackReferences { hash: pack.hash },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(hash, pack.hash);
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].retained_generation, saved.generation);
    assert_eq!(references[1].retained_generation, archived.generation);
    assert!(
        references
            .iter()
            .all(|r| r.slot == first.id && r.archived && r.name == first.name)
    );
    assert_eq!(std::fs::read(root.join("living-v2/head")).unwrap(), before);
    let LivingStoreResultV2::Pack(pack) = request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: pack.hash,
            expected_generation: pack.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        request(
            &mut store,
            LivingStoreRequestV2::DeletePack {
                hash: pack.hash,
                expected_generation: pack.generation
            }
        )
        .unwrap_err(),
        LivingStoreErrorV2::Referenced
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pack_reference_metadata_lists_old_catalog_when_head_uses_another_catalog() {
    use nyon_workshop_core::living::*;
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Pack(old_pack) = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let original = builtin_first_expansion_v2().unwrap();
    let old_archive = encode_living_archive_v2(&original).unwrap();
    let LivingStoreResultV2::Slot(slot) = request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Old exact catalog".into(),
            archive: old_archive.clone(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let bytes = std::str::from_utf8(LIVING_CORE_PACK_BYTES_V2)
        .unwrap()
        .replace(
            "\"pack_id\":\"nyon_living_core\"",
            "\"pack_id\":\"references_other\"",
        )
        .into_bytes();
    let catalog = decode_living_catalog_pack_v2(&bytes).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&old_archive).unwrap();
    let manifest: LivingGenesisManifestV2 =
        serde_json::from_value(value["genesis_manifest"].clone()).unwrap();
    let genesis = validate_living_genesis_manifest_v2(manifest, &catalog).unwrap();
    let authority = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        original.genesis_seed(),
        original.generator().clone(),
        genesis,
    )
    .unwrap();
    request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: bytes,
        },
    )
    .unwrap();
    let LivingStoreResultV2::Slot(head) = request(
        &mut store,
        LivingStoreRequestV2::SaveSlot {
            id: slot.id,
            expected_generation: slot.generation,
            archive: encode_living_archive_v2(&authority).unwrap(),
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_ne!(head.catalog_hash, old_pack.hash);
    let LivingStoreResultV2::PackReferences { hash, references } = request(
        &mut store,
        LivingStoreRequestV2::ListPackReferences {
            hash: old_pack.hash,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(hash, old_pack.hash);
    assert_eq!(
        references,
        vec![LivingPackReferenceV2 {
            slot: slot.id,
            name: slot.name.clone(),
            archived: false,
            retained_generation: slot.generation
        }]
    );
    assert_ne!(references[0].retained_generation, head.generation);
    let LivingStoreResultV2::Slot(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetArchived {
            id: head.id,
            expected_generation: head.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    let LivingStoreResultV2::PackReferences { references, .. } = request(
        &mut store,
        LivingStoreRequestV2::ListPackReferences {
            hash: old_pack.hash,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(
        references.is_empty(),
        "metadata advancement releases the older recovery archive"
    );
    assert_eq!(archived.generation.0, head.generation.0 + 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn pack_recreation_rejects_old_incarnation_generation_after_reopen() {
    let root = root();
    let mut store = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Pack(first) = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap() else {
        panic!()
    };
    let LivingStoreResultV2::Pack(old) = request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: first.hash,
            expected_generation: first.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    request(
        &mut store,
        LivingStoreRequestV2::DeletePack {
            hash: old.hash,
            expected_generation: old.generation,
        },
    )
    .unwrap();
    drop(store);
    let mut store = NativeLivingStoreV2::new(&root);
    store.inject_next_failure(nyon::living_store::native::NativeLivingFaultPoint::BeforeHead);
    assert!(matches!(
        request(
            &mut store,
            LivingStoreRequestV2::PutPack {
                canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec()
            }
        ),
        Err(LivingStoreErrorV2::Storage(_))
    ));
    drop(store);
    let mut store = NativeLivingStoreV2::new(&root);
    let LivingStoreResultV2::Pack(new) = request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    )
    .unwrap() else {
        panic!()
    };
    assert!(new.generation.0 > old.generation.0);
    let LivingStoreResultV2::Pack(archived) = request(
        &mut store,
        LivingStoreRequestV2::SetPackArchived {
            hash: new.hash,
            expected_generation: new.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(
        request(
            &mut store,
            LivingStoreRequestV2::DeletePack {
                hash: old.hash,
                expected_generation: old.generation
            }
        )
        .unwrap_err(),
        LivingStoreErrorV2::StaleGeneration
    );
    assert!(archived.generation.0 > old.generation.0);
    std::fs::remove_dir_all(root).unwrap();
}
