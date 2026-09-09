//! Living Galaxy V2 creator commands, their envelopes, and the immutable
//! revisions they become.
//!
//! Section 2 of
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` is binding:
//! "a multi-operation atomic edit is represented by one batch
//! command/envelope", every envelope carries the three expectation fields the
//! authority compares against its published cursor, and each accepted envelope
//! becomes exactly one revision whose identity is hashed over the envelope's
//! canonical command bytes.
//!
//! Section 10 adds the identity rule this module exists to make unavoidable:
//! entities a creator batch creates are named by
//! `SHA256(domain || revision_id_32 || entity_kind_u16 || batch_local_id_u16)`,
//! where the kind comes from the published registry in
//! [`super::ids`] and the local identifier is the position the batch declares.
//! [`LivingCommandV2::created_entities`] is the only place those two inputs are
//! paired, so a caller cannot invent an identity by choosing its own kind.
//!
//! # Local identifiers and reference direction
//!
//! Section 10: "Batch-local IDs start at zero and are unique within the
//! command." This module reads that as the strictest form it can have and still
//! be deterministic -- the creating operations declare `0, 1, 2, …` in
//! declaration order, with no gaps -- because a gap is unobservable in the
//! resulting identities and would leave two batches that create the same
//! entities indistinguishable in review yet different on the wire.
//!
//! A reference may name an existing entity or an earlier operation's local
//! identifier, never a later one and never its own. Validation is a pure
//! structural pass over the batch; whether the referenced *existing* entity is
//! present in a state is submission-time work against a projection, which the
//! queue task owns.
//!
//! # What this module decides rather than transcribes
//!
//! Section 8 lists the subjects a creator command covers -- topology and
//! archetypes, deposits, inventories, colonies and ownership, facilities and
//! queues, civilization policies and base relations, agreements, fleets and
//! orders, routes, scheduled hazards, and explicit cascades -- without fixing
//! an operation schema. The variants below are this task's schema for that
//! list, and their field order is normative in the same way every other Living
//! V2 record's is: it is hashed, through the canonical command bytes, into
//! every revision identity. `tests/living_command.rs` pins it against bytes
//! written from this declaration rather than read back out of the encoder.

use serde::{Deserialize, Serialize};

use super::catalog::{LivingNameV2, LivingSlugV2};
use super::ids::{
    LivingCatalogHashV2, LivingEntityIdV2, LivingEntityKindV2, LivingRevisionIdV2, LivingTickV2,
    creator_entity_id_v2, revision_id_v2,
};
use super::model::{
    LivingAgreementKindV2, LivingCivilizationStatusV2, LivingFacilityStatusV2,
    LivingFleetOrderKindV2, LivingInventoryV2, LivingPolicyV2, LivingResourceV2, LivingRouteKindV2,
    LivingShipmentDispositionV2,
};
use super::wire::{LIVING_MAX_ARCHIVE_BYTES_V2, LivingWireErrorV2, encode_canonical_v2};

/// The largest number of operations one creator batch may carry.
///
/// Section 1 sets this to the declared work-unit poll bound rather than to a
/// round number, so one batch can never exceed one poll's budget, and requires
/// the rejection to happen "before any sequence, ordinal, entity ID, queue
/// mutation, or resource is consumed".
pub const LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2: usize = 1_024;

/// What an operation points at: an entity that already exists, or an entity an
/// earlier operation of the same batch creates.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingCommandTargetV2 {
    /// An entity created earlier in this same batch.
    Local {
        /// That operation's batch-local identifier.
        local: u16,
    },
    /// An entity that already exists in the state.
    Existing {
        /// Its stable identity.
        id: LivingEntityIdV2,
    },
}

/// What a removal does about the target's dependents.
///
/// Section 8: "Deleting a world or civilization with dependents requires an
/// explicit reviewed cascade; reject ambiguous deletion." Neither variant is a
/// default, which is the point: the author states which one they reviewed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingCascadeDispositionV2 {
    /// Remove the listed dependents with the target. Every dependent must be
    /// named; an empty list is the ambiguous deletion the spec rejects.
    RemoveListed {
        /// The dependents this removal also removes.
        dependents: Vec<LivingCommandTargetV2>,
    },
    /// Refuse the removal if the target has any dependent at all.
    RejectIfReferenced,
}

/// One operation of a creator batch.
///
/// The fourteen creating variants correspond one to one with the published
/// entity-kind registry; every other variant edits or removes something that
/// already exists.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingCreatorOperationV2 {
    /// Create a star system.
    CreateSystem {
        /// Batch-local identifier.
        local: u16,
        /// Display name.
        name: LivingNameV2,
    },
    /// Create a star inside a system.
    CreateStar {
        /// Batch-local identifier.
        local: u16,
        /// The holding system.
        system: LivingCommandTargetV2,
        /// Catalog star archetype.
        archetype: LivingSlugV2,
        /// Display name.
        name: LivingNameV2,
    },
    /// Create a world inside a system.
    CreateWorld {
        /// Batch-local identifier.
        local: u16,
        /// The holding system.
        system: LivingCommandTargetV2,
        /// Catalog world archetype.
        archetype: LivingSlugV2,
        /// Display name.
        name: LivingNameV2,
        /// Owning civilization, or `null` for an unowned world.
        owner: Option<LivingCommandTargetV2>,
        /// The world-local stockpile.
        inventory: LivingInventoryV2,
    },
    /// Create a lane between two systems.
    CreateLane {
        /// Batch-local identifier.
        local: u16,
        /// The lower-identified endpoint.
        system_a: LivingCommandTargetV2,
        /// The higher-identified endpoint.
        system_b: LivingCommandTargetV2,
        /// Distance in units.
        distance_units: u64,
    },
    /// Create a civilization.
    CreateCivilization {
        /// Batch-local identifier.
        local: u16,
        /// Display name.
        name: LivingNameV2,
        /// Catalog policy preset.
        policy: LivingPolicyV2,
        /// Active or dormant.
        status: LivingCivilizationStatusV2,
    },
    /// Create a finite deposit on a world.
    CreateDeposit {
        /// Batch-local identifier.
        local: u16,
        /// The world the deposit sits on.
        world: LivingCommandTargetV2,
        /// What the deposit yields.
        resource: LivingResourceV2,
        /// Units remaining.
        remaining_units: u64,
    },
    /// Create an already-built facility.
    CreateFacility {
        /// Batch-local identifier.
        local: u16,
        /// The world it stands on.
        world: LivingCommandTargetV2,
        /// Catalog industry definition.
        definition: LivingSlugV2,
        /// Outcome of the most recent due attempt.
        status: LivingFacilityStatusV2,
        /// Alloy treated as paid, which the half refund is computed from.
        paid_alloy: u32,
        /// Current hit points.
        hit_points: u32,
        /// Next boundary its recipe is evaluated at, or `null`.
        next_due: Option<LivingTickV2>,
    },
    /// Create a paid facility construction already in progress.
    CreateConstructionJob {
        /// Batch-local identifier.
        local: u16,
        /// The world being built on.
        world: LivingCommandTargetV2,
        /// The civilization treated as having paid.
        owner: LivingCommandTargetV2,
        /// Catalog industry definition.
        definition: LivingSlugV2,
        /// Boundary the job was accepted at.
        accepted_tick: LivingTickV2,
        /// Boundary the job completes at.
        completion_tick: LivingTickV2,
        /// Alloy treated as paid.
        paid_alloy: u32,
    },
    /// Create a paid hull build already in progress.
    CreateHullJob {
        /// Batch-local identifier.
        local: u16,
        /// The world whose shipyard is building.
        world: LivingCommandTargetV2,
        /// The civilization treated as having paid.
        owner: LivingCommandTargetV2,
        /// Catalog hull definition.
        definition: LivingSlugV2,
        /// Boundary the job was accepted at.
        accepted_tick: LivingTickV2,
        /// Boundary the job completes at.
        completion_tick: LivingTickV2,
        /// Alloy treated as paid.
        paid_alloy: u32,
        /// The docked fleet the hull joins, or `null` to reserve a new one.
        target_fleet: Option<LivingCommandTargetV2>,
    },
    /// Create a docked fleet.
    CreateFleet {
        /// Batch-local identifier.
        local: u16,
        /// The owning civilization.
        owner: LivingCommandTargetV2,
        /// The world it is docked at.
        world: LivingCommandTargetV2,
    },
    /// Create a hull inside a fleet.
    CreateHull {
        /// Batch-local identifier.
        local: u16,
        /// The fleet it belongs to.
        fleet: LivingCommandTargetV2,
        /// Catalog hull definition.
        definition: LivingSlugV2,
        /// Current hit points.
        hit_points: u32,
        /// Whether it holds a prepaid return itinerary credit.
        return_credit: bool,
    },
    /// Create a standing freight route.
    CreateRoute {
        /// Batch-local identifier.
        local: u16,
        /// Which permission rule it runs under.
        kind: LivingRouteKindV2,
        /// Where cargo is drawn from.
        source_world: LivingCommandTargetV2,
        /// Where cargo is sent to.
        destination_world: LivingCommandTargetV2,
        /// What is carried.
        resource: LivingResourceV2,
        /// Units per dispatch.
        batch_size: u32,
        /// Ticks between dispatch attempts.
        cadence_ticks: u32,
        /// Units that must remain at the source.
        source_reserve: u32,
        /// The source owner whose permission validated it.
        source_owner: LivingCommandTargetV2,
        /// The receiving owner whose permission validated it.
        receiver_owner: LivingCommandTargetV2,
        /// Next boundary a dispatch is attempted at.
        next_due: LivingTickV2,
    },
    /// Create a shipment already in flight.
    CreateShipment {
        /// Batch-local identifier.
        local: u16,
        /// The owner that dispatched the cargo.
        dispatch_owner: LivingCommandTargetV2,
        /// The owner it is addressed to.
        intended_receiver: LivingCommandTargetV2,
        /// The world it left.
        source_world: LivingCommandTargetV2,
        /// The world it is addressed to.
        destination_world: LivingCommandTargetV2,
        /// What is carried.
        resource: LivingResourceV2,
        /// How much is carried.
        units: u32,
        /// Boundary it departed at.
        departure_tick: LivingTickV2,
        /// Boundary it arrives at.
        arrival_tick: LivingTickV2,
        /// What it is currently doing.
        disposition: LivingShipmentDispositionV2,
    },
    /// Schedule a lane hazard.
    CreateHazard {
        /// Batch-local identifier.
        local: u16,
        /// Catalog hazard definition.
        definition: LivingSlugV2,
        /// The lane it sits on.
        lane: LivingCommandTargetV2,
        /// First boundary it is active.
        start_tick: LivingTickV2,
        /// First boundary it is no longer active.
        end_tick: LivingTickV2,
    },
    /// Replace a world's stockpile.
    SetWorldInventory {
        /// The world.
        world: LivingCommandTargetV2,
        /// The replacement stockpile.
        inventory: LivingInventoryV2,
    },
    /// Transfer or clear a world's ownership as a creator override.
    SetWorldOwner {
        /// The world.
        world: LivingCommandTargetV2,
        /// The new owner, or `null` to make the world unowned.
        owner: Option<LivingCommandTargetV2>,
    },
    /// Give an owned world a hub, making it a colony.
    CreateColony {
        /// The world, which is also the colony's key.
        world: LivingCommandTargetV2,
        /// The owning civilization.
        owner: LivingCommandTargetV2,
    },
    /// Remove a colony's hub without removing its world.
    RemoveColony {
        /// The world.
        world: LivingCommandTargetV2,
    },
    /// Set a deposit's remaining units.
    SetDepositRemaining {
        /// The deposit.
        deposit: LivingCommandTargetV2,
        /// Units left.
        remaining_units: u64,
    },
    /// Change a civilization's policy preset.
    SetCivilizationPolicy {
        /// The civilization.
        civilization: LivingCommandTargetV2,
        /// The catalog policy preset.
        policy: LivingPolicyV2,
    },
    /// Set one directed relation's creator-authored base disposition, which
    /// section 6 exempts from decay.
    SetRelationBase {
        /// The civilization holding the opinion.
        from: LivingCommandTargetV2,
        /// The civilization the opinion is about.
        to: LivingCommandTargetV2,
        /// The base disposition.
        base_disposition: i32,
    },
    /// Create or replace a bilateral agreement over a half-open interval.
    SetAgreement {
        /// Which record this is.
        kind: LivingAgreementKindV2,
        /// The lower-identified participant.
        participant_a: LivingCommandTargetV2,
        /// The higher-identified participant.
        participant_b: LivingCommandTargetV2,
        /// First boundary it is in force.
        start_tick: LivingTickV2,
        /// First boundary it is no longer in force.
        end_tick: LivingTickV2,
    },
    /// Remove a bilateral agreement.
    RemoveAgreement {
        /// Which record this is.
        kind: LivingAgreementKindV2,
        /// The lower-identified participant.
        participant_a: LivingCommandTargetV2,
        /// The higher-identified participant.
        participant_b: LivingCommandTargetV2,
    },
    /// Force a war as a creator override, which section 6 requires to be
    /// identified as an override rather than presented as a negotiation.
    ForceWar {
        /// The lower-identified participant.
        participant_a: LivingCommandTargetV2,
        /// The higher-identified participant.
        participant_b: LivingCommandTargetV2,
        /// Which participant the chronicle names as declarer.
        declarer: LivingCommandTargetV2,
        /// First boundary the war is in force.
        start_tick: LivingTickV2,
        /// First boundary the war is no longer in force.
        end_tick: LivingTickV2,
    },
    /// Force peace as a creator override.
    ForcePeace {
        /// The lower-identified participant.
        participant_a: LivingCommandTargetV2,
        /// The higher-identified participant.
        participant_b: LivingCommandTargetV2,
    },
    /// Replace or clear a fleet's standing order.
    SetFleetOrder {
        /// The fleet.
        fleet: LivingCommandTargetV2,
        /// What it is ordered to do, or `null` to make it idle.
        kind: Option<LivingFleetOrderKindV2>,
        /// The destination world, or `null` for an order with no fixed target.
        target_world: Option<LivingCommandTargetV2>,
        /// Earliest boundary the order may depart on.
        not_before_tick: LivingTickV2,
        /// Remaining lanes of the chosen path, in travel order.
        lane_path: Vec<LivingCommandTargetV2>,
    },
    /// Set what a shipment in flight is doing, which is how a creator disposes
    /// of cargo without deleting it silently.
    SetShipmentDisposition {
        /// The shipment.
        shipment: LivingCommandTargetV2,
        /// Its new disposition.
        disposition: LivingShipmentDispositionV2,
    },
    /// Remove one entity, with an explicit decision about its dependents.
    RemoveEntity {
        /// What to remove.
        target: LivingCommandTargetV2,
        /// What to do about its dependents.
        cascade: LivingCascadeDispositionV2,
    },
}

fn push_local(target: &LivingCommandTargetV2, out: &mut Vec<u16>) {
    if let LivingCommandTargetV2::Local { local } = target {
        out.push(*local);
    }
}

fn push_optional_local(target: &Option<LivingCommandTargetV2>, out: &mut Vec<u16>) {
    if let Some(target) = target {
        push_local(target, out);
    }
}

impl LivingCreatorOperationV2 {
    /// The kind of entity this operation creates, or `None` if it edits or
    /// removes rather than creates.
    ///
    /// This is the single place an operation is paired with a registry kind, so
    /// a caller cannot choose a different one and derive a different identity
    /// for the same batch.
    pub const fn created_kind(&self) -> Option<LivingEntityKindV2> {
        match self {
            Self::CreateSystem { .. } => Some(LivingEntityKindV2::System),
            Self::CreateStar { .. } => Some(LivingEntityKindV2::Star),
            Self::CreateWorld { .. } => Some(LivingEntityKindV2::World),
            Self::CreateLane { .. } => Some(LivingEntityKindV2::Lane),
            Self::CreateCivilization { .. } => Some(LivingEntityKindV2::Civilization),
            Self::CreateDeposit { .. } => Some(LivingEntityKindV2::Deposit),
            Self::CreateFacility { .. } => Some(LivingEntityKindV2::Facility),
            Self::CreateConstructionJob { .. } => Some(LivingEntityKindV2::ConstructionJob),
            Self::CreateHullJob { .. } => Some(LivingEntityKindV2::HullJob),
            Self::CreateFleet { .. } => Some(LivingEntityKindV2::Fleet),
            Self::CreateHull { .. } => Some(LivingEntityKindV2::Hull),
            Self::CreateRoute { .. } => Some(LivingEntityKindV2::Route),
            Self::CreateShipment { .. } => Some(LivingEntityKindV2::Shipment),
            Self::CreateHazard { .. } => Some(LivingEntityKindV2::Hazard),
            _ => None,
        }
    }

    /// The batch-local identifier this operation declares, if it creates.
    pub const fn declared_local(&self) -> Option<u16> {
        match self {
            Self::CreateSystem { local, .. }
            | Self::CreateStar { local, .. }
            | Self::CreateWorld { local, .. }
            | Self::CreateLane { local, .. }
            | Self::CreateCivilization { local, .. }
            | Self::CreateDeposit { local, .. }
            | Self::CreateFacility { local, .. }
            | Self::CreateConstructionJob { local, .. }
            | Self::CreateHullJob { local, .. }
            | Self::CreateFleet { local, .. }
            | Self::CreateHull { local, .. }
            | Self::CreateRoute { local, .. }
            | Self::CreateShipment { local, .. }
            | Self::CreateHazard { local, .. } => Some(*local),
            _ => None,
        }
    }

    /// Every batch-local identifier this operation references.
    fn local_references(&self, out: &mut Vec<u16>) {
        match self {
            Self::CreateSystem { .. } => {}
            Self::CreateStar { system, .. } => push_local(system, out),
            Self::CreateWorld { system, owner, .. } => {
                push_local(system, out);
                push_optional_local(owner, out);
            }
            Self::CreateLane {
                system_a, system_b, ..
            } => {
                push_local(system_a, out);
                push_local(system_b, out);
            }
            Self::CreateCivilization { .. } => {}
            Self::CreateDeposit { world, .. }
            | Self::CreateFacility { world, .. }
            | Self::RemoveColony { world }
            | Self::SetWorldInventory { world, .. } => push_local(world, out),
            Self::CreateConstructionJob { world, owner, .. }
            | Self::CreateColony { world, owner }
            | Self::CreateFleet { owner, world, .. } => {
                push_local(world, out);
                push_local(owner, out);
            }
            Self::CreateHullJob {
                world,
                owner,
                target_fleet,
                ..
            } => {
                push_local(world, out);
                push_local(owner, out);
                push_optional_local(target_fleet, out);
            }
            Self::CreateHull { fleet, .. }
            | Self::SetShipmentDisposition {
                shipment: fleet, ..
            } => {
                push_local(fleet, out);
            }
            Self::CreateRoute {
                source_world,
                destination_world,
                source_owner,
                receiver_owner,
                ..
            } => {
                push_local(source_world, out);
                push_local(destination_world, out);
                push_local(source_owner, out);
                push_local(receiver_owner, out);
            }
            Self::CreateShipment {
                dispatch_owner,
                intended_receiver,
                source_world,
                destination_world,
                ..
            } => {
                push_local(dispatch_owner, out);
                push_local(intended_receiver, out);
                push_local(source_world, out);
                push_local(destination_world, out);
            }
            Self::CreateHazard { lane, .. } => push_local(lane, out),
            Self::SetWorldOwner { world, owner } => {
                push_local(world, out);
                push_optional_local(owner, out);
            }
            Self::SetDepositRemaining { deposit, .. } => push_local(deposit, out),
            Self::SetCivilizationPolicy { civilization, .. } => push_local(civilization, out),
            Self::SetRelationBase { from, to, .. } => {
                push_local(from, out);
                push_local(to, out);
            }
            Self::SetAgreement {
                participant_a,
                participant_b,
                ..
            }
            | Self::RemoveAgreement {
                participant_a,
                participant_b,
                ..
            }
            | Self::ForcePeace {
                participant_a,
                participant_b,
            } => {
                push_local(participant_a, out);
                push_local(participant_b, out);
            }
            Self::ForceWar {
                participant_a,
                participant_b,
                declarer,
                ..
            } => {
                push_local(participant_a, out);
                push_local(participant_b, out);
                push_local(declarer, out);
            }
            Self::SetFleetOrder {
                fleet,
                target_world,
                lane_path,
                ..
            } => {
                push_local(fleet, out);
                push_optional_local(target_world, out);
                for lane in lane_path {
                    push_local(lane, out);
                }
            }
            Self::RemoveEntity { target, cascade } => {
                push_local(target, out);
                if let LivingCascadeDispositionV2::RemoveListed { dependents } = cascade {
                    for dependent in dependents {
                        push_local(dependent, out);
                    }
                }
            }
        }
    }
}

/// One creator command.
///
/// Section 2 represents a multi-operation atomic edit as one batch, so this
/// enum has one variant today. It is an enum rather than a struct because the
/// wire contract requires a `type` discriminant on the document, and because a
/// second command family could be added later without moving the batch's own
/// fields.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingCommandV2 {
    /// A batch of operations applied atomically, in declaration order.
    CreatorBatch {
        /// The operations, in the order they apply.
        operations: Vec<LivingCreatorOperationV2>,
    },
}

/// Whether a command applies synchronously at the current boundary or before
/// the simulation phases of the next one.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingCommandModeV2 {
    /// Applied synchronously while paused, at the current completed boundary,
    /// which section 2 permits only with an empty running queue.
    Paused,
    /// Queued and applied before the simulation phases of the next boundary.
    Running,
}

/// A submitted command with the three expectations section 2 requires.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingCommandEnvelopeV2 {
    /// The revision the submitter believes is committed, or `null` before any
    /// revision exists.
    pub expected_committed_revision: Option<LivingRevisionIdV2>,
    /// The completed boundary the submitter believes the authority is at.
    pub expected_tick: LivingTickV2,
    /// The queue tail the submitter believes it is appending to; zero when the
    /// submitter believes the queue is empty.
    pub expected_pending_sequence: u64,
    /// Whether this applies while paused or at the next boundary.
    pub mode: LivingCommandModeV2,
    /// The command itself.
    pub command: LivingCommandV2,
}

/// An accepted envelope, carrying the sequence and boundary it was accepted
/// with.
///
/// Section 2 fixes both meanings and says they "do not change after replay or
/// branch creation": `application_tick` is `t+1` for a running envelope
/// accepted at completed tick `t` and the unchanged current boundary for a
/// paused one, and `accepted_sequence` is the document-global sequence the
/// envelope was allocated.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingAcceptedCommandV2 {
    /// The document-global sequence this envelope was allocated.
    pub accepted_sequence: u64,
    /// The boundary this envelope applies at.
    pub application_tick: LivingTickV2,
    /// The envelope as submitted.
    pub envelope: LivingCommandEnvelopeV2,
}

/// One immutable creator revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingRevisionV2 {
    /// Stable identity, hashed over the canonical command bytes.
    pub id: LivingRevisionIdV2,
    /// The revision produced immediately before this one, or `null` for the
    /// first.
    pub parent: Option<LivingRevisionIdV2>,
    /// The application boundary.
    pub tick: LivingTickV2,
    /// The envelope's document-global accepted sequence.
    pub ordinal: u64,
    /// The command this revision records. Section 10: revisions contain
    /// creator inputs only.
    pub command: LivingCommandV2,
}

/// Every structural way a creator command can be refused.
///
/// These are the refusals a batch can earn on its own, before any state is
/// consulted. Referential checks against a projection belong to submission.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingCommandErrorV2 {
    /// The batch exceeds the declared operation bound.
    #[error("a Living V2 creator batch carries at most {limit} operations, not {operations}")]
    BatchTooLarge {
        /// Operations submitted.
        operations: usize,
        /// The declared bound.
        limit: usize,
    },
    /// Batch-local identifiers are not `0, 1, 2, …` in declaration order.
    #[error("Living V2 batch-local identifier {found} was declared where {expected} was due")]
    LocalIdentifierOutOfOrder {
        /// The identifier the declaration order required.
        expected: u16,
        /// The identifier the batch declared.
        found: u16,
    },
    /// An operation references a local identifier declared later in the batch,
    /// or its own.
    #[error("a Living V2 operation references batch-local identifier {local} before it exists")]
    ForwardLocalReference {
        /// The referenced identifier.
        local: u16,
    },
    /// An operation references a local identifier the batch never declares.
    #[error("a Living V2 operation references undeclared batch-local identifier {local}")]
    UnknownLocalReference {
        /// The referenced identifier.
        local: u16,
    },
    /// A cascade named no dependents, which is the ambiguous deletion section 8
    /// rejects rather than a reviewed one.
    #[error("a Living V2 removal cascade must name the dependents it removes")]
    EmptyCascade,
    /// A cascade named the same dependent twice.
    #[error("a Living V2 removal cascade names a dependent twice")]
    RepeatedCascadeDependent,
    /// The command could not be encoded as canonical bytes.
    #[error("the Living V2 command is not canonical")]
    Wire(#[from] LivingWireErrorV2),
}

impl LivingCommandV2 {
    /// The operations of this command, in declaration order.
    pub fn operations(&self) -> &[LivingCreatorOperationV2] {
        let Self::CreatorBatch { operations } = self;
        operations
    }

    /// The canonical bytes a revision identity is hashed over.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingWireErrorV2> {
        encode_canonical_v2(self, LIVING_MAX_ARCHIVE_BYTES_V2)
    }

    /// Check the batch bound, the local identifier numbering, the reference
    /// direction, and every cascade, without consulting any state.
    pub fn validate(&self) -> Result<(), LivingCommandErrorV2> {
        let operations = self.operations();
        if operations.len() > LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2 {
            return Err(LivingCommandErrorV2::BatchTooLarge {
                operations: operations.len(),
                limit: LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2,
            });
        }

        // Two passes, so a reference to a later local is reported as a forward
        // reference rather than as an unknown one. The distinction matters to
        // the author: one is a reordering, the other is a typo.
        let declared: Vec<u16> = operations
            .iter()
            .filter_map(LivingCreatorOperationV2::declared_local)
            .collect();

        let mut available: Vec<u16> = Vec::new();
        let mut next = 0_u16;
        let mut references = Vec::new();
        for operation in operations {
            references.clear();
            operation.local_references(&mut references);
            for local in &references {
                if !available.contains(local) {
                    return Err(if declared.contains(local) {
                        LivingCommandErrorV2::ForwardLocalReference { local: *local }
                    } else {
                        LivingCommandErrorV2::UnknownLocalReference { local: *local }
                    });
                }
            }

            if let LivingCreatorOperationV2::RemoveEntity {
                cascade: LivingCascadeDispositionV2::RemoveListed { dependents },
                ..
            } = operation
            {
                if dependents.is_empty() {
                    return Err(LivingCommandErrorV2::EmptyCascade);
                }
                for (position, dependent) in dependents.iter().enumerate() {
                    if dependents[position + 1..].contains(dependent) {
                        return Err(LivingCommandErrorV2::RepeatedCascadeDependent);
                    }
                }
            }

            if let Some(local) = operation.declared_local() {
                if local != next {
                    return Err(LivingCommandErrorV2::LocalIdentifierOutOfOrder {
                        expected: next,
                        found: local,
                    });
                }
                available.push(local);
                next = next.saturating_add(1);
            }
        }
        Ok(())
    }

    /// The entities this command creates when recorded as `revision`, in
    /// declaration order.
    ///
    /// Each identity is
    /// `SHA256(domain || revision_id_32 || entity_kind_u16 || batch_local_id_u16)`
    /// truncated to sixteen bytes, with the kind taken from the published
    /// registry rather than from the operation's position or its Rust layout.
    pub fn created_entities(
        &self,
        revision: LivingRevisionIdV2,
    ) -> Vec<(u16, LivingEntityKindV2, LivingEntityIdV2)> {
        self.operations()
            .iter()
            .filter_map(|operation| {
                let local = operation.declared_local()?;
                let kind = operation.created_kind()?;
                Some((
                    local,
                    kind,
                    creator_entity_id_v2(revision, kind.ordinal(), local),
                ))
            })
            .collect()
    }
}

impl LivingRevisionV2 {
    /// Seal one accepted envelope into an immutable revision.
    ///
    /// The command is validated first, then hashed: an invalid batch never
    /// reaches an identity, which is what section 1 means by rejecting "before
    /// any sequence, ordinal, entity ID, queue mutation, or resource is
    /// consumed".
    pub fn seal(
        catalog_hash: LivingCatalogHashV2,
        genesis_seed: [u8; 32],
        parent: Option<LivingRevisionIdV2>,
        accepted: &LivingAcceptedCommandV2,
    ) -> Result<Self, LivingCommandErrorV2> {
        let command = &accepted.envelope.command;
        command.validate()?;
        let bytes = command.canonical_bytes()?;
        let id = revision_id_v2(
            catalog_hash,
            genesis_seed,
            parent,
            accepted.application_tick,
            accepted.accepted_sequence,
            &bytes,
        );
        Ok(Self {
            id,
            parent,
            tick: accepted.application_tick,
            ordinal: accepted.accepted_sequence,
            command: command.clone(),
        })
    }

    /// The entities this revision's command creates.
    pub fn created_entities(&self) -> Vec<(u16, LivingEntityKindV2, LivingEntityIdV2)> {
        self.command.created_entities(self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system(local: u16, name: &str) -> LivingCreatorOperationV2 {
        LivingCreatorOperationV2::CreateSystem {
            local,
            name: LivingNameV2::new(name).expect("a printable name"),
        }
    }

    #[test]
    fn a_creating_operation_pairs_itself_with_one_registry_kind() {
        let operation = system(0, "Vale");
        assert_eq!(
            operation.created_kind(),
            Some(LivingEntityKindV2::System),
            "the operation, not the caller, chooses the kind"
        );
        assert_eq!(operation.declared_local(), Some(0));

        let edit = LivingCreatorOperationV2::RemoveColony {
            world: LivingCommandTargetV2::Existing {
                id: LivingEntityIdV2([0x11; 16]),
            },
        };
        assert_eq!(edit.created_kind(), None);
        assert_eq!(edit.declared_local(), None);
    }

    #[test]
    fn local_identifiers_run_from_zero_in_declaration_order() {
        let command = LivingCommandV2::CreatorBatch {
            operations: vec![system(0, "Vale"), system(1, "Confluence")],
        };
        command.validate().expect("consecutive locals validate");

        let gap = LivingCommandV2::CreatorBatch {
            operations: vec![system(0, "Vale"), system(2, "Confluence")],
        };
        assert_eq!(
            gap.validate(),
            Err(LivingCommandErrorV2::LocalIdentifierOutOfOrder {
                expected: 1,
                found: 2
            })
        );
    }
}
