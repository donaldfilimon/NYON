pub const WORLD_COUNT: usize = 7;
pub const DEFAULT_SEED: u64 = 0x4947_5731_2026_0902;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorldName(Box<str>);

impl WorldName {
    pub const MAX_BYTES: usize = 24;

    pub fn new(value: impl AsRef<str>) -> Result<Self, WorldNameError> {
        let value = value.as_ref();
        if value.is_empty() || value.len() > Self::MAX_BYTES {
            return Err(WorldNameError::Length);
        }
        if value.starts_with(' ') || value.ends_with(' ') || value.contains("  ") {
            return Err(WorldNameError::Spacing);
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_uppercase()
                || byte.is_ascii_digit()
                || matches!(byte, b' ' | b'.' | b'-' | b'/')
        }) {
            return Err(WorldNameError::Character);
        }
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorldName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for WorldName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::ops::Deref for WorldName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WorldNameError {
    #[error("world names must contain 1 through 24 bytes")]
    Length,
    #[error("world names cannot have leading, trailing, or repeated spaces")]
    Spacing,
    #[error("world names use only uppercase ASCII letters, digits, space, period, dash, or slash")]
    Character,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Tick(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WorldId(pub u8);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FleetId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommandSequence(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Energy(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Strength(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Faction {
    Union,
    Helix,
    Choir,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind {
    Atmosphere,
    Hydrosphere,
    Topology,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldAdjustment {
    Increase,
    Decrease,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectorPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldLevels {
    pub atmosphere: u8,
    pub hydrosphere: u8,
    pub topology: u8,
}

impl FieldLevels {
    pub const fn new(atmosphere: u8, hydrosphere: u8, topology: u8) -> Self {
        Self {
            atmosphere,
            hydrosphere,
            topology,
        }
    }

    pub const fn get(self, field: FieldKind) -> u8 {
        match field {
            FieldKind::Atmosphere => self.atmosphere,
            FieldKind::Hydrosphere => self.hydrosphere,
            FieldKind::Topology => self.topology,
        }
    }

    pub fn set(&mut self, field: FieldKind, value: u8) {
        match field {
            FieldKind::Atmosphere => self.atmosphere = value,
            FieldKind::Hydrosphere => self.hydrosphere = value,
            FieldKind::Topology => self.topology = value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RulesV1 {
    pub tick_hz: u32,
    pub win_world_count: u8,
    pub ai_period_ticks: u64,
    pub hazard_first_tick: u64,
    pub hazard_period_ticks: u64,
    pub hazard_duration_ticks: u64,
    pub maximum_energy: Energy,
    pub maximum_defense: Strength,
    pub minimum_launch: Energy,
    pub field_raise_cost: Energy,
    pub field_lower_refund: Energy,
    pub base_fleet_speed: u64,
}

impl Default for RulesV1 {
    fn default() -> Self {
        Self {
            tick_hz: 60,
            win_world_count: 5,
            ai_period_ticks: 60,
            hazard_first_tick: 2700,
            hazard_period_ticks: 2700,
            hazard_duration_ticks: 720,
            maximum_energy: Energy(250_000),
            maximum_defense: Strength(100_000),
            minimum_launch: Energy(10_000),
            field_raise_cost: Energy(20_000),
            field_lower_refund: Energy(10_000),
            base_fleet_speed: 23,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub id: WorldId,
    pub name: WorldName,
    pub position: SectorPoint,
    pub owner: Option<Faction>,
    pub defense: Strength,
    pub energy: Energy,
    pub base_output_per_second: Energy,
    pub base_regeneration_per_second: Strength,
    pub fields: FieldLevels,
    pub production_remainder: u64,
    pub regeneration_remainder: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fleet {
    pub id: FleetId,
    pub faction: Faction,
    pub source: WorldId,
    pub destination: WorldId,
    pub strength: Strength,
    pub route_length: u64,
    pub progress: u64,
    pub speed_per_second: u64,
    pub hydrosphere_level: u8,
    pub movement_remainder: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HazardKind {
    IonStorm,
    GravityTide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveHazard {
    pub event_index: u64,
    pub kind: HazardKind,
    pub affected_field: FieldKind,
    pub start: Tick,
    pub end: Tick,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    FactionVictory { winner: Faction },
    PlayerEliminated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Running,
    Finished { at: Tick, outcome: Outcome },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameCommand {
    Launch {
        source: WorldId,
        destination: WorldId,
    },
    TuneField {
        world: WorldId,
        field: FieldKind,
        adjustment: FieldAdjustment,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandEnvelope {
    pub tick: Tick,
    pub sequence: CommandSequence,
    pub command: GameCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Campaign {
    pub rules_version: u32,
    pub generator_version: u32,
    pub seed: u64,
    pub next_tick: Tick,
    pub phase: Phase,
    pub worlds: [World; WORLD_COUNT],
    pub fleets: Vec<Fleet>,
    pub active_hazard: Option<ActiveHazard>,
    pub next_hazard_index: u64,
    pub next_fleet_id: FleetId,
}

impl Campaign {
    pub fn new(seed: u64, _rules: RulesV1) -> Self {
        let mut generator = SplitMix64::new(seed);
        let worlds = std::array::from_fn(|index| {
            let (anchor_x, anchor_y) = WORLD_ANCHORS[index];
            let x_jitter = (generator.next() % 1001) as i32 - 500;
            let y_jitter = (generator.next() % 1001) as i32 - 500;
            let base_output_per_second = Energy(1800 + generator.next() % 1201);
            let defense = Strength(40_000 + generator.next() % 20_001);
            let base_regeneration_per_second = Strength(250 + generator.next() % 251);
            let fields = FieldLevels::new(
                (3 + generator.next() % 5) as u8,
                (3 + generator.next() % 5) as u8,
                (3 + generator.next() % 5) as u8,
            );
            let owner = INITIAL_OWNERS[index];
            World {
                id: WorldId(index as u8),
                name: WorldName::new(WORLD_NAMES[index]).expect("factory world names are valid"),
                position: SectorPoint {
                    x: anchor_x + x_jitter,
                    y: anchor_y + y_jitter,
                },
                owner,
                defense,
                energy: Energy(if owner.is_some() { 80_000 } else { 0 }),
                base_output_per_second,
                base_regeneration_per_second,
                fields,
                production_remainder: 0,
                regeneration_remainder: 0,
            }
        });

        Self {
            rules_version: 1,
            generator_version: 1,
            seed,
            next_tick: Tick(0),
            phase: Phase::Running,
            worlds,
            fleets: Vec::new(),
            active_hazard: None,
            next_hazard_index: 0,
            next_fleet_id: FleetId(0),
        }
    }

    pub(crate) fn from_scenario(scenario: &crate::scenario::ScenarioV1) -> Self {
        Self {
            rules_version: crate::scenario::RULES_VERSION,
            generator_version: crate::scenario::GENERATOR_VERSION,
            seed: scenario.seed(),
            next_tick: Tick(0),
            phase: Phase::Running,
            worlds: scenario.worlds().clone(),
            fleets: Vec::new(),
            active_hazard: None,
            next_hazard_index: 0,
            next_fleet_id: FleetId(0),
        }
    }

    pub fn controlled_count(&self, faction: Faction) -> usize {
        self.worlds
            .iter()
            .filter(|world| world.owner == Some(faction))
            .count()
    }

    pub fn fleet_count(&self, faction: Faction) -> usize {
        self.fleets
            .iter()
            .filter(|fleet| fleet.faction == faction)
            .count()
    }
}

const WORLD_ANCHORS: [(i32, i32); WORLD_COUNT] = [
    (1000, 5000),
    (9000, 2500),
    (9000, 7500),
    (5000, 1000),
    (5000, 9000),
    (4000, 4000),
    (6000, 6000),
];

pub(crate) const WORLD_NAMES: [&str; WORLD_COUNT] = [
    "ASTER VALE",
    "KHEPRI",
    "MERIDIAN",
    "VESPER",
    "ORISON",
    "NACRE",
    "UMBRA",
];

const INITIAL_OWNERS: [Option<Faction>; WORLD_COUNT] = [
    Some(Faction::Union),
    Some(Faction::Helix),
    Some(Faction::Choir),
    None,
    None,
    None,
    None,
];

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_campaign_has_dense_world_ids_and_expected_ownership() {
        let campaign = Campaign::new(DEFAULT_SEED, RulesV1::default());
        assert_eq!(campaign.worlds.len(), WORLD_COUNT);
        for (index, world) in campaign.worlds.iter().enumerate() {
            assert_eq!(world.id, WorldId(index as u8));
            assert!(!world.name.is_empty());
        }
        assert_eq!(campaign.controlled_count(Faction::Union), 1);
        assert_eq!(campaign.controlled_count(Faction::Helix), 1);
        assert_eq!(campaign.controlled_count(Faction::Choir), 1);
        assert_eq!(
            campaign
                .worlds
                .iter()
                .filter(|world| world.owner.is_none())
                .count(),
            4
        );
    }

    #[test]
    fn campaign_truth_contains_integer_field_levels() {
        let fields = FieldLevels::new(6, 5, 4);
        assert_eq!(fields.get(FieldKind::Atmosphere), 6);
        assert_eq!(fields.get(FieldKind::Hydrosphere), 5);
        assert_eq!(fields.get(FieldKind::Topology), 4);
    }

    #[test]
    fn default_seed_matches_the_frozen_generator_vector() {
        let campaign = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let expected = [
            ((1037, 5027), 2904, 43735, 488, [6, 7, 3]),
            ((8793, 2818), 2678, 44244, 463, [5, 6, 7]),
            ((8517, 7827), 2246, 43986, 405, [6, 3, 7]),
            ((4811, 1320), 2591, 58596, 454, [7, 7, 6]),
            ((4905, 8955), 2848, 46169, 275, [7, 4, 7]),
            ((3746, 3709), 1944, 43254, 405, [5, 4, 3]),
            ((5925, 6136), 2998, 45102, 474, [6, 7, 7]),
        ];
        for (world, (position, output, defense, regeneration, fields)) in
            campaign.worlds.iter().zip(expected)
        {
            assert_eq!((world.position.x, world.position.y), position);
            assert_eq!(world.base_output_per_second, Energy(output));
            assert_eq!(world.defense, Strength(defense));
            assert_eq!(world.base_regeneration_per_second, Strength(regeneration));
            assert_eq!(
                [
                    world.fields.atmosphere,
                    world.fields.hydrosphere,
                    world.fields.topology
                ],
                fields
            );
        }
    }
}
