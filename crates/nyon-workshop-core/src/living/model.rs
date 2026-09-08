//! The authoritative Living Galaxy V2 state schema.
//!
//! `LivingGalaxyStateV2`'s twenty-four fields and their order are normative
//! text from `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`
//! section 10, subsection "Authoritative state schema". Reordering one of them
//! is a format break, not a cosmetic edit: the field order is the canonical
//! byte order and therefore an input to every state digest.
//!
//! Fields four through twenty-three are logical maps, encoded as arrays sorted
//! strictly ascending by a stable key. That contract is enforced by
//! [`LivingSortedVecV2`], whose decoder already rejects unsorted and duplicated
//! keys, so no insertion order can reach the authority and no validation below
//! repeats the check.
//!
//! # What the record types below are, and are not
//!
//! Section 10 publishes the state's own field list. It does not publish the
//! field list of each record inside those collections, so those come from the
//! prose obligations of sections 3 through 7 — routes, shipments and
//! observations are enumerated there almost verbatim — and their **order** is
//! declared here. Section 10's "the schema-declared field order is normative"
//! makes this file that declaration. A reader comparing an independent
//! implementation against the specification alone cannot reproduce these bytes;
//! that gap is recorded rather than papered over.
//!
//! # Two rules that are easy to get backwards
//!
//! Inventory is world-local, not colony-local: section 3 says so directly, and
//! section 5 requires an unowned destination to keep delivered cargo as local
//! inventory, which no colony record could hold. [`LivingWorldV2::inventory`]
//! is therefore the only stockpile in the schema.
//!
//! Fields two and three are **historical**. Section 2 freezes the
//! `accepted_sequence` and `branch_sequence` values that a state was produced
//! with, so that an older branch's digest stays reproducible. The engine's
//! durable allocator high-water marks are a separate thing entirely: they are
//! never fields of this record and never inputs to its digest.

use serde::{Deserialize, Serialize};

use super::catalog::{
    LIVING_RESOURCE_STORAGE_LIMIT_V2, LivingNameV2, LivingSlugV2, ValidatedLivingCatalogPackV2,
};
use super::ids::{LivingEntityIdV2, LivingStateDigestV2, LivingTickV2, state_digest_v2};
use super::wire::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LivingKeyedV2, LivingSortedVecV2, LivingWireErrorV2,
    encode_canonical_v2,
};

// ------------------------------------------------------------ authority maxima

/// Section 1: at most sixteen civilizations.
pub const LIVING_MAX_CIVILIZATIONS_V2: usize = 16;
/// Section 1: at most sixty-four systems.
pub const LIVING_MAX_SYSTEMS_V2: usize = 64;
/// Section 1: at most one hundred and twenty-eight stars.
pub const LIVING_MAX_STARS_V2: usize = 128;
/// Section 1: at most five hundred and twelve worlds.
pub const LIVING_MAX_WORLDS_V2: usize = 512;
/// Section 1: at most two hundred and fifty-six lanes.
pub const LIVING_MAX_LANES_V2: usize = 256;
/// Section 1: at most one thousand and twenty-four deposits.
pub const LIVING_MAX_DEPOSITS_V2: usize = 1_024;
/// Section 1: at most two thousand and forty-eight industries.
///
/// The limits table calls this collection "industries"; the state calls it
/// `facilities`. They are the same bound on the same objects.
pub const LIVING_MAX_INDUSTRIES_V2: usize = 2_048;
/// Section 1: at most two thousand and forty-eight freight routes.
pub const LIVING_MAX_ROUTES_V2: usize = 2_048;
/// Section 1: at most four thousand and ninety-six shipments.
pub const LIVING_MAX_SHIPMENTS_V2: usize = 4_096;
/// Section 1: at most two hundred and fifty-six fleets.
pub const LIVING_MAX_FLEETS_V2: usize = 256;
/// Section 1: at most two thousand and forty-eight hulls in total.
pub const LIVING_MAX_HULLS_V2: usize = 2_048;
/// Section 1: at most sixteen hulls inside one fleet.
pub const LIVING_MAX_HULLS_PER_FLEET_V2: usize = 16;
/// Section 1: at most one hundred and twenty-eight active or scheduled hazards.
pub const LIVING_MAX_HAZARDS_V2: usize = 128;

/// Section 6: a directed relation is an integer in `[-100,100]`.
pub const LIVING_RELATION_BOUND_V2: i32 = 100;
/// Section 6: aid and trade credit accrue per one hundred delivered units, so a
/// stored partial accumulator is always below one hundred.
pub const LIVING_DELIVERY_ACCUMULATOR_BOUND_V2: u32 = 100;
/// Section 6: ownership transfers after one hundred consecutive eligible ticks.
pub const LIVING_OCCUPATION_TRANSFER_TICKS_V2: u32 = 100;

// -------------------------------------------------------------------- errors

/// Every way an assembled Living V2 state can fail its validation obligations.
///
/// Ordering and duplicate-key failures inside one collection are not here: the
/// canonical wire rejects them before a state is ever constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingValidationErrorV2 {
    /// A collection exceeds its declared authority maximum.
    #[error("Living V2 collection {collection} holds {found} records; the limit is {limit}")]
    Capacity {
        /// Name of the collection, as it appears in the state schema.
        collection: &'static str,
        /// Declared maximum from the section 1 limits table.
        limit: usize,
        /// Number of records found.
        found: usize,
    },
    /// One identifier is used by two different objects.
    #[error("Living V2 identity {context} is not globally unique")]
    DuplicateIdentity {
        /// Where the collision was observed.
        context: &'static str,
    },
    /// A record references an object that the state or catalog does not hold.
    #[error("Living V2 record references a missing {reference}")]
    DanglingReference {
        /// The reference that could not be resolved.
        reference: &'static str,
    },
    /// A declared integer falls outside its documented range.
    #[error("Living V2 field {field} is outside its declared range")]
    Range {
        /// The offending field.
        field: &'static str,
    },
    /// A half-open interval does not start strictly before it ends.
    #[error("Living V2 interval {field} is not a nonempty half-open interval")]
    Interval {
        /// The offending interval.
        field: &'static str,
    },
    /// The state does not encode to canonical bytes.
    #[error("Living V2 state wire error: {0}")]
    Wire(#[from] LivingWireErrorV2),
}

// -------------------------------------------------------------- scalar types

/// One of the three stockpiled resources of section 3.
///
/// Variants are declared in wire-string order, which is also the order the
/// canonical sort keys below compare in.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingResourceV2 {
    /// Refined alloy, the currency of construction.
    Alloy,
    /// Energy, consumed by recipes, travel and repair.
    Energy,
    /// Raw ore, drawn from finite deposits.
    Ore,
}

impl LivingResourceV2 {
    /// The stable wire discriminant, which is also the sort key.
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::Alloy => "alloy",
            Self::Energy => "energy",
            Self::Ore => "ore",
        }
    }
}

/// A world-local stockpile.
///
/// Section 3: "Inventory is world-local and capped at 10000 units per resource.
/// No empire-wide invisible pool exists."
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingInventoryV2 {
    /// Stored energy.
    pub energy: u32,
    /// Stored ore.
    pub ore: u32,
    /// Stored alloy.
    pub alloy: u32,
}

impl LivingInventoryV2 {
    /// The stored quantity of one resource.
    pub const fn quantity(&self, resource: LivingResourceV2) -> u32 {
        match resource {
            LivingResourceV2::Alloy => self.alloy,
            LivingResourceV2::Energy => self.energy,
            LivingResourceV2::Ore => self.ore,
        }
    }

    fn validate(&self, field: &'static str) -> Result<(), LivingValidationErrorV2> {
        let over = self.energy > LIVING_RESOURCE_STORAGE_LIMIT_V2
            || self.ore > LIVING_RESOURCE_STORAGE_LIMIT_V2
            || self.alloy > LIVING_RESOURCE_STORAGE_LIMIT_V2;
        if over {
            return Err(LivingValidationErrorV2::Range { field });
        }
        Ok(())
    }
}

/// Why the most recent due attempt of a hub operation or facility recipe
/// produced nothing.
///
/// Section 3 requires the latest reasons to be stored separately for
/// inspection, and section 2 requires a blocked attempt to advance its clock
/// anyway, so these are canonical state rather than a derived report.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingBlockedReasonV2 {
    /// The finite deposit could not supply the required units.
    MissingDeposit {
        /// Units the recipe consumes per recurrence.
        required: u32,
        /// Units remaining in the deposit.
        available: u32,
    },
    /// Local inventory could not supply an input.
    MissingInput {
        /// The input that fell short.
        resource: LivingResourceV2,
        /// Units the recipe consumes per recurrence.
        required: u32,
        /// Units held locally.
        available: u32,
    },
    /// Accepting the output would exceed the per-resource storage limit, so no
    /// input was consumed at all.
    OutputFull {
        /// The output that would overflow.
        resource: LivingResourceV2,
    },
}

impl LivingBlockedReasonV2 {
    /// The stable wire discriminant.
    pub const fn as_wire_str(&self) -> &'static str {
        match self {
            Self::MissingDeposit { .. } => "missing_deposit",
            Self::MissingInput { .. } => "missing_input",
            Self::OutputFull { .. } => "output_full",
        }
    }

    const fn resource_key(&self) -> &'static str {
        match self {
            Self::MissingDeposit { .. } => "",
            Self::MissingInput { resource, .. } | Self::OutputFull { resource } => {
                resource.as_wire_str()
            }
        }
    }
}

impl LivingKeyedV2 for LivingBlockedReasonV2 {
    type Key = (&'static str, &'static str);

    fn living_key(&self) -> Self::Key {
        (self.as_wire_str(), self.resource_key())
    }
}

// ------------------------------------------------------------ 4-7: topology

/// One star system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingSystemV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// Display name.
    pub name: LivingNameV2,
}

impl LivingKeyedV2 for LivingSystemV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// One star, belonging to exactly one system.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingStarV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The system that holds this star.
    pub system: LivingEntityIdV2,
    /// Catalog star archetype.
    pub archetype: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
}

impl LivingKeyedV2 for LivingStarV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// One world: the only place inventory, facilities and colonies exist.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingWorldV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The system that holds this world.
    pub system: LivingEntityIdV2,
    /// Catalog world archetype.
    pub archetype: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Current owner, or `null` for an unowned world.
    pub owner: Option<LivingEntityIdV2>,
    /// The world-local stockpile, which an unowned world also holds.
    pub inventory: LivingInventoryV2,
}

impl LivingKeyedV2 for LivingWorldV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// A direct lane between two distinct systems.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingLaneV2 {
    /// Stable identity, and the tie-breaking key of a travel path.
    pub id: LivingEntityIdV2,
    /// The lower-identified endpoint system.
    pub system_a: LivingEntityIdV2,
    /// The higher-identified endpoint system.
    pub system_b: LivingEntityIdV2,
    /// Distance in units; section 4 turns it into `max(10, ceil(units/10))`
    /// travel ticks.
    pub distance_units: u64,
}

impl LivingKeyedV2 for LivingLaneV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// ------------------------------------------------------- 8: civilizations

/// Whether a civilization still holds a colony.
///
/// Section 6: losing the last colony makes a civilization Dormant, never
/// deleted.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingCivilizationStatusV2 {
    /// Holds at least one colony.
    Active,
    /// Holds no colony but keeps identity, policies, relations and fleets.
    Dormant,
}

/// The catalog policy preset a civilization follows.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LivingPolicyV2(pub LivingSlugV2);

/// One civilization.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingCivilizationV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Catalog policy preset.
    pub policy: LivingPolicyV2,
    /// Active or dormant.
    pub status: LivingCivilizationStatusV2,
}

impl LivingKeyedV2 for LivingCivilizationV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// -------------------------------------------------- 9-10: deposits, colonies

/// A finite resource deposit on one world.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDepositV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The world the deposit sits on.
    pub world: LivingEntityIdV2,
    /// What the deposit yields.
    pub resource: LivingResourceV2,
    /// Units left. Section 3 keeps a final reserve below four visible and
    /// unused rather than rounding it into existence.
    pub remaining_units: u64,
}

impl LivingKeyedV2 for LivingDepositV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// The colony hub, with its three independent recurrence clocks.
///
/// Section 3 fixes the periods: 2 energy every 10 ticks, 1 ore every 100, and a
/// fallback fabricator every 100 while local alloy is below 120. Every due
/// attempt advances its own clock whether it runs or is blocked.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHubV2 {
    /// Next boundary the energy trickle is evaluated at.
    pub energy_next_due: LivingTickV2,
    /// Next boundary the ore trickle is evaluated at.
    pub ore_next_due: LivingTickV2,
    /// Next boundary the fallback fabricator is evaluated at.
    pub fallback_next_due: LivingTickV2,
    /// Reasons the most recent blocked hub attempt produced nothing.
    pub blocked_reasons: LivingSortedVecV2<LivingBlockedReasonV2>,
}

/// An owned world with a hub.
///
/// Section 3: "A colony is an owned world with a hub; not every world must have
/// a colony." The world identity is the colony identity, so no second
/// identifier exists to disagree with it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingColonyV2 {
    /// The world this colony occupies, and the colony's stable key.
    pub world: LivingEntityIdV2,
    /// The owning civilization, which equals the world's owner.
    pub owner: LivingEntityIdV2,
    /// The persistent hub, which survives occupation.
    pub hub: LivingHubV2,
}

impl LivingKeyedV2 for LivingColonyV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.world
    }
}

// ------------------------------------- 11-13: facilities and paid job queues

/// Whether a facility's most recent due attempt produced output.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingFacilityStatusV2 {
    /// The most recent due attempt ran.
    Operational,
    /// The most recent due attempt produced nothing; `blocked_reasons` holds
    /// the complete reason set.
    Blocked,
}

/// One built industrial facility, including a defense battery.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingFacilityV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The world the facility stands on.
    pub world: LivingEntityIdV2,
    /// Catalog industry definition.
    pub definition: LivingSlugV2,
    /// Outcome of the most recent due attempt.
    pub status: LivingFacilityStatusV2,
    /// Alloy actually paid, which section 3's half refund is computed from.
    pub paid_alloy: u32,
    /// Current hit points; only a defense facility can be damaged.
    pub hit_points: u32,
    /// Next boundary the facility's recipe is evaluated at, or `null` for a
    /// facility with no recurring recipe.
    pub next_due: Option<LivingTickV2>,
    /// Next boundary a repair is attempted at, or `null` when undamaged.
    pub next_repair: Option<LivingTickV2>,
    /// Reasons the most recent blocked attempt produced nothing.
    pub blocked_reasons: LivingSortedVecV2<LivingBlockedReasonV2>,
}

impl LivingKeyedV2 for LivingFacilityV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// A paid, in-progress facility construction.
///
/// Section 2: accepted at `T` for duration `D`, it completes at `T+D`, and the
/// completed facility's first recipe is due one period after that.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingConstructionJobV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The world the facility is being built on.
    pub world: LivingEntityIdV2,
    /// The civilization that paid for it.
    pub owner: LivingEntityIdV2,
    /// Catalog industry definition.
    pub definition: LivingSlugV2,
    /// Boundary the job was accepted at.
    pub accepted_tick: LivingTickV2,
    /// Boundary the job completes at.
    pub completion_tick: LivingTickV2,
    /// Alloy paid up front, which a cancellation half-refunds.
    pub paid_alloy: u32,
}

impl LivingKeyedV2 for LivingConstructionJobV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// A paid, in-progress hull build.
///
/// Section 1: the job either names an existing docked fleet or reserves the
/// creation of a new docked fleet, and that reservation blocks the target
/// fleet from departing, splitting or merging until completion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHullJobV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The world whose shipyard is building the hull.
    pub world: LivingEntityIdV2,
    /// The civilization that paid for it.
    pub owner: LivingEntityIdV2,
    /// Catalog hull definition.
    pub definition: LivingSlugV2,
    /// Boundary the job was accepted at.
    pub accepted_tick: LivingTickV2,
    /// Boundary the job completes at.
    pub completion_tick: LivingTickV2,
    /// Alloy paid up front, which a cancellation half-refunds.
    pub paid_alloy: u32,
    /// The docked fleet the hull joins, or `null` when the job reserves the
    /// creation of a new docked fleet.
    pub target_fleet: Option<LivingEntityIdV2>,
}

impl LivingKeyedV2 for LivingHullJobV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// ---------------------------------------------------- 14: fleets and hulls

/// One hull inside a fleet.
///
/// Section 4: splitting and merging preserve every hull's identity, hit points
/// and fuel credit, and a credit dies with its hull.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHullV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// Catalog hull definition.
    pub definition: LivingSlugV2,
    /// Current hit points.
    pub hit_points: u32,
    /// Whether this hull holds a prepaid return itinerary credit.
    pub return_credit: bool,
    /// Next boundary a repair is attempted at, or `null` when undamaged.
    pub next_repair: Option<LivingTickV2>,
}

impl LivingKeyedV2 for LivingHullV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// Where a fleet is.
///
/// Section 4: a departed leg retains its endpoints and duration, and combat and
/// occupation happen at world destinations rather than transit waypoints.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingFleetLocationV2 {
    /// Stationary at one world.
    Docked {
        /// The world the fleet sits at.
        world: LivingEntityIdV2,
    },
    /// In transit on a committed leg.
    Travelling {
        /// The world the current leg departed from.
        origin_world: LivingEntityIdV2,
        /// The world the current leg arrives at.
        destination_world: LivingEntityIdV2,
        /// Boundary the current leg departed at.
        departure_tick: LivingTickV2,
        /// Boundary the current leg arrives at.
        arrival_tick: LivingTickV2,
    },
}

/// What a fleet has been ordered to do.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingFleetOrderKindV2 {
    /// Move to a hostile world under a declared war.
    Attack,
    /// Hold at an owned colony under threat.
    Defend,
    /// Seek never-observed systems, then the oldest observation.
    Explore,
    /// Move to a named world.
    Move,
    /// Travel back to an owned colony using prepaid credits.
    Return,
    /// Carry an ark to an eligible unowned world.
    Settle,
}

/// A fleet's standing order.
///
/// Section 2: a newly accepted order cannot depart on the boundary that
/// accepted it, which `not_before_tick` records rather than recomputes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingFleetOrderV2 {
    /// What the fleet was ordered to do.
    pub kind: LivingFleetOrderKindV2,
    /// The destination world, or `null` for an order with no fixed target.
    pub target_world: Option<LivingEntityIdV2>,
    /// Earliest boundary the order may depart on.
    pub not_before_tick: LivingTickV2,
    /// Remaining lanes of the chosen path, in travel order. Section 4 resolves
    /// an equal-duration tie by the lexicographically smallest lane path, so
    /// this array's order is meaningful and is not a sorted map.
    pub lane_path: Vec<LivingEntityIdV2>,
}

/// One fleet of co-owned hulls.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingFleetV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The owning civilization.
    pub owner: LivingEntityIdV2,
    /// Where the fleet is.
    pub location: LivingFleetLocationV2,
    /// The standing order, or `null` when idle.
    pub order: Option<LivingFleetOrderV2>,
    /// The fleet's hulls, sorted by hull identity.
    pub hulls: LivingSortedVecV2<LivingHullV2>,
}

impl LivingKeyedV2 for LivingFleetV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// ------------------------------------------------ 15-16: routes and shipments

/// Which permission rule a freight route runs under.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingRouteKindV2 {
    /// A creator-configured one-way aid route, which needs no agreement and
    /// cannot take another civilization's stock.
    Aid,
    /// Both endpoints belong to one civilization; no agreement is needed.
    Internal,
    /// Cross-civilization freight, which needs a current bilateral trade
    /// agreement at every dispatch.
    Trade,
}

/// A standing freight route.
///
/// Section 5 enumerates what a route records: source, destination, resource,
/// batch size, cadence, source reserve, and the participating owners whose
/// permissions validated it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingFreightRouteV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// Which permission rule the route runs under.
    pub kind: LivingRouteKindV2,
    /// Where cargo is drawn from.
    pub source_world: LivingEntityIdV2,
    /// Where cargo is sent to.
    pub destination_world: LivingEntityIdV2,
    /// What is carried.
    pub resource: LivingResourceV2,
    /// Units per dispatch, before any hazard reduction.
    pub batch_size: u32,
    /// Ticks between dispatch attempts.
    pub cadence_ticks: u32,
    /// Units that must remain at the source after a dispatch.
    pub source_reserve: u32,
    /// The source owner whose permission validated the route.
    pub source_owner: LivingEntityIdV2,
    /// The receiving owner whose permission validated the route.
    pub receiver_owner: LivingEntityIdV2,
    /// Next boundary a dispatch is attempted at.
    pub next_due: LivingTickV2,
    /// Whether either endpoint's current owner has diverged from the
    /// participants, which suspends the route until the current source owner
    /// validates an adjustment.
    pub suspended: bool,
}

impl LivingKeyedV2 for LivingFreightRouteV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// What a shipment is currently doing.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingShipmentDispositionV2 {
    /// Travelling toward its destination.
    Outbound,
    /// Travelling back to its original source after a war return.
    Returning,
    /// Arrived, but the chosen recipient's storage is full; it retries each
    /// tick and re-evaluates disposition before capacity.
    WaitingForStorage,
}

/// One shipment in flight, waiting or returning.
///
/// Section 5: a shipment records dispatch owner, intended receiver, units,
/// departure and arrival ticks, and return status, and that manifest is
/// immutable. It deliberately holds no reference back to its route, because a
/// route may be suspended, adjusted or removed while the cargo it dispatched is
/// still resolving.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingShipmentV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// The owner that dispatched the cargo.
    pub dispatch_owner: LivingEntityIdV2,
    /// The owner the cargo was addressed to.
    pub intended_receiver: LivingEntityIdV2,
    /// The world the cargo left.
    pub source_world: LivingEntityIdV2,
    /// The world the cargo is addressed to.
    pub destination_world: LivingEntityIdV2,
    /// What is carried.
    pub resource: LivingResourceV2,
    /// How much is carried.
    pub units: u32,
    /// Boundary the cargo departed at.
    pub departure_tick: LivingTickV2,
    /// Boundary the cargo arrives at.
    pub arrival_tick: LivingTickV2,
    /// What the shipment is currently doing.
    pub disposition: LivingShipmentDispositionV2,
}

impl LivingKeyedV2 for LivingShipmentV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// ------------------------------------- 17-19: relations, agreements and wars

/// The bounded reason counters behind one directed relation.
///
/// Section 6 fixes each counter's cap or floor and makes both the counters and
/// the partial hundred-unit delivery accumulators canonical state.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingRelationReasonsV2 {
    /// Recipient toward sender, `+10` per 100 delivered aid units, capped at
    /// `+20`.
    pub delivered_aid: i32,
    /// Receiver toward sender, `+1` per 100 delivered trade units, capped at
    /// `+20`.
    pub successful_trade: i32,
    /// Existing neighbor toward settler, `-10` per new adjacent colony, floored
    /// at `-20`.
    pub adjacent_rival_settlement: i32,
    /// Victim toward declarer, `-30`, floored at `-30`.
    pub war_declared_against_it: i32,
    /// Former owner toward captor, `-30` per colony, floored at `-60`.
    pub colony_captured_from_it: i32,
    /// Delivered aid units not yet worth a whole counter step.
    pub aid_units_remainder: u32,
    /// Delivered trade units not yet worth a whole counter step.
    pub trade_units_remainder: u32,
}

/// One directed relation.
///
/// Section 6: relations are directed, and the total is the clamped sum of the
/// creator-set base disposition and the bounded reason counters.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingRelationV2 {
    /// The civilization holding the opinion.
    pub from: LivingEntityIdV2,
    /// The civilization the opinion is about.
    pub to: LivingEntityIdV2,
    /// The creator-set base disposition, which never decays.
    pub base_disposition: i32,
    /// The bounded event counters.
    pub reasons: LivingRelationReasonsV2,
}

impl LivingRelationV2 {
    /// The clamped sum reported to gameplay.
    pub fn total(&self) -> i32 {
        let sum = self.base_disposition
            + self.reasons.delivered_aid
            + self.reasons.successful_trade
            + self.reasons.adjacent_rival_settlement
            + self.reasons.war_declared_against_it
            + self.reasons.colony_captured_from_it;
        sum.clamp(-LIVING_RELATION_BOUND_V2, LIVING_RELATION_BOUND_V2)
    }
}

impl LivingKeyedV2 for LivingRelationV2 {
    type Key = (LivingEntityIdV2, LivingEntityIdV2);

    fn living_key(&self) -> Self::Key {
        (self.from, self.to)
    }
}

/// Which bilateral record an agreement is.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingAgreementKindV2 {
    /// Blocks new war while active.
    Nonaggression,
    /// Permits cross-civilization freight.
    Trade,
    /// The 600-tick record a war automatically becomes.
    Truce,
}

impl LivingAgreementKindV2 {
    /// The stable wire discriminant, which is also part of the sort key.
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::Nonaggression => "nonaggression",
            Self::Trade => "trade",
            Self::Truce => "truce",
        }
    }
}

/// One bilateral agreement over a half-open tick interval.
///
/// Section 6 makes the interval end-exclusive: an agreement `[0,1200)` prevents
/// an otherwise eligible war through 1199 and not at 1200.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingAgreementV2 {
    /// Which record this is.
    pub kind: LivingAgreementKindV2,
    /// The lower-identified participant.
    pub participant_a: LivingEntityIdV2,
    /// The higher-identified participant.
    pub participant_b: LivingEntityIdV2,
    /// First boundary the record is in force.
    pub start_tick: LivingTickV2,
    /// First boundary the record is no longer in force.
    pub end_tick: LivingTickV2,
}

impl LivingKeyedV2 for LivingAgreementV2 {
    type Key = (LivingEntityIdV2, LivingEntityIdV2, &'static str);

    fn living_key(&self) -> Self::Key {
        (
            self.participant_a,
            self.participant_b,
            self.kind.as_wire_str(),
        )
    }
}

/// One declared war over a half-open tick interval.
///
/// Section 6: identical bilateral proposals create one war, so the sort key is
/// the ordered participant pair rather than the direction of the declaration,
/// which is kept only so the chronicle can name the declarer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingWarV2 {
    /// The lower-identified participant.
    pub participant_a: LivingEntityIdV2,
    /// The higher-identified participant.
    pub participant_b: LivingEntityIdV2,
    /// Which participant declared it.
    pub declarer: LivingEntityIdV2,
    /// First boundary the war is in force.
    pub start_tick: LivingTickV2,
    /// First boundary the war is no longer in force.
    pub end_tick: LivingTickV2,
}

impl LivingKeyedV2 for LivingWarV2 {
    type Key = (LivingEntityIdV2, LivingEntityIdV2);

    fn living_key(&self) -> Self::Key {
        (self.participant_a, self.participant_b)
    }
}

// ------------------------------------------- 20-21: observations and hazards

/// A counted kind of facility a civilization observed on a world.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingObservedFacilityV2 {
    /// Catalog industry definition.
    pub definition: LivingSlugV2,
    /// How many were seen.
    pub count: u32,
}

impl LivingKeyedV2 for LivingObservedFacilityV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.definition.clone()
    }
}

/// A counted kind of hull a civilization observed stationed at a world.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingObservedHullV2 {
    /// Catalog hull definition.
    pub definition: LivingSlugV2,
    /// How many were seen.
    pub count: u32,
}

impl LivingKeyedV2 for LivingObservedHullV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.definition.clone()
    }
}

/// What one civilization last saw of one world.
///
/// Section 4: store each world's observed tick, owner, remaining deposit,
/// facilities and stationed hulls. An old observation persists with its
/// original timestamp after visibility ends, and hidden changes never update
/// it. Rival stockpile quantities are deliberately absent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingObservationV2 {
    /// The observing civilization.
    pub civilization: LivingEntityIdV2,
    /// The observed world.
    pub world: LivingEntityIdV2,
    /// Boundary the observation was taken at.
    pub observed_tick: LivingTickV2,
    /// Owner seen at that boundary, or `null` for an unowned world.
    pub owner: Option<LivingEntityIdV2>,
    /// Deposit units seen remaining.
    pub remaining_deposit: u64,
    /// Facilities seen, sorted by definition.
    pub facilities: LivingSortedVecV2<LivingObservedFacilityV2>,
    /// Hulls seen stationed, sorted by definition.
    pub stationed_hulls: LivingSortedVecV2<LivingObservedHullV2>,
}

impl LivingKeyedV2 for LivingObservationV2 {
    type Key = (LivingEntityIdV2, LivingEntityIdV2);

    fn living_key(&self) -> Self::Key {
        (self.civilization, self.world)
    }
}

/// One scheduled hazard on one lane, over a half-open tick interval.
///
/// Section 5: multiple storms on a lane use the strongest reduction rather than
/// compounding, and they affect new dispatches only.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHazardV2 {
    /// Stable identity.
    pub id: LivingEntityIdV2,
    /// Catalog hazard definition.
    pub definition: LivingSlugV2,
    /// The lane the hazard sits on.
    pub lane: LivingEntityIdV2,
    /// First boundary the hazard is active.
    pub start_tick: LivingTickV2,
    /// First boundary the hazard is no longer active.
    pub end_tick: LivingTickV2,
}

impl LivingKeyedV2 for LivingHazardV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.id
    }
}

// -------------------------------------- 22-23: claims and occupation progress

/// One civilization's participation in a world's settlement claim window.
///
/// Section 5: a first eligible arrival at boundary `A` opens the half-open
/// window `[A,A+50)` and arbitration runs at its close, so every claim on one
/// world shares that close boundary. Multiple arks from one civilization are
/// one claim, which is why the key is the world and the claimant.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingSettlementClaimV2 {
    /// The world being claimed.
    pub world: LivingEntityIdV2,
    /// The claiming civilization.
    pub civilization: LivingEntityIdV2,
    /// Boundary this claimant arrived at.
    pub arrival_tick: LivingTickV2,
    /// The shared boundary arbitration runs at.
    pub close_tick: LivingTickV2,
}

impl LivingKeyedV2 for LivingSettlementClaimV2 {
    type Key = (LivingEntityIdV2, LivingEntityIdV2);

    fn living_key(&self) -> Self::Key {
        (self.world, self.civilization)
    }
}

/// Occupation progress by a unique eligible claimant.
///
/// Section 6: a unique eligible claimant increments progress once per completed
/// tick and any invalidating condition resets it, so a contested world holds no
/// record at all rather than a record with zero progress.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingOccupationV2 {
    /// The world being occupied.
    pub world: LivingEntityIdV2,
    /// The unique eligible claimant.
    pub claimant: LivingEntityIdV2,
    /// Consecutive eligible ticks accumulated.
    pub progress_ticks: u32,
}

impl LivingKeyedV2 for LivingOccupationV2 {
    type Key = LivingEntityIdV2;

    fn living_key(&self) -> Self::Key {
        self.world
    }
}

// ------------------------------------------------------ 24: reserved counters

/// Reserved document-global counters, empty in this rules version.
///
/// Section 10 keeps this record rather than omitting it so that adding a
/// document-global counter later cannot reorder fields one through
/// twenty-three. The cost is deliberate and stated so it is not later mistaken
/// for an oversight: it contributes a constant `{}` to every state digest,
/// permanently, and populating or removing it is a format break that needs new
/// vectors. The per-relation delivery accumulators of section 6 are not this
/// record; they live in [`LivingRelationReasonsV2`].
///
/// The braces are load-bearing. A unit struct would encode as `null`, and the
/// reserved field must encode as an empty object.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingCountersV2 {}

// ------------------------------------------------------------- the state

/// The authoritative Living Galaxy V2 state.
///
/// The twenty-four fields below are in the normative order published by
/// section 10. Fields four through twenty-three are logical maps encoded as
/// arrays sorted by stable key.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingGalaxyStateV2 {
    /// 1. Completed tick boundaries.
    pub tick: LivingTickV2,
    /// 2. The historical accepted-envelope sequence this state was produced
    ///    with. Never the live allocator mark.
    pub accepted_sequence: u64,
    /// 3. The historical fork sequence this state was produced with. Never the
    ///    live allocator mark.
    pub branch_sequence: u64,
    /// 4. Star systems.
    pub systems: LivingSortedVecV2<LivingSystemV2>,
    /// 5. Stars.
    pub stars: LivingSortedVecV2<LivingStarV2>,
    /// 6. Worlds, which hold all inventory.
    pub worlds: LivingSortedVecV2<LivingWorldV2>,
    /// 7. Lanes between systems.
    pub lanes: LivingSortedVecV2<LivingLaneV2>,
    /// 8. Civilizations.
    pub civilizations: LivingSortedVecV2<LivingCivilizationV2>,
    /// 9. Finite deposits.
    pub deposits: LivingSortedVecV2<LivingDepositV2>,
    /// 10. Colonies, keyed by their world.
    pub colonies: LivingSortedVecV2<LivingColonyV2>,
    /// 11. Built facilities.
    pub facilities: LivingSortedVecV2<LivingFacilityV2>,
    /// 12. Paid facility construction in progress.
    pub construction_jobs: LivingSortedVecV2<LivingConstructionJobV2>,
    /// 13. Paid hull construction in progress.
    pub hull_jobs: LivingSortedVecV2<LivingHullJobV2>,
    /// 14. Fleets, with their hulls nested inside the fleet record.
    pub fleets: LivingSortedVecV2<LivingFleetV2>,
    /// 15. Standing freight routes.
    pub routes: LivingSortedVecV2<LivingFreightRouteV2>,
    /// 16. Shipments in flight, waiting or returning.
    pub shipments: LivingSortedVecV2<LivingShipmentV2>,
    /// 17. Directed relations.
    pub relations: LivingSortedVecV2<LivingRelationV2>,
    /// 18. Bilateral agreements and truces.
    pub agreements: LivingSortedVecV2<LivingAgreementV2>,
    /// 19. Declared wars.
    pub wars: LivingSortedVecV2<LivingWarV2>,
    /// 20. Per-civilization world observations.
    pub observations: LivingSortedVecV2<LivingObservationV2>,
    /// 21. Active or scheduled hazards.
    pub hazards: LivingSortedVecV2<LivingHazardV2>,
    /// 22. Open settlement claims.
    pub settlement_claims: LivingSortedVecV2<LivingSettlementClaimV2>,
    /// 23. Occupation progress.
    pub occupations: LivingSortedVecV2<LivingOccupationV2>,
    /// 24. Reserved document-global counters; empty in this rules version.
    pub counters: LivingCountersV2,
}

impl LivingGalaxyStateV2 {
    /// The canonical bytes this state hashes as.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingWireErrorV2> {
        encode_canonical_v2(self, LIVING_MAX_ARCHIVE_BYTES_V2)
    }

    /// `state_digest = SHA256("NYON-LIVING-STATE-V2\0" || canonical_bytes)`.
    ///
    /// The live allocator high-water marks of section 2 are deliberately not
    /// reachable from here: they are not fields of this record, so no path
    /// exists from them into this digest.
    pub fn digest(&self) -> Result<LivingStateDigestV2, LivingWireErrorV2> {
        Ok(state_digest_v2(&self.canonical_bytes()?))
    }

    /// Check every collection maximum, every intra-state reference, every
    /// catalog reference, and the declared integer and interval ranges.
    ///
    /// Ordering and duplicate keys inside one collection are not checked here:
    /// the canonical wire already rejects them, and a second implementation of
    /// the same rule is a second place for it to drift.
    pub fn validate(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        self.check_capacities()?;
        self.check_global_identity_uniqueness()?;
        self.check_topology(catalog)?;
        self.check_civilizations(catalog)?;
        self.check_colonies_and_industry(catalog)?;
        self.check_fleets(catalog)?;
        self.check_freight(catalog)?;
        self.check_diplomacy()?;
        self.check_observations_and_hazards(catalog)?;
        self.check_claims_and_occupations()?;
        Ok(())
    }

    fn check_capacities(&self) -> Result<(), LivingValidationErrorV2> {
        let rows: [(&'static str, usize, usize); 12] = [
            (
                "civilizations",
                LIVING_MAX_CIVILIZATIONS_V2,
                self.civilizations.as_slice().len(),
            ),
            (
                "systems",
                LIVING_MAX_SYSTEMS_V2,
                self.systems.as_slice().len(),
            ),
            ("stars", LIVING_MAX_STARS_V2, self.stars.as_slice().len()),
            ("worlds", LIVING_MAX_WORLDS_V2, self.worlds.as_slice().len()),
            ("lanes", LIVING_MAX_LANES_V2, self.lanes.as_slice().len()),
            (
                "deposits",
                LIVING_MAX_DEPOSITS_V2,
                self.deposits.as_slice().len(),
            ),
            (
                "facilities",
                LIVING_MAX_INDUSTRIES_V2,
                self.facilities.as_slice().len(),
            ),
            ("routes", LIVING_MAX_ROUTES_V2, self.routes.as_slice().len()),
            (
                "shipments",
                LIVING_MAX_SHIPMENTS_V2,
                self.shipments.as_slice().len(),
            ),
            ("fleets", LIVING_MAX_FLEETS_V2, self.fleets.as_slice().len()),
            (
                "hazards",
                LIVING_MAX_HAZARDS_V2,
                self.hazards.as_slice().len(),
            ),
            (
                "colonies",
                self.worlds.as_slice().len(),
                self.colonies.as_slice().len(),
            ),
        ];
        for (collection, limit, found) in rows {
            if found > limit {
                return Err(LivingValidationErrorV2::Capacity {
                    collection,
                    limit,
                    found,
                });
            }
        }

        let mut hulls = 0_usize;
        for fleet in self.fleets.as_slice() {
            let count = fleet.hulls.as_slice().len();
            if count > LIVING_MAX_HULLS_PER_FLEET_V2 {
                return Err(LivingValidationErrorV2::Capacity {
                    collection: "fleet hulls",
                    limit: LIVING_MAX_HULLS_PER_FLEET_V2,
                    found: count,
                });
            }
            hulls += count;
        }
        if hulls > LIVING_MAX_HULLS_V2 {
            return Err(LivingValidationErrorV2::Capacity {
                collection: "hulls",
                limit: LIVING_MAX_HULLS_V2,
                found: hulls,
            });
        }
        Ok(())
    }

    fn check_global_identity_uniqueness(&self) -> Result<(), LivingValidationErrorV2> {
        let mut identities: Vec<LivingEntityIdV2> = Vec::new();
        identities.extend(self.systems.as_slice().iter().map(|row| row.id));
        identities.extend(self.stars.as_slice().iter().map(|row| row.id));
        identities.extend(self.worlds.as_slice().iter().map(|row| row.id));
        identities.extend(self.lanes.as_slice().iter().map(|row| row.id));
        identities.extend(self.civilizations.as_slice().iter().map(|row| row.id));
        identities.extend(self.deposits.as_slice().iter().map(|row| row.id));
        identities.extend(self.facilities.as_slice().iter().map(|row| row.id));
        identities.extend(self.construction_jobs.as_slice().iter().map(|row| row.id));
        identities.extend(self.hull_jobs.as_slice().iter().map(|row| row.id));
        identities.extend(self.fleets.as_slice().iter().map(|row| row.id));
        identities.extend(
            self.fleets
                .as_slice()
                .iter()
                .flat_map(|fleet| fleet.hulls.as_slice().iter().map(|hull| hull.id)),
        );
        identities.extend(self.routes.as_slice().iter().map(|row| row.id));
        identities.extend(self.shipments.as_slice().iter().map(|row| row.id));
        identities.extend(self.hazards.as_slice().iter().map(|row| row.id));
        identities.sort_unstable();
        if identities.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(LivingValidationErrorV2::DuplicateIdentity {
                context: "authority object",
            });
        }
        Ok(())
    }

    fn check_topology(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for star in self.stars.as_slice() {
            require(has_id(&self.systems, star.system), "star system")?;
            require(
                catalog
                    .star_archetypes()
                    .iter()
                    .any(|row| row.id == star.archetype),
                "star archetype",
            )?;
        }
        for world in self.worlds.as_slice() {
            require(has_id(&self.systems, world.system), "world system")?;
            require(
                catalog
                    .world_archetypes()
                    .iter()
                    .any(|row| row.id == world.archetype),
                "world archetype",
            )?;
            if let Some(owner) = world.owner {
                require(has_id(&self.civilizations, owner), "world owner")?;
            }
            world.inventory.validate("world inventory")?;
        }
        for lane in self.lanes.as_slice() {
            require(has_id(&self.systems, lane.system_a), "lane endpoint")?;
            require(has_id(&self.systems, lane.system_b), "lane endpoint")?;
            if lane.system_a >= lane.system_b {
                return Err(LivingValidationErrorV2::Range {
                    field: "lane endpoints",
                });
            }
            if lane.distance_units == 0 {
                return Err(LivingValidationErrorV2::Range {
                    field: "lane distance_units",
                });
            }
        }
        for deposit in self.deposits.as_slice() {
            require(has_id(&self.worlds, deposit.world), "deposit world")?;
            require(
                catalog.resource(deposit.resource.as_wire_str()).is_some(),
                "deposit resource",
            )?;
        }
        Ok(())
    }

    fn check_civilizations(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for civilization in self.civilizations.as_slice() {
            require(
                catalog.policy(civilization.policy.0.as_str()).is_some(),
                "civilization policy",
            )?;
        }
        Ok(())
    }

    fn check_colonies_and_industry(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for colony in self.colonies.as_slice() {
            let world = find_world(&self.worlds, colony.world)?;
            require(has_id(&self.civilizations, colony.owner), "colony owner")?;
            if world.owner != Some(colony.owner) {
                return Err(LivingValidationErrorV2::Range {
                    field: "colony owner",
                });
            }
        }
        for facility in self.facilities.as_slice() {
            require(has_id(&self.worlds, facility.world), "facility world")?;
            require(
                catalog.industry(facility.definition.as_str()).is_some(),
                "facility definition",
            )?;
        }
        for job in self.construction_jobs.as_slice() {
            require(has_id(&self.worlds, job.world), "construction job world")?;
            require(
                has_id(&self.civilizations, job.owner),
                "construction job owner",
            )?;
            require(
                catalog.industry(job.definition.as_str()).is_some(),
                "construction job definition",
            )?;
            interval(
                job.accepted_tick,
                job.completion_tick,
                "construction job duration",
            )?;
        }
        for job in self.hull_jobs.as_slice() {
            require(has_id(&self.worlds, job.world), "hull job world")?;
            require(has_id(&self.civilizations, job.owner), "hull job owner")?;
            require(
                catalog.hull(job.definition.as_str()).is_some(),
                "hull job definition",
            )?;
            if let Some(fleet) = job.target_fleet {
                require(has_id(&self.fleets, fleet), "hull job target fleet")?;
            }
            interval(job.accepted_tick, job.completion_tick, "hull job duration")?;
        }
        Ok(())
    }

    fn check_fleets(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for fleet in self.fleets.as_slice() {
            require(has_id(&self.civilizations, fleet.owner), "fleet owner")?;
            match fleet.location {
                LivingFleetLocationV2::Docked { world } => {
                    require(has_id(&self.worlds, world), "fleet world")?;
                }
                LivingFleetLocationV2::Travelling {
                    origin_world,
                    destination_world,
                    departure_tick,
                    arrival_tick,
                } => {
                    require(has_id(&self.worlds, origin_world), "fleet world")?;
                    require(has_id(&self.worlds, destination_world), "fleet world")?;
                    interval(departure_tick, arrival_tick, "fleet leg")?;
                }
            }
            if let Some(order) = &fleet.order {
                if let Some(target) = order.target_world {
                    require(has_id(&self.worlds, target), "fleet order target")?;
                }
                for lane in &order.lane_path {
                    require(has_id(&self.lanes, *lane), "fleet order lane")?;
                }
            }
            for hull in fleet.hulls.as_slice() {
                let definition = catalog.hull(hull.definition.as_str()).ok_or(
                    LivingValidationErrorV2::DanglingReference {
                        reference: "hull definition",
                    },
                )?;
                if hull.hit_points == 0 || hull.hit_points > definition.hit_points {
                    return Err(LivingValidationErrorV2::Range {
                        field: "hull hit_points",
                    });
                }
            }
        }
        Ok(())
    }

    fn check_freight(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for route in self.routes.as_slice() {
            require(has_id(&self.worlds, route.source_world), "route world")?;
            require(has_id(&self.worlds, route.destination_world), "route world")?;
            require(
                has_id(&self.civilizations, route.source_owner),
                "route owner",
            )?;
            require(
                has_id(&self.civilizations, route.receiver_owner),
                "route owner",
            )?;
            require(
                catalog.resource(route.resource.as_wire_str()).is_some(),
                "route resource",
            )?;
            if route.source_world == route.destination_world {
                return Err(LivingValidationErrorV2::Range {
                    field: "route endpoints",
                });
            }
            if route.batch_size == 0 || route.cadence_ticks == 0 {
                return Err(LivingValidationErrorV2::Range {
                    field: "route cadence",
                });
            }
            if route.batch_size > LIVING_RESOURCE_STORAGE_LIMIT_V2
                || route.source_reserve > LIVING_RESOURCE_STORAGE_LIMIT_V2
            {
                return Err(LivingValidationErrorV2::Range {
                    field: "route batch_size",
                });
            }
        }
        for shipment in self.shipments.as_slice() {
            require(
                has_id(&self.worlds, shipment.source_world),
                "shipment world",
            )?;
            require(
                has_id(&self.worlds, shipment.destination_world),
                "shipment world",
            )?;
            require(
                has_id(&self.civilizations, shipment.dispatch_owner),
                "shipment owner",
            )?;
            require(
                has_id(&self.civilizations, shipment.intended_receiver),
                "shipment owner",
            )?;
            require(
                catalog.resource(shipment.resource.as_wire_str()).is_some(),
                "shipment resource",
            )?;
            if shipment.units == 0 || shipment.units > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
                return Err(LivingValidationErrorV2::Range {
                    field: "shipment units",
                });
            }
            interval(
                shipment.departure_tick,
                shipment.arrival_tick,
                "shipment travel",
            )?;
        }
        Ok(())
    }

    fn check_diplomacy(&self) -> Result<(), LivingValidationErrorV2> {
        for relation in self.relations.as_slice() {
            require(has_id(&self.civilizations, relation.from), "relation party")?;
            require(has_id(&self.civilizations, relation.to), "relation party")?;
            if relation.from == relation.to {
                return Err(LivingValidationErrorV2::Range {
                    field: "relation parties",
                });
            }
            bounded(
                relation.base_disposition,
                -LIVING_RELATION_BOUND_V2,
                LIVING_RELATION_BOUND_V2,
                "relation base_disposition",
            )?;
            let reasons = &relation.reasons;
            bounded(reasons.delivered_aid, 0, 20, "relation delivered_aid")?;
            bounded(reasons.successful_trade, 0, 20, "relation successful_trade")?;
            bounded(
                reasons.adjacent_rival_settlement,
                -20,
                0,
                "relation adjacent_rival_settlement",
            )?;
            bounded(
                reasons.war_declared_against_it,
                -30,
                0,
                "relation war_declared_against_it",
            )?;
            bounded(
                reasons.colony_captured_from_it,
                -60,
                0,
                "relation colony_captured_from_it",
            )?;
            if reasons.aid_units_remainder >= LIVING_DELIVERY_ACCUMULATOR_BOUND_V2
                || reasons.trade_units_remainder >= LIVING_DELIVERY_ACCUMULATOR_BOUND_V2
            {
                return Err(LivingValidationErrorV2::Range {
                    field: "relation delivery accumulator",
                });
            }
        }
        for agreement in self.agreements.as_slice() {
            require(
                has_id(&self.civilizations, agreement.participant_a),
                "agreement party",
            )?;
            require(
                has_id(&self.civilizations, agreement.participant_b),
                "agreement party",
            )?;
            if agreement.participant_a >= agreement.participant_b {
                return Err(LivingValidationErrorV2::Range {
                    field: "agreement participants",
                });
            }
            interval(agreement.start_tick, agreement.end_tick, "agreement")?;
        }
        for war in self.wars.as_slice() {
            require(has_id(&self.civilizations, war.participant_a), "war party")?;
            require(has_id(&self.civilizations, war.participant_b), "war party")?;
            if war.participant_a >= war.participant_b {
                return Err(LivingValidationErrorV2::Range {
                    field: "war participants",
                });
            }
            if war.declarer != war.participant_a && war.declarer != war.participant_b {
                return Err(LivingValidationErrorV2::Range {
                    field: "war declarer",
                });
            }
            interval(war.start_tick, war.end_tick, "war")?;
        }
        Ok(())
    }

    fn check_observations_and_hazards(
        &self,
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> Result<(), LivingValidationErrorV2> {
        for observation in self.observations.as_slice() {
            require(
                has_id(&self.civilizations, observation.civilization),
                "observer",
            )?;
            require(has_id(&self.worlds, observation.world), "observed world")?;
            if let Some(owner) = observation.owner {
                require(has_id(&self.civilizations, owner), "observed owner")?;
            }
            if observation.observed_tick > self.tick {
                return Err(LivingValidationErrorV2::Range {
                    field: "observation observed_tick",
                });
            }
            for facility in observation.facilities.as_slice() {
                require(
                    catalog.industry(facility.definition.as_str()).is_some(),
                    "observed facility definition",
                )?;
            }
            for hull in observation.stationed_hulls.as_slice() {
                require(
                    catalog.hull(hull.definition.as_str()).is_some(),
                    "observed hull definition",
                )?;
            }
        }
        for hazard in self.hazards.as_slice() {
            require(has_id(&self.lanes, hazard.lane), "hazard lane")?;
            require(
                catalog.hazard(hazard.definition.as_str()).is_some(),
                "hazard definition",
            )?;
            interval(hazard.start_tick, hazard.end_tick, "hazard")?;
        }
        Ok(())
    }

    fn check_claims_and_occupations(&self) -> Result<(), LivingValidationErrorV2> {
        for claim in self.settlement_claims.as_slice() {
            require(has_id(&self.worlds, claim.world), "claim world")?;
            require(has_id(&self.civilizations, claim.civilization), "claimant")?;
            interval(claim.arrival_tick, claim.close_tick, "claim window")?;
        }
        // Section 5 opens one window per world, so every claim on a world
        // shares the boundary arbitration runs at.
        for pair in self.settlement_claims.as_slice().windows(2) {
            if pair[0].world == pair[1].world && pair[0].close_tick != pair[1].close_tick {
                return Err(LivingValidationErrorV2::Range {
                    field: "claim close_tick",
                });
            }
        }
        for occupation in self.occupations.as_slice() {
            require(has_id(&self.worlds, occupation.world), "occupation world")?;
            require(
                has_id(&self.civilizations, occupation.claimant),
                "occupation claimant",
            )?;
            if occupation.progress_ticks > LIVING_OCCUPATION_TRANSFER_TICKS_V2 {
                return Err(LivingValidationErrorV2::Range {
                    field: "occupation progress_ticks",
                });
            }
        }
        Ok(())
    }
}

fn has_id<T>(items: &LivingSortedVecV2<T>, id: LivingEntityIdV2) -> bool
where
    T: LivingKeyedV2<Key = LivingEntityIdV2>,
{
    items
        .as_slice()
        .binary_search_by_key(&id, LivingKeyedV2::living_key)
        .is_ok()
}

fn find_world(
    worlds: &LivingSortedVecV2<LivingWorldV2>,
    id: LivingEntityIdV2,
) -> Result<&LivingWorldV2, LivingValidationErrorV2> {
    worlds
        .as_slice()
        .binary_search_by_key(&id, |row| row.id)
        .map(|index| &worlds.as_slice()[index])
        .map_err(|_| LivingValidationErrorV2::DanglingReference {
            reference: "colony world",
        })
}

fn require(present: bool, reference: &'static str) -> Result<(), LivingValidationErrorV2> {
    if present {
        Ok(())
    } else {
        Err(LivingValidationErrorV2::DanglingReference { reference })
    }
}

fn interval(
    start: LivingTickV2,
    end: LivingTickV2,
    field: &'static str,
) -> Result<(), LivingValidationErrorV2> {
    if start.0 < end.0 {
        Ok(())
    } else {
        Err(LivingValidationErrorV2::Interval { field })
    }
}

fn bounded(
    value: i32,
    low: i32,
    high: i32,
    field: &'static str,
) -> Result<(), LivingValidationErrorV2> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(LivingValidationErrorV2::Range { field })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::wire::decode_canonical_v2;

    /// The exact canonical encoding of an empty state, written out rather than
    /// generated: it pins all twenty-four field names, their published order,
    /// and the reserved empty object of field twenty-four.
    const EMPTY_STATE: &[u8] = br#"{"tick":0,"accepted_sequence":0,"branch_sequence":0,"systems":[],"stars":[],"worlds":[],"lanes":[],"civilizations":[],"deposits":[],"colonies":[],"facilities":[],"construction_jobs":[],"hull_jobs":[],"fleets":[],"routes":[],"shipments":[],"relations":[],"agreements":[],"wars":[],"observations":[],"hazards":[],"settlement_claims":[],"occupations":[],"counters":{}}"#;

    #[test]
    fn the_empty_state_encodes_the_published_field_order() {
        let state = LivingGalaxyStateV2::default();
        assert_eq!(state.canonical_bytes().expect("encodes"), EMPTY_STATE);
    }

    #[test]
    fn the_empty_state_decodes_back_from_its_own_canonical_bytes() {
        let decoded: LivingGalaxyStateV2 =
            decode_canonical_v2(EMPTY_STATE, LIVING_MAX_ARCHIVE_BYTES_V2).expect("decodes");
        assert_eq!(decoded, LivingGalaxyStateV2::default());
    }

    #[test]
    fn the_reserved_counters_record_encodes_as_an_empty_object() {
        let bytes = serde_json::to_vec(&LivingCountersV2::default()).expect("encodes");
        assert_eq!(bytes, b"{}");
    }

    #[test]
    fn the_state_carries_no_allocator_high_water_field() {
        let text = str::from_utf8(EMPTY_STATE).expect("ascii");
        assert!(text.contains(r#""accepted_sequence":0"#));
        assert!(text.contains(r#""branch_sequence":0"#));
        assert!(!text.contains("high_water"));
    }

    #[test]
    fn a_relation_total_is_the_clamped_sum_of_base_and_counters() {
        let relation = LivingRelationV2 {
            from: LivingEntityIdV2([1; 16]),
            to: LivingEntityIdV2([2; 16]),
            base_disposition: 90,
            reasons: LivingRelationReasonsV2 {
                delivered_aid: 20,
                successful_trade: 20,
                ..LivingRelationReasonsV2::default()
            },
        };
        assert_eq!(relation.total(), LIVING_RELATION_BOUND_V2);
    }

    #[test]
    fn blocked_reasons_sort_by_discriminant_then_resource() {
        let reasons = vec![
            LivingBlockedReasonV2::MissingDeposit {
                required: 4,
                available: 1,
            },
            LivingBlockedReasonV2::MissingInput {
                resource: LivingResourceV2::Energy,
                required: 4,
                available: 3,
            },
            LivingBlockedReasonV2::OutputFull {
                resource: LivingResourceV2::Alloy,
            },
        ];
        assert!(LivingSortedVecV2::new(reasons).is_ok());
    }

    #[test]
    fn an_out_of_order_blocked_reason_set_is_rejected() {
        let reasons = vec![
            LivingBlockedReasonV2::OutputFull {
                resource: LivingResourceV2::Alloy,
            },
            LivingBlockedReasonV2::MissingDeposit {
                required: 4,
                available: 1,
            },
        ];
        assert!(LivingSortedVecV2::new(reasons).is_err());
    }
}
