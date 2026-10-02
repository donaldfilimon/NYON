pub use crate::scenario::store::{
    NativePathEnvironment, NativePlatform, NativeScenarioStore, native_scenario_path,
};

use crate::{
    app::{App, AppCore, AppEvent},
    preferences::store::{NativePreferencesStore, PreferencesStore, PreferencesStoreError},
    scenario::{ScenarioDraft, ValidationReport},
    workshop::store::{
        NativeWorkshopStore, StoreJobId, StoreJobState, WorkshopStore, WorkshopStoreError,
        WorkshopStoreRequest,
    },
};
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Debug, thiserror::Error)]
pub enum NativeStartupError {
    #[error(transparent)]
    Store(#[from] crate::scenario::store::StoreError),
    #[error(transparent)]
    Scenario(#[from] ValidationReport),
    #[error(transparent)]
    WorkshopStore(#[from] WorkshopStoreError),
    #[error("native event loop failed: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("application startup failed: {0}")]
    Application(String),
}

pub fn run() -> Result<(), NativeStartupError> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init();
    let store = NativeScenarioStore::for_current_platform()?;
    let preference_store = RuntimePreferencesStore::new();
    let workshop_store = match NativeWorkshopStore::for_current_platform() {
        Ok(store) => RuntimeWorkshopStore::Native(store),
        Err(error) => {
            log::error!("Workshop storage is unavailable; Classic remains usable: {error}");
            RuntimeWorkshopStore::Unavailable(error)
        }
    };
    let scenario = ScenarioDraft::factory_default().validated()?;
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let event_proxy = event_loop.create_proxy();
    let mut app = App::with_workshop_store(
        AppCore::new_with_preferences(scenario, store, preference_store),
        workshop_store,
        event_proxy,
    );
    if let Ok(root) = living_application_root() {
        app.runtime_mut().install_living_storage(
            Box::new(crate::living_store::native::NativeLivingStoreV2::new(
                root.clone(),
            )),
            Box::new(crate::library_selection::native::NativeLibraryCoordinator::new(root.clone())),
        );
        app.install_living_audio_path(root.join("living-audio-v1.json"));
        app.runtime_mut().install_living_transfer(Box::new(
            crate::living_transfer::native::NativeLivingTransfer::default(),
        ));
    }
    event_loop.run_app(&mut app)?;
    if let Some(message) = app.take_fatal_message() {
        return Err(NativeStartupError::Application(message));
    }
    Ok(())
}

fn living_application_root() -> Result<std::path::PathBuf, WorkshopStoreError> {
    use crate::workshop::store::{
        NativeWorkshopPlatform, WorkshopPathEnvironment, native_workshop_root,
    };
    #[cfg(target_os = "macos")]
    let platform = NativeWorkshopPlatform::MacOs;
    #[cfg(target_os = "windows")]
    let platform = NativeWorkshopPlatform::Windows;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let platform = NativeWorkshopPlatform::Linux;
    let environment = WorkshopPathEnvironment {
        home: std::env::var_os("HOME").map(std::path::PathBuf::from),
        appdata: std::env::var_os("APPDATA").map(std::path::PathBuf::from),
        xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(std::path::PathBuf::from),
    };
    let workshop = native_workshop_root(platform, &environment)?;
    Ok(workshop
        .parent()
        .expect("versioned store root has parent")
        .to_path_buf())
}

enum RuntimeWorkshopStore {
    Native(NativeWorkshopStore),
    Unavailable(WorkshopStoreError),
}

impl WorkshopStore for RuntimeWorkshopStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        match self {
            Self::Native(store) => store.start(request),
            Self::Unavailable(error) => Err(error.clone()),
        }
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        match self {
            Self::Native(store) => store.abandon(job),
            Self::Unavailable(_) => false,
        }
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        match self {
            Self::Native(store) => store.poll(job),
            Self::Unavailable(_) => StoreJobState::Unknown,
        }
    }
}

struct RuntimePreferencesStore {
    store: Result<NativePreferencesStore, String>,
}

impl RuntimePreferencesStore {
    fn new() -> Self {
        Self {
            store: NativePreferencesStore::for_current_platform()
                .map_err(|error| error.to_string()),
        }
    }
}

impl PreferencesStore for RuntimePreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError> {
        match &self.store {
            Ok(store) => store.load(),
            Err(message) => Err(PreferencesStoreError::UnavailablePath(message.clone())),
        }
    }

    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError> {
        match &mut self.store {
            Ok(store) => store.save(payload),
            Err(message) => Err(PreferencesStoreError::UnavailablePath(message.clone())),
        }
    }
}
