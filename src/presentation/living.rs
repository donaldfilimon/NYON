//! Read-only Living projection. Geometry is in logical pixels and never enters
//! the authority or its digest.

use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;
use nyon_workshop_core::living::{
    LivingEntityIdV2, LivingFleetLocationV2, LivingGalaxyStateV2, LivingResourceV2,
    LivingShipmentDispositionV2, LivingTickV2,
};

use crate::living_camera::{LivingCameraLevel, LivingCameraScope, LivingCameraState};
use crate::living_navigation::{LivingEntityKind, LivingEntitySelection, navigator_rows};
use crate::{engine::primitives::PrimitiveBatch, ui::platform::PlatformRect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingWorldAppearance {
    Temperate,
    Arid,
    Ice,
    Gas,
    Rocky,
}
impl LivingWorldAppearance {
    pub fn for_archetype(archetype: &str) -> Self {
        match archetype {
            "temperate" | "ocean" => Self::Temperate,
            "arid" | "barren" => Self::Arid,
            "ice" => Self::Ice,
            "gas" | "gas_giant" => Self::Gas,
            _ => Self::Rocky,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct LivingWorldMarker {
    pub id: LivingEntityIdV2,
    pub name: String,
    pub position: Vec2,
    pub color: [f32; 4],
    pub appearance: LivingWorldAppearance,
    pub surface_seed: u64,
    pub owner: Option<LivingEntityIdV2>,
    pub completed_buildings: usize,
    pub occupied: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingFreightVisual {
    pub resource: LivingResourceV2,
    pub units: u32,
    pub progress_percent: u8,
    pub eta_ticks: u64,
    pub waiting_for_storage: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LivingEntityMarker {
    pub selection: LivingEntitySelection,
    pub name: String,
    pub position: Vec2,
    pub visible: bool,
    pub archetype: Option<String>,
    pub owner: Option<LivingEntityIdV2>,
    pub hull_count: usize,
    pub blocked: bool,
    pub direction: Vec2,
    pub freight_visual: Option<LivingFreightVisual>,
}

/// Arbitrary supported entity counts; Classic's fixed scene is independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LivingGhostKind {
    Added,
    Removed,
    Changed,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LivingGhostMarker {
    pub selection: LivingEntitySelection,
    pub position: Vec2,
    pub kind: LivingGhostKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LivingSceneLabel {
    pub selection: LivingEntitySelection,
    pub baseline: Vec2,
    pub bounds: PlatformRect,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LivingSceneFrame {
    pub bounds: Option<PlatformRect>,
    pub ghosts: Vec<LivingGhostMarker>,
    pub selected_world: Option<LivingEntityIdV2>,
    pub selected_entity: Option<LivingEntitySelection>,
    pub entities: Vec<LivingEntityMarker>,
    pub worlds: Vec<LivingWorldMarker>,
    pub systems: Vec<Vec2>,
    pub lanes: Vec<(Vec2, Vec2, bool)>,
    pub fleets: Vec<Vec2>,
    pub freight: Vec<Vec2>,
    pub scope: Option<LivingCameraLevel>,
}

impl LivingSceneFrame {
    /// Fit static members of the selected scope; remote cargo and travellers
    /// never pull a system/world camera back to the whole galaxy.
    pub fn fitted_camera(
        state: &LivingGalaxyStateV2,
        bounds: PlatformRect,
        scope: LivingCameraScope,
    ) -> LivingCameraState {
        let frame = Self::extract(state, bounds);
        let rows = navigator_rows(state);
        let parents = rows
            .iter()
            .map(|row| (row.selection, row.parent))
            .collect::<BTreeMap<_, _>>();
        let root = scope.entity.map(|id| {
            LivingEntitySelection::new(
                match scope.level {
                    LivingCameraLevel::System => LivingEntityKind::System,
                    _ => LivingEntityKind::World,
                },
                id,
            )
        });
        let members = rows
            .iter()
            .filter(|row| {
                if matches!(
                    row.selection.kind,
                    LivingEntityKind::Fleet
                        | LivingEntityKind::Hull
                        | LivingEntityKind::Shipment
                        | LivingEntityKind::Civilization
                        | LivingEntityKind::Lane
                        | LivingEntityKind::Hazard
                ) {
                    return false;
                }
                if scope.level == LivingCameraLevel::Galaxy {
                    return true;
                }
                let mut key = Some(row.selection);
                while let Some(selected) = key {
                    if Some(selected) == root {
                        return true;
                    }
                    key = parents.get(&selected).copied().flatten();
                }
                false
            })
            .map(|row| row.selection)
            .collect::<BTreeSet<_>>();
        let points = frame
            .entities
            .iter()
            .filter(|marker| members.contains(&marker.selection))
            .map(|marker| marker.position)
            .collect::<Vec<_>>();
        let Some(first) = points.first().copied() else {
            return LivingCameraState::default();
        };
        let mut min = first;
        let mut max = first;
        for point in points {
            min = min.min(point);
            max = max.max(point);
        }
        let base = match scope.level {
            LivingCameraLevel::Galaxy => 1.0,
            LivingCameraLevel::System => 3.0,
            LivingCameraLevel::World => 8.0,
        };
        let origin = root
            .and_then(|selection| {
                frame
                    .entities
                    .iter()
                    .find(|marker| marker.selection == selection)
                    .map(|marker| marker.position)
            })
            .unwrap_or((bounds.min + bounds.max) * 0.5);
        let available = (bounds.max - bounds.min - Vec2::splat(112.0)).max(Vec2::splat(1.0));
        // Scoped views keep their system/world at the canvas center. Fit the
        // farthest static member on either side rather than translating the target.
        let span = if root.is_some() {
            (max - origin).abs().max((min - origin).abs()) * 2.0
        } else {
            max - min
        };
        let extent = (span * base).max(Vec2::splat(1.0));
        let zoom = (available.x / extent.x)
            .min(available.y / extent.y)
            .clamp(0.25, 8.0);
        LivingCameraState {
            zoom,
            pan: if root.is_some() {
                Vec2::ZERO
            } else {
                (-(min + max) * 0.5 + origin)
                    .mul_add(Vec2::splat(base * zoom), Vec2::ZERO)
                    .clamp(Vec2::splat(-4096.0), Vec2::splat(4096.0))
            },
            ..Default::default()
        }
    }

    pub fn extract(state: &LivingGalaxyStateV2, bounds: PlatformRect) -> Self {
        let mut frame = Self {
            bounds: Some(bounds),
            ..Self::default()
        };
        let mut system_positions = BTreeMap::new();
        let mut world_positions = BTreeMap::new();
        let mut fleet_positions = BTreeMap::new();
        let mut shipment_positions = BTreeMap::new();
        let center = (bounds.min + bounds.max) * 0.5;
        let half = ((bounds.max - bounds.min) * 0.5 - Vec2::splat(48.0)).max(Vec2::ZERO);
        let mut completed_buildings = BTreeMap::new();
        for facility in state.facilities.as_slice() {
            *completed_buildings.entry(facility.world).or_insert(0usize) += 1;
        }
        let facility_by_id: BTreeMap<_, _> = state
            .facilities
            .as_slice()
            .iter()
            .map(|row| (row.id, row))
            .collect();
        let fleet_by_id: BTreeMap<_, _> = state
            .fleets
            .as_slice()
            .iter()
            .map(|row| (row.id, row))
            .collect();
        let shipment_by_id: BTreeMap<_, _> = state
            .shipments
            .as_slice()
            .iter()
            .map(|row| (row.id, row))
            .collect();
        let star_by_id: BTreeMap<_, _> = state
            .stars
            .as_slice()
            .iter()
            .map(|row| (row.id, row))
            .collect();
        let world_by_id: BTreeMap<_, _> = state
            .worlds
            .as_slice()
            .iter()
            .map(|row| (row.id, row))
            .collect();
        let systems = state.systems.as_slice();
        for system in systems {
            // Placement belongs to presentation, keyed by identity so a creator
            // insertion or removal never reframes the remaining objects.
            let position = center + stable_offset(system.id) * half * 0.72;
            system_positions.insert(system.id, position);
            frame.systems.push(position);
            let worlds = state
                .worlds
                .as_slice()
                .iter()
                .filter(|world| world.system == system.id)
                .collect::<Vec<_>>();
            for world in &worlds {
                let angle = (presentation_key(world.id) as u32) as f32 / u32::MAX as f32
                    * std::f32::consts::TAU;
                let orbit = (half.x.min(half.y) * 0.22).clamp(0.0, 42.0);
                let position = position + Vec2::new(angle.cos(), angle.sin()) * orbit;
                world_positions.insert(world.id, position);
                let color = match world.owner.map(|owner| presentation_key(owner) % 4) {
                    Some(0) => [0.46, 0.63, 0.84, 1.0],
                    Some(1) => [0.77, 0.59, 0.39, 1.0],
                    Some(2) => [0.65, 0.88, 0.48, 1.0],
                    Some(_) => [0.83, 0.58, 0.94, 1.0],
                    None => [0.65, 0.68, 0.76, 1.0],
                };
                frame.worlds.push(LivingWorldMarker {
                    id: world.id,
                    name: world.name.as_str().to_owned(),
                    position,
                    color,
                    appearance: LivingWorldAppearance::for_archetype(world.archetype.as_str()),
                    surface_seed: presentation_key(world.id),
                    owner: world.owner,
                    occupied: state
                        .occupations
                        .as_slice()
                        .iter()
                        .any(|row| row.world == world.id),
                    completed_buildings: completed_buildings.get(&world.id).copied().unwrap_or(0),
                });
            }
        }
        for lane in state.lanes.as_slice() {
            if let (Some(from), Some(to)) = (
                system_positions.get(&lane.system_a),
                system_positions.get(&lane.system_b),
            ) {
                let hazardous = state.hazards.as_slice().iter().any(|hazard| {
                    hazard.lane == lane.id
                        && hazard.start_tick <= state.tick
                        && state.tick < hazard.end_tick
                });
                frame.lanes.push((*from, *to, hazardous));
            }
        }
        for fleet in state.fleets.as_slice() {
            let position = match fleet.location {
                LivingFleetLocationV2::Docked { world } => world_positions.get(&world).copied(),
                LivingFleetLocationV2::Travelling {
                    origin_world,
                    destination_world,
                    departure_tick,
                    arrival_tick,
                } => transit(
                    &world_positions,
                    origin_world,
                    destination_world,
                    departure_tick,
                    arrival_tick,
                    state.tick,
                ),
            };
            if let Some(position) = position {
                frame.fleets.push(position);
                fleet_positions.insert(fleet.id, position);
            }
        }
        for shipment in state.shipments.as_slice() {
            let returning = shipment.disposition == LivingShipmentDispositionV2::Returning
                || shipment.returning_to_source;
            let (origin, destination) = if returning {
                (shipment.destination_world, shipment.source_world)
            } else {
                (shipment.source_world, shipment.destination_world)
            };
            if let Some(position) = transit(
                &world_positions,
                origin,
                destination,
                shipment.departure_tick,
                shipment.arrival_tick,
                state.tick,
            ) {
                frame.freight.push(position);
                shipment_positions.insert(shipment.id, position);
            }
        }
        let lane_positions: BTreeMap<_, _> = state
            .lanes
            .as_slice()
            .iter()
            .filter_map(|lane| {
                Some((
                    lane.id,
                    (*system_positions.get(&lane.system_a)?
                        + *system_positions.get(&lane.system_b)?)
                        * 0.5,
                ))
            })
            .collect();
        for row in navigator_rows(state) {
            use LivingEntityKind as K;
            let position = match row.selection.kind {
                K::System => system_positions.get(&row.selection.id).copied(),
                K::Star => state
                    .stars
                    .as_slice()
                    .iter()
                    .find(|star| star.id == row.selection.id)
                    .and_then(|star| system_positions.get(&star.system))
                    .map(|point| *point + stable_offset(row.selection.id) * 16.0),
                K::World => world_positions.get(&row.selection.id).copied(),
                K::Lane => lane_positions.get(&row.selection.id).copied(),
                K::Hazard => state
                    .hazards
                    .as_slice()
                    .iter()
                    .find(|hazard| hazard.id == row.selection.id)
                    .and_then(|hazard| lane_positions.get(&hazard.lane))
                    .map(|point| *point + Vec2::new(0.0, -14.0)),
                K::Fleet => fleet_positions
                    .get(&row.selection.id)
                    .map(|point| *point + Vec2::new(0.0, -18.0)),
                K::Shipment => shipment_positions
                    .get(&row.selection.id)
                    .map(|point| *point + Vec2::new(0.0, 18.0)),
                _ => row
                    .world
                    .and_then(|world| world_positions.get(&world))
                    .map(|point| *point + stable_offset(row.selection.id) * 6.0),
            };
            if let Some(position) = position {
                frame.entities.push(LivingEntityMarker {
                    archetype: match row.selection.kind {
                        K::Star => star_by_id
                            .get(&row.selection.id)
                            .map(|row| row.archetype.as_str().to_owned()),
                        K::Fleet => fleet_by_id
                            .get(&row.selection.id)
                            .and_then(|row| row.hulls.as_slice().first())
                            .map(|row| row.definition.as_str().to_owned()),
                        K::Facility => facility_by_id
                            .get(&row.selection.id)
                            .map(|row| row.definition.as_str().to_owned()),
                        _ => None,
                    },
                    owner: match row.selection.kind {
                        K::Fleet => fleet_by_id.get(&row.selection.id).map(|row| row.owner),
                        K::Civilization => Some(row.selection.id),
                        _ => row
                            .world
                            .and_then(|id| world_by_id.get(&id))
                            .and_then(|row| row.owner),
                    },
                    hull_count: fleet_by_id
                        .get(&row.selection.id)
                        .map_or(0, |row| row.hulls.as_slice().len()),
                    blocked: facility_by_id.get(&row.selection.id).is_some_and(|row| {
                        row.disabled || !row.blocked_reasons.as_slice().is_empty()
                    }),
                    direction: if let Some(shipment) = shipment_by_id.get(&row.selection.id) {
                        let returning = shipment.disposition
                            == LivingShipmentDispositionV2::Returning
                            || shipment.returning_to_source;
                        let (origin, destination) = if returning {
                            (shipment.destination_world, shipment.source_world)
                        } else {
                            (shipment.source_world, shipment.destination_world)
                        };
                        match (
                            world_positions.get(&origin),
                            world_positions.get(&destination),
                        ) {
                            (Some(from), Some(to)) => (*to - *from).normalize_or_zero(),
                            _ => Vec2::X,
                        }
                    } else {
                        Vec2::X
                    },
                    freight_visual: shipment_by_id.get(&row.selection.id).map(|shipment| {
                        let duration = shipment
                            .arrival_tick
                            .0
                            .saturating_sub(shipment.departure_tick.0)
                            .max(1);
                        let elapsed = state
                            .tick
                            .0
                            .saturating_sub(shipment.departure_tick.0)
                            .min(duration);
                        LivingFreightVisual {
                            resource: shipment.resource,
                            units: shipment.units,
                            progress_percent: ((u128::from(elapsed) * 100) / u128::from(duration))
                                as u8,
                            eta_ticks: shipment.arrival_tick.0.saturating_sub(state.tick.0),
                            waiting_for_storage: shipment.disposition
                                == LivingShipmentDispositionV2::WaitingForStorage,
                        }
                    }),
                    name: row.label.clone(),
                    selection: row.selection,
                    position,
                    visible: matches!(
                        row.selection.kind,
                        K::System
                            | K::Star
                            | K::World
                            | K::Lane
                            | K::Fleet
                            | K::Shipment
                            | K::Hazard
                    ),
                });
            }
        }
        frame
    }

    /// Compatibility world focus; camera transforms never enter authority.
    pub fn focus(&mut self, bounds: PlatformRect, world: LivingEntityIdV2, zoom: f32) {
        let Some(origin) = self
            .worlds
            .iter()
            .find(|row| row.id == world)
            .map(|row| row.position)
        else {
            return;
        };
        let center = (bounds.min + bounds.max) * 0.5;
        self.project_geometry(
            bounds,
            |point| center + (point - origin) * zoom,
            zoom >= 8.0,
        );
    }
    pub fn apply_camera(
        &mut self,
        bounds: PlatformRect,
        scope: LivingCameraScope,
        camera: LivingCameraState,
    ) {
        self.bounds = Some(bounds);
        self.scope = Some(scope.level);
        let center = (bounds.min + bounds.max) * 0.5;
        let default_target = scope.entity.map(|id| {
            LivingEntitySelection::new(
                match scope.level {
                    LivingCameraLevel::System => LivingEntityKind::System,
                    _ => LivingEntityKind::World,
                },
                id,
            )
        });
        let locate = |selection| {
            self.entities
                .iter()
                .find(|marker| marker.selection == selection)
                .map(|marker| marker.position)
        };
        let origin = camera
            .focus_target
            .and_then(locate)
            .or_else(|| default_target.and_then(locate))
            .unwrap_or(center);
        let base = match scope.level {
            LivingCameraLevel::Galaxy => 1.0,
            LivingCameraLevel::System => 3.0,
            LivingCameraLevel::World => 8.0,
        };
        let angle = camera.yaw_degrees.to_radians();
        let (sine, cosine) = angle.sin_cos();
        let pitch = camera.pitch_degrees.to_radians().sin() / 55.0f32.to_radians().sin();
        self.project_geometry(
            bounds,
            |point| {
                let offset = point - origin;
                let rotated = Vec2::new(
                    offset.x * cosine - offset.y * sine,
                    (offset.x * sine + offset.y * cosine) * pitch,
                );
                center + camera.pan + rotated * (base * camera.zoom)
            },
            scope.level == LivingCameraLevel::World,
        );
    }
    fn project_geometry(
        &mut self,
        bounds: PlatformRect,
        project: impl Fn(Vec2) -> Vec2,
        show_details: bool,
    ) {
        let inside = |point: Vec2| {
            point.x >= bounds.min.x + 22.0
                && point.y >= bounds.min.y + 22.0
                && point.x <= bounds.max.x - 22.0
                && point.y <= bounds.max.y - 22.0
        };
        for marker in &mut self.entities {
            let offset = match marker.selection.kind {
                LivingEntityKind::Fleet => Vec2::new(0.0, -18.0),
                LivingEntityKind::Shipment => Vec2::new(0.0, 18.0),
                _ => Vec2::ZERO,
            };
            let base = marker.position - offset;
            marker.direction =
                (project(base + marker.direction) - project(base)).normalize_or_zero();
            marker.position = project(base) + offset;
            if show_details || self.selected_entity == Some(marker.selection) {
                marker.visible = true;
            }
        }
        self.entities.retain(|marker| inside(marker.position));
        for row in &mut self.worlds {
            row.position = project(row.position);
        }
        self.worlds.retain(|row| inside(row.position));
        for rows in [&mut self.systems, &mut self.fleets, &mut self.freight] {
            for point in rows.iter_mut() {
                *point = project(*point);
            }
            rows.retain(|point| inside(*point));
        }
        for (from, to, _) in &mut self.lanes {
            *from = project(*from);
            *to = project(*to);
        }
        self.lanes
            .retain(|(from, to, _)| inside(*from) && inside(*to));
    }

    pub fn hit_entity(&self, point: Vec2) -> Option<LivingEntitySelection> {
        self.entities
            .iter()
            .filter(|marker| {
                marker.visible && marker.position.distance_squared(point) <= 22.0 * 22.0
            })
            .min_by(|left, right| {
                left.position
                    .distance_squared(point)
                    .total_cmp(&right.position.distance_squared(point))
                    .then_with(|| left.selection.cmp(&right.selection))
            })
            .map(|marker| marker.selection)
    }

    pub fn hit_world(&self, point: Vec2) -> Option<LivingEntityIdV2> {
        self.worlds
            .iter()
            .filter(|world| world.position.distance_squared(point) <= 22.0 * 22.0)
            .min_by(|left, right| {
                left.position
                    .distance_squared(point)
                    .total_cmp(&right.position.distance_squared(point))
            })
            .map(|world| world.id)
    }

    /// Compatibility projection using the same measured placement as rendering.
    pub fn labels(&self) -> Vec<(LivingEntitySelection, Vec2, String)> {
        let metrics = crate::ui::AtlasMetrics::embedded().expect("embedded atlas metrics");
        self.measured_labels(&metrics)
            .into_iter()
            .map(|label| {
                (
                    label.selection,
                    Vec2::new(
                        (label.bounds.min.x + label.bounds.max.x) * 0.5,
                        label.bounds.min.y,
                    ),
                    label.text,
                )
            })
            .collect()
    }

    /// Place complete SDF glyph quads once, in selection-first order. Nearby
    /// alternatives avoid both accepted labels and visible marker bodies.
    pub fn measured_labels(&self, metrics: &crate::ui::AtlasMetrics) -> Vec<LivingSceneLabel> {
        self.measured_labels_avoiding(metrics, &[])
    }

    /// Use the actual installed renderer records, including status/readout text.
    pub fn measured_labels_for_ui(
        &self,
        metrics: &crate::ui::AtlasMetrics,
        frame: &crate::ui::platform::PlatformUiFrame,
    ) -> Vec<LivingSceneLabel> {
        let mut exclusions: Vec<_> = frame
            .controls
            .iter()
            .map(|control| control.bounds)
            .chain(frame.sighted_text.iter().map(|text| text.bounds))
            .chain(frame.visible_nodes.iter().map(|record| record.bounds))
            .collect();
        exclusions.extend(frame.layout.chrome.iter().copied());
        exclusions.extend(frame.drawer);
        self.measured_labels_avoiding(metrics, &exclusions)
    }

    /// Re-anchor around actual UI chrome; omit a label when every nearby anchor
    /// is occluded. Entity navigation remains available independently.
    pub fn measured_labels_avoiding(
        &self,
        metrics: &crate::ui::AtlasMetrics,
        exclusions: &[PlatformRect],
    ) -> Vec<LivingSceneLabel> {
        let Some(canvas) = self.bounds else {
            return Vec::new();
        };
        let mut candidates = self
            .entities
            .iter()
            .filter(|marker| {
                marker.visible
                    && (self.selected_entity == Some(marker.selection)
                        || matches!(
                            marker.selection.kind,
                            LivingEntityKind::System | LivingEntityKind::World
                        ))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|marker| {
            (
                self.selected_entity != Some(marker.selection),
                marker.selection.kind != LivingEntityKind::System,
                marker.selection,
            )
        });
        let radius = |marker: &LivingEntityMarker| match marker.selection.kind {
            LivingEntityKind::World
                if self.scope == Some(LivingCameraLevel::World)
                    && self.selected_world == Some(marker.selection.id) =>
            {
                36.0
            }
            LivingEntityKind::World | LivingEntityKind::Fleet => 16.0,
            _ => 12.0,
        };
        let bodies = self
            .entities
            .iter()
            .filter(|marker| marker.visible)
            .map(|marker| {
                let r = radius(marker);
                PlatformRect::from_xywh(
                    marker.position.x - r,
                    marker.position.y - r,
                    r * 2.0,
                    r * 2.0,
                )
            })
            .collect::<Vec<_>>();
        let mut occupied = Vec::<PlatformRect>::new();
        let mut labels = Vec::new();
        for marker in candidates {
            let label = if let Some(freight) = marker.freight_visual {
                if freight.waiting_for_storage {
                    format!(
                        "Freight: waiting for storage | {} {}",
                        freight.units,
                        freight.resource.as_wire_str()
                    )
                } else {
                    format!(
                        "Freight {}% | ETA {} ticks | {} {}",
                        freight.progress_percent,
                        freight.eta_ticks,
                        freight.units,
                        freight.resource.as_wire_str()
                    )
                }
            } else {
                marker.name.clone()
            };
            let Ok(boundaries) =
                metrics.measured_boundaries(13.0, crate::ui::FontWeight::Regular, &label)
            else {
                continue;
            };
            let mut offsets = label
                .char_indices()
                .zip(boundaries.iter())
                .filter_map(|((_, character), (_, offset))| (character != ' ').then_some(*offset));
            let Some(first) = offsets.next() else {
                continue;
            };
            let last = offsets.last().unwrap_or(first);
            let scale = 13.0 / metrics.generator.font_px;
            let cell = metrics.generator.cell_size as f32 * scale;
            let local_min = Vec2::new(
                first - metrics.generator.font_origin_x as f32 * scale,
                -metrics.generator.font_baseline_y as f32 * scale,
            );
            let size = Vec2::new(last - first + cell, cell);
            let gap = radius(marker) + 4.0;
            let p = marker.position;
            let anchors = [
                p + Vec2::new(-size.x / 2.0, gap),
                p + Vec2::new(-size.x / 2.0, -gap - size.y),
                p + Vec2::new(gap, -size.y / 2.0),
                p + Vec2::new(-gap - size.x, -size.y / 2.0),
                p + Vec2::new(gap, gap),
                p + Vec2::new(-gap - size.x, gap),
                p + Vec2::new(gap, -gap - size.y),
                p + Vec2::new(-gap - size.x, -gap - size.y),
            ];
            for anchor in anchors {
                let bounds = PlatformRect::from_xywh(anchor.x, anchor.y, size.x, size.y);
                if !bounds.min.is_finite()
                    || !bounds.max.is_finite()
                    || bounds.min.cmplt(canvas.min).any()
                    || bounds.max.cmpgt(canvas.max).any()
                    || occupied
                        .iter()
                        .chain(&bodies)
                        .chain(exclusions)
                        .any(|other| bounds.intersection(*other).is_some())
                {
                    continue;
                }
                occupied.push(bounds);
                labels.push(LivingSceneLabel {
                    selection: marker.selection,
                    baseline: anchor - local_min,
                    bounds,
                    text: label,
                });
                break;
            }
        }
        labels
    }

    pub fn draw(&self, batch: &mut PrimitiveBatch, high_contrast: bool) {
        for (from, to, hazardous) in &self.lanes {
            batch.line(
                *from,
                *to,
                if high_contrast { 3.0 } else { 2.0 },
                if *hazardous {
                    [1.0, 0.70, 0.28, 1.0]
                } else {
                    [0.35, 0.47, 0.64, 1.0]
                },
            );
            if *hazardous {
                for segment in 0..8 {
                    let start = segment as f32 / 8.0;
                    batch.line(
                        from.lerp(*to, start),
                        from.lerp(*to, start + 0.055),
                        if high_contrast { 5.0 } else { 4.0 },
                        [0.15, 0.20, 0.30, 1.0],
                    );
                }
            }
        }
        for position in &self.systems {
            let frame_color = if high_contrast {
                [0.70, 0.76, 0.86, 1.0]
            } else {
                [0.28, 0.36, 0.50, 0.8]
            };
            batch.ring(
                *position,
                34.0,
                if high_contrast { 2.0 } else { 1.0 },
                frame_color,
            );
            if self.scope != Some(LivingCameraLevel::Galaxy) {
                for radius in [54.0, 76.0] {
                    batch.ring(*position, radius, 1.0, [0.28, 0.36, 0.50, 0.5]);
                }
            }
        }
        for world in &self.worlds {
            let focused = self.scope == Some(LivingCameraLevel::World)
                && self.selected_world == Some(world.id);
            let radius = if focused { 32.0 } else { 12.0 };
            draw_world(batch, world, radius, high_contrast);
            if world.occupied {
                for index in 0..4 {
                    let angle = index as f32 * std::f32::consts::FRAC_PI_2;
                    let radial = Vec2::new(angle.cos(), angle.sin());
                    let side = Vec2::new(-radial.y, radial.x);
                    let center = world.position + radial * (radius + 6.0);
                    batch.line(
                        center - side * 4.0,
                        center + side * 4.0,
                        3.0,
                        [0.96, 0.38, 0.38, 1.0],
                    );
                }
            }
            if let Some(owner) = world.owner {
                batch.ring(
                    world.position,
                    radius + 3.0,
                    if high_contrast { 3.0 } else { 2.0 },
                    world.color,
                );
                draw_crest(
                    batch,
                    world.position + Vec2::new(radius + 8.0, 0.0),
                    owner,
                    world.color,
                );
            }
            if self.scope != Some(LivingCameraLevel::Galaxy) && world.completed_buildings > 0 {
                for index in 0..world.completed_buildings.min(6) {
                    batch.quad(
                        world.position + Vec2::new(-12.0 + index as f32 * 5.0, radius + 9.0),
                        Vec2::new(1.5, 2.0 + ((index % 2) as f32)),
                        [0.84, 0.87, 0.93, 1.0],
                    );
                }
            }
        }
        for marker in &self.entities {
            if !marker.visible {
                continue;
            }
            use LivingEntityKind as K;
            match marker.selection.kind {
                K::System | K::World => {}
                K::Fleet => {
                    draw_fleet(
                        batch,
                        marker.position,
                        marker.archetype.as_deref().unwrap_or("scout"),
                        high_contrast,
                    );
                    if let Some(owner) = marker.owner {
                        draw_crest(
                            batch,
                            marker.position + Vec2::new(-11.0, 0.0),
                            owner,
                            [0.65, 0.79, 0.92, 1.0],
                        );
                    }
                    for index in 0..marker.hull_count.min(16) {
                        batch.disc(
                            marker.position
                                + Vec2::new(
                                    -8.0 + (index % 8) as f32 * 2.0,
                                    9.0 + (index / 8) as f32 * 3.0,
                                ),
                            0.7,
                            [0.95, 0.95, 0.90, 1.0],
                        );
                    }
                }
                K::Shipment => {
                    let p = marker.position;
                    let direction = marker.direction;
                    let side = Vec2::new(-direction.y, direction.x);
                    let c = [0.65, 0.84, 0.92, 1.0];
                    batch.quad(p, Vec2::new(4.0, 3.0), c);
                    if let Some(freight) = marker.freight_visual {
                        let ink = [0.08, 0.15, 0.24, 1.0];
                        match freight.resource {
                            LivingResourceV2::Energy => {
                                batch.line(
                                    p + Vec2::new(1.0, -2.0),
                                    p + Vec2::new(-1.0, 0.0),
                                    1.0,
                                    ink,
                                );
                                batch.line(
                                    p + Vec2::new(-1.0, 0.0),
                                    p + Vec2::new(1.0, 0.0),
                                    1.0,
                                    ink,
                                );
                                batch.line(
                                    p + Vec2::new(1.0, 0.0),
                                    p + Vec2::new(-1.0, 2.0),
                                    1.0,
                                    ink,
                                );
                            }
                            LivingResourceV2::Ore => {
                                for (from, to) in [
                                    (Vec2::new(0.0, -2.0), Vec2::new(2.0, 0.0)),
                                    (Vec2::new(2.0, 0.0), Vec2::new(0.0, 2.0)),
                                    (Vec2::new(0.0, 2.0), Vec2::new(-2.0, 0.0)),
                                    (Vec2::new(-2.0, 0.0), Vec2::new(0.0, -2.0)),
                                ] {
                                    batch.line(p + from, p + to, 1.0, ink);
                                }
                            }
                            LivingResourceV2::Alloy => {
                                for y in [-1.0, 1.0] {
                                    batch.line(
                                        p + Vec2::new(-2.0, y),
                                        p + Vec2::new(2.0, y),
                                        1.0,
                                        ink,
                                    );
                                }
                            }
                        }
                        if self.selected_entity == Some(marker.selection) {
                            let from = p + Vec2::new(-10.0, 13.0);
                            let to = p + Vec2::new(10.0, 13.0);
                            batch.line(from, to, 3.0, [0.25, 0.33, 0.45, 1.0]);
                            batch.line(
                                from,
                                from.lerp(to, f32::from(freight.progress_percent) / 100.0),
                                3.0,
                                [0.32, 0.94, 1.0, 1.0],
                            );
                        }
                    }

                    batch.line(p - direction * 10.0, p - direction * 7.0, 2.0, c);
                    batch.line(
                        p + direction * 5.0 - side * 3.0,
                        p + direction * 9.0,
                        2.0,
                        c,
                    );
                    batch.line(
                        p + direction * 9.0,
                        p + direction * 5.0 + side * 3.0,
                        2.0,
                        c,
                    );
                }
                K::Star => {
                    let dwarf = marker.archetype.as_deref() == Some("red_dwarf");
                    let color = if dwarf {
                        [1.0, 0.57, 0.40, 1.0]
                    } else {
                        [1.0, 0.91, 0.70, 1.0]
                    };
                    batch.ring(
                        marker.position,
                        if dwarf { 7.0 } else { 10.0 },
                        2.0,
                        [color[0], color[1], color[2], 0.3],
                    );
                    batch.disc(marker.position, if dwarf { 3.0 } else { 5.0 }, color);
                    if !dwarf {
                        for delta in [Vec2::X, Vec2::Y] {
                            batch.line(
                                marker.position - delta * 8.0,
                                marker.position + delta * 8.0,
                                1.0,
                                color,
                            );
                        }
                    }
                }
                K::Hazard => {
                    let p = marker.position;
                    let c = [1.0, 0.70, 0.28, 1.0];
                    batch.ring(p, 8.0, 2.0, c);
                    batch.line(p + Vec2::new(-3.0, -5.0), p + Vec2::new(2.0, 0.0), 2.0, c);
                    batch.line(p + Vec2::new(2.0, 0.0), p + Vec2::new(-2.0, 5.0), 2.0, c);
                }
                K::Civilization => draw_crest(
                    batch,
                    marker.position,
                    marker.selection.id,
                    [0.67, 0.79, 0.95, 1.0],
                ),
                K::Facility => {
                    let p = marker.position;
                    let c = if marker.blocked {
                        [1.0, 0.70, 0.28, 1.0]
                    } else {
                        [0.64, 0.79, 0.80, 1.0]
                    };
                    match marker.archetype.as_deref() {
                        Some("solar_array") => {
                            batch.quad(p, Vec2::new(5.0, 3.0), c);
                            batch.line(
                                p - Vec2::new(0.0, 3.0),
                                p + Vec2::new(0.0, 3.0),
                                1.0,
                                [0.10, 0.15, 0.22, 1.0],
                            );
                        }
                        Some("defense_battery") => {
                            batch.ring(p, 5.0, 2.0, c);
                            batch.line(p, p + Vec2::new(6.0, -6.0), 2.0, c);
                        }
                        Some("shipyard") => {
                            batch.line(p + Vec2::new(-5.0, -5.0), p + Vec2::new(-5.0, 5.0), 2.0, c);
                            batch.line(p + Vec2::new(5.0, -5.0), p + Vec2::new(5.0, 5.0), 2.0, c);
                            batch.line(p + Vec2::new(-5.0, 5.0), p + Vec2::new(5.0, 5.0), 2.0, c);
                        }
                        Some("extractor") => {
                            batch.line(p + Vec2::new(-5.0, 4.0), p + Vec2::new(0.0, -5.0), 2.0, c);
                            batch.line(p + Vec2::new(0.0, -5.0), p + Vec2::new(5.0, 4.0), 2.0, c);
                            batch.line(p, p + Vec2::new(0.0, 6.0), 2.0, c);
                        }
                        _ => {
                            batch.quad(p, Vec2::new(4.0, 3.0), c);
                            batch.line(
                                p + Vec2::new(-3.0, -3.0),
                                p + Vec2::new(-3.0, -7.0),
                                2.0,
                                c,
                            );
                        }
                    }
                    if marker.blocked {
                        batch.line(
                            p - Vec2::splat(5.0),
                            p + Vec2::splat(5.0),
                            2.0,
                            [0.10, 0.15, 0.22, 1.0],
                        );
                    }
                }
                K::Lane => batch.disc(marker.position, 3.0, [0.45, 0.65, 0.90, 1.0]),
                _ => batch.quad(marker.position, Vec2::splat(4.0), [0.55, 0.90, 0.80, 1.0]),
            }
            if self.selected_entity == Some(marker.selection) {
                batch.ring(
                    marker.position,
                    if marker.selection.kind == K::World
                        && self.scope == Some(LivingCameraLevel::World)
                    {
                        40.0
                    } else {
                        22.0
                    },
                    2.0,
                    [0.32, 0.94, 1.0, 1.0],
                );
            }
        }
        for ghost in &self.ghosts {
            let color = match ghost.kind {
                LivingGhostKind::Added => [0.30, 1.0, 0.70, 0.75],
                LivingGhostKind::Removed => [1.0, 0.40, 0.40, 0.85],
                LivingGhostKind::Changed => [0.50, 0.85, 1.0, 0.75],
            };
            batch.ring(
                ghost.position,
                18.0,
                if high_contrast { 3.0 } else { 2.0 },
                color,
            );
            let p = ghost.position;
            match ghost.kind {
                LivingGhostKind::Removed => {
                    batch.line(p - Vec2::splat(9.0), p + Vec2::splat(9.0), 3.0, color);
                    batch.line(
                        p + Vec2::new(-9.0, 9.0),
                        p + Vec2::new(9.0, -9.0),
                        3.0,
                        color,
                    );
                }
                LivingGhostKind::Added => {
                    batch.line(p - Vec2::X * 8.0, p + Vec2::X * 8.0, 2.0, color);
                    batch.line(p - Vec2::Y * 8.0, p + Vec2::Y * 8.0, 2.0, color);
                }
                LivingGhostKind::Changed => {
                    batch.ring(p, 12.0, 1.5, color);
                }
            }
        }
    }
}

fn draw_world(batch: &mut PrimitiveBatch, world: &LivingWorldMarker, r: f32, contrast: bool) {
    use LivingWorldAppearance as A;
    let base = match world.appearance {
        A::Temperate => [0.19, 0.42, 0.59, 1.0],
        A::Arid => [0.57, 0.40, 0.27, 1.0],
        A::Ice => [0.69, 0.80, 0.87, 1.0],
        A::Gas => [0.51, 0.48, 0.66, 1.0],
        A::Rocky => [0.46, 0.49, 0.54, 1.0],
    };
    batch.disc(world.position, r, base);
    let accent = if contrast {
        [0.93, 0.94, 0.90, 1.0]
    } else {
        [base[0] + 0.14, base[1] + 0.14, base[2] + 0.10, 1.0]
    };
    let phase = (world.surface_seed % 360) as f32 * std::f32::consts::PI / 180.0;
    match world.appearance {
        A::Temperate => {
            for index in 0..3 {
                let angle = phase + index as f32 * 2.1;
                batch.disc(
                    world.position + Vec2::new(angle.cos(), angle.sin()) * r * 0.45,
                    r * 0.20,
                    [0.49, 0.65, 0.43, 1.0],
                );
            }
            batch.line(
                world.position + Vec2::new(-r * 0.7, r * 0.35),
                world.position + Vec2::new(r * 0.7, r * 0.35),
                1.5,
                accent,
            );
        }
        A::Gas => {
            for band in [-0.5, 0.0, 0.5] {
                let y = r * band;
                let x = (r * r - y * y).sqrt() * 0.85;
                batch.line(
                    world.position + Vec2::new(-x, y),
                    world.position + Vec2::new(x, y),
                    r * 0.18,
                    accent,
                );
            }
            batch.line(
                world.position + Vec2::new(-r * 1.5, r * 0.35),
                world.position + Vec2::new(r * 1.5, -r * 0.35),
                2.0,
                accent,
            );
        }
        A::Ice => {
            for index in 0..3 {
                let angle = phase + index as f32 * 2.1;
                batch.line(
                    world.position,
                    world.position + Vec2::new(angle.cos(), angle.sin()) * r * 0.85,
                    1.5,
                    [0.21, 0.39, 0.52, 1.0],
                );
            }
        }
        A::Arid | A::Rocky => {
            for index in 0..3 {
                let angle = phase + index as f32 * 2.1;
                batch.ring(
                    world.position + Vec2::new(angle.cos(), angle.sin()) * r * 0.45,
                    r * 0.16,
                    1.0,
                    accent,
                );
            }
        }
    }
    batch.ring(
        world.position,
        r,
        if contrast { 2.0 } else { 1.0 },
        [0.81, 0.86, 0.90, 1.0],
    );
}
fn draw_crest(batch: &mut PrimitiveBatch, p: Vec2, id: LivingEntityIdV2, color: [f32; 4]) {
    let pattern = presentation_key(id) % 4;
    if pattern.is_multiple_of(2) {
        batch.ring(p, 4.0, 1.5, color);
    } else {
        batch.quad(p, Vec2::splat(3.0), color);
    }
    if pattern >= 2 {
        batch.line(
            p - Vec2::new(4.0, 4.0),
            p + Vec2::new(4.0, 4.0),
            1.5,
            [0.94, 0.95, 0.90, 1.0],
        );
    } else {
        batch.disc(p, 1.0, [0.10, 0.15, 0.22, 1.0]);
    }
}
fn draw_fleet(batch: &mut PrimitiveBatch, p: Vec2, definition: &str, contrast: bool) {
    let c = if contrast {
        [1.0, 1.0, 0.95, 1.0]
    } else {
        [0.85, 0.88, 0.82, 1.0]
    };
    match definition {
        "colony_ark" => {
            batch.quad(p, Vec2::new(5.0, 3.0), c);
            batch.ring(p, 7.0, 1.5, c);
        }
        "escort" => {
            batch.line(p + Vec2::new(-6.0, -5.0), p + Vec2::new(5.0, 0.0), 3.0, c);
            batch.line(p + Vec2::new(5.0, 0.0), p + Vec2::new(-6.0, 5.0), 3.0, c);
        }
        _ => {
            batch.line(p + Vec2::new(-5.0, -3.0), p + Vec2::new(5.0, 0.0), 1.5, c);
            batch.line(p + Vec2::new(5.0, 0.0), p + Vec2::new(-5.0, 3.0), 1.5, c);
        }
    }
}

fn transit(
    positions: &BTreeMap<LivingEntityIdV2, Vec2>,
    origin: LivingEntityIdV2,
    destination: LivingEntityIdV2,
    departure: LivingTickV2,
    arrival: LivingTickV2,
    tick: LivingTickV2,
) -> Option<Vec2> {
    let from = positions.get(&origin)?;
    let to = positions.get(&destination)?;
    let duration = arrival.0.saturating_sub(departure.0).max(1);
    let progress = tick.0.saturating_sub(departure.0).min(duration) as f32 / duration as f32;
    Some(from.lerp(*to, progress))
}

fn stable_offset(id: LivingEntityIdV2) -> Vec2 {
    let key = presentation_key(id);
    let x = key as u32;
    let y = (key >> 32) as u32;
    Vec2::new(x as f32, y as f32) / u32::MAX as f32 * 2.0 - Vec2::ONE
}

// Presentation-only mixing uses all identity bytes. Never an authority hash.
fn presentation_key(id: LivingEntityIdV2) -> u64 {
    id.0.iter().fold(0xcbf2_9ce4_8422_2325, |key, byte| {
        (key ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3)
    })
}
