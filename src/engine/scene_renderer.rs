use std::{
    future::Future,
    mem::size_of,
    num::NonZeroU64,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;

use crate::{
    engine::{
        gpu::GpuContext,
        quality::{GraphicsResourceState, ResolvedGraphicsQuality},
        resources::{
            FLEET_VERTEX_COUNT, FleetInstance, HALO_VERTEX_COUNT, HaloInstance,
            MAX_ROUTE_PARTICLES, MeshVertex, PARTICLE_VERTEX_COUNT, ParticleInstance,
            ROUTE_VERTEX_COUNT, RouteInstance, SPHERE_INDEX_COUNT, WorldInstance,
            generate_sphere_mesh,
        },
        shader::{
            POSTPROCESS_WGSL, ROUTES_WGSL, SPACE_WGSL, ShaderError, WORLDS_WGSL, validate_wgsl,
        },
    },
    presentation::{GraphicsQuality, MAX_SCENE_FLEETS, MAX_SCENE_ROUTES, SceneFrame},
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const BLOOM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const SCENE_GLOBALS_BINDING: u32 = 0;
pub const SCENE_GLOBALS_SIZE: u64 = size_of::<SceneGlobals>() as u64;
pub const BLOOM_TEXTURE_BINDING: u32 = 0;
pub const BLOOM_SAMPLER_BINDING: u32 = 1;
pub const BLOOM_TEXTURE_SAMPLE_TYPE: wgpu::TextureSampleType =
    wgpu::TextureSampleType::Float { filterable: true };
pub const BLOOM_TEXTURE_VIEW_DIMENSION: wgpu::TextureViewDimension = wgpu::TextureViewDimension::D2;
pub const BLOOM_SAMPLER_BINDING_TYPE: wgpu::SamplerBindingType =
    wgpu::SamplerBindingType::Filtering;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SceneGlobals {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 4],
    visual: [f32; 4],
    effects: [f32; 4],
}

const _: () = assert!(size_of::<SceneGlobals>() == 112);

#[derive(Debug, thiserror::Error)]
pub enum SceneRendererError {
    #[error(transparent)]
    Shader(#[from] ShaderError),
    #[error("the presentation surface is not configured")]
    UnconfiguredSurface,
    #[error("scene contains {actual} worlds; exactly {expected} are required")]
    InvalidWorldCount { actual: usize, expected: usize },
    #[error("scene {kind} count {actual} exceeds renderer capacity {maximum}")]
    InstanceCapacity {
        kind: &'static str,
        actual: usize,
        maximum: usize,
    },
}

struct ScenePipelines {
    background: wgpu::RenderPipeline,
    worlds: wgpu::RenderPipeline,
    routes: wgpu::RenderPipeline,
    fleets: wgpu::RenderPipeline,
    halos: wgpu::RenderPipeline,
    particles: wgpu::RenderPipeline,
}

struct SurfaceTargets {
    physical_size: [u32; 2],
    sample_count: u32,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    _msaa_texture: Option<wgpu::Texture>,
    msaa_view: Option<wgpu::TextureView>,
    _bloom_texture: Option<wgpu::Texture>,
    bloom_view: Option<wgpu::TextureView>,
    bloom_bind_group: Option<wgpu::BindGroup>,
}

/// Device-epoch-owned tactical renderer. All inputs are presentation-derived;
/// this type never mutates campaign state.
pub struct SceneRenderer {
    format: wgpu::TextureFormat,
    globals: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    sphere_vertices: wgpu::Buffer,
    sphere_indices: wgpu::Buffer,
    world_instances: wgpu::Buffer,
    route_instances: wgpu::Buffer,
    fleet_instances: wgpu::Buffer,
    halo_instances: wgpu::Buffer,
    particle_instances: wgpu::Buffer,
    low: ScenePipelines,
    high: Option<ScenePipelines>,
    resource_state: GraphicsResourceState,
    glow_routes: wgpu::RenderPipeline,
    glow_fleets: wgpu::RenderPipeline,
    glow_halos: wgpu::RenderPipeline,
    glow_particles: wgpu::RenderPipeline,
    postprocess: wgpu::RenderPipeline,
    bloom_layout: wgpu::BindGroupLayout,
    bloom_sampler: wgpu::Sampler,
    targets: Option<SurfaceTargets>,
}

impl SceneRenderer {
    pub fn new(gpu: &GpuContext) -> Result<Self, SceneRendererError> {
        let format = gpu
            .surface_format()
            .ok_or(SceneRendererError::UnconfiguredSurface)?;
        validate_wgsl("assets/shaders/space.wgsl", SPACE_WGSL)?;
        validate_wgsl("assets/shaders/worlds.wgsl", WORLDS_WGSL)?;
        validate_wgsl("assets/shaders/routes.wgsl", ROUTES_WGSL)?;
        validate_wgsl("assets/shaders/postprocess.wgsl", POSTPROCESS_WGSL)?;

        let globals = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene globals"),
            size: SCENE_GLOBALS_SIZE,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("scene globals layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: SCENE_GLOBALS_BINDING,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(SCENE_GLOBALS_SIZE),
                        },
                        count: None,
                    }],
                });
        let globals_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene globals bind group"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: SCENE_GLOBALS_BINDING,
                resource: globals.as_entire_binding(),
            }],
        });

        let space_shader = shader_module(&gpu.device, "space shader", SPACE_WGSL);
        let worlds_shader = shader_module(&gpu.device, "world shader", WORLDS_WGSL);
        let routes_shader = shader_module(&gpu.device, "route and fleet shader", ROUTES_WGSL);
        let scene_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("tactical scene pipeline layout"),
                bind_group_layouts: &[Some(&globals_layout)],
                immediate_size: 0,
            });
        let low = create_scene_pipelines(
            &gpu.device,
            format,
            1,
            &scene_layout,
            &space_shader,
            &worlds_shader,
            &routes_shader,
        );

        let mut resource_state =
            GraphicsResourceState::new(supports_high_resources(&gpu.adapter, &gpu.device, format));
        let high = if resource_state.high_available() {
            match try_optional_high_resources(&gpu.device, "High scene pipelines", || {
                create_scene_pipelines(
                    &gpu.device,
                    format,
                    4,
                    &scene_layout,
                    &space_shader,
                    &worlds_shader,
                    &routes_shader,
                )
            }) {
                Ok(pipelines) => Some(pipelines),
                Err(error) => {
                    log::warn!("{error}; continuing with Low graphics for this device epoch");
                    resource_state.disable_high();
                    None
                }
            }
        } else {
            None
        };

        let glow_routes = create_transparent_pipeline(
            &gpu.device,
            BLOOM_FORMAT,
            1,
            None,
            &scene_layout,
            &routes_shader,
            "route_vs",
            "route_fs",
            &[Some(RouteInstance::LAYOUT)],
            wgpu::PrimitiveTopology::LineList,
            "bloom routes pipeline",
            additive_blend(),
        );
        let glow_fleets = create_transparent_pipeline(
            &gpu.device,
            BLOOM_FORMAT,
            1,
            None,
            &scene_layout,
            &routes_shader,
            "fleet_vs",
            "fleet_fs",
            &[Some(FleetInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "bloom fleets pipeline",
            additive_blend(),
        );
        let glow_halos = create_transparent_pipeline(
            &gpu.device,
            BLOOM_FORMAT,
            1,
            None,
            &scene_layout,
            &routes_shader,
            "halo_vs",
            "halo_fs",
            &[Some(HaloInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "bloom faction halos pipeline",
            additive_blend(),
        );
        let glow_particles = create_transparent_pipeline(
            &gpu.device,
            BLOOM_FORMAT,
            1,
            None,
            &scene_layout,
            &routes_shader,
            "particle_vs",
            "particle_fs",
            &[Some(ParticleInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "bloom route particles pipeline",
            additive_blend(),
        );

        let bloom_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloom texture layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: BLOOM_TEXTURE_BINDING,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: BLOOM_TEXTURE_SAMPLE_TYPE,
                            view_dimension: BLOOM_TEXTURE_VIEW_DIMENSION,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: BLOOM_SAMPLER_BINDING,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(BLOOM_SAMPLER_BINDING_TYPE),
                        count: None,
                    },
                ],
            });
        let post_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("postprocess pipeline layout"),
                bind_group_layouts: &[Some(&bloom_layout)],
                immediate_size: 0,
            });
        let post_shader = shader_module(&gpu.device, "postprocess shader", POSTPROCESS_WGSL);
        let postprocess = create_fullscreen_pipeline(
            &gpu.device,
            format,
            1,
            &post_layout,
            &post_shader,
            "postprocess pipeline",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let bloom_sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let mesh = generate_sphere_mesh();
        let sphere_vertices = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("procedural sphere vertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let sphere_indices = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("procedural sphere indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        Ok(Self {
            format,
            globals,
            globals_bind_group,
            sphere_vertices,
            sphere_indices,
            world_instances: instance_buffer::<WorldInstance>(&gpu.device, 7, "world instances"),
            route_instances: instance_buffer::<RouteInstance>(
                &gpu.device,
                MAX_SCENE_ROUTES,
                "route instances",
            ),
            fleet_instances: instance_buffer::<FleetInstance>(
                &gpu.device,
                MAX_SCENE_FLEETS,
                "fleet instances",
            ),
            halo_instances: instance_buffer::<HaloInstance>(&gpu.device, 7, "faction halos"),
            particle_instances: instance_buffer::<ParticleInstance>(
                &gpu.device,
                MAX_ROUTE_PARTICLES,
                "route particles",
            ),
            low,
            high,
            resource_state,
            glow_routes,
            glow_fleets,
            glow_halos,
            glow_particles,
            postprocess,
            bloom_layout,
            bloom_sampler,
            targets: None,
        })
    }

    pub fn render(
        &mut self,
        gpu: &GpuContext,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        physical_size: [u32; 2],
        requested_quality: GraphicsQuality,
        scene: &SceneFrame,
    ) -> Result<ResolvedGraphicsQuality, SceneRendererError> {
        if physical_size[0] == 0 || physical_size[1] == 0 {
            self.targets = None;
            return Ok(self.resource_state.resolve(requested_quality));
        }
        if scene.worlds.len() != 7 {
            return Err(SceneRendererError::InvalidWorldCount {
                actual: scene.worlds.len(),
                expected: 7,
            });
        }
        check_capacity("route", scene.routes.len(), MAX_SCENE_ROUTES)?;
        check_capacity("fleet", scene.fleets.len(), MAX_SCENE_FLEETS)?;

        let mut quality = self.resource_state.resolve(requested_quality);
        quality.bloom &= scene.effects.bloom_requested;
        quality = self.ensure_targets(gpu, physical_size, quality);
        let uploads = super::resources::SceneUploads::from_frame(scene, quality.particle_divisor);
        write_instances(&gpu.queue, &self.world_instances, &uploads.worlds);
        write_instances(&gpu.queue, &self.route_instances, &uploads.routes);
        write_instances(&gpu.queue, &self.fleet_instances, &uploads.fleets);
        write_instances(&gpu.queue, &self.halo_instances, &uploads.halos);
        write_instances(&gpu.queue, &self.particle_instances, &uploads.particles);

        let inverse_view = scene.camera.view.inverse();
        let camera_position = inverse_view.transform_point3(Vec3::ZERO);
        let visual_seed = scene.background.visual_seed;
        let globals = SceneGlobals {
            view_projection: scene.camera.view_projection.to_cols_array_2d(),
            camera_position: camera_position.extend(1.0).to_array(),
            visual: [
                scene.visual_time,
                (visual_seed & 0x00FF_FFFF) as f32,
                scene.background.parallax.x,
                scene.background.parallax.y,
            ],
            effects: [
                inverse_projection_aspect(scene.camera.projection),
                if quality.effective == GraphicsQuality::High {
                    1.0
                } else {
                    0.0
                },
                if scene.effects.animate_halos {
                    1.0
                } else {
                    0.0
                },
                if scene.background.static_background {
                    1.0
                } else {
                    0.0
                },
            ],
        };
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));

        let targets = self.targets.as_ref().expect("nonzero target was created");
        let pipelines = if quality.sample_count == 4 {
            self.high.as_ref().unwrap_or(&self.low)
        } else {
            &self.low
        };
        let main_view = targets.msaa_view.as_ref().unwrap_or(surface_view);

        render_background(encoder, main_view, pipelines, &self.globals_bind_group);
        self.render_worlds(encoder, main_view, &targets.depth_view, pipelines);
        self.render_transparents(
            encoder,
            main_view,
            (quality.sample_count == 4).then_some(surface_view),
            &targets.depth_view,
            pipelines,
            uploads.routes.len() as u32,
            uploads.fleets.len() as u32,
            uploads.halos.len() as u32,
            uploads.particles.len() as u32,
        );

        if quality.bloom
            && let (Some(bloom_view), Some(bloom_bind_group)) =
                (&targets.bloom_view, &targets.bloom_bind_group)
        {
            self.render_bloom(
                encoder,
                bloom_view,
                uploads.routes.len() as u32,
                uploads.fleets.len() as u32,
                uploads.halos.len() as u32,
                uploads.particles.len() as u32,
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("restrained bloom composite"),
                color_attachments: &[Some(color_attachment(
                    surface_view,
                    None,
                    wgpu::LoadOp::Load,
                ))],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.postprocess);
            pass.set_bind_group(0, bloom_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        Ok(quality)
    }

    pub fn resize(&mut self, physical_size: [u32; 2]) {
        if !requires_surface_targets(physical_size)
            || self
                .targets
                .as_ref()
                .is_some_and(|targets| targets.physical_size != physical_size)
        {
            self.targets = None;
        }
    }

    pub fn release_surface_targets(&mut self) {
        self.targets = None;
    }

    fn ensure_targets(
        &mut self,
        gpu: &GpuContext,
        physical_size: [u32; 2],
        quality: ResolvedGraphicsQuality,
    ) -> ResolvedGraphicsQuality {
        if self.targets.as_ref().is_some_and(|targets| {
            targets.physical_size == physical_size
                && targets.sample_count == quality.sample_count
                && targets.bloom_view.is_some() == quality.bloom
        }) {
            return quality;
        }

        if quality.effective == GraphicsQuality::High {
            match try_optional_high_resources(&gpu.device, "High surface targets", || {
                create_surface_targets(
                    &gpu.device,
                    self.format,
                    physical_size,
                    quality,
                    &self.bloom_layout,
                    &self.bloom_sampler,
                )
            }) {
                Ok(targets) => {
                    self.targets = Some(targets);
                    return quality;
                }
                Err(error) => {
                    log::warn!("{error}; continuing with Low graphics for this device epoch");
                    self.targets = None;
                    self.high = None;
                    self.resource_state.disable_high();
                }
            }
        }

        let low = self.resource_state.resolve(quality.requested);
        self.targets = Some(create_surface_targets(
            &gpu.device,
            self.format,
            physical_size,
            low,
            &self.bloom_layout,
            &self.bloom_sampler,
        ));
        low
    }

    fn render_worlds(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        pipelines: &ScenePipelines,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("depth-tested procedural worlds"),
            color_attachments: &[Some(color_attachment(color_view, None, wgpu::LoadOp::Load))],
            depth_stencil_attachment: Some(depth_attachment(depth_view, wgpu::LoadOp::Clear(1.0))),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipelines.worlds);
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        pass.set_vertex_buffer(0, self.sphere_vertices.slice(..));
        pass.set_vertex_buffer(1, self.world_instances.slice(..));
        pass.set_index_buffer(self.sphere_indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..SPHERE_INDEX_COUNT as u32, 0, 0..7);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_transparents(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        color_view: &wgpu::TextureView,
        resolve_target: Option<&wgpu::TextureView>,
        depth_view: &wgpu::TextureView,
        pipelines: &ScenePipelines,
        route_count: u32,
        fleet_count: u32,
        halo_count: u32,
        particle_count: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transparent tactical routes fleets and halos"),
            color_attachments: &[Some(color_attachment(
                color_view,
                resolve_target,
                wgpu::LoadOp::Load,
            ))],
            depth_stencil_attachment: Some(depth_attachment(depth_view, wgpu::LoadOp::Load)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        if route_count > 0 {
            pass.set_pipeline(&pipelines.routes);
            pass.set_vertex_buffer(0, self.route_instances.slice(..));
            pass.draw(0..ROUTE_VERTEX_COUNT, 0..route_count);
        }
        if fleet_count > 0 {
            pass.set_pipeline(&pipelines.fleets);
            pass.set_vertex_buffer(0, self.fleet_instances.slice(..));
            pass.draw(0..FLEET_VERTEX_COUNT, 0..fleet_count);
        }
        if halo_count > 0 {
            pass.set_pipeline(&pipelines.halos);
            pass.set_vertex_buffer(0, self.halo_instances.slice(..));
            pass.draw(0..HALO_VERTEX_COUNT, 0..halo_count);
        }
        if particle_count > 0 {
            pass.set_pipeline(&pipelines.particles);
            pass.set_vertex_buffer(0, self.particle_instances.slice(..));
            pass.draw(0..PARTICLE_VERTEX_COUNT, 0..particle_count);
        }
    }

    fn render_bloom(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bloom_view: &wgpu::TextureView,
        route_count: u32,
        fleet_count: u32,
        halo_count: u32,
        particle_count: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("half-resolution emissive extraction"),
            color_attachments: &[Some(color_attachment(
                bloom_view,
                None,
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            ))],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind_group, &[]);
        if route_count > 0 {
            pass.set_pipeline(&self.glow_routes);
            pass.set_vertex_buffer(0, self.route_instances.slice(..));
            pass.draw(0..ROUTE_VERTEX_COUNT, 0..route_count);
        }
        if fleet_count > 0 {
            pass.set_pipeline(&self.glow_fleets);
            pass.set_vertex_buffer(0, self.fleet_instances.slice(..));
            pass.draw(0..FLEET_VERTEX_COUNT, 0..fleet_count);
        }
        if halo_count > 0 {
            pass.set_pipeline(&self.glow_halos);
            pass.set_vertex_buffer(0, self.halo_instances.slice(..));
            pass.draw(0..HALO_VERTEX_COUNT, 0..halo_count);
        }
        if particle_count > 0 {
            pass.set_pipeline(&self.glow_particles);
            pass.set_vertex_buffer(0, self.particle_instances.slice(..));
            pass.draw(0..PARTICLE_VERTEX_COUNT, 0..particle_count);
        }
    }
}

fn render_background(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    pipelines: &ScenePipelines,
    globals: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("procedural space background"),
        color_attachments: &[Some(color_attachment(
            color_view,
            None,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        ))],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(&pipelines.background);
    pass.set_bind_group(0, globals, &[]);
    pass.draw(0..3, 0..1);
}

fn create_scene_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sample_count: u32,
    layout: &wgpu::PipelineLayout,
    space_shader: &wgpu::ShaderModule,
    worlds_shader: &wgpu::ShaderModule,
    routes_shader: &wgpu::ShaderModule,
) -> ScenePipelines {
    ScenePipelines {
        background: create_fullscreen_pipeline(
            device,
            format,
            sample_count,
            layout,
            space_shader,
            if sample_count == 4 {
                "high space background pipeline"
            } else {
                "low space background pipeline"
            },
            None,
        ),
        worlds: create_world_pipeline(device, format, sample_count, layout, worlds_shader),
        routes: create_transparent_pipeline(
            device,
            format,
            sample_count,
            Some(DEPTH_FORMAT),
            layout,
            routes_shader,
            "route_vs",
            "route_fs",
            &[Some(RouteInstance::LAYOUT)],
            wgpu::PrimitiveTopology::LineList,
            "tactical route pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        ),
        fleets: create_transparent_pipeline(
            device,
            format,
            sample_count,
            Some(DEPTH_FORMAT),
            layout,
            routes_shader,
            "fleet_vs",
            "fleet_fs",
            &[Some(FleetInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "tactical fleet pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        ),
        halos: create_transparent_pipeline(
            device,
            format,
            sample_count,
            Some(DEPTH_FORMAT),
            layout,
            routes_shader,
            "halo_vs",
            "halo_fs",
            &[Some(HaloInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "faction halo pipeline",
            additive_blend(),
        ),
        particles: create_transparent_pipeline(
            device,
            format,
            sample_count,
            Some(DEPTH_FORMAT),
            layout,
            routes_shader,
            "particle_vs",
            "particle_fs",
            &[Some(ParticleInstance::LAYOUT)],
            wgpu::PrimitiveTopology::TriangleList,
            "route particle pipeline",
            additive_blend(),
        ),
    }
}

fn create_fullscreen_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sample_count: u32,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &'static str,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: vertex_state(shader, "vs_main", &[]),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_world_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sample_count: u32,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
) -> wgpu::RenderPipeline {
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("instanced procedural world pipeline"),
        layout: Some(layout),
        vertex: vertex_state(
            shader,
            "vs_main",
            &[Some(MeshVertex::LAYOUT), Some(WorldInstance::LAYOUT)],
        ),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: Some(depth_state(true)),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn create_transparent_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    sample_count: u32,
    depth_format: Option<wgpu::TextureFormat>,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    vertex_entry: &'static str,
    fragment_entry: &'static str,
    buffers: &[Option<wgpu::VertexBufferLayout<'static>>],
    topology: wgpu::PrimitiveTopology,
    label: &'static str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let targets = [Some(wgpu::ColorTargetState {
        format,
        blend: Some(blend),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: vertex_state(shader, vertex_entry, buffers),
        primitive: wgpu::PrimitiveState {
            topology,
            ..Default::default()
        },
        depth_stencil: depth_format.map(|_| depth_state(false)),
        multisample: wgpu::MultisampleState {
            count: sample_count,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fragment_entry),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn vertex_state<'a>(
    shader: &'a wgpu::ShaderModule,
    entry_point: &'static str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'static>>],
) -> wgpu::VertexState<'a> {
    wgpu::VertexState {
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        buffers,
    }
}

fn depth_state(write_enabled: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: Some(write_enabled),
        depth_compare: Some(if write_enabled {
            wgpu::CompareFunction::Less
        } else {
            wgpu::CompareFunction::LessEqual
        }),
        stencil: Default::default(),
        bias: Default::default(),
    }
}

fn additive_blend() -> wgpu::BlendState {
    let component = wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

fn supports_high_resources(
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
) -> bool {
    let surface = adapter.get_texture_format_features(surface_format);
    let depth = adapter.get_texture_format_features(DEPTH_FORMAT);
    let bloom = adapter.get_texture_format_features(BLOOM_FORMAT);
    let bloom_usages =
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    let bloom_flags =
        wgpu::TextureFormatFeatureFlags::FILTERABLE | wgpu::TextureFormatFeatureFlags::BLENDABLE;

    device.limits().max_texture_dimension_2d > 0
        && surface
            .allowed_usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        && surface.flags.sample_count_supported(4)
        && surface
            .flags
            .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE)
        && depth.flags.sample_count_supported(4)
        && bloom.allowed_usages.contains(bloom_usages)
        && bloom.flags.contains(bloom_flags)
}

fn create_surface_targets(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    physical_size: [u32; 2],
    quality: ResolvedGraphicsQuality,
    bloom_layout: &wgpu::BindGroupLayout,
    bloom_sampler: &wgpu::Sampler,
) -> SurfaceTargets {
    let extent = wgpu::Extent3d {
        width: physical_size[0],
        height: physical_size[1],
        depth_or_array_layers: 1,
    };
    let depth_texture = texture(
        device,
        "tactical depth",
        extent,
        DEPTH_FORMAT,
        quality.sample_count,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let (msaa_texture, msaa_view) = if quality.sample_count > 1 {
        let texture = texture(
            device,
            "tactical multisample color",
            extent,
            surface_format,
            quality.sample_count,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (Some(texture), Some(view))
    } else {
        (None, None)
    };
    let (bloom_texture, bloom_view, bloom_bind_group) = if quality.bloom {
        let bloom_extent = wgpu::Extent3d {
            width: (physical_size[0] / 2).max(1),
            height: (physical_size[1] / 2).max(1),
            depth_or_array_layers: 1,
        };
        let texture = texture(
            device,
            "half-resolution bloom",
            bloom_extent,
            BLOOM_FORMAT,
            1,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom texture bind group"),
            layout: bloom_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: BLOOM_TEXTURE_BINDING,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: BLOOM_SAMPLER_BINDING,
                    resource: wgpu::BindingResource::Sampler(bloom_sampler),
                },
            ],
        });
        (Some(texture), Some(view), Some(bind_group))
    } else {
        (None, None, None)
    };
    SurfaceTargets {
        physical_size,
        sample_count: quality.sample_count,
        _depth_texture: depth_texture,
        depth_view,
        _msaa_texture: msaa_texture,
        msaa_view,
        _bloom_texture: bloom_texture,
        bloom_view,
        bloom_bind_group,
    }
}

#[derive(Debug, thiserror::Error)]
enum OptionalResourceError {
    #[error("optional {label} acquisition failed while polling the device: {diagnostic}")]
    DevicePoll {
        label: &'static str,
        diagnostic: String,
    },
    #[error("optional {label} acquisition reported a {scope} error: {diagnostic}")]
    Scope {
        label: &'static str,
        scope: &'static str,
        diagnostic: String,
    },
    #[error("optional {label} acquisition did not resolve its {scope} error scope")]
    Pending {
        label: &'static str,
        scope: &'static str,
    },
}

fn try_optional_high_resources<T>(
    device: &wgpu::Device,
    label: &'static str,
    create: impl FnOnce() -> T,
) -> Result<T, OptionalResourceError> {
    let validation_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let out_of_memory_scope = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let internal_scope = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let resources = create();
    let mut internal = Box::pin(internal_scope.pop());
    let mut out_of_memory = Box::pin(out_of_memory_scope.pop());
    let mut validation = Box::pin(validation_scope.pop());

    #[cfg(not(target_arch = "wasm32"))]
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_millis(250)),
        })
        .map_err(|error| OptionalResourceError::DevicePoll {
            label,
            diagnostic: error.to_string(),
        })?;
    #[cfg(target_arch = "wasm32")]
    device
        .poll(wgpu::PollType::Poll)
        .map_err(|error| OptionalResourceError::DevicePoll {
            label,
            diagnostic: error.to_string(),
        })?;

    poll_error_scope(internal.as_mut(), label, "internal")?;
    poll_error_scope(out_of_memory.as_mut(), label, "out-of-memory")?;
    poll_error_scope(validation.as_mut(), label, "validation")?;
    Ok(resources)
}

fn poll_error_scope<F>(
    future: Pin<&mut F>,
    label: &'static str,
    scope: &'static str,
) -> Result<(), OptionalResourceError>
where
    F: Future<Output = Option<wgpu::Error>>,
{
    let waker = Waker::noop();
    match future.poll(&mut Context::from_waker(waker)) {
        Poll::Ready(None) => Ok(()),
        Poll::Ready(Some(error)) => Err(OptionalResourceError::Scope {
            label,
            scope,
            diagnostic: error.to_string(),
        }),
        Poll::Pending => Err(OptionalResourceError::Pending { label, scope }),
    }
}

fn inverse_projection_aspect(projection: glam::Mat4) -> f32 {
    let x_scale = projection.x_axis.x.abs();
    let y_scale = projection.y_axis.y.abs();
    if x_scale.is_finite() && y_scale.is_finite() && y_scale > f32::EPSILON {
        (x_scale / y_scale).clamp(0.01, 100.0)
    } else {
        1.0
    }
}

fn shader_module(
    device: &wgpu::Device,
    label: &'static str,
    source: &'static str,
) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}

fn instance_buffer<T: Pod>(
    device: &wgpu::Device,
    count: usize,
    label: &'static str,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: (size_of::<T>() * count) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn write_instances<T: Pod>(queue: &wgpu::Queue, buffer: &wgpu::Buffer, instances: &[T]) {
    if !instances.is_empty() {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(instances));
    }
}

fn check_capacity(
    kind: &'static str,
    actual: usize,
    maximum: usize,
) -> Result<(), SceneRendererError> {
    if actual > maximum {
        Err(SceneRendererError::InstanceCapacity {
            kind,
            actual,
            maximum,
        })
    } else {
        Ok(())
    }
}

const fn requires_surface_targets(size: [u32; 2]) -> bool {
    size[0] > 0 && size[1] > 0
}

fn texture(
    device: &wgpu::Device,
    label: &'static str,
    size: wgpu::Extent3d,
    format: wgpu::TextureFormat,
    sample_count: u32,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn color_attachment<'a>(
    view: &'a wgpu::TextureView,
    resolve_target: Option<&'a wgpu::TextureView>,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPassColorAttachment<'a> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target,
        ops: wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        },
    }
}

fn depth_attachment(
    view: &wgpu::TextureView,
    load: wgpu::LoadOp<f32>,
) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load,
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_sized_targets_never_require_surface_dependent_resources() {
        assert!(!requires_surface_targets([0, 900]));
        assert!(!requires_surface_targets([1440, 0]));
        assert!(requires_surface_targets([1, 1]));
    }

    #[test]
    fn scene_globals_match_the_wgsl_uniform_contract() {
        assert_eq!(size_of::<SceneGlobals>(), 112);
        assert_eq!(size_of::<glam::Mat4>(), 64);
    }

    #[test]
    fn billboard_aspect_uses_logical_projection_not_physical_target_size() {
        for logical_viewport in [[1440.0_f32, 900.0_f32], [960.0, 600.0]] {
            let aspect = logical_viewport[0] / logical_viewport[1];
            let projection = glam::camera::rh::proj::directx::perspective(
                55.0_f32.to_radians(),
                aspect,
                0.1,
                80.0,
            );
            let correction = inverse_projection_aspect(projection);
            assert!((correction - logical_viewport[1] / logical_viewport[0]).abs() <= 0.000_001);
        }
    }
}
