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
