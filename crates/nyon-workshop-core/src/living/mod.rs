//! Living Galaxy V2: a strictly versioned deterministic authority island.
//!
//! This module is a separate rule set from Classic RulesV1 and WorkshopV1. It
//! keeps its own identities, hash domains and canonical wire, and shares none
//! of theirs. There is deliberately no conversion between a WorkshopV1 wrapper
//! and a Living V2 wrapper in either direction, even where serialized widths
//! match, so the compiler cannot let a V1 value satisfy a V2 parameter.
//!
//! The binding contract is
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`; the
//! reviewed byte vectors are `tests/fixtures/living-v2/vectors.json`.

pub mod ids;
pub mod wire;

pub use ids::{
    LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2, LIVING_AUTO_ENTITY_DOMAIN_V2, LIVING_CLAIM_DOMAIN_V2,
    LIVING_CREATOR_ENTITY_DOMAIN_V2, LIVING_EVENT_DOMAIN_V2, LIVING_FORK_BRANCH_DOMAIN_V2,
    LIVING_OPTIONAL_ABSENT_V2, LIVING_OPTIONAL_PRESENT_V2, LIVING_PACK_DOMAIN_V2,
    LIVING_RECEIPT_DOMAIN_V2, LIVING_REVISION_DOMAIN_V2, LIVING_ROOT_BRANCH_DOMAIN_V2,
    LIVING_STATE_DOMAIN_V2, LivingAutonomousEntityInputsV2, LivingBranchIdV2, LivingCatalogHashV2,
    LivingEntityIdV2, LivingEventIdV2, LivingReceiptDigestV2, LivingRevisionIdV2,
    LivingStateDigestV2, LivingTickV2, archive_integrity_v2, autonomous_entity_digest_v2,
    autonomous_entity_id_v2, catalog_hash_v2, claim_rank_v2, creator_entity_digest_v2,
    creator_entity_id_v2, event_digest_v2, event_id_v2, fork_branch_digest_v2, fork_branch_id_v2,
    receipt_digest_v2, revision_id_v2, root_branch_digest_v2, root_branch_id_v2, state_digest_v2,
};
pub use wire::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LIVING_MAX_CANONICAL_DEPTH_V2, LIVING_MAX_PACK_BYTES_V2,
    LivingKeyedV2, LivingSortedVecV2, LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2,
};

/// The Living Galaxy rule set version. Distinct from the WorkshopV1 version and
/// never interchangeable with it.
pub const LIVING_RULES_VERSION: u32 = 2;

/// Authoritative ticks per second.
pub const LIVING_TICK_HZ: u32 = 10;
