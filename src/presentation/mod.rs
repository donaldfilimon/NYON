//! Presentation-only camera, scene extraction, and picking.

mod camera;
mod interaction;
mod picking;
mod scene;
pub mod ui;

pub use camera::*;
pub use interaction::*;
pub use picking::*;
pub use scene::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GraphicsQuality {
    #[default]
    Auto,
    Low,
    High,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PresentationPreferences {
    pub graphics_quality: GraphicsQuality,
    pub motion: MotionPreference,
    pub high_contrast: bool,
}

impl Default for PresentationPreferences {
    fn default() -> Self {
        Self {
            graphics_quality: GraphicsQuality::Auto,
            motion: MotionPreference::Full,
            high_contrast: false,
        }
    }
}
