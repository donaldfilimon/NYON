//! Current Rust Classic authority oracle for the Zig migration. No fixture
//! generation or production source mutation; parity reads stdout from both.
use std::io::{self, Write};

use nyon::game::{
    model::{Faction, FieldAdjustment, FieldKind, GameCommand, RulesV1, WorldId},
    simulation::{CommandResult, GameEvent, Simulation, TickReport},
};

fn owner_tag(owner: Option<Faction>) -> u8 {
    match owner {
        None => 0,
        Some(Faction::Union) => 1,
        Some(Faction::Helix) => 2,
        Some(Faction::Choir) => 3,
    }
}

fn trace(out: &mut impl Write, sim: &Simulation, report: &TickReport) -> io::Result<()> {
    write!(
        out,
        "{}\t{:016X}\t",
        sim.state().next_tick.0,
        sim.canonical_fingerprint()
    )?;
    if report.command_results.is_empty() {
        write!(out, "-")?;
    }
    for result in &report.command_results {
        match result {
            CommandResult::Accepted { sequence } => write!(out, "A{},", sequence.0)?,
            CommandResult::Rejected { sequence, reason } => {
                write!(out, "R{}:{reason:?},", sequence.0)?
            }
        }
    }
    write!(out, "\t")?;
    if report.events.is_empty() {
        write!(out, "-")?;
    }
    for event in &report.events {
        match event {
            GameEvent::FleetLaunched(id) => write!(out, "L{},", id.0)?,
            GameEvent::WorldCaptured { world, owner } => {
                write!(out, "C{}:{},", world.0, owner_tag(*owner))?
            }
            GameEvent::HazardStarted(hazard) => {
                use nyon::game::model::HazardKind;
                let kind = match hazard.kind {
                    HazardKind::IonStorm => 0,
                    HazardKind::GravityTide => 1,
                };
                let field = match hazard.affected_field {
                    FieldKind::Atmosphere => 0,
                    FieldKind::Hydrosphere => 1,
                    FieldKind::Topology => 2,
                };
                write!(
                    out,
                    "H{}:{kind}:{field}:{}:{},",
                    hazard.event_index, hazard.start.0, hazard.end.0
                )?;
            }
            GameEvent::HazardEnded(kind) => {
                use nyon::game::model::HazardKind;
                write!(
                    out,
                    "E{},",
                    match kind {
                        HazardKind::IonStorm => 0,
                        HazardKind::GravityTide => 1,
                    }
                )?;
            }
            GameEvent::CampaignFinished(outcome) => {
                use nyon::game::model::Outcome;
                match outcome {
                    Outcome::FactionVictory { winner } => {
                        write!(out, "V{},", owner_tag(Some(*winner)))?
                    }
                    Outcome::PlayerEliminated => write!(out, "P,")?,
                }
            }
        }
    }
    writeln!(out)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 3 && args[1] == "scenario-default" {
        let seed = u64::from_str_radix(&args[2], 16)?;
        let scenario =
            nyon::scenario::ScenarioDraft::generated(seed, RulesV1::default()).validated()?;
        println!(
            "OK\t{:016X}\t{}",
            scenario.fingerprint().0,
            nyon::scenario::codec::encode(&scenario)?
        );
        return Ok(());
    }
    if args.len() == 3 && args[1] == "scenario-json" {
        use nyon::scenario::codec::CodecError;
        match nyon::scenario::codec::decode(&args[2]) {
            Ok(scenario) => println!(
                "OK\t{:016X}\t{}",
                scenario.fingerprint().0,
                nyon::scenario::codec::encode(&scenario)?
            ),
            Err(error) => println!(
                "ERR\t{}",
                match error {
                    CodecError::TooLarge => "TooLarge",
                    CodecError::Json(_) => "Json",
                    CodecError::Version => "Version",
                    CodecError::CanonicalNumber { .. } => "CanonicalNumber",
                    CodecError::Validation(_) => "Validation",
                }
            ),
        }
        return Ok(());
    }
    if args.len() == 2 && args[1] == "preferences-default" {
        println!(
            "OK\t{}",
            nyon::preferences::encode(&nyon::preferences::UserPreferencesV1::default())?
        );
        return Ok(());
    }
    if args.len() == 3 && args[1] == "preferences-json" {
        use nyon::preferences::PreferencesCodecError;
        match nyon::preferences::decode(&args[2]) {
            Ok(preferences) => println!("OK\t{}", nyon::preferences::encode(&preferences)?),
            Err(error) => println!(
                "ERR\t{}",
                match error {
                    PreferencesCodecError::TooLarge => "TooLarge",
                    PreferencesCodecError::Json(_) => "Json",
                    PreferencesCodecError::Version => "Version",
                    PreferencesCodecError::UiScale(_) => "UiScale",
                }
            ),
        }
        return Ok(());
    }
    if !(3..=4).contains(&args.len()) || (args.len() == 4 && args[3] != "scripted") {
        return Err("usage: zig_classic_oracle SEED_HEX TICKS [scripted]".into());
    }
    let seed = u64::from_str_radix(&args[1], 16)?;
    let ticks: u32 = args[2].parse()?;
    let mut sim = Simulation::generated(seed, RulesV1::default());
    let mut out = io::BufWriter::new(io::stdout().lock());
    writeln!(out, "0\t{:016X}\t-\t-", sim.canonical_fingerprint())?;
    for iteration in 0..ticks {
        if args.len() == 4 && matches!(sim.state().phase, nyon::game::model::Phase::Running) {
            use nyon::game::model::CommandSequence;
            if iteration % 173 == 0 {
                sim.enqueue_next(
                    CommandSequence(2),
                    GameCommand::Launch {
                        source: WorldId(0),
                        destination: WorldId((iteration / 173 % 6 + 1) as u8),
                    },
                )
                .unwrap();
                sim.enqueue_next(
                    CommandSequence(1),
                    GameCommand::TuneField {
                        world: WorldId(0),
                        field: FieldKind::Atmosphere,
                        adjustment: FieldAdjustment::Increase,
                    },
                )
                .unwrap();
                sim.enqueue_next(
                    CommandSequence(0),
                    GameCommand::Launch {
                        source: WorldId(0),
                        destination: WorldId(0),
                    },
                )
                .unwrap();
            }
            if iteration % 257 == 0 {
                sim.enqueue_next(
                    CommandSequence(3),
                    GameCommand::TuneField {
                        world: WorldId(0),
                        field: FieldKind::Topology,
                        adjustment: FieldAdjustment::Decrease,
                    },
                )
                .unwrap();
            }
        }
        let report = sim.step();
        trace(&mut out, &sim, &report)?;
    }
    out.flush()?;
    Ok(())
}
