#[cfg(not(target_arch = "wasm32"))]
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

use super::MAX_JSON_BYTES;
use crate::scenario::store::load_primary_or_legacy;

pub const PREFERENCES_FILE_NAME: &str = "preferences-v1.json";
pub const LOCAL_STORAGE_KEY: &str = "nyon.preferences.v1";
pub const LEGACY_LOCAL_STORAGE_KEY: &str = "intergalactic-warfare.preferences.v1";

#[derive(Debug, thiserror::Error)]
pub enum PreferencesStoreError {
    #[error("preference storage failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("preference storage slot exceeds the {max_bytes}-byte limit")]
    OversizedSlot { max_bytes: usize },
    #[error("preference storage path is unavailable: {0}")]
    UnavailablePath(String),
    #[error("preference storage failed before atomic persist: {0}")]
    BeforePersist(String),
    #[error("browser preference storage failed: {0}")]
    Browser(String),
}

pub trait PreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError>;
    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryPreferencesStore {
    slot: Option<String>,
    load_failure: Option<String>,
    fail_next_save: Option<String>,
}

impl MemoryPreferencesStore {
    pub fn with_slot(payload: impl Into<String>) -> Self {
        Self {
            slot: Some(payload.into()),
            load_failure: None,
            fail_next_save: None,
        }
    }

    pub fn deny_load(&mut self, message: impl Into<String>) {
        self.load_failure = Some(message.into());
    }

    pub fn fail_next_save(&mut self, message: impl Into<String>) {
        self.fail_next_save = Some(message.into());
    }
}

impl PreferencesStore for MemoryPreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError> {
        if let Some(message) = &self.load_failure {
            return Err(PreferencesStoreError::Browser(message.clone()));
        }
        if self
            .slot
            .as_ref()
            .is_some_and(|payload| payload.len() > MAX_JSON_BYTES)
        {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        Ok(self.slot.clone())
    }

    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError> {
        if payload.len() > MAX_JSON_BYTES {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        if let Some(message) = self.fail_next_save.take() {
            return Err(PreferencesStoreError::BeforePersist(message));
        }
        self.slot = Some(payload.into());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use crate::scenario::store::{NativePathEnvironment, NativePlatform};

#[cfg(not(target_arch = "wasm32"))]
pub fn native_preferences_path(
    platform: NativePlatform,
    environment: &NativePathEnvironment,
) -> Result<PathBuf, PreferencesStoreError> {
    let scenario_path = crate::scenario::store::native_scenario_path(platform, environment)
        .map_err(|error| PreferencesStoreError::UnavailablePath(error.to_string()))?;
    Ok(scenario_path.with_file_name(PREFERENCES_FILE_NAME))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn legacy_native_preferences_path(
    platform: NativePlatform,
    environment: &NativePathEnvironment,
) -> Result<PathBuf, PreferencesStoreError> {
    let scenario_path = crate::scenario::store::legacy_native_scenario_path(platform, environment)
        .map_err(|error| PreferencesStoreError::UnavailablePath(error.to_string()))?;
    Ok(scenario_path.with_file_name(PREFERENCES_FILE_NAME))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct NativePreferencesStore {
    path: PathBuf,
    legacy_path: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativePreferencesStore {
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            legacy_path: None,
        }
    }

    pub fn at_paths(path: impl Into<PathBuf>, legacy_path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            legacy_path: Some(legacy_path.into()),
        }
    }

    pub fn for_current_platform() -> Result<Self, PreferencesStoreError> {
        let environment = NativePathEnvironment {
            home: std::env::var_os("HOME").map(PathBuf::from),
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
        };
        #[cfg(target_os = "macos")]
        let platform = NativePlatform::MacOs;
        #[cfg(target_os = "windows")]
        let platform = NativePlatform::Windows;
        #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
        let platform = NativePlatform::Linux;
        Ok(Self::at_paths(
            native_preferences_path(platform, &environment)?,
            legacy_native_preferences_path(platform, &environment)?,
        ))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn load_path(path: &Path) -> Result<Option<String>, PreferencesStoreError> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(PreferencesStoreError::Io(error)),
        };
        let mut bytes = Vec::with_capacity(MAX_JSON_BYTES + 1);
        file.take((MAX_JSON_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_JSON_BYTES {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        let payload = String::from_utf8(bytes).map_err(|error| {
            PreferencesStoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        Ok(Some(payload))
    }

    pub fn save_with_before_persist<F>(
        &self,
        payload: &str,
        before_persist: F,
    ) -> Result<(), PreferencesStoreError>
    where
        F: FnOnce() -> Result<(), PreferencesStoreError>,
    {
        if payload.len() > MAX_JSON_BYTES {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        let parent = self.path.parent().ok_or_else(|| {
            PreferencesStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "preference slot has no parent directory",
            ))
        })?;
        std::fs::create_dir_all(parent)?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary.write_all(payload.as_bytes())?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;
        before_persist()?;
        temporary
            .persist(&self.path)
            .map_err(|error| PreferencesStoreError::Io(error.error))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl PreferencesStore for NativePreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError> {
        load_primary_or_legacy(
            || Self::load_path(&self.path),
            || match self.legacy_path.as_deref() {
                Some(path) => Self::load_path(path),
                None => Ok(None),
            },
        )
    }

    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError> {
        self.save_with_before_persist(payload, || Ok(()))
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, Default)]
pub struct WebPreferencesStore;

#[cfg(target_arch = "wasm32")]
impl WebPreferencesStore {
    fn storage() -> Result<web_sys::Storage, PreferencesStoreError> {
        let window = web_sys::window().ok_or_else(|| {
            PreferencesStoreError::Browser("browser window is unavailable".into())
        })?;
        window
            .local_storage()
            .map_err(|error| {
                PreferencesStoreError::Browser(format!("localStorage access was denied: {error:?}"))
            })?
            .ok_or_else(|| PreferencesStoreError::Browser("localStorage is unavailable".into()))
    }
}

#[cfg(target_arch = "wasm32")]
impl PreferencesStore for WebPreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError> {
        let storage = Self::storage()?;
        let payload = load_primary_or_legacy(
            || {
                storage.get_item(LOCAL_STORAGE_KEY).map_err(|error| {
                    PreferencesStoreError::Browser(format!("localStorage load failed: {error:?}"))
                })
            },
            || {
                storage.get_item(LEGACY_LOCAL_STORAGE_KEY).map_err(|error| {
                    PreferencesStoreError::Browser(format!(
                        "legacy localStorage load failed: {error:?}"
                    ))
                })
            },
        )?;
        if payload
            .as_ref()
            .is_some_and(|value| value.len() > MAX_JSON_BYTES)
        {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        Ok(payload)
    }

    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError> {
        if payload.len() > MAX_JSON_BYTES {
            return Err(PreferencesStoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        Self::storage()?
            .set_item(LOCAL_STORAGE_KEY, payload)
            .map_err(|error| {
                PreferencesStoreError::Browser(format!("localStorage save failed: {error:?}"))
            })
    }
}
