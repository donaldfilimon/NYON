//! Living Galaxy V2 identities and the frozen SHA-256 hash domains.
//!
//! Every literal and every framing rule in this module is normative text from
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` section 10.
//! Domain literals are ASCII bytes including their terminating NUL, and every
//! fixed-width number is little-endian.
//!
//! These wrappers are deliberately distinct from the WorkshopV1 identities in
//! the sibling `ids` module. No conversion exists in either direction, even
//! where serialized widths match, so a V1 value can never satisfy a V2
//! parameter by accident.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::LIVING_RULES_VERSION;

/// `catalog_hash = SHA256(domain || canonical_catalog_bytes)`.
pub const LIVING_PACK_DOMAIN_V2: &[u8] = b"NYON-LIVING-PACK-V2\0";
/// `state_digest = SHA256(domain || canonical_state_bytes)`.
pub const LIVING_STATE_DOMAIN_V2: &[u8] = b"NYON-LIVING-STATE-V2\0";
/// `archive_integrity = SHA256(domain || canonical_payload_bytes)`.
pub const LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2: &[u8] = b"NYON-LIVING-ARCHIVE-INTEGRITY-V2\0";
/// `receipt_digest = SHA256(domain || canonical_receipt_bytes)`.
pub const LIVING_RECEIPT_DOMAIN_V2: &[u8] = b"NYON-LIVING-RECEIPT-V2\0";
/// Domain of the creator revision identity.
pub const LIVING_REVISION_DOMAIN_V2: &[u8] = b"NYON-LIVING-REVISION-V2\0";
/// Domain of entities created by a creator batch.
pub const LIVING_CREATOR_ENTITY_DOMAIN_V2: &[u8] = b"NYON-LIVING-CREATOR-ENTITY-V2\0";
/// Domain of entities created autonomously during a tick.
pub const LIVING_AUTO_ENTITY_DOMAIN_V2: &[u8] = b"NYON-LIVING-AUTO-ENTITY-V2\0";
/// Domain of the document's root history branch.
pub const LIVING_ROOT_BRANCH_DOMAIN_V2: &[u8] = b"NYON-LIVING-ROOT-BRANCH-V2\0";
/// Domain of a forked history branch.
pub const LIVING_FORK_BRANCH_DOMAIN_V2: &[u8] = b"NYON-LIVING-FORK-BRANCH-V2\0";
/// Domain of one ordered event inside a tick receipt.
pub const LIVING_EVENT_DOMAIN_V2: &[u8] = b"NYON-LIVING-EVENT-V2\0";
/// Domain of the settlement claim rank.
pub const LIVING_CLAIM_DOMAIN_V2: &[u8] = b"NYON-LIVING-CLAIM-V2\0";

/// Optional tag for an absent value.
pub const LIVING_OPTIONAL_ABSENT_V2: u8 = 0x00;
/// Optional tag preceding a present fixed-width value.
pub const LIVING_OPTIONAL_PRESENT_V2: u8 = 0x01;

/// Completed authoritative tick boundaries. A base-10 integer on the wire, not
/// hexadecimal: V2 canonical numbers are base-10 integers.
#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct LivingTickV2(pub u64);

/// Identity of one immutable creator revision.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingRevisionIdV2(pub [u8; 32]);

/// Identity of one authority entity, the first 16 bytes of its entity digest.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingEntityIdV2(pub [u8; 16]);

/// Identity of one history branch, the first 16 bytes of its branch digest.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingBranchIdV2(pub [u8; 16]);

/// Canonical hash of the validated catalog pack a document is bound to.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingCatalogHashV2(pub [u8; 32]);

/// Canonical digest of one authoritative state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingStateDigestV2(pub [u8; 32]);

/// Canonical digest of one tick receipt.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingReceiptDigestV2(pub [u8; 32]);

/// Identity of one receipt event, the first 16 bytes of its event digest.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LivingEventIdV2(pub [u8; 16]);

macro_rules! living_fixed_hex_v2 {
    ($type:ident, $length:expr) => {
        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let mut output = String::with_capacity($length * 2);
                for byte in self.0 {
                    output.push(char::from(HEX[usize::from(byte >> 4)]));
                    output.push(char::from(HEX[usize::from(byte & 0x0f)]));
                }
                serializer.serialize_str(&output)
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let input = String::deserialize(deserializer)?;
                if input.len() != $length * 2 {
                    return Err(serde::de::Error::custom(
                        "Living V2 identity has the wrong fixed width",
                    ));
                }
                let mut bytes = [0_u8; $length];
                for (index, pair) in input.as_bytes().chunks_exact(2).enumerate() {
                    let high = living_hex_nibble_v2(pair[0]);
                    let low = living_hex_nibble_v2(pair[1]);
                    match (high, low) {
                        (Some(high), Some(low)) => bytes[index] = (high << 4) | low,
                        _ => {
                            return Err(serde::de::Error::custom(
                                "Living V2 identity is not lowercase hexadecimal",
                            ));
                        }
                    }
                }
                Ok(Self(bytes))
            }
        }
    };
}

living_fixed_hex_v2!(LivingRevisionIdV2, 32);
living_fixed_hex_v2!(LivingCatalogHashV2, 32);
living_fixed_hex_v2!(LivingStateDigestV2, 32);
living_fixed_hex_v2!(LivingReceiptDigestV2, 32);
living_fixed_hex_v2!(LivingEntityIdV2, 16);
living_fixed_hex_v2!(LivingBranchIdV2, 16);
living_fixed_hex_v2!(LivingEventIdV2, 16);

fn living_hex_nibble_v2(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

/// One kind of Living V2 authority entity, as published by the section 10
/// registry.
///
/// The registry fixes the authority entity set: exactly those records that
/// carry an identity of their own. A colony, relation, agreement, war,
/// observation, settlement claim and occupation are deliberately absent,
/// because each is keyed by the identities it relates rather than by one of
/// its own.
///
/// `entity_kind_u16` is a hash input to both entity formulas, so these values
/// are normative, never reordered and never reissued: renumbering one produces
/// different entity identities for the same history. That is why the wire and
/// hash value comes from [`LivingEntityKindV2::ordinal`], an explicit arm per
/// variant, and never from this enum's Rust layout.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LivingEntityKindV2 {
    /// A star system.
    System,
    /// A star belonging to one system.
    Star,
    /// A world, which holds all inventory.
    World,
    /// A lane between two systems.
    Lane,
    /// A civilization.
    Civilization,
    /// A finite resource deposit.
    Deposit,
    /// A built industrial facility, including a defense battery.
    Facility,
    /// A paid, in-progress facility construction.
    ConstructionJob,
    /// A paid, in-progress hull build.
    HullJob,
    /// A fleet of co-owned hulls.
    Fleet,
    /// One hull inside a fleet.
    Hull,
    /// A standing freight route.
    Route,
    /// One shipment in flight, waiting or returning.
    Shipment,
    /// A scheduled lane hazard.
    Hazard,
}

/// The published entity-kind registry, in its normative order.
///
/// The task that fixed the authority entity set assigned these values, as
/// section 10 delegates. They follow the state schema's own field order and are
/// numbered from one, like the published phase and intent registries.
pub const LIVING_ENTITY_KIND_REGISTRY_V2: [(LivingEntityKindV2, u16); 14] = [
    (LivingEntityKindV2::System, 1),
    (LivingEntityKindV2::Star, 2),
    (LivingEntityKindV2::World, 3),
    (LivingEntityKindV2::Lane, 4),
    (LivingEntityKindV2::Civilization, 5),
    (LivingEntityKindV2::Deposit, 6),
    (LivingEntityKindV2::Facility, 7),
    (LivingEntityKindV2::ConstructionJob, 8),
    (LivingEntityKindV2::HullJob, 9),
    (LivingEntityKindV2::Fleet, 10),
    (LivingEntityKindV2::Hull, 11),
    (LivingEntityKindV2::Route, 12),
    (LivingEntityKindV2::Shipment, 13),
    (LivingEntityKindV2::Hazard, 14),
];

impl LivingEntityKindV2 {
    /// The published `entity_kind_u16` of this kind.
    pub const fn ordinal(self) -> u16 {
        match self {
            Self::System => 1,
            Self::Star => 2,
            Self::World => 3,
            Self::Lane => 4,
            Self::Civilization => 5,
            Self::Deposit => 6,
            Self::Facility => 7,
            Self::ConstructionJob => 8,
            Self::HullJob => 9,
            Self::Fleet => 10,
            Self::Hull => 11,
            Self::Route => 12,
            Self::Shipment => 13,
            Self::Hazard => 14,
        }
    }

    /// The kind an ordinal names, or `None` for a value the registry does not
    /// publish. Zero is not a kind, and neither is any value above the last
    /// published row.
    pub const fn from_ordinal(ordinal: u16) -> Option<Self> {
        match ordinal {
            1 => Some(Self::System),
            2 => Some(Self::Star),
            3 => Some(Self::World),
            4 => Some(Self::Lane),
            5 => Some(Self::Civilization),
            6 => Some(Self::Deposit),
            7 => Some(Self::Facility),
            8 => Some(Self::ConstructionJob),
            9 => Some(Self::HullJob),
            10 => Some(Self::Fleet),
            11 => Some(Self::Hull),
            12 => Some(Self::Route),
            13 => Some(Self::Shipment),
            14 => Some(Self::Hazard),
            _ => None,
        }
    }
}

/// One of the ten ordered steps of a Living V2 tick boundary, as published by
/// the section 10 registry.
///
/// `phase_u16` is a hash input to the autonomous entity formula, so like the
/// entity kinds these values are normative, never reordered, and taken from
/// [`LivingPhaseV2::ordinal`] rather than from Rust layout. The step list is
/// numbered from one and so is this registry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LivingPhaseV2 {
    /// 1. Apply queued validated creator interventions.
    ApplyCreatorInterventions,
    /// 2. Expire agreements, truces and wars; update hazard boundaries.
    ExpireLifecycles,
    /// 3. Resolve travel arrivals and freight delivery, return and capture.
    ResolveArrivals,
    /// 4. Resolve retreat departures, combat rounds, occupation and settlement
    ///    claims.
    ResolveConflict,
    /// 5. Complete construction; run hubs, solar, extraction, processing and
    ///    repairs.
    RunProduction,
    /// 6. Refresh civilization observations.
    RefreshObservations,
    /// 7. Update diplomacy and resolve proposals.
    UpdateDiplomacy,
    /// 8. Generate economic and fleet intents; validate and reserve centrally.
    GenerateIntents,
    /// 9. Dispatch scheduled fleet orders and eligible freight routes.
    DispatchOrders,
    /// 10. Produce ordered event receipts, update counters, hash and commit.
    CommitBoundary,
}

/// The published phase registry, in its normative order.
pub const LIVING_PHASE_REGISTRY_V2: [(LivingPhaseV2, u16); 10] = [
    (LivingPhaseV2::ApplyCreatorInterventions, 1),
    (LivingPhaseV2::ExpireLifecycles, 2),
    (LivingPhaseV2::ResolveArrivals, 3),
    (LivingPhaseV2::ResolveConflict, 4),
    (LivingPhaseV2::RunProduction, 5),
    (LivingPhaseV2::RefreshObservations, 6),
    (LivingPhaseV2::UpdateDiplomacy, 7),
    (LivingPhaseV2::GenerateIntents, 8),
    (LivingPhaseV2::DispatchOrders, 9),
    (LivingPhaseV2::CommitBoundary, 10),
];

impl LivingPhaseV2 {
    /// The published `phase_u16` of this step.
    pub const fn ordinal(self) -> u16 {
        match self {
            Self::ApplyCreatorInterventions => 1,
            Self::ExpireLifecycles => 2,
            Self::ResolveArrivals => 3,
            Self::ResolveConflict => 4,
            Self::RunProduction => 5,
            Self::RefreshObservations => 6,
            Self::UpdateDiplomacy => 7,
            Self::GenerateIntents => 8,
            Self::DispatchOrders => 9,
            Self::CommitBoundary => 10,
        }
    }

    /// The step an ordinal names, or `None` for a value the registry does not
    /// publish. Zero is not a phase, because the step list is one-based.
    pub const fn from_ordinal(ordinal: u16) -> Option<Self> {
        match ordinal {
            1 => Some(Self::ApplyCreatorInterventions),
            2 => Some(Self::ExpireLifecycles),
            3 => Some(Self::ResolveArrivals),
            4 => Some(Self::ResolveConflict),
            5 => Some(Self::RunProduction),
            6 => Some(Self::RefreshObservations),
            7 => Some(Self::UpdateDiplomacy),
            8 => Some(Self::GenerateIntents),
            9 => Some(Self::DispatchOrders),
            10 => Some(Self::CommitBoundary),
            _ => None,
        }
    }
}

macro_rules! living_registry_serde_v2 {
    ($type:ident, $noun:literal) => {
        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_u16(self.ordinal())
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let ordinal = u16::deserialize(deserializer)?;
                Self::from_ordinal(ordinal).ok_or_else(|| {
                    serde::de::Error::custom(concat!("the registry publishes no ", $noun))
                })
            }
        }
    };
}

living_registry_serde_v2!(LivingEntityKindV2, "entity kind with that ordinal");
living_registry_serde_v2!(LivingPhaseV2, "phase with that ordinal");

/// Framed inputs of the autonomous entity identity.
///
/// The spec groups these into one never-reordered tuple, and the same tuple may
/// be consumed only once per document; that single-use rule belongs to the
/// authority that allocates identities, not to this pure framing function.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LivingAutonomousEntityInputsV2 {
    /// Canonical hash of the bound catalog pack.
    pub catalog_hash: LivingCatalogHashV2,
    /// The document's 32-byte genesis seed.
    pub genesis_seed: [u8; 32],
    /// Branch the entity is created on.
    pub branch: LivingBranchIdV2,
    /// Boundary the entity is created at.
    pub tick: LivingTickV2,
    /// Schema-table phase ordinal, never a Rust enum layout value.
    pub phase: u16,
    /// Acting entity; route dispatch uses the route as its own actor.
    pub actor: LivingEntityIdV2,
    /// Stable per-boundary intent or dispatch ordinal for that actor.
    pub intent_ordinal: u16,
    /// Schema-table entity kind, never a Rust enum layout value.
    pub entity_kind: u16,
    /// Zero-based local identifier within the intent.
    pub local_id: u16,
}

fn living_digest_v2(domain: &[u8], parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(domain);
    for part in parts {
        hash.update(part);
    }
    hash.finalize().into()
}

fn living_first_sixteen_v2(digest: [u8; 32]) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes
}

/// `catalog_hash = SHA256("NYON-LIVING-PACK-V2\0" || canonical_catalog_bytes)`.
pub fn catalog_hash_v2(canonical_catalog_bytes: &[u8]) -> LivingCatalogHashV2 {
    LivingCatalogHashV2(living_digest_v2(
        LIVING_PACK_DOMAIN_V2,
        &[canonical_catalog_bytes],
    ))
}

/// `state_digest = SHA256("NYON-LIVING-STATE-V2\0" || canonical_state_bytes)`.
pub fn state_digest_v2(canonical_state_bytes: &[u8]) -> LivingStateDigestV2 {
    LivingStateDigestV2(living_digest_v2(
        LIVING_STATE_DOMAIN_V2,
        &[canonical_state_bytes],
    ))
}

/// `archive_integrity = SHA256(domain || canonical_payload_bytes)`, over the
/// ordered archive payload without its own integrity field.
pub fn archive_integrity_v2(canonical_payload_bytes: &[u8]) -> [u8; 32] {
    living_digest_v2(
        LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2,
        &[canonical_payload_bytes],
    )
}

/// `receipt_digest = SHA256(domain || canonical_receipt_bytes)`.
pub fn receipt_digest_v2(canonical_receipt_bytes: &[u8]) -> LivingReceiptDigestV2 {
    LivingReceiptDigestV2(living_digest_v2(
        LIVING_RECEIPT_DOMAIN_V2,
        &[canonical_receipt_bytes],
    ))
}

/// `revision_id = SHA256(domain || rules_u32 || catalog_hash_32 ||
/// genesis_seed_32 || parent_tag_u8 || parent_32_if_present || tick_u64 ||
/// ordinal_u64 || command_length_u64 || canonical_command_bytes)`.
pub fn revision_id_v2(
    catalog_hash: LivingCatalogHashV2,
    genesis_seed: [u8; 32],
    parent: Option<LivingRevisionIdV2>,
    tick: LivingTickV2,
    ordinal: u64,
    canonical_command_bytes: &[u8],
) -> LivingRevisionIdV2 {
    let mut hash = Sha256::new();
    hash.update(LIVING_REVISION_DOMAIN_V2);
    hash.update(LIVING_RULES_VERSION.to_le_bytes());
    hash.update(catalog_hash.0);
    hash.update(genesis_seed);
    match parent {
        Some(parent) => {
            hash.update([LIVING_OPTIONAL_PRESENT_V2]);
            hash.update(parent.0);
        }
        None => hash.update([LIVING_OPTIONAL_ABSENT_V2]),
    }
    hash.update(tick.0.to_le_bytes());
    hash.update(ordinal.to_le_bytes());
    hash.update((canonical_command_bytes.len() as u64).to_le_bytes());
    hash.update(canonical_command_bytes);
    LivingRevisionIdV2(hash.finalize().into())
}

/// `creator_entity_digest = SHA256(domain || revision_id_32 ||
/// entity_kind_u16 || batch_local_id_u16)`.
pub fn creator_entity_digest_v2(
    revision: LivingRevisionIdV2,
    entity_kind: u16,
    batch_local_id: u16,
) -> [u8; 32] {
    living_digest_v2(
        LIVING_CREATOR_ENTITY_DOMAIN_V2,
        &[
            &revision.0,
            &entity_kind.to_le_bytes(),
            &batch_local_id.to_le_bytes(),
        ],
    )
}

/// The first 16 bytes of [`creator_entity_digest_v2`].
pub fn creator_entity_id_v2(
    revision: LivingRevisionIdV2,
    entity_kind: u16,
    batch_local_id: u16,
) -> LivingEntityIdV2 {
    LivingEntityIdV2(living_first_sixteen_v2(creator_entity_digest_v2(
        revision,
        entity_kind,
        batch_local_id,
    )))
}

/// `autonomous_entity_digest = SHA256(domain || rules_u32 || catalog_hash_32 ||
/// genesis_seed_32 || branch_id_16 || tick_u64 || phase_u16 || actor_id_16 ||
/// intent_ordinal_u16 || entity_kind_u16 || local_id_u16)`.
pub fn autonomous_entity_digest_v2(inputs: &LivingAutonomousEntityInputsV2) -> [u8; 32] {
    living_digest_v2(
        LIVING_AUTO_ENTITY_DOMAIN_V2,
        &[
            &LIVING_RULES_VERSION.to_le_bytes(),
            &inputs.catalog_hash.0,
            &inputs.genesis_seed,
            &inputs.branch.0,
            &inputs.tick.0.to_le_bytes(),
            &inputs.phase.to_le_bytes(),
            &inputs.actor.0,
            &inputs.intent_ordinal.to_le_bytes(),
            &inputs.entity_kind.to_le_bytes(),
            &inputs.local_id.to_le_bytes(),
        ],
    )
}

/// The first 16 bytes of [`autonomous_entity_digest_v2`].
pub fn autonomous_entity_id_v2(inputs: &LivingAutonomousEntityInputsV2) -> LivingEntityIdV2 {
    LivingEntityIdV2(living_first_sixteen_v2(autonomous_entity_digest_v2(inputs)))
}

/// `root_branch_digest = SHA256(domain || rules_u32 || catalog_hash_32 ||
/// genesis_seed_32 || genesis_manifest_digest_32)`.
pub fn root_branch_digest_v2(
    catalog_hash: LivingCatalogHashV2,
    genesis_seed: [u8; 32],
    genesis_manifest_digest: LivingStateDigestV2,
) -> [u8; 32] {
    living_digest_v2(
        LIVING_ROOT_BRANCH_DOMAIN_V2,
        &[
            &LIVING_RULES_VERSION.to_le_bytes(),
            &catalog_hash.0,
            &genesis_seed,
            &genesis_manifest_digest.0,
        ],
    )
}

/// The first 16 bytes of [`root_branch_digest_v2`].
pub fn root_branch_id_v2(
    catalog_hash: LivingCatalogHashV2,
    genesis_seed: [u8; 32],
    genesis_manifest_digest: LivingStateDigestV2,
) -> LivingBranchIdV2 {
    LivingBranchIdV2(living_first_sixteen_v2(root_branch_digest_v2(
        catalog_hash,
        genesis_seed,
        genesis_manifest_digest,
    )))
}

/// `fork_branch_digest = SHA256(domain || parent_branch_16 ||
/// fork_revision_32 || fork_tick_u64 || branch_ordinal_u64)`.
pub fn fork_branch_digest_v2(
    parent_branch: LivingBranchIdV2,
    fork_revision: LivingRevisionIdV2,
    fork_tick: LivingTickV2,
    branch_ordinal: u64,
) -> [u8; 32] {
    living_digest_v2(
        LIVING_FORK_BRANCH_DOMAIN_V2,
        &[
            &parent_branch.0,
            &fork_revision.0,
            &fork_tick.0.to_le_bytes(),
            &branch_ordinal.to_le_bytes(),
        ],
    )
}

/// The first 16 bytes of [`fork_branch_digest_v2`].
pub fn fork_branch_id_v2(
    parent_branch: LivingBranchIdV2,
    fork_revision: LivingRevisionIdV2,
    fork_tick: LivingTickV2,
    branch_ordinal: u64,
) -> LivingBranchIdV2 {
    LivingBranchIdV2(living_first_sixteen_v2(fork_branch_digest_v2(
        parent_branch,
        fork_revision,
        fork_tick,
        branch_ordinal,
    )))
}

/// `event_digest = SHA256(domain || receipt_digest_32 || event_ordinal_u16)`.
pub fn event_digest_v2(receipt: LivingReceiptDigestV2, event_ordinal: u16) -> [u8; 32] {
    living_digest_v2(
        LIVING_EVENT_DOMAIN_V2,
        &[&receipt.0, &event_ordinal.to_le_bytes()],
    )
}

/// The first 16 bytes of [`event_digest_v2`].
pub fn event_id_v2(receipt: LivingReceiptDigestV2, event_ordinal: u16) -> LivingEventIdV2 {
    LivingEventIdV2(living_first_sixteen_v2(event_digest_v2(
        receipt,
        event_ordinal,
    )))
}

/// `claim_rank = SHA256(domain || rules_u32 || genesis_seed_32 || world_id_16 ||
/// claim_close_tick_u64 || civilization_id_16)`.
///
/// Arbitration compares the whole 32-byte rank, never a truncation of it.
pub fn claim_rank_v2(
    genesis_seed: [u8; 32],
    world: LivingEntityIdV2,
    claim_close_tick: LivingTickV2,
    civilization: LivingEntityIdV2,
) -> [u8; 32] {
    living_digest_v2(
        LIVING_CLAIM_DOMAIN_V2,
        &[
            &LIVING_RULES_VERSION.to_le_bytes(),
            &genesis_seed,
            &world.0,
            &claim_close_tick.0.to_le_bytes(),
            &civilization.0,
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_domain_literal_terminates_with_a_nul_byte() {
        for domain in [
            LIVING_PACK_DOMAIN_V2,
            LIVING_STATE_DOMAIN_V2,
            LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2,
            LIVING_RECEIPT_DOMAIN_V2,
            LIVING_REVISION_DOMAIN_V2,
            LIVING_CREATOR_ENTITY_DOMAIN_V2,
            LIVING_AUTO_ENTITY_DOMAIN_V2,
            LIVING_ROOT_BRANCH_DOMAIN_V2,
            LIVING_FORK_BRANCH_DOMAIN_V2,
            LIVING_EVENT_DOMAIN_V2,
            LIVING_CLAIM_DOMAIN_V2,
        ] {
            assert_eq!(domain.last(), Some(&0));
            assert!(domain[..domain.len() - 1].is_ascii());
            assert!(domain.starts_with(b"NYON-LIVING-"));
            assert!(domain[..domain.len() - 1].ends_with(b"-V2"));
        }
    }

    #[test]
    fn truncated_identities_are_the_leading_sixteen_digest_bytes() {
        let revision = LivingRevisionIdV2([0x2a; 32]);
        let digest = creator_entity_digest_v2(revision, 1, 0);
        assert_eq!(creator_entity_id_v2(revision, 1, 0).0, digest[..16]);
    }
}
