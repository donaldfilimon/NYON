//! Platform-neutral product shell for Classic Sector and Galaxy Workshop.
//!
//! This module deliberately sits outside [`AppCore`]. The RulesV1 client and
//! the Workshop authority are sibling sessions; neither is adapted into the
//! other's mode or command model.

use std::{collections::VecDeque, time::Duration};

use crate::{
    app::AppCore,
    preferences::store::PreferencesStore,
    scenario::store::ScenarioStore,
    workshop::{
        ArchiveDecodeJob, ArchiveDecodeStatus, CatalogHash, ValidatedCatalogPackV1,
        WorkshopHistory, archive_catalog_hash, decode_catalog_pack, encode_catalog_pack,
        session::{
            WorkshopAction, WorkshopSession, WorkshopSessionError, WorkshopSessionSnapshot,
            WorkshopUpdate,
        },
        store::{
            LoadedSlot, SlotId, StoreJobId, StoreJobState, WorkshopStore, WorkshopStoreError,
            WorkshopStoreRequest, WorkshopStoreResult,
        },
    },
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

struct ContinueCandidate {
    loaded: LoadedSlot,
    history: WorkshopHistory,
}

#[derive(Debug)]
enum ContinueBootstrap {
    Idle,
    Listing(StoreJobId),
    Loading {
        job: StoreJobId,
        selected_slot: SlotId,
    },
    LoadingPrevious {
        job: StoreJobId,
        selected_slot: SlotId,
        original_failure: ClientDiagnostic,
    },
    LoadingCatalog {
        job: StoreJobId,
        selected_slot: SlotId,
        loaded: LoadedSlot,
        expected_hash: CatalogHash,
    },
    Decoding {
        loaded: LoadedSlot,
        decoder: Box<ArchiveDecodeJob>,
    },
}

#[derive(Clone, Debug)]
enum CatalogImport {
    Idle,
    Storing {
        job: StoreJobId,
        catalog: Box<ValidatedCatalogPackV1>,
        start_seed: Option<u64>,
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
    continue_bootstrap: ContinueBootstrap,
    continue_candidate: Option<ContinueCandidate>,
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
            continue_bootstrap: ContinueBootstrap::Idle,
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
        !matches!(&self.continue_bootstrap, ContinueBootstrap::Idle)
    }

    pub const fn catalog_import_active(&self) -> bool {
        !matches!(&self.catalog_import, CatalogImport::Idle)
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
        let job = self
            .workshop_store
            .start(WorkshopStoreRequest::ListSlots)
            .map_err(|error| {
                self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                ClientRuntimeError::Store(error)
            })?;
        self.continue_bootstrap = ContinueBootstrap::Listing(job);
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

    fn begin_catalog_import_inner(
        &mut self,
        bytes: &[u8],
        start_seed: Option<u64>,
    ) -> Result<CatalogHash, ClientRuntimeError> {
        if self.catalog_import_active() {
            return Err(ClientRuntimeError::CatalogImportActive);
        }
        if start_seed.is_some() {
            self.ensure_resident_workshop_replaceable()?;
        }
        let catalog = decode_catalog_pack(bytes).map_err(|_| {
            self.enter_recovery(
                ClientDiagnosticCode::Catalog,
                "The imported Workshop catalog failed validation",
            );
            ClientRuntimeError::InvalidCatalog
        })?;
        let canonical_pack = encode_catalog_pack(&catalog).map_err(|_| {
            self.enter_recovery(
                ClientDiagnosticCode::Catalog,
                "The imported Workshop catalog could not be canonicalized",
            );
            ClientRuntimeError::InvalidCatalog
        })?;
        let expected_hash = catalog.catalog_hash();
        let job = self
            .workshop_store
            .start(WorkshopStoreRequest::PutPack {
                canonical_pack: canonical_pack.into_boxed_slice(),
            })
            .map_err(|error| {
                self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                ClientRuntimeError::Store(error)
            })?;
        self.catalog_import = CatalogImport::Storing {
            job,
            catalog: Box::new(catalog),
            start_seed,
        };
        Ok(expected_hash)
    }

    fn poll_catalog_import(&mut self) {
        let state = std::mem::replace(&mut self.catalog_import, CatalogImport::Idle);
        let CatalogImport::Storing {
            job,
            catalog,
            start_seed,
        } = state
        else {
            return;
        };
        match self.workshop_store.poll(job) {
            StoreJobState::Pending => {
                self.catalog_import = CatalogImport::Storing {
                    job,
                    catalog,
                    start_seed,
                };
            }
            StoreJobState::Unknown => self.enter_recovery(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop storage forgot the active catalog import job",
            ),
            StoreJobState::Complete(Err(error)) => {
                self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
            }
            StoreJobState::Complete(Ok(WorkshopStoreResult::PackStored { hash })) => {
                if hash != catalog.catalog_hash() {
                    self.enter_recovery(
                        ClientDiagnosticCode::StoreProtocol,
                        "Workshop storage persisted a catalog under the wrong hash",
                    );
                    return;
                }
                self.imported_catalog = Some((*catalog).clone());
                if let Some(seed) = start_seed
                    && self.install_new_workshop(*catalog, seed).is_err()
                {
                    // The resident Workshop became unreplaceable while the pack
                    // was persisting. The catalog is kept, so surface the
                    // rejection on the same recovery path every other import
                    // completion failure uses rather than dropping it.
                    self.enter_recovery(
                        ClientDiagnosticCode::RouteUnavailable,
                        "The imported Workshop catalog was saved, but the resident Workshop \
                         changed while it was saving. Save the resident Workshop, then start a \
                         new Workshop from the imported catalog.",
                    );
                }
            }
            StoreJobState::Complete(Ok(_)) => self.enter_recovery(
                ClientDiagnosticCode::StoreProtocol,
                "Workshop catalog import returned an unexpected result",
            ),
        }
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

    fn poll_continue_bootstrap(&mut self) {
        let state = std::mem::replace(&mut self.continue_bootstrap, ContinueBootstrap::Idle);
        match state {
            ContinueBootstrap::Idle => {}
            ContinueBootstrap::Listing(job) => match self.workshop_store.poll(job) {
                StoreJobState::Pending => {
                    self.continue_bootstrap = ContinueBootstrap::Listing(job);
                }
                StoreJobState::Unknown => self
                    .bootstrap_protocol_failure("Workshop storage forgot the active slot-list job"),
                StoreJobState::Complete(Err(error)) => {
                    self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                }
                StoreJobState::Complete(Ok(WorkshopStoreResult::Slots(list))) => {
                    let Some(selected_slot) = list.selected_continue else {
                        self.continue_bootstrap = ContinueBootstrap::Idle;
                        self.screen = self.bootstrap_return;
                        return;
                    };
                    let selected_is_valid = list.slots.iter().any(|summary| {
                        summary.id == selected_slot
                            && summary.selected_for_continue
                            && !summary.archived
                    });
                    if !selected_is_valid {
                        self.bootstrap_protocol_failure(
                            "The explicitly selected Continue slot is missing or archived",
                        );
                        return;
                    }
                    match self.workshop_store.start(WorkshopStoreRequest::LoadSlot {
                        slot: selected_slot,
                    }) {
                        Ok(job) => {
                            self.continue_bootstrap =
                                ContinueBootstrap::Loading { job, selected_slot };
                        }
                        Err(error) => {
                            self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                        }
                    }
                }
                StoreJobState::Complete(Ok(_)) => self.bootstrap_protocol_failure(
                    "Workshop slot-list job returned an unexpected result",
                ),
            },
            ContinueBootstrap::Loading { job, selected_slot } => {
                match self.workshop_store.poll(job) {
                    StoreJobState::Pending => {
                        self.continue_bootstrap = ContinueBootstrap::Loading { job, selected_slot };
                    }
                    StoreJobState::Unknown => self.bootstrap_protocol_failure(
                        "Workshop storage forgot the active Continue load job",
                    ),
                    StoreJobState::Complete(Err(error)) => {
                        self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
                    }
                    StoreJobState::Complete(Ok(WorkshopStoreResult::SlotLoaded(loaded))) => {
                        if loaded.slot != selected_slot {
                            self.bootstrap_protocol_failure(
                                "Workshop storage loaded a slot other than the selected Continue slot",
                            );
                            return;
                        }
                        self.prepare_loaded_continue(selected_slot, loaded);
                    }
                    StoreJobState::Complete(Ok(_)) => self.bootstrap_protocol_failure(
                        "Workshop Continue load returned an unexpected result",
                    ),
                }
            }
            ContinueBootstrap::LoadingPrevious {
                job,
                selected_slot,
                original_failure,
            } => match self.workshop_store.poll(job) {
                StoreJobState::Pending => {
                    self.continue_bootstrap = ContinueBootstrap::LoadingPrevious {
                        job,
                        selected_slot,
                        original_failure,
                    };
                }
                StoreJobState::Unknown => self.bootstrap_protocol_failure(
                    "Workshop storage forgot the previous-generation load job",
                ),
                StoreJobState::Complete(Err(_)) => {
                    self.enter_recovery(original_failure.code, original_failure.message);
                }
                StoreJobState::Complete(Ok(WorkshopStoreResult::SlotLoaded(loaded))) => {
                    if loaded.slot != selected_slot || !loaded.recovered_from_previous {
                        self.bootstrap_protocol_failure(
                            "Workshop storage returned an invalid previous-generation load",
                        );
                        return;
                    }
                    self.prepare_loaded_continue(selected_slot, loaded);
                }
                StoreJobState::Complete(Ok(_)) => self.bootstrap_protocol_failure(
                    "Workshop previous-generation load returned an unexpected result",
                ),
            },
            ContinueBootstrap::LoadingCatalog {
                job,
                selected_slot,
                loaded,
                expected_hash,
            } => match self.workshop_store.poll(job) {
                StoreJobState::Pending => {
                    self.continue_bootstrap = ContinueBootstrap::LoadingCatalog {
                        job,
                        selected_slot,
                        loaded,
                        expected_hash,
                    };
                }
                StoreJobState::Unknown => self.bootstrap_protocol_failure(
                    "Workshop storage forgot the active catalog load job",
                ),
                StoreJobState::Complete(Err(error)) => {
                    self.recover_previous_or_enter(
                        loaded,
                        ClientDiagnosticCode::Store,
                        error.to_string(),
                    );
                }
                StoreJobState::Complete(Ok(WorkshopStoreResult::PackLoaded {
                    hash,
                    canonical_pack,
                })) => {
                    if loaded.slot != selected_slot || hash != expected_hash {
                        self.bootstrap_protocol_failure(
                            "Workshop storage returned a catalog other than the archive's exact catalog",
                        );
                        return;
                    }
                    let catalog = match decode_catalog_pack(&canonical_pack) {
                        Ok(catalog) => catalog,
                        Err(_) => {
                            self.recover_previous_or_enter(
                                loaded,
                                ClientDiagnosticCode::Catalog,
                                "The saved Workshop catalog failed validation",
                            );
                            return;
                        }
                    };
                    let canonical = match encode_catalog_pack(&catalog) {
                        Ok(canonical) => canonical,
                        Err(_) => {
                            self.recover_previous_or_enter(
                                loaded,
                                ClientDiagnosticCode::Catalog,
                                "The saved Workshop catalog could not be canonicalized",
                            );
                            return;
                        }
                    };
                    if catalog.catalog_hash() != expected_hash
                        || canonical.as_slice() != canonical_pack.as_ref()
                    {
                        self.recover_previous_or_enter(
                            loaded,
                            ClientDiagnosticCode::Catalog,
                            "The saved Workshop catalog was noncanonical or had the wrong hash",
                        );
                        return;
                    }
                    self.begin_continue_replay(loaded, catalog);
                }
                StoreJobState::Complete(Ok(_)) => self.bootstrap_protocol_failure(
                    "Workshop catalog load returned an unexpected result",
                ),
            },
            ContinueBootstrap::Decoding {
                loaded,
                mut decoder,
            } => match decoder.poll(ARCHIVE_REPLAY_UNITS_PER_UPDATE) {
                Ok(ArchiveDecodeStatus::Pending) => {
                    self.continue_bootstrap = ContinueBootstrap::Decoding { loaded, decoder };
                }
                Ok(ArchiveDecodeStatus::Complete) => match (*decoder).finish() {
                    Ok(history) => self.finish_continue_replay(loaded, history),
                    Err(error) => {
                        self.recover_previous_or_enter(
                            loaded,
                            ClientDiagnosticCode::Archive,
                            error.to_string(),
                        );
                    }
                },
                Err(error) => {
                    self.recover_previous_or_enter(
                        loaded,
                        ClientDiagnosticCode::Archive,
                        error.to_string(),
                    );
                }
            },
        }
    }

    fn prepare_loaded_continue(&mut self, selected_slot: SlotId, loaded: LoadedSlot) {
        if loaded.recovered_from_previous != (loaded.generation != loaded.head_generation) {
            self.bootstrap_protocol_failure(
                "Workshop storage returned inconsistent loaded and head generations",
            );
            return;
        }
        let catalog = match decode_catalog_pack(CORE_PACK_V1) {
            Ok(catalog) => catalog,
            Err(_) => {
                self.enter_recovery(
                    ClientDiagnosticCode::Catalog,
                    "The built-in Workshop catalog failed validation",
                );
                return;
            }
        };
        let expected_hash = match archive_catalog_hash(&loaded.archive) {
            Ok(hash) => hash,
            Err(error) => {
                self.recover_previous_or_enter(
                    loaded,
                    ClientDiagnosticCode::Archive,
                    error.to_string(),
                );
                return;
            }
        };
        if catalog.catalog_hash() == expected_hash {
            self.begin_continue_replay(loaded, catalog);
            return;
        }
        match self.workshop_store.start(WorkshopStoreRequest::GetPack {
            hash: expected_hash,
        }) {
            Ok(job) => {
                self.continue_bootstrap = ContinueBootstrap::LoadingCatalog {
                    job,
                    selected_slot,
                    loaded,
                    expected_hash,
                };
            }
            Err(error) => {
                self.recover_previous_or_enter(
                    loaded,
                    ClientDiagnosticCode::Store,
                    error.to_string(),
                );
            }
        }
    }

    fn recover_previous_or_enter(
        &mut self,
        loaded: LoadedSlot,
        code: ClientDiagnosticCode,
        message: impl Into<String>,
    ) {
        let message = message.into();
        if loaded.recovered_from_previous {
            self.enter_recovery(code, message);
            return;
        }
        let slot = loaded.slot;
        match self
            .workshop_store
            .start(WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation: loaded.head_generation,
            }) {
            Ok(job) => {
                self.continue_bootstrap = ContinueBootstrap::LoadingPrevious {
                    job,
                    selected_slot: slot,
                    original_failure: ClientDiagnostic { code, message },
                };
            }
            Err(error) => {
                self.enter_recovery(ClientDiagnosticCode::Store, error.to_string());
            }
        }
    }

    fn begin_continue_replay(&mut self, loaded: LoadedSlot, catalog: ValidatedCatalogPackV1) {
        match ArchiveDecodeJob::new(&catalog, &loaded.archive) {
            Ok(decoder) => {
                self.continue_bootstrap = ContinueBootstrap::Decoding {
                    loaded,
                    decoder: Box::new(decoder),
                };
            }
            Err(error) => {
                self.recover_previous_or_enter(
                    loaded,
                    ClientDiagnosticCode::Archive,
                    error.to_string(),
                );
            }
        }
    }

    fn finish_continue_replay(&mut self, loaded: LoadedSlot, history: WorkshopHistory) {
        let recovered = loaded.recovered_from_previous;
        self.continue_candidate = Some(ContinueCandidate { loaded, history });
        if recovered {
            self.enter_recovery(
                ClientDiagnosticCode::Archive,
                "The latest save was invalid; a previous valid generation is available",
            );
        } else {
            self.screen = self.bootstrap_return;
        }
    }

    fn bootstrap_protocol_failure(&mut self, message: &'static str) {
        self.continue_bootstrap = ContinueBootstrap::Idle;
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
