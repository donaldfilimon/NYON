# NYON Rust Tactical Ground Combat Design

Status: Proposed Rust recovery design. The user approved the overall recovery sequence; this artifact does not claim subsystem/runtime acceptance.

**Goal:** Implement deterministic paid troops, cube-seam movement, simultaneous combat, retreat and occupation integrated with cities.

**Architecture:** Run four internal tactical rounds inside the common authoritative hour. Freeze movement/fire snapshots, apply summed damage, and publish city/combat effects with the same global descriptor. Exact cube graph distance and transported cardinal rays serve different rules.

## Global constraints

- Rust stable 1.99.1, edition 2024; preserve `forbid(unsafe_code)` and pure-core dependencies.
- Existing source, dirty work, save identities, independent fixtures and historical evidence remain intact.
- Compilation only while Donald's test/CI pause applies. Test scenarios below are future acceptance, not executed or green.
- No commits, pushes, deployment, new production dependencies, automatic profile migration or Zig implementation work.
- Current evidence is in `docs/qualification/2026-10-19-rust-stable-compilation.md`; source revisions are pinned in `docs/qualification/2026-10-19-rust-recovery-inputs.md`.


## Rules and interfaces

Carry the source tactical class table, capacities, commands and wire tags from the pinned tactical and city annex revisions. Family1 commands and kinds7..9/events64..66 belong to the single city-combat identity. No independent tactical timer, platform physics, random initiative or player-provided hostility override.

Movement is single-occupancy; an initially occupied cell is unavailable even if its occupant plans to move. Check occupancy before credit, resolve contention by the fixed ID ordering, and expose congestion. Fire uses transported cardinal rays and frozen attack eligibility; summed damage permits simultaneous mutual destruction. Source phase4/8/12 precedence and zero/default terminal fields are normative author responses awaiting merged review. Phase12 threshold4 stages success0; city35 is the sole final capture/block emitter at13.

Recruitment is paid through city grants; crew/roster/cargo conservation is coupled. Effect/result/grant schemas and emitters must agree with the corrected city composition before code. Road targets are forbidden; solar remains passive under occupation. No local Tactical result independently mutates city ownership.

## Feasibility gate

CCM06 concrete native/wasm layout, largest encoding and peak memory remain open. CCM07 exact-distance proof and maximum-load complete-hour timing remain open. The historical cached query timing is not full-tick proof. Retain the maximum-capacity fixture and p95 target; exceeding it requires an equivalent algorithm revision or explicit versioned scope decision, never silently lower caps.

## Acceptance

Future tests pin numeric class tables, cube seams/corners, cardinal ray transport/blockers, occupied-before-credit rejection, contested moves, simultaneous fire/mutual destruction, cooldown/ammo/crew conservation, retreat reserves, occupation threshold and capture interrupted by hall damage. Replay and new-version save/reopen must reproduce the coupled city/combat result. Player previews call the same pure formulas.

## Self-review

Checked scope, contract precedence, preserved identities, paused-verification policy and known source-review gaps. Referenced historical numerical contracts remain bound by the input hash manifest, not silently accepted through their old approval labels. Product implementation and runtime acceptance statuses must remain separate.
