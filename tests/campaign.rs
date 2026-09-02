use intergalactic_warfare::game::{model::*, simulation::*};

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

fn simultaneous_arrival_fixture(reverse: bool) -> Campaign {
    let rules = RulesV1::default();
    let mut campaign = Campaign::new(DEFAULT_SEED, rules);
    let target = &mut campaign.worlds[0];
    target.owner = Some(Faction::Union);
    target.defense = Strength(10_000);
    target.base_regeneration_per_second = Strength(0);
    target.fields.topology = 0;
    campaign.fleets = vec![
        arriving_fleet(0, Faction::Helix, WorldId(0), 20_000),
        arriving_fleet(1, Faction::Choir, WorldId(0), 10_000),
    ];
    if reverse {
        campaign.fleets.reverse();
    }
    campaign.next_fleet_id = FleetId(2);
    campaign
}

fn step_fixture(campaign: Campaign) -> Simulation {
    let mut simulation = Simulation::from_campaign(campaign, RulesV1::default());
    simulation.step();
    simulation
}

fn scripted_simulation() -> Simulation {
    let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    simulation
        .enqueue_next(
            CommandSequence(0),
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(3),
            },
        )
        .unwrap();
    simulation
}

fn union_fleet_only_fixture() -> Simulation {
    let rules = RulesV1::default();
    let mut campaign = Campaign::new(DEFAULT_SEED, rules);
    for (index, world) in campaign.worlds.iter_mut().enumerate() {
        world.owner = match index {
            0 | 1 => Some(Faction::Helix),
            2 | 3 => Some(Faction::Choir),
            _ => None,
        };
    }
    campaign.fleets = vec![Fleet {
        id: FleetId(0),
        faction: Faction::Union,
        source: WorldId(0),
        destination: WorldId(4),
        strength: Strength(20_000),
        route_length: 1_000,
        progress: 0,
        speed_per_second: 0,
        hydrosphere_level: 0,
        movement_remainder: 0,
    }];
    campaign.next_fleet_id = FleetId(1);
    Simulation::from_campaign(campaign, rules)
}

fn run_scripted_union_commander(simulation: &mut Simulation, maximum_tick: u64) -> Option<Tick> {
    let mut sequence = 0;
    while simulation.state().next_tick.0 <= maximum_tick {
        if let Phase::Finished { at, .. } = simulation.state().phase {
            return Some(at);
        }

        let tick = simulation.state().next_tick.0;
        if tick.is_multiple_of(600) {
            let source = simulation
                .state()
                .worlds
                .iter()
                .filter(|world| world.owner == Some(Faction::Union) && world.energy.0 / 2 >= 10_000)
                .min_by_key(|world| (std::cmp::Reverse(world.energy.0 / 2), world.id));
            if let Some(source) = source {
                let target = simulation
                    .state()
                    .worlds
                    .iter()
                    .filter(|world| world.owner != Some(Faction::Union))
                    .min_by_key(|world| {
                        let owner_inbound = world.owner.map_or(0, |owner| {
                            simulation
                                .state()
                                .fleets
                                .iter()
                                .filter(|fleet| {
                                    fleet.destination == world.id && fleet.faction == owner
                                })
                                .map(|fleet| fleet.strength.0)
                                .fold(0_u64, u64::saturating_add)
                        });
                        (world.defense.0.saturating_add(owner_inbound), world.id)
                    });
                if let Some(target) = target {
                    simulation
                        .enqueue_next(
                            CommandSequence(sequence),
                            GameCommand::Launch {
                                source: source.id,
                                destination: target.id,
                            },
                        )
                        .unwrap();
                    sequence += 1;
                }
            }
        }
        simulation.step();
    }
    match simulation.state().phase {
        Phase::Finished { at, .. } if at.0 <= maximum_tick => Some(at),
        _ => None,
    }
}

#[test]
fn scripted_commander_observes_a_finish_on_the_inclusive_limit() {
    let rules = RulesV1::default();
    let mut campaign = Campaign::new(DEFAULT_SEED, rules);
    campaign.next_tick = Tick(54_000);
    for world in &mut campaign.worlds[..5] {
        world.owner = Some(Faction::Union);
    }
    let mut simulation = Simulation::from_campaign(campaign, rules);

    assert_eq!(
        run_scripted_union_commander(&mut simulation, 54_000),
        Some(Tick(54_000))
    );
}

#[test]
fn hazard_boundaries_are_half_open_and_repeatable() {
    let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    let mut repeat = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    simulation.step_n(2700);
    repeat.step_n(2700);
    assert_eq!(simulation.state().active_hazard, None);
    simulation.step();
    repeat.step();
    assert_eq!(
        simulation.state().active_hazard.unwrap().kind,
        HazardKind::IonStorm
    );
    assert_eq!(
        simulation.state().active_hazard,
        repeat.state().active_hazard
    );
    simulation.step_n(719);
    assert!(simulation.state().active_hazard.is_some());
    simulation.step();
    assert_eq!(simulation.state().active_hazard, None);
}

#[test]
fn fleet_storage_order_cannot_change_simultaneous_combat() {
    let left = simultaneous_arrival_fixture(false);
    let right = simultaneous_arrival_fixture(true);
    assert_eq!(
        step_fixture(left).canonical_fingerprint(),
        step_fixture(right).canonical_fingerprint()
    );
}

#[test]
fn identical_replay_and_chunked_steps_produce_identical_fingerprints() {
    let mut one = scripted_simulation();
    let mut two = scripted_simulation();
    for _ in 0..3600 {
        one.step();
    }
    for _ in 0..60 {
        two.step_n(60);
    }
    assert_eq!(one.canonical_fingerprint(), two.canonical_fingerprint());
}

#[test]
fn a_union_fleet_in_transit_prevents_elimination() {
    let mut simulation = union_fleet_only_fixture();
    simulation.step();
    assert_eq!(simulation.state().phase, Phase::Running);
}

#[test]
fn deterministic_balance_sentinel_finishes_between_ten_and_fifteen_minutes() {
    let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    let finished_tick = run_scripted_union_commander(&mut simulation, 54_000)
        .expect("scripted campaign must finish within fifteen simulated minutes");
    assert!(
        (36_000..=54_000).contains(&finished_tick.0),
        "campaign finished at {finished_tick:?} with {:?}",
        simulation.state().phase
    );
}

fn execute_once(
    campaign: Campaign,
    rules: RulesV1,
    command: GameCommand,
) -> Result<(), CommandRejection> {
    let mut simulation = Simulation::from_campaign(campaign, rules);
    simulation.enqueue_next(CommandSequence(0), command)?;
    match simulation.step().command_results.as_slice() {
        [CommandResult::Accepted { .. }] => Ok(()),
        [CommandResult::Rejected { reason, .. }] => Err(*reason),
        results => panic!("expected one command result, got {results:?}"),
    }
}

fn assert_preview_matches_execution(
    campaign: Campaign,
    rules: RulesV1,
    command: GameCommand,
) -> Result<CommandPreview, CommandRejection> {
    let simulation = Simulation::from_campaign(campaign.clone(), rules);
    let fingerprint = simulation.canonical_fingerprint();
    let preview = simulation.preview_command(command);
    assert_eq!(simulation.canonical_fingerprint(), fingerprint);
    assert_eq!(simulation.pending_command_count(), 0);
    assert_eq!(
        preview.as_ref().map(|_| ()).map_err(|reason| *reason),
        execute_once(campaign, rules, command)
    );
    preview
}

#[test]
fn command_preview_and_execution_share_every_launch_and_tuning_rejection() {
    let rules = RulesV1::default();
    let base = Campaign::new(DEFAULT_SEED, rules);
    let accepted = assert_preview_matches_execution(
        base.clone(),
        rules,
        GameCommand::Launch {
            source: WorldId(0),
            destination: WorldId(3),
        },
    )
    .unwrap();
    assert_eq!(accepted.source, WorldId(0));
    assert_eq!(accepted.destination, Some(WorldId(3)));
    assert_eq!(accepted.launch_strength, Some(Strength(40_000)));

    for (command, expected) in [
        (
            GameCommand::Launch {
                source: WorldId(9),
                destination: WorldId(3),
            },
            CommandRejection::InvalidWorld,
        ),
        (
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(9),
            },
            CommandRejection::InvalidWorld,
        ),
        (
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(0),
            },
            CommandRejection::SameSourceAndTarget,
        ),
        (
            GameCommand::Launch {
                source: WorldId(1),
                destination: WorldId(0),
            },
            CommandRejection::SourceNotOwnedByIssuer,
        ),
        (
            GameCommand::TuneField {
                world: WorldId(1),
                field: FieldKind::Atmosphere,
                adjustment: FieldAdjustment::Increase,
            },
            CommandRejection::SourceNotOwnedByIssuer,
        ),
        (
            GameCommand::TuneField {
                world: WorldId(9),
                field: FieldKind::Atmosphere,
                adjustment: FieldAdjustment::Increase,
            },
            CommandRejection::InvalidWorld,
        ),
    ] {
        assert_eq!(
            assert_preview_matches_execution(base.clone(), rules, command),
            Err(expected)
        );
    }

    for (field, adjustment) in [
        (FieldKind::Atmosphere, FieldAdjustment::Increase),
        (FieldKind::Hydrosphere, FieldAdjustment::Decrease),
        (FieldKind::Topology, FieldAdjustment::Increase),
    ] {
        assert!(
            assert_preview_matches_execution(
                base.clone(),
                rules,
                GameCommand::TuneField {
                    world: WorldId(0),
                    field,
                    adjustment,
                },
            )
            .is_ok()
        );
    }

    let mut insufficient = base.clone();
    insufficient.worlds[0].energy = Energy(1);
    assert_eq!(
        assert_preview_matches_execution(
            insufficient,
            rules,
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(3),
            },
        ),
        Err(CommandRejection::InsufficientEnergy)
    );

    let mut no_raise_energy = base.clone();
    no_raise_energy.worlds[0].energy = Energy(0);
    assert_eq!(
        assert_preview_matches_execution(
            no_raise_energy,
            rules,
            GameCommand::TuneField {
                world: WorldId(0),
                field: FieldKind::Atmosphere,
                adjustment: FieldAdjustment::Increase,
            },
        ),
        Err(CommandRejection::InsufficientEnergy)
    );

    let mut maximum_field = base.clone();
    maximum_field.worlds[0].fields.atmosphere = 10;
    assert_eq!(
        assert_preview_matches_execution(
            maximum_field,
            rules,
            GameCommand::TuneField {
                world: WorldId(0),
                field: FieldKind::Atmosphere,
                adjustment: FieldAdjustment::Increase,
            },
        ),
        Err(CommandRejection::FieldAtLimit)
    );

    let mut minimum_field = base;
    minimum_field.worlds[0].fields.topology = 0;
    assert_eq!(
        assert_preview_matches_execution(
            minimum_field,
            rules,
            GameCommand::TuneField {
                world: WorldId(0),
                field: FieldKind::Topology,
                adjustment: FieldAdjustment::Decrease,
            },
        ),
        Err(CommandRejection::FieldAtLimit)
    );
}

#[test]
fn command_preview_reports_finished_campaign_without_mutation() {
    let rules = RulesV1::default();
    let mut campaign = Campaign::new(DEFAULT_SEED, rules);
    campaign.phase = Phase::Finished {
        at: Tick(12),
        outcome: Outcome::PlayerEliminated,
    };
    assert_eq!(
        assert_preview_matches_execution(
            campaign,
            rules,
            GameCommand::Launch {
                source: WorldId(0),
                destination: WorldId(3),
            },
        ),
        Err(CommandRejection::CampaignFinished)
    );
}
