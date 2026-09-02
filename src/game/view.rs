use crate::advisory::{AdvisoryBackend, AdvisorySnapshot};
use crate::engine::primitives::PrimitiveBatch;
use crate::game::model::{
    Campaign, Faction, FieldKind, HazardKind, Outcome, Phase, SectorPoint, World, WorldId,
};
use crate::game::simulation::fleet_progress_ratio;
use glam::Vec2;

const LOGICAL_SECTOR_EXTENT: f32 = 10_000.0;
const STAR_COUNT: u64 = 72;

const BACKGROUND: [f32; 4] = [0.018, 0.027, 0.075, 1.0];
const PANEL: [f32; 4] = [0.045, 0.075, 0.145, 0.96];
const GRID: [f32; 4] = [0.12, 0.32, 0.44, 0.50];
const TEXT: [f32; 4] = [0.72, 0.92, 1.0, 1.0];
const MUTED_TEXT: [f32; 4] = [0.42, 0.62, 0.74, 1.0];
const UNION: [f32; 4] = [0.24, 0.74, 1.0, 1.0];
const HELIX: [f32; 4] = [1.0, 0.34, 0.36, 1.0];
const CHOIR: [f32; 4] = [0.82, 0.42, 1.0, 1.0];
const NEUTRAL: [f32; 4] = [0.62, 0.68, 0.75, 1.0];
const SELECTED: [f32; 4] = [1.0, 0.88, 0.22, 1.0];
const HAZARD: [f32; 4] = [1.0, 0.45, 0.12, 1.0];

/// Responsive screen-space bounds for the tactical table and its two command panels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameLayout {
    pub viewport: Vec2,
    pub playfield_min: Vec2,
    pub playfield_max: Vec2,
    pub ui_scale: f32,
}

impl GameLayout {
    pub fn new(viewport: Vec2) -> Self {
        Self::with_user_scale(viewport, 1.0)
    }

    pub fn with_user_scale(viewport: Vec2, user_scale: f32) -> Self {
        let user_scale = if user_scale.is_finite() {
            user_scale.clamp(0.85, 1.30)
        } else {
            1.0
        };
        let ui_scale = ((viewport.x / 1440.0)
            .min(viewport.y / 900.0)
            .clamp(0.72, 1.15)
            * user_scale)
            .clamp(0.72, 1.50);
        let left = 240.0 * ui_scale;
        let right = 280.0 * ui_scale;
        let top = 64.0 * ui_scale;
        let bottom = if viewport.x < 1_100.0 {
            102.0
        } else {
            56.0 * ui_scale
        };
        let (playfield_min_x, playfield_max_x) = non_inverted_axis(left, viewport.x - right);
        let (playfield_min_y, playfield_max_y) = non_inverted_axis(top, viewport.y - bottom);
        Self {
            viewport,
            playfield_min: Vec2::new(playfield_min_x, playfield_min_y),
            playfield_max: Vec2::new(playfield_max_x, playfield_max_y),
            ui_scale,
        }
    }

    pub fn world_to_screen(&self, world_position: SectorPoint) -> Vec2 {
        let unit = Vec2::new(
            world_position.x.clamp(0, 10_000) as f32 / LOGICAL_SECTOR_EXTENT,
            world_position.y.clamp(0, 10_000) as f32 / LOGICAL_SECTOR_EXTENT,
        );
        self.playfield_min + (self.playfield_max - self.playfield_min) * unit
    }
}

fn non_inverted_axis(min: f32, max: f32) -> (f32, f32) {
    if min <= max {
        (min, max)
    } else {
        let collapsed = min + (max - min) * 0.5;
        (collapsed, collapsed)
    }
}

/// Session-only interaction state.  It never mutates campaign truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewState {
    pub selected_world: Option<WorldId>,
    pub hovered_world: Option<WorldId>,
    pub selected_field: FieldKind,
    pub paused: bool,
    pub speed_multiplier: u8,
    pub help_visible: bool,
}

pub fn hit_test_world(game: &Campaign, layout: &GameLayout, cursor: Vec2) -> Option<WorldId> {
    let radius_squared = (22.0 * layout.ui_scale).powi(2);
    game.worlds
        .iter()
        .filter(|world| {
            layout
                .world_to_screen(world.position)
                .distance_squared(cursor)
                <= radius_squared
        })
        .map(|world| world.id)
        .min()
}

pub fn hit_test_scenario_control(layout: &GameLayout, cursor: Vec2) -> bool {
    let scale = layout.ui_scale;
    let minimum = Vec2::new(
        layout.viewport.x - 142.0 * scale,
        layout.viewport.y - 50.0 * scale,
    );
    let maximum = Vec2::new(
        layout.viewport.x - 8.0 * scale,
        layout.viewport.y - 6.0 * scale,
    );
    cursor.cmpge(minimum).all() && cursor.cmple(maximum).all()
}

/// Rebuilds all presentation geometry from immutable campaign and UI state.
pub fn build_frame(
    game: &Campaign,
    view: ViewState,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    batch.clear();
    let center = layout.viewport * 0.5;
    batch.quad(center, center, BACKGROUND);

    draw_stars(game.seed, layout, batch);
    draw_tactical_grid(layout, batch);
    draw_fleet_trails(game, layout, batch);
    draw_worlds(game, view, layout, batch);
    draw_hud(game, view, layout, batch);
}

/// Builds only the compatibility primitive HUD that is composited after the
/// tactical 3D scene. The complete 2D frame remains available for fallback and
/// focused presentation tests.
pub fn build_hud_frame(
    game: &Campaign,
    view: ViewState,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    batch.clear();
    draw_hud(game, view, layout, batch);
}

/// Builds the legacy command-deck chrome below the SDF controls. Interactive
/// buttons are omitted because their SDF replacements own both visuals and hit
/// targets; focus and diagnostics are emitted into the final overlay instead.
pub fn build_command_deck_underlay(
    game: &Campaign,
    view: ViewState,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    batch.clear();
    draw_panels(layout, batch);
    draw_status_panel(game, view, layout, batch);
    draw_inspector_panel(game, view, layout, batch);
    if let Some(hazard) = game.active_hazard {
        draw_hazard_banner(hazard.kind, hazard.affected_field, layout, batch);
    }
    if view.help_visible {
        draw_help(layout, batch);
    }
    if let Phase::Finished { outcome, .. } = game.phase {
        draw_terminal(outcome, layout, batch);
    }
}

fn draw_hud(game: &Campaign, view: ViewState, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    draw_panels(layout, batch);
    draw_status_panel(game, view, layout, batch);
    draw_inspector_panel(game, view, layout, batch);
    draw_controls(layout, batch);
    if let Some(hazard) = game.active_hazard {
        draw_hazard_banner(hazard.kind, hazard.affected_field, layout, batch);
    }
    if view.help_visible {
        draw_help(layout, batch);
    }
    if let Phase::Finished { outcome, .. } = game.phase {
        draw_terminal(outcome, layout, batch);
    }
}

fn draw_panels(layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let scale = layout.ui_scale;
    let left_width = 240.0 * scale;
    let right_width = 280.0 * scale;
    let top_height = 64.0 * scale;
    let bottom_height = 56.0 * scale;
    batch.quad(
        Vec2::new(left_width * 0.5, layout.viewport.y * 0.5),
        Vec2::new(left_width * 0.5, layout.viewport.y * 0.5),
        PANEL,
    );
    batch.quad(
        Vec2::new(
            layout.viewport.x - right_width * 0.5,
            layout.viewport.y * 0.5,
        ),
        Vec2::new(right_width * 0.5, layout.viewport.y * 0.5),
        PANEL,
    );
    batch.quad(
        Vec2::new(layout.viewport.x * 0.5, top_height * 0.5),
        Vec2::new(layout.viewport.x * 0.5, top_height * 0.5),
        PANEL,
    );
    batch.quad(
        Vec2::new(
            layout.viewport.x * 0.5,
            layout.viewport.y - bottom_height * 0.5,
        ),
        Vec2::new(layout.viewport.x * 0.5, bottom_height * 0.5),
        PANEL,
    );
    batch.line(
        Vec2::new(left_width, 0.0),
        Vec2::new(left_width, layout.viewport.y),
        scale,
        GRID,
    );
    batch.line(
        Vec2::new(layout.viewport.x - right_width, 0.0),
        Vec2::new(layout.viewport.x - right_width, layout.viewport.y),
        scale,
        GRID,
    );
}

fn draw_stars(seed: u64, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let extent = layout.playfield_max - layout.playfield_min;
    for index in 0..STAR_COUNT {
        let x = unit_noise(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let y = unit_noise(seed ^ index.wrapping_mul(0xBF58_476D_1CE4_E5B9));
        let brightness = 0.2 + unit_noise(seed ^ index.wrapping_mul(0x94D0_49BB_1331_11EB)) * 0.45;
        let position = layout.playfield_min + extent * Vec2::new(x, y);
        batch.disc(
            position,
            layout.ui_scale * (0.5 + brightness),
            [brightness, brightness, 1.0, 0.85],
        );
    }
}

fn draw_tactical_grid(layout: &GameLayout, batch: &mut PrimitiveBatch) {
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let x = layout.playfield_min.x + (layout.playfield_max.x - layout.playfield_min.x) * t;
        let y = layout.playfield_min.y + (layout.playfield_max.y - layout.playfield_min.y) * t;
        batch.line(
            Vec2::new(x, layout.playfield_min.y),
            Vec2::new(x, layout.playfield_max.y),
            layout.ui_scale,
            GRID,
        );
        batch.line(
            Vec2::new(layout.playfield_min.x, y),
            Vec2::new(layout.playfield_max.x, y),
            layout.ui_scale,
            GRID,
        );
    }
    let center = (layout.playfield_min + layout.playfield_max) * 0.5;
    let half_size = (layout.playfield_max - layout.playfield_min) * 0.5;
    batch.ring(
        center,
        half_size.x.min(half_size.y),
        layout.ui_scale,
        [GRID[0], GRID[1], GRID[2], 0.75],
    );
}

fn draw_fleet_trails(game: &Campaign, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    for fleet in &game.fleets {
        let Some(source) = world_by_id(game, fleet.source) else {
            continue;
        };
        let Some(destination) = world_by_id(game, fleet.destination) else {
            continue;
        };
        let from = layout.world_to_screen(source.position);
        let to = layout.world_to_screen(destination.position);
        let color = faction_color(fleet.faction);
        batch.line(
            from,
            to,
            1.5 * layout.ui_scale,
            [color[0], color[1], color[2], 0.42],
        );
        let (progress, length) = fleet_progress_ratio(fleet);
        let point = from.lerp(to, progress as f32 / length as f32);
        batch.disc(point, 5.0 * layout.ui_scale, color);
    }
}

fn draw_worlds(game: &Campaign, view: ViewState, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    for world in &game.worlds {
        let position = layout.world_to_screen(world.position);
        let color = world.owner.map(faction_color).unwrap_or(NEUTRAL);
        for (index, level) in [
            world.fields.atmosphere,
            world.fields.hydrosphere,
            world.fields.topology,
        ]
        .into_iter()
        .enumerate()
        {
            let radius = (12.0 + index as f32 * 4.0 + level as f32 * 0.55) * layout.ui_scale;
            let band = match index {
                0 => [0.28, 0.78, 0.96, 0.72],
                1 => [0.20, 0.45, 1.0, 0.72],
                _ => [0.64, 0.44, 0.20, 0.72],
            };
            batch.ring(position, radius, 1.0 * layout.ui_scale, band);
        }
        batch.disc(position, 11.0 * layout.ui_scale, color);
        batch.ring(
            position,
            14.0 * layout.ui_scale,
            1.5 * layout.ui_scale,
            [color[0], color[1], color[2], 0.85],
        );
        if view.selected_world == Some(world.id) {
            batch.ring(
                position,
                24.0 * layout.ui_scale,
                2.0 * layout.ui_scale,
                SELECTED,
            );
        } else if view.hovered_world == Some(world.id) {
            batch.ring(
                position,
                21.0 * layout.ui_scale,
                1.5 * layout.ui_scale,
                TEXT,
            );
        }
        let label_origin = position + Vec2::new(16.0, -19.0) * layout.ui_scale;
        batch.text(
            label_origin,
            1.2 * layout.ui_scale,
            TEXT,
            world.name.as_str(),
        );
    }
}

fn draw_status_panel(
    game: &Campaign,
    view: ViewState,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    let scale = layout.ui_scale;
    let origin = Vec2::new(16.0 * scale, 84.0 * scale);
    text_line(batch, origin, scale, TEXT, "MISSION STATUS");
    text_line(
        batch,
        origin + Vec2::new(0.0, 22.0 * scale),
        scale,
        MUTED_TEXT,
        "SECTOR COMMAND",
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 44.0 * scale),
        scale,
        faction_color(Faction::Union),
        "UNION",
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 62.0 * scale),
        scale,
        MUTED_TEXT,
        &format!("WORLDS {}", game.controlled_count(Faction::Union)),
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 80.0 * scale),
        scale,
        MUTED_TEXT,
        &format!("FLEETS {}", game.fleet_count(Faction::Union)),
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 106.0 * scale),
        scale,
        if view.paused { HAZARD } else { TEXT },
        if view.paused { "PAUSED" } else { "RUNNING" },
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 124.0 * scale),
        scale,
        MUTED_TEXT,
        &format!("SPEED {}X", view.speed_multiplier),
    );
    text_line(
        batch,
        origin + Vec2::new(0.0, 146.0 * scale),
        scale,
        MUTED_TEXT,
        &format!("TICK {}", game.next_tick.0),
    );
}

fn draw_inspector_panel(
    game: &Campaign,
    view: ViewState,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    let scale = layout.ui_scale;
    let origin = Vec2::new(layout.viewport.x - 264.0 * scale, 84.0 * scale);
    text_line(batch, origin, scale, TEXT, "WORLD INSPECTOR");
    let Some(id) = view.selected_world else {
        text_line(
            batch,
            origin + Vec2::new(0.0, 26.0 * scale),
            scale,
            MUTED_TEXT,
            "SELECT A WORLD",
        );
        return;
    };
    let Some(world) = world_by_id(game, id) else {
        text_line(
            batch,
            origin + Vec2::new(0.0, 26.0 * scale),
            scale,
            HAZARD,
            "WORLD OFFLINE",
        );
        return;
    };
    let lines = [
        world.name.to_string(),
        format!("DEFENSE {}", world.defense.0),
        format!("ENERGY {}", world.energy.0),
        format!("ATM {}", world.fields.atmosphere),
        format!("HYD {}", world.fields.hydrosphere),
        format!("TOP {}", world.fields.topology),
        format!("FIELD {}", field_label(view.selected_field)),
    ];
    for (index, line) in lines.iter().enumerate() {
        text_line(
            batch,
            origin + Vec2::new(0.0, (26 + index as i32 * 18) as f32 * scale),
            scale,
            if index == 0 {
                world.owner.map(faction_color).unwrap_or(NEUTRAL)
            } else {
                MUTED_TEXT
            },
            line,
        );
    }
}

fn draw_controls(layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let scale = layout.ui_scale;
    let origin = Vec2::new(16.0 * scale, layout.viewport.y - 38.0 * scale);
    text_line(
        batch,
        origin,
        scale,
        TEXT,
        "CLICK SELECT  RIGHT/SPACE LAUNCH  P PAUSE  H HELP",
    );
    text_line(
        batch,
        Vec2::new(
            layout.viewport.x - 140.0 * scale,
            layout.viewport.y - 38.0 * scale,
        ),
        scale,
        MUTED_TEXT,
        "SCENARIO F4",
    );
}

pub fn draw_advisory(
    game: &Campaign,
    snapshot: Option<&AdvisorySnapshot>,
    updating: bool,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    let scale = layout.ui_scale;
    let origin = Vec2::new(layout.viewport.x - 264.0 * scale, 350.0 * scale);
    text_line(batch, origin, scale, TEXT, "NEURAL ADVISORY");
    text_line(
        batch,
        origin + Vec2::new(0.0, 20.0 * scale),
        scale,
        HAZARD,
        "ADVISORY ONLY",
    );
    let Some(snapshot) = snapshot else {
        text_line(
            batch,
            origin + Vec2::new(0.0, 44.0 * scale),
            scale,
            MUTED_TEXT,
            "INITIALIZING",
        );
        return;
    };
    let priority = world_by_id(game, snapshot.priority)
        .map(|world| world.name.as_str())
        .unwrap_or("UNKNOWN");
    let backend = match snapshot.backend {
        AdvisoryBackend::Cpu => "CPU",
        AdvisoryBackend::WebGpu => "WEBGPU",
        AdvisoryBackend::CpuFallback => "CPU FALLBACK",
    };
    for (index, line) in [
        format!("PRIORITY {priority}"),
        format!(
            "SCORE {:.1}%",
            snapshot.scores[usize::from(snapshot.priority.0)] * 100.0
        ),
        format!("SOURCE TICK {}", snapshot.metadata.source_tick.0),
        backend.to_owned(),
        if updating {
            "UPDATING".into()
        } else {
            "CURRENT".into()
        },
    ]
    .iter()
    .enumerate()
    {
        text_line(
            batch,
            origin + Vec2::new(0.0, (46.0 + index as f32 * 18.0) * scale),
            scale,
            if updating && index == 4 {
                SELECTED
            } else {
                MUTED_TEXT
            },
            line,
        );
    }
}

pub fn draw_recoverable_message(message: &str, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let scale = layout.ui_scale;
    let label = message.to_ascii_uppercase();
    let center = Vec2::new(layout.viewport.x * 0.5, layout.viewport.y - 82.0 * scale);
    batch.quad(
        center,
        Vec2::new(220.0 * scale, 20.0 * scale),
        [0.42, 0.08, 0.10, 0.95],
    );
    batch.text(
        center - Vec2::new(205.0 * scale, 4.0 * scale),
        scale,
        TEXT,
        &label.chars().take(68).collect::<String>(),
    );
}

fn draw_hazard_banner(
    kind: HazardKind,
    field: FieldKind,
    layout: &GameLayout,
    batch: &mut PrimitiveBatch,
) {
    let scale = layout.ui_scale;
    let center = Vec2::new(
        (layout.playfield_min.x + layout.playfield_max.x) * 0.5,
        32.0 * scale,
    );
    batch.quad(
        center,
        Vec2::new(190.0 * scale, 20.0 * scale),
        [HAZARD[0], HAZARD[1], HAZARD[2], 0.22],
    );
    let label = format!("HAZARD {} {}", hazard_label(kind), field_label(field));
    batch.text(
        center - Vec2::new(label.len() as f32 * 3.0 * scale, 4.0 * scale),
        scale,
        HAZARD,
        &label,
    );
}

fn draw_help(layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let scale = layout.ui_scale;
    let center = layout.viewport * 0.5;
    batch.quad(
        center,
        Vec2::new(180.0 * scale, 70.0 * scale),
        [0.02, 0.04, 0.10, 0.96],
    );
    text_line(
        batch,
        center + Vec2::new(-145.0 * scale, -40.0 * scale),
        scale,
        TEXT,
        "COMMAND HELP",
    );
    text_line(
        batch,
        center + Vec2::new(-145.0 * scale, -16.0 * scale),
        scale,
        MUTED_TEXT,
        "CLICK WORLD TO SELECT",
    );
    text_line(
        batch,
        center + Vec2::new(-145.0 * scale, 6.0 * scale),
        scale,
        MUTED_TEXT,
        "SPACE LAUNCHES TO HOVERED WORLD",
    );
    text_line(
        batch,
        center + Vec2::new(-145.0 * scale, 28.0 * scale),
        scale,
        MUTED_TEXT,
        "F1 F2 F3 SELECT FIELD",
    );
}

fn draw_terminal(outcome: Outcome, layout: &GameLayout, batch: &mut PrimitiveBatch) {
    let scale = layout.ui_scale;
    let center = layout.viewport * 0.5;
    batch.quad(
        center,
        Vec2::new(230.0 * scale, 72.0 * scale),
        [0.0, 0.0, 0.02, 0.93],
    );
    let label = match outcome {
        Outcome::FactionVictory { winner } => format!("{} VICTORY", faction_label(winner)),
        Outcome::PlayerEliminated => "UNION ELIMINATED".to_owned(),
    };
    batch.text(
        center - Vec2::new(label.len() as f32 * 3.0 * scale, 18.0 * scale),
        1.8 * scale,
        SELECTED,
        &label,
    );
    text_line(
        batch,
        center + Vec2::new(-70.0 * scale, 25.0 * scale),
        scale,
        MUTED_TEXT,
        "MISSION COMPLETE",
    );
}

fn text_line(batch: &mut PrimitiveBatch, origin: Vec2, scale: f32, color: [f32; 4], text: &str) {
    batch.text(origin, scale, color, text);
}

fn world_by_id(game: &Campaign, id: WorldId) -> Option<&World> {
    game.worlds.iter().find(|world| world.id == id)
}

const fn faction_color(faction: Faction) -> [f32; 4] {
    match faction {
        Faction::Union => UNION,
        Faction::Helix => HELIX,
        Faction::Choir => CHOIR,
    }
}

const fn faction_label(faction: Faction) -> &'static str {
    match faction {
        Faction::Union => "UNION",
        Faction::Helix => "HELIX",
        Faction::Choir => "CHOIR",
    }
}

const fn field_label(field: FieldKind) -> &'static str {
    match field {
        FieldKind::Atmosphere => "ATM",
        FieldKind::Hydrosphere => "HYD",
        FieldKind::Topology => "TOP",
    }
}

const fn hazard_label(kind: HazardKind) -> &'static str {
    match kind {
        HazardKind::IonStorm => "ION STORM",
        HazardKind::GravityTide => "GRAVITY TIDE",
    }
}

fn unit_noise(mut value: u64) -> f32 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((value ^ (value >> 31)) >> 40) as f32 / ((1_u32 << 24) - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::primitives::PrimitiveBatch;
    use crate::game::model::{
        ActiveHazard, Campaign, DEFAULT_SEED, Fleet, FleetId, HazardKind, Phase, RulesV1, Strength,
        Tick,
    };
    use glam::Vec2;

    fn default_view() -> ViewState {
        ViewState {
            selected_world: None,
            hovered_world: None,
            selected_field: FieldKind::Atmosphere,
            paused: false,
            speed_multiplier: 1,
            help_visible: false,
        }
    }

    #[test]
    fn sector_bounds_map_linearly_and_inclusively() {
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        assert_eq!(
            layout.world_to_screen(SectorPoint { x: 0, y: 0 }),
            layout.playfield_min
        );
        assert_eq!(
            layout.world_to_screen(SectorPoint {
                x: 10_000,
                y: 10_000
            }),
            layout.playfield_max
        );
    }

    #[test]
    fn center_of_a_world_hits_that_world() {
        let game = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        let center = layout.world_to_screen(game.worlds[0].position);
        assert_eq!(hit_test_world(&game, &layout, center), Some(WorldId(0)));
    }

    #[test]
    fn overlap_hits_the_lowest_world_id() {
        let mut game = Campaign::new(DEFAULT_SEED, RulesV1::default());
        game.worlds[1].position = game.worlds[0].position;
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        let center = layout.world_to_screen(game.worlds[0].position);
        assert_eq!(hit_test_world(&game, &layout, center), Some(WorldId(0)));
    }

    #[test]
    fn layout_reserves_command_table_panels_at_desktop_and_compact_sizes() {
        let desktop = GameLayout::new(Vec2::new(1440.0, 900.0));
        assert_eq!(desktop.ui_scale, 1.0);
        assert_eq!(desktop.playfield_min, Vec2::new(240.0, 64.0));
        assert_eq!(desktop.playfield_max, Vec2::new(1160.0, 844.0));

        let compact = GameLayout::new(Vec2::new(960.0, 600.0));
        assert_eq!(compact.ui_scale, 0.72);
        assert_eq!(compact.playfield_min, Vec2::new(172.8, 46.08));
        assert_eq!(compact.playfield_max, Vec2::new(758.4, 498.0));
    }

    #[test]
    fn layout_collapses_without_inverting_at_reservation_boundaries() {
        let boundary = GameLayout::new(Vec2::new(374.4, 148.08));
        assert_eq!(boundary.ui_scale, 0.72);
        assert_eq!(boundary.playfield_min, boundary.playfield_max);
        assert!(
            boundary
                .playfield_min
                .abs_diff_eq(Vec2::new(172.8, 46.08), 0.001)
        );

        let below = GameLayout::new(Vec2::new(320.0, 80.0));
        assert_eq!(below.ui_scale, 0.72);
        assert_eq!(below.playfield_min, below.playfield_max);
        assert!(below.playfield_min.is_finite());
        assert!(below.playfield_max.is_finite());
        assert_eq!(
            below.world_to_screen(SectorPoint { x: 0, y: 0 }),
            below.world_to_screen(SectorPoint {
                x: 10_000,
                y: 10_000
            })
        );
    }

    #[test]
    fn frame_builder_handles_empty_selected_help_and_terminal_views_with_finite_vertices() {
        let game = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        for view in [
            default_view(),
            ViewState {
                selected_world: Some(WorldId(0)),
                ..default_view()
            },
            ViewState {
                help_visible: true,
                ..default_view()
            },
        ] {
            let mut batch = PrimitiveBatch::default();
            build_frame(&game, view, &layout, &mut batch);
            assert!(!batch.vertices().is_empty());
            assert!(batch.vertices().iter().all(|vertex| {
                vertex
                    .position
                    .iter()
                    .chain(vertex.color.iter())
                    .chain(vertex.local.iter())
                    .all(|value| value.is_finite())
            }));
        }
        let mut terminal = game.clone();
        terminal.phase = Phase::Finished {
            at: Tick(1),
            outcome: Outcome::PlayerEliminated,
        };
        let mut batch = PrimitiveBatch::default();
        build_frame(&terminal, default_view(), &layout, &mut batch);
        assert!(!batch.vertices().is_empty());
    }

    #[test]
    fn hud_builder_excludes_the_legacy_two_dimensional_playfield() {
        let game = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        let mut complete = PrimitiveBatch::default();
        let mut hud = PrimitiveBatch::default();

        build_frame(&game, default_view(), &layout, &mut complete);
        build_hud_frame(&game, default_view(), &layout, &mut hud);

        assert!(!hud.vertices().is_empty());
        assert!(hud.vertices().len() < complete.vertices().len());
        assert!(hud.vertices().iter().all(|vertex| {
            vertex
                .position
                .iter()
                .chain(vertex.color.iter())
                .chain(vertex.local.iter())
                .all(|value| value.is_finite())
        }));
    }

    #[test]
    fn frame_builder_adds_fleet_trails_and_hazard_banner() {
        let mut game = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
        let mut baseline = PrimitiveBatch::default();
        build_frame(&game, default_view(), &layout, &mut baseline);

        game.fleets.push(Fleet {
            id: FleetId(99),
            faction: Faction::Union,
            source: WorldId(0),
            destination: WorldId(1),
            strength: Strength(10_000),
            route_length: 100,
            progress: 50,
            speed_per_second: 23,
            hydrosphere_level: 5,
            movement_remainder: 0,
        });
        game.active_hazard = Some(ActiveHazard {
            event_index: 0,
            kind: HazardKind::IonStorm,
            affected_field: FieldKind::Atmosphere,
            start: Tick(1),
            end: Tick(2),
        });
        let mut enriched = PrimitiveBatch::default();
        build_frame(&game, default_view(), &layout, &mut enriched);
        assert!(enriched.vertices().len() > baseline.vertices().len());
        assert!(enriched.vertices().iter().all(|vertex| {
            vertex
                .position
                .iter()
                .chain(vertex.color.iter())
                .chain(vertex.local.iter())
                .all(|value| value.is_finite())
        }));
    }
}
