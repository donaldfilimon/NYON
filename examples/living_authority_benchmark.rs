//! Current resident Step path measurements, excluding renderer/platform timing.
use nyon::{living_content::LivingContent, sandbox_session::SandboxSession};
use nyon_workshop_core::living::*;
use std::time::Instant;

fn topology() -> Result<LivingGalaxyAuthorityV2, Box<dyn std::error::Error>> {
    let baseline = LivingContent::Seeded(0x4e59_4f4e_5045_5246).open()?;
    let mut state = LivingGalaxyStateV2::default();
    let id = |value: u64| {
        let mut bytes = [0x42; 16];
        bytes[8..].copy_from_slice(&value.to_be_bytes());
        LivingEntityIdV2(bytes)
    };
    state.systems = LivingSortedVecV2::new(
        (0..64)
            .map(|i| LivingSystemV2 {
                id: id(i + 1),
                name: LivingNameV2::new(format!("System {i}")).unwrap(),
            })
            .collect(),
    )?;
    let prototype = &baseline.state().worlds.as_slice()[0];
    state.worlds = LivingSortedVecV2::new(
        (0..512)
            .map(|i| {
                let mut row = prototype.clone();
                row.id = id(1000 + i);
                row.system = id(1 + i / 8);
                row.name = LivingNameV2::new(format!("World {i}")).unwrap();
                row.owner = None;
                row.inventory = LivingInventoryV2::default();
                row
            })
            .collect(),
    )?;
    state.civilizations = LivingSortedVecV2::new(
        (0..16)
            .map(|i| {
                let mut row = baseline.state().civilizations.as_slice()[0].clone();
                row.id = id(2000 + i);
                row.name = LivingNameV2::new(format!("Dormant {i}")).unwrap();
                row.status = LivingCivilizationStatusV2::Dormant;
                row
            })
            .collect(),
    )?;
    let catalog = living_core_pack_v2()?;
    state.validate(&catalog)?;
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )?;
    Ok(LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0x42; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("benchmark_topology")?,
            generator_version: 1,
        },
        genesis,
    )?)
}
fn capacity(staggered: bool) -> Result<LivingGalaxyAuthorityV2, Box<dyn std::error::Error>> {
    let base = topology()?;
    let mut state = base.state().clone();
    let id = |value: u64| {
        let mut bytes = [0x43; 16];
        bytes[8..].copy_from_slice(&value.to_be_bytes());
        LivingEntityIdV2(bytes)
    };
    let mut civilizations = state.civilizations.as_slice().to_vec();
    for row in &mut civilizations {
        row.status = LivingCivilizationStatusV2::Active;
    }
    let mut worlds = state.worlds.as_slice().to_vec();
    let mut colonies = Vec::new();
    let mut facilities = Vec::new();
    let mut routes = Vec::new();
    let mut shipments = Vec::new();
    let mut deposits = Vec::new();
    let mut fleets = Vec::new();
    for (i, world) in worlds.iter_mut().enumerate() {
        let owner = civilizations[i / 32].id;
        world.owner = Some(owner);
        world.inventory = LivingInventoryV2 {
            energy: 1000,
            ore: 1000,
            alloy: 1000,
        };
        let due = if staggered { 1 + (i % 10) as u64 } else { 1 };
        colonies.push(LivingColonyV2 {
            world: world.id,
            owner,
            hub: LivingHubV2 {
                energy_next_due: LivingTickV2(10),
                ore_next_due: LivingTickV2(100),
                fallback_next_due: LivingTickV2(100),
                blocked_reasons: LivingSortedVecV2::default(),
            },
        });
        deposits.push(LivingDepositV2 {
            id: id(10000 + i as u64),
            world: world.id,
            resource: LivingResourceV2::Ore,
            remaining_units: 20000,
        });
        for (j, definition) in ["solar_array", "extractor", "foundry", "shipyard"]
            .iter()
            .enumerate()
        {
            facilities.push(LivingFacilityV2 {
                id: id(20000 + (i * 4 + j) as u64),
                world: world.id,
                definition: LivingSlugV2::new(*definition)?,
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 0,
                hit_points: 0,
                next_due: match j {
                    0 | 1 => Some(LivingTickV2(10)),
                    2 => Some(LivingTickV2(20)),
                    _ => None,
                },
                next_repair: None,
                blocked_reasons: LivingSortedVecV2::default(),
                disabled: false,
            });
        }
        for j in 0..4 {
            let destination = base.state().worlds.as_slice()[i / 8 * 8 + (i % 8 + j + 1) % 8].id;
            routes.push(LivingFreightRouteV2 {
                id: id(30000 + (i * 4 + j) as u64),
                kind: LivingRouteKindV2::Internal,
                source_world: world.id,
                destination_world: destination,
                resource: LivingResourceV2::Ore,
                batch_size: 10,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: owner,
                receiver_owner: owner,
                next_due: LivingTickV2(due),
                suspended: false,
            });
        }
        for j in 0..8 {
            let destination = base.state().worlds.as_slice()[i / 8 * 8 + (i % 8 + 1) % 8].id;
            shipments.push(LivingShipmentV2 {
                id: id(40000 + (i * 8 + j) as u64),
                dispatch_owner: owner,
                intended_receiver: owner,
                source_world: world.id,
                destination_world: destination,
                resource: LivingResourceV2::Ore,
                units: 10,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(due),
                disposition: LivingShipmentDispositionV2::Outbound,
                dispatch_kind: Some(LivingRouteKindV2::Internal),
                returning_to_source: false,
            });
        }
        if i % 2 == 0 {
            let hulls = (0..8)
                .map(|j| LivingHullV2 {
                    id: id(60000 + (i / 2 * 8 + j) as u64),
                    definition: LivingSlugV2::new("escort").unwrap(),
                    hit_points: 10,
                    return_credit: false,
                    next_repair: None,
                })
                .collect();
            fleets.push(LivingFleetV2 {
                id: id(50000 + (i / 2) as u64),
                owner,
                location: LivingFleetLocationV2::Docked { world: world.id },
                order: None,
                hulls: LivingSortedVecV2::new(hulls)?,
            });
        }
    }
    state.worlds = LivingSortedVecV2::new(worlds)?;
    state.civilizations = LivingSortedVecV2::new(civilizations)?;
    state.colonies = LivingSortedVecV2::new(colonies)?;
    state.facilities = LivingSortedVecV2::new(facilities)?;
    state.routes = LivingSortedVecV2::new(routes)?;
    state.shipments = LivingSortedVecV2::new(shipments)?;
    state.deposits = LivingSortedVecV2::new(deposits)?;
    state.fleets = LivingSortedVecV2::new(fleets)?;
    let catalog = living_core_pack_v2()?;
    state.validate(&catalog)?;
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )?;
    Ok(LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0x43; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new(if staggered {
                "benchmark_capacity_staggered"
            } else {
                "benchmark_capacity_simultaneous"
            })?,
            generator_version: 1,
        },
        genesis,
    )?)
}
fn counts(state: &LivingGalaxyStateV2) -> serde_json::Value {
    serde_json::json!({"systems":state.systems.as_slice().len(),"worlds":state.worlds.as_slice().len(),
        "civilizations":state.civilizations.as_slice().len(),"facilities":state.facilities.as_slice().len(),
        "routes":state.routes.as_slice().len(),"shipments":state.shipments.as_slice().len(),
        "fleets":state.fleets.as_slice().len(),"hulls":state.fleets.as_slice().iter().map(|f|f.hulls.as_slice().len()).sum::<usize>()})
}
fn percentile(samples: &[u128], percent: usize) -> f64 {
    let rank = (samples.len() * percent).div_ceil(100).max(1) - 1;
    samples[rank] as f64 / 1000.0
}
enum Runner {
    Resident(Box<SandboxSession>),
    Authority(Box<LivingGalaxyAuthorityV2>),
}
impl Runner {
    fn authority(&self) -> &LivingGalaxyAuthorityV2 {
        match self {
            Self::Resident(s) => s.authority(),
            Self::Authority(a) => a,
        }
    }
    fn state(&self) -> &LivingGalaxyStateV2 {
        self.authority().state()
    }
    fn bookmark(&self) -> Result<LivingBookmarkV2, String> {
        self.authority().bookmark().map_err(|e| e.to_string())
    }
    fn step_once(&mut self) -> Result<(), String> {
        match self {
            Self::Resident(s) => s.step_once(),
            Self::Authority(a) => a.step().map(|_| ()).map_err(|e| e.to_string()),
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = String::from("vale");
    let mut steps = 3600_u64;
    let mut warmup = 10_u64;
    let mut authority_only = false;
    let mut host = String::from("unidentified host");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--authority-only" => authority_only = true,
            "--fixture" => fixture = args.next().ok_or("--fixture requires a value")?,
            "--steps" => steps = args.next().ok_or("--steps requires a value")?.parse()?,
            "--warmup" => warmup = args.next().ok_or("--warmup requires a value")?.parse()?,
            "--host" => host = args.next().ok_or("--host requires a value")?,
            "--help" => {
                println!(
                    "--fixture vale|expansion|seeded|topology|capacity|capacity-staggered --steps 3600 --warmup 10 --host NAME [--authority-only]\nTimes SandboxSession::step_once; topology is NOT the full declared capacity fixture. First ten measured steps print a runtime estimate before the remaining run."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument {arg}").into()),
        }
    }
    if steps == 0 || steps > 100_000 || warmup > 10_000 {
        return Err("steps must be 1..=100000 and warmup <=10000".into());
    }
    let authority = match fixture.as_str() {
        "vale" => LivingContent::ValeConfluence.open()?,
        "expansion" => LivingContent::FirstExpansion.open()?,
        "seeded" => LivingContent::Seeded(0x4e59_4f4e_5045_5246).open()?,
        "topology" => topology()?,
        "capacity" => capacity(false)?,
        "capacity-staggered" => capacity(true)?,
        _ => return Err("unknown fixture".into()),
    };
    authority.state().validate(authority.catalog())?;
    let catalog = authority.catalog().catalog_hash();
    let seed = authority.genesis_seed();
    let initial_counts = counts(authority.state());
    let mut session = if authority_only {
        Runner::Authority(Box::new(authority))
    } else {
        Runner::Resident(Box::new(SandboxSession::from_authority(authority)))
    };
    for _ in 0..warmup {
        session.step_once()?;
    }
    let initial = session.bookmark()?;
    let started = Instant::now();
    let mut samples = Vec::with_capacity(steps as usize);
    let mut events = 0_u64;
    let mut failure = None;
    for i in 0..steps {
        let boundary = Instant::now();
        if let Err(error) = session.step_once() {
            failure = Some(error);
            break;
        }
        samples.push(boundary.elapsed().as_nanos());
        events += session
            .authority()
            .last_receipt()
            .ok_or("missing receipt")?
            .events
            .len() as u64;
        if i == 9 {
            eprintln!(
                "pilot: 10 steps {:.3}s; estimated remaining {:.3}s (later simulation load can differ)",
                started.elapsed().as_secs_f64(),
                started.elapsed().as_secs_f64() / 10.0 * (steps - 10) as f64
            );
        }
    }
    let elapsed = started.elapsed();
    session.state().validate(session.authority().catalog())?;
    let completed = samples.len();
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({
            "host":host,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"fixture":fixture,
            "path":if authority_only { "LivingGalaxyAuthorityV2::step (authority only)" } else { "SandboxSession::step_once -> LivingGalaxyAuthorityV2::step (current resident Step path)" },
            "catalog":catalog,"seed":seed,"initial_counts":initial_counts,"final_counts":counts(session.state()),
            "warmup":warmup,"requested_steps":steps,"completed_samples":completed,"events":events,
            "initial_bookmark":initial,"final_bookmark":session.bookmark()?,"elapsed_ms":elapsed.as_secs_f64()*1000.0,
            "p50_us":(!samples.is_empty()).then(||percentile(&samples,50)),
            "p95_us":(!samples.is_empty()).then(||percentile(&samples,95)),
            "max_us":samples.last().map(|n|*n as f64/1000.0),"fault":failure,
            "qualification":if authority_only { "authority timing only; no renderer/input/wasm/endurance qualification" } else { "resident Step plus Chronicle bookkeeping only; no renderer/input/wasm/endurance qualification" }
        })
    );
    if let Some(error) = failure {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topology_fixture_is_valid_and_uses_real_resident_step() {
        let authority = topology().unwrap();
        assert_eq!(authority.state().systems.as_slice().len(), 64);
        assert_eq!(authority.state().worlds.as_slice().len(), 512);
        assert_eq!(authority.state().civilizations.as_slice().len(), 16);
        let mut session = SandboxSession::from_authority(authority);
        session.step_once().unwrap();
        assert_eq!(session.tick(), 1);
        session
            .state()
            .validate(session.authority().catalog())
            .unwrap();
        assert_eq!(
            session.authority().last_receipt().unwrap().state_digest,
            session.state().digest().unwrap()
        );
    }
    #[test]
    fn capacity_fixtures_validate_and_step_real_boundaries() {
        for staggered in [false, true] {
            let authority = capacity(staggered).unwrap();
            let c = counts(authority.state());
            for (key, expected) in [
                ("systems", 64),
                ("worlds", 512),
                ("civilizations", 16),
                ("facilities", 2048),
                ("routes", 2048),
                ("shipments", 4096),
                ("fleets", 256),
                ("hulls", 2048),
            ] {
                assert_eq!(c[key], expected);
            }
            let mut direct = authority.clone();
            let expected_receipt = direct.step().unwrap();
            let mut session = SandboxSession::from_authority(authority);
            session.step_once().unwrap();
            assert_eq!(session.authority().last_receipt(), Some(&expected_receipt));
            session
                .state()
                .validate(session.authority().catalog())
                .unwrap();
            assert_eq!(
                session.authority().last_receipt().unwrap().state_digest,
                session.state().digest().unwrap()
            );
        }
    }
    #[test]
    fn nearest_rank_percentiles_handle_small_samples() {
        assert_eq!(percentile(&[1000], 95), 1.0);
        assert_eq!(percentile(&[1000, 2000, 3000, 4000], 50), 2.0);
        assert_eq!(percentile(&[1000, 2000, 3000, 4000], 95), 4.0);
    }
}
