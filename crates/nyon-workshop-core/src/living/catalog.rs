//! The validated Living Galaxy V2 rules catalog and its built-in pack.
//!
//! Every numeric value in `assets/living/core-pack-v2.json` is normative text
//! from `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`
//! sections 3, 4, 5 and 7. This module owns only validation, canonical bytes
//! and typed lookup; it decides no rules of its own.
//!
//! The wire contract is section 10: `kind="NYON_LIVING_GALAXY_DATA"`,
//! `format_version=2`, `rules_version=2`, and the top-level fields in exactly
//! the declared order `kind`, `format_version`, `rules_version`, `pack_id`,
//! `pack_version`, `star_archetypes`, `world_archetypes`, `resources`,
//! `industry_definitions`, `hull_definitions`, `hazard_definitions`,
//! `policy_definitions`. Field order here is the wire order, so reordering a
//! struct field is a breaking format change and not a cosmetic edit.
//!
//! Star and world archetypes deliberately carry identity and name only. The
//! generation parameters they will need belong to the genesis task that
//! consumes them; adding fields later is a `pack_version` bump, which is why
//! the built-in pack declares one.

use serde::{Deserialize, Serialize};

use super::ids::{LivingCatalogHashV2, catalog_hash_v2};
use super::wire::{
    LIVING_MAX_PACK_BYTES_V2, LivingKeyedV2, LivingSortedVecV2, LivingWireErrorV2,
    decode_canonical_v2,
};

/// The only document kind this module accepts.
pub const LIVING_PACK_KIND_V2: &str = "NYON_LIVING_GALAXY_DATA";
/// Wire format version of a Living V2 catalog pack.
pub const LIVING_PACK_FORMAT_VERSION_V2: u32 = 2;

/// Section 3: "Inventory is world-local and capped at 10000 units per
/// resource."
pub const LIVING_RESOURCE_STORAGE_LIMIT_V2: u32 = 10_000;
/// Section 3: "Each colony has a persistent hub, six industrial slots".
pub const LIVING_COLONY_INDUSTRY_SLOTS_V2: u32 = 6;

/// The bytes of the built-in validated rules pack.
///
/// Section 10: "The initial rules release ships these values as one built-in
/// validated pack, not remotely fetched content."
pub const LIVING_CORE_PACK_BYTES_V2: &[u8] =
    include_bytes!("../../../../assets/living/core-pack-v2.json");

/// Every way a Living V2 catalog pack can fail validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingCatalogErrorV2 {
    /// The canonical wire contract rejected the bytes.
    #[error("Living V2 catalog wire error: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// The document is not a Living V2 catalog pack.
    #[error("Living V2 catalog packs declare kind {LIVING_PACK_KIND_V2}")]
    Kind,
    /// The wire format version is not the one this module implements.
    #[error("Living V2 catalog format version {found} is not {LIVING_PACK_FORMAT_VERSION_V2}")]
    FormatVersion {
        /// Version found in the document.
        found: u32,
    },
    /// The rules version is not Living V2's.
    #[error("Living V2 catalog rules version {found} is not 2")]
    RulesVersion {
        /// Version found in the document.
        found: u32,
    },
    /// A recurring effect declared a period of zero ticks, which would run
    /// unboundedly often on a shared boundary.
    #[error("Living V2 recurring effects declare a nonzero period")]
    ZeroPeriod,
    /// A construction job declared a duration of zero ticks, which would
    /// complete in the boundary that paid for it.
    #[error("Living V2 construction declares a nonzero duration")]
    ZeroDuration,
    /// A declared integer falls outside its documented range.
    #[error("Living V2 catalog value is outside its declared range")]
    Range,
    /// A recipe names a resource the pack does not declare.
    #[error("Living V2 recipe references an undeclared resource")]
    DanglingResource,
}

// ------------------------------------------------------------- scalar types

/// A stable lowercase identifier used to sort and address definitions.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LivingSlugV2(Box<str>);

impl LivingSlugV2 {
    /// Build a slug, rejecting anything outside the identifier contract.
    pub fn new(value: impl AsRef<str>) -> Result<Self, LivingCatalogErrorV2> {
        let value = value.as_ref();
        let valid = (1..=64).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && value.starts_with(|character: char| character.is_ascii_lowercase());
        if valid {
            Ok(Self(value.into()))
        } else {
            Err(LivingCatalogErrorV2::Range)
        }
    }

    /// The identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LivingSlugV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// A validated printable-ASCII display name.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LivingNameV2(Box<str>);

impl LivingNameV2 {
    /// Build a name under the printable-ASCII contract of section 10.
    pub fn new(value: impl AsRef<str>) -> Result<Self, LivingCatalogErrorV2> {
        let value = value.as_ref();
        let valid = (1..=64).contains(&value.len())
            && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
            && !value.starts_with(' ')
            && !value.ends_with(' ')
            && !value.contains("  ");
        if valid {
            Ok(Self(value.into()))
        } else {
            Err(LivingCatalogErrorV2::Range)
        }
    }

    /// The name text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LivingNameV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

/// One autonomous action kind a policy can favour.
///
/// These discriminants are a never-reordered schema table, not Rust enum
/// layout: the wire string is the stable key and the sort key.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingIntentKindV2 {
    /// Build a colony ark for an eligible settlement target.
    BuildArk,
    /// Build a defense battery at a threatened owned colony.
    BuildBattery,
    /// Build an escort while below the defense target.
    BuildEscort,
    /// Build an extractor on an observed usable deposit.
    BuildExtractor,
    /// Build a foundry with projected inputs.
    BuildFoundry,
    /// Build a scout while fewer than two exist.
    BuildScout,
    /// Build a missing shipyard when a legal hull job is needed.
    BuildShipyard,
    /// Build a solar array.
    BuildSolarArray,
    /// Defend an attacked owned colony.
    Defend,
    /// Establish a trade route under a bilateral agreement.
    EstablishTradeRoute,
    /// Settle an eligible world.
    Settle,
    /// Supply an owned colony below its reserve of a needed resource.
    Supply,
}

impl LivingIntentKindV2 {
    /// The stable wire discriminant, which is also the sort key.
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::BuildArk => "build_ark",
            Self::BuildBattery => "build_battery",
            Self::BuildEscort => "build_escort",
            Self::BuildExtractor => "build_extractor",
            Self::BuildFoundry => "build_foundry",
            Self::BuildScout => "build_scout",
            Self::BuildShipyard => "build_shipyard",
            Self::BuildSolarArray => "build_solar_array",
            Self::Defend => "defend",
            Self::EstablishTradeRoute => "establish_trade_route",
            Self::Settle => "settle",
            Self::Supply => "supply",
        }
    }
}

// -------------------------------------------------------------- definitions

/// A star archetype available to generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingStarArchetypeV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
}

impl LivingKeyedV2 for LivingStarArchetypeV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// A world archetype available to generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingWorldArchetypeV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
}

impl LivingKeyedV2 for LivingWorldArchetypeV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// One of the three stockpiled resources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingResourceDefinitionV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Per-world inventory cap for this resource.
    pub storage_limit: u32,
}

impl LivingKeyedV2 for LivingResourceDefinitionV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// A quantity of one declared resource.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingResourceAmountV2 {
    /// Identifier of a resource the pack declares.
    pub resource: LivingSlugV2,
    /// Units consumed or produced per recurrence.
    pub quantity: u32,
}

impl LivingKeyedV2 for LivingResourceAmountV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.resource.clone()
    }
}

/// What an industrial facility does once built.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingIndustryEffectV2 {
    /// A recurring conversion evaluated on its due boundary.
    Recipe {
        /// Ticks between due boundaries.
        period_ticks: u32,
        /// Units drawn from the world's finite deposit per recurrence.
        deposit_units: u32,
        /// Consumed inventory, sorted by resource.
        inputs: LivingSortedVecV2<LivingResourceAmountV2>,
        /// Produced inventory, sorted by resource.
        outputs: LivingSortedVecV2<LivingResourceAmountV2>,
    },
    /// Builds hulls rather than converting resources.
    HullAssembly {
        /// Hull jobs the facility runs at once.
        concurrent_jobs: u32,
    },
    /// Contributes to colony defense during combat rounds.
    Defense {
        /// Structure hit points.
        hit_points: u32,
        /// Damage dealt each combat round.
        damage_per_round: u32,
    },
}

/// A buildable industrial facility.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingIndustryDefinitionV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Alloy paid up front, per section 3.
    pub alloy_cost: u32,
    /// Ticks from acceptance to completion.
    pub build_ticks: u32,
    /// Industrial slots consumed of the colony's six.
    pub slot_cost: u32,
    /// How many may exist at one colony.
    pub max_per_colony: u32,
    /// Behaviour once complete.
    pub effect: LivingIndustryEffectV2,
}

impl LivingKeyedV2 for LivingIndustryDefinitionV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// A buildable hull.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHullDefinitionV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Alloy paid up front.
    pub alloy_cost: u32,
    /// Ticks from acceptance to completion.
    pub build_ticks: u32,
    /// Maximum hit points.
    pub hit_points: u32,
    /// Damage dealt each combat round.
    pub damage: u32,
}

impl LivingKeyedV2 for LivingHullDefinitionV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// What a scheduled hazard does while active.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingHazardEffectV2 {
    /// Scales a freight route's per-dispatch batch by an exact ratio.
    ///
    /// A ratio rather than a fraction because V2 canonical data carries no
    /// floating-point values, and because section 5 requires the reduction to
    /// round down.
    FreightBatchScale {
        /// Ratio numerator.
        numerator: u32,
        /// Ratio denominator; never zero.
        denominator: u32,
    },
}

impl LivingHazardEffectV2 {
    /// The declared batch ratio as `(numerator, denominator)`.
    pub const fn freight_batch_scale(&self) -> (u32, u32) {
        match self {
            Self::FreightBatchScale {
                numerator,
                denominator,
            } => (*numerator, *denominator),
        }
    }

    /// Apply the hazard to one dispatch batch, rounding down.
    pub const fn scale_batch(&self, batch: u32) -> u32 {
        let (numerator, denominator) = self.freight_batch_scale();
        batch / denominator * numerator + batch % denominator * numerator / denominator
    }
}

/// A hazard the creator can schedule.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHazardDefinitionV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Behaviour while the hazard is active.
    pub effect: LivingHazardEffectV2,
}

impl LivingKeyedV2 for LivingHazardDefinitionV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

/// One scoring bonus a policy applies to an action kind.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingPolicyBonusV2 {
    /// The favoured action kind.
    pub intent: LivingIntentKindV2,
    /// Points added to that action's base score.
    pub bonus: u32,
}

impl LivingKeyedV2 for LivingPolicyBonusV2 {
    type Key = &'static str;

    fn living_key(&self) -> Self::Key {
        self.intent.as_wire_str()
    }
}

/// A civilization policy preset.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingPolicyDefinitionV2 {
    /// Stable identifier.
    pub id: LivingSlugV2,
    /// Display name.
    pub name: LivingNameV2,
    /// Scoring bonuses, sorted by action kind.
    pub bonuses: LivingSortedVecV2<LivingPolicyBonusV2>,
}

impl LivingKeyedV2 for LivingPolicyDefinitionV2 {
    type Key = LivingSlugV2;

    fn living_key(&self) -> Self::Key {
        self.id.clone()
    }
}

// ------------------------------------------------------------- the document

/// The decoded catalog document, before validation.
///
/// Field order is the normative wire order. Reordering these fields changes
/// the canonical bytes and therefore the pack hash.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct LivingCatalogDocumentV2 {
    kind: String,
    format_version: u32,
    rules_version: u32,
    pack_id: LivingSlugV2,
    pack_version: u32,
    star_archetypes: LivingSortedVecV2<LivingStarArchetypeV2>,
    world_archetypes: LivingSortedVecV2<LivingWorldArchetypeV2>,
    resources: LivingSortedVecV2<LivingResourceDefinitionV2>,
    industry_definitions: LivingSortedVecV2<LivingIndustryDefinitionV2>,
    hull_definitions: LivingSortedVecV2<LivingHullDefinitionV2>,
    hazard_definitions: LivingSortedVecV2<LivingHazardDefinitionV2>,
    policy_definitions: LivingSortedVecV2<LivingPolicyDefinitionV2>,
}

/// A catalog pack that has passed every validation obligation, together with
/// the exact bytes it was validated from and their hash.
///
/// The document is private: callers reach it through the typed lookups below,
/// so no consumer can hold an unvalidated definition or reorder a collection
/// after the hash was taken.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedLivingCatalogPackV2 {
    document: LivingCatalogDocumentV2,
    canonical_bytes: Vec<u8>,
    hash: LivingCatalogHashV2,
}

/// Decode and validate one Living V2 catalog pack.
pub fn decode_living_catalog_pack_v2(
    bytes: &[u8],
) -> Result<ValidatedLivingCatalogPackV2, LivingCatalogErrorV2> {
    let document: LivingCatalogDocumentV2 =
        decode_canonical_v2(bytes, LIVING_MAX_PACK_BYTES_V2).map_err(LivingCatalogErrorV2::Wire)?;
    validate(&document)?;
    Ok(ValidatedLivingCatalogPackV2 {
        document,
        canonical_bytes: bytes.to_vec(),
        hash: catalog_hash_v2(bytes),
    })
}

/// The built-in validated rules pack shipped with this release.
pub fn living_core_pack_v2() -> Result<ValidatedLivingCatalogPackV2, LivingCatalogErrorV2> {
    decode_living_catalog_pack_v2(LIVING_CORE_PACK_BYTES_V2)
}

fn validate(document: &LivingCatalogDocumentV2) -> Result<(), LivingCatalogErrorV2> {
    if document.kind != LIVING_PACK_KIND_V2 {
        return Err(LivingCatalogErrorV2::Kind);
    }
    if document.format_version != LIVING_PACK_FORMAT_VERSION_V2 {
        return Err(LivingCatalogErrorV2::FormatVersion {
            found: document.format_version,
        });
    }
    if document.rules_version != super::LIVING_RULES_VERSION {
        return Err(LivingCatalogErrorV2::RulesVersion {
            found: document.rules_version,
        });
    }
    if document.pack_version == 0 {
        return Err(LivingCatalogErrorV2::Range);
    }

    for resource in document.resources.as_slice() {
        if resource.storage_limit == 0 || resource.storage_limit > LIVING_RESOURCE_STORAGE_LIMIT_V2
        {
            return Err(LivingCatalogErrorV2::Range);
        }
    }

    for industry in document.industry_definitions.as_slice() {
        if industry.build_ticks == 0 {
            return Err(LivingCatalogErrorV2::ZeroDuration);
        }
        if industry.slot_cost == 0
            || industry.slot_cost > LIVING_COLONY_INDUSTRY_SLOTS_V2
            || industry.max_per_colony == 0
            || industry.max_per_colony > LIVING_COLONY_INDUSTRY_SLOTS_V2
        {
            return Err(LivingCatalogErrorV2::Range);
        }
        match &industry.effect {
            LivingIndustryEffectV2::Recipe {
                period_ticks,
                deposit_units,
                inputs,
                outputs,
            } => {
                if *period_ticks == 0 {
                    return Err(LivingCatalogErrorV2::ZeroPeriod);
                }
                if *deposit_units > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
                    return Err(LivingCatalogErrorV2::Range);
                }
                if inputs.as_slice().is_empty()
                    && outputs.as_slice().is_empty()
                    && *deposit_units == 0
                {
                    return Err(LivingCatalogErrorV2::Range);
                }
                for amount in inputs.as_slice().iter().chain(outputs.as_slice()) {
                    check_amount(document, amount)?;
                }
            }
            LivingIndustryEffectV2::HullAssembly { concurrent_jobs } => {
                if *concurrent_jobs == 0 || *concurrent_jobs > LIVING_COLONY_INDUSTRY_SLOTS_V2 {
                    return Err(LivingCatalogErrorV2::Range);
                }
            }
            LivingIndustryEffectV2::Defense {
                hit_points,
                damage_per_round,
            } => {
                if *hit_points == 0 || *hit_points > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
                    return Err(LivingCatalogErrorV2::Range);
                }
                if *damage_per_round > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
                    return Err(LivingCatalogErrorV2::Range);
                }
            }
        }
    }

    for hull in document.hull_definitions.as_slice() {
        if hull.build_ticks == 0 {
            return Err(LivingCatalogErrorV2::ZeroDuration);
        }
        if hull.hit_points == 0
            || hull.hit_points > LIVING_RESOURCE_STORAGE_LIMIT_V2
            || hull.damage > LIVING_RESOURCE_STORAGE_LIMIT_V2
        {
            return Err(LivingCatalogErrorV2::Range);
        }
    }

    for hazard in document.hazard_definitions.as_slice() {
        let (numerator, denominator) = hazard.effect.freight_batch_scale();
        if denominator == 0 || numerator > denominator {
            return Err(LivingCatalogErrorV2::Range);
        }
    }

    for policy in document.policy_definitions.as_slice() {
        for bonus in policy.bonuses.as_slice() {
            if bonus.bonus == 0 || bonus.bonus > 1_000 {
                return Err(LivingCatalogErrorV2::Range);
            }
        }
    }

    Ok(())
}

fn check_amount(
    document: &LivingCatalogDocumentV2,
    amount: &LivingResourceAmountV2,
) -> Result<(), LivingCatalogErrorV2> {
    if amount.quantity == 0 || amount.quantity > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
        return Err(LivingCatalogErrorV2::Range);
    }
    let declared = document
        .resources
        .as_slice()
        .iter()
        .any(|resource| resource.id == amount.resource);
    if declared {
        Ok(())
    } else {
        Err(LivingCatalogErrorV2::DanglingResource)
    }
}

impl ValidatedLivingCatalogPackV2 {
    /// The exact bytes this pack was validated from.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// The pack's canonical hash, bound into every document that uses it.
    pub const fn catalog_hash(&self) -> LivingCatalogHashV2 {
        self.hash
    }

    /// The pack's stable identifier.
    pub fn pack_id(&self) -> &str {
        self.document.pack_id.as_str()
    }

    /// The pack's own content version.
    pub const fn pack_version(&self) -> u32 {
        self.document.pack_version
    }

    /// The wire format version, always [`LIVING_PACK_FORMAT_VERSION_V2`].
    pub const fn format_version(&self) -> u32 {
        self.document.format_version
    }

    /// The rules version, always Living V2's.
    pub const fn rules_version(&self) -> u32 {
        self.document.rules_version
    }

    /// Star archetypes in stable order.
    pub fn star_archetypes(&self) -> &[LivingStarArchetypeV2] {
        self.document.star_archetypes.as_slice()
    }

    /// World archetypes in stable order.
    pub fn world_archetypes(&self) -> &[LivingWorldArchetypeV2] {
        self.document.world_archetypes.as_slice()
    }

    /// Resources in stable order.
    pub fn resources(&self) -> &[LivingResourceDefinitionV2] {
        self.document.resources.as_slice()
    }

    /// Industrial facilities in stable order.
    pub fn industries(&self) -> &[LivingIndustryDefinitionV2] {
        self.document.industry_definitions.as_slice()
    }

    /// Hulls in stable order.
    pub fn hulls(&self) -> &[LivingHullDefinitionV2] {
        self.document.hull_definitions.as_slice()
    }

    /// Hazards in stable order.
    pub fn hazards(&self) -> &[LivingHazardDefinitionV2] {
        self.document.hazard_definitions.as_slice()
    }

    /// Policy presets in stable order.
    pub fn policies(&self) -> &[LivingPolicyDefinitionV2] {
        self.document.policy_definitions.as_slice()
    }

    /// Look one resource up by identifier.
    pub fn resource(&self, id: &str) -> Option<&LivingResourceDefinitionV2> {
        self.resources().iter().find(|row| row.id.as_str() == id)
    }

    /// Look one facility up by identifier.
    pub fn industry(&self, id: &str) -> Option<&LivingIndustryDefinitionV2> {
        self.industries().iter().find(|row| row.id.as_str() == id)
    }

    /// Look one hull up by identifier.
    pub fn hull(&self, id: &str) -> Option<&LivingHullDefinitionV2> {
        self.hulls().iter().find(|row| row.id.as_str() == id)
    }

    /// Look one hazard up by identifier.
    pub fn hazard(&self, id: &str) -> Option<&LivingHazardDefinitionV2> {
        self.hazards().iter().find(|row| row.id.as_str() == id)
    }

    /// Look one policy up by identifier.
    pub fn policy(&self, id: &str) -> Option<&LivingPolicyDefinitionV2> {
        self.policies().iter().find(|row| row.id.as_str() == id)
    }

    /// The bonus a policy applies to one action kind, or zero.
    ///
    /// An unknown policy or action kind scores zero rather than failing:
    /// scoring asks this question for every candidate, and section 7 gives
    /// Neutral no bonus at all.
    pub fn policy_bonus(&self, policy_id: &str, intent: &str) -> u32 {
        self.policy(policy_id)
            .and_then(|policy| {
                policy
                    .bonuses
                    .as_slice()
                    .iter()
                    .find(|row| row.intent.as_wire_str() == intent)
            })
            .map_or(0, |row| row.bonus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_rejects_uppercase_leading_digits_and_empty_text() {
        assert!(LivingSlugV2::new("solar_array").is_ok());
        assert!(LivingSlugV2::new("Solar").is_err());
        assert!(LivingSlugV2::new("2nd").is_err());
        assert!(LivingSlugV2::new("").is_err());
    }

    #[test]
    fn a_name_rejects_padding_runs_and_control_bytes() {
        assert!(LivingNameV2::new("Solar Array").is_ok());
        assert!(LivingNameV2::new(" Solar").is_err());
        assert!(LivingNameV2::new("Solar ").is_err());
        assert!(LivingNameV2::new("Solar  Array").is_err());
        assert!(LivingNameV2::new("Solar\tArray").is_err());
    }

    #[test]
    fn the_hazard_ratio_rounds_down_without_overflowing_a_large_batch() {
        let half = LivingHazardEffectV2::FreightBatchScale {
            numerator: 1,
            denominator: 2,
        };
        assert_eq!(half.scale_batch(0), 0);
        assert_eq!(half.scale_batch(7), 3);
        assert_eq!(half.scale_batch(u32::MAX), u32::MAX / 2);

        let two_thirds = LivingHazardEffectV2::FreightBatchScale {
            numerator: 2,
            denominator: 3,
        };
        assert_eq!(two_thirds.scale_batch(10), 6);
        assert_eq!(two_thirds.scale_batch(u32::MAX), 2_863_311_530);
    }
}

// Private move-only catalog reclamation. Retain incomplete jobs and deferred
// tickets: ordinary Drop is not a bounded cancellation path. Record accounting
// does not qualify allocator duration, including canonical-byte deallocation.
#[derive(Default, Debug, Eq, PartialEq)]
struct CatalogRetirementCounts {
    buffer_moves: u64,
    rows: u64,
    nested_records: u64,
    storage_releases: u64,
    parent_releases: u64,
    document_releases: u64,
    root_releases: u64,
    shared_attempts: u64,
}
enum CatalogRetirementRows {
    Stars(std::vec::IntoIter<LivingStarArchetypeV2>),
    Worlds(std::vec::IntoIter<LivingWorldArchetypeV2>),
    Resources(std::vec::IntoIter<LivingResourceDefinitionV2>),
    Industries(std::vec::IntoIter<LivingIndustryDefinitionV2>),
    Hulls(std::vec::IntoIter<LivingHullDefinitionV2>),
    Hazards(std::vec::IntoIter<LivingHazardDefinitionV2>),
    Policies(std::vec::IntoIter<LivingPolicyDefinitionV2>),
}
impl CatalogRetirementRows {
    fn begin(document: &mut LivingCatalogDocumentV2, index: usize) -> Self {
        match index {
            0 => Self::Stars(
                std::mem::take(&mut document.star_archetypes)
                    .into_vec()
                    .into_iter(),
            ),
            1 => Self::Worlds(
                std::mem::take(&mut document.world_archetypes)
                    .into_vec()
                    .into_iter(),
            ),
            2 => Self::Resources(
                std::mem::take(&mut document.resources)
                    .into_vec()
                    .into_iter(),
            ),
            3 => Self::Industries(
                std::mem::take(&mut document.industry_definitions)
                    .into_vec()
                    .into_iter(),
            ),
            4 => Self::Hulls(
                std::mem::take(&mut document.hull_definitions)
                    .into_vec()
                    .into_iter(),
            ),
            5 => Self::Hazards(
                std::mem::take(&mut document.hazard_definitions)
                    .into_vec()
                    .into_iter(),
            ),
            6 => Self::Policies(
                std::mem::take(&mut document.policy_definitions)
                    .into_vec()
                    .into_iter(),
            ),
            _ => unreachable!("catalog collection index"),
        }
    }
    fn remaining(&self) -> usize {
        match self {
            Self::Stars(rows) => rows.len(),
            Self::Worlds(rows) => rows.len(),
            Self::Resources(rows) => rows.len(),
            Self::Industries(rows) => rows.len(),
            Self::Hulls(rows) => rows.len(),
            Self::Hazards(rows) => rows.len(),
            Self::Policies(rows) => rows.len(),
        }
    }
    fn retire_row(&mut self, parent: &mut Option<CatalogRetirementParent>) {
        match self {
            Self::Stars(rows) => {
                drop(rows.next().expect("remaining row"));
            }
            Self::Worlds(rows) => {
                drop(rows.next().expect("remaining row"));
            }
            Self::Resources(rows) => {
                drop(rows.next().expect("remaining row"));
            }
            Self::Industries(rows) => {
                *parent = Some(CatalogRetirementParent::Industries(
                    rows.next().expect("remaining row"),
                ))
            }
            Self::Hulls(rows) => {
                drop(rows.next().expect("remaining row"));
            }
            Self::Hazards(rows) => {
                drop(rows.next().expect("remaining row"));
            }
            Self::Policies(rows) => {
                *parent = Some(CatalogRetirementParent::Policies(
                    rows.next().expect("remaining row"),
                ))
            }
        }
    }
}
enum CatalogRetirementParent {
    Industries(LivingIndustryDefinitionV2),
    Policies(LivingPolicyDefinitionV2),
}
enum CatalogRetirementChildren {
    Amounts(std::vec::IntoIter<LivingResourceAmountV2>),
    Bonuses(std::vec::IntoIter<LivingPolicyBonusV2>),
}
impl CatalogRetirementChildren {
    fn remaining(&self) -> usize {
        match self {
            Self::Amounts(rows) => rows.len(),
            Self::Bonuses(rows) => rows.len(),
        }
    }
    fn retire_record(&mut self) {
        match self {
            Self::Amounts(rows) => drop(rows.next().expect("remaining amount")),
            Self::Bonuses(rows) => {
                rows.next().expect("remaining bonus");
            }
        }
    }
}
impl CatalogRetirementParent {
    fn child(&mut self, index: usize) -> Option<CatalogRetirementChildren> {
        match (self, index) {
            (
                Self::Industries(LivingIndustryDefinitionV2 {
                    effect: LivingIndustryEffectV2::Recipe { inputs, .. },
                    ..
                }),
                0,
            ) => Some(CatalogRetirementChildren::Amounts(
                std::mem::take(inputs).into_vec().into_iter(),
            )),
            (
                Self::Industries(LivingIndustryDefinitionV2 {
                    effect: LivingIndustryEffectV2::Recipe { outputs, .. },
                    ..
                }),
                1,
            ) => Some(CatalogRetirementChildren::Amounts(
                std::mem::take(outputs).into_vec().into_iter(),
            )),
            (Self::Policies(row), 0) => Some(CatalogRetirementChildren::Bonuses(
                std::mem::take(&mut row.bonuses).into_vec().into_iter(),
            )),
            _ => None,
        }
    }
    fn child_count(&self) -> usize {
        match self {
            Self::Industries(row) => {
                if matches!(row.effect, LivingIndustryEffectV2::Recipe { .. }) {
                    2
                } else {
                    0
                }
            }
            Self::Policies(_) => 1,
        }
    }
}
#[must_use = "Retain or resume this exact catalog owner through counted retirement."]
pub(crate) struct LivingDeferredCatalogRetirementV2 {
    source: std::sync::Arc<ValidatedLivingCatalogPackV2>,
}
#[must_use = "SharedReleased releases a handle; Deferred retains the catalog payload."]
pub(crate) enum LivingCatalogRetirementDispositionV2 {
    Drained,
    SharedReleased,
    Deferred(LivingDeferredCatalogRetirementV2),
}
pub(crate) struct LivingCatalogRetirementJobV2 {
    owned: Option<ValidatedLivingCatalogPackV2>,
    boxed: Option<Box<ValidatedLivingCatalogPackV2>>,
    shared: Option<(
        std::sync::Arc<ValidatedLivingCatalogPackV2>,
        super::step_job::LivingStateRetirementPolicyV2,
    )>,
    rows: Option<CatalogRetirementRows>,
    parent: Option<CatalogRetirementParent>,
    children: Option<CatalogRetirementChildren>,
    collection: usize,
    child_index: usize,
    bytes_released: bool,
    document_released: bool,
    ready: bool,
    delivered: bool,
    disposition: Option<LivingCatalogRetirementDispositionV2>,
    last_poll_visits: usize,
    total_visits: u64,
    counts: CatalogRetirementCounts,
}
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private catalog retirement precedes child integration."
    )
)]
impl LivingCatalogRetirementJobV2 {
    fn empty() -> Self {
        Self {
            owned: None,
            boxed: None,
            shared: None,
            rows: None,
            parent: None,
            children: None,
            collection: 0,
            child_index: 0,
            bytes_released: false,
            document_released: false,
            ready: false,
            delivered: false,
            disposition: None,
            last_poll_visits: 0,
            total_visits: 0,
            counts: Default::default(),
        }
    }
    pub(crate) fn begin_owned(pack: ValidatedLivingCatalogPackV2) -> Self {
        Self {
            owned: Some(pack),
            ..Self::empty()
        }
    }
    pub(crate) fn begin_boxed(pack: Box<ValidatedLivingCatalogPackV2>) -> Self {
        Self {
            boxed: Some(pack),
            ..Self::empty()
        }
    }
    pub(crate) fn begin_shared(
        source: std::sync::Arc<ValidatedLivingCatalogPackV2>,
        policy: super::step_job::LivingStateRetirementPolicyV2,
    ) -> Self {
        Self {
            shared: Some((source, policy)),
            ..Self::empty()
        }
    }
    pub(crate) fn resume(ticket: LivingDeferredCatalogRetirementV2) -> Self {
        Self::begin_shared(
            ticket.source,
            super::step_job::LivingStateRetirementPolicyV2::DeferUntilUnique,
        )
    }
    pub(crate) const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    pub(crate) const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        self.last_poll_visits = 0;
        if budget == 0 || budget > super::step_job::LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(super::step_job::LivingLifecycleJobErrorV2::Budget);
        }
        while self.last_poll_visits < budget && !self.ready {
            self.visit();
            self.last_poll_visits += 1;
            self.total_visits += 1;
        }
        Ok(if self.ready {
            super::step_job::LivingLifecycleProgressV2::Ready
        } else {
            super::step_job::LivingLifecycleProgressV2::Pending
        })
    }
    pub(crate) fn finish(
        &mut self,
    ) -> Result<LivingCatalogRetirementDispositionV2, super::step_job::LivingLifecycleJobErrorV2>
    {
        if !self.ready || self.delivered {
            return Err(super::step_job::LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(self.disposition.take().expect("ready catalog disposition"))
    }
    fn visit(&mut self) {
        use super::step_job::LivingStateRetirementPolicyV2;
        use std::sync::Arc;
        if let Some((source, policy)) = self.shared.take() {
            self.counts.shared_attempts += 1;
            let payload = match policy {
                LivingStateRetirementPolicyV2::ReleaseHandle => match Arc::into_inner(source) {
                    Some(pack) => Some(pack),
                    None => {
                        self.disposition =
                            Some(LivingCatalogRetirementDispositionV2::SharedReleased);
                        None
                    }
                },
                LivingStateRetirementPolicyV2::DeferUntilUnique => match Arc::try_unwrap(source) {
                    Ok(pack) => Some(pack),
                    Err(source) => {
                        self.disposition = Some(LivingCatalogRetirementDispositionV2::Deferred(
                            LivingDeferredCatalogRetirementV2 { source },
                        ));
                        None
                    }
                },
            };
            if let Some(pack) = payload {
                self.owned = Some(pack);
            } else {
                self.ready = true;
            }
            return;
        }
        if let Some(children) = &mut self.children {
            if children.remaining() != 0 {
                children.retire_record();
                self.counts.nested_records += 1;
            } else {
                drop(self.children.take().expect("exhausted children"));
                self.counts.storage_releases += 1;
                self.child_index += 1;
            }
            return;
        }
        if let Some(parent) = &mut self.parent {
            if self.child_index < parent.child_count() {
                self.children = parent.child(self.child_index);
                assert!(self.children.is_some(), "declared catalog child");
                self.counts.buffer_moves += 1;
                return;
            }
            drop(self.parent.take().expect("empty catalog parent"));
            self.counts.parent_releases += 1;
            self.child_index = 0;
            return;
        }
        if let Some(rows) = &mut self.rows {
            if rows.remaining() != 0 {
                rows.retire_row(&mut self.parent);
                self.counts.rows += 1;
            } else {
                drop(self.rows.take().expect("exhausted rows"));
                self.counts.storage_releases += 1;
                self.collection += 1;
            }
            return;
        }
        let pack = self
            .owned
            .as_mut()
            .or(self.boxed.as_deref_mut())
            .expect("catalog owner");
        if self.collection < 7 {
            self.rows = Some(CatalogRetirementRows::begin(
                &mut pack.document,
                self.collection,
            ));
            self.counts.buffer_moves += 1;
            return;
        }
        if !self.bytes_released {
            drop(std::mem::take(&mut pack.canonical_bytes));
            self.counts.storage_releases += 1;
            self.bytes_released = true;
            return;
        }
        if !self.document_released {
            // The private, no-longer-observable document has no collection owners.
            // Empty replacements allocate no storage and need no validation.
            drop(std::mem::take(&mut pack.document.kind));
            drop(std::mem::take(&mut pack.document.pack_id.0));
            self.counts.document_releases += 1;
            self.document_released = true;
            return;
        }
        drop(self.owned.take());
        drop(self.boxed.take());
        self.counts.root_releases += 1;
        self.disposition = Some(LivingCatalogRetirementDispositionV2::Drained);
        self.ready = true;
    }
}

#[cfg(test)]
mod retirement_tests {
    use super::super::step_job::{
        LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingStateRetirementPolicyV2,
    };
    use super::*;
    use std::sync::Arc;

    fn run(
        mut job: LivingCatalogRetirementJobV2,
        budgets: &[usize],
    ) -> (
        LivingCatalogRetirementDispositionV2,
        CatalogRetirementCounts,
        u64,
    ) {
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        let mut index = 0;
        while job.poll(budgets[index % budgets.len()]).unwrap() != LivingLifecycleProgressV2::Ready
        {
            assert!(job.last_poll_visits() <= budgets[index % budgets.len()]);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            ));
            index += 1;
        }
        let total = job.total_visits();
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), total);
        let result = job.finish().unwrap();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        (result, job.counts, total)
    }
    fn expected(pack: &ValidatedLivingCatalogPackV2) -> CatalogRetirementCounts {
        let mut counts = CatalogRetirementCounts {
            buffer_moves: 7,
            rows: (pack.star_archetypes().len()
                + pack.world_archetypes().len()
                + pack.resources().len()
                + pack.industries().len()
                + pack.hulls().len()
                + pack.hazards().len()
                + pack.policies().len()) as u64,
            storage_releases: 8,
            document_releases: 1,
            root_releases: 1,
            ..Default::default()
        };
        for industry in pack.industries() {
            counts.parent_releases += 1;
            if let LivingIndustryEffectV2::Recipe {
                inputs, outputs, ..
            } = &industry.effect
            {
                counts.buffer_moves += 2;
                counts.storage_releases += 2;
                counts.nested_records +=
                    (inputs.as_slice().len() + outputs.as_slice().len()) as u64;
            }
        }
        for policy in pack.policies() {
            counts.parent_releases += 1;
            counts.buffer_moves += 1;
            counts.storage_releases += 1;
            counts.nested_records += policy.bonuses.as_slice().len() as u64;
        }
        counts
    }
    fn sum(c: &CatalogRetirementCounts) -> u64 {
        c.buffer_moves
            + c.rows
            + c.nested_records
            + c.storage_releases
            + c.parent_releases
            + c.document_releases
            + c.root_releases
            + c.shared_attempts
    }
    #[test]
    fn catalog_retirement_every_collection_and_nested_field_owned_boxed() {
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 1024, 7]] {
            for boxed in [false, true] {
                let pack = living_core_pack_v2().unwrap();
                assert!(
                    !pack.star_archetypes().is_empty()
                        && !pack.world_archetypes().is_empty()
                        && !pack.resources().is_empty()
                        && !pack.industries().is_empty()
                        && !pack.hulls().is_empty()
                        && !pack.hazards().is_empty()
                        && !pack.policies().is_empty()
                );
                let counts = expected(&pack);
                assert_eq!(
                    counts,
                    CatalogRetirementCounts {
                        buffer_moves: 18,
                        rows: 21,
                        nested_records: 15,
                        storage_releases: 19,
                        parent_releases: 10,
                        document_releases: 1,
                        root_releases: 1,
                        shared_attempts: 0
                    }
                );
                let job = if boxed {
                    let owner = Box::new(pack);
                    let pointer = &*owner as *const _;
                    let job = LivingCatalogRetirementJobV2::begin_boxed(owner);
                    assert_eq!(job.boxed.as_deref().unwrap() as *const _, pointer);
                    job
                } else {
                    let pointer = pack.canonical_bytes.as_ptr();
                    let job = LivingCatalogRetirementJobV2::begin_owned(pack);
                    assert_eq!(
                        job.owned.as_ref().unwrap().canonical_bytes.as_ptr(),
                        pointer
                    );
                    job
                };
                let (out, actual, total) = run(job, budgets);
                assert!(matches!(out, LivingCatalogRetirementDispositionV2::Drained));
                assert_eq!(actual, counts);
                assert_eq!(total, sum(&counts));
            }
        }
    }
    #[test]
    fn catalog_retirement_empty_allocated_outer_and_canonical_buffers() {
        let mut pack = living_core_pack_v2().unwrap();
        macro_rules! empty { ($($field:ident),*) => {$({ let mut v = std::mem::take(&mut pack.document.$field).into_vec(); v.clear(); assert!(v.capacity() > 0); pack.document.$field = LivingSortedVecV2::new(v).unwrap(); })*}; }
        empty!(
            star_archetypes,
            world_archetypes,
            resources,
            industry_definitions,
            hull_definitions,
            hazard_definitions,
            policy_definitions
        );
        pack.canonical_bytes.clear();
        assert!(pack.canonical_bytes.capacity() > 0);
        let mut job = LivingCatalogRetirementJobV2::begin_owned(pack);
        for invalid in [0, 1025] {
            assert_eq!(job.poll(invalid), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), 0);
        }
        let (out, counts, total) = run(job, &[1]);
        assert!(matches!(out, LivingCatalogRetirementDispositionV2::Drained));
        assert_eq!(counts.buffer_moves, 7);
        assert_eq!(counts.storage_releases, 8);
        assert_eq!(total, 17);
    }
    #[test]
    fn catalog_retirement_empty_allocated_children_keep_parent_until_release() {
        let mut pack = living_core_pack_v2().unwrap();
        let mut industries = std::mem::take(&mut pack.document.industry_definitions).into_vec();
        for row in &mut industries {
            if let LivingIndustryEffectV2::Recipe {
                inputs, outputs, ..
            } = &mut row.effect
            {
                for child in [inputs, outputs] {
                    let mut v = std::mem::take(child).into_vec();
                    v.clear();
                    v.reserve(16);
                    *child = LivingSortedVecV2::new(v).unwrap();
                }
            }
        }
        pack.document.industry_definitions = LivingSortedVecV2::new(industries).unwrap();
        let mut policies = std::mem::take(&mut pack.document.policy_definitions).into_vec();
        for row in &mut policies {
            let mut v = std::mem::take(&mut row.bonuses).into_vec();
            v.clear();
            v.reserve(16);
            row.bonuses = LivingSortedVecV2::new(v).unwrap();
        }
        pack.document.policy_definitions = LivingSortedVecV2::new(policies).unwrap();
        let expected = expected(&pack);
        let mut job = LivingCatalogRetirementJobV2::begin_owned(pack);
        while job.children.is_none() {
            job.poll(1).unwrap();
        }
        assert_eq!(job.children.as_ref().unwrap().remaining(), 0);
        assert!(job.parent.is_some());
        let released = job.counts.storage_releases;
        let total = job.total_visits();
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        assert_eq!(job.total_visits(), total);
        job.poll(1).unwrap();
        assert!(job.parent.is_some());
        assert!(job.children.is_none());
        assert_eq!(job.counts.storage_releases, released + 1);
        let (_, actual, _) = run(job, &[1]);
        assert_eq!(actual, expected);
    }
    #[test]
    fn catalog_retirement_partial_iterator_and_nested_suffix_preserved() {
        let pack = living_core_pack_v2().unwrap();
        let expected = expected(&pack);
        let mut job = LivingCatalogRetirementJobV2::begin_owned(pack);
        job.poll(1).unwrap();
        let rows = job.rows.as_ref().unwrap().remaining();
        assert!(rows > 0);
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.rows.as_ref().unwrap().remaining(), rows);
        while job.children.as_ref().is_none_or(|c| c.remaining() == 0) {
            job.poll(1).unwrap();
        }
        let remaining = job.children.as_ref().unwrap().remaining();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        job.poll(1).unwrap();
        assert_eq!(job.children.as_ref().unwrap().remaining(), remaining - 1);
        assert!(job.parent.is_some());
        let (_, actual, _) = run(job, &[1, 64]);
        assert_eq!(actual, expected);
    }
    #[test]
    fn catalog_retirement_shared_release_both_orders_and_concurrent() {
        for reverse in [false, true] {
            let source = Arc::new(living_core_pack_v2().unwrap());
            let alias = Arc::clone(&source);
            let expected = source.canonical_bytes().to_vec();
            let (first, second) = if reverse {
                (alias, source)
            } else {
                (source, alias)
            };
            let (out, counts, total) = run(
                LivingCatalogRetirementJobV2::begin_shared(
                    first,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[1024],
            );
            assert!(matches!(
                out,
                LivingCatalogRetirementDispositionV2::SharedReleased
            ));
            assert_eq!(counts.shared_attempts, 1);
            assert_eq!(total, 1);
            assert_eq!(second.canonical_bytes(), expected);
            let (out, counts, total) = run(
                LivingCatalogRetirementJobV2::begin_shared(
                    second,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[1],
            );
            assert!(matches!(out, LivingCatalogRetirementDispositionV2::Drained));
            assert_eq!(total, sum(&counts));
        }
        let source = Arc::new(living_core_pack_v2().unwrap());
        let alias = Arc::clone(&source);
        let thread = std::thread::spawn(move || {
            run(
                LivingCatalogRetirementJobV2::begin_shared(
                    source,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[1],
            )
            .0
        });
        let other = run(
            LivingCatalogRetirementJobV2::begin_shared(
                alias,
                LivingStateRetirementPolicyV2::ReleaseHandle,
            ),
            &[64],
        )
        .0;
        assert!(matches!(
            (thread.join().unwrap(), other),
            (
                LivingCatalogRetirementDispositionV2::Drained,
                LivingCatalogRetirementDispositionV2::SharedReleased
            ) | (
                LivingCatalogRetirementDispositionV2::SharedReleased,
                LivingCatalogRetirementDispositionV2::Drained
            )
        ));
    }
    #[test]
    fn catalog_retirement_deferred_exact_identity_and_weak_unique_resume() {
        let source = Arc::new(living_core_pack_v2().unwrap());
        let alias = Arc::clone(&source);
        let pointer = Arc::as_ptr(&source);
        let weak = Arc::downgrade(&source);
        let (out, _, total) = run(
            LivingCatalogRetirementJobV2::begin_shared(
                source,
                LivingStateRetirementPolicyV2::DeferUntilUnique,
            ),
            &[1024],
        );
        assert_eq!(total, 1);
        let LivingCatalogRetirementDispositionV2::Deferred(ticket) = out else {
            panic!("deferred");
        };
        assert_eq!(Arc::as_ptr(&ticket.source), pointer);
        let (out, _, total) = run(LivingCatalogRetirementJobV2::resume(ticket), &[1]);
        assert_eq!(total, 1);
        let LivingCatalogRetirementDispositionV2::Deferred(ticket) = out else {
            panic!("deferred");
        };
        assert_eq!(Arc::as_ptr(&ticket.source), pointer);
        assert!(weak.upgrade().is_some());
        drop(alias);
        let (out, counts, total) = run(LivingCatalogRetirementJobV2::resume(ticket), &[64]);
        assert!(matches!(out, LivingCatalogRetirementDispositionV2::Drained));
        assert_eq!(total, sum(&counts));
        assert!(weak.upgrade().is_none());
    }
}

#[cfg(test)]
mod retirement_large_tests {
    use super::*;
    #[test]
    fn catalog_retirement_malformed_large_nested_buffer_remains_sliced() {
        let mut pack = living_core_pack_v2().unwrap();
        pack.document.kind = String::new();
        pack.document.format_version = 999;
        let mut rows = std::mem::take(&mut pack.document.industry_definitions).into_vec();
        let row = rows
            .iter_mut()
            .find(|r| matches!(r.effect, LivingIndustryEffectV2::Recipe { .. }))
            .unwrap();
        let LivingIndustryEffectV2::Recipe {
            inputs,
            period_ticks,
            ..
        } = &mut row.effect
        else {
            unreachable!()
        };
        *period_ticks = 0;
        *inputs = LivingSortedVecV2::new(
            (0..2048)
                .map(|n| LivingResourceAmountV2 {
                    resource: LivingSlugV2::new(format!("undeclared_{n:04}")).unwrap(),
                    quantity: 0,
                })
                .collect(),
        )
        .unwrap();
        pack.document.industry_definitions = LivingSortedVecV2::new(rows).unwrap();
        let mut job = LivingCatalogRetirementJobV2::begin_owned(pack);
        while job.children.as_ref().is_none_or(|c| c.remaining() != 2048) {
            job.poll(1).unwrap();
        }
        assert!(job.parent.is_some());
        job.poll(64).unwrap();
        assert_eq!(job.last_poll_visits(), 64);
        assert_eq!(job.children.as_ref().unwrap().remaining(), 1984);
        assert!(job.parent.is_some());
        while job.poll(1024).unwrap() != super::super::step_job::LivingLifecycleProgressV2::Ready {}
        assert!(matches!(
            job.finish().unwrap(),
            LivingCatalogRetirementDispositionV2::Drained
        ));
        assert_eq!(job.counts.nested_records, 2062);
    }
}
