//! Platform-neutral product shell for Classic Sector and Galaxy Workshop.
//!
//! This module deliberately sits outside [`AppCore`]. The RulesV1 client and
//! the Workshop authority are sibling sessions; neither is adapted into the
//! other's mode or command model.

use std::{collections::VecDeque, time::Duration};

pub mod library;

use crate::{
    app::{
        AppCore,
        transfer::{
            HandoffOutcome, SuggestedName, TransferAdapter, TransferError, TransferFailureCode,
            TransferJobId, TransferJobState, TransferKind, TransferOutcome, TransferRequest,
        },
    },
    preferences::store::PreferencesStore,
    scenario::store::ScenarioStore,
    workshop::{
        CatalogHash, ValidatedCatalogPackV1, WorkshopHistory, decode_catalog_pack,
        encode_catalog_pack,
        session::{
            WorkshopAction, WorkshopSession, WorkshopSessionError, WorkshopSessionSnapshot,
            WorkshopUpdate,
        },
        store::{
            LoadedSlot, SaveGeneration, SlotId, SlotList, SlotName, StoreJobId, StoreJobState,
            WorkshopStore, WorkshopStoreError, WorkshopStoreRequest, WorkshopStoreResult,
        },
    },
};

use library::{
    LibraryBeginError, LibraryCandidate, LibraryEvent, LibraryOpen, SlotIntent,
    WorkshopLibraryClient,
};

const MAX_CLIENT_DIAGNOSTICS: usize = 100;
const ARCHIVE_REPLAY_UNITS_PER_UPDATE: u64 = 1_024;
const CORE_PACK_V1: &[u8] = include_bytes!("../../assets/workshop/core-pack-v1.json");

/// The only top-level screens backed by V1 capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientScreen {
    MainMenu,
    ClassicSector,
    GalaxyWorkshop,
    Settings,
    /// Workshop slot management and portable transfer. A sibling of
    /// [`ClientScreen::Settings`] rather than a Workshop drawer section,
    /// because the addendum requires main-menu entry and there is no Workshop
    /// frame to hang a drawer on there.
    ///
    /// Opening it is a **lateral** route: it never prepares a durable
    /// transition, so the resident session keeps its dirty state, its queued
    /// actions and its recovery obligations untouched. Gating happens per
    /// action inside the Library, not at the door.
    Library,
    Loading,
    RecoverableError,
}

/// The Classic and Workshop sessions remain siblings with independent rules.
///
/// The public contract intentionally stores the Workshop session directly.
/// Boxing it here would change the locked `Workshop(WorkshopSession)` shape.
#[allow(clippy::large_enum_variant)]
pub enum ActiveSession {
    None,
    Classic,
    Workshop(WorkshopSession),
}

/// Main-menu routes with working V1 behavior. Credits is an overlay on the
/// main menu because it does not warrant another product screen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MainMenuRoute {
    NewWorkshop,
    Continue,
    /// A lateral route: it replaces nothing, so the unsaved-resident gate on
    /// New Workshop and Classic Sector does not apply to it.
    Library,
    ClassicSector,
    Settings,
    Credits,
    #[cfg(not(target_arch = "wasm32"))]
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MainMenuCapability {
    pub route: MainMenuRoute,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientRuntimeEffect {
    None,
    CreditsOpened,
    #[cfg(not(target_arch = "wasm32"))]
    QuitRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientDiagnosticCode {
    Store,
    StoreProtocol,
    Catalog,
    Archive,
    ContinueUnavailable,
    WorkshopInactive,
    RouteUnavailable,
    /// A Library action was refused because the resident Workshop occupies the
    /// slot it named. Distinct from [`Self::RouteUnavailable`] because the
    /// user-visible reason is a different one: the row is not wrong, the
    /// timing is.
    ResidentSlot,
    /// A Library open found the row changed under it: a newer head, an
    /// archive, or a Continue compare-and-swap the store refused. Addendum §4
    /// calls this a refreshable conflict, not a failure, so the remedy offered
    /// is to re-list rather than to repeat the same stale request.
    StaleSave,
    /// A portable transfer failed or the adapter broke the protocol. Its
    /// detail is a bounded [`TransferFailureCode`], never platform text.
    Transfer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientDiagnostic {
    pub code: ClientDiagnosticCode,
    pub message: String,
}

/// The public snapshot of the catalog-import state machine.
///
/// Catalog import is an explicit Library-facing state machine, not a Boolean
/// and not the global recovery screen: the Library has to offer Retry, Cancel,
/// and Start When Safe over a runtime that is still on its own screen. Only
/// bounded typed facts appear here. The validated pack, its canonical bytes,
/// and any requested start seed stay internal retry material, so no raw
/// imported content can reach a caller through this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogImportStatus {
    Idle,
    Storing {
        expected_hash: CatalogHash,
        starts_new_workshop: bool,
    },
    Stored {
        hash: CatalogHash,
    },
    /// The store rejected the pack and the import can be retried from retained
    /// material. Offers Retry and Cancel.
    StoreFailed {
        expected_hash: CatalogHash,
        code: ClientDiagnosticCode,
        starts_new_workshop: bool,
    },
    /// The pack is durably stored, but the resident Workshop was not
    /// replaceable when the requested new Workshop was due to start. Offers
    /// Start When Safe and Cancel; the stored pack is never withdrawn.
    StartBlocked {
        hash: CatalogHash,
    },
}

/// Which Library slot request a machine state names.
///
/// The first four are plain store requests. Open, Use for Continue and row
/// Export go through [`WorkshopLibraryClient`] instead, because §4 assigns
/// them the exact-catalog resolution the plain machine deliberately does not
/// perform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SlotRequestKind {
    List,
    Rename,
    Archive,
    Unarchive,
    /// An exact-catalog open of one row.
    Open,
    /// The same validation, selecting the row for Continue without opening it.
    UseForContinue,
    /// The same validation as a pure read, preparing portable bytes. Selects
    /// nothing and installs nothing.
    Export,
    /// Stage 2 of row Export: prepared bytes are with the transfer adapter
    /// (route-design task 11).
    HandOff,
}

/// Where prepared export bytes came from, so the label cannot claim more than
/// the bytes are. The origin's own facts travel inside the variant: a row
/// export names its slot and generation, an active export names neither,
/// because there may be no slot at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportSource {
    /// A row's stored head, exactly as listed.
    Head {
        slot: SlotId,
        generation: SaveGeneration,
    },
    /// A row's retained predecessor, because the head did not validate.
    /// Nothing was promoted and the Continue marker did not move.
    RecoveredPredecessor {
        slot: SlotId,
        generation: SaveGeneration,
    },
    /// Export this galaxy: the open Workshop's in-memory state, which may be
    /// dirty (addendum §10). `continue_ready` records whether, when the bytes
    /// were captured, that state was the saved generation selected for
    /// Continue; only then may the label drop §10's qualifier.
    ActiveWorkshop { continue_ready: bool },
    /// Export content pack: the open Workshop's exact catalog.
    ActivePack { hash: CatalogHash },
}

impl ExportSource {
    /// The origin's label. The predecessor's text and the unsaved active
    /// export's text are quoted from the addendum (§4, §10) verbatim.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Head { .. } => "Latest save",
            Self::RecoveredPredecessor { .. } => {
                "Recovered predecessor; stored head and Continue unchanged"
            }
            Self::ActiveWorkshop {
                continue_ready: false,
            } => "Portable export; Workshop not saved for Continue",
            Self::ActiveWorkshop {
                continue_ready: true,
            } => "Open Workshop, as saved for Continue",
            Self::ActivePack { .. } => "Content pack of the open Workshop",
        }
    }

    /// The label a status line must carry to the end, when the bytes are
    /// something other than what a plain "portable copy" suggests: a
    /// recovered predecessor, or open-Workshop state that is not the saved
    /// Continue generation.
    pub const fn qualifier(self) -> Option<&'static str> {
        match self {
            Self::RecoveredPredecessor { .. }
            | Self::ActiveWorkshop {
                continue_ready: false,
            } => Some(self.label()),
            Self::Head { .. }
            | Self::ActiveWorkshop {
                continue_ready: true,
            }
            | Self::ActivePack { .. } => None,
        }
    }

    /// The row the bytes were read from, for a row export.
    pub const fn slot(self) -> Option<SlotId> {
        match self {
            Self::Head { slot, .. } | Self::RecoveredPredecessor { slot, .. } => Some(slot),
            Self::ActiveWorkshop { .. } | Self::ActivePack { .. } => None,
        }
    }

    /// The stored generation the bytes were read from, for a row export.
    pub const fn generation(self) -> Option<SaveGeneration> {
        match self {
            Self::Head { generation, .. } | Self::RecoveredPredecessor { generation, .. } => {
                Some(generation)
            }
            Self::ActiveWorkshop { .. } | Self::ActivePack { .. } => None,
        }
    }

    /// What the bytes are.
    pub const fn kind(self) -> TransferKind {
        match self {
            Self::ActivePack { .. } => TransferKind::ContentPack,
            Self::Head { .. } | Self::RecoveredPredecessor { .. } | Self::ActiveWorkshop { .. } => {
                TransferKind::WorkshopArchive
            }
        }
    }
}

/// Validated export bytes waiting for the platform handoff (route-design
/// task 11). Bounded by the store's own limit for their kind: a row export
/// holds the bytes `LoadSlot` returned, an active export the bytes the
/// session or catalog encoder produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedExport {
    pub source: ExportSource,
    /// The sanitized name the handoff suggests (§7).
    pub suggested_name: SuggestedName,
    pub bytes: Box<[u8]>,
}

impl PreparedExport {
    pub const fn kind(&self) -> TransferKind {
        self.source.kind()
    }
}

/// The bounded typed facts the Library presents for its slot-request machine.
///
/// Failure is a state here, never the global recovery screen: §6 forbids
/// collapsing Library failures into `RecoverableError`, and a panel offering
/// Retry and Cancel cannot exist over a runtime that has already replaced the
/// screen. The retained request stays internal, so nothing a caller reads can
/// carry a slot name or archive bytes back out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibrarySlotsStatus {
    /// Nothing in flight and nothing waiting on the user. The cached list, if
    /// any, is readable through [`ClientRuntime::library_slots`]; `None` there
    /// with `Idle` here means there is no list to render and the caller must
    /// ask for one, which is distinct from having listed an empty store. It is
    /// reached both before the first list and after a cancelled mutation.
    Idle,
    /// A request is in flight. Offers Cancel.
    Working {
        kind: SlotRequestKind,
        slot: Option<SlotId>,
    },
    /// The store rejected the request and it can be retried from the retained
    /// request. Offers Retry and Cancel.
    Failed {
        kind: SlotRequestKind,
        slot: Option<SlotId>,
        code: ClientDiagnosticCode,
    },
    /// A validated open is waiting for the user. `recovered` says it is the
    /// slot's retained predecessor rather than its head. Offers Open and
    /// Cancel.
    Held { slot: SlotId, recovered: bool },
    /// A row export found an invalid head and a valid predecessor. §4's
    /// explicit export-recovery choice: offers Export previous and Cancel.
    /// Nothing has been promoted or selected.
    ExportRecoveryOffered { slot: SlotId },
    /// Row-export bytes are validated and waiting for the platform handoff.
    /// The bytes themselves are read through
    /// [`ClientRuntime::prepared_export`]. Offers the handoff (disabled
    /// while no transfer adapter is installed) and Discard.
    ///
    /// While the handoff runs the status is `Working` with
    /// [`SlotRequestKind::HandOff`]; a cancelled handoff returns here.
    ExportReady { source: ExportSource },
    /// The handoff failed or was refused, and the prepared bytes are still
    /// held. Offers the handoff again, on its own control, and Discard.
    HandOffFailed {
        source: ExportSource,
        code: TransferFailureCode,
    },
    /// The adapter reported the copy handed off. The bytes are released;
    /// `outcome` is the only thing the product may claim about where they
    /// went. Offers Done, which frees the lane.
    ExportHandedOff {
        source: ExportSource,
        outcome: HandoffOutcome,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClientRuntimeError {
    #[error("Continue bootstrap is already active")]
    BootstrapActive,
    #[error("a Workshop catalog import is already active")]
    CatalogImportActive,
    #[error("a Workshop Library slot request is already waiting")]
    LibraryRequestActive,
    #[error("the resident Workshop occupies that slot")]
    ResidentSlot,
    #[error("Continue is unavailable until its selected save validates")]
    ContinueUnavailable,
    #[error("Galaxy Workshop is not the active session")]
    WorkshopInactive,
    #[error("no portable transfer adapter is installed")]
    TransferUnavailable,
    #[error("portable transfer failed: {}", .0.message())]
    Transfer(TransferFailureCode),
    #[error("the requested route is not available from the current screen")]
    RouteUnavailable,
    #[error("the built-in Workshop catalog did not validate")]
    BuiltInCatalog,
    #[error("the Workshop catalog pack did not validate")]
    InvalidCatalog,
    #[error("no successfully persisted imported Workshop catalog is available")]
    ImportedCatalogUnavailable,
    #[error("Workshop storage rejected the request: {0}")]
    Store(WorkshopStoreError),
    #[error("Workshop action mailbox rejected the request: {0}")]
    WorkshopAction(WorkshopSessionError),
}

/// Retained retry material for one catalog import. Bounded by the store's
/// existing pack-size limit, because the canonical bytes are exactly what
/// `PutPack` already accepted.
#[derive(Clone, Debug)]
struct ImportMaterial {
    catalog: Box<ValidatedCatalogPackV1>,
    canonical_pack: Box<[u8]>,
    start_seed: Option<u64>,
}

#[derive(Clone, Debug)]
enum CatalogImport {
    Idle,
    Storing {
        job: StoreJobId,
        material: ImportMaterial,
    },
    Stored {
        hash: CatalogHash,
    },
    StoreFailed {
        code: ClientDiagnosticCode,
        material: ImportMaterial,
    },
    /// Carries its own catalog rather than reading back `imported_catalog`, so
    /// Start When Safe cannot install a different pack than the one that was
    /// stored for it.
    StartBlocked {
        hash: CatalogHash,
        seed: u64,
        catalog: Box<ValidatedCatalogPackV1>,
    },
}

/// The Library's one slot-request lane, as a poll-driven machine.
///
/// It retains the whole [`WorkshopStoreRequest`] as retry material for the same
/// reason [`ImportMaterial`] retains the validated pack: Retry must not ask the
/// user to retype a slot name or re-pick a row. All four requests are small and
/// already `Clone`, so there is no bound to argue about.
///
/// Exactly one request is outstanding at a time. Starting a second would
/// overwrite the state holding the first one's job, and nothing would ever poll
/// it — the wedged-lane defect [`WorkshopStore::abandon`] exists to close, and
/// three of the four requests sit in the Commit lane the resident Workshop
/// needs to save.
#[derive(Debug)]
enum LibrarySlots {
    Idle,
    Working {
        job: StoreJobId,
        request: WorkshopStoreRequest,
    },
    Failed {
        request: WorkshopStoreRequest,
        code: ClientDiagnosticCode,
    },
    /// Route-design task 12: an exact-catalog open of one row is in flight.
    /// Its jobs live in [`ClientRuntime::open_client`], not here; this state is
    /// what keeps the lane occupied while they run, so a Rename cannot contend
    /// for the Commit lane the open's `SelectContinue` needs.
    Opening {
        slot: SlotId,
        generation: SaveGeneration,
        purpose: OpenPurpose,
    },
    /// The open failed or conflicted. A conflict is retried by re-listing,
    /// never by repeating the request with the generation it already found
    /// stale.
    OpenFailed {
        slot: SlotId,
        generation: SaveGeneration,
        purpose: OpenPurpose,
        code: ClientDiagnosticCode,
    },
    /// A fully validated candidate the runtime did not install on arrival:
    /// a retained predecessor, which needs the user's explicit acceptance
    /// (§4's recovery offer), or a clean head that arrived while the resident
    /// Workshop could not be replaced or the Library was no longer on screen
    /// (§4 item 9 preserves it).
    Held {
        candidate: Box<LibraryCandidate>,
    },
    /// Route-design task 12c: a row export whose head did not validate, and
    /// whose predecessor did. Only the loaded predecessor is kept; the
    /// replayed history is not needed to export it.
    ExportRecoveryOffered {
        loaded: Box<LoadedSlot>,
    },
    /// Validated row-export bytes. The lane stays occupied until the user
    /// discards them or hands them off (task 11), so the one request strip is
    /// the one place they are offered.
    ExportReady {
        export: Box<PreparedExport>,
    },
    /// Route-design task 11: the prepared bytes are with the transfer
    /// adapter. They are kept, because a cancelled or failed handoff returns
    /// the user to them without a second export.
    HandingOff {
        job: TransferJobId,
        export: Box<PreparedExport>,
    },
    /// The handoff was refused or failed. The bytes are kept for the handoff
    /// control to try again; §7 requires that retry to be its own direct
    /// activation, so the generic Retry does not reach it.
    HandOffFailed {
        export: Box<PreparedExport>,
        code: TransferFailureCode,
    },
    /// The adapter reported the handoff. Only the facts remain.
    ExportHandedOff {
        source: ExportSource,
        outcome: HandoffOutcome,
    },
}

/// What an exact-catalog open of one row is for. The single encoding of
/// that choice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OpenPurpose {
    /// Library Open: validate, claim Continue, then replace the session.
    Install,
    /// Use for Continue: validate and claim Continue, then keep the
    /// candidate for the Continue route. Nothing is installed.
    SelectOnly,
    /// Row Export (task 12c): validate as a pure read and keep the bytes.
    /// Nothing is selected, promoted or installed.
    Export,
}

impl OpenPurpose {
    const fn kind(self) -> SlotRequestKind {
        match self {
            Self::Install => SlotRequestKind::Open,
            Self::SelectOnly => SlotRequestKind::UseForContinue,
            Self::Export => SlotRequestKind::Export,
        }
    }

    const fn intent(self) -> SlotIntent {
        match self {
            Self::Install | Self::SelectOnly => SlotIntent::Open,
            Self::Export => SlotIntent::Export,
        }
    }
}

/// Row-export bytes from a validated load. The source follows the load, so a
/// predecessor can never be labelled as the head.
fn prepared_export(loaded: LoadedSlot) -> PreparedExport {
    let slot = loaded.slot;
    let generation = loaded.generation;
    PreparedExport {
        source: if loaded.recovered_from_previous {
            ExportSource::RecoveredPredecessor { slot, generation }
        } else {
            ExportSource::Head { slot, generation }
        },
        suggested_name: SuggestedName::workshop_archive(&loaded.name),
        bytes: loaded.archive,
    }
}

/// The typed identity of a slot request, for the public status only.
///
/// Deliberately total over the request vocabulary rather than partial: a
/// request this machine never submits would be a construction defect, and
/// `List` is the honest answer for a state that cannot exist rather than a
/// panic in a product path.
fn slot_request_kind(request: &WorkshopStoreRequest) -> SlotRequestKind {
    match request {
        WorkshopStoreRequest::RenameSlot { .. } => SlotRequestKind::Rename,
        WorkshopStoreRequest::ArchiveSlot { .. } => SlotRequestKind::Archive,
        WorkshopStoreRequest::UnarchiveSlot { .. } => SlotRequestKind::Unarchive,
        _ => SlotRequestKind::List,
    }
}

fn slot_request_slot(request: &WorkshopStoreRequest) -> Option<SlotId> {
    match request {
        WorkshopStoreRequest::RenameSlot { slot, .. }
        | WorkshopStoreRequest::ArchiveSlot { slot }
        | WorkshopStoreRequest::UnarchiveSlot { slot } => Some(*slot),
        _ => None,
    }
}

/// Whether a request advances a slot's observable state, and therefore
/// invalidates any generation a caller is holding from an earlier list.
const fn slot_request_mutates(request: &WorkshopStoreRequest) -> bool {
    !matches!(request, WorkshopStoreRequest::ListSlots)
}

/// Owns the unchanged RulesV1 client, the Workshop persistence adapter, and
/// exactly one selected product session.
pub struct ClientRuntime<S, P, W>
where
    S: ScenarioStore,
    P: PreferencesStore,
    W: WorkshopStore,
{
    classic: AppCore<S, P>,
    workshop_store: W,
    active_session: ActiveSession,
    screen: ClientScreen,
    settings_return: ClientScreen,
    bootstrap_return: ClientScreen,
    library_return: ClientScreen,
    library: WorkshopLibraryClient,
    /// The second client instance, for Library Open. Separate from
    /// [`Self::library`] because startup Continue's outcomes are screen policy
    /// for the Loading screen, while an open's are the Library's own.
    open_client: WorkshopLibraryClient,
    slot_requests: LibrarySlots,
    slot_list: Option<SlotList>,
    continue_candidate: Option<LibraryCandidate>,
    catalog_import: CatalogImport,
    imported_catalog: Option<ValidatedCatalogPackV1>,
    /// The portable transfer adapter, when one is installed. No product entry
    /// point installs one until the §8 compatibility spike is accepted, so
    /// the handoff stays disabled with a visible reason.
    transfer: Option<Box<dyn TransferAdapter>>,
    diagnostics: VecDeque<ClientDiagnostic>,
    recovery: Option<ClientDiagnostic>,
    credits_visible: bool,
}

impl<S, P, W> ClientRuntime<S, P, W>
where
    S: ScenarioStore,
    P: PreferencesStore,
    W: WorkshopStore,
{
    /// Constructs the shell without parsing a Workshop pack, constructing a
    /// Workshop authority, or touching Workshop storage.
    pub fn new(classic: AppCore<S, P>, workshop_store: W) -> Self {
        Self {
            classic,
            workshop_store,
            active_session: ActiveSession::None,
            screen: ClientScreen::MainMenu,
            settings_return: ClientScreen::MainMenu,
            bootstrap_return: ClientScreen::MainMenu,
            library_return: ClientScreen::MainMenu,
            library: WorkshopLibraryClient::default(),
            open_client: WorkshopLibraryClient::default(),
            slot_requests: LibrarySlots::Idle,
            slot_list: None,
            continue_candidate: None,
            catalog_import: CatalogImport::Idle,
            imported_catalog: None,
            transfer: None,
            diagnostics: VecDeque::new(),
            recovery: None,
            credits_visible: false,
        }
    }

    pub const fn screen(&self) -> ClientScreen {
        self.screen
    }

    pub const fn active_session(&self) -> &ActiveSession {
        &self.active_session
    }

    pub fn classic(&self) -> &AppCore<S, P> {
        &self.classic
    }

    pub fn classic_mut(&mut self) -> &mut AppCore<S, P> {
        &mut self.classic
    }

    pub fn workshop_snapshot(&self) -> Option<&WorkshopSessionSnapshot> {
        match &self.active_session {
            ActiveSession::Workshop(session) => Some(session.snapshot()),
            ActiveSession::None | ActiveSession::Classic => None,
        }
    }

    pub fn diagnostics(&self) -> impl ExactSizeIterator<Item = &ClientDiagnostic> {
        self.diagnostics.iter()
    }

    pub const fn recovery_diagnostic(&self) -> Option<&ClientDiagnostic> {
        self.recovery.as_ref()
    }

    pub const fn credits_visible(&self) -> bool {
        self.credits_visible
    }

    pub fn dismiss_credits(&mut self) {
        self.credits_visible = false;
    }

    pub fn continue_available(&self) -> bool {
        if matches!(self.active_session, ActiveSession::Workshop(_)) {
            self.resident_workshop_continue_ready()
                || (!self.resident_workshop_blocks_replacement()
                    && self.continue_candidate.is_some())
        } else {
            self.continue_candidate.is_some()
        }
    }

    pub const fn continue_bootstrap_active(&self) -> bool {
        self.library.is_active()
    }

    /// True only while a `PutPack` job is in flight.
    ///
    /// This is deliberately narrower than "the import machine has unresolved
    /// intent": a retryable failure or a blocked start is *waiting on the user*,
    /// not on the store. Use [`Self::catalog_import_status`] for the full state.
    pub const fn catalog_import_active(&self) -> bool {
        matches!(&self.catalog_import, CatalogImport::Storing { .. })
    }

    /// The bounded typed facts the Library presents for the import machine.
    pub fn catalog_import_status(&self) -> CatalogImportStatus {
        match &self.catalog_import {
            CatalogImport::Idle => CatalogImportStatus::Idle,
            CatalogImport::Storing { material, .. } => CatalogImportStatus::Storing {
                expected_hash: material.catalog.catalog_hash(),
                starts_new_workshop: material.start_seed.is_some(),
            },
            CatalogImport::Stored { hash } => CatalogImportStatus::Stored { hash: *hash },
            CatalogImport::StoreFailed { code, material } => CatalogImportStatus::StoreFailed {
                expected_hash: material.catalog.catalog_hash(),
                code: *code,
                starts_new_workshop: material.start_seed.is_some(),
            },
            CatalogImport::StartBlocked { hash, .. } => {
                CatalogImportStatus::StartBlocked { hash: *hash }
            }
        }
    }

    /// True while the import machine holds intent only the user can resolve:
    /// an in-flight store, a retryable failure, or a blocked start.
    const fn catalog_import_unresolved(&self) -> bool {
        matches!(
            &self.catalog_import,
            CatalogImport::Storing { .. }
                | CatalogImport::StoreFailed { .. }
                | CatalogImport::StartBlocked { .. }
        )
    }

    pub fn imported_catalog_hash(&self) -> Option<CatalogHash> {
        self.imported_catalog
            .as_ref()
            .map(ValidatedCatalogPackV1::catalog_hash)
    }

    pub fn menu_capabilities(&self) -> Vec<MainMenuCapability> {
        let replacement_blocked = self.resident_workshop_blocks_replacement();
        let capabilities = vec![
            MainMenuCapability {
                route: MainMenuRoute::NewWorkshop,
                enabled: !replacement_blocked,
            },
            MainMenuCapability {
                route: MainMenuRoute::Continue,
                enabled: self.continue_available(),
            },
            MainMenuCapability {
                route: MainMenuRoute::Library,
                enabled: true,
            },
            MainMenuCapability {
                route: MainMenuRoute::ClassicSector,
                enabled: !replacement_blocked,
            },
            MainMenuCapability {
                route: MainMenuRoute::Settings,
                enabled: true,
            },
            MainMenuCapability {
                route: MainMenuRoute::Credits,
                enabled: true,
            },
        ];
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut native_capabilities = capabilities;
            native_capabilities.push(MainMenuCapability {
                route: MainMenuRoute::Quit,
                enabled: true,
            });
            native_capabilities
        }
        #[cfg(target_arch = "wasm32")]
        {
            capabilities
        }
    }

    /// Starts the explicit ListSlots -> LoadSlot -> authoritative-decode
    /// pipeline. The selected slot is never inferred from another slot.
    pub fn begin_continue_bootstrap(&mut self) -> Result<(), ClientRuntimeError> {
        if self.continue_bootstrap_active() {
            return Err(ClientRuntimeError::BootstrapActive);
        }
        self.continue_candidate = None;
        self.bootstrap_return = match self.screen {
            ClientScreen::Loading | ClientScreen::RecoverableError => {
                self.screen_for_active_session()
            }
            screen => screen,
        };
        // The active check above already refused this case, so `Active` is
        // unreachable here; it is mapped rather than unwrapped because the
        // client enforces its own precondition and this route must not
        // contradict it.
        self.library
            .begin(&mut self.workshop_store, LibraryOpen::SelectedContinue)
            .map_err(|error| match error {
                LibraryBeginError::Active => ClientRuntimeError::BootstrapActive,
                LibraryBeginError::Store(error) => {
                    self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                    ClientRuntimeError::Store(error)
                }
            })?;
        self.screen = ClientScreen::Loading;
        Ok(())
    }

    /// Polls product-level jobs, then gives the active Workshop its one typed
    /// action drain / bounded-step / persistence update.
    pub fn update(&mut self, frame_delta: Duration) -> WorkshopUpdate {
        self.poll_continue_bootstrap();
        self.poll_catalog_import();
        // Deliberately not gated on the Library being the current screen. A
        // request started in the Library and still in flight when the user
        // closes it must still reach a terminal state, or its lane leaks for
        // the store's lifetime.
        self.poll_library_slots();
        self.poll_library_open();
        self.poll_library_handoff();
        match &mut self.active_session {
            ActiveSession::Workshop(session) => {
                session.update(frame_delta, &mut self.workshop_store)
            }
            ActiveSession::None | ActiveSession::Classic => WorkshopUpdate::default(),
        }
    }

    /// Dispatches one of the visible main-menu capabilities. New Workshop uses
    /// deterministic seed zero; callers offering seeded creation can use
    /// [`Self::start_new_workshop`] directly.
    pub fn select_menu_route(
        &mut self,
        route: MainMenuRoute,
    ) -> Result<ClientRuntimeEffect, ClientRuntimeError> {
        let recovery_continue = self.screen == ClientScreen::RecoverableError
            && route == MainMenuRoute::Continue
            && self.continue_available();
        if self.screen != ClientScreen::MainMenu && !recovery_continue {
            self.push_diagnostic(
                ClientDiagnosticCode::RouteUnavailable,
                "Main-menu route requested while the main menu was not active",
            );
            return Err(ClientRuntimeError::RouteUnavailable);
        }

        if matches!(
            route,
            MainMenuRoute::NewWorkshop | MainMenuRoute::ClassicSector
        ) && self.resident_workshop_blocks_replacement()
        {
            self.push_diagnostic(
                ClientDiagnosticCode::RouteUnavailable,
                "Save the resident Workshop before replacing its active session",
            );
            return Err(ClientRuntimeError::RouteUnavailable);
        }

        match route {
            MainMenuRoute::NewWorkshop => {
                self.start_new_workshop(0)?;
                Ok(ClientRuntimeEffect::None)
            }
            MainMenuRoute::Continue => {
                self.continue_selected_workshop()?;
                Ok(ClientRuntimeEffect::None)
            }
            MainMenuRoute::Library => {
                self.open_library()?;
                Ok(ClientRuntimeEffect::None)
            }
            MainMenuRoute::ClassicSector => {
                self.active_session = ActiveSession::Classic;
                self.screen = ClientScreen::ClassicSector;
                self.recovery = None;
                Ok(ClientRuntimeEffect::None)
            }
            MainMenuRoute::Settings => {
                self.settings_return = ClientScreen::MainMenu;
                self.screen = ClientScreen::Settings;
                Ok(ClientRuntimeEffect::None)
            }
            MainMenuRoute::Credits => {
                self.credits_visible = true;
                Ok(ClientRuntimeEffect::CreditsOpened)
            }
            #[cfg(not(target_arch = "wasm32"))]
            MainMenuRoute::Quit => Ok(ClientRuntimeEffect::QuitRequested),
        }
    }

    /// Lazily validates the embedded pack and creates the requested deterministic
    /// Workshop only after the user chooses creation.
    pub fn start_new_workshop(&mut self, seed: u64) -> Result<(), ClientRuntimeError> {
        self.ensure_resident_workshop_replaceable()?;
        let catalog = decode_catalog_pack(CORE_PACK_V1).map_err(|_| {
            self.enter_recovery(
                ClientDiagnosticCode::Catalog,
                "The built-in Workshop catalog failed validation",
            );
            ClientRuntimeError::BuiltInCatalog
        })?;
        self.install_new_workshop(catalog, seed)
    }

    /// Validates arbitrary `.nyonpack.json` bytes, canonicalizes them, and
    /// begins persisting the exact content-addressed pack. The current session
    /// is left untouched until persistence succeeds.
    pub fn begin_catalog_import(
        &mut self,
        bytes: &[u8],
    ) -> Result<CatalogHash, ClientRuntimeError> {
        self.begin_catalog_import_inner(bytes, None)
    }

    /// Validates and persists a custom catalog, then installs a new paused
    /// Workshop only after the store confirms the exact expected hash.
    pub fn begin_new_workshop_from_catalog(
        &mut self,
        bytes: &[u8],
        seed: u64,
    ) -> Result<CatalogHash, ClientRuntimeError> {
        self.begin_catalog_import_inner(bytes, Some(seed))
    }

    /// Starts a Workshop from the most recently validated and successfully
    /// persisted imported catalog.
    pub fn start_new_workshop_from_imported_catalog(
        &mut self,
        seed: u64,
    ) -> Result<(), ClientRuntimeError> {
        self.ensure_resident_workshop_replaceable()?;
        let catalog = self
            .imported_catalog
            .clone()
            .ok_or(ClientRuntimeError::ImportedCatalogUnavailable)?;
        self.install_new_workshop(catalog, seed)
    }

    /// Returns the canonical bytes of the active Workshop's exact catalog.
    pub fn export_active_catalog(&self) -> Result<Box<[u8]>, ClientRuntimeError> {
        let ActiveSession::Workshop(session) = &self.active_session else {
            return Err(ClientRuntimeError::WorkshopInactive);
        };
        encode_catalog_pack(session.history().catalog())
            .map(Vec::into_boxed_slice)
            .map_err(|_| ClientRuntimeError::InvalidCatalog)
    }

    /// Retries a [`CatalogImportStatus::StoreFailed`] import from its retained
    /// material, without asking the user to reselect the file.
    pub fn retry_catalog_import(&mut self) -> Result<CatalogHash, ClientRuntimeError> {
        match std::mem::replace(&mut self.catalog_import, CatalogImport::Idle) {
            CatalogImport::StoreFailed { material, .. } => self.start_catalog_store(material),
            other => {
                self.catalog_import = other;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No retryable Workshop catalog import is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// Clears pending client intent for the import machine and nothing else.
    ///
    /// A pack that already reached durable storage stays stored: the store
    /// request vocabulary has no pack deletion at all, and the recorded
    /// imported catalog is a fact about what persisted, not pending intent.
    /// The active session, its digest, its queued work, and its recovery
    /// obligations are untouched.
    ///
    /// Cancel is offered in the two states that wait on the user and, since the
    /// store gained [`WorkshopStore::abandon`], in `Storing` too. Abandoning
    /// the in-flight `PutPack` releases its Commit lane immediately, so the
    /// resident Workshop's own commit and recovery-persistence obligation stay
    /// eligible to run; previously the only way out of that lane was to poll to
    /// completion, so cancel had to be refused.
    ///
    /// Abandoning does not recall the write. A pack that was going to store
    /// still stores, which is consistent with the paragraph above: cancel never
    /// withdraws a stored pack either way. What it drops is the client's
    /// interest in the outcome.
    ///
    /// Cancellation is ordinary, so it pushes no diagnostic; only the refusal
    /// does.
    pub fn cancel_catalog_import(&mut self) -> Result<(), ClientRuntimeError> {
        match std::mem::replace(&mut self.catalog_import, CatalogImport::Idle) {
            CatalogImport::Storing { job, .. } => {
                self.workshop_store.abandon(job);
                Ok(())
            }
            CatalogImport::StoreFailed { .. } | CatalogImport::StartBlocked { .. } => Ok(()),
            restored @ (CatalogImport::Idle | CatalogImport::Stored { .. }) => {
                self.catalog_import = restored;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No cancellable Workshop catalog import is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// Starts the Workshop a [`CatalogImportStatus::StartBlocked`] import still
    /// owes, once the resident Workshop has become replaceable again.
    pub fn start_workshop_from_blocked_import(&mut self) -> Result<(), ClientRuntimeError> {
        let (hash, seed, catalog) = match &self.catalog_import {
            CatalogImport::StartBlocked {
                hash,
                seed,
                catalog,
            } => (*hash, *seed, (**catalog).clone()),
            _ => {
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No stored Workshop catalog is waiting to start a new Workshop",
                );
                return Err(ClientRuntimeError::RouteUnavailable);
            }
        };
        self.install_new_workshop(catalog, seed)?;
        self.catalog_import = CatalogImport::Stored { hash };
        Ok(())
    }

    /// A fresh import is accepted from `Idle` and from a terminal `Stored`, and
    /// refused while the machine holds intent only Retry, Cancel, or Start When
    /// Safe can resolve. Silently overwriting retained material would discard
    /// a decision the user has not made yet.
    fn begin_catalog_import_inner(
        &mut self,
        bytes: &[u8],
        start_seed: Option<u64>,
    ) -> Result<CatalogHash, ClientRuntimeError> {
        if self.catalog_import_unresolved() {
            return Err(ClientRuntimeError::CatalogImportActive);
        }
        if start_seed.is_some() {
            self.ensure_resident_workshop_replaceable()?;
        }
        // Content that never validates has no retry material and no Library
        // state: it is a typed rejection that leaves the runtime exactly where
        // it was, not a recoverable error that replaces the screen.
        let catalog = decode_catalog_pack(bytes).map_err(|_| {
            self.push_diagnostic(
                ClientDiagnosticCode::Catalog,
                "The imported Workshop catalog failed validation",
            );
            ClientRuntimeError::InvalidCatalog
        })?;
        let canonical_pack = encode_catalog_pack(&catalog).map_err(|_| {
            self.push_diagnostic(
                ClientDiagnosticCode::Catalog,
                "The imported Workshop catalog could not be canonicalized",
            );
            ClientRuntimeError::InvalidCatalog
        })?;
        self.start_catalog_store(ImportMaterial {
            catalog: Box::new(catalog),
            canonical_pack: canonical_pack.into_boxed_slice(),
            start_seed,
        })
    }

    fn start_catalog_store(
        &mut self,
        material: ImportMaterial,
    ) -> Result<CatalogHash, ClientRuntimeError> {
        let expected_hash = material.catalog.catalog_hash();
        match self.workshop_store.start(WorkshopStoreRequest::PutPack {
            canonical_pack: material.canonical_pack.clone(),
        }) {
            Ok(job) => {
                self.catalog_import = CatalogImport::Storing { job, material };
                Ok(expected_hash)
            }
            Err(error) => {
                let message = error.to_string();
                self.retain_failed_catalog_import(ClientDiagnosticCode::Store, message, material);
                Err(ClientRuntimeError::Store(error))
            }
        }
    }

    fn poll_catalog_import(&mut self) {
        // Only `Storing` polls. Every other state is terminal or waits on the
        // user, and must survive an arbitrary number of further frames.
        let CatalogImport::Storing { job, .. } = &self.catalog_import else {
            return;
        };
        let job = *job;
        match self.workshop_store.poll(job) {
            StoreJobState::Pending => {}
            StoreJobState::Unknown => self.fail_catalog_import(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop storage forgot the active catalog import job",
            ),
            StoreJobState::Complete(Err(error)) => {
                self.fail_catalog_import(ClientDiagnosticCode::Store, error.to_string());
            }
            StoreJobState::Complete(Ok(WorkshopStoreResult::PackStored { hash })) => {
                self.finish_catalog_import(hash);
            }
            StoreJobState::Complete(Ok(_)) => self.fail_catalog_import(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop catalog import returned an unexpected result",
            ),
        }
    }

    /// Moves an in-flight import to its retryable failure state, retaining the
    /// validated pack so Retry never asks the user to reselect the file.
    fn fail_catalog_import(&mut self, code: ClientDiagnosticCode, message: impl Into<String>) {
        let CatalogImport::Storing { material, .. } =
            std::mem::replace(&mut self.catalog_import, CatalogImport::Idle)
        else {
            return;
        };
        self.retain_failed_catalog_import(code, message, material);
    }

    fn retain_failed_catalog_import(
        &mut self,
        code: ClientDiagnosticCode,
        message: impl Into<String>,
        material: ImportMaterial,
    ) {
        self.push_diagnostic(code, message);
        self.catalog_import = CatalogImport::StoreFailed { code, material };
    }

    fn finish_catalog_import(&mut self, hash: CatalogHash) {
        let CatalogImport::Storing { material, .. } =
            std::mem::replace(&mut self.catalog_import, CatalogImport::Idle)
        else {
            return;
        };
        if hash != material.catalog.catalog_hash() {
            self.retain_failed_catalog_import(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop storage persisted a catalog under the wrong hash",
                material,
            );
            return;
        }
        self.imported_catalog = Some((*material.catalog).clone());
        let Some(seed) = material.start_seed else {
            self.catalog_import = CatalogImport::Stored { hash };
            return;
        };
        let catalog = material.catalog;
        if self.install_new_workshop((*catalog).clone(), seed).is_err() {
            // The resident Workshop became unreplaceable while the pack was
            // persisting. The pack is durably stored and is never withdrawn, so
            // this is the addendum's blocked-start state offering Start When
            // Safe and Cancel, not a dropped failure and not the global
            // recovery screen.
            self.push_diagnostic(
                ClientDiagnosticCode::RouteUnavailable,
                "The imported Workshop catalog was saved, but the resident Workshop changed \
                 while it was saving. Save the resident Workshop, then start a new Workshop \
                 from the imported catalog.",
            );
            self.catalog_import = CatalogImport::StartBlocked {
                hash,
                seed,
                catalog,
            };
            return;
        }
        self.catalog_import = CatalogImport::Stored { hash };
    }

    fn install_new_workshop(
        &mut self,
        catalog: ValidatedCatalogPackV1,
        seed: u64,
    ) -> Result<(), ClientRuntimeError> {
        self.ensure_resident_workshop_replaceable()?;
        let history = WorkshopHistory::from_seed_u64(catalog, seed);
        self.active_session = ActiveSession::Workshop(WorkshopSession::new(history));
        self.screen = ClientScreen::GalaxyWorkshop;
        self.recovery = None;
        Ok(())
    }

    pub fn enqueue_workshop_action(
        &mut self,
        action: WorkshopAction,
    ) -> Result<(), ClientRuntimeError> {
        match &mut self.active_session {
            ActiveSession::Workshop(session) => session
                .enqueue(action)
                .map_err(ClientRuntimeError::WorkshopAction),
            ActiveSession::None | ActiveSession::Classic => {
                self.push_diagnostic(
                    ClientDiagnosticCode::WorkshopInactive,
                    "Workshop action requested without an active Workshop",
                );
                Err(ClientRuntimeError::WorkshopInactive)
            }
        }
    }

    pub fn open_settings(&mut self) {
        self.settings_return = self.screen_for_active_session();
        self.screen = ClientScreen::Settings;
    }

    pub fn close_settings(&mut self) {
        if self.screen == ClientScreen::Settings {
            self.screen = self.settings_return;
        }
    }

    /// Opens the Library laterally from one of the three product screens.
    ///
    /// **It must not prepare a durable transition.** `return_to_main_menu`
    /// deliberately pauses and saves a resident Workshop before leaving it;
    /// this route deliberately does not, because §2 requires the resident
    /// session, its dirty state, its queued actions and its recovery
    /// obligations to survive opening the Library untouched. Gating happens per
    /// action inside the Library.
    ///
    /// The return screen is the screen actually being left, not
    /// [`Self::screen_for_active_session`]. Those differ exactly when a
    /// Workshop is resident while the user is on the main menu — the state
    /// `return_to_main_menu` produces — and the addendum's "returns to the
    /// exact prior screen" is the stricter of the two.
    ///
    /// `Library`, `Loading` and `RecoverableError` are refused. Re-entry would
    /// make the Library its own return screen and strand the user; the two
    /// transient screens are owned by machines that write `self.screen`
    /// themselves, so a Library opened from either would be silently replaced.
    ///
    /// That enumeration is not the whole set of screen-writing machines, and
    /// refusing those two screens does not make the Library immune to the rest.
    /// A catalog import started *with a seed* calls `install_new_workshop` from
    /// inside [`Self::update`] when its pack lands, which writes
    /// `ClientScreen::GalaxyWorkshop` over whatever is on screen, the Library
    /// included. That is inherited behavior and is left as it is: the user
    /// asked for a new Workshop from that pack, and arriving in it when the
    /// pack stores is the honest answer rather than parking them in a Library
    /// whose `library_return` no longer describes anything. It is recorded here
    /// so the next reader does not re-derive the incomplete list above.
    pub fn open_library(&mut self) -> Result<(), ClientRuntimeError> {
        if !matches!(
            self.screen,
            ClientScreen::MainMenu | ClientScreen::ClassicSector | ClientScreen::GalaxyWorkshop
        ) {
            self.push_diagnostic(
                ClientDiagnosticCode::RouteUnavailable,
                "Library opens from the main menu, Classic Sector, or Galaxy Workshop",
            );
            return Err(ClientRuntimeError::RouteUnavailable);
        }
        self.library_return = self.screen;
        self.screen = ClientScreen::Library;
        // A refusal here is already recorded in the slot-request status, which
        // is the surface offering Retry and Cancel. Opening the screen itself
        // succeeded, so it must not report the store's answer as its own.
        if matches!(self.slot_requests, LibrarySlots::Idle) {
            let _ = self.dispatch_slot_request(WorkshopStoreRequest::ListSlots);
        }
        Ok(())
    }

    /// Returns to the exact screen the Library was opened from.
    ///
    /// Symmetrically to [`Self::open_library`], it touches neither the active
    /// session nor the credits overlay nor any in-flight slot request: a
    /// request still in flight keeps being polled by [`Self::update`].
    pub fn close_library(&mut self) {
        if self.screen == ClientScreen::Library {
            self.screen = self.library_return;
        }
    }

    /// The listed slots, or `None` when there is no list worth trusting.
    ///
    /// **`None` is never "no slots"**: an empty store lists as `Some` with an
    /// empty `slots`, and that is the only representation of emptiness. `None`
    /// has exactly one meaning for a caller — there is no list to render, ask
    /// for one — reached three ways: the Library has not listed yet; a
    /// successful mutation dropped a list that now misstates a name, an
    /// archived flag or the Continue marker; or a cancelled mutation dropped
    /// one whose generations the abandoned write may have invalidated.
    ///
    /// Only the first two are followed by an automatic re-list, so `None` with
    /// [`LibrarySlotsStatus::Idle`] is a real resting state after a cancel and
    /// not a frame in flight. Pair this with
    /// [`Self::library_slots_status`] rather than reading either alone.
    pub const fn library_slots(&self) -> Option<&SlotList> {
        self.slot_list.as_ref()
    }

    pub fn library_slots_status(&self) -> LibrarySlotsStatus {
        match &self.slot_requests {
            LibrarySlots::Idle => LibrarySlotsStatus::Idle,
            LibrarySlots::Working { request, .. } => LibrarySlotsStatus::Working {
                kind: slot_request_kind(request),
                slot: slot_request_slot(request),
            },
            LibrarySlots::Failed { request, code } => LibrarySlotsStatus::Failed {
                kind: slot_request_kind(request),
                slot: slot_request_slot(request),
                code: *code,
            },
            LibrarySlots::Opening { slot, purpose, .. } => LibrarySlotsStatus::Working {
                kind: purpose.kind(),
                slot: Some(*slot),
            },
            LibrarySlots::OpenFailed {
                slot,
                code,
                purpose,
                ..
            } => LibrarySlotsStatus::Failed {
                kind: purpose.kind(),
                slot: Some(*slot),
                code: *code,
            },
            LibrarySlots::Held { candidate } => LibrarySlotsStatus::Held {
                slot: candidate.loaded.slot,
                recovered: candidate.loaded.recovered_from_previous,
            },
            LibrarySlots::ExportRecoveryOffered { loaded } => {
                LibrarySlotsStatus::ExportRecoveryOffered { slot: loaded.slot }
            }
            LibrarySlots::ExportReady { export } => LibrarySlotsStatus::ExportReady {
                source: export.source,
            },
            LibrarySlots::HandingOff { export, .. } => LibrarySlotsStatus::Working {
                kind: SlotRequestKind::HandOff,
                slot: export.source.slot(),
            },
            LibrarySlots::HandOffFailed { export, code } => LibrarySlotsStatus::HandOffFailed {
                source: export.source,
                code: *code,
            },
            LibrarySlots::ExportHandedOff { source, outcome } => {
                LibrarySlotsStatus::ExportHandedOff {
                    source: *source,
                    outcome: *outcome,
                }
            }
        }
    }

    /// The validated export bytes, while the lane holds them: Ready, with the
    /// adapter, or after a failed handoff. Released once handed off or
    /// discarded.
    pub fn prepared_export(&self) -> Option<&PreparedExport> {
        match &self.slot_requests {
            LibrarySlots::ExportReady { export }
            | LibrarySlots::HandingOff { export, .. }
            | LibrarySlots::HandOffFailed { export, .. } => Some(export),
            _ => None,
        }
    }

    /// Installs the portable transfer adapter (route-design task 11).
    ///
    /// Refused while a handoff is in flight, because the job it owns belongs
    /// to the adapter it would replace and could then never be polled or
    /// abandoned.
    pub fn install_transfer_adapter(
        &mut self,
        adapter: Box<dyn TransferAdapter>,
    ) -> Result<(), ClientRuntimeError> {
        if matches!(self.slot_requests, LibrarySlots::HandingOff { .. }) {
            return Err(ClientRuntimeError::LibraryRequestActive);
        }
        self.transfer = Some(adapter);
        Ok(())
    }

    /// Whether a portable transfer adapter is installed.
    pub const fn transfer_available(&self) -> bool {
        self.transfer.is_some()
    }

    /// Stage 2 of row Export: hands the prepared bytes to the transfer
    /// adapter (addendum §7).
    ///
    /// Accepted only from [`LibrarySlotsStatus::ExportReady`] and
    /// [`LibrarySlotsStatus::HandOffFailed`]. Every refusal and failure keeps
    /// the prepared bytes: they are expensive to re-derive and the user's to
    /// discard, so this is the route design's "preserve at dispatch" case.
    /// With no adapter installed the state is left exactly as it was.
    pub fn hand_off_library_export(&mut self) -> Result<(), ClientRuntimeError> {
        let export = match std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle) {
            LibrarySlots::ExportReady { export } | LibrarySlots::HandOffFailed { export, .. }
                if self.transfer.is_some() =>
            {
                export
            }
            restored @ (LibrarySlots::ExportReady { .. } | LibrarySlots::HandOffFailed { .. }) => {
                self.slot_requests = restored;
                return Err(ClientRuntimeError::TransferUnavailable);
            }
            other => {
                self.slot_requests = other;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No prepared Workshop Library copy is waiting for a handoff",
                );
                return Err(ClientRuntimeError::RouteUnavailable);
            }
        };
        let request = TransferRequest::HandOffExport {
            kind: export.kind(),
            suggested_name: export.suggested_name.clone(),
            bytes: export.bytes.clone(),
        };
        let started = match self.transfer.as_mut() {
            Some(adapter) => adapter.start(request),
            None => Err(TransferError::new(TransferFailureCode::Unavailable)),
        };
        match started {
            Ok(job) => {
                self.slot_requests = LibrarySlots::HandingOff { job, export };
                Ok(())
            }
            Err(error) => {
                self.fail_handoff(export, error.code);
                Err(ClientRuntimeError::Transfer(error.code))
            }
        }
    }

    fn fail_handoff(&mut self, export: Box<PreparedExport>, code: TransferFailureCode) {
        self.push_diagnostic(ClientDiagnosticCode::Transfer, code.message());
        self.slot_requests = LibrarySlots::HandOffFailed { export, code };
    }

    /// Delivers the handoff's one terminal answer. Not gated on the Library
    /// being on screen, like the other Library polls.
    fn poll_library_handoff(&mut self) {
        let LibrarySlots::HandingOff { job, .. } = &self.slot_requests else {
            return;
        };
        let job = *job;
        let state = match self.transfer.as_mut() {
            Some(adapter) => adapter.poll(job),
            None => TransferJobState::Unknown,
        };
        if matches!(state, TransferJobState::Pending) {
            return;
        }
        let LibrarySlots::HandingOff { export, .. } =
            std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle)
        else {
            return;
        };
        match state {
            TransferJobState::Pending => {}
            TransferJobState::Complete(Ok(TransferOutcome::ExportHandedOff(outcome))) => {
                self.slot_requests = LibrarySlots::ExportHandedOff {
                    source: export.source,
                    outcome,
                };
            }
            // Dismissing the save is ordinary: back to Ready, no diagnostic.
            TransferJobState::Complete(Ok(TransferOutcome::Cancelled)) => {
                self.slot_requests = LibrarySlots::ExportReady { export };
            }
            TransferJobState::Complete(Ok(TransferOutcome::ImportChosen { .. }))
            | TransferJobState::Unknown => {
                self.fail_handoff(export, TransferFailureCode::Protocol);
            }
            TransferJobState::Complete(Err(error)) => self.fail_handoff(export, error.code),
        }
    }

    /// Re-lists the bounded slots.
    pub fn refresh_library_slots(&mut self) -> Result<(), ClientRuntimeError> {
        self.start_slot_request(WorkshopStoreRequest::ListSlots)
    }

    /// Renames one slot through the canonical [`SlotName`] type.
    ///
    /// Not gated on the resident Workshop being replaceable: §6 allows Rename
    /// and Unarchive for unrelated slots to proceed while an active session is
    /// not replaceable, and contention is the store's lane to refuse, not this
    /// runtime's to pre-empt.
    pub fn rename_library_slot(
        &mut self,
        slot: SlotId,
        name: SlotName,
    ) -> Result<(), ClientRuntimeError> {
        self.start_slot_request(WorkshopStoreRequest::RenameSlot { slot, name })
    }

    /// Archives one slot.
    ///
    /// §3's refusal of the resident Workshop's own slot is enforced in
    /// [`Self::dispatch_slot_request`], not here, so that Retry is gated by the
    /// same rule as the first attempt.
    pub fn archive_library_slot(&mut self, slot: SlotId) -> Result<(), ClientRuntimeError> {
        self.start_slot_request(WorkshopStoreRequest::ArchiveSlot { slot })
    }

    /// Clears one slot's archived flag and nothing else.
    pub fn unarchive_library_slot(&mut self, slot: SlotId) -> Result<(), ClientRuntimeError> {
        self.start_slot_request(WorkshopStoreRequest::UnarchiveSlot { slot })
    }

    /// Retries a [`LibrarySlotsStatus::Failed`] request from its retained
    /// request, without asking the user to re-pick the row.
    ///
    /// A retry that has become illegal in the meantime — an Archive whose slot
    /// is now resident — is refused by [`Self::dispatch_slot_request`] without
    /// starting anything. That refusal must not also swallow the decision the
    /// user still has to resolve, so the retained request is put back and the
    /// Library keeps offering Retry and Cancel over it.
    pub fn retry_library_slot_request(&mut self) -> Result<(), ClientRuntimeError> {
        match std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle) {
            LibrarySlots::Failed { request, code } => {
                let outcome = self.dispatch_slot_request(request.clone());
                if outcome.is_err() && matches!(self.slot_requests, LibrarySlots::Idle) {
                    self.slot_requests = LibrarySlots::Failed { request, code };
                }
                outcome
            }
            // A conflict is resolved by re-listing: repeating the request would
            // only find the same stale generation again.
            LibrarySlots::OpenFailed {
                code: ClientDiagnosticCode::StaleSave,
                ..
            } => {
                self.slot_list = None;
                self.dispatch_slot_request(WorkshopStoreRequest::ListSlots)
            }
            LibrarySlots::OpenFailed {
                slot,
                generation,
                purpose,
                code,
            } => {
                let outcome = self.dispatch_open(slot, generation, purpose);
                if outcome.is_err() && matches!(self.slot_requests, LibrarySlots::Idle) {
                    self.slot_requests = LibrarySlots::OpenFailed {
                        slot,
                        generation,
                        purpose,
                        code,
                    };
                }
                outcome
            }
            other => {
                self.slot_requests = other;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No retryable Workshop Library slot request is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// Clears pending Library slot intent and nothing else.
    ///
    /// Cancelling an in-flight request abandons its job so the lane is released
    /// immediately, which matters because three of the four sit in the Commit
    /// lane the resident Workshop needs. Abandoning drops the outcome, not the
    /// work: a mutation that was going to land still lands, so the cached list
    /// is dropped with it. The store's own contract is that a caller which
    /// abandons a mutation must re-list before trusting any generation it held,
    /// and discarding the list is how that is enforced rather than documented.
    ///
    /// Cancellation is ordinary, so it pushes no diagnostic; only the refusal
    /// does.
    pub fn cancel_library_slot_request(&mut self) -> Result<(), ClientRuntimeError> {
        match std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle) {
            LibrarySlots::Working { job, request } => {
                self.workshop_store.abandon(job);
                if slot_request_mutates(&request) {
                    self.slot_list = None;
                }
                Ok(())
            }
            LibrarySlots::Failed { request, .. } => {
                if slot_request_mutates(&request) {
                    self.slot_list = None;
                }
                Ok(())
            }
            // An open that reached `Selecting`, or finished it and is held, may
            // have moved the Continue marker (see
            // `WorkshopLibraryClient::abandon`), so the list is dropped with it.
            // An export never starts a mutation, so the list it was
            // activated from is still true and is kept.
            LibrarySlots::Opening {
                purpose: OpenPurpose::Export,
                ..
            } => {
                self.open_client.abandon(&mut self.workshop_store);
                Ok(())
            }
            LibrarySlots::Opening { .. } => {
                self.open_client.abandon(&mut self.workshop_store);
                self.slot_list = None;
                Ok(())
            }
            // Stop waiting for the adapter. The outcome is dropped, not the
            // work: a download already accepted cannot be recalled, so
            // nothing is claimed either way and the bytes stay ready.
            LibrarySlots::HandingOff { job, export } => {
                if let Some(adapter) = self.transfer.as_mut() {
                    adapter.abandon(job);
                }
                self.slot_requests = LibrarySlots::ExportReady { export };
                Ok(())
            }
            // Discarding prepared or offered export bytes releases them and
            // changes nothing stored; dismissing a handoff receipt frees the
            // lane.
            LibrarySlots::ExportRecoveryOffered { .. }
            | LibrarySlots::ExportReady { .. }
            | LibrarySlots::HandOffFailed { .. }
            | LibrarySlots::ExportHandedOff { .. } => Ok(()),
            LibrarySlots::Held { .. } => {
                self.slot_list = None;
                Ok(())
            }
            LibrarySlots::OpenFailed { .. } => Ok(()),
            LibrarySlots::Idle => {
                self.slot_requests = LibrarySlots::Idle;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No cancellable Workshop Library slot request is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// Opens one listed row through the exact-catalog client (addendum §4).
    ///
    /// `generation` is the head the activated row displayed. The client
    /// compares it before replaying, and on a clean head claims the Continue
    /// marker with a compare-and-swap before anything is installed, so the
    /// resident session is replaced only after every store mutation the open
    /// needs has succeeded (§4 item 10).
    ///
    /// Refused, with nothing started, while another Library request holds the
    /// lane, while startup Continue is running, while the resident Workshop
    /// cannot be replaced (§6), or for the resident Workshop's own slot.
    pub fn open_library_slot(
        &mut self,
        slot: SlotId,
        generation: SaveGeneration,
    ) -> Result<(), ClientRuntimeError> {
        if !matches!(self.slot_requests, LibrarySlots::Idle) {
            return Err(ClientRuntimeError::LibraryRequestActive);
        }
        self.dispatch_open(slot, generation, OpenPurpose::Install)
    }

    /// Makes one listed row the Continue save without opening it (§4).
    ///
    /// Runs the same exact-catalog validation and the same generation-checked
    /// `SelectContinue` as Open, then keeps the validated candidate where
    /// startup Continue keeps its own, so the Continue route installs it
    /// without a second replay.
    ///
    /// **Refused while any Workshop is resident**, which is stricter than §4's
    /// replacement gate. A resident session tracks its own Continue selection
    /// and re-selects its slot whenever it saves, so a marker moved underneath
    /// it would disagree with what Continue returns to in this run and be
    /// silently undone by the next save.
    pub fn use_library_slot_for_continue(
        &mut self,
        slot: SlotId,
        generation: SaveGeneration,
    ) -> Result<(), ClientRuntimeError> {
        if !matches!(self.slot_requests, LibrarySlots::Idle) {
            return Err(ClientRuntimeError::LibraryRequestActive);
        }
        self.dispatch_open(slot, generation, OpenPurpose::SelectOnly)
    }

    /// Prepares a portable copy of one listed row (§4, route-design task 12c).
    ///
    /// The same exact-catalog validation and full replay as Open, as a pure
    /// read: no `SelectContinue`, no promotion, no installation. Validated
    /// bytes are held as [`LibrarySlotsStatus::ExportReady`] until the user
    /// discards them or [`Self::hand_off_library_export`] hands them off.
    ///
    /// **Not gated on the resident Workshop.** §6 does not name row Export and
    /// §4 says it mutates nothing, so neither an unsaved resident nor the
    /// resident's own slot is a reason to refuse it: exporting the resident's
    /// slot reads its stored generation, not the in-memory session, and the
    /// status says which generation it is. Refused only while another Library
    /// request holds the lane or startup Continue is running.
    pub fn export_library_slot(
        &mut self,
        slot: SlotId,
        generation: SaveGeneration,
    ) -> Result<(), ClientRuntimeError> {
        if !matches!(self.slot_requests, LibrarySlots::Idle) {
            return Err(ClientRuntimeError::LibraryRequestActive);
        }
        self.dispatch_open(slot, generation, OpenPurpose::Export)
    }

    /// Accepts §4's export-recovery choice: the already-validated predecessor
    /// becomes the prepared bytes, labelled
    /// [`ExportSource::RecoveredPredecessor`]. Calls neither
    /// `PromoteRecoveredSlot` nor `SelectContinue`; repairing the slot is
    /// still Open's job.
    pub fn accept_library_export_recovery(&mut self) -> Result<(), ClientRuntimeError> {
        match std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle) {
            LibrarySlots::ExportRecoveryOffered { loaded } => {
                self.slot_requests = LibrarySlots::ExportReady {
                    export: Box::new(prepared_export(*loaded)),
                };
                Ok(())
            }
            other => {
                self.slot_requests = other;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No Workshop Library export recovery is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// Installs the held candidate the user accepted.
    ///
    /// Still refused while the resident Workshop cannot be replaced; the
    /// candidate stays held, so accepting again once it is safe needs no
    /// replay.
    pub fn accept_library_open(&mut self) -> Result<(), ClientRuntimeError> {
        match std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle) {
            LibrarySlots::Held { candidate } => {
                if let Err(error) = self.ensure_resident_workshop_replaceable() {
                    self.slot_requests = LibrarySlots::Held { candidate };
                    return Err(error);
                }
                self.install_opened(*candidate);
                Ok(())
            }
            other => {
                self.slot_requests = other;
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "No validated Workshop Library open is waiting",
                );
                Err(ClientRuntimeError::RouteUnavailable)
            }
        }
    }

    /// The one point at which a Library open starts, for the first attempt and
    /// for Retry alike, so both are gated by the same rules.
    ///
    /// Every refusal here is checked before the client begins, so a refused
    /// open reaches no store lane and needs no compensation.
    fn dispatch_open(
        &mut self,
        slot: SlotId,
        generation: SaveGeneration,
        purpose: OpenPurpose,
    ) -> Result<(), ClientRuntimeError> {
        if self.library.is_active() {
            return Err(ClientRuntimeError::BootstrapActive);
        }
        if purpose == OpenPurpose::SelectOnly
            && matches!(self.active_session, ActiveSession::Workshop(_))
        {
            self.push_diagnostic(
                ClientDiagnosticCode::RouteUnavailable,
                "Continue follows the resident Workshop while it is open",
            );
            return Err(ClientRuntimeError::RouteUnavailable);
        }
        // Both residency refusals protect an installation or a Continue
        // selection. An export performs neither (§4), so neither applies.
        if purpose != OpenPurpose::Export {
            if self.resident_slot() == Some(slot) {
                self.push_diagnostic(
                    ClientDiagnosticCode::RouteUnavailable,
                    "The resident Workshop's own slot is already open",
                );
                return Err(ClientRuntimeError::RouteUnavailable);
            }
            self.ensure_resident_workshop_replaceable()?;
        }
        match self.open_client.begin(
            &mut self.workshop_store,
            LibraryOpen::Slot {
                slot,
                expected_generation: generation,
                intent: purpose.intent(),
            },
        ) {
            Ok(()) => {
                self.slot_requests = LibrarySlots::Opening {
                    slot,
                    generation,
                    purpose,
                };
                Ok(())
            }
            // `open_client` is active only while the lane holds `Opening`,
            // which the callers have already ruled out.
            Err(LibraryBeginError::Active) => Err(ClientRuntimeError::LibraryRequestActive),
            Err(LibraryBeginError::Store(error)) => {
                self.push_diagnostic(ClientDiagnosticCode::Store, error.to_string());
                self.slot_requests = LibrarySlots::OpenFailed {
                    slot,
                    generation,
                    purpose,
                    code: ClientDiagnosticCode::Store,
                };
                Err(ClientRuntimeError::Store(error))
            }
        }
    }

    /// Translates one open outcome into Library state. Never
    /// [`Self::enter_recovery`]: §6 keeps Library failures on the Library.
    fn poll_library_open(&mut self) {
        let LibrarySlots::Opening {
            slot,
            generation,
            purpose,
        } = self.slot_requests
        else {
            return;
        };
        let failed = |code| LibrarySlots::OpenFailed {
            slot,
            generation,
            purpose,
            code,
        };
        match self.open_client.poll(&mut self.workshop_store) {
            LibraryEvent::Pending => {}
            // An export never installs, so neither the screen nor the resident
            // decides anything here. A predecessor is only ever offered: §4
            // says an invalid head "never silently exports its predecessor".
            LibraryEvent::Ready(candidate) if purpose == OpenPurpose::Export => {
                let LibraryCandidate { loaded, .. } = *candidate;
                self.slot_requests = if loaded.recovered_from_previous {
                    LibrarySlots::ExportRecoveryOffered {
                        loaded: Box::new(loaded),
                    }
                } else {
                    LibrarySlots::ExportReady {
                        export: Box::new(prepared_export(loaded)),
                    }
                };
            }
            // A clean head that was only to be selected is now the Continue
            // save: keep it where startup Continue keeps its candidate, and
            // re-list, because the marker moved. A predecessor falls through
            // to the held offer below, since installing it is the only route
            // that repairs the slot.
            LibraryEvent::Ready(candidate)
                if purpose == OpenPurpose::SelectOnly
                    && !candidate.loaded.recovered_from_previous =>
            {
                self.continue_candidate = Some(*candidate);
                self.slot_requests = LibrarySlots::Idle;
                self.slot_list = None;
                let _ = self.dispatch_slot_request(WorkshopStoreRequest::ListSlots);
            }
            LibraryEvent::Ready(candidate) => {
                // Installing swaps the screen, so it happens only while the
                // user is still looking at the Library and nothing forbids
                // it. A predecessor always waits for explicit acceptance.
                if candidate.loaded.recovered_from_previous
                    || self.screen != ClientScreen::Library
                    || self.resident_workshop_blocks_replacement()
                {
                    self.slot_requests = LibrarySlots::Held { candidate };
                } else {
                    self.slot_requests = LibrarySlots::Idle;
                    self.install_opened(*candidate);
                }
            }
            LibraryEvent::Failed(diagnostic) => {
                self.slot_requests = failed(diagnostic.code);
                self.push_diagnostic(diagnostic.code, diagnostic.message);
            }
            LibraryEvent::ProtocolFailure(message) => {
                self.slot_requests = failed(ClientDiagnosticCode::StoreProtocol);
                self.push_diagnostic(ClientDiagnosticCode::StoreProtocol, message);
            }
            // An explicit open reads its row from the list, never the marker.
            LibraryEvent::NoCandidate => {
                self.slot_requests = failed(ClientDiagnosticCode::StoreProtocol);
                self.push_diagnostic(
                    ClientDiagnosticCode::StoreProtocol,
                    "A Library open reported no Continue candidate it never asked for",
                );
            }
            // §4's refreshable conflict. The validated candidate a refused
            // compare-and-swap carries is not kept: it is a generation the
            // store no longer calls the head, so nothing could install it, and
            // the retry §4 offers is "after refresh", which re-lists.
            LibraryEvent::StaleRow { .. }
            | LibraryEvent::ArchivedRow { .. }
            | LibraryEvent::ContinueConflict { .. } => {
                self.slot_requests = failed(ClientDiagnosticCode::StaleSave);
                self.push_diagnostic(
                    ClientDiagnosticCode::StaleSave,
                    "The Library row changed before it could be opened",
                );
            }
        }
    }

    /// Replaces the active session with a validated candidate.
    ///
    /// The cached list and any startup Continue candidate are dropped: a clean
    /// open has just moved the Continue marker, and a recovered one is about to
    /// through the session's own promotion obligation.
    fn install_opened(&mut self, candidate: LibraryCandidate) {
        self.active_session = ActiveSession::Workshop(WorkshopSession::from_loaded(
            candidate.history,
            candidate.loaded.slot,
            candidate.loaded.generation,
            candidate.loaded.head_generation,
            candidate.loaded.recovered_from_previous,
        ));
        self.continue_candidate = None;
        self.slot_list = None;
        self.screen = ClientScreen::GalaxyWorkshop;
        self.recovery = None;
    }

    /// Drops a validated Continue candidate that names `slot`.
    ///
    /// Called when an `ArchiveSlot` reaches the store, because from that moment
    /// the archive lands whether or not its outcome is ever observed. The store
    /// clears its own Continue marker, but it cannot reach a candidate this
    /// runtime is already holding in memory.
    fn withdraw_continue_candidate(&mut self, slot: SlotId) {
        if self
            .continue_candidate
            .as_ref()
            .is_some_and(|candidate| candidate.loaded.slot == slot)
        {
            self.continue_candidate = None;
        }
    }

    /// The slot a resident Workshop currently occupies, if any.
    pub(crate) fn resident_slot(&self) -> Option<SlotId> {
        match &self.active_session {
            ActiveSession::Workshop(session) => session.snapshot().store.slot,
            ActiveSession::None | ActiveSession::Classic => None,
        }
    }

    /// Accepts a request only from `Idle`. A `Working` or `Failed` machine
    /// holds either a live job or a decision the user has not made yet, and
    /// overwriting either is the wedged-lane defect.
    fn start_slot_request(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<(), ClientRuntimeError> {
        if !matches!(self.slot_requests, LibrarySlots::Idle) {
            return Err(ClientRuntimeError::LibraryRequestActive);
        }
        self.dispatch_slot_request(request)
    }

    /// The one point at which any Library slot request reaches the store.
    ///
    /// Both archive-related invariants live here rather than on the individual
    /// transitions that reach them, because attaching either to one transition
    /// leaves the others open. Three edges dispatch an `ArchiveSlot`: the first
    /// attempt, a Retry from a retained failure, and — for the compensation
    /// below — every way the request's outcome can afterwards be lost.
    ///
    /// **The residency refusal (§3).** Guarding only the first attempt assumes
    /// residency cannot change between a failure and its Retry. It can, by an
    /// ordinary path: the archive fails while the slot is not resident, the
    /// user closes the Library, opens that slot through Continue, and returns
    /// to a retained `Failed` that [`Self::open_library`] deliberately
    /// preserves. Refusing here gates every edge with one rule.
    ///
    /// **The candidate withdrawal (§10).** Once `start` returns `Ok` the
    /// archive is going to land, and no later observation can take that back:
    /// [`WorkshopStore::abandon`] drops the outcome, not the work, and a
    /// [`StoreJobState::Unknown`] says nothing about whether the write
    /// happened. So the validated Continue candidate has to be withdrawn at the
    /// moment the request reaches the store, not when a success is observed —
    /// otherwise cancelling an in-flight Archive leaves a candidate that
    /// [`Self::continue_selected_workshop`] would install as resident, which is
    /// "no archived slot is opened or selected for Continue" verbatim.
    ///
    /// Withdrawing on a request the store later rejects is deliberate
    /// over-correction. It costs one bootstrap to rebuild a purely in-memory
    /// artifact; the opposite error installs an archived slot whose next save
    /// must fail as archived.
    fn dispatch_slot_request(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<(), ClientRuntimeError> {
        if let WorkshopStoreRequest::ArchiveSlot { slot } = &request
            && self.resident_slot() == Some(*slot)
        {
            self.push_diagnostic(
                ClientDiagnosticCode::ResidentSlot,
                "The resident Workshop's own slot cannot be archived",
            );
            return Err(ClientRuntimeError::ResidentSlot);
        }
        match self.workshop_store.start(request.clone()) {
            Ok(job) => {
                if let WorkshopStoreRequest::ArchiveSlot { slot } = &request {
                    self.withdraw_continue_candidate(*slot);
                }
                self.slot_requests = LibrarySlots::Working { job, request };
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                self.retain_failed_slot_request(ClientDiagnosticCode::Store, message, request);
                Err(ClientRuntimeError::Store(error))
            }
        }
    }

    fn poll_library_slots(&mut self) {
        let LibrarySlots::Working { job, .. } = &self.slot_requests else {
            return;
        };
        let job = *job;
        match self.workshop_store.poll(job) {
            StoreJobState::Pending => {}
            StoreJobState::Unknown => self.fail_slot_request(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop storage forgot the active Library slot request",
            ),
            StoreJobState::Complete(Err(error)) => {
                self.fail_slot_request(ClientDiagnosticCode::Store, error.to_string());
            }
            StoreJobState::Complete(Ok(result)) => self.finish_slot_request(result),
        }
    }

    /// Moves an in-flight request to its retryable failure state.
    ///
    /// Never [`Self::enter_recovery`]: §6 forbids collapsing a Library failure
    /// into the global recovery screen, and every arm above reaches here.
    fn fail_slot_request(&mut self, code: ClientDiagnosticCode, message: impl Into<String>) {
        let LibrarySlots::Working { request, .. } =
            std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle)
        else {
            return;
        };
        self.retain_failed_slot_request(code, message, request);
    }

    fn retain_failed_slot_request(
        &mut self,
        code: ClientDiagnosticCode,
        message: impl Into<String>,
        request: WorkshopStoreRequest,
    ) {
        self.push_diagnostic(code, message);
        self.slot_requests = LibrarySlots::Failed { request, code };
    }

    /// Matches the store's answer against the exact request that asked it.
    ///
    /// A result of the wrong shape, or naming a different slot, is a protocol
    /// failure rather than something to absorb: accepting it would let the
    /// Library report a rename of a row the user never activated.
    fn finish_slot_request(&mut self, result: WorkshopStoreResult) {
        let LibrarySlots::Working { request, .. } =
            std::mem::replace(&mut self.slot_requests, LibrarySlots::Idle)
        else {
            return;
        };
        match (&request, result) {
            (WorkshopStoreRequest::ListSlots, WorkshopStoreResult::Slots(list)) => {
                self.slot_list = Some(list);
                return;
            }
            (
                WorkshopStoreRequest::RenameSlot { slot, .. },
                WorkshopStoreResult::SlotRenamed { slot: answered },
            )
            | (
                WorkshopStoreRequest::UnarchiveSlot { slot },
                WorkshopStoreResult::SlotUnarchived { slot: answered },
            ) if *slot == answered => {}
            (
                WorkshopStoreRequest::ArchiveSlot { slot },
                WorkshopStoreResult::SlotArchived { slot: answered },
            ) if *slot == answered => {
                // Dispatch already withdrew the candidate, which is the edge
                // that covers cancellation and a forgotten job as well as
                // success. This repeat is not redundant and must not be
                // asserted away: `begin_continue_bootstrap` is not gated on
                // this machine, so against an adapter with real asynchrony a
                // bootstrap can list a slot the worker has not archived yet,
                // pass the library client's own archived-slot check, and
                // install a fresh candidate between dispatch and this arm.
                // The call is idempotent.
                self.withdraw_continue_candidate(answered);
            }
            _ => {
                self.retain_failed_slot_request(
                    ClientDiagnosticCode::StoreProtocol,
                    "Workshop storage answered a Library slot request with an unexpected result",
                    request,
                );
                return;
            }
        }
        // The cached list now misstates a name, an archived flag, or the
        // Continue marker. Drop it and re-list rather than presenting a row the
        // user could activate with a generation this mutation invalidated.
        self.slot_list = None;
        let _ = self.dispatch_slot_request(WorkshopStoreRequest::ListSlots);
    }

    /// Leaves the active product surface without allowing a resident Workshop
    /// to bypass its pause/save/Continue-selection durability contract.
    pub fn return_to_main_menu(&mut self) -> Result<(), ClientRuntimeError> {
        if let ActiveSession::Workshop(session) = &mut self.active_session {
            session
                .prepare_for_durable_transition()
                .map_err(ClientRuntimeError::WorkshopAction)?;
        }
        self.screen = ClientScreen::MainMenu;
        self.credits_visible = false;
        Ok(())
    }

    pub fn return_to_active_session(&mut self) {
        self.screen = self.screen_for_active_session();
    }

    pub fn dismiss_recovery(&mut self) {
        self.recovery = None;
        if self.screen == ClientScreen::RecoverableError {
            self.screen = self.screen_for_active_session();
        }
    }

    fn continue_selected_workshop(&mut self) -> Result<(), ClientRuntimeError> {
        if self.resident_workshop_continue_ready() {
            self.screen = ClientScreen::GalaxyWorkshop;
            self.recovery = None;
            return Ok(());
        }
        self.ensure_resident_workshop_replaceable()?;
        let Some(candidate) = self.continue_candidate.take() else {
            self.push_diagnostic(
                ClientDiagnosticCode::ContinueUnavailable,
                "Continue requested without a selected, validated save",
            );
            return Err(ClientRuntimeError::ContinueUnavailable);
        };
        self.active_session = ActiveSession::Workshop(WorkshopSession::from_loaded(
            candidate.history,
            candidate.loaded.slot,
            candidate.loaded.generation,
            candidate.loaded.head_generation,
            candidate.loaded.recovered_from_previous,
        ));
        self.screen = ClientScreen::GalaxyWorkshop;
        self.recovery = None;
        Ok(())
    }

    fn resident_workshop_continue_ready(&self) -> bool {
        matches!(&self.active_session, ActiveSession::Workshop(session) if session.continue_ready())
    }

    /// Whether the resident Workshop holds unsaved work or a persistence
    /// obligation, so no route may replace it. §6's gate for Library Open.
    pub fn resident_workshop_blocks_replacement(&self) -> bool {
        match &self.active_session {
            ActiveSession::Workshop(session) => session.replacement_blocked(),
            ActiveSession::None | ActiveSession::Classic => false,
        }
    }

    fn ensure_resident_workshop_replaceable(&mut self) -> Result<(), ClientRuntimeError> {
        if !self.resident_workshop_blocks_replacement() {
            return Ok(());
        }
        self.push_diagnostic(
            ClientDiagnosticCode::RouteUnavailable,
            "Save the resident Workshop before replacing its active session",
        );
        Err(ClientRuntimeError::RouteUnavailable)
    }

    /// Translates one [`WorkshopLibraryClient`] outcome into screen policy.
    ///
    /// The five continue-bootstrap phases moved to
    /// [`library`] unchanged; what stayed here is exactly the part the client
    /// must not own. Every arm below is the disposition the corresponding
    /// former arm performed inline, which is what makes the extraction
    /// behavior-preserving rather than merely compiling.
    fn poll_continue_bootstrap(&mut self) {
        match self.library.poll(&mut self.workshop_store) {
            LibraryEvent::Pending => {}
            LibraryEvent::NoCandidate => self.screen = self.bootstrap_return,
            // Task 6 review Finding 9. Startup Continue treats a slot archived
            // under it exactly as no Continue, and that is not a softening:
            // `ArchiveSlot` clears the Continue marker when it archives the
            // selected slot, so by the time this arrives the store genuinely
            // has no Continue target. Recovery would be the wrong answer -- the
            // user archived the slot, nothing failed -- and installing the
            // candidate is what addendum §3 forbids. `continue_candidate` is
            // left untouched because this route never assigned it.
            LibraryEvent::ArchivedRow { .. } => self.screen = self.bootstrap_return,
            LibraryEvent::Ready(candidate) => {
                let recovered = candidate.loaded.recovered_from_previous;
                self.continue_candidate = Some(*candidate);
                if recovered {
                    self.enter_recovery(
                        ClientDiagnosticCode::Archive,
                        "The latest save was invalid; a previous valid generation is available",
                    );
                } else {
                    self.screen = self.bootstrap_return;
                }
            }
            LibraryEvent::Failed(diagnostic) => {
                self.enter_recovery(diagnostic.code, diagnostic.message);
            }
            LibraryEvent::ProtocolFailure(message) => self.bootstrap_protocol_failure(message),
            // Startup Continue reads the store's own marker and never carries a
            // separately observed generation, so it cannot request either
            // conflict. Handled rather than asserted away: if one ever arrives
            // here the store answered a question this route did not ask, which
            // is the same class of fault as the arms above.
            LibraryEvent::StaleRow { .. } | LibraryEvent::ContinueConflict { .. } => self
                .bootstrap_protocol_failure(
                    "Workshop Continue bootstrap received a generation conflict it never requested",
                ),
        }
    }

    fn bootstrap_protocol_failure(&mut self, message: &'static str) {
        self.continue_candidate = None;
        self.enter_recovery(ClientDiagnosticCode::StoreProtocol, message);
    }

    fn screen_for_active_session(&self) -> ClientScreen {
        match self.active_session {
            ActiveSession::None => ClientScreen::MainMenu,
            ActiveSession::Classic => ClientScreen::ClassicSector,
            ActiveSession::Workshop(_) => ClientScreen::GalaxyWorkshop,
        }
    }

    fn enter_recovery(&mut self, code: ClientDiagnosticCode, message: impl Into<String>) {
        let diagnostic = ClientDiagnostic {
            code,
            message: message.into(),
        };
        self.recovery = Some(diagnostic.clone());
        self.push_diagnostic(diagnostic.code, diagnostic.message);
        self.screen = ClientScreen::RecoverableError;
    }

    fn push_diagnostic(&mut self, code: ClientDiagnosticCode, message: impl Into<String>) {
        if self.diagnostics.len() == MAX_CLIENT_DIAGNOSTICS {
            self.diagnostics.pop_front();
        }
        self.diagnostics.push_back(ClientDiagnostic {
            code,
            message: message.into(),
        });
    }
}
