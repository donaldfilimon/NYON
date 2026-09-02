use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};

use crate::{
    game::model::{Faction, FieldKind, HazardKind},
    presentation::{SceneFleet, SceneFrame, SceneRoute, SceneWorld},
};

pub const SPHERE_LATITUDE_SEGMENTS: u32 = 24;
pub const SPHERE_LONGITUDE_SEGMENTS: u32 = 48;
pub const SPHERE_VERTEX_COUNT: usize =
    ((SPHERE_LATITUDE_SEGMENTS + 1) * (SPHERE_LONGITUDE_SEGMENTS + 1)) as usize;
pub const SPHERE_INDEX_COUNT: usize =
    (SPHERE_LATITUDE_SEGMENTS * SPHERE_LONGITUDE_SEGMENTS * 6) as usize;
pub const ROUTE_VERTEX_COUNT: u32 = 64;
pub const FLEET_VERTEX_COUNT: u32 = 6;
pub const HALO_VERTEX_COUNT: u32 = 6;
pub const PARTICLE_VERTEX_COUNT: u32 = 6;
pub const MAX_ROUTE_PARTICLES: usize = 512;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl MeshVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct WorldInstance {
    pub model_0: [f32; 4],
    pub model_1: [f32; 4],
    pub model_2: [f32; 4],
    pub model_3: [f32; 4],
    pub color: [f32; 4],
    pub status: [f32; 4],
    pub fields: [f32; 4],
    pub metadata: [u32; 4],
}

impl WorldInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32x4,
            5 => Float32x4,
            6 => Float32x4,
            7 => Float32x4,
            8 => Float32x4,
            9 => Uint32x4
        ],
    };

    pub fn from_scene(world: &SceneWorld, visual_time: f32) -> Self {
        let rotation = Quat::from_rotation_y(visual_time * 0.08 + f32::from(world.id.0) * 0.47);
        let model = Mat4::from_scale_rotation_translation(
            Vec3::splat(world.radius),
            rotation,
            world.position,
        );
        let columns = model.to_cols_array_2d();
        Self {
            model_0: columns[0],
            model_1: columns[1],
            model_2: columns[2],
            model_3: columns[3],
            color: world.ownership_color,
            status: [
                world.energy,
                world.defense,
                if world.selected { 1.0 } else { 0.0 },
                if world.hovered { 1.0 } else { 0.0 },
            ],
            fields: [
                world.fields[0],
                world.fields[1],
                world.fields[2],
                field_parameter(world.hazard_field),
            ],
            metadata: [
                u32::from(world.id.0),
                world.ownership_pattern,
                hazard_parameter(world.hazard),
                0,
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct RouteInstance {
    pub start: [f32; 4],
    pub control: [f32; 4],
    pub end: [f32; 4],
    pub color: [f32; 4],
}

impl RouteInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![
            0 => Float32x4,
            1 => Float32x4,
            2 => Float32x4,
            3 => Float32x4
        ],
    };

    pub fn from_scene(route: &SceneRoute) -> Self {
        Self {
            start: route.start.extend(1.0).to_array(),
            control: route.control.extend(1.0).to_array(),
            end: route.end.extend(1.0).to_array(),
            color: faction_color(route.faction, 0.58),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FleetInstance {
    pub position_size: [f32; 4],
    pub color: [f32; 4],
}

impl FleetInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
    };

    pub fn from_scene(fleet: &SceneFleet) -> Self {
        Self {
            position_size: fleet
                .position
                .extend(0.0075 + fleet.strength * 0.0085)
                .to_array(),
            color: faction_color(fleet.faction, 0.92),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct HaloInstance {
    pub position_radius: [f32; 4],
    pub color: [f32; 4],
}

impl HaloInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
    };

    fn from_scene(world: &SceneWorld, visual_time: f32, animate: bool) -> Self {
        let phase = visual_time * 1.35 + f32::from(world.id.0) * 0.83;
        let pulse = if animate { phase.sin() * 0.0025 } else { 0.0 };
        let mut color = world.ownership_color;
        color[3] = if world.selected {
            0.72
        } else if world.hovered {
            0.52
        } else {
            0.32
        };
        Self {
            position_radius: world
                .position
                .extend(0.023 + world.radius * 0.018 + pulse)
                .to_array(),
            color,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ParticleInstance {
    pub position_size: [f32; 4],
    pub color: [f32; 4],
}

impl ParticleInstance {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
    };

    fn from_route(route: &SceneRoute, t: f32) -> Self {
        Self {
            position_size: quadratic_bezier(route.start, route.control, route.end, t)
                .extend(0.0028)
                .to_array(),
            color: faction_color(route.faction, 0.56),
        }
    }
}

const _: () = assert!(size_of::<MeshVertex>() == 24);
const _: () = assert!(size_of::<WorldInstance>() == 128);
const _: () = assert!(size_of::<RouteInstance>() == 64);
const _: () = assert!(size_of::<FleetInstance>() == 32);
const _: () = assert!(size_of::<HaloInstance>() == 32);
const _: () = assert!(size_of::<ParticleInstance>() == 32);

pub struct SphereMeshData {
    pub vertices: Vec<MeshVertex>,
    pub indices: Vec<u32>,
}

pub fn generate_sphere_mesh() -> SphereMeshData {
    let mut vertices = Vec::with_capacity(SPHERE_VERTEX_COUNT);
    for latitude in 0..=SPHERE_LATITUDE_SEGMENTS {
        let v = latitude as f32 / SPHERE_LATITUDE_SEGMENTS as f32;
        let phi = v * std::f32::consts::PI;
        let y = phi.cos();
        let ring = phi.sin();
        for longitude in 0..=SPHERE_LONGITUDE_SEGMENTS {
            let u = longitude as f32 / SPHERE_LONGITUDE_SEGMENTS as f32;
            let theta = u * std::f32::consts::TAU;
            let normal = Vec3::new(ring * theta.cos(), y, ring * theta.sin());
            vertices.push(MeshVertex {
                position: normal.to_array(),
                normal: normal.to_array(),
            });
        }
    }

    let row = SPHERE_LONGITUDE_SEGMENTS + 1;
    let mut indices = Vec::with_capacity(SPHERE_INDEX_COUNT);
    for latitude in 0..SPHERE_LATITUDE_SEGMENTS {
        for longitude in 0..SPHERE_LONGITUDE_SEGMENTS {
            let top_left = latitude * row + longitude;
            let bottom_left = (latitude + 1) * row + longitude;
            indices.extend_from_slice(&[
                top_left,
                bottom_left,
                top_left + 1,
                top_left + 1,
                bottom_left,
                bottom_left + 1,
            ]);
        }
    }
    SphereMeshData { vertices, indices }
}

pub struct SceneUploads {
    pub worlds: Vec<WorldInstance>,
    pub routes: Vec<RouteInstance>,
    pub fleets: Vec<FleetInstance>,
    pub halos: Vec<HaloInstance>,
    pub particles: Vec<ParticleInstance>,
}

impl SceneUploads {
    pub fn from_frame(frame: &SceneFrame, particle_divisor: u32) -> Self {
        let mut routes = frame.routes.iter().collect::<Vec<_>>();
        routes.sort_by(|left, right| {
            camera_depth(frame.camera.view, route_midpoint(right))
                .total_cmp(&camera_depth(frame.camera.view, route_midpoint(left)))
                .then_with(|| route_key(left).cmp(&route_key(right)))
        });
        let mut fleets = frame.fleets.iter().collect::<Vec<_>>();
        fleets.sort_by(|left, right| {
            camera_depth(frame.camera.view, right.position)
                .total_cmp(&camera_depth(frame.camera.view, left.position))
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut particle_routes = frame.routes.iter().collect::<Vec<_>>();
        particle_routes.sort_by_key(|route| route_key(route));
        let particle_budget =
            usize::try_from(frame.effects.particle_budget / particle_divisor.max(1))
                .unwrap_or(MAX_ROUTE_PARTICLES)
                .min(MAX_ROUTE_PARTICLES);
        let particles = route_particles(&particle_routes, particle_budget, frame.visual_time);

        Self {
            worlds: frame
                .worlds
                .iter()
                .map(|world| WorldInstance::from_scene(world, frame.visual_time))
                .collect(),
            routes: routes.into_iter().map(RouteInstance::from_scene).collect(),
            fleets: fleets.into_iter().map(FleetInstance::from_scene).collect(),
            halos: frame
                .worlds
                .iter()
                .map(|world| {
                    HaloInstance::from_scene(world, frame.visual_time, frame.effects.animate_halos)
                })
                .collect(),
            particles,
        }
    }
}

fn route_particles(
    routes: &[&SceneRoute],
    particle_budget: usize,
    visual_time: f32,
) -> Vec<ParticleInstance> {
    if routes.is_empty() || particle_budget == 0 {
        return Vec::new();
    }

    let particles_per_route = particle_budget.div_ceil(routes.len()).max(1);
    (0..particle_budget)
        .map(|particle_index| {
            let route = routes[particle_index % routes.len()];
            let lane = particle_index / routes.len();
            let key = route_key(route);
            let offset =
                f32::from(key.0) * 0.173 + f32::from(key.1) * 0.097 + f32::from(key.2) * 0.211;
            let t =
                ((lane as f32 + 0.5) / particles_per_route as f32 + visual_time * 0.065 + offset)
                    .fract();
            ParticleInstance::from_route(route, t)
        })
        .collect()
}

fn route_midpoint(route: &SceneRoute) -> Vec3 {
    quadratic_bezier(route.start, route.control, route.end, 0.5)
}

fn quadratic_bezier(start: Vec3, control: Vec3, end: Vec3, t: f32) -> Vec3 {
    let one_minus = 1.0 - t;
    start * one_minus * one_minus + control * (2.0 * one_minus * t) + end * t * t
}

fn camera_depth(view: Mat4, position: Vec3) -> f32 {
    let depth = -view.transform_point3(position).z;
    if depth.is_finite() { depth } else { 0.0 }
}

fn route_key(route: &SceneRoute) -> (u8, u8, u8) {
    (
        route.source.0,
        route.destination.0,
        faction_tag(route.faction),
    )
}

const fn faction_color(faction: Faction, alpha: f32) -> [f32; 4] {
    match faction {
        Faction::Union => [0.12, 0.82, 1.0, alpha],
        Faction::Helix => [1.0, 0.28, 0.12, alpha],
        Faction::Choir => [0.76, 0.32, 1.0, alpha],
    }
}

const fn faction_tag(faction: Faction) -> u8 {
    match faction {
        Faction::Union => 0,
        Faction::Helix => 1,
        Faction::Choir => 2,
    }
}

const fn hazard_parameter(hazard: Option<HazardKind>) -> u32 {
    match hazard {
        None => 0,
        Some(HazardKind::IonStorm) => 1,
        Some(HazardKind::GravityTide) => 2,
    }
}

const fn field_parameter(field: Option<FieldKind>) -> f32 {
    match field {
        None => 0.0,
        Some(FieldKind::Atmosphere) => 0.33,
        Some(FieldKind::Hydrosphere) => 0.66,
        Some(FieldKind::Topology) => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use std::mem::offset_of;

    use glam::Vec2;

    use crate::{
        engine::quality::resolve_quality,
        game::model::{FleetId, WorldId},
        presentation::{
            CameraMatrices, GraphicsQuality, SceneBackground, SceneEffects, SceneFleet, SceneFrame,
            SceneRoute, SceneWorld,
        },
    };

    use super::*;

    #[test]
    fn sphere_mesh_has_the_frozen_tessellation_and_unit_normals() {
        let mesh = generate_sphere_mesh();
        assert_eq!(mesh.vertices.len(), SPHERE_VERTEX_COUNT);
        assert_eq!(mesh.indices.len(), SPHERE_INDEX_COUNT);
        assert!(
            mesh.indices
                .iter()
                .all(|index| (*index as usize) < mesh.vertices.len())
        );
        assert!(
            mesh.vertices
                .iter()
                .all(|vertex| { (Vec3::from_array(vertex.normal).length() - 1.0).abs() <= 0.0001 })
        );
    }

    #[test]
    fn three_dimensional_abis_have_explicit_offsets_and_do_not_alias_primitives() {
        assert_eq!(size_of::<MeshVertex>(), 24);
        assert_eq!(offset_of!(MeshVertex, normal), 12);
        assert_eq!(size_of::<WorldInstance>(), 128);
        assert_eq!(offset_of!(WorldInstance, color), 64);
        assert_eq!(offset_of!(WorldInstance, status), 80);
        assert_eq!(offset_of!(WorldInstance, fields), 96);
        assert_eq!(offset_of!(WorldInstance, metadata), 112);
        assert_eq!(size_of::<RouteInstance>(), 64);
        assert_eq!(offset_of!(RouteInstance, color), 48);
        assert_eq!(size_of::<FleetInstance>(), 32);
        assert_eq!(size_of::<HaloInstance>(), 32);
        assert_eq!(size_of::<ParticleInstance>(), 32);
        assert_eq!(size_of::<crate::engine::primitives::Vertex>(), 36);
    }

    #[test]
    fn transparent_instances_sort_back_to_front_with_canonical_ties() {
        let mut frame = test_frame();
        frame.routes = vec![
            route(WorldId(2), WorldId(1), -1.0),
            route(WorldId(1), WorldId(2), -5.0),
            route(WorldId(0), WorldId(2), -5.0),
        ];
        frame.fleets = vec![
            fleet(FleetId(3), -1.0),
            fleet(FleetId(2), -5.0),
            fleet(FleetId(1), -5.0),
        ];

        let uploads = SceneUploads::from_frame(&frame, 1);
        let route_starts = uploads
            .routes
            .iter()
            .map(|route| route.start[0])
            .collect::<Vec<_>>();
        assert_eq!(route_starts, vec![0.0, 1.0, 2.0]);
        let fleet_x = uploads
            .fleets
            .iter()
            .map(|fleet| fleet.position_size[0])
            .collect::<Vec<_>>();
        assert_eq!(fleet_x, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn quality_and_motion_bound_presentation_only_effect_instances() {
        let mut frame = test_frame();
        frame.routes = vec![route(WorldId(0), WorldId(1), -2.0)];
        frame.worlds = vec![world(WorldId(0))];
        frame.effects.particle_budget = 20;

        let high_quality = resolve_quality(GraphicsQuality::High, true);
        let low_quality = resolve_quality(GraphicsQuality::Low, true);
        let high = SceneUploads::from_frame(&frame, high_quality.particle_divisor);
        let low = SceneUploads::from_frame(&frame, low_quality.particle_divisor);
        assert_eq!(high.particles.len(), 20);
        assert_eq!(low.particles.len(), 5);
        assert_eq!(high.halos.len(), 1);
        assert_ne!(high.halos[0].position_radius[3], 0.0302);

        frame.effects.particle_budget = 0;
        frame.effects.animate_halos = false;
        let reduced = SceneUploads::from_frame(&frame, 1);
        assert!(reduced.particles.is_empty());
        assert_eq!(reduced.halos.len(), 1);
        assert!((reduced.halos[0].position_radius[3] - 0.0302).abs() <= f32::EPSILON);

        frame.effects.particle_budget = u32::MAX;
        assert_eq!(
            SceneUploads::from_frame(&frame, 1).particles.len(),
            MAX_ROUTE_PARTICLES
        );
    }

    fn test_frame() -> SceneFrame {
        SceneFrame {
            worlds: Vec::new(),
            routes: Vec::new(),
            fleets: Vec::new(),
            background: SceneBackground {
                visual_seed: 0,
                parallax: Vec2::ZERO,
                static_background: false,
            },
            effects: SceneEffects {
                particle_budget: 0,
                animate_halos: true,
                bloom_requested: true,
                high_contrast: false,
            },
            advisory: None,
            camera: CameraMatrices {
                view: Mat4::IDENTITY,
                projection: Mat4::IDENTITY,
                view_projection: Mat4::IDENTITY,
                inverse_view_projection: Mat4::IDENTITY,
            },
            visual_time: 3.0,
        }
    }

    fn route(source: WorldId, destination: WorldId, z: f32) -> SceneRoute {
        SceneRoute {
            source,
            destination,
            faction: Faction::Union,
            start: Vec3::new(f32::from(source.0), 0.0, z),
            control: Vec3::new(f32::from(source.0), 0.5, z),
            end: Vec3::new(f32::from(source.0), 0.0, z),
        }
    }

    fn fleet(id: FleetId, z: f32) -> SceneFleet {
        SceneFleet {
            id,
            faction: Faction::Union,
            source: WorldId(0),
            destination: WorldId(1),
            position: Vec3::new(id.0 as f32, 0.0, z),
            strength: 0.5,
            progress: 0.5,
        }
    }

    fn world(id: WorldId) -> SceneWorld {
        SceneWorld {
            id,
            position: Vec3::ZERO,
            radius: 0.4,
            ownership_color: [0.12, 0.82, 1.0, 1.0],
            ownership_pattern: 0,
            fields: [0.0; 3],
            energy: 0.5,
            defense: 0.5,
            hovered: false,
            selected: false,
            hazard: None,
            hazard_field: None,
        }
    }
}
