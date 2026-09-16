//! The exact-catalog Library client's two generation-conflict paths.
//!
//! Both are addendum §4 obligations that startup Continue can never reach:
//! startup reads the store's own Continue marker, so it carries no separately
//! observed generation and issues no `SelectContinue`. They become reachable
//! the moment a second client opens a row it observed at a generation, which is
//! exactly what `WorkshopLibraryClient` is for.

use nyon::{
    app::client_runtime::library::{
        LibraryBeginError, LibraryEvent, LibraryOpen, WorkshopLibraryClient,
    },
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

#[test]
fn a_second_begin_is_refused_rather_than_stranding_the_first_open() {
    // The precondition is enforced, not documented. Overwriting the phase would
    // drop the job it holds, and nothing else polls it: that wedges the lane for
    // the store's lifetime, which is the exact defect `WorkshopStore::abandon`
    // exists to close. `Selecting` holds the Commit lane specifically, so the
    // cost would be the resident Workshop's ability to save.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0918);
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
    assert!(client.is_active());
    assert_eq!(
        client.begin(&mut store, LibraryOpen::SelectedContinue),
        Err(LibraryBeginError::Active)
    );

    // The refusal is total: the first open is untouched and still completes.
    let event = drive(&mut client, &mut store, |_, _| {});
    let candidate = match event {
        LibraryEvent::Ready(candidate) => candidate,
        other => panic!("the refused second begin disturbed the first: {other:?}"),
    };
    assert_eq!(candidate.loaded.slot, slot);
    assert_eq!(candidate.loaded.generation, generation);

    // Both lanes are free afterwards, which they would not be had the second
    // begin replaced a phase holding a job.
    assert!(
        store
            .start(WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: generation,
                archive: archive(0x0919),
            })
            .is_ok()
    );
    assert!(store.start(WorkshopStoreRequest::ListSlots).is_ok());
}

#[test]
fn a_second_begin_cannot_strand_the_commit_lane_job_that_selecting_holds() {
    // The refusal that actually matters, and the one the previous test cannot
    // reach. A second `SelectedContinue` starts `ListSlots` on the *LoadOrImport*
    // lane, so while the first open sits in `Selecting` -- which holds the
    // *Commit* lane -- the store has no reason to refuse it. Nothing but the
    // client's own guard stops the phase being overwritten and that Commit job
    // stranded, which is precisely the wedge that starves the resident
    // Workshop's save and its recovery-persistence obligation.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0A18);

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

    // Poll 1 completes the load and starts the replay; poll 2 completes the
    // replay and starts the generation-checked selection. Asserted rather than
    // assumed: if the phase shape ever changes, this fails loudly instead of
    // quietly testing a different state than it names.
    assert!(matches!(client.poll(&mut store), LibraryEvent::Pending));
    assert!(matches!(client.poll(&mut store), LibraryEvent::Pending));
    assert!(client.is_active());
    assert!(
        matches!(
            store.start(WorkshopStoreRequest::PutPack {
                canonical_pack: Box::from(
                    &include_bytes!("../assets/workshop/core-pack-v1.json")[..]
                ),
            }),
            Err(nyon::workshop::store::WorkshopStoreError::Busy { .. })
        ),
        "the client must be in Selecting, holding the Commit lane"
    );

    // The LoadOrImport lane is free, so the store would happily accept the
    // second open's `ListSlots`. Only the client refuses.
    assert_eq!(
        client.begin(&mut store, LibraryOpen::SelectedContinue),
        Err(LibraryBeginError::Active)
    );

    // The held Commit job is still reachable and still completes.
    let event = client.poll(&mut store);
    assert!(
        matches!(event, LibraryEvent::Ready(_)),
        "the selection job was stranded: {event:?}"
    );
    assert_eq!(list(&mut store).selected_continue, Some(slot));
    assert!(
        store
            .start(WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: generation,
                archive: archive(0x0A19),
            })
            .is_ok(),
        "the Commit lane must be free once the selection completed"
    );
}

/// Reaches `Selecting`, the only phase that holds the Commit lane, and asserts
/// it. Every abandon test below needs this state and none of them may assume it.
fn drive_to_selecting(
    store: &mut MemoryWorkshopStore,
) -> (WorkshopLibraryClient, SlotId, SaveGeneration) {
    let (slot, generation) = create(store, "Two-System Forge", 0x0B18);
    let mut client = WorkshopLibraryClient::default();
    client
        .begin(
            store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
            },
        )
        .unwrap();
    // Poll 1 completes the load and starts the replay; poll 2 completes the
    // replay and starts the generation-checked selection.
    assert!(matches!(client.poll(store), LibraryEvent::Pending));
    assert!(matches!(client.poll(store), LibraryEvent::Pending));
    assert!(client.is_active());
    assert!(
        matches!(
            store.start(WorkshopStoreRequest::PutPack {
                canonical_pack: Box::from(
                    &include_bytes!("../assets/workshop/core-pack-v1.json")[..]
                ),
            }),
            Err(nyon::workshop::store::WorkshopStoreError::Busy { .. })
        ),
        "the client must be in Selecting, holding the Commit lane"
    );
    (client, slot, generation)
}

#[test]
fn abandoning_a_selecting_open_frees_the_commit_lane_the_phase_held() {
    // Task 4 review Finding 3. `begin` refuses a second open so a job cannot be
    // stranded, but refusing is only half an escape: addendum §6's Retry and
    // Cancel must be able to give up on an in-flight open, and before this the
    // only way out of `Selecting` was to poll it to completion. A user who
    // cancelled instead would wedge the Commit lane for the store's lifetime,
    // starving the resident Workshop's save and its recovery-persistence
    // obligation. The client now spends the store-level `abandon` that
    // `df2457c` added for exactly this.
    let mut store = MemoryWorkshopStore::default();
    let (mut client, slot, generation) = drive_to_selecting(&mut store);

    assert!(
        client.abandon(&mut store),
        "abandoning an active open must report that it gave something up"
    );
    assert!(
        !client.is_active(),
        "abandon must return the client to Idle"
    );

    // The lane is free. This is the assertion the whole slice exists for: with
    // the `store.abandon` call removed it fails with `Busy`.
    let commit = store.start(WorkshopStoreRequest::CommitSlot {
        slot,
        expected_generation: generation,
        archive: archive(0x0B19),
    });
    assert!(
        commit.is_ok(),
        "the Commit lane is still wedged after abandon: {commit:?}"
    );
    // Deliberately not over-read: this succeeds at the *same* generation only
    // because `SelectContinue` does not move the head. The store's contract
    // still holds in general — abandoning a head-moving mutation leaves the
    // caller not knowing the head, and the next head-dependent request is
    // refused with `StaleGeneration` rather than overwriting it — so a caller
    // that abandons must re-list before trusting a generation it held.
    assert!(store.start(WorkshopStoreRequest::ListSlots).is_ok());
}

#[test]
fn abandoning_a_decoding_open_holds_no_store_job_and_still_returns_to_idle() {
    // `Decoding` is the one active phase with no `StoreJobId` at all: the replay
    // is pure CPU. Abandon must handle it without inventing a job to free, and
    // must not report it as inactive merely because there is nothing to abandon
    // in the store — the client state still has to be discarded.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0C18);
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
    assert!(matches!(client.poll(&mut store), LibraryEvent::Pending));
    assert!(client.is_active(), "poll 1 must leave the replay in flight");
    // Both lanes are already free here, which is what makes this case distinct.
    assert!(
        store
            .start(WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: generation,
                archive: archive(0x0C19),
            })
            .is_ok(),
        "Decoding must hold no Commit-lane job"
    );

    assert!(client.abandon(&mut store));
    assert!(!client.is_active());
}

#[test]
fn abandoning_an_idle_client_reports_that_it_gave_nothing_up() {
    // The store's own `abandon` returns `false` for an unknown or already
    // abandoned ID, and the client mirrors that rather than claiming a
    // cancellation it did not perform: a screen that shows "cancelled" off this
    // return value would otherwise lie on a double Cancel.
    let mut store = MemoryWorkshopStore::default();
    let mut client = WorkshopLibraryClient::default();
    assert!(!client.abandon(&mut store), "a fresh client holds nothing");

    let mut store = MemoryWorkshopStore::default();
    let (mut client, _, _) = drive_to_selecting(&mut store);
    assert!(client.abandon(&mut store));
    assert!(
        !client.abandon(&mut store),
        "the second abandon has nothing left to give up"
    );
}

#[test]
fn an_abandoned_client_accepts_a_fresh_begin_and_polls_as_idle() {
    // What Retry actually needs. An abandon that freed the lane but left the
    // client `Active` would trade a wedged store for a wedged client, and a
    // stale event surfacing from the discarded phase would be worse than
    // either: the screen would act on an open the user cancelled.
    let mut store = MemoryWorkshopStore::default();
    let (mut client, slot, generation) = drive_to_selecting(&mut store);
    assert!(client.abandon(&mut store));

    assert!(
        matches!(client.poll(&mut store), LibraryEvent::Pending),
        "the discarded phase must not surface an event"
    );

    client
        .begin(
            &mut store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
            },
        )
        .expect("a fresh open must be accepted after abandon");
    let event = drive(&mut client, &mut store, |_, _| {});
    let candidate = match event {
        LibraryEvent::Ready(candidate) => candidate,
        other => panic!("the retried open did not complete: {other:?}"),
    };
    assert_eq!(candidate.loaded.slot, slot);
    assert_eq!(candidate.loaded.generation, generation);
    assert_eq!(list(&mut store).selected_continue, Some(slot));
}

#[test]
fn abandoning_a_selecting_open_leaves_the_continue_marker_claimed() {
    // Measured, not reasoned about, and it is the one consequence of this
    // escape that a user can see. `Selecting` holds a `SelectContinue` job that
    // the memory adapter has *already executed* by the time `start` returned, so
    // abandoning it calls nothing off: the marker stays pointed at the candidate
    // whose open was cancelled. Consistent with the store's "abandons the
    // outcome, not the work", but a Cancel control that presents itself as
    // "nothing happened" would be lying, and this is the layer to learn that in
    // rather than filing it against the screen in Task 9.
    let mut store = MemoryWorkshopStore::default();
    let (before, before_generation) = create(&mut store, "Predecessor", 0x0D17);
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot: before,
            expected_generation: before_generation,
        },
    );
    assert_eq!(list(&mut store).selected_continue, Some(before));

    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0D18);
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
    assert!(matches!(client.poll(&mut store), LibraryEvent::Pending));
    assert!(matches!(client.poll(&mut store), LibraryEvent::Pending));
    assert!(client.abandon(&mut store));

    assert_eq!(
        list(&mut store).selected_continue,
        Some(slot),
        "the marker is expected to stay claimed by the cancelled open; if this \
         now reports the predecessor, an adapter learned to call the mutation \
         off and the `abandon` doc must be corrected"
    );
    assert_ne!(
        list(&mut store).selected_continue,
        Some(before),
        "abandon must not be read as restoring the previous marker"
    );
}

#[test]
fn a_slot_archived_between_the_list_and_the_load_is_refused_rather_than_opened() {
    // Task 6 review Finding 9, and the ordering that finding names: the archive
    // completes FIRST and the racing open's `LoadSlot` lands after. Addendum §3
    // line 39 is normative -- "an archived row cannot be opened or selected for
    // Continue until it is explicitly unarchived" -- and design line 428 says
    // CONTINUE selects only an unarchived slot, so installing this candidate
    // violates both.
    //
    // The finding called this "structurally unobservable under
    // `MemoryWorkshopStore`". That was true of the code, not of the adapter: the
    // store knew the slot was archived and had no way to say so. The injection
    // point is what makes it reachable. `begin` executes `ListSlots` immediately,
    // so the client is already holding a list snapshot taken while the slot was
    // unarchived; archiving before the first poll means poll 0 passes its
    // `!summary.archived` check against that stale snapshot and starts
    // `LoadSlot`, which memory executes now, against a record that is archived.
    // `ArchiveSlot` is Commit-class and `LoadSlot`/`ListSlots` are LoadOrImport,
    // so the lanes never contend -- which is exactly why nothing refused it.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0E18);
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

    let event = drive(&mut client, &mut store, |step, store| {
        if step == 0 {
            // Archive lands between the list the client already read and the
            // load it has not started. It also clears the Continue marker, so
            // after this there is genuinely no Continue to install.
            complete(store, WorkshopStoreRequest::ArchiveSlot { slot });
        }
    });

    match event {
        LibraryEvent::ArchivedRow { slot: refused } => assert_eq!(refused, slot),
        other => panic!("an archived slot was opened instead of refused: {other:?}"),
    }
    assert_eq!(
        list(&mut store).selected_continue,
        None,
        "ArchiveSlot must have cleared the marker, which is why NoCandidate is \
         the honest bootstrap outcome"
    );
    assert!(
        !client.is_active(),
        "the refusal is terminal; the client must hold nothing"
    );
}

#[test]
fn an_unarchived_slot_still_opens_so_the_refusal_is_not_unconditional() {
    // The control for the test above. A refusal that fired for every load would
    // pass that test while breaking every open, and the two assertions look
    // identical from the outside.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Two-System Forge", 0x0E19);
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    // Archived and then explicitly unarchived: per addendum §3 `UnarchiveSlot`
    // restores openability without restoring Continue, so this also pins that
    // the refusal reads live state rather than a sticky flag.
    complete(&mut store, WorkshopStoreRequest::ArchiveSlot { slot });
    complete(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot });
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
    match event {
        LibraryEvent::Ready(candidate) => assert_eq!(candidate.loaded.slot, slot),
        other => panic!("an unarchived slot must still open: {other:?}"),
    }
}
