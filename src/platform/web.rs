use wasm_bindgen::prelude::wasm_bindgen;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    platform::web::EventLoopExtWebSys,
};

use crate::{
    app::{App, AppCore, AppEvent},
    preferences::store::WebPreferencesStore,
    scenario::{
        ScenarioDraft, ValidationReport,
        codec::MAX_JSON_BYTES,
        store::{ScenarioStore, StoreError, load_primary_or_legacy},
    },
};

pub const LOCAL_STORAGE_KEY: &str = "nyon.scenario.v1";
pub const LEGACY_LOCAL_STORAGE_KEY: &str = "intergalactic-warfare.scenario.v1";

#[derive(Clone, Copy, Debug, Default)]
pub struct WebScenarioStore;

impl WebScenarioStore {
    fn storage() -> Result<web_sys::Storage, StoreError> {
        let window = web_sys::window()
            .ok_or_else(|| StoreError::Browser("browser window is unavailable".into()))?;
        window
            .local_storage()
            .map_err(|error| {
                StoreError::Browser(format!("localStorage access was denied: {error:?}"))
            })?
            .ok_or_else(|| StoreError::Browser("localStorage is unavailable".into()))
    }
}

impl ScenarioStore for WebScenarioStore {
    fn load(&self) -> Result<Option<String>, StoreError> {
        let storage = Self::storage()?;
        let payload = load_primary_or_legacy(
            || {
                storage.get_item(LOCAL_STORAGE_KEY).map_err(|error| {
                    StoreError::Browser(format!("localStorage load failed: {error:?}"))
                })
            },
            || {
                storage.get_item(LEGACY_LOCAL_STORAGE_KEY).map_err(|error| {
                    StoreError::Browser(format!("legacy localStorage load failed: {error:?}"))
                })
            },
        )?;
        if payload
            .as_ref()
            .is_some_and(|value| value.len() > MAX_JSON_BYTES)
        {
            return Err(StoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        Ok(payload)
    }

    fn save(&mut self, payload: &str) -> Result<(), StoreError> {
        if payload.len() > MAX_JSON_BYTES {
            return Err(StoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        Self::storage()?
            .set_item(LOCAL_STORAGE_KEY, payload)
            .map_err(|error| StoreError::Browser(format!("localStorage save failed: {error:?}")))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebStartupError {
    #[error(transparent)]
    Scenario(#[from] ValidationReport),
    #[error("browser event loop failed: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    run().map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}

pub fn run() -> Result<(), WebStartupError> {
    let scenario = ScenarioDraft::factory_default().validated()?;
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let event_proxy = event_loop.create_proxy();
    let app = App::new(
        AppCore::new_with_preferences(scenario, WebScenarioStore, WebPreferencesStore),
        event_proxy,
    );
    event_loop.spawn_app(app);
    Ok(())
}
