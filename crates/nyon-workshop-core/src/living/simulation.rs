//! Living Galaxy V2 creator queue and atomic boundary orchestration.
//!
//! Section 2 of
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` is binding.
//! The authority publishes a cursor of committed revision, completed tick and
//! pending tail; a submission must name all three exactly or it is stale. A
//! running envelope is validated against a private projection that already
//! holds every earlier accepted envelope for the same boundary, and a paused
//! envelope applies synchronously at the current boundary with an empty queue.
//!
//! A step clones the state, runs the ten published phases in registry order on
//! the clone, validates it, hashes it, seals the receipt, and only then swaps
//! every authority field at once. A fault leaves the state, the revision log
//! and the queue exactly as they were and pauses the authority with the typed
//! fault retained for diagnosis.
//!
//! # What this module decides rather than transcribes
//!
//! - Phases 2 through 9 are hooks with no behavior yet. Their rules belong to
//!   the civilizations plan, which modifies this file to fill them; they run
//!   here, in order and on their published cadence, so the boundary shape is
//!   fixed before any rule lands.
//! - Phase 1 applies the creator operations whose effect on a record is a
//!   direct field mapping: the six topology, civilization and deposit creations
//!   and the inventory, deposit and policy edits. Every other operation is
//!   refused at submission with [`LivingCommandRejectionV2::UnsupportedOperation`]
//!   rather than accepted and skipped, so no accepted revision can silently
//!   fail to mean what it says.
//! - Each applied revision emits exactly one `creator_intervention` event whose
//!   provenance names that revision.
//! - A state's historical `accepted_sequence` is the sequence of the last
//!   revision applied into it, which replay reproduces from the revision log
//!   alone. The live allocator mark is kept separately and never hashed.
//! - A lane is stored with its lower-identified endpoint first whatever order
//!   the batch named them in, because a batch creating both systems cannot know
//!   their identities' order before its revision exists.

use super::catalog::ValidatedLivingCatalogPackV2;
use super::command::{
    LivingAcceptedCommandV2, LivingCommandEnvelopeV2, LivingCommandErrorV2, LivingCommandModeV2,
    LivingCommandTargetV2, LivingCreatorOperationV2, LivingRevisionV2,
};
use super::genesis::{LivingGenesisGeneratorV2, ValidatedLivingGenesisV2};
use super::ids::{
    LIVING_PHASE_REGISTRY_V2, LivingBranchIdV2, LivingEntityIdV2, LivingPhaseV2,
    LivingRevisionIdV2, LivingTickV2,
};
use super::model::{
    LivingCivilizationV2, LivingDepositV2, LivingGalaxyStateV2, LivingLaneV2, LivingStarV2,
    LivingSystemV2, LivingValidationErrorV2, LivingWorldV2,
};
use super::receipt::{
    LivingEventKindV2, LivingEventProvenanceV2, LivingPendingEventsV2, LivingReceiptErrorV2,
    LivingTickReceiptV2, seal_living_tick_receipt_v2,
};
use super::wire::{LivingKeyedV2, LivingSortedVecV2, LivingWireErrorV2};

/// The deepest the running queue may grow before one boundary drains it.
///
/// Section 1 sets it to the declared work-unit poll bound, so one boundary's
/// queue can never exceed one poll's budget.
pub const LIVING_MAX_PENDING_QUEUE_DEPTH_V2: usize = 1_024;

/// Diplomacy runs on multiples of this many ticks (section 2, step 7).
const DIPLOMACY_PERIOD_TICKS: u64 = 100;

/// Intent generation runs on multiples of this many ticks (section 2, step 8).
const INTENT_PERIOD_TICKS: u64 = 50;

/// What the authority publishes for submitters to name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingCommandCursorV2 {
    /// The last committed revision, or `None` before any exists.
    pub committed_revision: Option<LivingRevisionIdV2>,
    /// The completed boundary.
    pub tick: LivingTickV2,
    /// The accepted sequence at the queue tail, or zero for an empty queue.
    pub pending_sequence: u64,
}

/// Why an authority could not be started.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingAuthorityErrorV2 {
    /// The genesis was validated under a different catalog than the one given.
    #[error("the Living V2 genesis was validated under a different catalog")]
    CatalogMismatch,
    /// The genesis state does not validate under the catalog.
    #[error("the Living V2 genesis state is invalid: {0}")]
    Validation(#[from] LivingValidationErrorV2),
}

/// Every way a submission can be refused.
///
/// A refused submission consumes no sequence, no entity identity, no queue
/// position and no resource.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingCommandRejectionV2 {
    /// The envelope named a committed revision other than the published one.
    #[error("the Living V2 envelope expects a committed revision that is not current")]
    StaleRevision {
        /// What the envelope named.
        expected: Option<LivingRevisionIdV2>,
        /// What the authority publishes.
        found: Option<LivingRevisionIdV2>,
    },
    /// The envelope named a completed tick other than the published one.
    #[error("the Living V2 envelope expects tick {expected:?}; the authority is at {found:?}")]
    StaleTick {
        /// What the envelope named.
        expected: LivingTickV2,
        /// What the authority publishes.
        found: LivingTickV2,
    },
    /// The envelope named a queue tail other than the published one.
    #[error("the Living V2 envelope expects queue tail {expected}; the tail is {found}")]
    StalePendingSequence {
        /// What the envelope named.
        expected: u64,
        /// What the authority publishes.
        found: u64,
    },
    /// The envelope's mode does not match the entry point it was given to.
    #[error("a {found:?} Living V2 envelope was given to the {expected:?} entry point")]
    ModeMismatch {
        /// The mode the entry point accepts.
        expected: LivingCommandModeV2,
        /// The mode the envelope carries.
        found: LivingCommandModeV2,
    },
    /// Paused application requires an empty running queue.
    #[error("a paused Living V2 edit requires an empty running queue")]
    PausedWithRunningQueue,
    /// The running queue is already at its declared depth.
    #[error("the Living V2 running queue holds at most {limit} envelopes")]
    QueueFull {
        /// The declared depth.
        limit: usize,
    },
    /// The authority is paused on a deterministic fault.
    #[error("the Living V2 authority is paused on a fault")]
    Faulted,
    /// The document-global sequence or the boundary counter is exhausted.
    #[error("the Living V2 sequence or tick counter is exhausted")]
    Exhausted,
    /// The batch failed its structural checks.
    #[error("the Living V2 command is structurally invalid: {0}")]
    Command(#[from] LivingCommandErrorV2),
    /// This slice does not yet apply the operation at this batch position.
    #[error("Living V2 operation {operation} is not applied by this rules slice")]
    UnsupportedOperation {
        /// Position of the operation in its batch.
        operation: usize,
    },
    /// An edit names an existing entity the projection does not hold.
    #[error("Living V2 operation {operation} names a missing {reference}")]
    MissingTarget {
        /// Position of the operation in its batch.
        operation: usize,
        /// What kind of record was expected.
        reference: &'static str,
    },
    /// The projection with this command applied does not validate.
    #[error("the Living V2 projection is invalid: {0}")]
    Projection(#[from] LivingValidationErrorV2),
    /// The committed paused edit could not be hashed or sealed.
    #[error("the Living V2 paused edit could not be sealed: {0}")]
    Seal(LivingDeterministicFaultV2),
}

/// A failure inside a boundary. The boundary commits nothing.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingDeterministicFaultV2 {
    /// An internal invariant did not hold.
    #[error("a Living V2 authority invariant failed")]
    Invariant,
    /// A capacity reservation could not be honored.
    #[error("a Living V2 capacity reservation failed")]
    Capacity,
    /// Checked arithmetic overflowed.
    #[error("Living V2 arithmetic overflowed")]
    Arithmetic,
    /// A queued envelope no longer reproduces its validated result.
    #[error("queued Living V2 envelope {accepted_sequence} did not reproduce: {reason}")]
    Replay {
        /// The envelope's accepted sequence.
        accepted_sequence: u64,
        /// Why it failed, as the submission path would have reported it.
        reason: Box<LivingCommandRejectionV2>,
    },
    /// The candidate state failed validation before commit.
    #[error("the Living V2 candidate state is invalid: {0}")]
    Validation(#[from] LivingValidationErrorV2),
    /// The candidate state could not be encoded.
    #[error("the Living V2 candidate state is not canonical: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// The receipt could not be sealed.
    #[error("the Living V2 receipt could not be sealed: {0}")]
    Receipt(#[from] LivingReceiptErrorV2),
}

/// The mutable view a phase receives of one boundary's candidate.
///
/// Phase code reaches the candidate only through this context and emits events
/// only through [`Self::emit`], which assigns the ordinal.
#[allow(dead_code)] // The civilization phase modules are the readers.
pub(crate) struct LivingStepContextV2<'a> {
    pub(crate) catalog: &'a ValidatedLivingCatalogPackV2,
    pub(crate) genesis_seed: [u8; 32],
    pub(crate) branch: LivingBranchIdV2,
    pub(crate) tick: LivingTickV2,
    pub(crate) state: &'a mut LivingGalaxyStateV2,
    pub(crate) events: &'a mut LivingPendingEventsV2,
}

impl LivingStepContextV2<'_> {
    /// Record one event of this boundary.
    pub(crate) fn emit(
        &mut self,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(provenance, kind)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct QueuedCommandV2 {
    accepted: LivingAcceptedCommandV2,
    revision: LivingRevisionV2,
}

/// The Living Galaxy V2 authority for one active view.
#[derive(Clone, Debug)]
pub struct LivingGalaxyAuthorityV2 {
    catalog: ValidatedLivingCatalogPackV2,
    genesis_seed: [u8; 32],
    generator: LivingGenesisGeneratorV2,
    branch: LivingBranchIdV2,
    state: LivingGalaxyStateV2,
    committed_revision: Option<LivingRevisionIdV2>,
    revisions: Vec<LivingRevisionV2>,
    queue: Vec<QueuedCommandV2>,
    accepted_sequence_high_water: u64,
    fault: Option<LivingDeterministicFaultV2>,
}

impl LivingGalaxyAuthorityV2 {
    /// Start an authority at a validated genesis.
    pub fn from_genesis(
        catalog: ValidatedLivingCatalogPackV2,
        seed: [u8; 32],
        generator: LivingGenesisGeneratorV2,
        genesis: ValidatedLivingGenesisV2,
    ) -> Result<Self, LivingAuthorityErrorV2> {
        if genesis.catalog_hash() != catalog.catalog_hash() {
            return Err(LivingAuthorityErrorV2::CatalogMismatch);
        }
        let state = genesis.state().clone();
        state.validate(&catalog)?;
        Ok(Self {
            branch: genesis.root_branch_id(seed),
            catalog,
            genesis_seed: seed,
            generator,
            state,
            committed_revision: None,
            revisions: Vec::new(),
            queue: Vec::new(),
            accepted_sequence_high_water: 0,
            fault: None,
        })
    }

    /// The committed state.
    pub const fn state(&self) -> &LivingGalaxyStateV2 {
        &self.state
    }

    /// The catalog this authority is bound to.
    pub const fn catalog(&self) -> &ValidatedLivingCatalogPackV2 {
        &self.catalog
    }

    /// The generator provenance of this authority's genesis.
    pub const fn generator(&self) -> &LivingGenesisGeneratorV2 {
        &self.generator
    }

    /// The branch this authority advances.
    pub const fn branch(&self) -> LivingBranchIdV2 {
        self.branch
    }

    /// Committed revisions in application order.
    pub fn revisions(&self) -> &[LivingRevisionV2] {
        &self.revisions
    }

    /// The fault the authority is paused on, if any.
    pub const fn fault(&self) -> Option<&LivingDeterministicFaultV2> {
        self.fault.as_ref()
    }

    /// The durable allocator mark: the last accepted sequence ever issued,
    /// including sequences a retained faulted queue has spent.
    pub const fn accepted_sequence_high_water(&self) -> u64 {
        self.accepted_sequence_high_water
    }

    /// The cursor a submitter must name.
    pub fn published_cursor(&self) -> LivingCommandCursorV2 {
        LivingCommandCursorV2 {
            committed_revision: self.committed_revision,
            tick: self.state.tick,
            pending_sequence: self
                .queue
                .last()
                .map_or(0, |queued| queued.accepted.accepted_sequence),
        }
    }

    /// Queue a running envelope for the next boundary.
    pub fn submit(
        &mut self,
        envelope: LivingCommandEnvelopeV2,
    ) -> Result<LivingAcceptedCommandV2, LivingCommandRejectionV2> {
        self.check_admission(&envelope, LivingCommandModeV2::Running)?;
        if self.queue.len() >= LIVING_MAX_PENDING_QUEUE_DEPTH_V2 {
            return Err(LivingCommandRejectionV2::QueueFull {
                limit: LIVING_MAX_PENDING_QUEUE_DEPTH_V2,
            });
        }
        let application_tick = self
            .state
            .tick
            .0
            .checked_add(1)
            .map(LivingTickV2)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let parent = self
            .queue
            .last()
            .map(|queued| queued.revision.id)
            .or(self.committed_revision);
        let (accepted, revision) = self.seal(envelope, application_tick, parent)?;

        let mut projection = self.state.clone();
        for queued in &self.queue {
            apply_revision(&mut projection, &queued.revision)?;
        }
        apply_revision(&mut projection, &revision)?;
        projection.validate(&self.catalog)?;

        self.accepted_sequence_high_water = accepted.accepted_sequence;
        self.queue.push(QueuedCommandV2 {
            accepted: accepted.clone(),
            revision,
        });
        Ok(accepted)
    }

    /// Apply a paused envelope synchronously at the current boundary.
    pub fn apply_paused(
        &mut self,
        envelope: LivingCommandEnvelopeV2,
    ) -> Result<LivingTickReceiptV2, LivingCommandRejectionV2> {
        self.check_admission(&envelope, LivingCommandModeV2::Paused)?;
        if !self.queue.is_empty() {
            return Err(LivingCommandRejectionV2::PausedWithRunningQueue);
        }
        let tick = self.state.tick;
        let (_, revision) = self.seal(envelope, tick, self.committed_revision)?;

        let mut candidate = self.state.clone();
        apply_revision(&mut candidate, &revision)?;
        candidate.validate(&self.catalog)?;

        let mut events = LivingPendingEventsV2::new();
        let mut seal = || -> Result<LivingTickReceiptV2, LivingDeterministicFaultV2> {
            events.record(
                LivingEventProvenanceV2::Creator {
                    revision: revision.id,
                },
                LivingEventKindV2::CreatorIntervention,
            )?;
            let digest = candidate.digest()?;
            Ok(seal_living_tick_receipt_v2(
                tick,
                vec![revision.id],
                &events,
                digest,
            )?)
        };
        let receipt = seal().map_err(LivingCommandRejectionV2::Seal)?;

        // Everything that can fail has run. Commit every field together.
        self.accepted_sequence_high_water = revision.ordinal;
        self.committed_revision = Some(revision.id);
        self.revisions.push(revision);
        self.state = candidate;
        Ok(receipt)
    }

    /// Advance one boundary, committing everything or nothing.
    pub fn step(&mut self) -> Result<LivingTickReceiptV2, LivingDeterministicFaultV2> {
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        match self.run_boundary() {
            Ok((candidate, receipt)) => {
                // The only mutation of a successful boundary, after every
                // fallible step has run.
                let applied = std::mem::take(&mut self.queue);
                if let Some(last) = applied.last() {
                    self.committed_revision = Some(last.revision.id);
                }
                self.revisions
                    .extend(applied.into_iter().map(|queued| queued.revision));
                self.state = candidate;
                Ok(receipt)
            }
            Err(fault) => {
                self.fault = Some(fault.clone());
                Err(fault)
            }
        }
    }

    /// Discard a queue retained by a fault and resume.
    ///
    /// The sequences the discarded envelopes were issued stay spent: the
    /// allocator mark does not move back.
    pub fn discard_faulted_queue(&mut self) {
        if self.fault.take().is_some() {
            self.queue.clear();
        }
    }

    fn check_admission(
        &self,
        envelope: &LivingCommandEnvelopeV2,
        mode: LivingCommandModeV2,
    ) -> Result<(), LivingCommandRejectionV2> {
        if self.fault.is_some() {
            return Err(LivingCommandRejectionV2::Faulted);
        }
        if envelope.mode != mode {
            return Err(LivingCommandRejectionV2::ModeMismatch {
                expected: mode,
                found: envelope.mode,
            });
        }
        let cursor = self.published_cursor();
        if envelope.expected_committed_revision != cursor.committed_revision {
            return Err(LivingCommandRejectionV2::StaleRevision {
                expected: envelope.expected_committed_revision,
                found: cursor.committed_revision,
            });
        }
        if envelope.expected_tick != cursor.tick {
            return Err(LivingCommandRejectionV2::StaleTick {
                expected: envelope.expected_tick,
                found: cursor.tick,
            });
        }
        if envelope.expected_pending_sequence != cursor.pending_sequence {
            return Err(LivingCommandRejectionV2::StalePendingSequence {
                expected: envelope.expected_pending_sequence,
                found: cursor.pending_sequence,
            });
        }
        Ok(())
    }

    /// Allocate the next sequence tentatively and seal the revision. Nothing
    /// is recorded until the caller commits.
    fn seal(
        &self,
        envelope: LivingCommandEnvelopeV2,
        application_tick: LivingTickV2,
        parent: Option<LivingRevisionIdV2>,
    ) -> Result<(LivingAcceptedCommandV2, LivingRevisionV2), LivingCommandRejectionV2> {
        envelope.command.validate()?;
        for (operation, op) in envelope.command.operations().iter().enumerate() {
            if !is_applied_by_this_slice(op) {
                return Err(LivingCommandRejectionV2::UnsupportedOperation { operation });
            }
        }
        let accepted_sequence = self
            .accepted_sequence_high_water
            .checked_add(1)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let accepted = LivingAcceptedCommandV2 {
            accepted_sequence,
            application_tick,
            envelope,
        };
        let revision = LivingRevisionV2::seal(
            self.catalog.catalog_hash(),
            self.genesis_seed,
            parent,
            &accepted,
        )?;
        Ok((accepted, revision))
    }

    fn run_boundary(
        &self,
    ) -> Result<(LivingGalaxyStateV2, LivingTickReceiptV2), LivingDeterministicFaultV2> {
        let tick = self
            .state
            .tick
            .0
            .checked_add(1)
            .map(LivingTickV2)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        let mut candidate = self.state.clone();
        let mut events = LivingPendingEventsV2::new();
        let mut context = LivingStepContextV2 {
            catalog: &self.catalog,
            genesis_seed: self.genesis_seed,
            branch: self.branch,
            tick,
            state: &mut candidate,
            events: &mut events,
        };

        for (phase, _) in LIVING_PHASE_REGISTRY_V2 {
            match phase {
                LivingPhaseV2::ApplyCreatorInterventions => {
                    apply_queued_interventions(&mut context, &self.queue)?;
                }
                LivingPhaseV2::ExpireLifecycles => advance_lifecycles(&mut context)?,
                LivingPhaseV2::ResolveArrivals => resolve_arrivals(&mut context)?,
                LivingPhaseV2::ResolveConflict => {
                    resolve_combat_occupation_and_claims(&mut context)?;
                }
                LivingPhaseV2::RunProduction => complete_jobs_and_run_recipes(&mut context)?,
                LivingPhaseV2::RefreshObservations => refresh_observations(&mut context)?,
                LivingPhaseV2::UpdateDiplomacy => {
                    if tick.0 % DIPLOMACY_PERIOD_TICKS == 0 {
                        update_diplomacy(&mut context)?;
                    }
                }
                LivingPhaseV2::GenerateIntents => {
                    if tick.0 % INTENT_PERIOD_TICKS == 0 {
                        generate_and_reserve_intents(&mut context)?;
                    }
                }
                LivingPhaseV2::DispatchOrders => dispatch_orders_and_freight(&mut context)?,
                LivingPhaseV2::CommitBoundary => context.state.tick = tick,
            }
        }

        candidate.validate(&self.catalog)?;
        let digest = candidate.digest()?;
        let mut applied: Vec<LivingRevisionIdV2> =
            self.queue.iter().map(|queued| queued.revision.id).collect();
        applied.sort_unstable();
        let receipt = seal_living_tick_receipt_v2(tick, applied, &events, digest)?;
        Ok((candidate, receipt))
    }
}

// ------------------------------------------------------------------ phases

/// Phase 1: replay the accepted queue into the candidate in sequence order.
fn apply_queued_interventions(
    ctx: &mut LivingStepContextV2<'_>,
    queue: &[QueuedCommandV2],
) -> Result<(), LivingDeterministicFaultV2> {
    for queued in queue {
        apply_revision(ctx.state, &queued.revision).map_err(|reason| {
            LivingDeterministicFaultV2::Replay {
                accepted_sequence: queued.accepted.accepted_sequence,
                reason: Box::new(reason),
            }
        })?;
        ctx.emit(
            LivingEventProvenanceV2::Creator {
                revision: queued.revision.id,
            },
            LivingEventKindV2::CreatorIntervention,
        )?;
    }
    Ok(())
}

// Phases 2 through 9 are supplied by the civilizations plan, which replaces
// these bodies without changing their signatures or their call order.

fn advance_lifecycles(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn resolve_arrivals(_ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn resolve_combat_occupation_and_claims(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn complete_jobs_and_run_recipes(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn refresh_observations(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn update_diplomacy(_ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn generate_and_reserve_intents(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

fn dispatch_orders_and_freight(
    _ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    Ok(())
}

// ------------------------------------------------------ creator application

fn is_applied_by_this_slice(operation: &LivingCreatorOperationV2) -> bool {
    matches!(
        operation,
        LivingCreatorOperationV2::CreateSystem { .. }
            | LivingCreatorOperationV2::CreateStar { .. }
            | LivingCreatorOperationV2::CreateWorld { .. }
            | LivingCreatorOperationV2::CreateLane { .. }
            | LivingCreatorOperationV2::CreateCivilization { .. }
            | LivingCreatorOperationV2::CreateDeposit { .. }
            | LivingCreatorOperationV2::SetWorldInventory { .. }
            | LivingCreatorOperationV2::SetDepositRemaining { .. }
            | LivingCreatorOperationV2::SetCivilizationPolicy { .. }
    )
}

/// Apply one revision's operations to `state`, in declaration order, and
/// record the revision's sequence as the state's historical sequence.
///
/// References to existing entities that a creation names are left for
/// [`LivingGalaxyStateV2::validate`] to resolve, so this is not a second
/// validator. An edit must find its record here, because it has nothing to
/// write into otherwise.
fn apply_revision(
    state: &mut LivingGalaxyStateV2,
    revision: &LivingRevisionV2,
) -> Result<(), LivingCommandRejectionV2> {
    let created = revision.created_entities();
    let resolve = |target: &LivingCommandTargetV2| -> LivingEntityIdV2 {
        match *target {
            LivingCommandTargetV2::Existing { id } => id,
            LivingCommandTargetV2::Local { local } => created
                .iter()
                .find(|(declared, _, _)| *declared == local)
                .map(|(_, _, id)| *id)
                // `LivingCommandV2::validate` already proved every local
                // reference names an earlier declaration.
                .unwrap_or(LivingEntityIdV2([0; 16])),
        }
    };
    let created_id =
        |local: u16| -> LivingEntityIdV2 { resolve(&LivingCommandTargetV2::Local { local }) };

    for (index, operation) in revision.command.operations().iter().enumerate() {
        match operation {
            LivingCreatorOperationV2::CreateSystem { local, name } => {
                insert(
                    &mut state.systems,
                    LivingSystemV2 {
                        id: created_id(*local),
                        name: name.clone(),
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateStar {
                local,
                system,
                archetype,
                name,
            } => {
                insert(
                    &mut state.stars,
                    LivingStarV2 {
                        id: created_id(*local),
                        system: resolve(system),
                        archetype: archetype.clone(),
                        name: name.clone(),
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateWorld {
                local,
                system,
                archetype,
                name,
                owner,
                inventory,
            } => {
                insert(
                    &mut state.worlds,
                    LivingWorldV2 {
                        id: created_id(*local),
                        system: resolve(system),
                        archetype: archetype.clone(),
                        name: name.clone(),
                        owner: owner.as_ref().map(resolve),
                        inventory: *inventory,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateLane {
                local,
                system_a,
                system_b,
                distance_units,
            } => {
                let first = resolve(system_a);
                let second = resolve(system_b);
                insert(
                    &mut state.lanes,
                    LivingLaneV2 {
                        id: created_id(*local),
                        system_a: first.min(second),
                        system_b: first.max(second),
                        distance_units: *distance_units,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateCivilization {
                local,
                name,
                policy,
                status,
            } => {
                insert(
                    &mut state.civilizations,
                    LivingCivilizationV2 {
                        id: created_id(*local),
                        name: name.clone(),
                        policy: policy.clone(),
                        status: *status,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateDeposit {
                local,
                world,
                resource,
                remaining_units,
            } => {
                insert(
                    &mut state.deposits,
                    LivingDepositV2 {
                        id: created_id(*local),
                        world: resolve(world),
                        resource: *resource,
                        remaining_units: *remaining_units,
                    },
                )?;
            }
            LivingCreatorOperationV2::SetWorldInventory { world, inventory } => {
                edit(&mut state.worlds, resolve(world), index, "world")?.inventory = *inventory;
            }
            LivingCreatorOperationV2::SetDepositRemaining {
                deposit,
                remaining_units,
            } => {
                edit(&mut state.deposits, resolve(deposit), index, "deposit")?.remaining_units =
                    *remaining_units;
            }
            LivingCreatorOperationV2::SetCivilizationPolicy {
                civilization,
                policy,
            } => {
                edit(
                    &mut state.civilizations,
                    resolve(civilization),
                    index,
                    "civilization",
                )?
                .policy = policy.clone();
            }
            _ => {
                return Err(LivingCommandRejectionV2::UnsupportedOperation { operation: index });
            }
        }
    }
    state.accepted_sequence = revision.ordinal;
    Ok(())
}

/// Insert one record at its sorted position. A repeated identity is a
/// validation failure, reported the way the state validator reports it.
fn insert<T>(
    collection: &mut LivingSortedVecV2<T>,
    record: T,
) -> Result<(), LivingValidationErrorV2>
where
    T: LivingKeyedV2,
{
    let mut records = std::mem::take(collection).into_vec();
    let key = record.living_key();
    let position = records.partition_point(|existing| existing.living_key() < key);
    if records
        .get(position)
        .is_some_and(|existing| existing.living_key() == key)
    {
        *collection = LivingSortedVecV2::new(records)?;
        return Err(LivingValidationErrorV2::DuplicateIdentity {
            context: "creator batch",
        });
    }
    records.insert(position, record);
    *collection = LivingSortedVecV2::new(records)?;
    Ok(())
}

fn edit<'a, T>(
    collection: &'a mut LivingSortedVecV2<T>,
    id: LivingEntityIdV2,
    operation: usize,
    reference: &'static str,
) -> Result<&'a mut T, LivingCommandRejectionV2>
where
    T: LivingKeyedV2<Key = LivingEntityIdV2>,
{
    collection
        .find_mut(&id)
        .ok_or(LivingCommandRejectionV2::MissingTarget {
            operation,
            reference,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::LIVING_RULES_VERSION;
    use crate::living::catalog::{LivingNameV2, LivingSlugV2, living_core_pack_v2};
    use crate::living::command::LivingCommandV2;
    use crate::living::genesis::{
        LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LivingGenesisManifestV2,
        validate_living_genesis_manifest_v2,
    };

    fn authority() -> LivingGalaxyAuthorityV2 {
        let catalog = living_core_pack_v2().unwrap();
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state: LivingGalaxyStateV2::default(),
            },
            &catalog,
        )
        .unwrap();
        LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            [0x33; 32],
            LivingGenesisGeneratorV2 {
                generator_id: LivingSlugV2::new("unit").unwrap(),
                generator_version: 1,
            },
            genesis,
        )
        .unwrap()
    }

    fn submit(authority: &mut LivingGalaxyAuthorityV2, operations: Vec<LivingCreatorOperationV2>) {
        let cursor = authority.published_cursor();
        authority
            .submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch { operations },
            })
            .unwrap();
    }

    fn system(text: &str) -> LivingCreatorOperationV2 {
        LivingCreatorOperationV2::CreateSystem {
            local: 0,
            name: LivingNameV2::new(text).unwrap(),
        }
    }

    /// Make the committed state disagree with what the queued envelopes were
    /// validated against. No public path can do this; it stands in for any
    /// defect that makes a supposedly accepted envelope stop reproducing.
    fn corrupt_committed_state(authority: &mut LivingGalaxyAuthorityV2) {
        authority.state.systems = LivingSortedVecV2::default();
    }

    #[test]
    fn a_fault_commits_nothing_retains_the_queue_and_pauses() {
        let mut authority = authority();
        submit(&mut authority, vec![system("Vale")]);
        authority.step().unwrap();
        let vale = authority.state.systems.as_slice()[0].id;

        submit(&mut authority, vec![system("Outer")]);
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: LivingCommandTargetV2::Existing { id: vale },
                archetype: LivingSlugV2::new("red_dwarf").unwrap(),
                name: LivingNameV2::new("Ember").unwrap(),
            }],
        );
        corrupt_committed_state(&mut authority);
        let state_before = authority.state.clone();
        let cursor_before = authority.published_cursor();
        let revisions_before = authority.revisions.clone();

        let fault = authority.step().unwrap_err();
        assert!(
            matches!(
                fault,
                LivingDeterministicFaultV2::Validation(
                    LivingValidationErrorV2::DanglingReference { .. }
                )
            ),
            "{fault:?}"
        );
        assert_eq!(authority.state, state_before, "no partial state");
        assert_eq!(
            authority.revisions, revisions_before,
            "no partial revisions"
        );
        assert_eq!(
            authority.published_cursor(),
            cursor_before,
            "the queue is retained for diagnosis"
        );
        assert_eq!(authority.fault(), Some(&fault));

        // Paused: stepping repeats the fault and submission is refused.
        assert_eq!(authority.step().unwrap_err(), fault);
        let cursor = authority.published_cursor();
        let refused = authority.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: Vec::new(),
            },
        });
        assert_eq!(refused, Err(LivingCommandRejectionV2::Faulted));

        // Discarding the queue resumes without reissuing its sequences.
        assert_eq!(authority.accepted_sequence_high_water(), 3);
        authority.discard_faulted_queue();
        assert!(authority.fault().is_none());
        assert_eq!(authority.published_cursor().pending_sequence, 0);
        submit(&mut authority, Vec::new());
        assert_eq!(authority.published_cursor().pending_sequence, 4);
        authority.step().unwrap();
        assert_eq!(authority.state.accepted_sequence, 4);
    }

    #[test]
    fn an_edit_whose_target_vanished_faults_as_a_replay_failure() {
        let mut authority = authority();
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: 0,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::CreateCivilization {
                        local: 0,
                        name: LivingNameV2::new("Kin").unwrap(),
                        policy: crate::living::model::LivingPolicyV2(
                            LivingSlugV2::new("neutral").unwrap(),
                        ),
                        status: crate::living::model::LivingCivilizationStatusV2::Dormant,
                    }],
                },
            })
            .unwrap();
        let kin = authority.state.civilizations.as_slice()[0].id;
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::SetCivilizationPolicy {
                civilization: LivingCommandTargetV2::Existing { id: kin },
                policy: crate::living::model::LivingPolicyV2(LivingSlugV2::new("trader").unwrap()),
            }],
        );
        authority.state.civilizations = LivingSortedVecV2::default();
        let fault = authority.step().unwrap_err();
        assert!(
            matches!(
                &fault,
                LivingDeterministicFaultV2::Replay {
                    accepted_sequence: 2,
                    reason,
                } if matches!(**reason, LivingCommandRejectionV2::MissingTarget { .. })
            ),
            "{fault:?}"
        );
        assert_eq!(authority.published_cursor().pending_sequence, 2);
    }

    #[test]
    fn phases_run_in_registry_order_and_the_emit_path_assigns_ordinals() {
        let catalog = living_core_pack_v2().unwrap();
        let mut state = LivingGalaxyStateV2::default();
        let mut events = LivingPendingEventsV2::new();
        let mut context = LivingStepContextV2 {
            catalog: &catalog,
            genesis_seed: [0; 32],
            branch: LivingBranchIdV2([0; 16]),
            tick: LivingTickV2(1),
            state: &mut state,
            events: &mut events,
        };
        for kind in [
            LivingEventKindV2::HazardStarted,
            LivingEventKindV2::HazardEnded,
        ] {
            context
                .emit(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ExpireLifecycles,
                    },
                    kind,
                )
                .unwrap();
        }
        let ordinals: Vec<u16> = events.payloads().iter().map(|row| row.ordinal).collect();
        assert_eq!(ordinals, vec![0, 1]);

        let order: Vec<u16> = LIVING_PHASE_REGISTRY_V2.iter().map(|row| row.1).collect();
        assert_eq!(order, (1..=10).collect::<Vec<_>>());
    }
}
