pub mod codec;
pub mod store;

use std::collections::BTreeMap;

use crate::game::model::{
    Campaign, DEFAULT_SEED, Energy, Faction, FieldLevels, RulesV1, Strength, WORLD_COUNT, World,
    WorldId, WorldName,
};

pub const FORMAT_VERSION: u32 = 1;
pub const RULES_VERSION: u32 = 1;
pub const GENERATOR_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorldDraft {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub owner: Option<Faction>,
    pub energy: u64,
    pub defense: u64,
    pub base_output: u64,
    pub base_regeneration: u64,
    pub atmosphere: u8,
    pub hydrosphere: u8,
    pub topology: u8,
}

impl WorldDraft {
    fn from_world(world: &World) -> Self {
        Self {
            name: world.name.to_string(),
            x: world.position.x,
            y: world.position.y,
            owner: world.owner,
            energy: world.energy.0,
            defense: world.defense.0,
            base_output: world.base_output_per_second.0,
            base_regeneration: world.base_regeneration_per_second.0,
            atmosphere: world.fields.atmosphere,
            hydrosphere: world.fields.hydrosphere,
            topology: world.fields.topology,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDraft {
    pub seed: u64,
    pub rules: RulesV1,
    pub worlds: [WorldDraft; WORLD_COUNT],
}

impl ScenarioDraft {
    pub fn factory_default() -> Self {
        Self::generated(DEFAULT_SEED, RulesV1::default())
    }

    pub fn generated(seed: u64, rules: RulesV1) -> Self {
        let campaign = Campaign::new(seed, rules);
        Self {
            seed,
            rules,
            worlds: std::array::from_fn(|index| WorldDraft::from_world(&campaign.worlds[index])),
        }
    }

    pub fn regenerate_worlds_from_seed(&mut self) {
        self.worlds = Self::generated(self.seed, self.rules).worlds;
    }

    pub fn validation_report(&self) -> ValidationReport {
        validate(self)
    }

    pub fn validated(&self) -> Result<ScenarioV1, ValidationReport> {
        let report = validate(self);
        if report.is_valid() {
            Ok(ScenarioV1::new(self, report.issues))
        } else {
            Err(report)
        }
    }
}

impl Default for ScenarioDraft {
    fn default() -> Self {
        Self::factory_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioIssueCode {
    TickRate,
    WinWorldCount,
    AiPeriod,
    HazardPeriod,
    HazardDuration,
    HazardBoundaryArithmetic,
    EnergyCap,
    DefenseCap,
    MinimumLaunch,
    FieldCostRelation,
    WorldName,
    WorldCoordinate,
    WorldFieldLevel,
    WorldEnergy,
    WorldDefense,
    DuplicatePosition,
    MissingUnion,
    InitialVictory,
    ProductionArithmetic,
    RegenerationArithmetic,
    FleetSpeedArithmetic,
    DuplicateName,
    ClosePosition,
    MissingHelix,
    MissingChoir,
    NoLaunchableUnion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioIssue {
    pub severity: IssueSeverity,
    pub code: ScenarioIssueCode,
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    issues: Vec<ScenarioIssue>,
}

impl ValidationReport {
    pub fn issues(&self) -> &[ScenarioIssue] {
        &self.issues
    }

    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == IssueSeverity::Warning)
            .count()
    }
}

impl std::fmt::Display for ValidationReport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "scenario has {} validation error(s)",
            self.error_count()
        )
    }
}

impl std::error::Error for ValidationReport {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScenarioFingerprint(pub u64);

impl ScenarioFingerprint {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayIdentity {
    pub scenario: ScenarioFingerprint,
    pub state: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioV1 {
    seed: u64,
    rules: RulesV1,
    worlds: [World; WORLD_COUNT],
    warnings: Vec<ScenarioIssue>,
    fingerprint: ScenarioFingerprint,
}

impl ScenarioV1 {
    fn new(draft: &ScenarioDraft, issues: Vec<ScenarioIssue>) -> Self {
        let worlds = std::array::from_fn(|index| {
            let world = &draft.worlds[index];
            World {
                id: WorldId(index as u8),
                name: WorldName::new(&world.name).expect("validated world name"),
                position: crate::game::model::SectorPoint {
                    x: world.x,
                    y: world.y,
                },
                owner: world.owner,
                defense: Strength(world.defense),
                energy: Energy(world.energy),
                base_output_per_second: Energy(world.base_output),
                base_regeneration_per_second: Strength(world.base_regeneration),
                fields: FieldLevels::new(world.atmosphere, world.hydrosphere, world.topology),
                production_remainder: 0,
                regeneration_remainder: 0,
            }
        });
        let mut scenario = Self {
            seed: draft.seed,
            rules: draft.rules,
            worlds,
            warnings: issues
                .into_iter()
                .filter(|issue| issue.severity == IssueSeverity::Warning)
                .collect(),
            fingerprint: ScenarioFingerprint(0),
        };
        scenario.fingerprint = scenario.compute_fingerprint();
        scenario
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn rules(&self) -> RulesV1 {
        self.rules
    }

    pub fn worlds(&self) -> &[World; WORLD_COUNT] {
        &self.worlds
    }

    pub fn warnings(&self) -> &[ScenarioIssue] {
        &self.warnings
    }

    pub fn fingerprint(&self) -> ScenarioFingerprint {
        self.fingerprint
    }

    pub fn to_draft(&self) -> ScenarioDraft {
        ScenarioDraft {
            seed: self.seed,
            rules: self.rules,
            worlds: std::array::from_fn(|index| WorldDraft::from_world(&self.worlds[index])),
        }
    }

    fn compute_fingerprint(&self) -> ScenarioFingerprint {
        let mut hash = Fnv1a64::new();
        hash.bytes(b"IWSC\0");
        hash.u32(FORMAT_VERSION);
        hash.u32(RULES_VERSION);
        hash.u32(GENERATOR_VERSION);
        hash.u64(self.seed);
        let rules = self.rules;
        hash.u32(rules.tick_hz);
        hash.u8(rules.win_world_count);
        for value in [
            rules.ai_period_ticks,
            rules.hazard_first_tick,
            rules.hazard_period_ticks,
            rules.hazard_duration_ticks,
            rules.maximum_energy.0,
            rules.maximum_defense.0,
            rules.minimum_launch.0,
            rules.field_raise_cost.0,
            rules.field_lower_refund.0,
            rules.base_fleet_speed,
        ] {
            hash.u64(value);
        }
        for world in &self.worlds {
            hash.u8(world.id.0);
            hash.u8(world.name.len() as u8);
            hash.bytes(world.name.as_bytes());
            hash.i32(world.position.x);
            hash.i32(world.position.y);
            hash.u8(owner_tag(world.owner));
            hash.u64(world.energy.0);
            hash.u64(world.defense.0);
            hash.u64(world.base_output_per_second.0);
            hash.u64(world.base_regeneration_per_second.0);
            hash.u8(world.fields.atmosphere);
            hash.u8(world.fields.hydrosphere);
            hash.u8(world.fields.topology);
        }
        ScenarioFingerprint(hash.finish())
    }
}

fn validate(draft: &ScenarioDraft) -> ValidationReport {
    let mut issues = Vec::new();
    let rules = draft.rules;
    error_if(
        &mut issues,
        rules.tick_hz != 60,
        ScenarioIssueCode::TickRate,
        "rules.tick_hz",
        "tick rate must remain 60",
    );
    error_if(
        &mut issues,
        !(2..=WORLD_COUNT as u8).contains(&rules.win_world_count),
        ScenarioIssueCode::WinWorldCount,
        "rules.win_world_count",
        "win world count must be between 2 and 7",
    );
    error_if(
        &mut issues,
        rules.ai_period_ticks == 0,
        ScenarioIssueCode::AiPeriod,
        "rules.ai_period_ticks",
        "AI period must be nonzero",
    );
    error_if(
        &mut issues,
        rules.hazard_period_ticks == 0,
        ScenarioIssueCode::HazardPeriod,
        "rules.hazard_period_ticks",
        "hazard period must be nonzero",
    );
    error_if(
        &mut issues,
        rules.hazard_duration_ticks == 0 || rules.hazard_duration_ticks > rules.hazard_period_ticks,
        ScenarioIssueCode::HazardDuration,
        "rules.hazard_duration_ticks",
        "hazard duration must be nonzero and no greater than its period",
    );
    let hazard_bounds_fit = rules
        .hazard_first_tick
        .checked_add(rules.hazard_duration_ticks)
        .is_some()
        && rules
            .hazard_first_tick
            .checked_add(rules.hazard_period_ticks)
            .and_then(|next| next.checked_add(rules.hazard_duration_ticks))
            .is_some();
    error_if(
        &mut issues,
        !hazard_bounds_fit,
        ScenarioIssueCode::HazardBoundaryArithmetic,
        "rules.hazard_first_tick",
        "the first two hazard boundaries must fit u64",
    );
    error_if(
        &mut issues,
        rules.maximum_energy.0 == 0,
        ScenarioIssueCode::EnergyCap,
        "rules.maximum_energy",
        "maximum energy must be nonzero",
    );
    error_if(
        &mut issues,
        rules.maximum_defense.0 == 0,
        ScenarioIssueCode::DefenseCap,
        "rules.maximum_defense",
        "maximum defense must be nonzero",
    );
    error_if(
        &mut issues,
        rules.minimum_launch.0 == 0 || rules.minimum_launch.0 > rules.maximum_energy.0 / 2,
        ScenarioIssueCode::MinimumLaunch,
        "rules.minimum_launch",
        "minimum launch must be between 1 and half maximum energy",
    );
    error_if(
        &mut issues,
        rules.field_lower_refund.0 > rules.field_raise_cost.0
            || rules.field_raise_cost.0 > rules.maximum_energy.0,
        ScenarioIssueCode::FieldCostRelation,
        "rules.field_raise_cost",
        "refund must not exceed raise cost, and raise cost must fit the energy cap",
    );

    let divisor = u128::from(60_u32) * 10_000;
    let mut positions = BTreeMap::new();
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    let mut union_count = 0_u8;
    let mut helix_count = 0_u8;
    let mut choir_count = 0_u8;
    let mut launchable_union = false;
    for (index, world) in draft.worlds.iter().enumerate() {
        let prefix = format!("worlds[{index}]");
        error_if(
            &mut issues,
            WorldName::new(&world.name).is_err(),
            ScenarioIssueCode::WorldName,
            &format!("{prefix}.name"),
            "world name is not canonical uppercase ASCII",
        );
        error_if(
            &mut issues,
            !(0..=10_000).contains(&world.x) || !(0..=10_000).contains(&world.y),
            ScenarioIssueCode::WorldCoordinate,
            &format!("{prefix}.position"),
            "world coordinates must be between 0 and 10000",
        );
        error_if(
            &mut issues,
            world.atmosphere > 10 || world.hydrosphere > 10 || world.topology > 10,
            ScenarioIssueCode::WorldFieldLevel,
            &format!("{prefix}.fields"),
            "field levels must be between 0 and 10",
        );
        error_if(
            &mut issues,
            world.energy > rules.maximum_energy.0,
            ScenarioIssueCode::WorldEnergy,
            &format!("{prefix}.energy"),
            "starting energy exceeds its cap",
        );
        error_if(
            &mut issues,
            world.defense > rules.maximum_defense.0,
            ScenarioIssueCode::WorldDefense,
            &format!("{prefix}.defense"),
            "starting defense exceeds its cap",
        );
        let production = u128::from(world.base_output)
            .checked_mul(15_000)
            .and_then(|value| value.checked_add(divisor - 1));
        error_if(
            &mut issues,
            production.is_none_or(|value| value / divisor > u128::from(u64::MAX)),
            ScenarioIssueCode::ProductionArithmetic,
            &format!("{prefix}.base_output"),
            "worst-case production arithmetic does not fit",
        );
        let regeneration = world.base_regeneration.checked_add(1_000);
        let regeneration_numerator = regeneration
            .map(u128::from)
            .and_then(|value| value.checked_mul(10_000))
            .and_then(|value| value.checked_add(divisor - 1));
        error_if(
            &mut issues,
            regeneration.is_none()
                || regeneration_numerator
                    .is_none_or(|value| value / divisor > u128::from(u64::MAX)),
            ScenarioIssueCode::RegenerationArithmetic,
            &format!("{prefix}.base_regeneration"),
            "worst-case regeneration arithmetic does not fit",
        );
        if let Some(previous) = positions.insert((world.x, world.y), index) {
            push_issue(
                &mut issues,
                IssueSeverity::Error,
                ScenarioIssueCode::DuplicatePosition,
                &format!("{prefix}.position"),
                format!("world position duplicates worlds[{previous}]"),
            );
        }
        if let Some(previous) = names.insert(&world.name, index) {
            push_issue(
                &mut issues,
                IssueSeverity::Warning,
                ScenarioIssueCode::DuplicateName,
                &format!("{prefix}.name"),
                format!("world name duplicates worlds[{previous}]"),
            );
        }
        match world.owner {
            Some(Faction::Union) => {
                union_count += 1;
                launchable_union |= world.energy / 2 >= rules.minimum_launch.0;
            }
            Some(Faction::Helix) => helix_count += 1,
            Some(Faction::Choir) => choir_count += 1,
            None => {}
        }
    }
    for left in 0..WORLD_COUNT {
        for right in left + 1..WORLD_COUNT {
            let a = &draft.worlds[left];
            let b = &draft.worlds[right];
            let dx = i128::from(a.x) - i128::from(b.x);
            let dy = i128::from(a.y) - i128::from(b.y);
            let distance_squared = dx * dx + dy * dy;
            if distance_squared > 0 && distance_squared < 250 * 250 {
                push_issue(
                    &mut issues,
                    IssueSeverity::Warning,
                    ScenarioIssueCode::ClosePosition,
                    &format!("worlds[{right}].position"),
                    format!("world is within 250 sectors of worlds[{left}]"),
                );
            }
        }
    }
    error_if(
        &mut issues,
        union_count == 0,
        ScenarioIssueCode::MissingUnion,
        "worlds",
        "at least one world must belong to the Union",
    );
    error_if(
        &mut issues,
        [union_count, helix_count, choir_count]
            .into_iter()
            .any(|count| count >= rules.win_world_count),
        ScenarioIssueCode::InitialVictory,
        "worlds",
        "no faction may begin at the win threshold",
    );
    warning_if(
        &mut issues,
        helix_count == 0,
        ScenarioIssueCode::MissingHelix,
        "worlds",
        "no world belongs to the Helix",
    );
    warning_if(
        &mut issues,
        choir_count == 0,
        ScenarioIssueCode::MissingChoir,
        "worlds",
        "no world belongs to the Choir",
    );
    warning_if(
        &mut issues,
        !launchable_union,
        ScenarioIssueCode::NoLaunchableUnion,
        "worlds",
        "the Union has no source able to launch",
    );

    let effective_speed = u128::from(rules.base_fleet_speed)
        .checked_mul(14_000)
        .map(|value| value / 10_000);
    let speed_u64 = effective_speed.and_then(|value| u64::try_from(value).ok());
    let speed_numerator = speed_u64
        .map(u128::from)
        .and_then(|value| value.checked_mul(10_000))
        .and_then(|value| value.checked_add(divisor - 1));
    error_if(
        &mut issues,
        speed_u64.is_none()
            || speed_numerator.is_none_or(|value| value / divisor > u128::from(u64::MAX)),
        ScenarioIssueCode::FleetSpeedArithmetic,
        "rules.base_fleet_speed",
        "worst-case hydrosphere arithmetic must fit",
    );

    ValidationReport { issues }
}

fn error_if(
    issues: &mut Vec<ScenarioIssue>,
    condition: bool,
    code: ScenarioIssueCode,
    field: &str,
    message: &str,
) {
    if condition {
        push_issue(issues, IssueSeverity::Error, code, field, message);
    }
}

fn warning_if(
    issues: &mut Vec<ScenarioIssue>,
    condition: bool,
    code: ScenarioIssueCode,
    field: &str,
    message: &str,
) {
    if condition {
        push_issue(issues, IssueSeverity::Warning, code, field, message);
    }
}

fn push_issue(
    issues: &mut Vec<ScenarioIssue>,
    severity: IssueSeverity,
    code: ScenarioIssueCode,
    field: &str,
    message: impl Into<String>,
) {
    issues.push(ScenarioIssue {
        severity,
        code,
        field: field.into(),
        message: message.into(),
    });
}

const fn owner_tag(owner: Option<Faction>) -> u8 {
    match owner {
        None => 0,
        Some(Faction::Union) => 1,
        Some(Faction::Helix) => 2,
        Some(Faction::Choir) => 3,
    }
}

struct Fnv1a64(u64);

impl Fnv1a64 {
    const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;

    const fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    const fn finish(self) -> u64 {
        self.0
    }
}
