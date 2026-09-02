use std::collections::{BTreeMap, BTreeSet};

use crate::game::model::*;
use crate::scenario::{ReplayIdentity, ScenarioDraft, ScenarioFingerprint, ScenarioV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandRejection {
    WrongTick,
    DuplicateSequence,
    CampaignFinished,
    InvalidWorld,
    SameSourceAndTarget,
    SourceNotOwnedByIssuer,
    InsufficientEnergy,
    FieldAtLimit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandResult {
    Accepted {
        sequence: CommandSequence,
    },
    Rejected {
        sequence: CommandSequence,
        reason: CommandRejection,
    },
}

/// A non-mutating description of a command that would be accepted against the
/// current campaign state. Presentation code may display it but cannot enqueue
/// or reserve a command sequence through this type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandPreview {
    pub command: GameCommand,
    pub source: WorldId,
    pub destination: Option<WorldId>,
    pub launch_strength: Option<Strength>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameEvent {
    FleetLaunched(FleetId),
    WorldCaptured {
        world: WorldId,
        owner: Option<Faction>,
    },
    HazardStarted(ActiveHazard),
    HazardEnded(HazardKind),
    CampaignFinished(Outcome),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickReport {
    pub tick: Tick,
    pub command_results: Vec<CommandResult>,
    pub events: Vec<GameEvent>,
}

pub struct Simulation {
    state: Campaign,
    rules: RulesV1,
    pending: BTreeMap<Tick, Vec<CommandEnvelope>>,
    scenario_fingerprint: Option<ScenarioFingerprint>,
}

impl Simulation {
    pub fn generated(seed: u64, rules: RulesV1) -> Self {
        let scenario = ScenarioDraft::generated(seed, rules).validated().ok();
        let state = Campaign::new(seed, rules);
        Self {
            state,
            rules,
            pending: BTreeMap::new(),
            scenario_fingerprint: scenario.map(|value| value.fingerprint()),
        }
    }

    /// Builds a simulation around complete campaign truth for controlled fixtures.
    pub fn from_campaign(state: Campaign, rules: RulesV1) -> Self {
        Self {
            state,
            rules,
            pending: BTreeMap::new(),
            scenario_fingerprint: None,
        }
    }

    pub fn from_scenario(scenario: &ScenarioV1) -> Self {
        Self {
            state: Campaign::from_scenario(scenario),
            rules: scenario.rules(),
            pending: BTreeMap::new(),
            scenario_fingerprint: Some(scenario.fingerprint()),
        }
    }

    pub fn state(&self) -> &Campaign {
        &self.state
    }

    pub fn pending_command_count(&self) -> usize {
        self.pending.values().map(Vec::len).sum()
    }

    pub fn scenario_fingerprint(&self) -> Option<ScenarioFingerprint> {
        self.scenario_fingerprint
    }

    pub fn replay_identity(&self) -> Option<ReplayIdentity> {
        self.scenario_fingerprint.map(|scenario| ReplayIdentity {
            scenario,
            state: self.canonical_fingerprint(),
        })
    }

    pub fn enqueue(&mut self, envelope: CommandEnvelope) -> Result<(), CommandRejection> {
        if matches!(self.state.phase, Phase::Finished { .. }) {
            return Err(CommandRejection::CampaignFinished);
        }
        if envelope.tick != self.state.next_tick {
            return Err(CommandRejection::WrongTick);
        }
        let commands = self.pending.entry(envelope.tick).or_default();
        if commands
            .iter()
            .any(|queued| queued.sequence == envelope.sequence)
        {
            return Err(CommandRejection::DuplicateSequence);
        }
        commands.push(envelope);
        Ok(())
    }

    pub fn enqueue_next(
        &mut self,
        sequence: CommandSequence,
        command: GameCommand,
    ) -> Result<(), CommandRejection> {
        self.enqueue(CommandEnvelope {
            tick: self.state.next_tick,
            sequence,
            command,
        })
    }

    pub fn preview_command(
        &self,
        command: GameCommand,
    ) -> Result<CommandPreview, CommandRejection> {
        if matches!(self.state.phase, Phase::Finished { .. }) {
            return Err(CommandRejection::CampaignFinished);
        }
        match command {
            GameCommand::Launch {
                source,
                destination,
            } => {
                let plan = validate_launch(
                    &self.state,
                    &self.rules,
                    Faction::Union,
                    source,
                    destination,
                )?;
                Ok(CommandPreview {
                    command,
                    source,
                    destination: Some(destination),
                    launch_strength: Some(Strength(plan.launch_strength)),
                })
            }
            GameCommand::TuneField { world, .. } => {
                validate_tune(&self.state, &self.rules, Faction::Union, command)?;
                Ok(CommandPreview {
                    command,
                    source: world,
                    destination: None,
                    launch_strength: None,
                })
            }
        }
    }

    pub fn step(&mut self) -> TickReport {
        let tick = self.state.next_tick;
        if matches!(self.state.phase, Phase::Finished { .. }) {
            return TickReport {
                tick,
                command_results: Vec::new(),
                events: Vec::new(),
            };
        }

        let mut events = Vec::new();
        self.update_hazard(tick, &mut events);
        let snapshot = self.state.clone();

        let mut command_results = Vec::new();
        let mut launched_this_tick = BTreeSet::new();
        let mut commands = self.pending.remove(&tick).unwrap_or_default();
        commands.sort_by_key(|envelope| envelope.sequence);
        for envelope in commands {
            match self.apply_player_command(envelope.command) {
                Ok(event) => {
                    if let Some(GameEvent::FleetLaunched(id)) = event {
                        launched_this_tick.insert(id);
                        events.push(GameEvent::FleetLaunched(id));
                    }
                    command_results.push(CommandResult::Accepted {
                        sequence: envelope.sequence,
                    });
                }
                Err(reason) => command_results.push(CommandResult::Rejected {
                    sequence: envelope.sequence,
                    reason,
                }),
            }
        }

        if tick.0 > 0
            && self.rules.ai_period_ticks > 0
            && tick.0.is_multiple_of(self.rules.ai_period_ticks)
        {
            let helix = self.ai_intent(&snapshot, Faction::Helix);
            let choir = self.ai_intent(&snapshot, Faction::Choir);
            for intent in [helix, choir].into_iter().flatten() {
                if let Ok(id) =
                    self.launch_for_faction(intent.faction, intent.source, intent.destination)
                {
                    launched_this_tick.insert(id);
                    events.push(GameEvent::FleetLaunched(id));
                }
            }
        }

        self.accrue_worlds();
        let arrivals = self.advance_fleets(&launched_this_tick);
        self.resolve_arrivals(arrivals, &mut events);
        self.resolve_outcome(tick, &mut events);
        self.state.next_tick = Tick(tick.0.saturating_add(1));

        TickReport {
            tick,
            command_results,
            events,
        }
    }

    pub fn step_n(&mut self, count: u32) {
        for _ in 0..count {
            self.step();
        }
    }

    pub fn canonical_fingerprint(&self) -> u64 {
        let mut hash = Fnv1a64::new();
        hash.u32(self.state.rules_version);
        hash.u32(self.state.generator_version);
        hash.u64(self.state.seed);
        hash.u64(self.state.next_tick.0);
        match self.state.phase {
            Phase::Running => hash.u8(0),
            Phase::Finished {
                outcome: Outcome::FactionVictory { winner },
                ..
            } => {
                hash.u8(1);
                hash.u8(faction_tag(winner));
            }
            Phase::Finished {
                outcome: Outcome::PlayerEliminated,
                ..
            } => hash.u8(2),
        }
        hash.u64(self.state.next_fleet_id.0);
        hash.u64(self.state.next_hazard_index);
        match self.state.active_hazard {
            None => hash.u8(0),
            Some(hazard) => {
                hash.u8(1);
                hash.u64(hazard.event_index);
                hash.u8(hazard_kind_tag(hazard.kind));
                hash.u8(field_tag(hazard.affected_field));
                hash.u64(hazard.start.0);
                hash.u64(hazard.end.0);
            }
        }
        let mut worlds: Vec<&World> = self.state.worlds.iter().collect();
        worlds.sort_by_key(|world| world.id);
        for world in worlds {
            hash.u8(world.id.0);
            hash.i32(world.position.x);
            hash.i32(world.position.y);
            hash.u8(owner_tag(world.owner));
            hash.u64(world.defense.0);
            hash.u64(world.energy.0);
            hash.u64(world.base_output_per_second.0);
            hash.u64(world.base_regeneration_per_second.0);
            hash.u8(world.fields.atmosphere);
            hash.u8(world.fields.hydrosphere);
            hash.u8(world.fields.topology);
            hash.u64(world.production_remainder);
            hash.u64(world.regeneration_remainder);
        }
        hash.u64(self.state.fleets.len() as u64);
        let mut fleets: Vec<&Fleet> = self.state.fleets.iter().collect();
        fleets.sort_by_key(|fleet| fleet.id);
        for fleet in fleets {
            hash.u64(fleet.id.0);
            hash.u8(faction_tag(fleet.faction));
            hash.u8(fleet.source.0);
            hash.u8(fleet.destination.0);
            hash.u64(fleet.strength.0);
            hash.u64(fleet.route_length);
            hash.u64(fleet.progress);
            hash.u64(fleet.speed_per_second);
            hash.u8(fleet.hydrosphere_level);
            hash.u64(fleet.movement_remainder);
        }
        hash.finish()
    }

    fn update_hazard(&mut self, tick: Tick, events: &mut Vec<GameEvent>) {
        if self
            .state
            .active_hazard
            .is_some_and(|hazard| hazard.end == tick)
        {
            let ended = self.state.active_hazard.take().expect("hazard exists");
            events.push(GameEvent::HazardEnded(ended.kind));
        }

        let event_index = self.state.next_hazard_index;
        let start = self
            .rules
            .hazard_first_tick
            .saturating_add(event_index.saturating_mul(self.rules.hazard_period_ticks));
        if tick.0 == start {
            let hazard = ActiveHazard {
                event_index,
                kind: if event_index.is_multiple_of(2) {
                    HazardKind::IonStorm
                } else {
                    HazardKind::GravityTide
                },
                affected_field: match splitmix64_once(self.state.seed ^ event_index) % 3 {
                    0 => FieldKind::Atmosphere,
                    1 => FieldKind::Hydrosphere,
                    _ => FieldKind::Topology,
                },
                start: tick,
                end: Tick(start.saturating_add(self.rules.hazard_duration_ticks)),
            };
            self.state.active_hazard = Some(hazard);
            self.state.next_hazard_index = event_index.saturating_add(1);
            events.push(GameEvent::HazardStarted(hazard));
        }
    }

    fn apply_player_command(
        &mut self,
        command: GameCommand,
    ) -> Result<Option<GameEvent>, CommandRejection> {
        match command {
            GameCommand::Launch {
                source,
                destination,
            } => self
                .launch_for_faction(Faction::Union, source, destination)
                .map(|id| Some(GameEvent::FleetLaunched(id))),
            GameCommand::TuneField {
                world,
                field,
                adjustment,
            } => {
                self.tune_field(Faction::Union, world, field, adjustment)?;
                Ok(None)
            }
        }
    }

    fn launch_for_faction(
        &mut self,
        faction: Faction,
        source: WorldId,
        destination: WorldId,
    ) -> Result<FleetId, CommandRejection> {
        let plan = validate_launch(&self.state, &self.rules, faction, source, destination)?;
        let id = self.state.next_fleet_id;
        self.state.next_fleet_id = FleetId(id.0.saturating_add(1));
        self.state.worlds[plan.source_index].energy = Energy(
            self.state.worlds[plan.source_index]
                .energy
                .0
                .saturating_sub(plan.launch_strength),
        );
        self.state.fleets.push(Fleet {
            id,
            faction,
            source,
            destination,
            strength: Strength(plan.launch_strength),
            route_length: plan.route_length,
            progress: 0,
            speed_per_second: plan.speed_per_second,
            hydrosphere_level: plan.hydrosphere_level,
            movement_remainder: 0,
        });
        self.state.fleets.sort_by_key(|fleet| fleet.id);
        Ok(id)
    }

    fn tune_field(
        &mut self,
        faction: Faction,
        id: WorldId,
        field: FieldKind,
        adjustment: FieldAdjustment,
    ) -> Result<(), CommandRejection> {
        let command = GameCommand::TuneField {
            world: id,
            field,
            adjustment,
        };
        let plan = validate_tune(&self.state, &self.rules, faction, command)?;
        let world = &mut self.state.worlds[plan.world_index];
        world.energy = Energy(plan.energy);
        world.fields.set(plan.field, plan.level);
        Ok(())
    }

    fn accrue_worlds(&mut self) {
        let atmosphere_hazard = self
            .state
            .active_hazard
            .is_some_and(|hazard| hazard.affected_field == FieldKind::Atmosphere);
        let topology_hazard = self
            .state
            .active_hazard
            .is_some_and(|hazard| hazard.affected_field == FieldKind::Topology);
        let divisor = u128::from(self.rules.tick_hz) * 10_000;

        for world in &mut self.state.worlds {
            if world.owner.is_none() {
                continue;
            }

            let atmosphere_bonus = u64::from(world.fields.atmosphere) * 500;
            let atmosphere_bonus = if atmosphere_hazard {
                atmosphere_bonus * 5_000 / 10_000
            } else {
                atmosphere_bonus
            };
            let production_numerator = u128::from(world.production_remainder)
                + u128::from(world.base_output_per_second.0)
                    * u128::from(10_000 + atmosphere_bonus);
            let production_gain = (production_numerator / divisor) as u64;
            let production_remainder = (production_numerator % divisor) as u64;
            if world.energy.0 >= self.rules.maximum_energy.0
                || world.energy.0.saturating_add(production_gain) >= self.rules.maximum_energy.0
            {
                world.energy = self.rules.maximum_energy;
                world.production_remainder = 0;
            } else {
                world.energy = Energy(world.energy.0 + production_gain);
                world.production_remainder = production_remainder;
            }

            let topology_bonus = u64::from(world.fields.topology) * 100;
            let topology_bonus = if topology_hazard {
                topology_bonus * 5_000 / 10_000
            } else {
                topology_bonus
            };
            let regeneration_rate = world
                .base_regeneration_per_second
                .0
                .saturating_add(topology_bonus);
            let regeneration_numerator =
                u128::from(world.regeneration_remainder) + u128::from(regeneration_rate) * 10_000;
            let regeneration_gain = (regeneration_numerator / divisor) as u64;
            let regeneration_remainder = (regeneration_numerator % divisor) as u64;
            if world.defense.0 >= self.rules.maximum_defense.0
                || world.defense.0.saturating_add(regeneration_gain) >= self.rules.maximum_defense.0
            {
                world.defense = self.rules.maximum_defense;
                world.regeneration_remainder = 0;
            } else {
                world.defense = Strength(world.defense.0 + regeneration_gain);
                world.regeneration_remainder = regeneration_remainder;
            }
        }
    }

    fn advance_fleets(
        &mut self,
        launched_this_tick: &BTreeSet<FleetId>,
    ) -> BTreeMap<WorldId, [u64; 3]> {
        let hydrosphere_hazard = self
            .state
            .active_hazard
            .is_some_and(|hazard| hazard.affected_field == FieldKind::Hydrosphere);
        let divisor = u128::from(self.rules.tick_hz) * 10_000;
        let mut arrivals: BTreeMap<WorldId, [u64; 3]> = BTreeMap::new();
        let mut remaining = Vec::with_capacity(self.state.fleets.len());

        for mut fleet in self.state.fleets.drain(..) {
            if !launched_this_tick.contains(&fleet.id) {
                let rate = if hydrosphere_hazard {
                    fleet_speed(self.rules.base_fleet_speed, fleet.hydrosphere_level, true)
                } else {
                    fleet.speed_per_second
                };
                let numerator = u128::from(fleet.movement_remainder) + u128::from(rate) * 10_000;
                let gain = (numerator / divisor) as u64;
                fleet.movement_remainder = (numerator % divisor) as u64;
                fleet.progress = fleet.progress.saturating_add(gain);
            }

            if !launched_this_tick.contains(&fleet.id) && fleet.progress >= fleet.route_length {
                let strengths = arrivals.entry(fleet.destination).or_insert([0; 3]);
                let index = faction_index(fleet.faction);
                strengths[index] = strengths[index].saturating_add(fleet.strength.0);
            } else {
                remaining.push(fleet);
            }
        }
        remaining.sort_by_key(|fleet| fleet.id);
        self.state.fleets = remaining;
        arrivals
    }

    fn resolve_arrivals(
        &mut self,
        arrivals: BTreeMap<WorldId, [u64; 3]>,
        events: &mut Vec<GameEvent>,
    ) {
        for (destination, arriving) in arrivals {
            let Some(index) = world_index(&self.state, destination) else {
                continue;
            };
            let world = &mut self.state.worlds[index];
            let previous_owner = world.owner;
            let mut faction_forces = arriving.map(u128::from);
            let neutral_force;
            if let Some(owner) = previous_owner {
                faction_forces[faction_index(owner)] = faction_forces[faction_index(owner)]
                    .saturating_add(u128::from(world.defense.0));
                neutral_force = 0;
            } else {
                neutral_force = u128::from(world.defense.0);
            }

            let total_faction_force: u128 = faction_forces.iter().copied().sum();
            let strict_winner = faction_forces
                .iter()
                .enumerate()
                .find(|(_, force)| {
                    **force > neutral_force + total_faction_force.saturating_sub(**force)
                })
                .map(|(index, force)| (index_faction(index), *force));

            let (new_owner, new_defense) = if let Some((winner, winner_force)) = strict_winner {
                let opposition = neutral_force + total_faction_force.saturating_sub(winner_force);
                let difference = winner_force.saturating_sub(opposition);
                (
                    Some(winner),
                    difference.clamp(1, u128::from(self.rules.maximum_defense.0)) as u64,
                )
            } else {
                let incumbent_force = previous_owner
                    .map(|owner| faction_forces[faction_index(owner)])
                    .unwrap_or(neutral_force);
                let incumbent_holds = faction_forces
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        previous_owner.is_none_or(|owner| faction_index(owner) != *index)
                    })
                    .all(|(_, force)| incumbent_force >= *force);
                (
                    if incumbent_holds {
                        previous_owner
                    } else {
                        None
                    },
                    1,
                )
            };

            world.owner = new_owner;
            world.defense = Strength(new_defense);
            if new_owner != previous_owner {
                world.energy = Energy(0);
                events.push(GameEvent::WorldCaptured {
                    world: destination,
                    owner: new_owner,
                });
            }
        }
    }

    fn resolve_outcome(&mut self, tick: Tick, events: &mut Vec<GameEvent>) {
        let outcome = [Faction::Union, Faction::Helix, Faction::Choir]
            .into_iter()
            .find(|faction| {
                self.state.controlled_count(*faction) >= usize::from(self.rules.win_world_count)
            })
            .map(|winner| Outcome::FactionVictory { winner })
            .or_else(|| {
                (self.state.controlled_count(Faction::Union) == 0
                    && self.state.fleet_count(Faction::Union) == 0)
                    .then_some(Outcome::PlayerEliminated)
            });
        if let Some(outcome) = outcome {
            self.state.phase = Phase::Finished { at: tick, outcome };
            events.push(GameEvent::CampaignFinished(outcome));
        }
    }

    fn ai_intent(&self, snapshot: &Campaign, faction: Faction) -> Option<AiIntent> {
        let threatened = snapshot
            .worlds
            .iter()
            .filter(|world| world.owner == Some(faction))
            .filter_map(|world| {
                let friendly = snapshot
                    .fleets
                    .iter()
                    .filter(|fleet| fleet.destination == world.id && fleet.faction == faction)
                    .map(|fleet| u128::from(fleet.strength.0))
                    .sum::<u128>();
                let hostile_fleets: Vec<&Fleet> = snapshot
                    .fleets
                    .iter()
                    .filter(|fleet| fleet.destination == world.id && fleet.faction != faction)
                    .collect();
                let hostile = hostile_fleets
                    .iter()
                    .map(|fleet| u128::from(fleet.strength.0))
                    .sum::<u128>();
                let defense = u128::from(world.defense.0);
                let deficit = hostile.saturating_sub(defense.saturating_add(friendly));
                (deficit > 0).then(|| {
                    let remaining = hostile_fleets
                        .iter()
                        .map(|fleet| fleet.route_length.saturating_sub(fleet.progress))
                        .min()
                        .unwrap_or(u64::MAX);
                    (world, deficit, remaining)
                })
            })
            .max_by(|left, right| {
                left.1
                    .cmp(&right.1)
                    .then_with(|| right.2.cmp(&left.2))
                    .then_with(|| right.0.id.cmp(&left.0.id))
            });

        if let Some((target, _, _)) = threatened {
            let source = snapshot
                .worlds
                .iter()
                .filter(|world| world.owner == Some(faction) && world.id != target.id)
                .filter(|world| world.energy.0 / 2 >= self.rules.minimum_launch.0)
                .min_by_key(|world| {
                    (
                        route_length(world.position, target.position),
                        std::cmp::Reverse(world.energy.0 / 2),
                        world.id,
                    )
                });
            if let Some(source) = source {
                return Some(AiIntent {
                    faction,
                    source: source.id,
                    destination: target.id,
                });
            }
        }

        let target = snapshot
            .worlds
            .iter()
            .filter(|world| world.owner != Some(faction))
            .min_by_key(|world| {
                let owner_inbound = world.owner.map_or(0, |owner| {
                    snapshot
                        .fleets
                        .iter()
                        .filter(|fleet| fleet.destination == world.id && fleet.faction == owner)
                        .map(|fleet| fleet.strength.0)
                        .fold(0_u64, u64::saturating_add)
                });
                (world.defense.0.saturating_add(owner_inbound), world.id)
            })?;
        let source = snapshot
            .worlds
            .iter()
            .filter(|world| world.owner == Some(faction))
            .filter(|world| world.energy.0 / 2 >= self.rules.minimum_launch.0)
            .min_by_key(|world| {
                (
                    std::cmp::Reverse(world.energy.0 / 2),
                    route_length(world.position, target.position),
                    world.id,
                )
            })?;
        Some(AiIntent {
            faction,
            source: source.id,
            destination: target.id,
        })
    }
}

#[derive(Clone, Copy)]
struct LaunchPlan {
    source_index: usize,
    launch_strength: u64,
    route_length: u64,
    speed_per_second: u64,
    hydrosphere_level: u8,
}

fn validate_launch(
    campaign: &Campaign,
    rules: &RulesV1,
    faction: Faction,
    source: WorldId,
    destination: WorldId,
) -> Result<LaunchPlan, CommandRejection> {
    let source_index = world_index(campaign, source).ok_or(CommandRejection::InvalidWorld)?;
    let destination_index =
        world_index(campaign, destination).ok_or(CommandRejection::InvalidWorld)?;
    if source == destination {
        return Err(CommandRejection::SameSourceAndTarget);
    }
    let source_world = &campaign.worlds[source_index];
    if source_world.owner != Some(faction) {
        return Err(CommandRejection::SourceNotOwnedByIssuer);
    }
    let launch_strength = source_world.energy.0 / 2;
    if launch_strength < rules.minimum_launch.0 {
        return Err(CommandRejection::InsufficientEnergy);
    }
    let destination_world = &campaign.worlds[destination_index];
    let hydrosphere_level = source_world.fields.hydrosphere;
    Ok(LaunchPlan {
        source_index,
        launch_strength,
        route_length: route_length(source_world.position, destination_world.position),
        speed_per_second: fleet_speed(rules.base_fleet_speed, hydrosphere_level, false),
        hydrosphere_level,
    })
}

#[derive(Clone, Copy)]
struct TunePlan {
    world_index: usize,
    field: FieldKind,
    level: u8,
    energy: u64,
}

fn validate_tune(
    campaign: &Campaign,
    rules: &RulesV1,
    faction: Faction,
    command: GameCommand,
) -> Result<TunePlan, CommandRejection> {
    let GameCommand::TuneField {
        world: id,
        field,
        adjustment,
    } = command
    else {
        unreachable!("tuning validation requires a tuning command");
    };
    let world_index = world_index(campaign, id).ok_or(CommandRejection::InvalidWorld)?;
    let world = &campaign.worlds[world_index];
    if world.owner != Some(faction) {
        return Err(CommandRejection::SourceNotOwnedByIssuer);
    }
    let level = world.fields.get(field);
    let (level, energy) = match adjustment {
        FieldAdjustment::Increase => {
            if level >= 10 {
                return Err(CommandRejection::FieldAtLimit);
            }
            if world.energy.0 < rules.field_raise_cost.0 {
                return Err(CommandRejection::InsufficientEnergy);
            }
            (level + 1, world.energy.0 - rules.field_raise_cost.0)
        }
        FieldAdjustment::Decrease => {
            if level == 0 {
                return Err(CommandRejection::FieldAtLimit);
            }
            (
                level - 1,
                world
                    .energy
                    .0
                    .saturating_add(rules.field_lower_refund.0)
                    .min(rules.maximum_energy.0),
            )
        }
    };
    Ok(TunePlan {
        world_index,
        field,
        level,
        energy,
    })
}

#[derive(Clone, Copy)]
struct AiIntent {
    faction: Faction,
    source: WorldId,
    destination: WorldId,
}

pub fn fleet_progress_ratio(fleet: &Fleet) -> (u64, u64) {
    (
        fleet.progress.min(fleet.route_length),
        fleet.route_length.max(1),
    )
}

fn world_index(campaign: &Campaign, id: WorldId) -> Option<usize> {
    campaign.worlds.iter().position(|world| world.id == id)
}

/// Returns floor(sqrt(dx^2 + dy^2)) using integer arithmetic only.
fn route_length(source: SectorPoint, destination: SectorPoint) -> u64 {
    let dx = i128::from(destination.x) - i128::from(source.x);
    let dy = i128::from(destination.y) - i128::from(source.y);
    integer_sqrt((dx * dx + dy * dy) as u128) as u64
}

fn integer_sqrt(value: u128) -> u128 {
    if value < 2 {
        return value;
    }
    let mut low = 1;
    let mut high = value.min(u128::from(u64::MAX)) + 1;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle;
        }
    }
    low
}

fn fleet_speed(base_speed: u64, hydrosphere_level: u8, hazard_active: bool) -> u64 {
    let bonus_basis_points = u64::from(hydrosphere_level) * 400;
    let bonus_basis_points = if hazard_active {
        bonus_basis_points * 5_000 / 10_000
    } else {
        bonus_basis_points
    };
    ((u128::from(base_speed) * u128::from(10_000 + bonus_basis_points)) / 10_000) as u64
}

fn splitmix64_once(seed: u64) -> u64 {
    let state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

const fn faction_index(faction: Faction) -> usize {
    match faction {
        Faction::Union => 0,
        Faction::Helix => 1,
        Faction::Choir => 2,
    }
}

const fn index_faction(index: usize) -> Faction {
    match index {
        0 => Faction::Union,
        1 => Faction::Helix,
        _ => Faction::Choir,
    }
}

const fn faction_tag(faction: Faction) -> u8 {
    match faction {
        Faction::Union => 1,
        Faction::Helix => 2,
        Faction::Choir => 3,
    }
}

const fn owner_tag(owner: Option<Faction>) -> u8 {
    match owner {
        None => 0,
        Some(faction) => faction_tag(faction),
    }
}

const fn field_tag(field: FieldKind) -> u8 {
    match field {
        FieldKind::Atmosphere => 0,
        FieldKind::Hydrosphere => 1,
        FieldKind::Topology => 2,
    }
}

const fn hazard_kind_tag(kind: HazardKind) -> u8 {
    match kind {
        HazardKind::IonStorm => 0,
        HazardKind::GravityTide => 1,
    }
}

struct Fnv1a64(u64);

impl Fnv1a64 {
    const fn new() -> Self {
        Self(0xCBF2_9CE4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01B3);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn winning_fixture(winner: Faction) -> Simulation {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        for world in &mut campaign.worlds[..5] {
            world.owner = Some(winner);
        }
        Simulation::from_campaign(campaign, rules)
    }

    fn arriving_fleet(id: u64, faction: Faction, destination: WorldId, strength: u64) -> Fleet {
        Fleet {
            id: FleetId(id),
            faction,
            source: WorldId(6),
            destination,
            strength: Strength(strength),
            route_length: 1,
            progress: 1,
            speed_per_second: 0,
            hydrosphere_level: 0,
            movement_remainder: 0,
        }
    }

    #[test]
    fn valid_launch_spends_exact_cost_and_allocates_monotonic_fleet_id() {
        let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        simulation
            .enqueue_next(
                CommandSequence(0),
                GameCommand::Launch {
                    source: WorldId(0),
                    destination: WorldId(2),
                },
            )
            .unwrap();
        let before = simulation.state().worlds[0].energy;
        let report = simulation.step();
        assert_eq!(
            report.command_results[0],
            CommandResult::Accepted {
                sequence: CommandSequence(0)
            }
        );
        assert_eq!(
            simulation.state().worlds[0].energy,
            Energy(before.0 - before.0 / 2 + 62)
        );
        assert_eq!(
            simulation.state().fleets[0].strength,
            Strength(before.0 / 2)
        );
        assert_eq!(simulation.state().fleets[0].id, FleetId(0));
    }

    #[test]
    fn sixty_ticks_preserve_fractional_production_without_drift() {
        let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        let before = simulation.state().worlds[0].energy;
        simulation.step_n(60);
        assert!(simulation.state().worlds[0].energy > before);
        assert!(simulation.state().worlds[0].production_remainder < 600_000);
    }

    #[test]
    fn finished_campaign_is_absorbing() {
        let mut simulation = winning_fixture(Faction::Union);
        simulation.step();
        let fingerprint = simulation.canonical_fingerprint();
        simulation.step_n(600);
        assert_eq!(simulation.canonical_fingerprint(), fingerprint);
    }

    #[test]
    fn default_campaign_digest_is_the_rules_v1_golden_value() {
        let simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        assert_eq!(simulation.canonical_fingerprint(), 0x67D9_6E98_3D6C_9330);
    }

    #[test]
    fn ai_first_evaluates_at_tick_60() {
        let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        simulation.step_n(60);
        assert!(simulation.state().fleets.is_empty());
        simulation.step();
        assert_eq!(simulation.state().fleets.len(), 2);
        assert_eq!(simulation.state().fleets[0].faction, Faction::Helix);
        assert_eq!(simulation.state().fleets[1].faction, Faction::Choir);
    }

    #[test]
    fn ai_ties_end_at_lowest_world_id() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        for world in &mut campaign.worlds {
            world.owner = None;
            world.defense = Strength(10_000);
            world.energy = Energy(0);
        }
        campaign.worlds[1].owner = Some(Faction::Helix);
        campaign.worlds[1].energy = Energy(80_000);
        campaign.worlds[2].owner = Some(Faction::Choir);
        campaign.worlds[6].owner = Some(Faction::Union);
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation.step_n(60);
        simulation.step();
        let helix = simulation
            .state()
            .fleets
            .iter()
            .find(|fleet| fleet.faction == Faction::Helix)
            .expect("Helix should launch on its first cadence");
        assert_eq!(helix.destination, WorldId(0));
    }

    #[test]
    fn field_round_trip_loses_energy() {
        let mut tuned = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        let mut control = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        tuned
            .enqueue_next(
                CommandSequence(0),
                GameCommand::TuneField {
                    world: WorldId(0),
                    field: FieldKind::Topology,
                    adjustment: FieldAdjustment::Decrease,
                },
            )
            .unwrap();
        tuned
            .enqueue_next(
                CommandSequence(1),
                GameCommand::TuneField {
                    world: WorldId(0),
                    field: FieldKind::Topology,
                    adjustment: FieldAdjustment::Increase,
                },
            )
            .unwrap();
        tuned.step();
        control.step();
        assert_eq!(
            control.state().worlds[0].energy.0 - tuned.state().worlds[0].energy.0,
            10_000
        );
        assert_eq!(
            tuned.state().worlds[0].fields.topology,
            control.state().worlds[0].fields.topology
        );
    }

    #[test]
    fn exact_force_combat_keeps_one_defense() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        let target = &mut campaign.worlds[0];
        target.owner = Some(Faction::Union);
        target.defense = Strength(20_000);
        target.base_regeneration_per_second = Strength(0);
        target.fields.topology = 0;
        campaign.fleets = vec![arriving_fleet(0, Faction::Helix, WorldId(0), 20_000)];
        campaign.next_fleet_id = FleetId(1);
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation.step();
        assert_eq!(simulation.state().worlds[0].owner, Some(Faction::Union));
        assert_eq!(simulation.state().worlds[0].defense, Strength(1));
    }

    #[test]
    fn attacker_tie_neutralizes_world() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        let target = &mut campaign.worlds[0];
        target.owner = Some(Faction::Union);
        target.defense = Strength(1);
        target.base_regeneration_per_second = Strength(0);
        target.fields.topology = 0;
        campaign.fleets = vec![
            arriving_fleet(0, Faction::Helix, WorldId(0), 20_000),
            arriving_fleet(1, Faction::Choir, WorldId(0), 20_000),
        ];
        campaign.next_fleet_id = FleetId(2);
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation.step();
        assert_eq!(simulation.state().worlds[0].owner, None);
        assert_eq!(simulation.state().worlds[0].defense, Strength(1));
    }

    #[test]
    fn capture_clears_energy_and_retains_fields() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        let target = &mut campaign.worlds[0];
        target.owner = Some(Faction::Union);
        target.defense = Strength(1_000);
        target.energy = Energy(50_000);
        target.base_regeneration_per_second = Strength(0);
        target.fields = FieldLevels::new(2, 4, 0);
        campaign.fleets = vec![arriving_fleet(0, Faction::Helix, WorldId(0), 5_000)];
        campaign.next_fleet_id = FleetId(1);
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation.step();
        let target = &simulation.state().worlds[0];
        assert_eq!(target.owner, Some(Faction::Helix));
        assert_eq!(target.defense, Strength(4_000));
        assert_eq!(target.energy, Energy(0));
        assert_eq!(target.fields, FieldLevels::new(2, 4, 0));
    }

    #[test]
    fn fifth_world_finishes_for_each_faction() {
        for winner in [Faction::Union, Faction::Helix, Faction::Choir] {
            let rules = RulesV1::default();
            let mut campaign = Campaign::new(DEFAULT_SEED, rules);
            for (index, world) in campaign.worlds.iter_mut().enumerate() {
                world.owner = (index < 5).then_some(winner);
            }
            let mut simulation = Simulation::from_campaign(campaign, rules);
            simulation.step();
            assert_eq!(
                simulation.state().phase,
                Phase::Finished {
                    at: Tick(0),
                    outcome: Outcome::FactionVictory { winner }
                }
            );
        }
    }

    #[test]
    fn rejected_command_preserves_fingerprint() {
        let mut rejected = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        let mut control = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        rejected
            .enqueue_next(
                CommandSequence(0),
                GameCommand::Launch {
                    source: WorldId(0),
                    destination: WorldId(0),
                },
            )
            .unwrap();
        let report = rejected.step();
        control.step();
        assert_eq!(
            report.command_results,
            vec![CommandResult::Rejected {
                sequence: CommandSequence(0),
                reason: CommandRejection::SameSourceAndTarget
            }]
        );
        assert_eq!(
            rejected.canonical_fingerprint(),
            control.canonical_fingerprint()
        );
    }

    #[test]
    fn same_tick_commands_execute_in_sequence_order() {
        let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
        simulation
            .enqueue_next(
                CommandSequence(1),
                GameCommand::Launch {
                    source: WorldId(0),
                    destination: WorldId(3),
                },
            )
            .unwrap();
        simulation
            .enqueue_next(
                CommandSequence(0),
                GameCommand::Launch {
                    source: WorldId(0),
                    destination: WorldId(4),
                },
            )
            .unwrap();
        let report = simulation.step();
        assert_eq!(
            report.command_results,
            vec![
                CommandResult::Accepted {
                    sequence: CommandSequence(0)
                },
                CommandResult::Accepted {
                    sequence: CommandSequence(1)
                }
            ]
        );
        assert_eq!(simulation.state().fleets[0].destination, WorldId(4));
        assert_eq!(simulation.state().fleets[0].strength, Strength(40_000));
        assert_eq!(simulation.state().fleets[1].destination, WorldId(3));
        assert_eq!(simulation.state().fleets[1].strength, Strength(20_000));
    }

    #[test]
    fn newly_launched_fleet_cannot_arrive_on_launch_tick() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        campaign.worlds[0].position = SectorPoint { x: 100, y: 100 };
        campaign.worlds[3].position = SectorPoint { x: 100, y: 100 };
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation
            .enqueue_next(
                CommandSequence(0),
                GameCommand::Launch {
                    source: WorldId(0),
                    destination: WorldId(3),
                },
            )
            .unwrap();
        simulation.step();
        assert_eq!(simulation.state().fleets.len(), 1);
        simulation.step();
        assert!(simulation.state().fleets.is_empty());
        assert_eq!(simulation.state().worlds[3].owner, None);
        assert_eq!(simulation.state().worlds[3].defense, Strength(1));
    }

    #[test]
    fn both_ai_factions_use_same_start_of_tick_snapshot() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        campaign.next_tick = Tick(60);
        for (index, world) in campaign.worlds.iter_mut().enumerate() {
            world.position = SectorPoint {
                x: i32::try_from(index).unwrap() * 100,
                y: 0,
            };
            world.owner = None;
            world.defense = Strength(90_000);
            world.energy = Energy(0);
        }
        campaign.worlds[0].owner = Some(Faction::Union);
        campaign.worlds[0].defense = Strength(2_000);
        campaign.worlds[1].owner = Some(Faction::Helix);
        campaign.worlds[1].defense = Strength(3_000);
        campaign.worlds[1].energy = Energy(80_000);
        campaign.worlds[2].owner = Some(Faction::Choir);
        campaign.worlds[2].defense = Strength(1_000);
        campaign.worlds[2].energy = Energy(80_000);
        campaign.worlds[3].owner = Some(Faction::Choir);
        campaign.worlds[3].energy = Energy(80_000);
        let mut simulation = Simulation::from_campaign(campaign, rules);
        simulation.step();
        let helix = simulation
            .state()
            .fleets
            .iter()
            .find(|fleet| fleet.faction == Faction::Helix)
            .unwrap();
        let choir = simulation
            .state()
            .fleets
            .iter()
            .find(|fleet| fleet.faction == Faction::Choir)
            .unwrap();
        assert_eq!(helix.destination, WorldId(2));
        assert_eq!(choir.source, WorldId(2));
        assert_eq!(choir.destination, WorldId(0));
    }

    #[test]
    fn atmosphere_hazard_halves_only_the_field_bonus() {
        let rules = RulesV1::default();
        let mut campaign = Campaign::new(DEFAULT_SEED, rules);
        campaign.next_tick = Tick(1);
        for world in &mut campaign.worlds {
            world.owner = None;
        }
        let target = &mut campaign.worlds[0];
        target.owner = Some(Faction::Union);
        target.energy = Energy(0);
        target.base_output_per_second = Energy(600);
        target.fields.atmosphere = 10;

        let mut ordinary = Simulation::from_campaign(campaign.clone(), rules);
        campaign.active_hazard = Some(ActiveHazard {
            event_index: 0,
            kind: HazardKind::IonStorm,
            affected_field: FieldKind::Atmosphere,
            start: Tick(0),
            end: Tick(100),
        });
        campaign.next_hazard_index = 1;
        let mut hazardous = Simulation::from_campaign(campaign, rules);
        ordinary.step();
        hazardous.step();
        assert_eq!(ordinary.state().worlds[0].energy, Energy(15));
        assert_eq!(hazardous.state().worlds[0].energy, Energy(12));
    }
}
