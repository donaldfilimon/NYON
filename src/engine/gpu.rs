use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("failed to create a presentation surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("failed to request a compatible graphics adapter: {0}")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),
    #[error("failed to request a graphics device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("the graphics adapter does not support the window surface")]
    UnsupportedSurface,
}

/// Owns the portable WebGPU instance, window surface, device, and presentation queue.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    window: Arc<Window>,
    physical_size: PhysicalSize<u32>,
}

impl GpuContext {
    /// Creates the instance and presentation surface synchronously. Native
    /// callers invoke this on the window-system thread before moving adapter
    /// and device acquisition to an executor.
    pub(crate) fn prepare(window: Arc<Window>) -> Result<PreparedGpuContext, GpuError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        #[cfg(target_arch = "wasm32")]
        {
            descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            descriptor.backends = wgpu::Backends::PRIMARY;
        }
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance.create_surface(Arc::clone(&window))?;

        Ok(PreparedGpuContext {
            instance,
            surface,
            window,
        })
    }

    /// Portable convenience constructor. Browser callers poll this on their
    /// event-loop thread; native application startup uses the split prepare /
    /// initialize path so final surface configuration returns to that thread.
    pub async fn new(window: Arc<Window>) -> Result<Self, GpuError> {
        let mut context = Self::prepare(window)?.initialize().await?;
        let physical_size = context.window.inner_size();
        context.resize(physical_size)?;
        Ok(context)
    }

    /// Updates the physical swapchain extent. Logical rendering coordinates are
    /// intentionally not accepted here.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<bool, GpuError> {
        let previous_format = self.surface_format();
        let config = surface_config(&self.surface, &self.adapter, size)?;
        if let Some(config) = &config {
            self.surface.configure(&self.device, config);
        }

        let next_format = config.as_ref().map(|config| config.format);
        self.physical_size = size;
        self.config = config;
        Ok(previous_format != next_format)
    }

    /// Recreates a lost surface from the retained owned window.
    pub fn recreate_surface(&mut self) -> Result<bool, GpuError> {
        let surface = self.instance.create_surface(Arc::clone(&self.window))?;
        let config = surface_config(&surface, &self.adapter, self.physical_size)?;
        if let Some(config) = &config {
            surface.configure(&self.device, config);
        }

        let format_changed = self.surface_format() != config.as_ref().map(|config| config.format);
        self.surface = surface;
        self.config = config;
        Ok(format_changed)
    }

    pub fn acquire(&self) -> Option<wgpu::CurrentSurfaceTexture> {
        self.config
            .as_ref()
            .map(|_| self.surface.get_current_texture())
    }

    pub fn physical_size(&self) -> PhysicalSize<u32> {
        self.physical_size
    }

    pub fn surface_format(&self) -> Option<wgpu::TextureFormat> {
        self.config.as_ref().map(|config| config.format)
    }
}

/// Main-thread-created resources that are safe to move to a native executor
/// for asynchronous adapter and device acquisition.
pub(crate) struct PreparedGpuContext {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    window: Arc<Window>,
}

impl PreparedGpuContext {
    pub(crate) async fn initialize(self) -> Result<GpuContext, GpuError> {
        let adapter = self
            .instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&self.surface),
                apply_limit_buckets: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Intergalactic Warfare device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await?;

        Ok(GpuContext {
            instance: self.instance,
            surface: self.surface,
            adapter,
            device,
            queue,
            config: None,
            window: self.window,
            physical_size: PhysicalSize::new(0, 0),
        })
    }
}

fn surface_config(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    size: PhysicalSize<u32>,
) -> Result<Option<wgpu::SurfaceConfiguration>, GpuError> {
    if !has_presentable_extent(size) {
        return Ok(None);
    }

    surface
        .get_default_config(adapter, size.width, size.height)
        .map(Some)
        .ok_or(GpuError::UnsupportedSurface)
}

const fn has_presentable_extent(size: PhysicalSize<u32>) -> bool {
    size.width != 0 && size.height != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_extent_suspends_surface_configuration() {
        assert!(!has_presentable_extent(PhysicalSize::new(0, 720)));
        assert!(!has_presentable_extent(PhysicalSize::new(1280, 0)));
        assert!(!has_presentable_extent(PhysicalSize::new(0, 0)));
        assert!(has_presentable_extent(PhysicalSize::new(1, 1)));
    }

    #[test]
    fn surface_state_is_committed_only_after_preparation_and_configuration() {
        let source = include_str!("gpu.rs");
        let resize_start = source.find("pub fn resize").unwrap();
        let recreate_start = source.find("pub fn recreate_surface").unwrap();
        let acquire_start = source.find("pub fn acquire").unwrap();

        let resize = &source[resize_start..recreate_start];
        assert!(
            resize.find("surface_config").unwrap() < resize.find("self.physical_size").unwrap()
        );
        assert!(
            resize.find("surface_config").unwrap() < resize.find("self.config = config").unwrap()
        );

        let recreate = &source[recreate_start..acquire_start];
        assert!(
            recreate.find("surface_config").unwrap() < recreate.find("surface.configure").unwrap()
        );
        assert!(
            recreate.find("surface.configure").unwrap()
                < recreate.find("self.surface = surface").unwrap()
        );
        assert!(
            recreate.find("surface.configure").unwrap()
                < recreate.find("self.config = config").unwrap()
        );
    }
}
