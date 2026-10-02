#![cfg(not(target_arch = "wasm32"))]
use nyon::{
    app::client_runtime::living::{LivingClient, LivingClientEvent},
    library_selection::native::NativeLibraryCoordinator,
    living_store::native::NativeLivingStoreV2,
};
use nyon_workshop_core::living::builtin_first_expansion_v2;
use std::time::{Duration, Instant};

fn make_client(root: &std::path::Path) -> LivingClient {
    LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    )
}
fn terminal(client: &mut LivingClient) -> LivingClientEvent {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let event = client.poll();
        if !matches!(event, LivingClientEvent::Pending) {
            return event;
        }
        assert!(Instant::now() < deadline, "Living client job timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn durable_save_and_neutral_continue_reopen_exact_authority() {
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::AbsentSelection
    ));
    client.select_neutral(None).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::SelectionCommitted
    ));
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..61 {
        authority.step().unwrap();
    }
    let expected = authority.bookmark().unwrap();
    client.save(&authority, false).unwrap();
    assert!(
        matches!(terminal(&mut client),LivingClientEvent::Saved {bookmark,leave:false} if bookmark==expected)
    );
    assert_eq!(client.saved, Some(expected));
    drop(client);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    match terminal(&mut reopened) {
        LivingClientEvent::Ready(authority) => assert_eq!(authority.bookmark().unwrap(), expected),
        LivingClientEvent::Failed(error) => panic!("exact reopen failed: {error}"),
        _ => panic!("expected exact Living candidate"),
    }
}

#[test]
fn recorded_digest_drift_enters_failure_without_installing_other_slot() {
    let root = tempfile::tempdir().unwrap();
    let mut first = make_client(root.path());
    first.read_selection().unwrap();
    let _ = terminal(&mut first);
    let authority = builtin_first_expansion_v2().unwrap();
    first.save(&authority, false).unwrap();
    let _ = terminal(&mut first);
    let mut reference = first.selection.as_ref().unwrap().target().unwrap().unwrap();
    reference.tick = 1;
    first.select_neutral(Some(reference)).unwrap();
    let _ = terminal(&mut first);
    drop(first);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    assert!(
        matches!(terminal(&mut reopened),LivingClientEvent::Failed(error) if error.contains("differs"))
    );
    assert!(reopened.slot.is_none());
    assert!(reopened.saved.is_none());
}

#[test]
fn cancelled_import_never_installs_or_selects_its_candidate() {
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    client.select_neutral(None).unwrap();
    let _ = terminal(&mut client);
    let before = client.selection.clone();
    client
        .import_candidate(Box::new(builtin_first_expansion_v2().unwrap()))
        .unwrap();
    client.cancel();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Cancelled
    ));
    assert_eq!(client.selection, before);
    assert!(client.slot.is_none());
    assert!(client.saved.is_none());
    assert!(!client.busy());
}

#[test]
fn archiving_recorded_nonresident_save_clears_neutral_continue_after_commit() {
    use nyon::living_store::LivingStoreRequestV2;
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    client
        .save(&builtin_first_expansion_v2().unwrap(), false)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    let descriptor = client.slot.clone().unwrap();
    client.new_resident().unwrap();
    client
        .mutate(LivingStoreRequestV2::SetArchived {
            id: descriptor.id,
            expected_generation: descriptor.generation,
            archived: true,
        })
        .unwrap();
    assert!(matches!(terminal(&mut client), LivingClientEvent::Mutated));
    assert_eq!(client.selection.as_ref().unwrap().target().unwrap(), None);
    drop(client);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut reopened),
        LivingClientEvent::NoSelection
    ));
}

#[test]
fn notebook_save_commits_attachment_before_ack_and_continue_restores_exact_metadata() {
    use nyon_workshop_core::living::{LivingNameV2, LivingNamedBookmarkV2};
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    let mut authority = builtin_first_expansion_v2().unwrap();
    authority.step().unwrap();
    let bookmark = authority.bookmark().unwrap();
    let named = LivingNamedBookmarkV2 {
        ordinal: 1,
        label: LivingNameV2::new("Before intervention").unwrap(),
        view: bookmark.view,
        digest: bookmark.digest,
    };
    client
        .save_with_notebook(&authority, vec![named.clone()], vec![], true)
        .unwrap();
    assert!(
        matches!(terminal(&mut client), LivingClientEvent::Saved { bookmark: actual, leave: true } if actual == bookmark)
    );
    assert_eq!(client.slot.as_ref().unwrap().generation.0, 2);
    drop(client);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    match terminal(&mut reopened) {
        LivingClientEvent::ReadyWithNotebook {
            authority,
            notebook,
        } => {
            assert_eq!(authority.bookmark().unwrap(), bookmark);
            assert_eq!(notebook.notebook().bookmarks, vec![named]);
            assert!(notebook.notebook().experiments.is_empty());
        }
        LivingClientEvent::Failed(error) => panic!("notebook exact reopen failed: {error}"),
        _ => panic!("expected validated notebook and authority"),
    }
}

#[test]
fn cancelled_notebook_preparation_has_no_store_or_coordinator_effects() {
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    let authority = builtin_first_expansion_v2().unwrap();
    client
        .save_with_notebook(&authority, vec![], vec![], false)
        .unwrap();
    client.cancel();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Cancelled
    ));
    assert!(client.slot.is_none());
    assert!(client.selection.is_none());
    assert!(!root.path().join("head").exists());
}

#[test]
fn invalid_notebook_witness_rejects_before_durable_save_or_selection() {
    use nyon_workshop_core::living::{LivingNameV2, LivingNamedBookmarkV2, LivingStateDigestV2};
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    let authority = builtin_first_expansion_v2().unwrap();
    let bookmark = authority.bookmark().unwrap();
    client
        .save_with_notebook(
            &authority,
            vec![LivingNamedBookmarkV2 {
                ordinal: 1,
                label: LivingNameV2::new("Invalid witness").unwrap(),
                view: bookmark.view,
                digest: LivingStateDigestV2([0; 32]),
            }],
            vec![],
            false,
        )
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Failed(_)
    ));
    assert!(client.slot.is_none());
    assert!(client.selection.is_none());
    assert!(!root.path().join("head").exists());
}

#[test]
fn continue_restores_retained_recorded_view_after_new_head_commit_and_validates_head_notebook() {
    use nyon_workshop_core::living::{LivingNameV2, LivingNamedBookmarkV2};
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    let mut authority = builtin_first_expansion_v2().unwrap();
    let old = authority.bookmark().unwrap();
    client.save(&authority, false).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    let recorded = client.selection.as_ref().unwrap().target().unwrap();
    authority.step().unwrap();
    let newer = authority.bookmark().unwrap();
    let named = LivingNamedBookmarkV2 {
        ordinal: 1,
        label: LivingNameV2::new("New committed head").unwrap(),
        view: newer.view,
        digest: newer.digest,
    };
    client
        .save_with_notebook(&authority, vec![named.clone()], vec![], false)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    // A partial save can publish this newer head while leaving the recorded
    // coordinator reference at the prior exact state.
    client.select_neutral(recorded).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::SelectionCommitted
    ));
    drop(client);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    match terminal(&mut reopened) {
        LivingClientEvent::ReadyWithNotebook {
            authority,
            notebook,
        } => {
            assert_eq!(authority.bookmark().unwrap(), old);
            assert_eq!(reopened.saved, Some(old));
            assert_eq!(notebook.notebook().bookmarks, vec![named]);
            assert_eq!(authority.state().tick.0, 0);
        }
        LivingClientEvent::Failed(error) => panic!("retained exact Continue failed: {error}"),
        _ => panic!("expected retained recorded view and validated head notebook"),
    }
}

struct NotebookQuotaStore(NativeLivingStoreV2);
impl nyon::living_store::LivingStoreV2 for NotebookQuotaStore {
    fn begin(
        &mut self,
        owner: u64,
        request: nyon::living_store::LivingStoreRequestV2,
    ) -> Result<nyon::living_store::LivingStoreJobIdV2, nyon::living_store::LivingStoreErrorV2>
    {
        if matches!(
            request,
            nyon::living_store::LivingStoreRequestV2::WriteNotebook { .. }
        ) {
            return Err(nyon::living_store::LivingStoreErrorV2::Storage(
                "QuotaExceededError test adapter".into(),
            ));
        }
        self.0.begin(owner, request)
    }
    fn poll(
        &mut self,
        owner: u64,
        job: nyon::living_store::LivingStoreJobIdV2,
        budget: usize,
    ) -> nyon::living_store::LivingStorePollV2 {
        self.0.poll(owner, job, budget)
    }
    fn cancel(
        &mut self,
        owner: u64,
        job: nyon::living_store::LivingStoreJobIdV2,
    ) -> Result<nyon::living_store::LivingStoreCancelV2, nyon::living_store::LivingStoreErrorV2>
    {
        self.0.cancel(owner, job)
    }
    fn release(&mut self, owner: u64, job: nyon::living_store::LivingStoreJobIdV2) -> bool {
        self.0.release(owner, job)
    }
}

#[test]
fn notebook_quota_failure_after_archive_commit_never_acknowledges_save_or_changes_continue() {
    use nyon::living_store::{LivingStoreRequestV2, LivingStoreResultV2, LivingStoreV2};
    let root = tempfile::tempdir().unwrap();
    let mut client = LivingClient::new(
        Box::new(NotebookQuotaStore(NativeLivingStoreV2::new(root.path()))),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    client.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::AbsentSelection
    ));
    client.select_neutral(None).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::SelectionCommitted
    ));
    let before = client.selection.clone();
    let authority = builtin_first_expansion_v2().unwrap();
    client
        .save_with_notebook(&authority, vec![], vec![], true)
        .unwrap();
    assert!(
        matches!(terminal(&mut client),LivingClientEvent::Failed(error) if error.contains("QuotaExceededError"))
    );
    assert!(client.saved.is_none());
    assert_eq!(client.selection, before);
    let slot = client
        .slot
        .as_ref()
        .expect("archive commit remains explicitly recoverable")
        .clone();
    let mut store = NativeLivingStoreV2::new(root.path());
    let job = store
        .begin(
            7,
            LivingStoreRequestV2::ReadNotebook {
                id: slot.id,
                generation: slot.generation,
            },
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match store.poll(7, job, 1024) {
            nyon::living_store::LivingStorePollV2::Pending { .. } => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            nyon::living_store::LivingStorePollV2::Ready(LivingStoreResultV2::NotebookBytes(
                None,
            )) => {
                assert!(store.release(7, job));
                break;
            }
            other => panic!("unexpected notebook result: {other:?}"),
        }
    }
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut reopened),
        LivingClientEvent::NoSelection
    ));
}

fn store_terminal(
    store: &mut NativeLivingStoreV2,
    request: nyon::living_store::LivingStoreRequestV2,
) -> Result<nyon::living_store::LivingStoreResultV2, nyon::living_store::LivingStoreErrorV2> {
    use nyon::living_store::{LivingStorePollV2, LivingStoreV2};
    let job = store.begin(7, request)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match store.poll(7, job, 1024) {
            LivingStorePollV2::Pending { .. } => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
            LivingStorePollV2::Ready(result) => {
                assert!(store.release(7, job));
                return Ok(result);
            }
            LivingStorePollV2::Failed(error) => {
                assert!(store.release(7, job));
                return Err(error);
            }
        }
    }
}
fn archived_recorded_client(
    root: &std::path::Path,
) -> (LivingClient, nyon::living_store::LivingSlotDescriptorV2) {
    use nyon::living_store::{LivingStoreRequestV2, LivingStoreResultV2};
    let mut client = make_client(root);
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    client
        .save(&builtin_first_expansion_v2().unwrap(), false)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    let slot = client.slot.clone().unwrap();
    client.new_resident().unwrap();
    let mut store = NativeLivingStoreV2::new(root);
    let LivingStoreResultV2::Slot(archived) = store_terminal(
        &mut store,
        LivingStoreRequestV2::SetArchived {
            id: slot.id,
            expected_generation: slot.generation,
            archived: true,
        },
    )
    .unwrap() else {
        panic!()
    };
    (client, archived)
}
#[test]
fn permanent_delete_failed_coordinator_cas_never_begins_authority_deletion() {
    use nyon::living_store::{LivingStoreRequestV2, LivingStoreResultV2};
    let root = tempfile::tempdir().unwrap();
    let (mut client, slot) = archived_recorded_client(root.path());
    let mut competing = make_client(root.path());
    competing.read_selection().unwrap();
    let _ = terminal(&mut competing);
    competing.select_neutral(None).unwrap();
    assert!(matches!(
        terminal(&mut competing),
        LivingClientEvent::SelectionCommitted
    ));
    client
        .mutate(LivingStoreRequestV2::DeleteArchivedSlot {
            id: slot.id,
            expected_generation: slot.generation,
        })
        .unwrap();
    assert!(
        matches!(terminal(&mut client),LivingClientEvent::Failed(error) if error.contains("generation changed"))
    );
    let mut store = NativeLivingStoreV2::new(root.path());
    let LivingStoreResultV2::Slots(slots) =
        store_terminal(&mut store, LivingStoreRequestV2::ListSlots).unwrap()
    else {
        panic!()
    };
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].generation, slot.generation);
}
#[test]
fn permanent_delete_store_failure_leaves_recoverable_slot_and_continue_unset() {
    use nyon::living_store::{LivingGenerationV2, LivingStoreRequestV2, LivingStoreResultV2};
    let root = tempfile::tempdir().unwrap();
    let (mut client, slot) = archived_recorded_client(root.path());
    client
        .mutate(LivingStoreRequestV2::DeleteArchivedSlot {
            id: slot.id,
            expected_generation: LivingGenerationV2(slot.generation.0 - 1),
        })
        .unwrap();
    assert!(
        matches!(terminal(&mut client),LivingClientEvent::Failed(error) if error.contains("stale generation"))
    );
    assert_eq!(client.selection.as_ref().unwrap().target().unwrap(), None);
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut reopened),
        LivingClientEvent::NoSelection
    ));
    let mut store = NativeLivingStoreV2::new(root.path());
    let LivingStoreResultV2::Slots(slots) =
        store_terminal(&mut store, LivingStoreRequestV2::ListSlots).unwrap()
    else {
        panic!()
    };
    assert_eq!(slots.len(), 1);
    assert!(slots[0].archived);
}
#[test]
fn permanent_living_delete_preserves_same_number_workshop_continue() {
    use nyon::living_store::LivingStoreRequestV2;
    let root = tempfile::tempdir().unwrap();
    let (mut client, slot) = archived_recorded_client(root.path());
    let mut target = client
        .selection
        .as_ref()
        .unwrap()
        .target()
        .unwrap()
        .unwrap();
    target.authority = nyon::library_selection::SelectedAuthority::WorkshopV1;
    client.select_neutral(Some(target.clone())).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::SelectionCommitted
    ));
    client
        .mutate(LivingStoreRequestV2::DeleteArchivedSlot {
            id: slot.id,
            expected_generation: slot.generation,
        })
        .unwrap();
    assert!(matches!(terminal(&mut client), LivingClientEvent::Mutated));
    assert_eq!(
        client.selection.as_ref().unwrap().target().unwrap(),
        Some(target.clone())
    );
    let mut reopened = make_client(root.path());
    reopened.read_selection().unwrap();
    assert!(
        matches!(terminal(&mut reopened),LivingClientEvent::WorkshopSelection(actual) if actual==target)
    );
}

fn custom_pack_authority() -> nyon_workshop_core::living::LivingGalaxyAuthorityV2 {
    use nyon_workshop_core::living::*;
    let original = builtin_first_expansion_v2().unwrap();
    let bytes = std::str::from_utf8(LIVING_CORE_PACK_BYTES_V2)
        .unwrap()
        .replace(
            "\"pack_id\":\"nyon_living_core\"",
            "\"pack_id\":\"client_qualification\"",
        );
    let catalog = decode_living_catalog_pack_v2(bytes.as_bytes()).unwrap();
    let value: serde_json::Value =
        serde_json::from_slice(&encode_living_archive_v2(&original).unwrap()).unwrap();
    let manifest: LivingGenesisManifestV2 =
        serde_json::from_value(value["genesis_manifest"].clone()).unwrap();
    let genesis = validate_living_genesis_manifest_v2(manifest, &catalog).unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        original.genesis_seed(),
        original.generator().clone(),
        genesis,
    )
    .unwrap()
}

#[test]
fn pack_client_lifecycle_cas_references_and_continue_isolation() {
    use nyon::app::client_runtime::living::LivingPackMutation;
    use nyon::living_store::LivingStoreRequestV2;
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client.read_selection().unwrap();
    let _ = terminal(&mut client);
    let authority = custom_pack_authority();
    client.save(&authority, false).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    client.save(&authority, false).unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    let slot = client.slot.clone().unwrap();
    client.list_packs().unwrap();
    let LivingClientEvent::PacksListed(packs) = terminal(&mut client) else {
        panic!("packs")
    };
    let original = packs
        .into_iter()
        .find(|p| p.hash == authority.catalog().catalog_hash())
        .unwrap();
    let selected = client.selection.clone();
    assert!(
        client
            .mutate_pack(
                &original,
                LivingPackMutation::SetArchived(true),
                Some(original.hash)
            )
            .is_err()
    );
    client.new_resident().unwrap();
    client
        .mutate_pack(&original, LivingPackMutation::SetArchived(true), None)
        .unwrap();
    assert!(
        matches!(terminal(&mut client), LivingClientEvent::Failed(message) if message.contains("retained galaxy history"))
    );
    assert_eq!(client.selection, selected);
    client
        .mutate(LivingStoreRequestV2::SetArchived {
            id: slot.id,
            expected_generation: slot.generation,
            archived: true,
        })
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Mutated | LivingClientEvent::SelectionCommitted
    ));
    client.list_pack_references(original.hash).unwrap();
    let LivingClientEvent::PackReferencesListed { hash, references } = terminal(&mut client) else {
        panic!("references")
    };
    assert_eq!(hash, original.hash);
    assert_eq!(references.len(), 2);
    assert!(
        references
            .iter()
            .all(|r| r.slot == slot.id && r.name == slot.name && r.archived)
    );
    assert_eq!(references[0].retained_generation, slot.generation);
    assert_eq!(references[1].retained_generation.0, slot.generation.0 + 1);
    assert_eq!(client.pack_references(hash).unwrap(), references);
    let selection_before_pack = client.selection.clone();
    client
        .mutate_pack(&original, LivingPackMutation::SetArchived(true), None)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::PackMutated
    ));
    let archived = client
        .packs()
        .iter()
        .find(|p| p.hash == original.hash)
        .unwrap()
        .clone();
    assert!(archived.archived);
    assert_eq!(archived.generation.0, original.generation.0 + 1);
    assert!(
        client
            .mutate_pack(&original, LivingPackMutation::Delete, None)
            .is_err()
    );
    client
        .mutate_pack(&archived, LivingPackMutation::Delete, None)
        .unwrap();
    assert!(
        matches!(terminal(&mut client), LivingClientEvent::Failed(message) if message.contains("retained galaxy history"))
    );
    assert_eq!(client.selection, selection_before_pack);
    client.list().unwrap();
    let LivingClientEvent::Listed(slots) = terminal(&mut client) else {
        panic!("slots")
    };
    let slot = slots.into_iter().find(|s| s.id == slot.id).unwrap();
    client
        .mutate(LivingStoreRequestV2::DeleteArchivedSlot {
            id: slot.id,
            expected_generation: slot.generation,
        })
        .unwrap();
    assert!(matches!(terminal(&mut client), LivingClientEvent::Mutated));
    client
        .mutate_pack(&archived, LivingPackMutation::Delete, None)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::PackMutated
    ));
    assert!(client.packs().iter().all(|p| p.hash != archived.hash));
    assert_eq!(client.selection, selection_before_pack);
}

#[test]
fn pack_client_protects_builtin_and_rejects_unreviewed_mutation() {
    use nyon::app::client_runtime::living::LivingPackMutation;
    use nyon::living_store::LivingStoreRequestV2;
    let root = tempfile::tempdir().unwrap();
    let mut client = make_client(root.path());
    client
        .save(&builtin_first_expansion_v2().unwrap(), false)
        .unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::Saved { .. }
    ));
    client.list_packs().unwrap();
    let LivingClientEvent::PacksListed(packs) = terminal(&mut client) else {
        panic!("packs")
    };
    let pack = packs.first().unwrap();
    assert!(
        client
            .mutate_pack(pack, LivingPackMutation::SetArchived(true), None)
            .unwrap_err()
            .contains("built-in")
    );
    assert!(
        client
            .mutate(LivingStoreRequestV2::DeletePack {
                hash: pack.hash,
                expected_generation: pack.generation
            })
            .is_err()
    );
    assert!(!client.busy());
}
