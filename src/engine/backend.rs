use std::fmt;

/// Presentation backend selected by wgpu for the current client.
///
/// This is diagnostic state only. It must never be encoded into Workshop
/// commands, archives, revisions, or state digests.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BackendKind {
    Metal,
    Dx12,
    Vulkan,
    Gl,
    WebGpu,
    WebGl2Low,
}

impl BackendKind {
    /// Reports the selected backend from wgpu's adapter metadata.
    pub fn from_adapter(adapter: &wgpu::Adapter) -> Option<Self> {
        Self::from_wgpu(adapter.get_info().backend)
    }

    /// Converts wgpu's backend identity into NYON's user-facing diagnostic.
    ///
    /// wgpu reports WebGL2 adapters as `Backend::Gl`; the target distinguishes
    /// browser WebGL2 from a native OpenGL/OpenGL ES fallback.
    pub const fn from_wgpu(backend: wgpu::Backend) -> Option<Self> {
        match backend {
            wgpu::Backend::Noop => None,
            wgpu::Backend::Vulkan => Some(Self::Vulkan),
            wgpu::Backend::Metal => Some(Self::Metal),
            wgpu::Backend::Dx12 => Some(Self::Dx12),
            wgpu::Backend::Gl => {
                #[cfg(target_arch = "wasm32")]
                {
                    Some(Self::WebGl2Low)
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    Some(Self::Gl)
                }
            }
            wgpu::Backend::BrowserWebGpu => Some(Self::WebGpu),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Metal => "METAL",
            Self::Dx12 => "DX12",
            Self::Vulkan => "VULKAN",
            Self::Gl => "GL",
            Self::WebGpu => "WEBGPU",
            Self::WebGl2Low => "WEBGL2 LOW",
        }
    }

    pub const fn is_low_capability(self) -> bool {
        matches!(self, Self::WebGl2Low)
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::BackendKind;

    #[test]
    fn native_wgpu_backends_have_stable_diagnostic_labels() {
        let cases = [
            (wgpu::Backend::Metal, BackendKind::Metal, "METAL"),
            (wgpu::Backend::Dx12, BackendKind::Dx12, "DX12"),
            (wgpu::Backend::Vulkan, BackendKind::Vulkan, "VULKAN"),
            (wgpu::Backend::Gl, BackendKind::Gl, "GL"),
            (wgpu::Backend::BrowserWebGpu, BackendKind::WebGpu, "WEBGPU"),
        ];

        for (wgpu_backend, expected, label) in cases {
            let backend = BackendKind::from_wgpu(wgpu_backend);
            assert_eq!(backend, Some(expected));
            assert_eq!(expected.label(), label);
            assert_eq!(expected.to_string(), label);
        }
        assert_eq!(BackendKind::from_wgpu(wgpu::Backend::Noop), None);
        assert!(!BackendKind::Gl.is_low_capability());
        assert!(BackendKind::WebGl2Low.is_low_capability());
    }
}
