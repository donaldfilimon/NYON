# NYON Rust Implemented Feature Parity Design

Status: Proposed Rust recovery design. The user approved the overall recovery sequence; this artifact does not claim subsystem/runtime acceptance.

**Goal:** Recover the implemented experimental capabilities in Rust while keeping the retained game usable and its existing identities unchanged.

**Architecture:** Use `nyon_workshop_core::recovery` as an isolated versioned API. Port semantics and canonical fields, not Zig memory representations. Reuse `src/game/simulation.rs` for existing integer navigation; no second Classic authority. Core exposes copied projections and typed commands; runtime owns policy and installation.

## Global constraints

- Rust stable 1.99.1, edition 2024; preserve `forbid(unsafe_code)` and pure-core dependencies.
- Existing source, dirty work, save identities, independent fixtures and historical evidence remain intact.
- Compilation only while Donald's test/CI pause applies. Test scenarios below are future acceptance, not executed or green.
- No commits, pushes, deployment, new production dependencies, automatic profile migration or Zig implementation work.
- Current evidence is in `docs/qualification/2026-10-19-rust-stable-compilation.md`; source revisions are pinned in `docs/qualification/2026-10-19-rust-recovery-inputs.md`.


## Contract and source precedence

The source inventory covers all 93 production Zig files, including untracked implementations. For each row compare concrete fields, phase order, rejection precedence, capacities and wire bytes. Missing features remain Pending; source presence is not acceptance. Pure-core ports retain their original rules-domain literals where they reproduce those bytes; no implicit conversion of existing identities.

## Port boundaries

- `recovery/roles.rs`: private `SessionPolicy`, checked constructor, typed civilization, provenance and exact admission error precedence. Autonomous commands are rejected by the public admission path.
- `recovery/surface.rs`: face order +X,-X,+Y,-Y,+Z,-Z, u16 side, u64 IDs, integer basis transformations, exact seams and corners. Zero side is invalid; 65535 is representable without intermediate overflow.
- `recovery/planet_economy/`: capacities 64 settlements/256 buildings/64 shipments/4096 history receipts, side4..32; two commodities, farm/quarry/house, explicit bypass, paid rules and COW transaction lifecycle. Port the seven command families, not a rich-city approximation.
- `recovery/coordinated_galaxy/`: qualified IDs, routing, one command order/budget, leased views, canonical histories and one failure-free global publication. Preserve incomplete fault/replay obligations as explicit gaps until implemented.
- `recovery/seeded_galaxy/`: keyed generator, canonical NYSG bytes, owned setup/decode lane, pages, digest, token/cancel/release/take semantics and CLI. No economic endowments in manifests.
- `recovery/expanded_session/`: NYONCAT1 catalogs, NYONAL01 ledger, detached metadata proposals. Store publication/CAS is a separate integration step.

## Acceptance

Compile native/core/both wasm profiles; compile regression targets without executing them. Later run literal seam/role witnesses; independent wire/digest witnesses; zero/one/max budgets; canceled/stale/cross-owner tokens; overflow/OOM/capacity preservation; replay, global snapshots, save conflicts and executable CLI comparisons. Do not replace goldens with the Rust output.

## Self-review

Checked scope, contract precedence, preserved identities, paused-verification policy and known source-review gaps. Referenced historical numerical contracts remain bound by the input hash manifest, not silently accepted through their old approval labels. Product implementation and runtime acceptance statuses must remain separate.
