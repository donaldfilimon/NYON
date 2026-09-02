use std::ops::Range;
use std::{mem::size_of, num::NonZeroU64};

use crate::engine::{
    gpu::GpuContext,
    primitives::{PrimitiveBatch, Vertex},
    quality::{ResolvedGraphicsQuality, resolve_quality},
    render_frame::RenderFrame,
    scene_renderer::{SceneRenderer, SceneRendererError},
    shader::{PRIMITIVES_WGSL, ShaderError, validate_wgsl},
    ui_renderer::{UiRenderer, UiRendererError},
};

const GLOBALS_SIZE: u64 = 16;
const INITIAL_VERTEX_CAPACITY: u64 = 64;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport: [f32; 2],
    padding: [f32; 2],
}

const _: () = assert!(size_of::<Globals>() == GLOBALS_SIZE as usize);

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error(transparent)]
    Shader(#[from] ShaderError),
    #[error(transparent)]
    Scene(#[from] SceneRendererError),
    #[error(transparent)]
    Ui(#[from] UiRendererError),
    #[error("the presentation surface is not configured")]
    UnconfiguredSurface,
    #[error("vertex count {count} exceeds the maximum draw count of {maximum}")]
    VertexCountTooLarge { count: usize, maximum: u32 },
    #[error("vertex byte size overflow for {count} vertices")]
    VertexByteSizeOverflow { count: usize },
    #[error("vertex data requires {required} bytes but the device limit is {maximum} bytes")]
    VertexBufferLimitExceeded { required: u64, maximum: u64 },
}

/// Composes the tactical scene and compatibility primitive HUD for an
/// immutable presentation frame.
pub struct Renderer {
    scene: SceneRenderer,
    ui: UiRenderer,
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: u64,
    max_vertex_buffer_size: u64,
}

impl Renderer {
    pub fn new(gpu: &GpuContext) -> Result<Self, RendererError> {
        let format = gpu
            .surface_format()
            .ok_or(RendererError::UnconfiguredSurface)?;
        validate_wgsl("assets/shaders/primitives.wgsl", PRIMITIVES_WGSL)?;
        let max_vertex_buffer_size = gpu.device.limits().max_buffer_size;
        let initial_vertex_capacity = INITIAL_VERTEX_CAPACITY.min(max_vertex_buffer_size);
        if initial_vertex_capacity == 0 {
            return Err(RendererError::VertexBufferLimitExceeded {
                required: 1,
                maximum: max_vertex_buffer_size,
            });
        }

        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("primitive shader"),
                source: wgpu::ShaderSource::Wgsl(PRIMITIVES_WGSL.into()),
            });
        let globals = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("primitive globals"),
            size: GLOBALS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("primitive globals layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(GLOBALS_SIZE),
                        },
                        count: None,
                    }],
                });
        let globals_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("primitive globals bind group"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("primitive pipeline layout"),
                bind_group_layouts: &[Some(&globals_layout)],
                immediate_size: 0,
            });
        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("primitive pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(Vertex::LAYOUT)],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            });
        let vertex_buffer = create_vertex_buffer(&gpu.device, initial_vertex_capacity);
        let scene = SceneRenderer::new(gpu)?;
        let ui = UiRenderer::new(gpu)?;

        Ok(Self {
            scene,
            ui,
            pipeline,
            globals,
            globals_bind_group,
            vertex_buffer,
            vertex_capacity: initial_vertex_capacity,
            max_vertex_buffer_size,
        })
    }

    /// Renders and presents one immutable frame. The tactical scene is optional
    /// so editor and diagnostic surfaces can retain the primitive-only path.
    pub fn render(
        &mut self,
        gpu: &GpuContext,
        surface_texture: wgpu::SurfaceTexture,
        frame: &RenderFrame<'_>,
    ) -> Result<ResolvedGraphicsQuality, RendererError> {
        if !frame.has_presentable_extent() {
            self.scene.release_surface_targets();
            return Ok(resolve_quality(frame.quality, false));
        }

        let primitive_ranges = self.prepare_primitive_layers(
            gpu,
            frame.logical_viewport,
            frame.primitive_underlay,
            frame.primitive_overlay,
        )?;
        let ui_draw_counts = self.ui.prepare(gpu, frame.logical_viewport, frame.ui)?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tactical render frame encoder"),
            });
        let quality = if let Some(scene) = frame.scene {
            self.scene.render(
                gpu,
                &mut encoder,
                &view,
                frame.physical_target,
                frame.quality,
                scene,
            )?
        } else {
            self.scene.release_surface_targets();
            clear_target(&mut encoder, &view);
            resolve_quality(frame.quality, false)
        };
        self.encode_primitive_range(
            &mut encoder,
            &view,
            "primitive HUD underlay",
            primitive_ranges.underlay,
        );
        self.ui.encode(&mut encoder, &view, ui_draw_counts);
        self.encode_primitive_range(
            &mut encoder,
            &view,
            "primitive focus and diagnostic overlay",
            primitive_ranges.overlay,
        );
        gpu.queue.submit(Some(encoder.finish()));
        gpu.queue.present(surface_texture);
        Ok(quality)
    }

    /// Invalidates only physical-size-dependent targets. Meshes, pipelines,
    /// and instance buffers remain owned by the current device epoch.
    pub fn resize(&mut self, physical_target: [u32; 2]) {
        self.scene.resize(physical_target);
    }

    fn prepare_primitive_layers(
        &mut self,
        gpu: &GpuContext,
        logical_viewport: [f32; 2],
        underlay: &PrimitiveBatch,
        overlay: &PrimitiveBatch,
    ) -> Result<PrimitiveDrawRanges, RendererError> {
        let total_count = underlay
            .vertices()
            .len()
            .checked_add(overlay.vertices().len())
            .ok_or(RendererError::VertexByteSizeOverflow { count: usize::MAX })?;
        let upload = vertex_upload_plan(
            total_count,
            self.vertex_capacity,
            self.max_vertex_buffer_size,
        )?;
        let globals = Globals {
            viewport: logical_viewport.map(|value| {
                if value.is_finite() {
                    value.max(0.0)
                } else {
                    0.0
                }
            }),
            padding: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        if let Some(new_capacity) = upload.new_capacity {
            self.vertex_capacity = new_capacity;
            self.vertex_buffer = create_vertex_buffer(&gpu.device, new_capacity);
        }
        let underlay_bytes = bytemuck::cast_slice(underlay.vertices());
        let overlay_bytes = bytemuck::cast_slice(overlay.vertices());
        debug_assert_eq!(
            u64::try_from(underlay_bytes.len().saturating_add(overlay_bytes.len())),
            Ok(upload.byte_len)
        );
        if !underlay_bytes.is_empty() {
            gpu.queue
                .write_buffer(&self.vertex_buffer, 0, underlay_bytes);
        }
        if !overlay_bytes.is_empty() {
            gpu.queue.write_buffer(
                &self.vertex_buffer,
                u64::try_from(underlay_bytes.len())
                    .expect("validated primitive byte length fits in u64"),
                overlay_bytes,
            );
        }

        let underlay_count = u32::try_from(underlay.vertices().len()).map_err(|_| {
            RendererError::VertexCountTooLarge {
                count: underlay.vertices().len(),
                maximum: u32::MAX,
            }
        })?;

        Ok(PrimitiveDrawRanges {
            underlay: 0..underlay_count,
            overlay: underlay_count..upload.vertex_count,
        })
    }

    fn encode_primitive_range(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        label: &'static str,
        vertices: Range<u32>,
    ) {
        if vertices.is_empty() {
            return;
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(vertices, 0..1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PrimitiveDrawRanges {
    underlay: Range<u32>,
    overlay: Range<u32>,
}

fn clear_target(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("primitive-only background clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct VertexUploadPlan {
    vertex_count: u32,
    byte_len: u64,
    new_capacity: Option<u64>,
}

fn vertex_upload_plan(
    count: usize,
    current_capacity: u64,
    max_buffer_size: u64,
) -> Result<VertexUploadPlan, RendererError> {
    let vertex_count = u32::try_from(count).map_err(|_| RendererError::VertexCountTooLarge {
        count,
        maximum: u32::MAX,
    })?;
    let byte_len = u64::from(vertex_count)
        .checked_mul(size_of::<Vertex>() as u64)
        .ok_or(RendererError::VertexByteSizeOverflow { count })?;
    if byte_len > max_buffer_size {
        return Err(RendererError::VertexBufferLimitExceeded {
            required: byte_len,
            maximum: max_buffer_size,
        });
    }

    let new_capacity = if byte_len > current_capacity {
        Some(
            byte_len
                .checked_next_power_of_two()
                .filter(|capacity| *capacity <= max_buffer_size)
                .unwrap_or(byte_len),
        )
    } else {
        None
    };

    Ok(VertexUploadPlan {
        vertex_count,
        byte_len,
        new_capacity,
    })
}

fn create_vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("primitive vertices"),
        size: size.max(1),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_plan_rejects_vertex_counts_that_cannot_be_drawn() {
        let count = usize::try_from(u64::from(u32::MAX) + 1).unwrap();
        assert!(matches!(
            vertex_upload_plan(count, INITIAL_VERTEX_CAPACITY, u64::MAX),
            Err(RendererError::VertexCountTooLarge { .. })
        ));
    }

    #[test]
    fn upload_plan_rejects_byte_ranges_above_the_device_limit() {
        let required = 10 * size_of::<Vertex>() as u64;
        assert!(matches!(
            vertex_upload_plan(10, INITIAL_VERTEX_CAPACITY, required - 1),
            Err(RendererError::VertexBufferLimitExceeded {
                required: actual,
                maximum,
            }) if actual == required && maximum == required - 1
        ));
    }

    #[test]
    fn upload_plan_uses_checked_growth_and_an_exact_limit_fallback() {
        let required = 3 * size_of::<Vertex>() as u64;
        let power_of_two = required.checked_next_power_of_two().unwrap();

        let grown = vertex_upload_plan(3, INITIAL_VERTEX_CAPACITY, power_of_two).unwrap();
        assert_eq!(grown.new_capacity, Some(power_of_two));

        let exact = vertex_upload_plan(3, INITIAL_VERTEX_CAPACITY, required).unwrap();
        assert_eq!(exact.new_capacity, Some(required));
        assert_eq!(exact.vertex_count, 3);
        assert_eq!(exact.byte_len, required);
    }

    #[test]
    fn upload_plan_keeps_a_sufficient_nonzero_buffer_for_an_empty_batch() {
        let plan = vertex_upload_plan(0, INITIAL_VERTEX_CAPACITY, u64::MAX).unwrap();
        assert_eq!(plan.vertex_count, 0);
        assert_eq!(plan.byte_len, 0);
        assert_eq!(plan.new_capacity, None);
    }
}
