use std::{
    collections::BTreeSet,
    env,
    error::Error,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, EntityId, GalaxyPointV1, ObjectName,
    ObjectRefV1, StateDigest, WorkshopHistory, WorkshopTick, decode_catalog_pack,
    model::{
        MAX_DEPOSITS, MAX_FACTIONS, MAX_HAZARDS, MAX_INDUSTRIES, MAX_LANES, MAX_ROUTES,
        MAX_SHIPMENTS, MAX_STARS, MAX_SYSTEMS, MAX_WORLDS, WorkshopStateV1,
    },
};
use serde::Serialize;

const CORE_PACK: &[u8] = include_bytes!("../../../assets/workshop/core-pack-v1.json");
const DEFAULT_SEED: u64 = 0x4e59_4f4e_574f_524b;
const DEFAULT_WARMUP_STEPS: usize = 100;
const DEFAULT_MEASURED_STEPS: usize = 10_000;
const MINIMUM_WARMUP_STEPS: usize = 100;
const MINIMUM_MEASURED_STEPS: usize = 10_000;
const P95_BUDGET_NS: u64 = 5_000_000;
const USED_ROUTE_LANES: usize = MAX_SYSTEMS / 2;

// Frozen after constructing the declared-capacity fixture and advancing the two
// deterministic priming ticks needed to reach 4,096 in-flight shipments.
const EXPECTED_FIXTURE_DIGEST_HEX: &str =
    "6fba38826647d9fb291e3cfaba7aea5d380f987b0d45dd3cb4aac15491db89bd";

type BenchResult<T> = Result<T, Box<dyn Error>>;

#[derive(Clone, Debug, Serialize)]
struct CapacityCoverage {
    kind: &'static str,
    actual: usize,
    limit: usize,
    reached: bool,
}

impl CapacityCoverage {
    const fn new(kind: &'static str, actual: usize, limit: usize) -> Self {
        Self {
            kind,
            actual,
            limit,
            reached: actual == limit,
        }
    }
}

#[derive(Debug, Serialize)]
struct HostMetadata {
    label: String,
    os_version: String,
    cpu: String,
    power_mode: String,
}

#[derive(Debug, Serialize)]
struct BuildMetadata {
    git_commit: String,
    git_dirty: String,
    artifact_sha256: String,
    rustc: String,
    target: String,
    profile: &'static str,
    crate_version: &'static str,
}

#[derive(Debug, Serialize)]
struct TimingSummary {
    unit: &'static str,
    sample_count: usize,
    minimum: u64,
    p50: u64,
    p95: u64,
    p99: u64,
    maximum: u64,
    mean: u64,
    budget_p95: u64,
    passed: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    kind: &'static str,
    format_version: u32,
    generated_utc: String,
    measured_operation: &'static str,
    host: HostMetadata,
    build: BuildMetadata,
    catalog_hash: String,
    genesis_seed: u64,
    qualification_eligible: bool,
    fixture_digest: String,
    pre_measurement_digest: String,
    final_digest: String,
    first_measured_tick: u64,
    final_tick: u64,
    warmup_steps: usize,
    capacity_coverage: Vec<CapacityCoverage>,
    unreached_capacities: Vec<&'static str>,
    timing: TimingSummary,
    samples_ns: Vec<u64>,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!(
                "Workshop authority benchmark failed before producing valid evidence: {error}"
            );
            ExitCode::from(1)
        }
    }
}

fn run() -> BenchResult<bool> {
    let config = Config::from_environment()?;
    let catalog = decode_catalog_pack(CORE_PACK)?;
    let catalog_hash = hex(&catalog.catalog_hash().0);
    let mut history = WorkshopHistory::from_seed_u64(catalog, config.seed);

    populate_declared_capacity_fixture(&mut history)?;
    for _ in 0..2 {
        black_box(history.step()?);
    }

    let fixture_digest = digest_hex(history.active_state_digest());
    if !EXPECTED_FIXTURE_DIGEST_HEX.is_empty()
        && fixture_digest.as_str() != EXPECTED_FIXTURE_DIGEST_HEX
    {
        return Err(format!(
            "declared-capacity fixture digest changed: expected {EXPECTED_FIXTURE_DIGEST_HEX}, got {fixture_digest}"
        )
        .into());
    }
    require_declared_capacities(history.state())?;

    for _ in 0..config.warmup_steps {
        black_box(history.step()?);
    }
    require_declared_capacities(history.state())?;
    let coverage = capacity_coverage(history.state());
    let unreached_capacities = coverage
        .iter()
        .filter_map(|item| (!item.reached).then_some(item.kind))
        .collect::<Vec<_>>();
    let pre_measurement_digest = digest_hex(history.active_state_digest());
    let first_measured_tick = history.state().tick.0;

    let mut samples_ns = Vec::with_capacity(config.measured_steps);
    for _ in 0..config.measured_steps {
        let started = Instant::now();
        let receipt = history.step()?;
        let elapsed = started.elapsed();
        black_box(receipt);
        samples_ns.push(u64::try_from(elapsed.as_nanos())?);
    }

    require_declared_capacities(history.state())?;
    let timing = summarize(&samples_ns)?;
    let passed = timing.passed;
    let report = BenchmarkReport {
        kind: "NYON_WORKSHOP_AUTHORITY_BENCHMARK",
        format_version: 1,
        generated_utc: env_value("NYON_WORKSHOP_BENCH_GENERATED_UTC"),
        measured_operation: "WorkshopHistory::step",
        host: HostMetadata {
            label: env_value("NYON_WORKSHOP_BENCH_HOST"),
            os_version: env_value("NYON_WORKSHOP_BENCH_OS_VERSION"),
            cpu: env_value("NYON_WORKSHOP_BENCH_CPU"),
            power_mode: env_value("NYON_WORKSHOP_BENCH_POWER_MODE"),
        },
        build: BuildMetadata {
            git_commit: env_value("NYON_WORKSHOP_BENCH_GIT_COMMIT"),
            git_dirty: env_value("NYON_WORKSHOP_BENCH_GIT_DIRTY"),
            artifact_sha256: env_value("NYON_WORKSHOP_BENCH_ARTIFACT_SHA256"),
            rustc: env_value("NYON_WORKSHOP_BENCH_RUSTC"),
            target: env_value("NYON_WORKSHOP_BENCH_TARGET"),
            profile: "release",
            crate_version: env!("CARGO_PKG_VERSION"),
        },
        catalog_hash,
        genesis_seed: config.seed,
        qualification_eligible: config.warmup_steps >= MINIMUM_WARMUP_STEPS
            && config.measured_steps >= MINIMUM_MEASURED_STEPS,
        fixture_digest,
        pre_measurement_digest,
        final_digest: digest_hex(history.active_state_digest()),
        first_measured_tick,
        final_tick: history.state().tick.0,
        warmup_steps: config.warmup_steps,
        capacity_coverage: coverage,
        unreached_capacities,
        timing,
        samples_ns,
    };

    write_report(&config.output, &report)?;
    println!(
        "Workshop authority benchmark report: {}",
        config.output.display()
    );
    for item in &report.capacity_coverage {
        println!(
            "capacity {:>10}: {:>4}/{:<4} {}",
            item.kind,
            item.actual,
            item.limit,
            if item.reached {
                "REACHED"
            } else {
                "NOT REACHED"
            }
        );
    }
    println!(
        "step timing: samples={} p50={:.3} ms p95={:.3} ms p99={:.3} ms budget={:.3} ms {}",
        report.timing.sample_count,
        nanos_to_millis(report.timing.p50),
        nanos_to_millis(report.timing.p95),
        nanos_to_millis(report.timing.p99),
        nanos_to_millis(report.timing.budget_p95),
        if passed { "PASS" } else { "FAIL" },
    );
    if !passed {
        eprintln!(
            "Workshop authority benchmark exceeded the fixed 5 ms p95 budget; see the raw report above."
        );
    }
    Ok(passed)
}

#[derive(Debug)]
struct Config {
    seed: u64,
    warmup_steps: usize,
    measured_steps: usize,
    output: PathBuf,
}

impl Config {
    fn from_environment() -> BenchResult<Self> {
        let allow_short = env::var("NYON_WORKSHOP_BENCH_ALLOW_SHORT").as_deref() == Ok("1");
        let warmup_steps =
            parse_usize_env("NYON_WORKSHOP_BENCH_WARMUP_STEPS", DEFAULT_WARMUP_STEPS)?;
        let measured_steps =
            parse_usize_env("NYON_WORKSHOP_BENCH_MEASURED_STEPS", DEFAULT_MEASURED_STEPS)?;
        if !allow_short && warmup_steps < MINIMUM_WARMUP_STEPS {
            return Err(format!(
                "full evidence requires at least {MINIMUM_WARMUP_STEPS} warmup steps"
            )
            .into());
        }
        if !allow_short && measured_steps < MINIMUM_MEASURED_STEPS {
            return Err(format!(
                "full evidence requires at least {MINIMUM_MEASURED_STEPS} measured steps"
            )
            .into());
        }
        if measured_steps == 0 {
            return Err("measured step count must not be zero".into());
        }
        let seed = env::var("NYON_WORKSHOP_BENCH_SEED")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()?
            .unwrap_or(DEFAULT_SEED);
        let output = env::var_os("NYON_WORKSHOP_BENCH_OUTPUT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/qualification/workshop-authority.json"));
        Ok(Self {
            seed,
            warmup_steps,
            measured_steps,
            output,
        })
    }
}

fn populate_declared_capacity_fixture(history: &mut WorkshopHistory) -> BenchResult<()> {
    let yellow_dwarf = CatalogId::new("yellow-dwarf")?;
    let rocky_world = CatalogId::new("rocky-world")?;
    let energy = CatalogId::new("energy")?;
    let ore = CatalogId::new("ore")?;
    let solar_array = CatalogId::new("solar-array")?;
    let ion_storm = CatalogId::new("ion-storm")?;

    create_entities(history, MAX_FACTIONS, |index, local| {
        Ok(CreatorOpV1::CreateFaction {
            local,
            name: ObjectName::new(format!("Faction {index:02}"))?,
            color_rgb: [
                u8::try_from((index * 17) % 256)?,
                u8::try_from((index * 47) % 256)?,
                u8::try_from((index * 83) % 256)?,
            ],
        })
    })?;

    let systems = create_entities(history, MAX_SYSTEMS, |index, local| {
        let pair = index / 2;
        let x = i64::try_from(pair * 4_096 + (index % 2) * 300)?;
        Ok(CreatorOpV1::CreateSystem {
            local,
            name: ObjectName::new(format!("System {index:02}"))?,
            position: GalaxyPointV1::new(x, 0)?,
        })
    })?;

    let stars = create_entities(history, MAX_STARS, |index, local| {
        Ok(CreatorOpV1::CreateStar {
            local,
            system: ObjectRefV1::Existing(systems[index / 2]),
            name: ObjectName::new(format!("Star {index:03}"))?,
            archetype_id: yellow_dwarf.clone(),
        })
    })?;

    let worlds = create_entities(history, MAX_WORLDS, |index, local| {
        let system_index = index / 8;
        Ok(CreatorOpV1::CreateWorld {
            local,
            system: ObjectRefV1::Existing(systems[system_index]),
            primary: ObjectRefV1::Existing(stars[system_index * 2]),
            name: ObjectName::new(format!("World {index:03}"))?,
            archetype_id: rocky_world.clone(),
            orbit_radius_milli_au: u32::try_from(1_000 + index)?,
            orbit_period_ticks: 10_000 + u64::try_from(index)?,
            phase_millidegrees: u32::try_from((index * 701) % 360_000)?,
        })
    })?;

    let lane_pairs = lane_pairs();
    let lanes = create_entities(history, MAX_LANES, |index, local| {
        let (a, b) = lane_pairs[index];
        Ok(CreatorOpV1::ConnectLane {
            local,
            a: ObjectRefV1::Existing(systems[a]),
            b: ObjectRefV1::Existing(systems[b]),
        })
    })?;

    create_entities(history, MAX_DEPOSITS, |index, local| {
        Ok(CreatorOpV1::CreateDeposit {
            local,
            world: ObjectRefV1::Existing(worlds[index / 2]),
            resource_id: ore.clone(),
            reserve_units: 1_000_000_000,
        })
    })?;

    create_entities(history, MAX_INDUSTRIES, |index, local| {
        Ok(CreatorOpV1::PlaceIndustry {
            local,
            world: ObjectRefV1::Existing(worlds[index / 4]),
            definition_id: solar_array.clone(),
            linked_deposit: None,
        })
    })?;

    create_entities(history, MAX_ROUTES, |index, local| {
        let source_world_index = index / 4;
        let route_slot = index % 4;
        let source_system = source_world_index / 8;
        let source_world_in_system = source_world_index % 8;
        let destination_system = source_system ^ 1;
        let destination_world_in_system = (source_world_in_system + route_slot) % 8;
        let destination_world_index = destination_system * 8 + destination_world_in_system;
        Ok(CreatorOpV1::ConnectRoute {
            local,
            source: ObjectRefV1::Existing(worlds[source_world_index]),
            destination: ObjectRefV1::Existing(worlds[destination_world_index]),
            resource_id: energy.clone(),
            batch_units: 4,
        })
    })?;

    create_entities(history, MAX_HAZARDS, |index, local| {
        // Keep hazards active and fully iterated without modifying the 32 lanes
        // that feed the stable two-tick shipment pipeline.
        let lane_index = USED_ROUTE_LANES + index;
        Ok(CreatorOpV1::ScheduleHazard {
            local,
            lane: ObjectRefV1::Existing(lanes[lane_index]),
            hazard_id: ion_storm.clone(),
            start_tick: WorkshopTick(0),
            duration_ticks: 36_000,
        })
    })?;
    Ok(())
}

fn create_entities<F>(
    history: &mut WorkshopHistory,
    count: usize,
    mut operation: F,
) -> BenchResult<Vec<EntityId>>
where
    F: FnMut(usize, BatchLocalId) -> BenchResult<CreatorOpV1>,
{
    let mut entities = Vec::with_capacity(count);
    let mut start = 0;
    while start < count {
        let chunk_len = (count - start).min(nyon_workshop_core::command::MAX_BATCH_OPERATIONS);
        let operations = (0..chunk_len)
            .map(|offset| {
                let local = BatchLocalId(u16::try_from(offset)?);
                operation(start + offset, local)
            })
            .collect::<BenchResult<Vec<_>>>()?;
        let receipt = history.submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations,
        })?;
        for offset in 0..chunk_len {
            let local = BatchLocalId(u16::try_from(offset)?);
            entities.push(receipt.entities[&local]);
        }
        start += chunk_len;
    }
    Ok(entities)
}

fn lane_pairs() -> Vec<(usize, usize)> {
    let mut pairs = (0..MAX_SYSTEMS)
        .step_by(2)
        .map(|left| (left, left + 1))
        .collect::<Vec<_>>();
    let mut unique = pairs.iter().copied().collect::<BTreeSet<_>>();
    'outer: for a in 0..MAX_SYSTEMS {
        for b in (a + 1)..MAX_SYSTEMS {
            if unique.insert((a, b)) {
                pairs.push((a, b));
                if pairs.len() == MAX_LANES {
                    break 'outer;
                }
            }
        }
    }
    assert_eq!(pairs.len(), MAX_LANES);
    pairs
}

fn require_declared_capacities(state: &WorkshopStateV1) -> BenchResult<()> {
    let missing = capacity_coverage(state)
        .into_iter()
        .filter(|item| !item.reached)
        .map(|item| format!("{}={}/{}", item.kind, item.actual, item.limit))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "declared-capacity fixture is incomplete: {}",
            missing.join(", ")
        )
        .into())
    }
}

fn capacity_coverage(state: &WorkshopStateV1) -> Vec<CapacityCoverage> {
    vec![
        CapacityCoverage::new("factions", state.factions.len(), MAX_FACTIONS),
        CapacityCoverage::new("systems", state.systems.len(), MAX_SYSTEMS),
        CapacityCoverage::new("stars", state.stars.len(), MAX_STARS),
        CapacityCoverage::new("worlds", state.worlds.len(), MAX_WORLDS),
        CapacityCoverage::new("lanes", state.lanes.len(), MAX_LANES),
        CapacityCoverage::new("deposits", state.deposits.len(), MAX_DEPOSITS),
        CapacityCoverage::new("industries", state.industries.len(), MAX_INDUSTRIES),
        CapacityCoverage::new("routes", state.routes.len(), MAX_ROUTES),
        CapacityCoverage::new("shipments", state.shipments.len(), MAX_SHIPMENTS),
        CapacityCoverage::new("hazards", state.hazards.len(), MAX_HAZARDS),
    ]
}

fn summarize(samples: &[u64]) -> BenchResult<TimingSummary> {
    if samples.is_empty() {
        return Err("cannot summarize zero timing samples".into());
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let total = sorted
        .iter()
        .try_fold(0_u128, |sum, sample| sum.checked_add(u128::from(*sample)))
        .ok_or("timing total overflow")?;
    let mean = u64::try_from(total / u128::try_from(sorted.len())?)?;
    let p95 = percentile(&sorted, 95);
    Ok(TimingSummary {
        unit: "nanoseconds",
        sample_count: sorted.len(),
        minimum: sorted[0],
        p50: percentile(&sorted, 50),
        p95,
        p99: percentile(&sorted, 99),
        maximum: sorted[sorted.len() - 1],
        mean,
        budget_p95: P95_BUDGET_NS,
        passed: p95 <= P95_BUDGET_NS,
    })
}

fn percentile(sorted: &[u64], percentage: usize) -> u64 {
    let rank = sorted.len().saturating_mul(percentage).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}

fn write_report(path: &Path, report: &BenchmarkReport) -> BenchResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(report)?;
    bytes.push(b'\n');
    fs::write(path, bytes)?;
    Ok(())
}

fn parse_usize_env(name: &str, default: usize) -> BenchResult<usize> {
    env::var(name)
        .ok()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map(|value| value.unwrap_or(default))
        .map_err(Into::into)
}

fn env_value(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| "unknown".to_owned())
}

fn digest_hex(digest: StateDigest) -> String {
    hex(&digest.0)
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn nanos_to_millis(nanos: u64) -> f64 {
    nanos as f64 / 1_000_000.0
}
