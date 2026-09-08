//! The exact-catalog Library client's two generation-conflict paths.
//!
//! Both are addendum §4 obligations that startup Continue can never reach:
//! startup reads the store's own Continue marker, so it carries no separately
//! observed generation and issues no `SelectContinue`. They become reachable
//! the moment a second client opens a row it observed at a generation, which is
//! exactly what `WorkshopLibraryClient` is for.

use nyon::{
    app::client_runtime::library::{LibraryEvent, LibraryOpen, WorkshopLibraryClient},
    workshop::{
        WorkshopHistory, decode_catalog_pack, encode_archive,
        store::{
            MemoryWorkshopStore, SaveGeneration, SlotId, SlotList, SlotName, StoreJobState,
            WorkshopStore, WorkshopStoreRequest, WorkshopStoreResult,
        },
    },
};

fn archive(seed: u64) -> Box<[u8]> {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    encode_archive(&WorkshopHistory::from_seed_u64(catalog, seed))
        .unwrap()
        .bytes
        .into_boxed_slice()
}

fn complete(store: &mut MemoryWorkshopStore, request: WorkshopStoreRequest) -> WorkshopStoreResult {
    let job = store.start(request).unwrap();
    match store.poll(job) {
        StoreJobState::Complete(Ok(result)) => result,
        state => panic!("unexpected Workshop store state: {state:?}"),
    }
}

fn create(store: &mut MemoryWorkshopStore, name: &str, seed: u64) -> (SlotId, SaveGeneration) {
    let created = complete(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new(name).unwrap(),
            archive: archive(seed),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    (slot, generation)
}

fn list(store: &mut MemoryWorkshopStore) -> SlotList {
    match complete(store, WorkshopStoreRequest::ListSlots) {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    }
}

/// Polls until the client leaves `Pending`, running `during` before each poll so
/// a test can inject a concurrent commit at a chosen phase.
fn drive(
    client: &mut WorkshopLibraryClient,
    store: &mut MemoryWorkshopStore,
    mut during: impl FnMut(usize, &mut MemoryWorkshopStore),
) -> LibraryEvent {
    for step in 0..32 {
        during(step, store);
        match client.poll(store) {
            LibraryEvent::Pending => {}
            event => return event,
        }
    }
    panic!("the Library client exceeded its bounded test polls");
}

#[test]
fn an_open_of_a_superseded_row_reports_a_stale_row_before_replaying_anything() {
    // §4 item 1: "compare the returned head generation with the row's observed
    // generation, and report a refreshable conflict instead of silently
    // accepting a newer head". The row was observed at generation 1 and the
    // head is 2 by the time the open lands.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0417);
    let committed = complete(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: archive(0x0418),
        },
    );
    let WorkshopStoreResult::SlotCommitted {
        generation: head, ..
    } = committed
    else {
        panic!("unexpected commit result: {committed:?}");
    };

    let mut client = WorkshopLibraryClient::default();
    client
        .begin(
            &mut store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
            },
        )
        .unwrap();
    let event = drive(&mut client, &mut store, |_, _| {});

    match event {
        LibraryEvent::StaleRow {
            slot: conflicted,
            observed,
            head: reported,
        } => {
            assert_eq!(conflicted, slot);
            assert_eq!(observed, generation);
            assert_eq!(reported, head);
        }
        other => panic!("a superseded row must not open: {other:?}"),
    }
    // The client is idle and holds nothing: there is no candidate to preserve
    // because nothing was replayed. The remedy is to re-list and retry.
    assert!(!client.is_active());
    // Nothing was written on the way out either.
    let slots = list(&mut store);
    assert_eq!(slots.selected_continue, None);
    assert_eq!(slots.slots[0].generation, head);
}

#[test]
fn a_commit_racing_a_validated_open_refuses_the_marker_and_keeps_the_candidate() {
    // Addendum §12's race qualification: "row generation N followed by a
    // concurrent N+1 commit before Open/Use/Export". The commit lands after the
    // row's generation check has already passed and while the archive is
    // replaying, so only the generation-checked `SelectContinue` can catch it.
    //
    // A second slot holds the Continue marker throughout, which is what makes
    // "the marker did not move" an assertion with teeth: had the client
    // accepted the newer head, the marker would name the opened slot instead.
    let mut store = MemoryWorkshopStore::default();
    let (other, other_generation) = create(&mut store, "Continue Target", 0x0517);
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot: other,
            expected_generation: other_generation,
        },
    );
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0518);

    let mut client = WorkshopLibraryClient::default();
    client
        .begin(
            &mut store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
            },
        )
        .unwrap();

    // Step 0 is the load poll and holds the LoadOrImport lane; from step 1 the
    // client is replaying and the Commit lane is free, which is precisely the
    // window the addendum names.
    let mut raced = false;
    let event = drive(&mut client, &mut store, |step, store| {
        if step == 1 && !raced {
            raced = true;
            let committed = complete(
                store,
                WorkshopStoreRequest::CommitSlot {
                    slot,
                    expected_generation: generation,
                    archive: archive(0x0519),
                },
            );
            assert!(
                matches!(committed, WorkshopStoreResult::SlotCommitted { .. }),
                "the racing commit must land: {committed:?}"
            );
        }
    });
    assert!(raced, "the racing commit never ran");

    let candidate = match event {
        LibraryEvent::ContinueConflict { candidate } => candidate,
        other => panic!("the superseded head must not be selected: {other:?}"),
    };
    // §4: "A conflict preserves the resident session and validated candidate".
    // The candidate is the generation the user actually chose, fully replayed,
    // not the one the race produced.
    assert_eq!(candidate.loaded.slot, slot);
    assert_eq!(candidate.loaded.generation, generation);
    assert!(!candidate.loaded.recovered_from_previous);
    assert_eq!(
        candidate.history.genesis_seed()[..8],
        0x0518_u64.to_le_bytes()
    );
    assert!(!client.is_active());

    // The marker never moved. `CommitSlot` clears any marker naming the slot it
    // advances, and this commit named a different slot, so a marker now
    // pointing at `slot` could only have come from the client accepting a head
    // it never validated.
    let slots = list(&mut store);
    assert_eq!(slots.selected_continue, Some(other));
    let opened = slots.slots.iter().find(|row| row.id == slot).unwrap();
    assert!(!opened.selected_for_continue);
    assert_eq!(opened.generation, SaveGeneration(generation.0 + 1));
}

#[test]
fn an_open_at_the_current_head_selects_that_exact_generation_for_continue() {
    // The success half of the same path, so the conflict tests above cannot
    // pass by never selecting at all. §4: "Explicit Open also selects that exact
    // validated generation for Continue before session installation."
    let mut store = MemoryWorkshopStore::default();
    let (other, other_generation) = create(&mut store, "Continue Target", 0x0617);
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot: other,
            expected_generation: other_generation,
        },
    );
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0618);

    let mut client = WorkshopLibraryClient::default();
    client
        .begin(
            &mut store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
            },
        )
        .unwrap();
    let event = drive(&mut client, &mut store, |_, _| {});

    let candidate = match event {
        LibraryEvent::Ready(candidate) => candidate,
        other => panic!("an open at the current head must succeed: {other:?}"),
    };
    assert_eq!(candidate.loaded.slot, slot);
    assert_eq!(candidate.loaded.generation, generation);
    assert_eq!(
        candidate.history.genesis_seed()[..8],
        0x0618_u64.to_le_bytes()
    );

    let slots = list(&mut store);
    assert_eq!(slots.selected_continue, Some(slot));
    let opened = slots.slots.iter().find(|row| row.id == slot).unwrap();
    assert!(opened.selected_for_continue);
    assert_eq!(opened.generation, generation);
}

#[test]
fn startup_continue_carries_no_observed_generation_and_claims_no_marker() {
    // The migration's own contract. `LibraryOpen::SelectedContinue` reads the
    // store's marker, so it can produce neither conflict, and it must not
    // re-select: the addendum's automatic save sequence owns that, and startup
    // "exposes no selected Continue candidate rather than opening a generation
    // that was never explicitly selected".
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0718);
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );

    let mut client = WorkshopLibraryClient::default();
    client
        .begin(&mut store, LibraryOpen::SelectedContinue)
        .unwrap();
    let event = drive(&mut client, &mut store, |_, _| {});

    let candidate = match event {
        LibraryEvent::Ready(candidate) => candidate,
        other => panic!("startup Continue must open the marked slot: {other:?}"),
    };
    assert_eq!(candidate.loaded.slot, slot);
    assert_eq!(candidate.loaded.generation, generation);
    assert!(!client.is_active());

    // Unchanged, not re-written: the marker is exactly where the store already
    // had it, and no Commit lane job was ever started on this path.
    let slots = list(&mut store);
    assert_eq!(slots.selected_continue, Some(slot));
    assert_eq!(slots.slots[0].generation, generation);
}

#[test]
fn a_slot_with_no_continue_marker_reports_no_candidate_rather_than_guessing() {
    let mut store = MemoryWorkshopStore::default();
    create(&mut store, "Two-System Forge", 0x0818);

    let mut client = WorkshopLibraryClient::default();
    client
        .begin(&mut store, LibraryOpen::SelectedContinue)
        .unwrap();
    assert!(matches!(
        drive(&mut client, &mut store, |_, _| {}),
        LibraryEvent::NoCandidate
    ));
    assert!(!client.is_active());
    assert_eq!(list(&mut store).selected_continue, None);
}
