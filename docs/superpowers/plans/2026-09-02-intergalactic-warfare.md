# Intergalactic Warfare WebGPU Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the complete deterministic seven-world strategy game on macOS, Windows, Linux, and browser WebGPU, with a detached scenario editor and a transparent CPU/WebGPU neural advisory that never controls gameplay.

**Architecture:** Pure campaign truth and fixed-step simulation remain under `src/game` with no window, GPU, browser, storage, editor, or advisory dependency. `src/scenario` validates detached drafts and owns explicit scenario identity; `src/editor` presents those drafts; top-level `src/advisory` computes read-only scores; `src/engine` owns input/time/shaders/GPU/primitives; and shared `src/app.rs` joins native or wasm winit lifecycle to those boundaries. Native creates the instance and surface on the winit thread, awaits adapter/device acquisition on Tokio, and returns the result through a generation-tagged marker; wasm uses `spawn_local`, a same-thread result mailbox, and the same marker-only `AppEvent`.

**Tech Stack:** Rust nightly-2026-09-01, edition 2024; winit 0.30.12; wgpu 30.0.1; Naga 30.0.1; glam 0.33.6; bytemuck 1.23; serde/serde_json; native Tokio 1.53.1/env_logger/tempfile; development-only pollster 0.4.0; exact wasm-bindgen 0.2.127, wasm-bindgen-futures 0.4.77, web-sys 0.3.104, console_error_panic_hook 0.1.7, and console_log 1.1.0; wasm-only web-time 1.1.0.

**Spec:** `docs/superpowers/specs/2026-09-02-intergalactic-warfare-design.md`

## Global Constraints

- Pin `nightly-2026-09-01` in `rust-toolchain.toml`; do not silently use the moving `nightly` channel.
- Use Rust edition 2024 and `#![forbid(unsafe_code)]` in both library and binary roots.
- Pin `wgpu = 30.0.1`, `naga = 30.0.1`, and stable `winit = 0.30.12` exactly.
- Do not add an ECS, game engine, UI framework, font renderer, physics engine, network service, telemetry system, or downloaded runtime asset.
- Do not add an ML or training framework. Neural Advisory V1 is the fixed, transparent 12 to 4 to 1 model specified in the design.
- Keep all worlds, factions, hazards, and strategic data fictional and deterministic.
- Game state and simulation tests must run without creating a window, surface, adapter, or device.
- Treat gameplay numbers as abstractions; do not label them as real orbital, nuclear, climatic, demographic, epidemiological, or geophysical calculations.
- Preserve the default seed `0x4947_5731_2026_0902`, RulesV1 base fleet speed 23, all generator draws, and default state digest `0x67D9_6E98_3D6C_9330` exactly.
- Campaign/`Simulation` is the sole gameplay authority. Scenario drafts, editor widgets, advisory scores, GPU values, browser objects, and presentation state never enter commands, AI, outcomes, or the canonical state digest.
- Shared native sources target macOS, Windows, and Linux. The browser target is `wasm32-unknown-unknown`, `rlib + cdylib`, and explicit `Backends::BROWSER_WEBGPU` with no WebGL fallback.
- Native uses a multi-thread Tokio runtime: instance/surface creation and final configuration stay on the winit thread while adapter/device acquisition runs on a worker. Wasm uses `wasm_bindgen_futures::spawn_local` and must not block or assert that `GpuContext` is `Send`.
- JSON payloads are at most 65,536 bytes. The seed is exactly 16 uppercase hex digits; every JSON `u64` field is a canonical decimal string to prevent browser precision loss.
- Scenario editing is detached. Apply validates and constructs a new tick-0 simulation in temporaries before one swap; Cancel and every failure leave the active simulation unchanged.
- Neural output is advisory only and has no API capable of emitting or enqueueing `GameCommand`.
- Current live native/manual evidence is macOS Metal only. CI definitions, cross-compilation, bundles, live browser runtime, GPU parity, and manual acceptance are reported as distinct layers.
- Do not leave unfinished-marker comments, placeholder panels, stub commands, or dead game branches in the completed tree.

Tasks 1 through 5 below are already implemented and reviewed at commit `46a3884`; their text is retained as the frozen foundation. Tasks 6 through 12 are the expanded executable sequence.

## Target File Map

```text
IntergalacticWarfareWebGPU/
|-- .github/workflows/ci.yml          native matrix and wasm bundle definition
|-- .gitignore                         target, dist, tool, and editor exclusions
|-- Cargo.toml                         pinned dependency graph and profiles
|-- Cargo.lock                         resolved reproducible graph
|-- README.md                          native/browser play, editor, advisory, gates
|-- rust-toolchain.toml                dated Rust 2026 toolchain
|-- tools/build-web.sh                 pinned wasm target/CLI build and bundle gate
|-- assets/shaders/primitives.wgsl     single WebGPU-compatible draw shader
|-- assets/shaders/advisory.wgsl       fixed 12 to 4 to 1 compute shader
|-- web/index.html                     browser module loader and canvas host
|-- dist/                              generated ignored JS/WASM output
|-- docs/superpowers/specs/...         approved product design
|-- docs/superpowers/plans/...         this executable plan
|-- src/lib.rs                         testable crate boundary
|-- src/main.rs                        native-only entry point
|-- src/app.rs                         shared ApplicationHandler and product modes
|-- src/platform/mod.rs                cfg-selected platform exports
|-- src/platform/native.rs             Tokio-hosted startup and application-support store
|-- src/platform/web.rs                spawn_app, async mailbox, localStorage
|-- src/engine/mod.rs                  engine exports and EngineError
|-- src/engine/gpu.rs                  async instance/surface/device/queue lifecycle
|-- src/engine/input.rs                pointer/touch, actions, navigation, text
|-- src/engine/primitives.rs           vertex batching and 5x7 glyph emission
|-- src/engine/render.rs               GPU pipeline, dynamic vertex upload, pass
|-- src/engine/shader.rs               Naga parse and validation boundary
|-- src/engine/time.rs                 clamped delta and fixed-step accumulator
|-- src/game/mod.rs                    game exports
|-- src/game/model.rs                  deterministic campaign data model
|-- src/game/simulation.rs             commands, economy, fleets, AI, hazards
|-- src/game/view.rs                   layout, hit testing, presentation builder
|-- src/scenario/mod.rs                drafts, owned names, validation, fingerprint
|-- src/scenario/codec.rs              strict 64 KiB versioned JSON codec
|-- src/scenario/store.rs              one-slot store trait and memory test double
|-- src/editor.rs                      widgets, focus, layout, draft state machine
|-- src/advisory/mod.rs                features, CPU model, controller, parity
|-- src/advisory/gpu.rs                optional wgpu compute/readback backend
|-- tests/campaign.rs                  end-to-end deterministic campaign tests
|-- tests/shaders.rs                   both shipped WGSL validation tests
|-- tests/scenario.rs                  validation, identity, codec, atomicity
|-- tests/advisory.rs                  CPU/controller/parity unit tests
`-- tests/advisory_gpu.rs              adapter-backed seven-score parity
```

### Task 1: Reproducible Rust 2026 Crate Foundation

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`
- Create: `src/lib.rs`

**Interfaces:**
- Consumes: the global dependency and toolchain constraints.
- Produces: a compile-clean `intergalactic_warfare` library root that later tasks extend by adding tested modules.

- [ ] **Step 1: Add the pinned manifest and toolchain**

```toml
# Cargo.toml
[package]
name = "intergalactic-warfare"
version = "0.1.0"
edition = "2024"
rust-version = "1.100"
description = "A deterministic fictional sector-control game on a custom WebGPU engine"
license = "MIT OR Apache-2.0"

[lib]
name = "intergalactic_warfare"
path = "src/lib.rs"

[dependencies]
bytemuck = { version = "1.23.2", features = ["derive"] }
env_logger = "0.11.8"
glam = "0.33.6"
log = "0.4.28"
naga = { version = "=30.0.1", features = ["wgsl-in"] }
pollster = "0.4.0"
thiserror = "2.0.16"
wgpu = "=30.0.1"
winit = "=0.30.12"

[profile.dev]
opt-level = 1

[profile.dev.package."*"]
opt-level = 2

[profile.release]
lto = "thin"
codegen-units = 1
strip = "debuginfo"
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "nightly-2026-09-01"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

```gitignore
/target/
/.DS_Store
*.swp
```

- [ ] **Step 2: Add compile-visible module roots with unsafe forbidden**

```rust
// src/lib.rs
#![forbid(unsafe_code)]
```

- [ ] **Step 3: Verify dependency resolution and the minimal library**

Run: `cargo check`

Expected: dependency resolution succeeds and the intentionally minimal library compiles.

- [ ] **Step 4: Commit the reproducible foundation**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml .gitignore src/lib.rs
git commit -m "chore(engine): pin the Rust 2026 WebGPU foundation"
```

### Task 2: Integer Campaign Model and Versioned Rules

**Files:**
- Create: `src/game/model.rs`
- Create: `src/game/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/game/model.rs`

**Interfaces:**
- Consumes: no engine, window, GPU, clock, or floating-point type.
- Produces: fixed-width identifiers and quantities, `RulesV1`, `World`, `Fleet`, `Campaign`, `GameCommand`, and terminal `Outcome`.

- [ ] **Step 1: Write model tests for the canonical seven-world campaign**

Add `pub mod game;` to `src/lib.rs` and `pub mod model;` to `src/game/mod.rs`, then add these tests at the end of `src/game/model.rs`.

```rust
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
        assert_eq!(campaign.worlds.iter().filter(|world| world.owner.is_none()).count(), 4);
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
        for (world, (position, output, defense, regeneration, fields))
            in campaign.worlds.iter().zip(expected)
        {
            assert_eq!((world.position.x, world.position.y), position);
            assert_eq!(world.base_output_per_second, Energy(output));
            assert_eq!(world.defense, Strength(defense));
            assert_eq!(world.base_regeneration_per_second, Strength(regeneration));
            assert_eq!([world.fields.atmosphere, world.fields.hydrosphere,
                world.fields.topology], fields);
        }
    }
}
```

- [ ] **Step 2: Run the focused tests and observe the missing-type failure**

Run: `cargo test game::model::tests --lib`

Expected: FAIL because the fixed-width campaign types are not defined.

- [ ] **Step 3: Implement identifiers, integer quantities, and rules**

```rust
pub const WORLD_COUNT: usize = 7;
pub const DEFAULT_SEED: u64 = 0x4947_5731_2026_0902;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct Tick(pub u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct WorldId(pub u8);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct FleetId(pub u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct CommandSequence(pub u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct Energy(pub u64);
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)] pub struct Strength(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Faction { Union, Helix, Choir }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind { Atmosphere, Hydrosphere, Topology }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldAdjustment { Increase, Decrease }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SectorPoint { pub x: i32, pub y: i32 }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldLevels { pub atmosphere: u8, pub hydrosphere: u8, pub topology: u8 }
impl FieldLevels {
    pub const fn new(atmosphere: u8, hydrosphere: u8, topology: u8) -> Self;
    pub const fn get(self, field: FieldKind) -> u8;
    pub fn set(&mut self, field: FieldKind, value: u8);
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
        Self { tick_hz: 60, win_world_count: 5, ai_period_ticks: 60,
            hazard_first_tick: 2700, hazard_period_ticks: 2700,
            hazard_duration_ticks: 720, maximum_energy: Energy(250_000),
            maximum_defense: Strength(100_000), minimum_launch: Energy(10_000),
            field_raise_cost: Energy(20_000), field_lower_refund: Energy(10_000),
            base_fleet_speed: 23 }
    }
}
```

- [ ] **Step 4: Implement campaign entities and domain commands**

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub id: WorldId,
    pub name: &'static str,
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
pub enum HazardKind { IonStorm, GravityTide }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveHazard {
    pub event_index: u64,
    pub kind: HazardKind,
    pub affected_field: FieldKind,
    pub start: Tick,
    pub end: Tick,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome { FactionVictory { winner: Faction }, PlayerEliminated }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase { Running, Finished { at: Tick, outcome: Outcome } }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameCommand {
    Launch { source: WorldId, destination: WorldId },
    TuneField { world: WorldId, field: FieldKind, adjustment: FieldAdjustment },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandEnvelope { pub tick: Tick, pub sequence: CommandSequence, pub command: GameCommand }

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
    pub fn new(seed: u64, rules: RulesV1) -> Self;
    pub fn controlled_count(&self, faction: Faction) -> usize;
    pub fn fleet_count(&self, faction: Faction) -> usize;
}
```

Implement Generator V1 exactly from the design: frozen SplitMix64, anchors `(1000,5000)`, `(9000,2500)`, `(9000,7500)`, `(5000,1000)`, `(5000,9000)`, `(4000,4000)`, `(6000,6000)`, and eight draws per world in world-ID order for x/y jitter, base output, starting defense, base regeneration, and three field levels. Names in world-ID order are Aster Vale, Khepri, Meridian, Vesper, Orison, Nacre, and Umbra. Initial owners are `Some(Union)`, `Some(Helix)`, `Some(Choir)`, then four neutral worlds. Owned worlds start at 80,000 energy and neutral worlds at zero. The renderer may convert positions, fields, energy, defense, and fleet progress to `f32`; no floating result flows back into campaign truth.

- [ ] **Step 5: Run and pass the model tests**

Run: `cargo test game::model::tests --lib`

Expected: 3 tests pass.

- [ ] **Step 6: Commit the model**

```bash
git add src/lib.rs src/game/mod.rs src/game/model.rs
git commit -m "feat(game): add the integer campaign model"
```

### Task 3: Exact Fixed-Tick Simulation, AI, Combat, and Outcomes

**Files:**
- Create: `src/game/simulation.rs`
- Modify: `src/game/mod.rs`
- Test: `src/game/simulation.rs`
- Create: `tests/campaign.rs`

**Interfaces:**
- Consumes: every public type from `game::model`; it never consumes wall time or rendering values.
- Produces: `Simulation::generated`, `enqueue`, `step`, `step_n`, `state`, `canonical_fingerprint`, and `fleet_progress_ratio`.

- [ ] **Step 1: Write failing command, economy, and terminal tests**

Add `pub mod simulation;` to `src/game/mod.rs`, then add these tests to `src/game/simulation.rs`.

```rust
#[test]
fn valid_launch_spends_exact_cost_and_allocates_monotonic_fleet_id() {
    let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    simulation.enqueue_next(CommandSequence(0), GameCommand::Launch {
        source: WorldId(0), destination: WorldId(2),
    }).unwrap();
    let before = simulation.state().worlds[0].energy;
    let report = simulation.step();
    assert_eq!(report.command_results[0], CommandResult::Accepted { sequence: CommandSequence(0) });
    assert_eq!(simulation.state().worlds[0].energy, Energy(before.0 - before.0 / 2));
    assert_eq!(simulation.state().fleets[0].strength, Strength(before.0 / 2));
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
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test game::simulation::tests --lib`

Expected: FAIL because `Simulation`, queueing, stepping, reports, and fingerprints are not defined.

- [ ] **Step 3: Implement the queue and typed command results**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandRejection {
    WrongTick, DuplicateSequence, CampaignFinished, InvalidWorld,
    SameSourceAndTarget, SourceNotOwnedByIssuer, InsufficientEnergy, FieldAtLimit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandResult {
    Accepted { sequence: CommandSequence },
    Rejected { sequence: CommandSequence, reason: CommandRejection },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameEvent {
    FleetLaunched(FleetId), WorldCaptured { world: WorldId, owner: Option<Faction> },
    HazardStarted(ActiveHazard), HazardEnded(HazardKind), CampaignFinished(Outcome),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TickReport { pub tick: Tick, pub command_results: Vec<CommandResult>, pub events: Vec<GameEvent> }

pub struct Simulation {
    state: Campaign,
    rules: RulesV1,
    pending: std::collections::BTreeMap<Tick, Vec<CommandEnvelope>>,
}
impl Simulation {
    pub fn generated(seed: u64, rules: RulesV1) -> Self;
    pub fn state(&self) -> &Campaign;
    pub fn enqueue(&mut self, envelope: CommandEnvelope) -> Result<(), CommandRejection>;
    pub fn enqueue_next(&mut self, sequence: CommandSequence, command: GameCommand)
        -> Result<(), CommandRejection>;
    pub fn step(&mut self) -> TickReport;
    pub fn step_n(&mut self, count: u32);
    pub fn canonical_fingerprint(&self) -> u64;
}

pub fn fleet_progress_ratio(fleet: &Fleet) -> (u64, u64) {
    (fleet.progress.min(fleet.route_length), fleet.route_length.max(1))
}
```

Implement the FNV-1a byte stream with explicit tags and widths: `u32` rules and generator versions; `u64` seed and next tick; `u8` phase tag (`0` running, `1` faction victory, `2` eliminated) followed by winning faction when present; `u64` next fleet ID and next hazard index; `u8` active-hazard tag followed, when present, by event index, kind tag, field tag, start, and end; then each world as ID `u8`, x/y `i32`, owner tag `u8` (`0` neutral, `1` Union, `2` Helix, `3` Choir), defense/energy/output/regeneration `u64`, three field bytes, and two `u64` remainders; then fleet count `u64` and every fleet scalar in FleetId order using fixed-width little-endian values. Pending commands and all session/view state are excluded. This encoding produces the pinned default digest above.

Queue entries only for `state.next_tick`; reject duplicate sequences. Sort same-tick commands by sequence. Selection, hover, pause, and restart remain session/UI state rather than campaign commands. A launch sends half the source energy rounded down when that half is at least 10,000, converting it one-for-one to strength. Increase moves one field level toward 10 and costs 20,000 milli-energy. Decrease moves one level toward 0 and refunds 10,000 up to the 250,000 cap, so repeated down/up cycles cannot mint energy.

- [ ] **Step 4: Implement one authoritative tick order and integer carry**

`Simulation::step` performs exactly this order:

1. Return without mutation when `Phase::Finished`.
2. Start or end a hazard at the half-open boundary for `state.next_tick`.
3. Clone the start-of-tick campaign for both AI decisions.
4. Apply external commands in ascending sequence.
5. At ticks divisible by 60 (first evaluation tick 60), generate both AI intents from the same snapshot and apply them in `Helix`, then `Choir` order.
6. Accrue energy and defense regeneration using integer remainder carry.
7. Advance every fleet and collect arrivals.
8. Resolve arrivals simultaneously per destination.
9. Resolve faction victory, otherwise player elimination.
10. Increment `next_tick`.

Use `u128` intermediates and this exact carry pattern for each per-second rate and basis-point factor:

```rust
let numerator = u128::from(previous_remainder)
    + u128::from(rate_per_second) * u128::from(factor_basis_points);
let divisor = u128::from(rules.tick_hz) * 10_000;
let gain = (numerator / divisor) as u64;
let remainder = (numerator % divisor) as u64;
```

Atmosphere production factor is `10_000 + level * 500` basis points. Topology regeneration per second is `world.base_regeneration_per_second + level * 100` milli-strength. A fleet snapshots hydrosphere at launch and its speed is `23 * (10_000 + level * 400) / 10_000` sector units per second. Compute route length as the floor of a documented integer square-root over `dx*dx + dy*dy`; preserve movement remainders. A fleet cannot arrive on its launch tick. Neutral worlds neither produce nor regenerate. Reaching either cap discards excess gain and clears that accumulator's remainder.

- [ ] **Step 5: Implement hazard intervals and stable AI tie-breaking**

Hazard event `n` starts at tick `2700 * (n + 1)` and occupies `[start, start + 720)`. Display kind alternates Ion Storm for even `n` and Gravity Tide for odd `n`. Its affected field is `splitmix64(seed ^ n) % 3` in atmosphere, hydrosphere, topology order. During the event only that field's contribution is multiplied by 5,000 basis points; base production, speed, and regeneration remain intact. A hydrosphere event affects fleets already in transit, using each fleet's snapshotted launch level.

At each AI cadence, compute every intent from the shared snapshot. A faction first reinforces the owned target with the largest positive `hostile inbound - defense - friendly inbound`, tied by smallest remaining hostile route then lowest world ID. Reinforcing sources tie by shortest route, greatest launchable half-energy strength, then world ID. If no feasible reinforcement exists, it attacks the non-owned target with lowest effective defense, tied by world ID; attacking sources tie by greatest launchable strength, shortest route, then world ID. All worlds are reachable. AI uses the same half-energy and 10,000 minimum rules as the player.

- [ ] **Step 6: Implement simultaneous arrival and terminal rules**

Aggregate arrivals by destination and faction before mutating a world. The incumbent force is current defense plus same-owner arrivals; every other force is that faction's arriving strength. If one faction force is strictly greater than the sum of every other force, it owns the world with the difference clamped to 1 through 100,000. Otherwise forces are exhausted: the incumbent remains at defense 1 when its force is at least every individual attacker, and the world becomes neutral at defense 1 in every other case. Captures clear stored energy, retain fields, and do not alter already-departed fleet factions. Consume every arriving fleet exactly once.

After arrivals, any faction controlling at least five worlds produces `Outcome::FactionVictory`. Otherwise, zero Union worlds plus zero Union fleets produces `Outcome::PlayerEliminated`. A Union fleet in transit prevents elimination. Terminal state freezes economy, hazards, AI, fleets, and gameplay mutation.

- [ ] **Step 7: Add headless ordering, hazard, combat, and replay tests**

```rust
// tests/campaign.rs
#[test]
fn hazard_boundaries_are_half_open_and_repeatable() {
    let mut simulation = Simulation::generated(DEFAULT_SEED, RulesV1::default());
    simulation.step_n(2700);
    assert_eq!(simulation.state().active_hazard, None);
    simulation.step();
    assert_eq!(simulation.state().active_hazard.unwrap().kind, HazardKind::IonStorm);
    simulation.step_n(719);
    assert!(simulation.state().active_hazard.is_some());
    simulation.step();
    assert_eq!(simulation.state().active_hazard, None);
}

#[test]
fn fleet_storage_order_cannot_change_simultaneous_combat() {
    let left = simultaneous_arrival_fixture(false);
    let right = simultaneous_arrival_fixture(true);
    assert_eq!(step_fixture(left).canonical_fingerprint(), step_fixture(right).canonical_fingerprint());
}

#[test]
fn identical_replay_and_chunked_steps_produce_identical_fingerprints() {
    let mut one = scripted_simulation();
    let mut two = scripted_simulation();
    for _ in 0..3600 { one.step(); }
    for _ in 0..60 { two.step_n(60); }
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
    assert!((36_000..=54_000).contains(&finished_tick.0));
}
```

Implement `run_scripted_union_commander` as a test-only policy that evaluates every 600 ticks, selects the Union source with greatest launchable half-energy (world-ID tie-break), and targets the non-Union world with lowest effective defense (world-ID tie-break); it enqueues one launch when valid and otherwise advances. This is a balance sentinel, not production AI. Also add focused unit tests named `ai_first_evaluates_at_tick_60`, `ai_ties_end_at_lowest_world_id`, `field_round_trip_loses_energy`, `exact_force_combat_keeps_one_defense`, `attacker_tie_neutralizes_world`, `capture_clears_energy_and_retains_fields`, `fifth_world_finishes_for_each_faction`, and `rejected_command_preserves_fingerprint`.

- [ ] **Step 8: Run the complete simulation gate**

Run: `cargo test game::simulation::tests --lib && cargo test --test campaign`

Expected: all command, integer-carry, AI, hazard-boundary, simultaneous-combat, outcome, and replay tests pass.

- [ ] **Step 9: Commit simulation behavior**

```bash
git add src/game/mod.rs src/game/simulation.rs tests/campaign.rs
git commit -m "feat(game): implement deterministic sector simulation"
```

### Task 4: Engine Input and Fixed Clock

**Files:**
- Create: `src/engine/input.rs`
- Create: `src/engine/time.rs`
- Create: `src/engine/mod.rs`
- Modify: `src/lib.rs`
- Test: `src/engine/input.rs`
- Test: `src/engine/time.rs`

**Interfaces:**
- Consumes: winit `KeyCode`, `MouseButton`, and `ElementState` values in the app adapter only.
- Produces: engine-owned `InputState`, `Action`, and `FixedClock` types that do not leak winit events into game code.

- [ ] **Step 1: Write failing edge-trigger and accumulator tests**

Add `pub mod engine;` to `src/lib.rs`; create `src/engine/mod.rs` with `pub mod input;` and `pub mod time;`; then add the focused tests below.

```rust
#[test]
fn a_pressed_action_is_consumed_once() {
    let mut input = InputState::default();
    input.set_action(Action::Launch, true);
    assert!(input.take_pressed(Action::Launch));
    assert!(!input.take_pressed(Action::Launch));
}

#[test]
fn fixed_clock_clamps_long_frames_and_limits_work() {
    let mut clock = FixedClock::new(60, 15);
    clock.push_frame(std::time::Duration::from_secs(2));
    let mut steps = 0;
    while clock.take_step() { steps += 1; }
    assert_eq!(steps, 15);
    assert!((0.0..1.0).contains(&clock.alpha()));
}
```

- [ ] **Step 2: Implement engine-owned input semantics**

```rust
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action { Field1, Field2, Field3, Decrease, Increase, Launch, Pause, Restart, Help, Cancel }

#[derive(Default)]
pub struct InputState {
    pub cursor_logical: glam::Vec2,
    pub left_clicked: bool,
    pub right_clicked: bool,
    held: std::collections::HashSet<Action>,
    pressed: std::collections::HashSet<Action>,
}

impl InputState {
    pub fn set_action(&mut self, action: Action, down: bool);
    pub fn take_pressed(&mut self, action: Action) -> bool;
    pub fn finish_frame(&mut self);
}
```

```rust
pub struct FixedClock {
    tick_hz: u32,
    scaled_accumulator: u128,
    pending_steps: usize,
    max_steps: usize,
}
impl FixedClock {
    pub fn new(tick_hz: u32, max_steps: usize) -> Self;
    pub fn push_frame(&mut self, delta: std::time::Duration);
    pub fn take_step(&mut self) -> bool;
    pub fn alpha(&self) -> f32;
    pub fn clear(&mut self);
}
```

Clamp each frame delta to 250 milliseconds, add `accepted_delta.as_nanos() * tick_hz` to `scaled_accumulator`, derive whole steps by dividing by 1,000,000,000, and preserve the remainder. Cap pending work at 15 steps and discard excess whole steps. `alpha` is the remainder divided by 1,000,000,000 for render interpolation only. Clear the accumulator on pause transitions and restart so paused wall time is never replayed.

- [ ] **Step 3: Run input and clock tests**

Run: `cargo test --lib engine::`

Expected: all focused tests pass.

- [ ] **Step 4: Commit the platform-neutral frame controls**

```bash
git add src/lib.rs src/engine/mod.rs src/engine/input.rs src/engine/time.rs
git commit -m "feat(engine): add deterministic input and fixed timing"
```

### Task 5: Primitive Batcher, Bitmap Font, Layout, and Hit Testing

**Files:**
- Create: `src/engine/primitives.rs`
- Create: `src/game/view.rs`
- Modify: `src/engine/mod.rs`
- Modify: `src/game/mod.rs`
- Test: `src/engine/primitives.rs`
- Test: `src/game/view.rs`

**Interfaces:**
- Consumes: `Campaign`, `fleet_progress_ratio`, viewport logical size, cursor position, and UI-only selection state.
- Produces: `Vertex`, `PrimitiveBatch`, `GameLayout`, `ViewState`, `hit_test_world`, and `build_frame`.

- [ ] **Step 1: Write failing geometry and hit-test tests**

Export `primitives` from `src/engine/mod.rs` and `view` from `src/game/mod.rs`, then add the focused tests below.

```rust
#[test]
fn quad_emits_two_triangles() {
    let mut batch = PrimitiveBatch::default();
    batch.quad(glam::Vec2::ZERO, glam::Vec2::ONE, [1.0; 4]);
    assert_eq!(batch.vertices().len(), 6);
}

#[test]
fn center_of_a_world_hits_that_world() {
    let game = Campaign::new(DEFAULT_SEED, RulesV1::default());
    let layout = GameLayout::new(glam::Vec2::new(1440.0, 900.0));
    let center = layout.world_to_screen(game.worlds[0].position);
    assert_eq!(hit_test_world(&game, &layout, center), Some(WorldId(0)));
}

#[test]
fn bitmap_glyph_vertices_stay_inside_their_seven_by_five_cell() {
    let origin = glam::Vec2::new(10.0, 20.0);
    let mut batch = PrimitiveBatch::default();
    batch.text(origin, 1.0, [1.0; 4], "A");
    assert!(!batch.vertices().is_empty());
    assert!(batch.vertices().iter().all(|vertex| {
        (10.0..=15.0).contains(&vertex.position[0])
            && (20.0..=27.0).contains(&vertex.position[1])
    }));
}
```

- [ ] **Step 2: Implement a single packed vertex and primitive API**

```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub local: [f32; 2],
    pub shape: u32,
}

impl Vertex {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Float32x2,
        3 => Uint32,
    ];
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &Self::ATTRIBUTES,
    };
}

#[derive(Default)]
pub struct PrimitiveBatch { vertices: Vec<Vertex> }
impl PrimitiveBatch {
    pub fn clear(&mut self);
    pub fn vertices(&self) -> &[Vertex];
    pub fn quad(&mut self, center: glam::Vec2, half_size: glam::Vec2, color: [f32; 4]);
    pub fn disc(&mut self, center: glam::Vec2, radius: f32, color: [f32; 4]);
    pub fn ring(&mut self, center: glam::Vec2, radius: f32, thickness: f32, color: [f32; 4]);
    pub fn line(&mut self, from: glam::Vec2, to: glam::Vec2, width: f32, color: [f32; 4]);
    pub fn text(&mut self, origin: glam::Vec2, scale: f32, color: [f32; 4], value: &str);
}
```

Reserve the low eight `shape` bits for the primitive kind: 0 is an unmasked quad, 1 is a disc using `local` distance, and 2 is a ring. Ring vertices pack the clamped, quantized `thickness / radius` ratio into the upper 24 bits, allowing the fixed 36-byte vertex ABI to retain visible per-ring width. The packed layout is 36 bytes with attributes at byte offsets 0, 8, 24, and 32. Text maps uppercase ASCII, digits, colon, dash, slash, percent, period, and space through a static 5-column by 7-row bit table; each active bit emits one shape-0 quad centered in its cell, a glyph advances six cells, and unsupported bytes render as a boxed replacement glyph.

- [ ] **Step 3: Implement responsive game layout and frame construction**

```rust
pub struct GameLayout {
    pub viewport: glam::Vec2,
    pub playfield_min: glam::Vec2,
    pub playfield_max: glam::Vec2,
    pub ui_scale: f32,
}

impl GameLayout {
    pub fn new(viewport: glam::Vec2) -> Self;
    pub fn world_to_screen(&self, world_position: SectorPoint) -> glam::Vec2;
}

#[derive(Clone, Copy, Debug)]
pub struct ViewState {
    pub selected_world: Option<WorldId>,
    pub hovered_world: Option<WorldId>,
    pub selected_field: FieldKind,
    pub paused: bool,
    pub help_visible: bool,
}

pub fn hit_test_world(game: &Campaign, layout: &GameLayout, cursor: glam::Vec2) -> Option<WorldId>;
pub fn build_frame(game: &Campaign, view: ViewState, layout: &GameLayout, batch: &mut PrimitiveBatch);
```

Reserve `240 * ui_scale` pixels left, `280 * ui_scale` right, `64 * ui_scale` top, and `56 * ui_scale` bottom. Clamp `ui_scale` to 0.72 through 1.15 based on the smaller viewport ratio to 1440 by 900. `build_frame` emits the fixed seeded star field, tactical grid, fleet trails, planet rings/discs/field bands, selection state, mission/status panels, inspector data, controls, hazard banner, and win/loss overlay. Every displayed label must be supported by the bitmap alphabet.

Map each integer sector coordinate from the inclusive logical range 0 through 10,000 linearly into `playfield_min..playfield_max`. A world hit uses a `22 * ui_scale` logical-pixel radius. When hit circles overlap, return the lowest `WorldId`, so pointer behavior is independent of vector iteration order.

- [ ] **Step 4: Run geometry and view tests**

Run: `cargo test --lib`

Expected: batching, glyph bounds, responsive layout, and hit tests pass.

- [ ] **Step 5: Commit the complete CPU presentation model**

```bash
git add src/engine/mod.rs src/game/mod.rs src/engine/primitives.rs src/game/view.rs
git commit -m "feat(ui): add GPU primitive and command-table layout"
```

> **Superseded record:** The following archived original Tasks 6 through 9 are retained for history only. Do not execute them; continue at **Expanded Executable Continuation**.

### Archived Original Task 6: Naga-Validated WGSL and GPU Renderer

**Files:**
- Create: `assets/shaders/primitives.wgsl`
- Create: `src/engine/shader.rs`
- Create: `src/engine/gpu.rs`
- Create: `src/engine/render.rs`
- Modify: `src/engine/mod.rs`
- Create: `tests/shaders.rs`

**Interfaces:**
- Consumes: `Vertex`, physical window size, and `Arc<Window>`.
- Produces: `validate_wgsl`, `GpuContext::new/resize/acquire`, and `Renderer::new/render`.

- [ ] **Step 1: Write a failing shipped-shader validation test**

Export `gpu`, `render`, and `shader` from `src/engine/mod.rs`, then add the integration test below.

```rust
use intergalactic_warfare::engine::shader::validate_wgsl;

#[test]
fn shipped_primitive_shader_is_valid_naga_wgsl() {
    let source = include_str!("../assets/shaders/primitives.wgsl");
    validate_wgsl("primitives.wgsl", source).expect("shader must validate");
}

#[test]
fn invalid_shader_reports_its_label() {
    let error = validate_wgsl("broken.wgsl", "@vertex fn nope(").unwrap_err();
    assert!(error.to_string().contains("broken.wgsl"));
}
```

- [ ] **Step 2: Define the shared engine error boundary**

```rust
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("surface creation failed: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("no compatible graphics adapter: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("graphics device creation failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("the selected adapter cannot configure this surface")]
    UnsupportedSurface,
    #[error(transparent)]
    Shader(#[from] shader::ShaderError),
}
```

- [ ] **Step 3: Implement Naga validation before pipeline creation**

```rust
#[derive(Debug, thiserror::Error)]
pub enum ShaderError {
    #[error("WGSL parse failed for {label}: {message}")]
    Parse { label: String, message: String },
    #[error("WGSL validation failed for {label}: {message}")]
    Validate { label: String, message: String },
}

pub fn validate_wgsl(label: &str, source: &str) -> Result<(), ShaderError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| ShaderError::Parse { label: label.into(), message: error.emit_to_string(source) })?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|error| ShaderError::Validate {
        label: label.into(), message: error.emit_to_string(source),
    })?;
    Ok(())
}
```

- [ ] **Step 4: Add the complete primitive shader**

```wgsl
struct Globals {
    viewport: vec2<f32>,
    time: f32,
    padding: f32,
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct VertexIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local: vec2<f32>,
    @location(3) shape: u32,
};
struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) @interpolate(flat) shape: u32,
};

@vertex fn vs_main(input: VertexIn) -> VertexOut {
    var out: VertexOut;
    let clip = vec2<f32>(input.position.x / globals.viewport.x * 2.0 - 1.0,
                         1.0 - input.position.y / globals.viewport.y * 2.0);
    out.clip_position = vec4<f32>(clip, 0.0, 1.0);
    out.color = input.color;
    out.local = input.local;
    out.shape = input.shape;
    return out;
}

@fragment fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    var alpha = input.color.a;
    let distance = length(input.local);
    let kind = input.shape & 0xffu;
    if kind == 1u { alpha *= 1.0 - smoothstep(0.92, 1.0, distance); }
    if kind == 2u {
        let width = f32(input.shape >> 8u) / 16777215.0;
        let outer = 1.0 - smoothstep(0.92, 1.0, distance);
        let inner_edge = clamp(1.0 - width, 0.0, 1.0);
        let inner = smoothstep(inner_edge - 0.04, inner_edge + 0.04, distance);
        alpha *= outer * inner;
    }
    if alpha <= 0.001 { discard; }
    return vec4<f32>(input.color.rgb, alpha);
}
```

- [ ] **Step 5: Implement surface and device lifecycle**

```rust
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub surface: Option<wgpu::Surface<'static>>,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: Option<wgpu::SurfaceConfiguration>,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl GpuContext {
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> Result<Self, EngineError>;
    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) -> Result<bool, EngineError>;
    pub fn recreate_surface(&mut self, window: std::sync::Arc<winit::window::Window>)
        -> Result<bool, EngineError>;
    pub fn acquire(&self) -> Option<wgpu::CurrentSurfaceTexture>;
}
```

This archived draft originally used `wgpu::Instance::default()`; that is superseded because it does not enforce the explicit native-primary/browser-WebGPU backend policy. The authoritative Task 6 below uses `InstanceDescriptor::new_without_display_handle()` and assigns the backend before constructing the instance. Request the adapter with `RequestAdapterOptions { power_preference: HighPerformance, force_fallback_adapter: false, compatible_surface: Some(&surface), apply_limit_buckets: false }`; the result is a `Result`, not an `Option`. Request the device with the one-argument `adapter.request_device(&DeviceDescriptor { label, required_features: Features::empty(), required_limits: Limits::default(), ..Default::default() })`; the defaulted wgpu 30 fields are experimental features disabled, performance memory hints, and trace off.

At nonzero size, call `surface.get_default_config(&adapter, width, height)` so all nine wgpu 30 fields, including `color_space`, are supported, then configure. At zero size set `config` to `None` and never call `configure`. Retain the instance because `CurrentSurfaceTexture::Lost` requires a newly created surface; Outdated only requires resize/reconfiguration. `resize` and `recreate_surface` return whether the render-target format changed so the app can rebuild the format-bound pipeline. `acquire` returns `None` while unconfigured and otherwise returns the exact `CurrentSurfaceTexture` enum.

- [ ] **Step 6: Implement the alpha-blended renderer**

```rust
pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    vertex_capacity: usize,
    logged_first_frame: bool,
}

impl Renderer {
    pub fn new(gpu: &GpuContext) -> Result<Self, EngineError>;
    pub fn render(&mut self, gpu: &GpuContext, frame: wgpu::SurfaceTexture,
        vertices: &[Vertex], time: f32);
}
```

Validate the included WGSL before `create_shader_module`. Grow the vertex buffer to the next power of two when required; upload only the used byte slice with `queue.write_buffer`. The wgpu 30 pipeline uses `VertexState { entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[Some(Vertex::LAYOUT)] }`, the analogous fragment state, `ALPHA_BLENDING`, triangle-list topology, no depth buffer, `multiview_mask: None`, and `cache: None`. The render pass attachment includes `depth_slice: None`; the pass descriptor includes `multiview_mask: None`. Clear to `(0.012, 0.018, 0.045, 1.0)`, draw once over `0..vertices.len()`, submit, then call `gpu.queue.present(frame)` because wgpu 30 presentation is a queue operation.

Create an explicit 16-byte uniform buffer for `Globals`, an explicit binding-0 uniform bind-group layout visible to the vertex stage, and a bind group from that layout. Use the same layout when creating the pipeline layout, then bind group 0 before drawing. This keeps the host-side resource contract inspectable instead of depending on inferred layout behavior.

Create or rebuild `Renderer` only while `gpu.config` is `Some`; if the first resume is zero-sized, wait for a nonzero resize. Log the selected adapter name/backend/device type and configured format/color space once during initialization. Log `first frame presented` once after the first successful `queue.present`. The app matches acquisition exactly: Success renders and presents; Suboptimal renders and presents before resize/reconfiguration; Timeout and Occluded skip; Outdated reconfigures for a later redraw; Lost calls `recreate_surface`; Validation sets the fatal error and exits. Rebuild the renderer whenever resize/recreation reports a format change. Never reconfigure while a live frame still exists.

- [ ] **Step 7: Run CPU shader validation and compile the GPU code**

Run: `cargo test --test shaders && cargo check`

Expected: both shader tests pass and the project compiles through the renderer modules.

- [ ] **Step 8: Commit the validated GPU backend**

```bash
git add assets/shaders/primitives.wgsl src/engine/mod.rs src/engine/shader.rs src/engine/gpu.rs src/engine/render.rs tests/shaders.rs
git commit -m "feat(render): add the Naga-validated WebGPU backend"
```

### Archived Original Task 7: Winit Application Lifecycle and Complete Play Controls

- [ ] **Step 1: Write a platform-neutral input-to-command mapping test**

```rust
#[test]
fn right_click_on_target_builds_launch_command_from_selection() {
    let game = Campaign::new(DEFAULT_SEED, RulesV1::default());
    let layout = GameLayout::new(glam::Vec2::new(1440.0, 900.0));
    let cursor = layout.world_to_screen(game.worlds[2].position);
    assert_eq!(pointer_command(&game, &layout, Some(WorldId(0)), cursor, PointerIntent::Launch),
        Some(GameCommand::Launch { source: WorldId(0), destination: WorldId(2) }));
}
```

- [ ] **Step 2: Implement the app state and lifecycle**

Add `pub mod app;` to `src/lib.rs`, add the `[[bin]]` entry naming `src/main.rs` to `Cargo.toml`, and create this binary entry point:

```rust
#![forbid(unsafe_code)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    intergalactic_warfare::app::run()?;
    Ok(())
}
```

```rust
struct App {
    window: Option<std::sync::Arc<winit::window::Window>>,
    gpu: Option<GpuContext>,
    renderer: Option<Renderer>,
    simulation: Simulation,
    input: InputState,
    clock: FixedClock,
    batch: PrimitiveBatch,
    last_frame: std::time::Instant,
    selected_world: Option<WorldId>,
    hovered_world: Option<WorldId>,
    selected_field: FieldKind,
    paused: bool,
    help_visible: bool,
    next_sequence: CommandSequence,
    fatal_error: Option<String>,
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn suspended(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId, event: winit::event::WindowEvent);
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop);
}

pub fn run() -> Result<(), AppError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    app.fatal_error.map_or(Ok(()), |message| Err(AppError::Startup(message)))
}
```

Define the application error boundary explicitly:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("event loop failed: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("application startup failed: {0}")]
    Startup(String),
}
```

A window, adapter, device, or renderer initialization failure stores its full message in `fatal_error` and exits the loop; it must not panic inside `ApplicationHandler`.

Create the window only in `resumed`, with title `Intergalactic Warfare // Sector Command`, inner size 1440 by 900, minimum size 960 by 600, and `WindowLevel::Normal`. Initialize GPU and renderer synchronously through `pollster::block_on` only during resume, tolerate redundant resume events, and render only from `WindowEvent::RedrawRequested`. In `suspended`, drop renderer and GPU state before returning so the surface releases its native handle; preserve the session simulation and window handle for an idempotent resume.

- [ ] **Step 3: Translate winit events into the documented controls**

Map physical keys Digit1, Digit2, Digit3, KeyQ, KeyE, Space, KeyP, KeyR, KeyH, and Escape to `Action`. Convert cursor physical coordinates by dividing through `window.scale_factor()`. Left press updates UI-only selection, right press queues a launch from the selected world to the hit target, Space launches to the currently hovered target, Q/E queue `TuneField`, and Escape clears selection. P toggles the UI/session pause flag and clears the fixed clock; R replaces the simulation with `Simulation::generated(DEFAULT_SEED, RulesV1::default())` and clears selection, pause, input edges, and the clock. H toggles only the help overlay. Never store hover, selection, pause, or help in `Campaign`, and never emit commands from the renderer.

- [ ] **Step 4: Drive fixed simulation and robust presentation**

On each redraw: push clamped wall time into `FixedClock`, translate input edges into monotonically sequenced commands for `simulation.state().next_tick`, and call `simulation.step()` once per available fixed step unless paused. Build `ViewState` from UI-only fields, rebuild the primitive batch from immutable campaign state, and render. Handle the exact wgpu 30 `CurrentSurfaceTexture` variants in Task 6 rather than matching the removed `SurfaceError` API: reconfigure Outdated, recreate Lost, skip Timeout or Occluded, exit on Validation, present Success, and present Suboptimal before reconfiguring. A zero-sized resize updates stored size but skips surface configuration and redraw while simulation continues. `about_to_wait` requests redraw continuously.

- [ ] **Step 5: Run app tests and compile all targets**

Run: `cargo test app::tests --lib && cargo check --all-targets`

Expected: command mapping tests pass and all native targets compile.

- [ ] **Step 6: Commit the playable application**

```bash
git add Cargo.toml src/lib.rs src/app.rs src/main.rs
git commit -m "feat(app): connect winit controls to the campaign"
```

### Archived Original Task 8: Player Documentation and Reusable Engine Skill

**Files:**
- Create: `README.md`
- Modify: `/Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine/SKILL.md`
- Review unchanged: `/Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine/agents/openai.yaml`
- Modify: `/Users/donaldfilimon/.grok/sync-targets.json`

**Interfaces:**
- Consumes: the implemented controls, architecture, exact verification commands, and machine central-skill policy.
- Produces: player/operator documentation and a validated discoverable skill for future custom Rust WebGPU game work.

- [ ] **Step 1: Write a concise player and developer README**

```markdown
# Intergalactic Warfare

A deterministic fictional sector-control game rendered by a custom Rust WebGPU engine.

## Play

- Left click: select a world
- Right click or Space: launch from the selected world to the hovered world
- 1 / 2 / 3: choose atmosphere, hydrosphere, or topology
- Q / E: reduce or increase the chosen field
- P: pause
- R: restart
- H: help
- Escape: clear selection

Run with `cargo run --release`.

## Verify

Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
`cargo test --all-targets`, and `cargo build --release`.
```

Expand this with the campaign objective, field effects, module ownership, deterministic simulation boundary, and an explicit statement that the game uses fictional abstractions rather than real-world targeting or effects data.

- [ ] **Step 2: Replace the initialized skill scaffold with focused instructions**

```markdown
---
name: rust-webgpu-game-engine
description: Build, extend, debug, and validate low-level Rust game engines and simulations that use winit, wgpu, WGSL, or Naga. Use for event-loop and window lifecycle design, GPU rendering or compute pipelines, shader validation, resource-layout bugs, deterministic simulation, and measured performance work. Do not route engine-only Bevy or Godot work here unless it requires direct low-level interop.
---

# Rust WebGPU Game Engine

Build against the repository's pinned Rust, winit, wgpu, and Naga versions. Inspect its instructions, `Cargo.toml`, `Cargo.lock`, toolchain file, target platforms, shaders, and existing gate before choosing APIs; these crates evolve quickly, so do not code from a remembered newer interface.

## Preserve the boundaries

- Let winit own application and window lifecycle, event ingestion, redraw scheduling, resize, suspend/resume, and shutdown.
- Keep deterministic simulation state and fixed-step advancement separate from rendering, interpolation, and presentation.
- Give the renderer explicit ownership of the instance, surface, adapter, device, queue, surface configuration, pipelines, bind groups, buffers, textures, and per-frame resources.
- Choose surface formats and modes from reported capabilities. Handle zero-sized windows and surface loss or reconfiguration explicitly.
- Acquire, encode, submit, and present once per rendered frame. Keep pipeline creation, large allocation, blocking polls, and readback out of the hot path unless measurement justifies them.
- Keep input actions and game commands distinct from raw platform events so simulation tests do not require a live window.

## Keep shaders and host layouts honest

- Validate every WGSL shader and retain useful diagnostics. Use direct Naga parsing and validation when the project already needs Naga-level inspection; otherwise wgpu pipeline creation may be the authoritative validation path.
- Keep shader entry points, vertex formats, bind-group layouts, resource usages, texture formats, and host-side structs synchronized.
- Check alignment, padding, minimum binding sizes, dynamic offsets, workgroup sizes, and dispatch bounds explicitly.
- Add bounds checks for compute workloads and make CPU/GPU indexing conventions identical.
- Do not add `unsafe` merely to make data upload convenient. Use byte-casting only for types whose representation and alignment are proven suitable.

## Validate the result

- Test simulation, scheduling, culling, transforms, resource planning, and other CPU-owned logic without a GPU.
- Add focused shader or pipeline validation and small GPU readback tests where they protect a real invariant.
- Exercise resize, suspend/resume, surface recovery, empty scenes, device or adapter unavailability, and teardown paths relevant to the supported platforms.
- Run the repository's declared gate. If none exists, use the applicable format, clippy, test, shader-validation, build, and executable-smoke checks.
- Distinguish compilation and unit-test evidence from a real window/GPU smoke test. Record the tested target, adapter/backend, and limitations.
- Profile before claiming a performance improvement; report measurements and avoid generalizing one adapter or platform result to all WebGPU targets.
```

Keep the skill self-contained because this workflow does not need a maintained reference or script. Preserve the generated `agents/openai.yaml` exactly as created.

- [ ] **Step 3: Validate the self-contained skill with a Python that has PyYAML**

```bash
PYTHONDONTWRITEBYTECODE=1 \
/Users/donaldfilimon/.local/pipx/venvs/pip/bin/python \
/Users/donaldfilimon/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
/Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine
```

Expected: `Skill is valid!` and exit 0. Plain `python3` is not the gate on this machine because its environment currently lacks PyYAML.

- [ ] **Step 4: Classify the central skill without disturbing unrelated skill work**

Add exactly `"rust-webgpu-game-engine"` to `catalog.portableSkills` in `/Users/donaldfilimon/.grok/sync-targets.json`. Before staging, require the live status to still show the four pre-existing tracked deletions `check-work/SKILL.md`, `code-review/SKILL.md`, `create-skill/SKILL.md`, and `imagine/SKILL.md`; preserve them exactly and stage only the new skill directory plus `sync-targets.json`. Commit those exact paths with `feat(skills): add Rust WebGPU engine guidance`.

Do not run the central or ABI synchronization while those unrelated source/catalog deletions remain unresolved. The driver validates the complete central directory before target selection and would fail even in dry-run mode. Report the skill as complete and centrally versioned, with cross-CLI propagation blocked by pre-existing catalog/source drift. Do not restore the four files, remove their catalog entries, stage their deletions, or edit generated target copies under this task.

- [ ] **Step 5: Commit only the project-owned documentation**

```bash
git add README.md
git commit -m "docs(game): add play and verification guidance"
```

The central skill is outside this repository and must be reported separately rather than staged into the game commit.

### Archived Original Task 9: Full Verification, Native Smoke Run, and Final Review

**Files:**
- Modify only files implicated by observed failures.
- Review: every tracked project file and the central skill files from Task 8.

**Interfaces:**
- Consumes: the complete repository and central skill.
- Produces: green automated gates, recorded native startup evidence, a clean default branch, and an honest acceptance report.

- [ ] **Step 1: Run formatting and lint gates**

Run: `cargo fmt --all --check`

Expected: exit 0.

Run: `cargo clippy --all-targets --all-features -- -D warnings`

Expected: exit 0 with no warnings.

- [ ] **Step 2: Run all tests and the release build**

Run: `cargo test --all-targets`

Expected: every model, simulation, input, clock, geometry, view, shader, app mapping, and integration test passes.

Run: `cargo build --release`

Expected: `target/release/intergalactic-warfare` is produced for macOS.

- [ ] **Step 3: Perform a bounded native startup smoke run**

Run the release binary in a PTY with `RUST_LOG=wgpu=warn,intergalactic_warfare=info`. Wait until startup logs confirm the adapter, surface format, and first rendered frame, then send an interrupt. If a panic, validation error, or surface error appears, fix it and repeat. Record this as startup/render evidence only; do not claim it proves a complete manual campaign.

- [ ] **Step 4: Perform manual visual and interaction acceptance at both target sizes**

Launch the release app and inspect the real native window at 1440 by 900. Select a Union world, launch at a neutral world, change each selected field, pause/unpause, toggle help, and restart; verify the inspector, energy, fleet trail, hazard area, pause indicator, and restarted state respond. Resize the native window to 960 by 600 and verify the top bar, side panels, world hit targets, bottom controls, and bitmap text remain visible without overlap or clipping. Capture one screenshot at each size for review, but do not commit screenshots unless the user asks for them. Record this as manual macOS acceptance, separate from automated tests.

- [ ] **Step 5: Review the diff and tracked tree as a fresh reviewer**

Run:

```bash
git status --short --branch
git log --oneline --decorate -10
git diff f2b6d94..HEAD
rg -n "FIXME|unimplemented!\(|panic!\(" src assets README.md
```

Expected: the main branch contains all planned commits, no uncommitted project changes remain, and the placeholder scan finds nothing requiring completion. Review ownership boundaries, shader attribute alignment, vertex stride, fixed-tick ordering, action edge semantics, and every user-visible control against the design.

- [ ] **Step 6: Re-run the complete acceptance gate after review fixes**

Run:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
PYTHONDONTWRITEBYTECODE=1 \
/Users/donaldfilimon/.local/pipx/venvs/pip/bin/python \
/Users/donaldfilimon/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
/Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine
```

Expected: every command exits 0.

- [ ] **Step 7: Commit any verification-driven corrections**

If Task 9 changed project files, commit them with a subject derived from the actual diff, for example:

```bash
git add <exact changed project paths>
git commit -m "fix(engine): harden the verified render loop"
```

If verification required no project change, create no empty commit.

## Archived Original Plan Self-Review Record

- Spec coverage: Tasks 2 and 3 cover the complete loop and deterministic rules; Tasks 4 through 7 cover input, time, presentation, shaders, GPU, errors, resize, and controls; Task 8 covers both documentation deliverables; Task 9 covers all five acceptance layers.
- Scope at the time of this archived review was one native vertical slice. The expanded continuation below supersedes its browser, editor, and advisory exclusions.
- Type consistency: `Campaign`, `Simulation`, `GameCommand`, `CommandEnvelope`, `PrimitiveBatch`, `GameLayout`, `GpuContext`, and `Renderer` keep one spelling and ownership boundary throughout.
- Verification honesty: automated tests, release compilation, and bounded startup are separate from a human-completed campaign playthrough.

The archived original Tasks 6 through 9 above are retained only to preserve the reviewed planning history. They are not executable after the user's scope expansion. The numbered continuation below is authoritative.

## Expanded Executable Continuation

### Task 6: Portable Naga-Validated WebGPU Renderer

**Files:**
- Create: `assets/shaders/primitives.wgsl`
- Create: `src/engine/shader.rs`
- Create: `src/engine/gpu.rs`
- Create: `src/engine/render.rs`
- Modify: `src/engine/mod.rs`
- Create: `tests/shaders.rs`

**Interfaces:**
- Consumes: the reviewed 36-byte `Vertex` ABI, `Arc<Window>`, physical surface size, and a separate logical viewport.
- Produces: `validate_wgsl`, async `GpuContext`, exact wgpu 30 acquisition states, and one-pass `Renderer`.

- [ ] **Step 1: RED — freeze shader validation and packed-ring consumption**

Add tests proving the shipped shader parses and validates with Naga, a parse error includes its label, and a parse-valid interface collision reaches the validation-error branch. Scan the shader source for a low-byte kind mask and upper-24-bit ring-width decode. Run:

```bash
cargo test --test shaders
```

Expected RED: the shader/module APIs do not exist.

- [ ] **Step 2: GREEN — implement the portable shader boundary**

Use `naga::front::wgsl::parse_str` and `Validator::new(ValidationFlags::all(), Capabilities::default())`. Emit path-aware diagnostics with `emit_to_string_with_path`. The primitive shader uses `@interpolate(flat)` for the integer shape, masks `shape & 0xffu`, decodes `shape >> 8u` over 16,777,215, computes derivatives before divergent branches, guards a zero viewport, and preserves straight alpha.

- [ ] **Step 3: GREEN — implement safe async GPU and surface ownership**

`GpuContext::new(Arc<Window>)` creates `Surface<'static>` from an owned window clone. For locked wgpu 30, start from `InstanceDescriptor::new_without_display_handle()`, assign `Backends::PRIMARY` natively or `Backends::BROWSER_WEBGPU` on wasm, and pass the descriptor by value to `Instance::new`; `InstanceDescriptor` has no `Default`.

Request `Features::empty()` and `Limits::default()`. Use `get_default_config` only at nonzero physical size. Model `CurrentSurfaceTexture::{Success,Suboptimal,Timeout,Occluded,Outdated,Lost,Validation}` exactly. Present through `Queue::present`. Never configure zero size or reconfigure while a suboptimal frame is live.

- [ ] **Step 4: GREEN — implement the logical-coordinate renderer**

Validate WGSL before pipeline creation. Create explicit 16-byte globals, bind-group and pipeline layouts, an alpha-blended pipeline, and a nonzero growable `VERTEX | COPY_DST` buffer. `Renderer::render` accepts both the physical frame and `logical_viewport: [f32; 2]`; clip conversion must use the same logical coordinates used by `GameLayout` and hit testing, never the Retina/CSS backing extent.

- [ ] **Step 5: Verify and commit**

```bash
cargo fmt --all --check
cargo test --test shaders
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
git add assets/shaders/primitives.wgsl src/engine/mod.rs src/engine/shader.rs src/engine/gpu.rs src/engine/render.rs tests/shaders.rs
git commit -m "feat(render): add the portable WebGPU renderer"
```

Expected: all gates pass; GPU creation is not required by the shader tests.

### Task 7: Validated Scenario Domain and Full Editor Core

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`, `src/lib.rs`, `src/game/mod.rs`, `src/game/model.rs`, `src/game/simulation.rs`, `src/engine/input.rs`
- Create: `src/scenario/mod.rs`, `src/scenario/codec.rs`, `src/scenario/store.rs`, `src/editor.rs`
- Create: `src/platform/mod.rs`, `src/platform/native.rs`
- Create: `tests/scenario.rs`, `tests/scenario_editor.rs`

**Interfaces:**
- Consumes: factory generation, `RulesV1`, fixed seven-world model, primitive batch, and platform-neutral input.
- Produces: private validated `ScenarioV1`, strict codec/fingerprint, new-session construction, one-slot persistence, and a detached keyboard/pointer editor.

- [ ] **Step 1: RED — scenario validation, identity, codec, and construction**

Write tests for the generated default, every boundary and relational validation rule, duplicate-position error, stable warnings, exact 64 KiB/unknown-field/canonical-number handling, JSON round-trip, and field-by-field fingerprint sensitivity. Assert a name-only edit changes `ScenarioFingerprint` but preserves the dynamic initial-state digest. Assert `Simulation::from_scenario` starts tick 0 with no fleets, hazard, queue, or remainders and preserves default digest `0x67D9_6E98_3D6C_9330`.

Run `cargo test --test scenario`.

Expected RED: scenario types and constructors do not exist.

- [ ] **Step 2: GREEN — owned names, draft validation, fingerprint, and codec**

Replace `World.name: &'static str` with bounded `WorldName(Box<str>)`. Implement `ScenarioDraft`, `WorldDraft`, private `ScenarioV1`, issues with error/warning severity, and the exact domain-separated FNV-1a byte stream from the design. Prove worst-case production, regeneration including `base_regeneration + 1000`, and hydrosphere speed including `base_speed * 14000 / 10000` fit their destination types.

Use serde only in the codec. Deny unknown fields, require exactly seven worlds, a 16-digit uppercase hex seed, canonical decimal strings for every `u64`, and pre-parse the 65,536-byte limit.

- [ ] **Step 3: GREEN — construct a new deterministic session**

Add crate-private `Campaign::from_scenario` and public `Simulation::from_scenario`, `scenario_fingerprint`, and `ReplayIdentity`. Keep `canonical_fingerprint` byte-for-byte unchanged. The editor/app must never call the controlled-fixture `from_campaign` seam.

- [ ] **Step 4: RED/GREEN — persistence semantics**

Add a memory store test double and native store tests proving round-trip, failed-write preservation, corrupt/oversized-load preservation, load-without-apply, empty-slot save, and occupied-slot overwrite confirmation/cancellation. Native paths follow macOS Application Support, Windows APPDATA, and Linux XDG/HOME conventions and save through a same-directory temporary file, flush, sync, and atomic persist. Task 7 defines the platform-neutral store trait only; Task 10 owns the concrete browser localStorage adapter after its exact wasm dependencies exist.

- [ ] **Step 5: RED/GREEN — editor widgets, focus, scrolling, and input**

Tests must cover 1440x900 wide layout, usable 960x600 compact layout, smaller non-inverted rectangles, 44x44 minimum targets, culling and scroll clamps, focus reveal, deterministic Tab order, keyboard/pointer intent parity, text/IME buffers, invalid numeric retention, confirmation state, and disabled Apply on errors.

Implement one ordered `EditorWidget` list as the source for drawing, hit testing, focus, labels/values, enabled/error state. Gameplay commands are impossible in editor mode. REVERT, FACTORY DEFAULTS, REGENERATE FROM SEED, LOAD, SAVE, CANCEL, and APPLY AND RESTART operate only on detached state until the app performs an atomic swap. SAVE requires confirmation only when it would replace an occupied slot; cancelling preserves both the prior payload and draft.

- [ ] **Step 6: Verify and commit**

```bash
cargo fmt --all --check
cargo test --test scenario --test scenario_editor
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
git add Cargo.toml Cargo.lock src/lib.rs src/game src/engine/input.rs src/scenario src/editor.rs src/platform tests/scenario.rs tests/scenario_editor.rs
git commit -m "feat(editor): add validated scenario authoring"
```

### Task 8: CPU and WebGPU Neural Advisory

**Files:**
- Create: `assets/shaders/advisory.wgsl`
- Create: `src/advisory/mod.rs`, `src/advisory/gpu.rs`
- Modify: `src/lib.rs`, `tests/shaders.rs`
- Create: `tests/advisory.rs`, `tests/advisory_gpu.rs`

**Interfaces:**
- Consumes: immutable campaign snapshots plus an optional shared wgpu device/queue.
- Produces: read-only seven-score snapshots with source metadata and backend status; no command-producing API.

- [ ] **Step 1: RED/GREEN — transparent CPU oracle**

Test all 12 normalized features, hazard adjustment, inbound aggregation, no-Union distance fallback, exact 57-weight packing, finite clamping, deterministic priority tie-break, and the factory-default seven `f32::to_bits()` values derived from the specified constants. Implement the fixed 12-to-4-to-1 MLP without an ML framework.

- [ ] **Step 2: RED/GREEN — controller scheduling and authority**

Test initialization/restart/material-event/30-tick scheduling, one in-flight request, newest-dirty coalescing, stale epoch/request/model/tick/fingerprint drops, parity tolerance, CPU fallback, and re-enable only on a new device epoch. A compile-time/source review must show advisory modules neither import nor return `GameCommand`.

- [ ] **Step 3: RED/GREEN — Naga-validated compute shader**

Add the exact 84-float features, 57-float weights, seven-float scores layout; `@workgroup_size(8)`; one guarded dispatch. Extend shader tests to validate both WGSL assets.

- [ ] **Step 4: Implement nonblocking wgpu compute**

Gate on `DownlevelFlags::COMPUTE_SHADERS`; otherwise remain CPU. Use explicit minimum binding sizes and usages, one readback in flight, `map_buffer_on_submit`, native `PollType::Poll`, browser auto-poll, and never `Wait` in the frame loop. Any map/device/nonfinite/range/parity failure disables GPU advisory for the epoch and publishes CPU fallback.

- [ ] **Step 5: Verify and commit**

```bash
cargo fmt --all --check
cargo test --test advisory --test shaders
cargo test --test advisory_gpu -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
git add assets/shaders/advisory.wgsl src/lib.rs src/advisory tests/shaders.rs tests/advisory.rs tests/advisory_gpu.rs
git commit -m "feat(advisory): add verified WebGPU neural scoring"
```

If no adapter is available, the adapter-backed test must report a skip rather than manufacturing GPU evidence; CPU and controller tests still pass.

### Task 9: Shared Application and Native Game

**Files:**
- Create: `src/app.rs`, `src/main.rs`
- Modify: `Cargo.toml`, `src/lib.rs`, `src/game/view.rs`, `src/editor.rs`, `src/platform/native.rs`, `src/engine/input.rs`
- Create: `tests/app.rs`

**Interfaces:**
- Consumes: renderer, fixed clock, simulation, editor, store, and advisory controller.
- Produces: one `ApplicationHandler<AppEvent>` with complete play/editor controls and native executable.

- [ ] **Step 1: RED — platform-neutral app transitions**

Test pointer/keyboard command mapping, pause clock clearing, editor open/cancel preservation, atomic apply/restart, active-scenario `R` restart, no editor-mode game commands, accepted-command advisory dirtiness, sequence reset, and surface-state decisions.

- [ ] **Step 2: GREEN — shared state machine**

Own `active_scenario`, `Simulation`, `AppMode`, `ScenarioEditor`, store, view/input/clock, command sequence, advisory, window/GPU/renderer, and fatal/recoverable messages. Fixed steps run only in Playing and unpaused. Opening, cancelling, applying, restarting, suspend, and resume clear the clock as specified.

- [ ] **Step 3: GREEN — complete input and visible UI**

Map mouse/touch, wheel, keyboard, key text, and IME commits without leaking winit types into domain modules. Render full command-table play view, help, phase overlays, SCENARIO button, complete editor, validation/store messages, and NEURAL ADVISORY / ADVISORY ONLY with backend/updating/source tick. Do not claim screen-reader semantics.

- [ ] **Step 4: GREEN — native lifecycle and recovery**

Create windows, instances, and surfaces only on the winit thread. `src/main.rs` hosts a native multi-thread Tokio runtime; `resumed` sends prepared adapter/device acquisition to a worker and returns the result through a standard-library mailbox plus generation-only `AppEvent`. Validate the generation, configure the latest physical size, and create renderer resources back on the winit thread. Cancel/invalidate pending work on suspension, recover every wgpu 30 surface state, rebuild format-bound resources on change, and keep logical/physical coordinate conversion explicit. Native startup errors return to `src/main.rs` and print once.

- [ ] **Step 5: Verify, bounded-smoke, and commit**

```bash
cargo fmt --all --check
cargo test --test app
cargo test --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
git add Cargo.toml Cargo.lock src/app.rs src/main.rs src/lib.rs src/game/view.rs src/editor.rs src/platform/native.rs src/engine/input.rs tests/app.rs
git commit -m "feat(app): ship the native strategy game"
```

Run the release executable in a PTY until adapter, surface, and first-frame logs appear, then interrupt. This proves startup/render only, not a complete campaign.

### Task 10: WASM Browser WebGPU Runtime

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`, `.gitignore`, `src/app.rs`, `src/lib.rs`
- Create: `src/platform/web.rs`, `tools/build-web.sh`, `web/index.html`
- Create/modify: browser-focused tests where practical

**Interfaces:**
- Consumes: the shared app and async `GpuContext`.
- Produces: `rlib + cdylib`, winit-owned focused canvas, nonblocking browser GPU startup, localStorage, and reproducible `dist/` bundle.

- [ ] **Step 1: RED — wasm compile and browser shell contract**

Add cfg tests/source assertions for the marker-only event, localStorage key, exact canvas attributes, and explicit browser backend. Run the pinned target check; expected RED is missing wasm entry/dependencies before implementation.

- [ ] **Step 2: GREEN — exact target-scoped dependencies and entry**

Make the library `crate-type = ["rlib", "cdylib"]`. Keep env_logger, tempfile, and Tokio native-only; keep pollster development-only for the adapter-backed test. Pin wasm-bindgen 0.2.127, wasm-bindgen-futures 0.4.77, console_error_panic_hook 0.1.7, console_log 1.1.0, and web-sys 0.3.104 with Storage/Window only. Use wasm-only web-time 1.1.0 for frame timing because this target's `std::time::Instant::now` panics at runtime.

Export a wasm startup function that initializes browser logging/panic reporting, builds `EventLoop<AppEvent>` and the App/mailbox, and calls `spawn_app`. `ApplicationHandler::resumed` receives the `ActiveEventLoop`, creates the winit Window/canvas with focusable/appended/prevent-default attributes, and launches GPU initialization. Use `spawn_local`, `Rc<RefCell<Option<Result<GpuContext, EngineError>>>>`, a generation token, and only `GpuInitFinished` through the proxy. Never send `GpuContext` or block.

- [ ] **Step 3: GREEN — browser storage and bundle**

Create `src/platform/web.rs` and implement recoverable localStorage load/save with the exact documented key. Commit a static module loader in `web/index.html`. `tools/build-web.sh` idempotently installs the `wasm32-unknown-unknown` target for `nightly-2026-09-01`, installs/uses the matching wasm-bindgen CLI under ignored `target/tools`, builds the release cdylib, and emits ignored `dist/intergalactic_warfare.js` plus `_bg.wasm`.

- [ ] **Step 4: Verify real browser runtime**

```bash
cargo check --target wasm32-unknown-unknown --lib
./tools/build-web.sh
```

Serve the repository on localhost and inspect a real WebGPU-capable browser. Verify canvas creation/focus, first rendered frame, play input, F4 editor, one name edit, SAVE, reload, LOAD, and localStorage recovery. Console must have no panic, WGSL, uncaught promise, or WebGPU validation error. Record compile, bundle, and live-browser evidence separately.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock .gitignore src/app.rs src/lib.rs src/platform/web.rs tools/build-web.sh web/index.html
git commit -m "feat(web): add the WebGPU browser runtime"
```

Generated `dist/` and `target/tools/` remain untracked.

### Task 11: Documentation, CI, and Reusable Engine Skill

**Files:**
- Create: `README.md`, `.github/workflows/ci.yml`
- Modify: project docs only if implementation differs from the accepted contract
- External create/modify: `/Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine/SKILL.md`, preserved `agents/openai.yaml`, and `/Users/donaldfilimon/.grok/sync-targets.json`

- [ ] **Step 1: Document the delivered product honestly**

Document native and browser build/run, all play/editor controls, JSON/persistence paths, deterministic boundaries, advisory-only status/fallback, keyboard-accessible canvas limitation, troubleshooting, and exact verification commands. Do not turn compilation into a runtime or cross-platform acceptance claim.

- [ ] **Step 2: Add provider CI definition**

Add macOS/Windows/Linux native format/clippy/test/build jobs using the dated toolchain, plus a wasm target check and pinned bundle job. CI configuration is evidence of intended coverage; provider success is claimed only after a provider runs it.

- [ ] **Step 3: Complete and validate the central skill**

Replace the scaffold with self-contained guidance covering deterministic authority, exact version/API research, Naga-before-wgpu, logical versus physical coordinates, safe `Arc<Window>` surfaces, wgpu 30 acquisition/presentation, browser `spawn_local` marker mailbox, advisory CPU-oracle/parity/fallback, scenario validation, TDD, and evidence separation. Preserve `agents/openai.yaml` exactly.

```bash
PYTHONDONTWRITEBYTECODE=1 /Users/donaldfilimon/.local/pipx/venvs/pip/bin/python \
  /Users/donaldfilimon/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  /Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine
```

- [ ] **Step 4: Classify without disturbing unrelated central work**

Add exactly `rust-webgpu-game-engine` to `catalog.portableSkills` in `/Users/donaldfilimon/.grok/sync-targets.json`. The manifest is outside the central skill Git repository, so commit only the new skill directory from `/Users/donaldfilimon/.grok/skills`. Preserve the four pre-existing unrelated deletions and skip central sync because its complete-directory validation would fail.

- [ ] **Step 5: Commit each ownership boundary**

```bash
git add README.md .github/workflows/ci.yml docs
git commit -m "docs(game): add cross-platform play and verification"
```

In the central skill repository, stage only `rust-webgpu-game-engine/` and commit `feat(skills): add Rust WebGPU engine guidance`. Report the external manifest as validated filesystem state without Git provenance.

### Task 12: Full Verification, Runtime Acceptance, and Final Review

**Files:**
- Modify only files implicated by observed failures.
- Review: every project file plus the new central skill and external manifest entry.

- [ ] **Step 1: Resolve deferred deterministic boundary review**

Test and, if necessary, fix `run_scripted_union_commander` so a terminal state reached exactly on inclusive tick 54,000 is observed. Preserve the calibrated tick-41,601 default sentinel and canonical digest.

- [ ] **Step 2: Run complete automated gates**

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
cargo check --target wasm32-unknown-unknown --lib
./tools/build-web.sh
PYTHONDONTWRITEBYTECODE=1 /Users/donaldfilimon/.local/pipx/venvs/pip/bin/python \
  /Users/donaldfilimon/.codex/skills/.system/skill-creator/scripts/quick_validate.py \
  /Users/donaldfilimon/.grok/skills/rust-webgpu-game-engine
```

- [ ] **Step 3: Native macOS acceptance**

Perform a bounded release startup and then manual interaction at 1440x900 and 960x600: select, launch, tune all fields, pause/resume, help, active-scenario restart, open/editor keyboard navigation, invalid value/error, cancel preservation, save/load, apply/restart, advisory label/backend, and resize. Capture but do not commit screenshots. Record adapter/backend, first frame, and any GPU parity result separately.

- [ ] **Step 4: Browser acceptance**

Serve the release bundle locally in a real WebGPU browser. Repeat first-frame/input/editor/save-reload-load/apply/advisory checks, inspect console, and verify explicit WebGPU behavior. A successful wasm compile or generated bundle alone is not browser-runtime proof.

- [ ] **Step 5: Fresh whole-tree review and correction loop**

```bash
git status --short --branch
git log --oneline --decorate -20
git diff 46a3884..HEAD
rg -n "TODO|FIXME|unimplemented!\(|todo!\(" src assets tests README.md web tools
```

Review authority boundaries, unsafe prohibition, vertex/shader ABI, logical/physical coordinates, surface recovery, scenario arithmetic and atomicity, JSON precision, editor mode gating, advisory command isolation, async wasm ownership, cross-platform cfgs, and documentation claims. Fix every important issue, rerun the complete gate, and commit corrections with a subject derived from the actual diff. Create no empty verification commit.

## Expanded Plan Self-Review Record

- Spec coverage: Tasks 6 through 12 cover portable rendering, full scenario editing, CPU/WebGPU advisory, shared native app, real browser runtime, CI/docs/skill, and distinct automated/runtime/manual evidence.
- Determinism: the reviewed RulesV1 order, generator, base speed 23, default digest, and canonical state byte stream remain unchanged.
- Authority: only validated commands enter `Simulation`; neither editor widgets nor advisory/GPU results can control gameplay.
- Portability: native-primary and browser-WebGPU backend selection, logical versus physical coordinates, cfg-scoped dependencies, async browser initialization, and target-specific persistence are explicit.
- Accessibility: keyboard operation and 44x44 targets are in scope; screen-reader semantics are not claimed.
- Verification honesty: unit tests, target compilation, release build, bundle generation, provider CI, live native GPU, live browser, GPU parity, and manual interaction are independent evidence layers.

---

## Tactical 3D Executable Continuation

The accepted design amendment above is implemented by Tasks 13 through 18.
These tasks begin only after Tasks 9 through 12 are green and committed. They
do not reopen RulesV1, campaign serialization, scenario JSON, `GameCommand`, the
36-byte primitive ABI, or advisory authority.

### Task 13: Accept the Tactical 3D Command-Deck Amendment

**Files:**
- Modify: `docs/superpowers/specs/2026-09-02-intergalactic-warfare-design.md`
- Modify: `docs/superpowers/plans/2026-09-02-intergalactic-warfare.md`
- Modify: `.superpowers/sdd/2026-09-02-intergalactic-warfare/progress.md`
- Create: `.superpowers/sdd/2026-09-02-intergalactic-warfare/task-13-brief.md`

**Interfaces:**
- Consumes: the committed cross-platform baseline and frozen contracts.
- Produces: one explicit amendment for 3D presentation, assets, usability, and
  evidence without changing campaign authority.

- [ ] Record the three superseded presentation constraints and every retained
  product/authority exclusion.
- [ ] Freeze the holographic visual language, four render stages, quality paths,
  camera limits, touch confirmation, preference record, asset versions, and
  acceptance evidence boundaries.
- [ ] Review the amendment against current source APIs and commit documentation
  only as `docs: accept the tactical 3D command deck`.

### Task 14: Camera, Scene Extraction, Picking, and Command Preview

**Files:**
- Create: `src/presentation/mod.rs`, `src/presentation/camera.rs`
- Create: `src/presentation/scene.rs`, `src/presentation/picking.rs`
- Modify: `src/game/simulation.rs`, `src/lib.rs`, `src/app/core.rs`
- Create: `tests/presentation.rs`
- Modify: `tests/campaign.rs`, `tests/app.rs`

**Interfaces:**
- Consumes: immutable `Campaign`, `ScenarioFingerprint`, interpolation alpha,
  `ViewState`, advisory presentation state, visual time, and preferences.
- Produces: `CameraState`, `CameraController`, `CameraMatrices`, `SceneRay`,
  `WorldHit`, `SceneFrame`, and pure `CommandPreview`.

- [ ] Add RED tests for finite matrices at all limits, camera clamps/reset,
  logical ray conversion, world-center picks, overlapping tie-breaks, gesture
  thresholds, HUD precedence, exactly seven extracted worlds, bounded effects,
  and presentation fingerprint isolation.
- [ ] Map sector `x/y` to centered presentation `x/z` and derive a domain-tagged
  visual seed without consuming Generator V1.
- [ ] Implement immutable scene extraction using canonical fleet progress ratios
  only; route curvature and interpolation never feed simulation.
- [ ] Refactor command validation so `preview_command` and execution return the
  same typed rejection for launch and tuning cases with no preview mutation.
- [ ] Integrate camera session state, orbit/zoom/reset, logical resize, and
  HUD-before-ray selection into `AppCore`/platform routing.
- [ ] Run focused presentation/campaign/app tests, the full CPU gate, digest and
  balance sentinels, review the diff, and commit
  `feat(presentation): add tactical camera and scene extraction`.

### Task 15: Tactical 3D Renderer and Capability Fallback

**Files:**
- Create: `src/engine/scene_renderer.rs`, `src/engine/render_frame.rs`
- Create: `src/engine/resources.rs`, `src/engine/quality.rs`
- Create: `assets/shaders/space.wgsl`, `assets/shaders/worlds.wgsl`
- Create: `assets/shaders/routes.wgsl`, `assets/shaders/postprocess.wgsl`
- Modify: `src/engine/render.rs`, `src/engine/shader.rs`, `src/engine/mod.rs`
- Modify: `src/app.rs`, `src/engine/gpu.rs`, `tests/shaders.rs`
- Create: `tests/renderer_contracts.rs`

**Interfaces:**
- Consumes: immutable `SceneFrame`, physical target, logical viewport, and
  graphics preference.
- Produces: `RenderFrame`, procedural background, depth-tested instanced worlds,
  transparent tactical geometry, optional bloom, and overlay composition.

- [ ] Add separate size-asserted mesh/world/route layouts and generate the fixed
  24x48 sphere once per device epoch. Preserve `Vertex` byte-for-byte.
- [ ] Naga-validate every labeled WGSL asset before pipeline creation and assert
  host/WGSL locations, strides, bindings, and texture layouts.
- [ ] Implement four logical stages, depth, transparent ordering, camera-facing
  fleets, procedural surface/atmosphere/hazard cues, halos, particles, and
  restrained half-resolution bloom.
- [ ] Implement High (4x MSAA/depth/bloom), Low (1x/no bloom/reduced effects),
  and Auto resource selection. Optional failure falls back monotonically;
  shader failure is fatal with label/diagnostic.
- [ ] Replace the renderer call site with `RenderFrame`. Zero extents allocate no
  surface targets; resize, lost surface, suspension, and new device epoch rebuild
  scene, depth, MSAA, postprocess, primitive, and advisory resources.
- [ ] Run shader/renderer/full gates plus bounded native first-frame and commit
  `feat(renderer): render the tactical 3D command deck`.

### Task 16: Licensed SDF Typography and Icon Atlas

**Files:**
- Create: `assets/ui/PROVENANCE.md`, `assets/ui/hashes.json`
- Create: `assets/ui/licenses/Inter-OFL.txt`, `assets/ui/licenses/Lucide-ISC.txt`
- Create: `assets/ui/fonts/Inter-Regular.ttf`, `assets/ui/fonts/Inter-SemiBold.ttf`
- Create: selected `assets/ui/icons/*.svg`
- Create: `tools/build-ui-atlas.rs` (or an equivalent pinned repository script)
- Create: `assets/ui/atlas.r8`, `assets/ui/atlas-metrics.json`
- Create: `src/engine/ui.rs`, `assets/shaders/ui_sdf.wgsl`
- Modify: `src/engine/render_frame.rs`, `src/engine/scene_renderer.rs`
- Create: `tests/ui_assets.rs`

**Interfaces:**
- Consumes: Inter 4.1 Regular/SemiBold, Lucide 1.27.0 commit `4aec3f8`,
  printable ASCII, and the frozen icon allowlist.
- Produces: embedded 1024x1024 R8 SDF bytes/metrics and bounded `UiBatch` glyph
  instances; the 5x7 primitive font remains fallback.

- [ ] Fetch only official pinned sources, record archive/file/output SHA-256,
  retain exact licenses/provenance, and commit only the selected icon subset.
- [ ] Build the atlas reproducibly with stable ordering and fixed parameters.
  Normal builds embed outputs and perform no asset filesystem/network access.
- [ ] Validate dimensions, glyph bounds, unique mappings, required coverage,
  metrics parsing, hashes, shader contract, and committed UI string coverage.
- [ ] Render SDF UI before the primitive diagnostic overlay, run native/wasm
  gates and visual text checks, then commit
  `feat(ui): add licensed SDF typography and icons`.

### Task 17: Offline Usability, Preferences, Touch, and Speed

**Files:**
- Create: `src/preferences/mod.rs`, `src/preferences/store.rs`
- Create: `src/app/onboarding.rs`, `src/app/settings.rs`
- Create: `src/presentation/ui.rs`, `src/presentation/interaction.rs`
- Modify: `src/app/core.rs`, `src/app.rs`, `src/engine/input.rs`
- Modify: `src/platform/native.rs`, `src/platform/web.rs`, `src/game/view.rs`
- Modify: `README.md`
- Create: `tests/preferences.rs`, `tests/usability.rs`

**Interfaces:**
- Consumes: `CommandPreview`, presentation camera/scene, scenario-independent
  `PreferencesStore`, and physical input translated at the platform boundary.
- Produces: onboarding, tooltips, command tray, explicit touch launch,
  settings/focus, `GameSpeed`, `GraphicsQuality`, `MotionPreference`, and
  `UserPreferencesV1`.

- [ ] Replace `paused: bool` with 0x/1x/2x/4x `GameSpeed`, preserving
  `paused()`. Clear residue on every speed change and prove wall-time scheduling
  equals direct canonical stepping.
- [ ] Add onboarding that pauses without catch-up, advances only from observed
  session actions, supports skip/reset/help restart, and persists completion
  outside scenario/campaign truth.
- [ ] Add HUD-first tooltips and command tray preview. Only explicit LAUNCH,
  right-click, or Space may enqueue the unchanged `GameCommand`.
- [ ] Add camera reset, settings, speed-down/up, and contextual launch actions.
  Add empty-space/middle-button orbit, wheel/pinch zoom, tap-select/preview/
  confirm, gesture thresholds, and touch tracking without accidental drag launch.
- [ ] Add settings for 85/100/115/130 UI scale, reduced motion, high contrast,
  Auto/Low/High graphics, and onboarding reset. All controls share one action
  path, visible focus, and minimum 44x44 targets.
- [ ] Implement separate 16 KiB `PreferencesStore` records at native
  `preferences-v1.json` beside the scenario and browser key
  `intergalactic-warfare.preferences.v1`. Every read/write/version/quota failure
  returns defaults plus one recoverable message and cannot affect scenario I/O.
- [ ] Test compact/wide layout, high contrast patterns, reduced-motion static
  cues, onboarding, malformed/oversized/denied stores, touch gestures, and
  editor/renderer isolation. Run full gates and commit
  `feat(usability): add offline command-deck guidance and settings`.

### Task 18: Runtime, Visual, Performance, and Whole-Tree Closeout

**Files:**
- Modify only files implicated by observed failures.
- Review: every project file, generated asset/provenance record, central skill,
  and separate scenario/preference storage boundary.

- [ ] Run the full automated gate exactly:

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo build --release
cargo test --test advisory_gpu -- --nocapture
cargo check --target wasm32-unknown-unknown --lib
./tools/build-web.sh
git diff --check
```

- [ ] On native macOS, exercise first frame, all commands, 3D select/launch,
  orbit/zoom/reset, touch-equivalent flow, tuning, 0x/1x/2x/4x, onboarding,
  preferences, editor save/load/apply, advisory, resize, recovery where feasible,
  and terminal overlays at 1440x900 and 960x600.
- [ ] On localhost BrowserWebGpu, repeat relevant focus/input/3D/editor/storage/
  preference/advisory/resize/reload flows and inspect console for panics,
  promise, WGSL, and WebGPU validation failures.
- [ ] Capture uncommitted screenshots for default, compact, high contrast,
  reduced motion, active hazard, fleet-heavy, editor wide/compact, and terminal.
  Repair hierarchy, contrast, clipping, depth, focus, faction distinction,
  bloom, and command-readability failures.
- [ ] On the recorded adapter, warm up and measure a 60-second fleet-heavy run
  at both accepted sizes. Report p95 frame time and any Auto downgrade. Record
  browser measurements separately and make no universal 60 fps claim.
- [ ] Run a fresh strict whole-tree review. Repair every Critical or Important
  finding and rerun every affected evidence layer. Keep provider CI, untested
  OS runtime, and inaccessible recovery paths as explicit non-claims.
- [ ] Commit only actual repairs/evidence documentation, if any, as
  `fix: close tactical command-deck acceptance findings`. Do not create an empty
  commit, push, deploy, publish, or claim provider results.

## Tactical Continuation Self-Review Record

- Determinism: all new mutable values are session/presentation state; no new
  value enters `Campaign`, RulesV1, generator state, scenario wire data, or the
  canonical digest.
- Authority: command preview shares validation but cannot enqueue; advisory and
  editor preview retain no command-producing authority.
- ABI: original primitive `Vertex` remains 36 bytes and new 3D/SDF ABIs are
  separate and size-asserted.
- Portability: native and browser use one scene/UI contract with explicit
  physical target resources and logical interaction coordinates.
- Assets: runtime visuals are procedural except the pinned licensed Inter/Lucide
  atlas embedded at build time.
- Usability: onboarding, touch confirmation, focus, UI scaling, high contrast,
  reduced motion, settings, and speed are offline and separately persisted.
- Evidence: CPU tests, native build/runtime, wasm compile/bundle, browser runtime,
  provider CI, GPU parity, performance, and visual acceptance stay distinct.
