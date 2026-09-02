#[cfg(not(target_arch = "wasm32"))]
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(not(target_arch = "wasm32"))]
use super::codec::MAX_JSON_BYTES;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("scenario storage failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("scenario storage slot exceeds the {max_bytes}-byte limit")]
    OversizedSlot { max_bytes: usize },
    #[error("scenario storage failed before atomic persist: {0}")]
    BeforePersist(String),
    #[error("browser scenario storage failed: {0}")]
    Browser(String),
}

pub trait ScenarioStore {
    fn load(&self) -> Result<Option<String>, StoreError>;
    fn save(&mut self, payload: &str) -> Result<(), StoreError>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MemoryScenarioStore {
    slot: Option<String>,
    fail_next_save: Option<String>,
}

impl MemoryScenarioStore {
    pub fn with_slot(payload: impl Into<String>) -> Self {
        Self {
            slot: Some(payload.into()),
            fail_next_save: None,
        }
    }

    pub fn fail_next_save(&mut self, message: impl Into<String>) {
        self.fail_next_save = Some(message.into());
    }
}

impl ScenarioStore for MemoryScenarioStore {
    fn load(&self) -> Result<Option<String>, StoreError> {
        Ok(self.slot.clone())
    }

    fn save(&mut self, payload: &str) -> Result<(), StoreError> {
        if let Some(message) = self.fail_next_save.take() {
            return Err(StoreError::BeforePersist(message));
        }
        self.slot = Some(payload.into());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePlatform {
    MacOs,
    Windows,
    Linux,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Default)]
pub struct NativePathEnvironment {
    pub home: Option<PathBuf>,
    pub appdata: Option<PathBuf>,
    pub xdg_data_home: Option<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_scenario_path(
    platform: NativePlatform,
    environment: &NativePathEnvironment,
) -> Result<PathBuf, StoreError> {
    let missing = || {
        StoreError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "required application-data environment path is unavailable",
        ))
    };
    match platform {
        NativePlatform::MacOs => Ok(environment
            .home
            .as_ref()
            .ok_or_else(missing)?
            .join("Library/Application Support/Intergalactic Warfare/scenario-v1.json")),
        NativePlatform::Windows => Ok(environment
            .appdata
            .as_ref()
            .ok_or_else(missing)?
            .join("Intergalactic Warfare/scenario-v1.json")),
        NativePlatform::Linux => {
            let root = if let Some(xdg) = environment
                .xdg_data_home
                .as_ref()
                .filter(|path| path.is_absolute())
            {
                xdg.clone()
            } else {
                environment
                    .home
                    .as_ref()
                    .filter(|path| path.is_absolute())
                    .ok_or_else(missing)?
                    .join(".local/share")
            };
            Ok(root.join("intergalactic-warfare/scenario-v1.json"))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct NativeScenarioStore {
    path: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeScenarioStore {
    pub fn at_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_current_platform() -> Result<Self, StoreError> {
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
        Ok(Self::at_path(native_scenario_path(platform, &environment)?))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_with_before_persist<F>(
        &self,
        payload: &str,
        before_persist: F,
    ) -> Result<(), StoreError>
    where
        F: FnOnce() -> Result<(), StoreError>,
    {
        let parent = self.path.parent().ok_or_else(|| {
            StoreError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "scenario slot has no parent directory",
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
            .map_err(|error| StoreError::Io(error.error))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ScenarioStore for NativeScenarioStore {
    fn load(&self) -> Result<Option<String>, StoreError> {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(StoreError::Io(error)),
        };
        let mut bytes = Vec::with_capacity(MAX_JSON_BYTES + 1);
        file.take((MAX_JSON_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_JSON_BYTES {
            return Err(StoreError::OversizedSlot {
                max_bytes: MAX_JSON_BYTES,
            });
        }
        let payload = String::from_utf8(bytes).map_err(|error| {
            StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        Ok(Some(payload))
    }

    fn save(&mut self, payload: &str) -> Result<(), StoreError> {
        self.save_with_before_persist(payload, || Ok(()))
    }
}
