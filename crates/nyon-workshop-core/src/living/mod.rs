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

pub mod catalog;
pub mod command;
pub mod genesis;
pub mod ids;
pub mod model;
pub mod receipt;
pub mod simulation;
pub mod wire;

pub use catalog::{
    LIVING_COLONY_INDUSTRY_SLOTS_V2, LIVING_CORE_PACK_BYTES_V2, LIVING_PACK_FORMAT_VERSION_V2,
    LIVING_PACK_KIND_V2, LIVING_RESOURCE_STORAGE_LIMIT_V2, LivingCatalogErrorV2,
    LivingHazardDefinitionV2, LivingHazardEffectV2, LivingHullDefinitionV2,
    LivingIndustryDefinitionV2, LivingIndustryEffectV2, LivingIntentKindV2, LivingNameV2,
    LivingPolicyBonusV2, LivingPolicyDefinitionV2, LivingResourceAmountV2,
    LivingResourceDefinitionV2, LivingSlugV2, LivingStarArchetypeV2, LivingWorldArchetypeV2,
    ValidatedLivingCatalogPackV2, decode_living_catalog_pack_v2, living_core_pack_v2,
};
pub use command::{
    LIVING_MAX_CREATOR_BATCH_OPERATIONS_V2, LivingAcceptedCommandV2, LivingCascadeDispositionV2,
    LivingCommandEnvelopeV2, LivingCommandErrorV2, LivingCommandModeV2, LivingCommandTargetV2,
    LivingCommandV2, LivingCreatorOperationV2, LivingRevisionV2,
};
pub use genesis::{
    LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LIVING_HUB_ENERGY_PERIOD_TICKS_V2,
    LIVING_HUB_FALLBACK_PERIOD_TICKS_V2, LIVING_HUB_ORE_PERIOD_TICKS_V2, LivingGenesisErrorV2,
    LivingGenesisGeneratorV2, LivingGenesisManifestV2, ValidatedLivingGenesisV2,
    decode_living_genesis_manifest_v2, validate_living_genesis_manifest_v2,
};
pub use ids::{
    LIVING_ARCHIVE_INTEGRITY_DOMAIN_V2, LIVING_AUTO_ENTITY_DOMAIN_V2, LIVING_CLAIM_DOMAIN_V2,
    LIVING_CREATOR_ENTITY_DOMAIN_V2, LIVING_ENTITY_KIND_REGISTRY_V2, LIVING_EVENT_DOMAIN_V2,
    LIVING_FORK_BRANCH_DOMAIN_V2, LIVING_OPTIONAL_ABSENT_V2, LIVING_OPTIONAL_PRESENT_V2,
    LIVING_PACK_DOMAIN_V2, LIVING_PHASE_REGISTRY_V2, LIVING_RECEIPT_DOMAIN_V2,
    LIVING_REVISION_DOMAIN_V2, LIVING_ROOT_BRANCH_DOMAIN_V2, LIVING_STATE_DOMAIN_V2,
    LivingAutonomousEntityInputsV2, LivingBranchIdV2, LivingCatalogHashV2, LivingEntityIdV2,
    LivingEntityKindV2, LivingEventIdV2, LivingPhaseV2, LivingReceiptDigestV2, LivingRevisionIdV2,
    LivingStateDigestV2, LivingTickV2, archive_integrity_v2, autonomous_entity_digest_v2,
    autonomous_entity_id_v2, catalog_hash_v2, claim_rank_v2, creator_entity_digest_v2,
    creator_entity_id_v2, event_digest_v2, event_id_v2, fork_branch_digest_v2, fork_branch_id_v2,
    receipt_digest_v2, revision_id_v2, root_branch_digest_v2, root_branch_id_v2, state_digest_v2,
};
pub use model::{
    LIVING_DELIVERY_ACCUMULATOR_BOUND_V2, LIVING_MAX_CIVILIZATIONS_V2, LIVING_MAX_DEPOSITS_V2,
    LIVING_MAX_FLEETS_V2, LIVING_MAX_HAZARDS_V2, LIVING_MAX_HULLS_PER_FLEET_V2,
    LIVING_MAX_HULLS_V2, LIVING_MAX_INDUSTRIES_V2, LIVING_MAX_LANES_V2, LIVING_MAX_ROUTES_V2,
    LIVING_MAX_SHIPMENTS_V2, LIVING_MAX_STARS_V2, LIVING_MAX_SYSTEMS_V2, LIVING_MAX_WORLDS_V2,
    LIVING_OCCUPATION_TRANSFER_TICKS_V2, LIVING_RELATION_BOUND_V2, LivingAgreementKindV2,
    LivingAgreementV2, LivingBlockedReasonV2, LivingCivilizationStatusV2, LivingCivilizationV2,
    LivingColonyV2, LivingConstructionJobV2, LivingCountersV2, LivingDepositV2,
    LivingFacilityStatusV2, LivingFacilityV2, LivingFleetLocationV2, LivingFleetOrderKindV2,
    LivingFleetOrderV2, LivingFleetV2, LivingFreightRouteV2, LivingGalaxyStateV2, LivingHazardV2,
    LivingHubV2, LivingHullJobV2, LivingHullV2, LivingInventoryV2, LivingLaneV2,
    LivingObservationV2, LivingObservedFacilityV2, LivingObservedHullV2, LivingOccupationV2,
    LivingPolicyV2, LivingRelationReasonsV2, LivingRelationV2, LivingResourceV2, LivingRouteKindV2,
    LivingSettlementClaimV2, LivingShipmentDispositionV2, LivingShipmentV2, LivingStarV2,
    LivingSystemV2, LivingValidationErrorV2, LivingWarV2, LivingWorldV2,
};
pub use receipt::{
    LIVING_MAX_BOUNDARY_EVENTS_V2, LivingEventKindV2, LivingEventPayloadV2,
    LivingEventProvenanceV2, LivingEventV2, LivingPendingEventV2, LivingPendingEventsV2,
    LivingReceiptErrorV2, LivingReceiptPayloadV2, LivingTickReceiptV2, seal_living_tick_receipt_v2,
};
pub use simulation::{
    LIVING_MAX_PENDING_QUEUE_DEPTH_V2, LivingAuthorityErrorV2, LivingCommandCursorV2,
    LivingCommandRejectionV2, LivingDeterministicFaultV2, LivingGalaxyAuthorityV2,
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
