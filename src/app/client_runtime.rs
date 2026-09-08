//! Platform-neutral product shell for Classic Sector and Galaxy Workshop.
//!
//! This module deliberately sits outside [`AppCore`]. The RulesV1 client and
//! the Workshop authority are sibling sessions; neither is adapted into the
//! other's mode or command model.

use std::{collections::VecDeque, time::Duration};

pub mod library;

use crate::{
    app::AppCore,
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
            StoreJobId, StoreJobState, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
            WorkshopStoreResult,
        },
    },
};

use library::{
    LibraryBeginError, LibraryCandidate, LibraryEvent, LibraryOpen, WorkshopLibraryClient,
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

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClientRuntimeError {
    #[error("Continue bootstrap is already active")]
    BootstrapActive,
    #[error("a Workshop catalog import is already active")]
    CatalogImportActive,
    #[error("Continue is unavailable until its selected save validates")]
    ContinueUnavailable,
    #[error("Galaxy Workshop is not the active session")]
    WorkshopInactive,
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
    library: WorkshopLibraryClient,
    continue_candidate: Option<LibraryCandidate>,
    catalog_import: CatalogImport,
    imported_catalog: Option<ValidatedCatalogPackV1>,
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
            library: WorkshopLibraryClient::default(),
            continue_candidate: None,
            catalog_import: CatalogImport::Idle,
            imported_catalog: None,
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

    fn resident_workshop_blocks_replacement(&self) -> bool {
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
