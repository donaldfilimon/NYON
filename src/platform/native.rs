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
use std::{ffi::OsString, path::PathBuf};
use winit::event_loop::{ControlFlow, EventLoop};

#[derive(Debug, thiserror::Error)]
pub enum NativeStartupError {
    #[error("NYON_PROFILE_DIR must be a nonempty absolute directory path")]
    InvalidProfileDirectory,
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

/// Resolve once before any store can consult the default profile.
fn resolve_profile_root(value: Option<OsString>) -> Result<Option<PathBuf>, NativeStartupError> {
    value
        .map(|value| {
            let root = PathBuf::from(value);
            if root.as_os_str().is_empty() || !root.is_absolute() {
                Err(NativeStartupError::InvalidProfileDirectory)
            } else {
                Ok(root)
            }
        })
        .transpose()
}

fn scenario_store(
    root: Option<&std::path::Path>,
) -> Result<NativeScenarioStore, NativeStartupError> {
    match root {
        Some(root) => Ok(NativeScenarioStore::at_path(root.join("scenario-v1.json"))),
        None => Ok(NativeScenarioStore::for_current_platform()?),
    }
}

fn workshop_store(
    root: Option<&std::path::Path>,
) -> Result<NativeWorkshopStore, WorkshopStoreError> {
    match root {
        Some(root) => NativeWorkshopStore::at_root(root.join("workshop-v1")),
        None => NativeWorkshopStore::for_current_platform(),
    }
}

fn living_store(root: &std::path::Path) -> crate::living_store::native::NativeLivingStoreV2 {
    crate::living_store::native::NativeLivingStoreV2::new(root)
}

fn library_coordinator(
    root: &std::path::Path,
) -> crate::library_selection::native::NativeLibraryCoordinator {
    crate::library_selection::native::NativeLibraryCoordinator::new(root)
}

fn audio_path(root: &std::path::Path) -> PathBuf {
    root.join("living-audio-v1.json")
}

pub fn run() -> Result<(), NativeStartupError> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init();
    let profile_root = resolve_profile_root(std::env::var_os("NYON_PROFILE_DIR"))?;
    let store = scenario_store(profile_root.as_deref())?;
    let preference_store = RuntimePreferencesStore::new(profile_root.as_deref());
    let workshop_store = match workshop_store(profile_root.as_deref()) {
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
    if let Ok(root) = profile_root.map(Ok).unwrap_or_else(living_application_root) {
        app.runtime_mut().install_living_storage(
            Box::new(living_store(&root)),
            Box::new(library_coordinator(&root)),
        );
        app.install_living_audio_path(audio_path(&root));
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
    fn new(root: Option<&std::path::Path>) -> Self {
        Self {
            store: match root {
                Some(root) => Ok(NativePreferencesStore::at_path(
                    root.join(crate::preferences::store::PREFERENCES_FILE_NAME),
                )),
                None => NativePreferencesStore::for_current_platform()
                    .map_err(|error| error.to_string()),
            },
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

#[cfg(test)]
mod native_profile_tests {
    use super::*;
    use crate::scenario::store::ScenarioStore;

    #[test]
    fn profile_parse_is_explicit_and_absolute() {
        assert!(resolve_profile_root(None).unwrap().is_none());
        for value in ["", "relative", "./profile"] {
            assert!(matches!(
                resolve_profile_root(Some(value.into())),
                Err(NativeStartupError::InvalidProfileDirectory)
            ));
        }
        let root = std::env::temp_dir().join("nyon-profile");
        assert_eq!(
            resolve_profile_root(Some(root.clone().into_os_string())).unwrap(),
            Some(root)
        );
    }

    #[cfg(unix)]
    #[test]
    fn profile_accepts_absolute_non_utf8_paths() {
        use std::os::unix::ffi::OsStringExt;
        let value = OsString::from_vec(b"/tmp/nyon-profile-\xff".to_vec());
        assert_eq!(
            resolve_profile_root(Some(value.clone())).unwrap(),
            Some(PathBuf::from(value))
        );
    }

    #[test]
    fn profile_scenario_preferences_and_audio_save_and_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let root = resolve_profile_root(Some(directory.path().as_os_str().to_owned()))
            .unwrap()
            .unwrap();
        let mut scenario = scenario_store(Some(&root)).unwrap();
        assert_eq!(scenario.path(), root.join("scenario-v1.json"));
        assert_eq!(scenario.load().unwrap(), None);
        scenario.save("profile scenario").unwrap();
        assert_eq!(
            scenario_store(Some(&root))
                .unwrap()
                .load()
                .unwrap()
                .as_deref(),
            Some("profile scenario")
        );
        let mut preferences = RuntimePreferencesStore::new(Some(&root));
        assert_eq!(
            preferences.store.as_ref().unwrap().path(),
            root.join("preferences-v1.json")
        );
        assert_eq!(preferences.load().unwrap(), None);
        preferences.save("profile preferences").unwrap();
        assert_eq!(
            RuntimePreferencesStore::new(Some(&root))
                .load()
                .unwrap()
                .as_deref(),
            Some("profile preferences")
        );
        let mut audio = crate::living_audio::LivingAudio::default();
        audio.load_native(audio_path(&root));
        audio.settings.master = 25;
        audio.persist().unwrap();
        let mut reopened = crate::living_audio::LivingAudio::default();
        reopened.load_native(audio_path(&root));
        assert_eq!(reopened.settings.master, 25);
        assert!(audio_path(&root).is_file());
    }

    #[test]
    fn profile_living_and_library_save_and_reopen() {
        use crate::app::client_runtime::living::{LivingClient, LivingClientEvent};
        fn terminal(client: &mut LivingClient) -> LivingClientEvent {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            loop {
                let event = client.poll();
                if !matches!(event, LivingClientEvent::Pending) {
                    return event;
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let make_client = || {
            LivingClient::new(
                Box::new(living_store(root)),
                Box::new(library_coordinator(root)),
            )
        };
        let mut client = make_client();
        client.read_selection().unwrap();
        assert!(matches!(
            terminal(&mut client),
            LivingClientEvent::AbsentSelection
        ));
        let authority = nyon_workshop_core::living::builtin_first_expansion_v2().unwrap();
        let expected = authority.bookmark().unwrap();
        client.save(&authority, false).unwrap();
        assert!(
            matches!(terminal(&mut client), LivingClientEvent::Saved { bookmark, .. } if bookmark == expected)
        );
        drop(client);
        assert!(root.join("living-v2").is_dir());
        assert!(root.join("library-selection-v1.head").is_file());
        let mut reopened = make_client();
        reopened.read_selection().unwrap();
        assert!(
            matches!(terminal(&mut reopened), LivingClientEvent::Ready(authority) if authority.bookmark().unwrap() == expected)
        );
    }

    #[test]
    fn profile_workshop_save_and_reopen() {
        use crate::workshop::store::{SlotName, WorkshopStoreResult};
        fn complete(
            store: &mut NativeWorkshopStore,
            request: WorkshopStoreRequest,
        ) -> WorkshopStoreResult {
            let job = store.start(request).unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                if let StoreJobState::Complete(result) = store.poll(job) {
                    return result.unwrap();
                }
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let mut store = workshop_store(Some(root)).unwrap();
        assert_eq!(store.root(), root.join("workshop-v1"));
        let slot = match complete(
            &mut store,
            WorkshopStoreRequest::CreateSlot {
                name: SlotName::new("profile").unwrap(),
                archive: br#"{"format":"workshop-test","version":1}"#
                    .to_vec()
                    .into_boxed_slice(),
            },
        ) {
            WorkshopStoreResult::SlotCreated { slot, .. } => slot,
            other => panic!("unexpected result {other:?}"),
        };
        drop(store);
        let mut reopened = workshop_store(Some(root)).unwrap();
        assert!(matches!(
            complete(&mut reopened, WorkshopStoreRequest::LoadSlot { slot }),
            WorkshopStoreResult::SlotLoaded(loaded) if loaded.archive.as_ref() == br#"{"format":"workshop-test","version":1}"#
        ));
    }
}
