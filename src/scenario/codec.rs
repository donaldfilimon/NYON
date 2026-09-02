use serde::{Deserialize, Serialize};

use crate::game::model::{Energy, Faction, RulesV1, Strength};

use super::{
    FORMAT_VERSION, GENERATOR_VERSION, RULES_VERSION, ScenarioDraft, ScenarioV1, ValidationReport,
    WorldDraft,
};

pub const MAX_JSON_BYTES: usize = 65_536;

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("scenario JSON exceeds the 65536-byte limit")]
    TooLarge,
    #[error("invalid scenario JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported scenario format or rules version")]
    Version,
    #[error("{field} is not a canonical {kind} string")]
    CanonicalNumber {
        field: &'static str,
        kind: &'static str,
    },
    #[error(transparent)]
    Validation(#[from] ValidationReport),
}

pub fn encode(scenario: &ScenarioV1) -> Result<String, CodecError> {
    let draft = scenario.to_draft();
    let wire = WireScenario::from_draft(&draft);
    Ok(serde_json::to_string(&wire)?)
}

pub fn encode_draft(draft: &ScenarioDraft) -> Result<String, CodecError> {
    encode(&draft.validated()?)
}

pub fn decode(json: &str) -> Result<ScenarioV1, CodecError> {
    decode_draft(json)?.validated().map_err(CodecError::from)
}

pub fn decode_draft(json: &str) -> Result<ScenarioDraft, CodecError> {
    if json.len() > MAX_JSON_BYTES {
        return Err(CodecError::TooLarge);
    }
    let wire: WireScenario = serde_json::from_str(json)?;
    wire.into_draft()
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireScenario {
    format_version: u32,
    rules_version: u32,
    generator_version: u32,
    seed: String,
    rules: WireRules,
    worlds: [WireWorld; 7],
}

impl WireScenario {
    fn from_draft(draft: &ScenarioDraft) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            rules_version: RULES_VERSION,
            generator_version: GENERATOR_VERSION,
            seed: format!("{:016X}", draft.seed),
            rules: WireRules::from_rules(draft.rules),
            worlds: std::array::from_fn(|index| WireWorld::from_draft(&draft.worlds[index])),
        }
    }

    fn into_draft(self) -> Result<ScenarioDraft, CodecError> {
        if self.format_version != FORMAT_VERSION
            || self.rules_version != RULES_VERSION
            || self.generator_version != GENERATOR_VERSION
        {
            return Err(CodecError::Version);
        }
        let seed = parse_seed(&self.seed)?;
        let rules = self.rules.into_rules()?;
        let worlds = self
            .worlds
            .map(WireWorld::into_draft)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| CodecError::Version)?;
        Ok(ScenarioDraft {
            seed,
            rules,
            worlds,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireRules {
    tick_hz: u32,
    win_world_count: u8,
    ai_period_ticks: String,
    hazard_first_tick: String,
    hazard_period_ticks: String,
    hazard_duration_ticks: String,
    maximum_energy: String,
    maximum_defense: String,
    minimum_launch: String,
    field_raise_cost: String,
    field_lower_refund: String,
    base_fleet_speed: String,
}

impl WireRules {
    fn from_rules(rules: RulesV1) -> Self {
        Self {
            tick_hz: rules.tick_hz,
            win_world_count: rules.win_world_count,
            ai_period_ticks: rules.ai_period_ticks.to_string(),
            hazard_first_tick: rules.hazard_first_tick.to_string(),
            hazard_period_ticks: rules.hazard_period_ticks.to_string(),
            hazard_duration_ticks: rules.hazard_duration_ticks.to_string(),
            maximum_energy: rules.maximum_energy.0.to_string(),
            maximum_defense: rules.maximum_defense.0.to_string(),
            minimum_launch: rules.minimum_launch.0.to_string(),
            field_raise_cost: rules.field_raise_cost.0.to_string(),
            field_lower_refund: rules.field_lower_refund.0.to_string(),
            base_fleet_speed: rules.base_fleet_speed.to_string(),
        }
    }

    fn into_rules(self) -> Result<RulesV1, CodecError> {
        Ok(RulesV1 {
            tick_hz: self.tick_hz,
            win_world_count: self.win_world_count,
            ai_period_ticks: parse_u64("rules.ai_period_ticks", &self.ai_period_ticks)?,
            hazard_first_tick: parse_u64("rules.hazard_first_tick", &self.hazard_first_tick)?,
            hazard_period_ticks: parse_u64("rules.hazard_period_ticks", &self.hazard_period_ticks)?,
            hazard_duration_ticks: parse_u64(
                "rules.hazard_duration_ticks",
                &self.hazard_duration_ticks,
            )?,
            maximum_energy: Energy(parse_u64("rules.maximum_energy", &self.maximum_energy)?),
            maximum_defense: Strength(parse_u64("rules.maximum_defense", &self.maximum_defense)?),
            minimum_launch: Energy(parse_u64("rules.minimum_launch", &self.minimum_launch)?),
            field_raise_cost: Energy(parse_u64("rules.field_raise_cost", &self.field_raise_cost)?),
            field_lower_refund: Energy(parse_u64(
                "rules.field_lower_refund",
                &self.field_lower_refund,
            )?),
            base_fleet_speed: parse_u64("rules.base_fleet_speed", &self.base_fleet_speed)?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireWorld {
    name: String,
    x: i32,
    y: i32,
    owner: WireOwner,
    energy: String,
    defense: String,
    base_output: String,
    base_regeneration: String,
    atmosphere: u8,
    hydrosphere: u8,
    topology: u8,
}

impl WireWorld {
    fn from_draft(world: &WorldDraft) -> Self {
        Self {
            name: world.name.clone(),
            x: world.x,
            y: world.y,
            owner: WireOwner::from_owner(world.owner),
            energy: world.energy.to_string(),
            defense: world.defense.to_string(),
            base_output: world.base_output.to_string(),
            base_regeneration: world.base_regeneration.to_string(),
            atmosphere: world.atmosphere,
            hydrosphere: world.hydrosphere,
            topology: world.topology,
        }
    }

    fn into_draft(self) -> Result<WorldDraft, CodecError> {
        Ok(WorldDraft {
            name: self.name,
            x: self.x,
            y: self.y,
            owner: self.owner.into_owner(),
            energy: parse_u64("world.energy", &self.energy)?,
            defense: parse_u64("world.defense", &self.defense)?,
            base_output: parse_u64("world.base_output", &self.base_output)?,
            base_regeneration: parse_u64("world.base_regeneration", &self.base_regeneration)?,
            atmosphere: self.atmosphere,
            hydrosphere: self.hydrosphere,
            topology: self.topology,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum WireOwner {
    Neutral,
    Union,
    Helix,
    Choir,
}

impl WireOwner {
    const fn from_owner(owner: Option<Faction>) -> Self {
        match owner {
            None => Self::Neutral,
            Some(Faction::Union) => Self::Union,
            Some(Faction::Helix) => Self::Helix,
            Some(Faction::Choir) => Self::Choir,
        }
    }

    const fn into_owner(self) -> Option<Faction> {
        match self {
            Self::Neutral => None,
            Self::Union => Some(Faction::Union),
            Self::Helix => Some(Faction::Helix),
            Self::Choir => Some(Faction::Choir),
        }
    }
}

fn parse_seed(value: &str) -> Result<u64, CodecError> {
    if value.len() != 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
    {
        return Err(CodecError::CanonicalNumber {
            field: "seed",
            kind: "16-digit uppercase hexadecimal",
        });
    }
    u64::from_str_radix(value, 16).map_err(|_| CodecError::CanonicalNumber {
        field: "seed",
        kind: "16-digit uppercase hexadecimal",
    })
}

fn parse_u64(field: &'static str, value: &str) -> Result<u64, CodecError> {
    let canonical = value == "0"
        || (!value.starts_with('0')
            && !value.is_empty()
            && value.bytes().all(|b| b.is_ascii_digit()));
    if !canonical {
        return Err(CodecError::CanonicalNumber {
            field,
            kind: "unsigned decimal",
        });
    }
    value.parse().map_err(|_| CodecError::CanonicalNumber {
        field,
        kind: "unsigned decimal",
    })
}
