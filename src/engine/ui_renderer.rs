use std::{mem::size_of, num::NonZeroU64};

use crate::{
    engine::{
        gpu::GpuContext,
        shader::{ShaderError, validate_wgsl},
    },
    ui::{
        AtlasMetrics, AtlasValidationError, MAX_UI_GLYPHS, MAX_UI_PANELS, UI_ATLAS_BYTES,
        UI_ATLAS_HEIGHT, UI_ATLAS_WIDTH, UI_SDF_WGSL, UiBatch, UiGlyphInstance, UiPanelInstance,
    },
};

const UI_GLOBALS_SIZE: u64 = size_of::<UiGlobals>() as u64;
const UI_INSTANCE_BUFFER_SIZE: u64 = (MAX_UI_GLYPHS * size_of::<UiGlyphInstance>()) as u64;
const UI_PANEL_BUFFER_SIZE: u64 = (MAX_UI_PANELS * size_of::<UiPanelInstance>()) as u64;
const SDF_WIDTH: f32 = 1.0 / 16.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct UiGlobals {
    viewport: [f32; 2],
    sdf_width: f32,
    padding: f32,
}

const _: () = assert!(size_of::<UiGlobals>() == 16);
const _: () = assert!(size_of::<UiGlyphInstance>() == 48);
const _: () = assert!(size_of::<UiPanelInstance>() == 32);

#[derive(Debug, thiserror::Error)]
pub enum UiRendererError {
    #[error(transparent)]
    Shader(#[from] ShaderError),
    #[error("failed to parse embedded UI atlas metrics: {0}")]
    MetricsParse(#[from] serde_json::Error),
    #[error(transparent)]
    Metrics(#[from] AtlasValidationError),
    #[error("the presentation surface is not configured")]
    UnconfiguredSurface,
    #[error("UI {kind} instance count {actual} exceeds renderer capacity {maximum}")]
    InstanceCapacity {
        kind: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("UI {kind} instances require {required} bytes but the device limit is {maximum} bytes")]
    InstanceBufferLimitExceeded {
        kind: &'static str,
        required: u64,
        maximum: u64,
    },
}

/// Device-epoch-owned SDF command-interface renderer. The atlas, pipeline, and
/// bounded upload buffer are recreated whenever the parent `Renderer` is.
pub struct UiRenderer {
    panel_pipeline: wgpu::RenderPipeline,
    glyph_pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    panels: wgpu::Buffer,
    instances: wgpu::Buffer,
    _atlas: wgpu::Texture,
}

impl UiRenderer {
    pub fn new(gpu: &GpuContext) -> Result<Self, UiRendererError> {
        let format = gpu
            .surface_format()
            .ok_or(UiRendererError::UnconfiguredSurface)?;
        validate_wgsl("assets/shaders/ui_sdf.wgsl", UI_SDF_WGSL)?;
        AtlasMetrics::embedded()?.validate()?;

        let max_buffer_size = gpu.device.limits().max_buffer_size;
        check_device_buffer_limit("glyph", UI_INSTANCE_BUFFER_SIZE, max_buffer_size)?;
        check_device_buffer_limit("panel", UI_PANEL_BUFFER_SIZE, max_buffer_size)?;

        let extent = wgpu::Extent3d {
            width: UI_ATLAS_WIDTH,
            height: UI_ATLAS_HEIGHT,
            depth_or_array_layers: 1,
        };
        let atlas = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("embedded UI SDF atlas"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            UI_ATLAS_BYTES,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(UI_ATLAS_WIDTH),
                rows_per_image: Some(UI_ATLAS_HEIGHT),
            },
            extent,
        );
        let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UI SDF atlas sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let globals = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI SDF globals"),
            size: UI_GLOBALS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instances = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI SDF glyph instances"),
            size: UI_INSTANCE_BUFFER_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let panels = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI panel instances"),
            size: UI_PANEL_BUFFER_SIZE,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("UI SDF bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: NonZeroU64::new(UI_GLOBALS_SIZE),
                            },
                            count: None,
                        },
                    ],
                });
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("UI SDF bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: globals.as_entire_binding(),
                },
            ],
        });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI SDF pipeline layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI SDF shader"),
                source: wgpu::ShaderSource::Wgsl(UI_SDF_WGSL.into()),
            });
        let panel_pipeline = create_pipeline(
            &gpu.device,
            format,
            &pipeline_layout,
            &shader,
            "UI panel pipeline",
            "panel_vs",
            "panel_fs",
            UiPanelInstance::LAYOUT,
        );
        let glyph_pipeline = create_pipeline(
            &gpu.device,
            format,
            &pipeline_layout,
            &shader,
            "UI SDF glyph pipeline",
            "vs_main",
            "fs_main",
            UiGlyphInstance::LAYOUT,
        );

        Ok(Self {
            panel_pipeline,
            glyph_pipeline,
            globals,
            bind_group,
            panels,
            instances,
            _atlas: atlas,
        })
    }

    pub(crate) fn prepare(
        &self,
        gpu: &GpuContext,
        logical_viewport: [f32; 2],
        batch: Option<&UiBatch>,
    ) -> Result<UiDrawCounts, UiRendererError> {
        if logical_viewport
            .iter()
            .any(|extent| !extent.is_finite() || *extent <= 0.0)
        {
            return Ok(UiDrawCounts::default());
        }
        let Some(batch) = batch else {
            return Ok(UiDrawCounts::default());
        };
        let counts = ui_draw_counts(batch.glyphs().len(), batch.panels().len())?;
        let globals = UiGlobals {
            viewport: logical_viewport,
            sdf_width: SDF_WIDTH,
            padding: 0.0,
        };
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        if !batch.glyphs().is_empty() {
            gpu.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(batch.glyphs()));
        }
        if !batch.panels().is_empty() {
            gpu.queue
                .write_buffer(&self.panels, 0, bytemuck::cast_slice(batch.panels()));
        }
        Ok(counts)
    }

    pub(crate) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        counts: UiDrawCounts,
    ) {
        if counts == UiDrawCounts::default() {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SDF text and icons"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.bind_group, &[]);
        if counts.panels != 0 {
            pass.set_pipeline(&self.panel_pipeline);
            pass.set_vertex_buffer(0, self.panels.slice(..));
            pass.draw(0..6, 0..counts.panels);
        }
        if counts.glyphs != 0 {
            pass.set_pipeline(&self.glyph_pipeline);
            pass.set_vertex_buffer(0, self.instances.slice(..));
            pass.draw(0..6, 0..counts.glyphs);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct UiDrawCounts {
    panels: u32,
    glyphs: u32,
}

fn ui_draw_counts(glyphs: usize, panels: usize) -> Result<UiDrawCounts, UiRendererError> {
    if glyphs > MAX_UI_GLYPHS {
        return Err(UiRendererError::InstanceCapacity {
            kind: "glyph",
            actual: glyphs,
            maximum: MAX_UI_GLYPHS,
        });
    }
    if panels > MAX_UI_PANELS {
        return Err(UiRendererError::InstanceCapacity {
            kind: "panel",
            actual: panels,
            maximum: MAX_UI_PANELS,
        });
    }
    Ok(UiDrawCounts {
        panels: u32::try_from(panels).expect("MAX_UI_PANELS fits in u32"),
        glyphs: u32::try_from(glyphs).expect("MAX_UI_GLYPHS fits in u32"),
    })
}

fn check_device_buffer_limit(
    kind: &'static str,
    required: u64,
    maximum: u64,
) -> Result<(), UiRendererError> {
    if required > maximum {
        Err(UiRendererError::InstanceBufferLimitExceeded {
            kind,
            required,
            maximum,
        })
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &'static str,
    vertex_entry: &'static str,
    fragment_entry: &'static str,
    buffer: wgpu::VertexBufferLayout<'static>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vertex_entry),
            compilation_options: Default::default(),
            buffers: &[Some(buffer)],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_upload_is_bounded_by_the_frozen_batch_capacity() {
        assert_eq!(
            ui_draw_counts(MAX_UI_GLYPHS, MAX_UI_PANELS).unwrap(),
            UiDrawCounts {
                glyphs: MAX_UI_GLYPHS as u32,
                panels: MAX_UI_PANELS as u32,
            }
        );
        assert!(matches!(
            ui_draw_counts(MAX_UI_GLYPHS + 1, 0),
            Err(UiRendererError::InstanceCapacity { kind: "glyph", .. })
        ));
        assert!(matches!(
            ui_draw_counts(0, MAX_UI_PANELS + 1),
            Err(UiRendererError::InstanceCapacity { kind: "panel", .. })
        ));
        assert_eq!(UI_INSTANCE_BUFFER_SIZE, MAX_UI_GLYPHS as u64 * 48);
        assert_eq!(UI_PANEL_BUFFER_SIZE, MAX_UI_PANELS as u64 * 32);
    }
}
