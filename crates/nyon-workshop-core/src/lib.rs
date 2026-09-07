#![forbid(unsafe_code)]

pub mod archive;
pub mod command;
pub mod history;
pub mod ids;
pub mod living;
pub mod model;
pub mod pack;
pub mod simulation;

pub use archive::{
    ArchiveDecodeJob, ArchiveDecodeStatus, ArchiveError, CanonicalArchive, archive_catalog_hash,
    decode_archive, decode_workshop_archive, encode_archive, encode_workshop_archive,
};
pub use command::{
    CreatorBatchV1, CreatorOpV1, CreatorReceiptV1, CreatorRejectionV1, WorkshopAuthority,
};
pub use history::{
    ActiveView, ActiveViewV1, BranchRef, BranchRefV1, CheckpointPolicy, HistoryError,
    WorkshopHistory, WorkshopHistoryV1,
};
pub use ids::{
    BatchLocalId, BranchId, CatalogHash, CatalogId, EntityId, GalaxyCoord, GalaxyPointV1,
    ObjectName, ObjectRefV1, RevisionId, StateDigest, WorkshopTick,
};
pub use model::{RevisionRecordV1, WorkshopStateV1};
pub use pack::{
    CatalogDefinitionKind, PackValidationError, ValidatedCatalogPackV1, decode_catalog_pack,
    encode_catalog_pack,
};
pub use simulation::{DeterministicFault, TickEventV1, TickReceiptV1};

pub const WORKSHOP_RULES_VERSION: u32 = 1;
pub const WORKSHOP_TICK_HZ: u32 = 10;
