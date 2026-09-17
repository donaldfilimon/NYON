//! The one exact-catalog open algorithm, extracted from continue-bootstrap.
//!
//! The Library addendum
//! (`docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md`) §4
//! opens by naming the startup Continue algorithm as *the* canonical open
//! algorithm and requiring that it "must be extracted for reuse, not
//! duplicated". This module is that extraction. Its phases are the former
//! `ContinueBootstrap` arms moved verbatim, so startup Continue runs the same
//! algorithm it ran before rather than a re-derivation of it.
//!
//! What the client owns is the algorithm and its typed outcomes. What it
//! deliberately does not own is screen policy: it never enters recovery, never
//! decides which screen to return to, and never installs a session. It reports
//! a [`LibraryEvent`] and [`super::ClientRuntime`] decides. That boundary is
//! what lets the Library present Retry over a live runtime rather than
//! collapsing into the global recovery screen, which §6 forbids.
//!
//! Startup Continue drives it through [`LibraryOpen::SelectedContinue`];
//! Library Open, Use for Continue (route-design tasks 12a and 12b) and row
//! Export (12c) drive it through [`LibraryOpen::Slot`], whose [`SlotIntent`]
//! says whether a validated head claims the Continue marker. §4 assigns all
//! three to this client and specifies their conflict handling here.

use crate::workshop::{
    ArchiveDecodeJob, ArchiveDecodeStatus, CatalogHash, ValidatedCatalogPackV1, WorkshopHistory,
    archive_catalog_hash, decode_catalog_pack, encode_catalog_pack,
    store::{
        LoadedSlot, SaveGeneration, SlotId, StoreJobId, StoreJobState, WorkshopStore,
        WorkshopStoreError, WorkshopStoreRequest, WorkshopStoreResult,
    },
};

use super::{
    ARCHIVE_REPLAY_UNITS_PER_UPDATE, CORE_PACK_V1, ClientDiagnostic, ClientDiagnosticCode,
};

/// What the caller asked the client to open.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryOpen {
    /// Startup Continue. The slot is read from the store's own Continue
    /// marker, so there is no separately observed generation to conflict with
    /// and no marker to re-select: the marker being current is the premise.
    SelectedContinue,
    /// An explicit Library activation of one row, carrying the head
    /// generation that row displayed. §4 requires the compare against that
    /// observed generation for every intent; `intent` decides what happens
    /// after the replay.
    Slot {
        slot: SlotId,
        expected_generation: SaveGeneration,
        intent: SlotIntent,
    },
}

/// What a [`LibraryOpen::Slot`] is for, as far as the store is concerned.
///
/// Made explicit in the variant rather than implicit in a private flag
/// (task 4 review Finding 2): a mode that always selects Continue, reused for
/// row Export, would compile, pass a naive test, and move the Continue marker
/// on an export.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotIntent {
    /// Library Open and Use for Continue: a clean head claims the Continue
    /// marker by compare-and-swap before it is reported ready, and an
    /// archived row is refused (§3).
    Open,
    /// Row Export: a pure read. §4 says it "does not mutate storage, select
    /// Continue, or install/replace a session", so no `SelectContinue` is
    /// ever started, and an archived row is **not** refused — §3 forbids
    /// opening or selecting an archived save, not reading it (task 7 review
    /// F5).
    Export,
}

/// A fully replayed, fully validated candidate. Nothing reaches this type that
/// has not decoded against its exact declared catalog and replayed to the end.
#[derive(Debug)]
pub struct LibraryCandidate {
    pub loaded: LoadedSlot,
    pub history: WorkshopHistory,
}

/// Why a [`WorkshopLibraryClient::begin`] was refused.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LibraryBeginError {
    /// An open is already in flight. Starting a second one would overwrite the
    /// phase holding the first one's job, and nothing would ever poll it: that
    /// wedges its lane for the store's lifetime, and `Selecting` holds the
    /// Commit lane specifically. Enforced rather than documented, because this
    /// is the exact defect class [`WorkshopStore::abandon`] exists to close.
    #[error("a Workshop Library open is already active")]
    Active,
    #[error("Workshop storage rejected the request: {0}")]
    Store(#[from] WorkshopStoreError),
}

/// One poll's outcome.
#[derive(Debug)]
pub enum LibraryEvent {
    /// Idle, or still working. The caller does nothing.
    Pending,
    /// There is no Continue marker to open. An ordinary outcome, not a fault.
    NoCandidate,
    /// A validated candidate. `loaded.recovered_from_previous` distinguishes a
    /// clean head from the same slot's retained predecessor; the client does
    /// not decide what that means for the screen.
    Ready(Box<LibraryCandidate>),
    /// A store or content failure. The client is idle again and holds nothing.
    Failed(ClientDiagnostic),
    /// The store broke its own protocol. Always fatal to the candidate.
    ProtocolFailure(&'static str),
    /// §4 item 1: the slot's head no longer matches the generation the
    /// activated row observed. Nothing was replayed, so there is no candidate
    /// to preserve; the remedy is to re-list and retry.
    StaleRow {
        slot: SlotId,
        observed: SaveGeneration,
        head: SaveGeneration,
    },
    /// The slot was archived by the time the load read it, so it may not be
    /// opened. Addendum §3: "an archived row cannot be opened or selected for
    /// Continue until it is explicitly unarchived", and design line 428 has
    /// `CONTINUE` selecting only an unarchived slot.
    ///
    /// Deliberately **not** a [`Self::ProtocolFailure`], which the `Listing`
    /// arm does use for an archived slot. There the store's own Continue marker
    /// pointed at an archived row, which means `ArchiveSlot` failed to clear it
    /// and the store broke its invariant. This is the legitimate case: an
    /// `ArchiveSlot` completed between the list and the load, on a lane that
    /// does not contend with either. Conflating them would file a race as a
    /// store bug.
    ArchivedRow { slot: SlotId },
    /// §4: the candidate replayed completely, but the head moved before the
    /// Continue marker could be set, so the compare-and-swap was refused. The
    /// validated candidate is preserved rather than discarded, and the stored
    /// marker is left exactly where it was: a conflict "never silently selects
    /// a newer head".
    ContinueConflict { candidate: Box<LibraryCandidate> },
}

enum Phase {
    Idle,
    Listing(StoreJobId),
    Loading {
        job: StoreJobId,
        selected_slot: SlotId,
    },
    LoadingPrevious {
        job: StoreJobId,
        selected_slot: SlotId,
        original_failure: ClientDiagnostic,
    },
    LoadingCatalog {
        job: StoreJobId,
        selected_slot: SlotId,
        loaded: LoadedSlot,
        expected_hash: CatalogHash,
    },
    Decoding {
        loaded: LoadedSlot,
        decoder: Box<ArchiveDecodeJob>,
    },
    Selecting {
        job: StoreJobId,
        candidate: Box<LibraryCandidate>,
    },
}

/// The exact-catalog open algorithm as a bounded, poll-driven state machine.
pub struct WorkshopLibraryClient {
    phase: Phase,
    /// `Some` only for an explicit row activation. Startup Continue has no
    /// separately observed generation, which is why migrating it onto this
    /// client cannot change its behavior.
    expected_generation: Option<SaveGeneration>,
    /// Whether a validated non-recovered candidate must claim the Continue
    /// marker before it may be installed.
    selects_continue: bool,
    /// Whether an archived row is refused. False only for row Export.
    refuses_archived: bool,
}

impl Default for WorkshopLibraryClient {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            expected_generation: None,
            selects_continue: false,
            refuses_archived: true,
        }
    }
}

impl WorkshopLibraryClient {
    pub const fn is_active(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }

    /// Starts one open.
    ///
    /// Refused while one is already in flight, so a caller cannot strand the
    /// job the current phase holds. The caller owns the failure policy for the
    /// initial `start`, so a store rejection is returned rather than absorbed.
    pub fn begin(
        &mut self,
        store: &mut impl WorkshopStore,
        open: LibraryOpen,
    ) -> Result<(), LibraryBeginError> {
        if self.is_active() {
            return Err(LibraryBeginError::Active);
        }
        match open {
            LibraryOpen::SelectedContinue => {
                let job = store.start(WorkshopStoreRequest::ListSlots)?;
                self.expected_generation = None;
                self.selects_continue = false;
                self.refuses_archived = true;
                self.phase = Phase::Listing(job);
            }
            LibraryOpen::Slot {
                slot,
                expected_generation,
                intent,
            } => {
                let job = store.start(WorkshopStoreRequest::LoadSlot { slot })?;
                self.expected_generation = Some(expected_generation);
                self.selects_continue = intent == SlotIntent::Open;
                self.refuses_archived = intent == SlotIntent::Open;
                self.phase = Phase::Loading {
                    job,
                    selected_slot: slot,
                };
            }
        }
        Ok(())
    }

    /// Gives up on the open in flight, freeing the store lane its phase holds,
    /// and returns whether there was one.
    ///
    /// This is the other half of the [`LibraryBeginError::Active`] guard.
    /// Refusing a second `begin` stops a job being stranded by a *caller
    /// mistake*; this is the route out for a caller that has deliberately
    /// stopped wanting the answer, which is what addendum §6's Retry and Cancel
    /// are. Without it the only exit from [`Phase::Selecting`] was to poll it to
    /// completion, so a cancelled open wedged the Commit lane for the store's
    /// lifetime and starved the resident Workshop's save.
    ///
    /// **It abandons the outcome, not the work**, inheriting every consequence
    /// [`WorkshopStore::abandon`] documents. In particular, abandoning from
    /// `Selecting` gives up a head-dependent mutation, so any generation the
    /// caller was holding is no longer known to be current: **re-list before
    /// trusting one.** That is safe rather than corrupting only because every
    /// head-dependent mutation compares and swaps.
    ///
    /// **The consequence a Cancel control must absorb: abandoning from
    /// `Selecting` can leave the Continue marker CLAIMED for the open that was
    /// cancelled.** The memory and browser adapters have executed the
    /// `SelectContinue` by the time `start` returned, so there is nothing left
    /// to call off — `selected_continue` is already the abandoned candidate's
    /// slot, and it stays there. This is not corruption and not a defect in
    /// this method; it is what "abandons the outcome, not the work" means for
    /// this particular request, measured rather than reasoned about and pinned
    /// by `abandoning_a_selecting_open_leaves_the_continue_marker_claimed`.
    /// A screen that must present Cancel as *nothing happened* has to select
    /// the previous marker back itself, and cannot do it from a generation it
    /// held before abandoning.
    ///
    /// The validated candidate in `Selecting` is **discarded**, unlike
    /// [`LibraryEvent::ContinueConflict`], which preserves it. The distinction
    /// is who decided to stop: a conflict is the store refusing work the caller
    /// still wants, while this is the caller withdrawing. A candidate-preserving
    /// cancel would be a different method, and there is no screen to consume one
    /// yet.
    ///
    /// Returns `false` for an already-idle client, mirroring the store's own
    /// no-op return rather than claiming a cancellation that did not happen.
    pub fn abandon(&mut self, store: &mut impl WorkshopStore) -> bool {
        // Taking the phase unconditionally is what guarantees no discarded phase
        // can surface an event from a later `poll`.
        match std::mem::replace(&mut self.phase, Phase::Idle) {
            Phase::Idle => false,
            Phase::Listing(job)
            | Phase::Loading { job, .. }
            | Phase::LoadingPrevious { job, .. }
            | Phase::LoadingCatalog { job, .. }
            | Phase::Selecting { job, .. } => {
                // The store's return is deliberately not propagated. It reports
                // whether *that ID* held a lane, which is already false for a
                // job whose result landed before this call; the client is
                // answering the different question of whether it gave an open
                // up, and it did.
                store.abandon(job);
                true
            }
            // The replay is pure CPU and holds no job, so there is nothing to
            // free in the store -- but the open was still active and dropping
            // the decoder is still giving it up.
            Phase::Decoding { .. } => true,
        }
        // `expected_generation`, `selects_continue` and `refuses_archived` are
        // deliberately left as they are, and the reason is checkable rather
        // than a symmetry argument: all three are read only from inside a phase
        // (the `Loading` generation check, the archived refusal and the
        // selection decision), every phase originates in a `begin`, and `begin`
        // assigns all three on both of its arms. An `Idle` client never reads
        // either, so a stale value cannot be observed.
    }

    /// Advances one bounded step.
    pub fn poll(&mut self, store: &mut impl WorkshopStore) -> LibraryEvent {
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        match phase {
            Phase::Idle => LibraryEvent::Pending,
            Phase::Listing(job) => match store.poll(job) {
                StoreJobState::Pending => {
                    self.phase = Phase::Listing(job);
                    LibraryEvent::Pending
                }
                StoreJobState::Unknown => LibraryEvent::ProtocolFailure(
                    "Workshop storage forgot the active slot-list job",
                ),
                StoreJobState::Complete(Err(error)) => LibraryEvent::Failed(store_failure(&error)),
                StoreJobState::Complete(Ok(WorkshopStoreResult::Slots(list))) => {
                    let Some(selected_slot) = list.selected_continue else {
                        return LibraryEvent::NoCandidate;
                    };
                    let selected_is_valid = list.slots.iter().any(|summary| {
                        summary.id == selected_slot
                            && summary.selected_for_continue
                            && !summary.archived
                    });
                    if !selected_is_valid {
                        return LibraryEvent::ProtocolFailure(
                            "The explicitly selected Continue slot is missing or archived",
                        );
                    }
                    match store.start(WorkshopStoreRequest::LoadSlot {
                        slot: selected_slot,
                    }) {
                        Ok(job) => {
                            self.phase = Phase::Loading { job, selected_slot };
                            LibraryEvent::Pending
                        }
                        Err(error) => LibraryEvent::Failed(store_failure(&error)),
                    }
                }
                StoreJobState::Complete(Ok(_)) => LibraryEvent::ProtocolFailure(
                    "Workshop slot-list job returned an unexpected result",
                ),
            },
            Phase::Loading { job, selected_slot } => match store.poll(job) {
                StoreJobState::Pending => {
                    self.phase = Phase::Loading { job, selected_slot };
                    LibraryEvent::Pending
                }
                StoreJobState::Unknown => LibraryEvent::ProtocolFailure(
                    "Workshop storage forgot the active Continue load job",
                ),
                StoreJobState::Complete(Err(error)) => LibraryEvent::Failed(store_failure(&error)),
                StoreJobState::Complete(Ok(WorkshopStoreResult::SlotLoaded(loaded))) => {
                    if loaded.slot != selected_slot {
                        return LibraryEvent::ProtocolFailure(
                            "Workshop storage loaded a slot other than the selected Continue slot",
                        );
                    }
                    // §4 item 1, applied at the earliest point it can be: the
                    // row's observed generation is compared before any pack is
                    // fetched or any byte replayed, so a superseded row costs
                    // one load rather than a full replay.
                    if let Some(observed) = self.expected_generation
                        && loaded.head_generation != observed
                    {
                        return LibraryEvent::StaleRow {
                            slot: loaded.slot,
                            observed,
                            head: loaded.head_generation,
                        };
                    }
                    self.prepare_loaded(store, selected_slot, loaded)
                }
                StoreJobState::Complete(Ok(_)) => LibraryEvent::ProtocolFailure(
                    "Workshop Continue load returned an unexpected result",
                ),
            },
            Phase::LoadingPrevious {
                job,
                selected_slot,
                original_failure,
            } => match store.poll(job) {
                StoreJobState::Pending => {
                    self.phase = Phase::LoadingPrevious {
                        job,
                        selected_slot,
                        original_failure,
                    };
                    LibraryEvent::Pending
                }
                StoreJobState::Unknown => LibraryEvent::ProtocolFailure(
                    "Workshop storage forgot the previous-generation load job",
                ),
                StoreJobState::Complete(Err(_)) => LibraryEvent::Failed(original_failure),
                StoreJobState::Complete(Ok(WorkshopStoreResult::SlotLoaded(loaded))) => {
                    if loaded.slot != selected_slot || !loaded.recovered_from_previous {
                        return LibraryEvent::ProtocolFailure(
                            "Workshop storage returned an invalid previous-generation load",
                        );
                    }
                    self.prepare_loaded(store, selected_slot, loaded)
                }
                StoreJobState::Complete(Ok(_)) => LibraryEvent::ProtocolFailure(
                    "Workshop previous-generation load returned an unexpected result",
                ),
            },
            Phase::LoadingCatalog {
                job,
                selected_slot,
                loaded,
                expected_hash,
            } => match store.poll(job) {
                StoreJobState::Pending => {
                    self.phase = Phase::LoadingCatalog {
                        job,
                        selected_slot,
                        loaded,
                        expected_hash,
                    };
                    LibraryEvent::Pending
                }
                StoreJobState::Unknown => LibraryEvent::ProtocolFailure(
                    "Workshop storage forgot the active catalog load job",
                ),
                StoreJobState::Complete(Err(error)) => {
                    let failure = store_failure(&error);
                    self.recover_previous_or_fail(store, loaded, failure)
                }
                StoreJobState::Complete(Ok(WorkshopStoreResult::PackLoaded {
                    hash,
                    canonical_pack,
                })) => {
                    if loaded.slot != selected_slot || hash != expected_hash {
                        return LibraryEvent::ProtocolFailure(
                            "Workshop storage returned a catalog other than the archive's exact catalog",
                        );
                    }
                    let catalog = match decode_catalog_pack(&canonical_pack) {
                        Ok(catalog) => catalog,
                        Err(_) => {
                            return self.recover_previous_or_fail(
                                store,
                                loaded,
                                diagnostic(
                                    ClientDiagnosticCode::Catalog,
                                    "The saved Workshop catalog failed validation",
                                ),
                            );
                        }
                    };
                    let canonical = match encode_catalog_pack(&catalog) {
                        Ok(canonical) => canonical,
                        Err(_) => {
                            return self.recover_previous_or_fail(
                                store,
                                loaded,
                                diagnostic(
                                    ClientDiagnosticCode::Catalog,
                                    "The saved Workshop catalog could not be canonicalized",
                                ),
                            );
                        }
                    };
                    if catalog.catalog_hash() != expected_hash
                        || canonical.as_slice() != canonical_pack.as_ref()
                    {
                        return self.recover_previous_or_fail(
                            store,
                            loaded,
                            diagnostic(
                                ClientDiagnosticCode::Catalog,
                                "The saved Workshop catalog was noncanonical or had the wrong hash",
                            ),
                        );
                    }
                    self.begin_replay(store, loaded, catalog)
                }
                StoreJobState::Complete(Ok(_)) => LibraryEvent::ProtocolFailure(
                    "Workshop catalog load returned an unexpected result",
                ),
            },
            Phase::Decoding {
                loaded,
                mut decoder,
            } => match decoder.poll(ARCHIVE_REPLAY_UNITS_PER_UPDATE) {
                Ok(ArchiveDecodeStatus::Pending) => {
                    self.phase = Phase::Decoding { loaded, decoder };
                    LibraryEvent::Pending
                }
                Ok(ArchiveDecodeStatus::Complete) => match (*decoder).finish() {
                    Ok(history) => self.finish_replay(store, loaded, history),
                    Err(error) => {
                        let failure = diagnostic(ClientDiagnosticCode::Archive, error.to_string());
                        self.recover_previous_or_fail(store, loaded, failure)
                    }
                },
                Err(error) => {
                    let failure = diagnostic(ClientDiagnosticCode::Archive, error.to_string());
                    self.recover_previous_or_fail(store, loaded, failure)
                }
            },
            Phase::Selecting { job, candidate } => match store.poll(job) {
                StoreJobState::Pending => {
                    self.phase = Phase::Selecting { job, candidate };
                    LibraryEvent::Pending
                }
                StoreJobState::Unknown => LibraryEvent::ProtocolFailure(
                    "Workshop storage forgot the active Continue selection job",
                ),
                // A refused compare-and-swap is the addendum's conflict, not a
                // failure: the candidate is validated and stays validated, and
                // the stored marker is untouched. Every other store error is an
                // ordinary failure.
                StoreJobState::Complete(Err(WorkshopStoreError::StaleGeneration { .. })) => {
                    LibraryEvent::ContinueConflict { candidate }
                }
                StoreJobState::Complete(Err(error)) => LibraryEvent::Failed(store_failure(&error)),
                StoreJobState::Complete(Ok(WorkshopStoreResult::ContinueSelected {
                    slot,
                    generation,
                })) => {
                    if slot != candidate.loaded.slot || generation != candidate.loaded.generation {
                        return LibraryEvent::ProtocolFailure(
                            "Workshop storage selected a Continue generation other than the validated one",
                        );
                    }
                    LibraryEvent::Ready(candidate)
                }
                StoreJobState::Complete(Ok(_)) => LibraryEvent::ProtocolFailure(
                    "Workshop Continue selection returned an unexpected result",
                ),
            },
        }
    }

    fn prepare_loaded(
        &mut self,
        store: &mut impl WorkshopStore,
        selected_slot: SlotId,
        loaded: LoadedSlot,
    ) -> LibraryEvent {
        if loaded.recovered_from_previous != (loaded.generation != loaded.head_generation) {
            return LibraryEvent::ProtocolFailure(
                "Workshop storage returned inconsistent loaded and head generations",
            );
        }
        // Task 6 review Finding 9. Checked here rather than in either load arm
        // because both funnel through this function, so the head load and the
        // retained-predecessor recovery load are covered by one refusal instead
        // of two that could drift. Checked before the catalog is decoded and
        // before a byte is replayed, so an archived row costs one load. Row
        // Export reads archived rows, so it skips the refusal.
        if self.refuses_archived && loaded.archived {
            return LibraryEvent::ArchivedRow { slot: loaded.slot };
        }
        let catalog = match decode_catalog_pack(CORE_PACK_V1) {
            Ok(catalog) => catalog,
            Err(_) => {
                return LibraryEvent::Failed(diagnostic(
                    ClientDiagnosticCode::Catalog,
                    "The built-in Workshop catalog failed validation",
                ));
            }
        };
        let expected_hash = match archive_catalog_hash(&loaded.archive) {
            Ok(hash) => hash,
            Err(error) => {
                let failure = diagnostic(ClientDiagnosticCode::Archive, error.to_string());
                return self.recover_previous_or_fail(store, loaded, failure);
            }
        };
        if catalog.catalog_hash() == expected_hash {
            return self.begin_replay(store, loaded, catalog);
        }
        match store.start(WorkshopStoreRequest::GetPack {
            hash: expected_hash,
        }) {
            Ok(job) => {
                self.phase = Phase::LoadingCatalog {
                    job,
                    selected_slot,
                    loaded,
                    expected_hash,
                };
                LibraryEvent::Pending
            }
            Err(error) => {
                let failure = store_failure(&error);
                self.recover_previous_or_fail(store, loaded, failure)
            }
        }
    }

    /// §4 item 8: only the same slot's retained predecessor is ever offered,
    /// and only when the head itself was the thing that failed.
    fn recover_previous_or_fail(
        &mut self,
        store: &mut impl WorkshopStore,
        loaded: LoadedSlot,
        failure: ClientDiagnostic,
    ) -> LibraryEvent {
        if loaded.recovered_from_previous {
            return LibraryEvent::Failed(failure);
        }
        let slot = loaded.slot;
        match store.start(WorkshopStoreRequest::LoadPreviousGeneration {
            slot,
            expected_head_generation: loaded.head_generation,
        }) {
            Ok(job) => {
                self.phase = Phase::LoadingPrevious {
                    job,
                    selected_slot: slot,
                    original_failure: failure,
                };
                LibraryEvent::Pending
            }
            Err(error) => LibraryEvent::Failed(store_failure(&error)),
        }
    }

    fn begin_replay(
        &mut self,
        store: &mut impl WorkshopStore,
        loaded: LoadedSlot,
        catalog: ValidatedCatalogPackV1,
    ) -> LibraryEvent {
        match ArchiveDecodeJob::new(&catalog, &loaded.archive) {
            Ok(decoder) => {
                self.phase = Phase::Decoding {
                    loaded,
                    decoder: Box::new(decoder),
                };
                LibraryEvent::Pending
            }
            Err(error) => {
                let failure = diagnostic(ClientDiagnosticCode::Archive, error.to_string());
                self.recover_previous_or_fail(store, loaded, failure)
            }
        }
    }

    fn finish_replay(
        &mut self,
        store: &mut impl WorkshopStore,
        loaded: LoadedSlot,
        history: WorkshopHistory,
    ) -> LibraryEvent {
        // A recovered predecessor never claims the marker here. §4's recovery
        // order is promote, *then* select at the new head N+2, and the
        // promotion is a separate user acceptance the client does not own.
        // Selecting the predecessor's generation now would mark a Continue
        // target the store is about to supersede.
        let candidate = Box::new(LibraryCandidate { loaded, history });
        if !self.selects_continue || candidate.loaded.recovered_from_previous {
            return LibraryEvent::Ready(candidate);
        }
        match store.start(WorkshopStoreRequest::SelectContinue {
            slot: candidate.loaded.slot,
            expected_generation: candidate.loaded.generation,
        }) {
            Ok(job) => {
                self.phase = Phase::Selecting { job, candidate };
                LibraryEvent::Pending
            }
            Err(error) => LibraryEvent::Failed(store_failure(&error)),
        }
    }
}

fn diagnostic(code: ClientDiagnosticCode, message: impl Into<String>) -> ClientDiagnostic {
    ClientDiagnostic {
        code,
        message: message.into(),
    }
}

fn store_failure(error: &WorkshopStoreError) -> ClientDiagnostic {
    diagnostic(ClientDiagnosticCode::Store, error.to_string())
}
