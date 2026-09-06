//! One-way presentation extraction for Galaxy Workshop.
//!
//! Everything in this module is derived from an immutable authoritative state.
//! Display floats, interpolation, colors, labels, and semantic bounds never
//! flow back into `nyon-workshop-core`.

use std::mem::{align_of, offset_of, size_of};

use nyon_workshop_core::{
    EntityId, StateDigest, WorkshopStateV1, WorkshopTick,
    model::{IndustryV1, LaneV1, ShipmentV1, WorldV1},
};

const UNITS_PER_PARSEC: f64 = 1_024.0;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorkshopSystemInstance {
    pub position_radius: [f32; 4],
    pub color: [f32; 4],
    pub metadata: [u32; 4],
}

impl WorkshopSystemInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Uint32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

/// Presentation-only star marker data.
///
/// The complete entity identifier is retained in `metadata`; selection lives
/// in a separate flags lane so no identifier bits are sacrificed to UI state.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorkshopStarInstance {
    pub position_radius: [f32; 4],
    pub color: [f32; 4],
    pub metadata: [u32; 4],
    pub flags: [u32; 4],
}

impl WorkshopStarInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Uint32x4,
        3 => Uint32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorkshopWorldInstance {
    pub position_radius: [f32; 4],
    pub color: [f32; 4],
    pub orbit: [f32; 4],
    pub metadata: [u32; 4],
}

impl WorkshopWorldInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
        3 => Uint32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorkshopLaneInstance {
    pub from_to_x: [f32; 4],
    pub from_to_y: [f32; 4],
    pub color_width: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorkshopShipmentInstance {
    pub position_units: [f32; 4],
    pub color: [f32; 4],
}

const _: [(); 48] = [(); size_of::<WorkshopSystemInstance>()];
const _: [(); 4] = [(); align_of::<WorkshopSystemInstance>()];
const _: [(); 16] = [(); offset_of!(WorkshopSystemInstance, color)];
const _: [(); 32] = [(); offset_of!(WorkshopSystemInstance, metadata)];
const _: [(); 64] = [(); size_of::<WorkshopStarInstance>()];
const _: [(); 4] = [(); align_of::<WorkshopStarInstance>()];
const _: [(); 16] = [(); offset_of!(WorkshopStarInstance, color)];
const _: [(); 32] = [(); offset_of!(WorkshopStarInstance, metadata)];
const _: [(); 48] = [(); offset_of!(WorkshopStarInstance, flags)];
const _: [(); 64] = [(); size_of::<WorkshopWorldInstance>()];
const _: [(); 16] = [(); offset_of!(WorkshopWorldInstance, color)];
const _: [(); 32] = [(); offset_of!(WorkshopWorldInstance, orbit)];
const _: [(); 48] = [(); offset_of!(WorkshopWorldInstance, metadata)];
const _: [(); 48] = [(); size_of::<WorkshopLaneInstance>()];
const _: [(); 32] = [(); size_of::<WorkshopShipmentInstance>()];

#[derive(Clone, Debug, PartialEq)]
pub struct WorkshopSemanticEntry {
    pub entity: EntityId,
    pub role: WorkshopSemanticRole,
    pub label: String,
    pub detail: String,
    pub bounds: [f32; 4],
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopSemanticRole {
    System,
    Star,
    World,
    Industry,
    Lane,
    Route,
    Hazard,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkshopSceneFrame {
    pub tick: WorkshopTick,
    pub state_digest: StateDigest,
    pub systems: Vec<WorkshopSystemInstance>,
    pub stars: Vec<WorkshopStarInstance>,
    pub worlds: Vec<WorkshopWorldInstance>,
    pub lanes: Vec<WorkshopLaneInstance>,
    pub shipments: Vec<WorkshopShipmentInstance>,
    pub semantic_entries: Vec<WorkshopSemanticEntry>,
    pub production_status: Vec<String>,
    pub hazard_status: Vec<String>,
}

pub fn build_workshop_scene_frame(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    high_contrast: bool,
) -> WorkshopSceneFrame {
    let mut systems = Vec::with_capacity(state.systems.len());
    let mut stars = Vec::with_capacity(state.stars.len());
    let mut worlds = Vec::with_capacity(state.worlds.len());
    let mut lanes = Vec::with_capacity(state.lanes.len());
    let mut shipments = Vec::with_capacity(state.shipments.len());
    let mut semantic_entries = Vec::with_capacity(
        state.systems.len() + state.stars.len() + state.worlds.len() + state.industries.len(),
    );

    for (id, system) in &state.systems {
        let position = galaxy_position(system.position.x.get(), system.position.y.get());
        let color = ownership_color(state, *id, high_contrast);
        systems.push(WorkshopSystemInstance {
            position_radius: [position[0], position[1], 0.0, 0.12],
            color,
            metadata: entity_metadata(*id, selected == Some(*id)),
        });
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::System,
            label: system.name.to_string(),
            detail: format!(
                "System at ({}, {}) galaxy units",
                system.position.x.get(),
                system.position.y.get()
            ),
            bounds: centered_bounds(position, 24.0),
            selected: selected == Some(*id),
        });
    }

    let mut sorted_stars = state.stars.iter().collect::<Vec<_>>();
    sorted_stars.sort_unstable_by_key(|(id, _)| **id);
    for (id, star) in sorted_stars {
        let position = state
            .systems
            .get(&star.system)
            .map(|system| galaxy_position(system.position.x.get(), system.position.y.get()))
            .unwrap_or([0.0, 0.0]);
        let is_selected = selected == Some(*id);
        stars.push(WorkshopStarInstance {
            position_radius: [position[0], position[1], 0.0, 0.075],
            color: star_color(star.archetype_id.as_str(), high_contrast),
            metadata: entity_words(*id),
            flags: [u32::from(is_selected), 0, 0, 0],
        });
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::Star,
            label: star.name.to_string(),
            detail: format!("{} star", star.archetype_id),
            bounds: centered_bounds(position, 18.0),
            selected: is_selected,
        });
    }

    for (id, world) in &state.worlds {
        let (position, orbit) = world_position(state, world);
        let color = ownership_color(state, *id, high_contrast);
        worlds.push(WorkshopWorldInstance {
            position_radius: [position[0], position[1], 0.0, 0.05],
            color,
            orbit,
            metadata: entity_metadata(*id, selected == Some(*id)),
        });
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::World,
            label: world.name.to_string(),
            detail: format!(
                "{} world, orbit {} milli-AU",
                world.archetype_id, world.orbit_radius_milli_au
            ),
            bounds: centered_bounds(position, 16.0),
            selected: selected == Some(*id),
        });
    }

    for (id, lane) in &state.lanes {
        if let Some(instance) = lane_instance(state, lane, high_contrast) {
            lanes.push(instance);
            semantic_entries.push(WorkshopSemanticEntry {
                entity: *id,
                role: WorkshopSemanticRole::Lane,
                label: "Inter-system lane".to_owned(),
                detail: format!("{} distance units", lane.distance_units),
                bounds: lane_bounds(instance),
                selected: selected == Some(*id),
            });
        }
    }

    for shipment in state.shipments.values() {
        if let Some(instance) = shipment_instance(state, shipment) {
            shipments.push(instance);
        }
    }

    for (id, route) in &state.routes {
        let source_name = state
            .worlds
            .get(&route.source)
            .map_or("unknown world", |world| world.name.as_str());
        let destination_name = state
            .worlds
            .get(&route.destination)
            .map_or("unknown world", |world| world.name.as_str());
        let position = state
            .worlds
            .get(&route.source)
            .map(|world| world_position(state, world).0)
            .unwrap_or([0.0, 0.0]);
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::Route,
            label: format!("{} route", route.resource_id),
            detail: format!(
                "{source_name} to {destination_name}, {} units per dispatch",
                route.batch_units
            ),
            bounds: centered_bounds(position, 14.0),
            selected: selected == Some(*id),
        });
    }

    let mut production_status = Vec::with_capacity(state.industries.len());
    for (id, industry) in &state.industries {
        production_status.push(industry_status(*id, industry));
        let world_name = state
            .worlds
            .get(&industry.world)
            .map_or("unknown world", |world| world.name.as_str());
        let position = state
            .worlds
            .get(&industry.world)
            .map(|world| world_position(state, world).0)
            .unwrap_or([0.0, 0.0]);
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::Industry,
            label: industry.definition_id.to_string(),
            detail: format!(
                "{} on {world_name}",
                if industry.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                }
            ),
            bounds: centered_bounds(position, 14.0),
            selected: selected == Some(*id),
        });
    }

    let mut hazard_status = Vec::with_capacity(state.hazards.len());
    for (id, hazard) in &state.hazards {
        let end = hazard.start_tick.0.saturating_add(hazard.duration_ticks);
        let active = !hazard.cancelled && hazard.start_tick.0 <= state.tick.0 && state.tick.0 < end;
        let status = if hazard.cancelled {
            "CANCELLED"
        } else if active {
            "ACTIVE"
        } else if state.tick.0 >= end {
            "EXPIRED"
        } else {
            "SCHEDULED"
        };
        hazard_status.push(format!(
            "{} {} [{}..{})",
            hazard.hazard_id, status, hazard.start_tick.0, end
        ));
        semantic_entries.push(WorkshopSemanticEntry {
            entity: *id,
            role: WorkshopSemanticRole::Hazard,
            label: hazard.hazard_id.to_string(),
            detail: format!("{status} logistics hazard"),
            bounds: [0.0; 4],
            selected: selected == Some(*id),
        });
    }

    WorkshopSceneFrame {
        tick: state.tick,
        state_digest: state.digest(),
        systems,
        stars,
        worlds,
        lanes,
        shipments,
        semantic_entries,
        production_status,
        hazard_status,
    }
}

fn galaxy_position(x: i64, y: i64) -> [f32; 2] {
    [
        (x as f64 / UNITS_PER_PARSEC) as f32,
        (y as f64 / UNITS_PER_PARSEC) as f32,
    ]
}

fn world_position(state: &WorkshopStateV1, world: &WorldV1) -> ([f32; 2], [f32; 4]) {
    let center = state
        .systems
        .get(&world.system)
        .map(|system| galaxy_position(system.position.x.get(), system.position.y.get()))
        .unwrap_or([0.0, 0.0]);
    let period = world.orbit_period_ticks.max(1);
    let phase_turns = f64::from(world.phase_millidegrees) / 360_000.0;
    let tick_turns = (state.tick.0 % period) as f64 / period as f64;
    let angle = (phase_turns + tick_turns) * std::f64::consts::TAU;
    let (sin, cos) = angle.sin_cos();
    let radius = f64::from(world.orbit_radius_milli_au) * 0.000_001;
    let position = [
        center[0] + (cos * radius) as f32,
        center[1] + (sin * radius) as f32,
    ];
    (
        position,
        [center[0], center[1], radius as f32, angle as f32],
    )
}

fn lane_instance(
    state: &WorkshopStateV1,
    lane: &LaneV1,
    high_contrast: bool,
) -> Option<WorkshopLaneInstance> {
    let a = state.systems.get(&lane.a)?;
    let b = state.systems.get(&lane.b)?;
    let from = galaxy_position(a.position.x.get(), a.position.y.get());
    let to = galaxy_position(b.position.x.get(), b.position.y.get());
    Some(WorkshopLaneInstance {
        from_to_x: [from[0], to[0], 0.0, 0.0],
        from_to_y: [from[1], to[1], 0.0, 0.0],
        color_width: if high_contrast {
            [1.0, 1.0, 1.0, 2.0]
        } else {
            [0.22, 0.68, 0.92, 1.25]
        },
    })
}

fn lane_bounds(instance: WorkshopLaneInstance) -> [f32; 4] {
    let min_x = instance.from_to_x[0].min(instance.from_to_x[1]);
    let max_x = instance.from_to_x[0].max(instance.from_to_x[1]);
    let min_y = instance.from_to_y[0].min(instance.from_to_y[1]);
    let max_y = instance.from_to_y[0].max(instance.from_to_y[1]);
    [min_x, min_y, max_x - min_x, max_y - min_y]
}

fn shipment_instance(
    state: &WorkshopStateV1,
    shipment: &ShipmentV1,
) -> Option<WorkshopShipmentInstance> {
    let route = state.routes.get(&shipment.route)?;
    let source_world = state.worlds.get(&route.source)?;
    let destination_world = state.worlds.get(&route.destination)?;
    let from = world_position(state, source_world).0;
    let to = world_position(state, destination_world).0;
    let span = shipment
        .arrival_tick
        .0
        .saturating_sub(shipment.dispatched_tick.0)
        .max(1);
    let elapsed = state
        .tick
        .0
        .saturating_sub(shipment.dispatched_tick.0)
        .min(span);
    let progress = elapsed as f32 / span as f32;
    let position = [
        from[0] + (to[0] - from[0]) * progress,
        from[1] + (to[1] - from[1]) * progress,
    ];
    Some(WorkshopShipmentInstance {
        position_units: [position[0], position[1], shipment.units as f32, progress],
        color: [0.95, 0.72, 0.20, 1.0],
    })
}

fn ownership_color(state: &WorkshopStateV1, entity: EntityId, high_contrast: bool) -> [f32; 4] {
    let fallback = if high_contrast {
        [0.90, 0.90, 0.90, 1.0]
    } else {
        [0.38, 0.72, 0.94, 1.0]
    };
    state
        .owners
        .get(&entity)
        .and_then(|faction| state.factions.get(faction))
        .map_or(fallback, |faction| {
            let [red, green, blue] = faction.color_rgb;
            [
                f32::from(red) / 255.0,
                f32::from(green) / 255.0,
                f32::from(blue) / 255.0,
                1.0,
            ]
        })
}

fn star_color(archetype_id: &str, high_contrast: bool) -> [f32; 4] {
    if high_contrast {
        return [1.0, 1.0, 0.82, 1.0];
    }
    match archetype_id {
        "red-dwarf" => [1.0, 0.34, 0.18, 1.0],
        "blue-giant" => [0.42, 0.78, 1.0, 1.0],
        "white-dwarf" => [0.82, 0.92, 1.0, 1.0],
        _ => [1.0, 0.74, 0.24, 1.0],
    }
}

fn entity_words(id: EntityId) -> [u32; 4] {
    let bytes = id.0;
    [
        u32::from_le_bytes(bytes[0..4].try_into().expect("four bytes")),
        u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes")),
        u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes")),
        u32::from_le_bytes(bytes[12..16].try_into().expect("four bytes")),
    ]
}

fn entity_metadata(id: EntityId, selected: bool) -> [u32; 4] {
    let bytes = id.0;
    [
        u32::from_le_bytes(bytes[0..4].try_into().expect("four bytes")),
        u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes")),
        u32::from_le_bytes(bytes[8..12].try_into().expect("four bytes")),
        u32::from(selected),
    ]
}

fn centered_bounds(position: [f32; 2], extent: f32) -> [f32; 4] {
    [
        position[0] - extent * 0.5,
        position[1] - extent * 0.5,
        extent,
        extent,
    ]
}

fn industry_status(id: EntityId, industry: &IndustryV1) -> String {
    format!(
        "{:02X}{:02X}{:02X}{:02X} {} {}",
        id.0[0],
        id.0[1],
        id.0[2],
        id.0[3],
        industry.definition_id,
        if industry.enabled {
            "ENABLED"
        } else {
            "DISABLED"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_layouts_match_the_declared_shader_contract() {
        assert_eq!(size_of::<WorkshopSystemInstance>(), 48);
        assert_eq!(offset_of!(WorkshopSystemInstance, color), 16);
        assert_eq!(offset_of!(WorkshopSystemInstance, metadata), 32);
        assert_eq!(WorkshopSystemInstance::LAYOUT.array_stride, 48);
        assert_eq!(size_of::<WorkshopStarInstance>(), 64);
        assert_eq!(offset_of!(WorkshopStarInstance, color), 16);
        assert_eq!(offset_of!(WorkshopStarInstance, metadata), 32);
        assert_eq!(offset_of!(WorkshopStarInstance, flags), 48);
        assert_eq!(WorkshopStarInstance::LAYOUT.array_stride, 64);
        assert_eq!(size_of::<WorkshopWorldInstance>(), 64);
        assert_eq!(offset_of!(WorkshopWorldInstance, orbit), 32);
        assert_eq!(offset_of!(WorkshopWorldInstance, metadata), 48);
        assert_eq!(WorkshopWorldInstance::LAYOUT.array_stride, 64);
    }

    #[test]
    fn extraction_is_ordered_and_cannot_mutate_authority() {
        let state = WorkshopStateV1::default();
        let before = state.digest();
        let first = build_workshop_scene_frame(&state, None, false);
        let second = build_workshop_scene_frame(&state, None, false);
        assert_eq!(first, second);
        assert_eq!(state.digest(), before);
    }
}
