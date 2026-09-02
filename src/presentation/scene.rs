use std::collections::BTreeSet;

use glam::{Vec2, Vec3};

use crate::{
    advisory::AdvisorySnapshot,
    game::{
        model::{Campaign, Faction, FieldKind, FleetId, HazardKind, WORLD_COUNT, WorldId},
        simulation::fleet_progress_ratio,
        view::ViewState,
    },
    scenario::ScenarioFingerprint,
};

use super::{
    CameraMatrices, CameraState, GraphicsQuality, MotionPreference, PresentationPreferences,
};

pub const MAX_SCENE_FLEETS: usize = 512;
pub const MAX_SCENE_ROUTES: usize = 256;
pub const HIGH_PARTICLE_BUDGET: u32 = 512;
pub const LOW_PARTICLE_BUDGET: u32 = 96;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneWorld {
    pub id: WorldId,
    pub position: Vec3,
    pub radius: f32,
    pub ownership_color: [f32; 4],
    pub ownership_pattern: u32,
    pub fields: [f32; 3],
    pub energy: f32,
    pub defense: f32,
    pub hovered: bool,
    pub selected: bool,
    pub hazard: Option<HazardKind>,
    pub hazard_field: Option<FieldKind>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRoute {
    pub source: WorldId,
    pub destination: WorldId,
    pub faction: Faction,
    pub start: Vec3,
    pub control: Vec3,
    pub end: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneFleet {
    pub id: FleetId,
    pub faction: Faction,
    pub source: WorldId,
    pub destination: WorldId,
    pub position: Vec3,
    pub strength: f32,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneBackground {
    pub visual_seed: u64,
    pub parallax: Vec2,
    pub static_background: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneEffects {
    pub particle_budget: u32,
    pub animate_halos: bool,
    pub bloom_requested: bool,
    pub high_contrast: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneAdvisory {
    pub priority: WorldId,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneFrame {
    pub worlds: Vec<SceneWorld>,
    pub routes: Vec<SceneRoute>,
    pub fleets: Vec<SceneFleet>,
    pub background: SceneBackground,
    pub effects: SceneEffects,
    pub advisory: Option<SceneAdvisory>,
    pub camera: CameraMatrices,
    pub visual_time: f32,
}

pub fn visual_seed(fingerprint: ScenarioFingerprint) -> u64 {
    let mut value = fingerprint.get() ^ 0x5649_5355_414C_5F31;
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub fn sector_to_command_plane(x: i32, y: i32) -> Vec3 {
    Vec3::new(
        x.clamp(0, 10_000) as f32 / 1_000.0 - 5.0,
        0.0,
        y.clamp(0, 10_000) as f32 / 1_000.0 - 5.0,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_scene_frame(
    campaign: &Campaign,
    scenario_fingerprint: ScenarioFingerprint,
    interpolation_alpha: f32,
    view: ViewState,
    advisory: Option<&AdvisorySnapshot>,
    visual_time: f32,
    camera: &CameraState,
    preferences: PresentationPreferences,
) -> SceneFrame {
    let visual_time = if visual_time.is_finite() {
        visual_time.max(0.0)
    } else {
        0.0
    };
    let interpolation_alpha = if interpolation_alpha.is_finite() {
        interpolation_alpha.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let active_hazard = campaign.active_hazard;
    let worlds: Vec<_> = campaign
        .worlds
        .iter()
        .map(|world| {
            let fields = [
                world.fields.atmosphere,
                world.fields.hydrosphere,
                world.fields.topology,
            ];
            SceneWorld {
                id: world.id,
                position: sector_to_command_plane(world.position.x, world.position.y),
                radius: 0.31
                    + (world.energy.0.min(250_000) as f32 / 250_000.0) * 0.10
                    + (world.defense.0.min(100_000) as f32 / 100_000.0) * 0.06,
                ownership_color: faction_color(world.owner),
                ownership_pattern: faction_pattern(world.owner, preferences.high_contrast),
                fields: fields.map(|level| f32::from(level) / 10.0),
                energy: world.energy.0.min(250_000) as f32 / 250_000.0,
                defense: world.defense.0.min(100_000) as f32 / 100_000.0,
                hovered: view.hovered_world == Some(world.id),
                selected: view.selected_world == Some(world.id),
                hazard: active_hazard.map(|hazard| hazard.kind),
                hazard_field: active_hazard.map(|hazard| hazard.affected_field),
            }
        })
        .collect();
    debug_assert_eq!(worlds.len(), WORLD_COUNT);

    let mut fleets = campaign.fleets.iter().collect::<Vec<_>>();
    fleets.sort_by_key(|fleet| fleet.id);
    fleets.truncate(MAX_SCENE_FLEETS);

    let mut route_keys = BTreeSet::new();
    let mut routes = Vec::new();
    let mut scene_fleets = Vec::with_capacity(fleets.len());
    for fleet in fleets {
        let Some(source) = campaign.worlds.get(usize::from(fleet.source.0)) else {
            continue;
        };
        let Some(destination) = campaign.worlds.get(usize::from(fleet.destination.0)) else {
            continue;
        };
        let start = sector_to_command_plane(source.position.x, source.position.y);
        let end = sector_to_command_plane(destination.position.x, destination.position.y);
        let (numerator, denominator) = fleet_progress_ratio(fleet);
        let progress = if denominator == 0 {
            1.0
        } else {
            (numerator as f64 / denominator as f64) as f32
        }
        .clamp(0.0, 1.0);
        let display_progress =
            (progress + interpolation_alpha / denominator.max(1) as f32).clamp(0.0, 1.0);
        let control = route_control(start, end, fleet.id.0);
        scene_fleets.push(SceneFleet {
            id: fleet.id,
            faction: fleet.faction,
            source: fleet.source,
            destination: fleet.destination,
            position: quadratic_bezier(start, control, end, display_progress),
            strength: (fleet.strength.0.min(100_000) as f32 / 100_000.0).max(0.08),
            progress,
        });

        let key = (
            fleet.source.0,
            fleet.destination.0,
            faction_tag(fleet.faction),
        );
        if routes.len() < MAX_SCENE_ROUTES && route_keys.insert(key) {
            routes.push(SceneRoute {
                source: fleet.source,
                destination: fleet.destination,
                faction: fleet.faction,
                start,
                control,
                end,
            });
        }
    }

    let reduced_motion = preferences.motion == MotionPreference::Reduced;
    let high_path = preferences.graphics_quality != GraphicsQuality::Low;
    let seed = visual_seed(scenario_fingerprint);
    SceneFrame {
        worlds,
        routes,
        fleets: scene_fleets,
        background: SceneBackground {
            visual_seed: seed,
            parallax: if reduced_motion {
                Vec2::ZERO
            } else {
                Vec2::new(camera.yaw.sin(), camera.pitch.cos()) * 0.025
            },
            static_background: reduced_motion,
        },
        effects: SceneEffects {
            particle_budget: if reduced_motion {
                0
            } else if high_path {
                HIGH_PARTICLE_BUDGET
            } else {
                LOW_PARTICLE_BUDGET
            },
            animate_halos: !reduced_motion,
            bloom_requested: high_path,
            high_contrast: preferences.high_contrast,
        },
        advisory: advisory.map(|snapshot| SceneAdvisory {
            priority: snapshot.priority,
            score: snapshot.scores[usize::from(snapshot.priority.0)],
        }),
        camera: camera.current_matrices,
        visual_time: if reduced_motion { 0.0 } else { visual_time },
    }
}

fn route_control(start: Vec3, end: Vec3, route_tag: u64) -> Vec3 {
    let midpoint = (start + end) * 0.5;
    let length = start.distance(end);
    midpoint + Vec3::Y * (0.55 + length * 0.12 + (route_tag % 5) as f32 * 0.025)
}

fn quadratic_bezier(start: Vec3, control: Vec3, end: Vec3, t: f32) -> Vec3 {
    let one_minus = 1.0 - t;
    start * one_minus * one_minus + control * (2.0 * one_minus * t) + end * t * t
}

const fn faction_color(faction: Option<Faction>) -> [f32; 4] {
    match faction {
        Some(Faction::Union) => [0.12, 0.82, 1.0, 1.0],
        Some(Faction::Helix) => [1.0, 0.28, 0.12, 1.0],
        Some(Faction::Choir) => [0.76, 0.32, 1.0, 1.0],
        None => [0.48, 0.56, 0.66, 1.0],
    }
}

const fn faction_pattern(faction: Option<Faction>, high_contrast: bool) -> u32 {
    if !high_contrast {
        return 0;
    }
    match faction {
        Some(Faction::Union) => 1,
        Some(Faction::Helix) => 2,
        Some(Faction::Choir) => 3,
        None => 4,
    }
}

const fn faction_tag(faction: Faction) -> u8 {
    match faction {
        Faction::Union => 0,
        Faction::Helix => 1,
        Faction::Choir => 2,
    }
}
