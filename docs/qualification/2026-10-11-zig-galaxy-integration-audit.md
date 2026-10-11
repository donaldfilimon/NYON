# Coordinated galaxy authority: integration audit

Date: 2026-10-11. Status: Current source inventory; architecture below Proposed.
Donald selected coordinating separate planet authorities at 06:45 EDT. Donald approved the staged-preparation/coordinator-snapshot design direction at 06:49 EDT. Written specification, numeric capacities and new gameplay still require their own review. Existing planet slice and full-game acceptance boundaries remain unchanged.

## Current integration barriers

- `zig/src/planet_economy/ids.zig:13`: planet 1 and civilization 7 are fixed. Config, command validation, invariant validation and cell storage reconstruction assume these identities. Separate instances presently collide.
- `zig/src/planet_economy/state.zig:21`: each planet reserves 8,642,560 bytes. Multiplication by a galaxy-sized planet count needs a new explicit aggregate memory contract; current memory evidence qualifies one island only.
- `zig/src/planet_economy/state.zig:108`: local metadata owns tick, revision, sequence and history. `job.zig:119` admits local revision and derives the next tick locally.
- `zig/src/planet_economy/job.zig:246`: polling automatically progresses from history/invariant validation into local publication. A later failure on another planet cannot undo an already-published island. Calling existing poll APIs sequentially is not atomic global execution.
- `zig/src/planet_economy/job.zig:166`: cancellation/failure is delivered before cleanup finishes. Coordinator ownership must persist until each lane drains.
- `zig/src/planet_economy/replay.zig:10`: replay authenticates one local genesis/transcript, not a coordinated global transaction.

## Recommended design within the selected approach

Separate planet allocations and local simulation remain independently owned. A single coordinator owns global time, strategic candidate state, routed commands, transaction identity and the committed read descriptor. Enrolled planets cannot independently advance or publish through a host.

A staged planet API prepares a candidate at an explicit global target tick and stops after history/invariant validation. Prepared lanes remain locked. Strategic state and all participants must prepare before publication. One caller work allowance pays routing, simulation, history, validation, installation, abort and retirement.

A preparation barrier alone is insufficient: per-planet physical swaps can still expose mixed ticks. Coordinator reads therefore resolve retained root handles through one committed global descriptor. Physical installation proceeds incrementally while that descriptor still exposes the previous complete galaxy. One descriptor swap publishes the complete next tick. Pre-publication cancellation aborts hidden installed candidates; post-publication cancellation reports already-published and drains retirement. Root retention and abort must be explicitly specified and independently tested.

Keep `nyon.planet-economy.v0` as its qualified island contract. Coordinated galaxy rules receive a new versioned identity; do not silently alter island vectors or read legacy user data.

## Capability preservation from existing sources

Classic `src/game/model.rs:282` has deterministic seven-world generation; its frozen draw order is a reference, not a general galaxy generator. Living `crates/nyon-workshop-core/src/living/model.rs:362` defines civilization policy/status; `rules.rs:1328` deterministic autonomous intent reservation. Production/construction and freight are operational at `rules.rs:3976` and `:4702`; preserve conservation, blocked-recipe atomicity and arrival dependencies while integrating detailed settlements.

Living diplomacy at `rules.rs:1115` uses common-snapshot bilateral proposals; relations/agreements/wars are at `model.rs:861`. Fleets at `model.rs:621` retain travel endpoints/durations; dispatch/travel rules at `rules.rs:3836` and `:5478`. Combat at `rules.rs:4227` resolves strategic hull/facility damage; occupation/claims at `:4408` and `:4464`. These do not implement tactical terrain-aware ground units.

Research/ecology/population are unfinished V3 proposals, not accepted new gameplay: `docs/superpowers/specs/2026-10-02-living-v3-rules-approval.md` remains Proposed. No Rust execution was used for this audit. Source existence is not a fresh test result.

## Decisions required in the written design

Freeze system/planet/civilization/entity/history capacities and aggregate requested memory; planet seed derivation and qualified references; local/global sequence and receipt ordering; global phase and same-tick visibility; all-planet versus deterministic inactive advancement; autonomous actor permissions; colonization, research, ecology, diplomacy, fleets/cargo and tactical integration; descriptor/root lifetime, stale completion and partial-install abort; global replay/canonical encoding and independent vectors.

Durable sessions and native/browser hosts remain required subsequent work. This audit proves no graphical runtime, persistence, accessibility or final Rust retirement. No product code, dependency, instruction file, user profile, commit or deployment changed.
