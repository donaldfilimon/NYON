//! Versioned, scenario-independent local user preferences.

pub mod store;

use serde::{Deserialize, Serialize};

use crate::presentation::{GraphicsQuality, MotionPreference, PresentationPreferences};

use self::store::{PreferencesStore, PreferencesStoreError};

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_JSON_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UiScale {
    Percent85,
    #[default]
    Percent100,
    Percent115,
    Percent130,
}

impl UiScale {
    pub const fn percent(self) -> u16 {
        match self {
            Self::Percent85 => 85,
            Self::Percent100 => 100,
            Self::Percent115 => 115,
            Self::Percent130 => 130,
        }
    }

    pub const fn factor(self) -> f32 {
        self.percent() as f32 / 100.0
    }
}

impl TryFrom<u16> for UiScale {
    type Error = PreferencesCodecError;

    fn try_from(percent: u16) -> Result<Self, Self::Error> {
        match percent {
            85 => Ok(Self::Percent85),
            100 => Ok(Self::Percent100),
            115 => Ok(Self::Percent115),
            130 => Ok(Self::Percent130),
            _ => Err(PreferencesCodecError::UiScale(percent)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserPreferencesV1 {
    pub ui_scale: UiScale,
    pub motion: MotionPreference,
    pub high_contrast: bool,
    pub graphics_quality: GraphicsQuality,
    pub onboarding_completed: bool,
}

impl UserPreferencesV1 {
    pub const fn presentation(self) -> PresentationPreferences {
        PresentationPreferences {
            graphics_quality: self.graphics_quality,
            motion: self.motion,
            high_contrast: self.high_contrast,
        }
    }
}

impl Default for UserPreferencesV1 {
    fn default() -> Self {
        Self {
            ui_scale: UiScale::Percent100,
            motion: MotionPreference::Full,
            high_contrast: false,
            graphics_quality: GraphicsQuality::Auto,
            onboarding_completed: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PreferencesCodecError {
    #[error("preferences JSON exceeds the 16384-byte limit")]
    TooLarge,
    #[error("invalid preferences JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported preferences format version")]
    Version,
    #[error("unsupported UI scale percentage {0}")]
    UiScale(u16),
}

#[derive(Debug, thiserror::Error)]
pub enum PreferencesFailure {
    #[error(transparent)]
    Codec(#[from] PreferencesCodecError),
    #[error(transparent)]
    Store(#[from] PreferencesStoreError),
}

#[derive(Debug)]
pub struct PreferencesOutcome {
    pub preferences: UserPreferencesV1,
    pub recoverable_failure: Option<PreferencesFailure>,
}

impl PreferencesOutcome {
    pub fn recoverable_message(&self) -> Option<String> {
        self.recoverable_failure.as_ref().map(ToString::to_string)
    }

    fn success(preferences: UserPreferencesV1) -> Self {
        Self {
            preferences,
            recoverable_failure: None,
        }
    }

    fn fallback(error: impl Into<PreferencesFailure>) -> Self {
        Self {
            preferences: UserPreferencesV1::default(),
            recoverable_failure: Some(error.into()),
        }
    }
}

pub fn encode(preferences: &UserPreferencesV1) -> Result<String, PreferencesCodecError> {
    let payload = serde_json::to_string(&WirePreferences::from_preferences(*preferences))?;
    if payload.len() > MAX_JSON_BYTES {
        return Err(PreferencesCodecError::TooLarge);
    }
    Ok(payload)
}

pub fn decode(json: &str) -> Result<UserPreferencesV1, PreferencesCodecError> {
    if json.len() > MAX_JSON_BYTES {
        return Err(PreferencesCodecError::TooLarge);
    }
    let wire: WirePreferences = serde_json::from_str(json)?;
    wire.into_preferences()
}

pub fn load_or_default(store: &impl PreferencesStore) -> PreferencesOutcome {
    match store.load() {
        Ok(None) => PreferencesOutcome::success(UserPreferencesV1::default()),
        Ok(Some(payload)) => match decode(&payload) {
            Ok(preferences) => PreferencesOutcome::success(preferences),
            Err(error) => PreferencesOutcome::fallback(error),
        },
        Err(error) => PreferencesOutcome::fallback(error),
    }
}

pub fn save_or_default(
    store: &mut impl PreferencesStore,
    preferences: &UserPreferencesV1,
) -> PreferencesOutcome {
    let payload = match encode(preferences) {
        Ok(payload) => payload,
        Err(error) => return PreferencesOutcome::fallback(error),
    };
    match store.save(&payload) {
        Ok(()) => PreferencesOutcome::success(*preferences),
        Err(error) => PreferencesOutcome::fallback(error),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WirePreferences {
    format_version: u32,
    ui_scale_percent: u16,
    motion: WireMotion,
    high_contrast: bool,
    graphics_quality: WireGraphicsQuality,
    onboarding_completed: bool,
}

impl WirePreferences {
    const fn from_preferences(preferences: UserPreferencesV1) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            ui_scale_percent: preferences.ui_scale.percent(),
            motion: WireMotion::from_preference(preferences.motion),
            high_contrast: preferences.high_contrast,
            graphics_quality: WireGraphicsQuality::from_quality(preferences.graphics_quality),
            onboarding_completed: preferences.onboarding_completed,
        }
    }

    fn into_preferences(self) -> Result<UserPreferencesV1, PreferencesCodecError> {
        if self.format_version != FORMAT_VERSION {
            return Err(PreferencesCodecError::Version);
        }
        Ok(UserPreferencesV1 {
            ui_scale: self.ui_scale_percent.try_into()?,
            motion: self.motion.into_preference(),
            high_contrast: self.high_contrast,
            graphics_quality: self.graphics_quality.into_quality(),
            onboarding_completed: self.onboarding_completed,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WireMotion {
    Full,
    Reduced,
}

impl WireMotion {
    const fn from_preference(preference: MotionPreference) -> Self {
        match preference {
            MotionPreference::Full => Self::Full,
            MotionPreference::Reduced => Self::Reduced,
        }
    }

    const fn into_preference(self) -> MotionPreference {
        match self {
            Self::Full => MotionPreference::Full,
            Self::Reduced => MotionPreference::Reduced,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WireGraphicsQuality {
    Auto,
    Low,
    High,
}

impl WireGraphicsQuality {
    const fn from_quality(quality: GraphicsQuality) -> Self {
        match quality {
            GraphicsQuality::Auto => Self::Auto,
            GraphicsQuality::Low => Self::Low,
            GraphicsQuality::High => Self::High,
        }
    }

    const fn into_quality(self) -> GraphicsQuality {
        match self {
            Self::Auto => GraphicsQuality::Auto,
            Self::Low => GraphicsQuality::Low,
            Self::High => GraphicsQuality::High,
        }
    }
}
