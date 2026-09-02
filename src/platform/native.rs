pub use crate::scenario::store::{
    NativePathEnvironment, NativePlatform, NativeScenarioStore, native_scenario_path,
};

use crate::{
    app::{App, AppCore, AppEvent},
    preferences::store::{NativePreferencesStore, PreferencesStore, PreferencesStoreError},
    scenario::{ScenarioDraft, ValidationReport},
};
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Debug, thiserror::Error)]
pub enum NativeStartupError {
    #[error(transparent)]
    Store(#[from] crate::scenario::store::StoreError),
    #[error(transparent)]
    Scenario(#[from] ValidationReport),
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
    let scenario = ScenarioDraft::factory_default().validated()?;
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let event_proxy = event_loop.create_proxy();
    let mut app = App::new(
        AppCore::new_with_preferences(scenario, store, preference_store),
        event_proxy,
    );
    event_loop.run_app(&mut app)?;
    if let Some(message) = app.take_fatal_message() {
        return Err(NativeStartupError::Application(message));
    }
    Ok(())
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
