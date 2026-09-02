use crate::{
    engine::primitives::PrimitiveBatch,
    presentation::{GraphicsQuality, SceneFrame},
    ui::UiBatch,
};

/// Immutable input to one rendered frame. Logical layout and physical target
/// sizes remain explicit and separate.
pub struct RenderFrame<'a> {
    pub logical_viewport: [f32; 2],
    pub physical_target: [u32; 2],
    pub quality: GraphicsQuality,
    /// The tactical scene is absent for editor and diagnostic frames that only
    /// need the compatibility primitive overlay.
    pub scene: Option<&'a SceneFrame>,
    /// SDF command-interface geometry is optional while startup, failures, and
    /// the detached editor retain the primitive-only compatibility path.
    pub ui: Option<&'a UiBatch>,
    /// Compatibility chrome that must sit below the SDF command interface.
    /// Keeping it separate prevents legacy translucent panels from dimming the
    /// new text and controls.
    pub primitive_underlay: &'a PrimitiveBatch,
    /// Focus, diagnostics, and primitive-only fallbacks that must remain the
    /// final visible stage.
    pub primitive_overlay: &'a PrimitiveBatch,
}

impl RenderFrame<'_> {
    pub const fn has_presentable_extent(&self) -> bool {
        self.physical_target[0] != 0 && self.physical_target[1] != 0
    }
}
