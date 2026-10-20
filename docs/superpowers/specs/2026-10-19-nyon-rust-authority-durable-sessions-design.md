# NYON Rust Full-scale Authority and Durable Sessions Design

Status: Proposed Rust recovery design. The user approved the overall recovery sequence; this artifact does not claim subsystem/runtime acceptance.

**Goal:** Define one Rust authority/session publication domain with bounded work and durable history, preserving old profiles.

**Architecture:** Pure authority and pure session graph share value contracts through an integration bridge; neither imports a platform store. One selected pair publishes the authority root and graph prefix together. Native and browser brokers own I/O and report values tied to owner/generation/request tickets.

## Global constraints

- Rust stable 1.99.1, edition 2024; preserve `forbid(unsafe_code)` and pure-core dependencies.
- Existing source, dirty work, save identities, independent fixtures and historical evidence remain intact.
- Compilation only while Donald's test/CI pause applies. Test scenarios below are future acceptance, not executed or green.
- No commits, pushes, deployment, new production dependencies, automatic profile migration or Zig implementation work.
- Current evidence is in `docs/qualification/2026-10-19-rust-stable-compilation.md`; source revisions are pinned in `docs/qualification/2026-10-19-rust-recovery-inputs.md`.


## Inputs and reconciliation

Read full-scale authority, initial economy codec, expanded durable sessions, durable bridge and Task0 reconciliation addendum from the pinned input manifest. Adopt the addendum's pending-pin states, bounded broker inventory, cache-only windows and purpose-specific cold replay into one Rust contract. The historical fix1 review explicitly says external adoption is missing: this design specifies the intended reconciliation but does not mark Task0 accepted.

## Publication and ownership

`Owner`, `Ticket`, `Prefix`, `PairKey`, `AuthorityRef`, `GraphRef`, `Range`, `Purpose` are copied qualified values. One resident domain and one detached replay domain each own two pair slots; no third authority. Use 64 public pair leases, eight role-limited history windows per authority, one global source-read lane, one shared insertion lane and at most two I/O permits. Share the outer 0..1024 allowance across domains, hashing, ownership and terminal delivery.

A privately prepared seal contains already validated/refcounted authority and graph state. The final selector switch does no allocation, hashing, I/O or fallible work. Durable CAS follows resident publication; a conflict leaves the resident pair committed and dirty. Invalid foreign offers cannot abort another job.

## History and replay

Stream logical transactions up to32MiB through1MiB segments. Retain exact authenticated prefix ownership beyond root retirement. Cache backpressure is explicit. Read-only existing-node replay at graph capacity needs no insertion reservation. Permit genesis parent absence for the explicit cold tick-zero replay case as well as initialization. Every pending request ends in an explicit accepted/rejected/released disposition; late replies cannot keep authority pointers alive.

## Proof before integration

Rust owner/control/page/index/cursor types must have concrete native/wasm size/alignment/peak witnesses. Do not reuse Zig requested-byte totals as Rust evidence. Exact codec, journal EOF, node reuse, crash/CAS recovery and resource bounds precede product adoption. Full-step timing is separate from per-poll latency.

## Self-review

Checked scope, contract precedence, preserved identities, paused-verification policy and known source-review gaps. Referenced historical numerical contracts remain bound by the input hash manifest, not silently accepted through their old approval labels. Product implementation and runtime acceptance statuses must remain separate.
