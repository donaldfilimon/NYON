use crate::presentation::GraphicsQuality;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedGraphicsQuality {
    pub requested: GraphicsQuality,
    pub effective: GraphicsQuality,
    pub sample_count: u32,
    pub bloom: bool,
    pub particle_divisor: u32,
}

/// Monotonic device-epoch state for optional High resources. Once any optional
/// High acquisition fails, the renderer stays on the required Low path until a
/// new device epoch constructs a fresh state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphicsResourceState {
    high_available: bool,
}

impl GraphicsResourceState {
    pub const fn new(high_available: bool) -> Self {
        Self { high_available }
    }

    pub const fn resolve(self, requested: GraphicsQuality) -> ResolvedGraphicsQuality {
        resolve_quality(requested, self.high_available)
    }

    pub const fn high_available(self) -> bool {
        self.high_available
    }

    pub fn disable_high(&mut self) {
        self.high_available = false;
    }
}

pub const fn resolve_quality(
    requested: GraphicsQuality,
    high_resources_supported: bool,
) -> ResolvedGraphicsQuality {
    let use_high = !matches!(requested, GraphicsQuality::Low) && high_resources_supported;
    ResolvedGraphicsQuality {
        requested,
        effective: if use_high {
            GraphicsQuality::High
        } else {
            GraphicsQuality::Low
        },
        sample_count: if use_high { 4 } else { 1 },
        bloom: use_high,
        particle_divisor: if use_high { 1 } else { 4 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_and_high_fall_back_monotonically_when_optional_resources_are_missing() {
        for requested in [GraphicsQuality::Auto, GraphicsQuality::High] {
            assert_eq!(
                resolve_quality(requested, false).effective,
                GraphicsQuality::Low
            );
            assert_eq!(resolve_quality(requested, false).sample_count, 1);
            assert!(!resolve_quality(requested, false).bloom);
        }
        assert_eq!(resolve_quality(GraphicsQuality::Auto, true).sample_count, 4);
        assert!(resolve_quality(GraphicsQuality::High, true).bloom);
        assert_eq!(resolve_quality(GraphicsQuality::Low, true).sample_count, 1);
    }

    #[test]
    fn injected_high_creation_failure_disables_high_for_the_device_epoch() {
        let mut resources = GraphicsResourceState::new(true);
        assert_eq!(
            resources.resolve(GraphicsQuality::Auto).effective,
            GraphicsQuality::High
        );

        resources.disable_high();
        for requested in [GraphicsQuality::Auto, GraphicsQuality::High] {
            let resolved = resources.resolve(requested);
            assert_eq!(resolved.effective, GraphicsQuality::Low);
            assert_eq!(resolved.sample_count, 1);
            assert!(!resolved.bloom);
        }

        resources.disable_high();
        assert!(!resources.high_available());
    }
}
