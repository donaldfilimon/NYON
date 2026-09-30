# NYON Galaxy Workshop V1 — qualification evidence record

Date: 2026-09-08
Plan: `docs/superpowers/plans/2026-09-02-nyon-galaxy-workshop-qualification.md`
Preparation and naming decisions: `docs/qualification/2026-09-08-task-1-preparation.md`

## Status conferred by this record

**ARTIFACT-QUALIFIED. Nothing stronger, and the gap is by design rather than by omission.**

This record discharges **Task 1 only**. It establishes source-gate and artifact
evidence for one candidate commit on one host. It is **not** runtime evidence, and
it confers no runtime, accessibility, recovery, offline, performance or
cross-platform status.

Stated plainly because the plan requires the ceiling be named rather than
discovered: **nine of the fourteen live-matrix rows need Windows 11 or Ubuntu
hosts that do not exist on this machine**, and the plan forbids substituting
compilation for live evidence. Runtime-qualified and release-qualified are
therefore **unreachable from this session by design**. Task 6 Step 4 explicitly
permits committing an artifact-qualified checkpoint with unavailable rows labelled
pending, which is what this is.

---

## Step 1 — Candidate provenance

| Field | Value |
|---|---|
| Commit | `60db100a53eb5b813a740bef47abd9c5eec57607` (`60db100`) |
| Branch | `main` |
| Working tree | **clean** — `git status --short --branch` reported no modified, staged or untracked paths |
| Whitespace | `git diff --check` exit **0** |
| Toolchain pin | `nightly-2026-09-01` (`rust-toolchain.toml`, components `clippy`, `rustfmt`, profile `minimal`) |
| rustc | `1.100.0-nightly (0dfb098f3 2026-08-31)`, commit-hash `0dfb098f3aeecbe38c2566ca090193280e7349e7` |
| Host | `aarch64-apple-darwin` |
| LLVM | `23.1.0` |

The plan asks that unrelated untracked paths be listed separately and excluded from
the release diff. **There were none**: the tree was clean at freeze time, which is
why this run waited for a quiet tree rather than proceeding earlier in the session.

## Step 2 — The five source gates

Every exit code was read **from the command itself**, written to its own log file,
never through a pipe or a trailing `echo`. That distinction is not pedantry here: a
backgrounded gate earlier this same session reported exit 0 while `clippy` had
exited 101, because the wrapper's status was read instead of the command's.

| Gate | Exit |
|---|---:|
| `cargo fmt --all --check` | **0** |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **0** |
| `cargo test --workspace --all-targets` | **0** |
| `cargo check -p nyon-workshop-core --target wasm32-unknown-unknown` | **0** |
| `git diff --check` | **0** |

**Workspace totals: 566 passed, 0 failed, 43 binaries.**

`--workspace` is load-bearing and not stylistic: `default-members = ["."]` scopes a
bare invocation to the root package and silently skips every `nyon-workshop-core`
suite while printing green.

## Step 3 — Named subsets, each run individually

Counts are from **isolated invocations**, one suite per `cargo test` call. An
earlier attempt to derive them by parsing the combined workspace log produced
misaligned pairings — it credited `workshop_library` with 23 tests when its own run
reports 7 — and was discarded rather than published. Per-suite counts from a
combined log are not trustworthy without care that this record does not claim.

The plan's Step 3 names a `source-contract` suite that does not exist. Per the
preparation document it expands to **three** source-*text* contracts, marked ※ below.
`renderer_contracts` is deliberately excluded from that slot: it is a *structural*
contract (`offset_of`/`size_of`, real frame building), a different kind of evidence.

| Suite | Exit | Tests |
|---|---:|---:|
| `rules_v1_facade` | 0 | 3 |
| `campaign` | 0 | 8 |
| `scenario` | 0 | 20 |
| `shaders` | 0 | 9 |
| `identity` ※ | 0 | 3 |
| `browser_contract` ※ | 0 | 3 |
| `workshop_store_web` ※ | 0 | 9 |
| `workshop_session` | 0 | 14 |
| `workshop_store` | 0 | 23 |
| `workshop_recovery` | 0 | 7 |
| `workshop_client` | 0 | 23 |
| `workshop_library` | 0 | 7 |
| `workshop_presentation` | 0 | 5 |
| `workshop_accessibility` | 0 | 11 |
| `workshop_web` | 0 | 11 |
| core `pack` | 0 | 9 |
| core `creator` | 0 | 11 |
| core `simulation` | 0 | 8 |
| core `two_system_forge` | 0 | 1 |
| core `archive` | 0 | 3 |
| core `history` | 0 | 6 |

### Scope correction — 100 of these tests are not Workshop V1

`cargo test --workspace` now sweeps in the Living Galaxy V2 authority island, which
is a **different program** with its own plans, spec and vector corpus. Measured by
direct invocation:

| Owner | Tests | Suites |
|---|---:|---:|
| Workspace total | 566 | 43 |
| — of which library unit tests (`--lib`) | 112 | — |
| — of which integration tests | 454 | 40 |
| **Living Galaxy V2** (`living_catalog` 24, `living_model` 51, `living_wire` 25) | **100** | 3 |
| Integration tests in the Workshop V1 candidate's scope | 354 | 37 |

**Quoting 566 as a Workshop V1 figure overstates it.** The 354 is itself the whole
product's integration surface — engine, UI, app, advisory and scenario suites
included — not a Workshop-only count.

## Step 4 — Artifacts

Run in the mandated order, with `check-workshop.sh` **after** both web builds. Its
freshness guard compares `dist/` mtimes against build inputs, so running it first
produces a failure that is not a code problem.

| Command | Exit |
|---|---:|
| `cargo build --release` | **0** |
| `./tools/build-web-webgpu.sh` | **0** |
| `./tools/build-web-webgl.sh` | **0** |
| `./tools/check-workshop.sh` | **0** |

A browser release is a **pair** of artifacts, not one. `web/loader.js` selects
between them before graphics initialization and falls back to the WebGL2 artifact
when the WebGPU preflight fails or when WebGPU initialization throws.

| Artifact | SHA-256 | Bytes |
|---|---|---:|
| `dist/webgpu/nyon.js` | `0bec7ddfb1f0ae8d8a7027219d04079fb3597b231cd4302c17ccdcb19f1469ea` | — |
| `dist/webgpu/nyon_bg.wasm` | `31ab2c5491af4873d82548eb567c9a0f15d9315cb1c37af863cf57a62b517ae8` | 5,516,964 |
| `dist/webgl/nyon.js` | `d7917ad93a7b4e27058464f7ae34f5fe0f5498eda6cfc2a61973e76366a9fa94` | — |
| `dist/webgl/nyon_bg.wasm` | `f393527590ecb574de7afbebf8b7859c67396763b90f286bbaa87e47e71d1e8e` | 7,436,699 |

**`dist/` is gitignored**, so these hashes cover untracked build outputs. They
identify the artifacts this run produced; they are not recoverable from the
repository and do not survive a rebuild unless the build is reproducible, which
this record does not claim to have tested.

---

## What this record does NOT establish

Stated rather than implied, because every one of these is a plausible misreading of
a page full of zeros.

- **Native startup.** Nothing here launched the binary.
- **Browser startup, rendering, input or storage.** A successful target compile and
  a generated bundle are not evidence that the page runs. Neither artifact above was
  loaded in any browser.
- **GPU parity.** `advisory_gpu` printing `SKIP:` is a pass on a machine without a
  usable adapter, not GPU coverage.
- **The IndexedDB adapter.** `src/workshop/store/web/wasm.rs` executes **nowhere in
  this repository** — there is no `wasm-bindgen-test` harness. It has compile and
  clippy evidence on the wasm target and nothing more. `workshop_store_web` passing
  is a source-text contract plus a host-side transaction model; it is not browser
  coverage, and its name invites exactly that misreading.
- **Manual visual acceptance, accessibility inspection, fault injection, offline
  behaviour, recorded p95 timing** — all Tasks 2 through 6, none run.
- **Two-System Forge steps 16–18 through the product.** Baseline Finding 2 is still
  open: `RequestExport`, `RequestImport` and `RequestLoad` are declared and
  constructed nowhere in `src/`. Task 3 remains blocked, and the plan forbids closing
  that here — a product gap returns to the owning child plan. The Library route
  design is that return, and its five prerequisites all landed on 2026-09-08, but the
  screen itself is not built.

## Task 2 Step 1 — Deterministic authority review (2026-09-30)

The status banner and the "Tasks 2 through 6, none run" bullet describe the
2026-09-08 Task 1 checkpoint. This section is Task 2 Step 1 only. Steps 2
through 6, and Tasks 3 through 6, are not claimed here.

Reviewed `nyon-workshop-core` Workshop V1 authority at `b7a69f2` (parent of
the commit that adds this section). Living V2 is a separate rule set
(`living/mod.rs`) and is outside this step. The root presentation camera uses
`f32`; it is not the authority.

| Hazard | Where inspected | Result |
|---|---|---|
| Unordered traversal | Entity, revision, and branch maps are `BTreeMap` / `BTreeSet` (`model.rs`, `simulation.rs`, `history.rs`, `archive.rs`, `command.rs`) | Typed-ID order. `ordered_state_encoding_ignores_map_insertion_order` |
| Float or wall-clock state | No `SystemTime` or `Instant` in the core sources. Pack `visit_f64` rejects floats (`pack.rs`). Orbit and travel use integers (`simulation.rs` distance ceiling) | Presentation floats stay out of `WorkshopStateV1` |
| Unchecked arithmetic | `step_state` and batch apply use `checked_add` / `checked_sub` / `checked_mul` and return `DeterministicFault` | No wrapping on the tick or batch path |
| Partial mutation | `step_state` writes `state` only after every phase (`simulation.rs` assignment after the event digest). `apply_batch` clones, and replay assigns the candidate only after the digest matches (`history.rs`) | `shipment_capacity_fault_rolls_back_the_complete_tick` |
| Inconsistent encoding | Domain-separated SHA-256 for state, tick events, revision, and entity (`model.rs`, `simulation.rs`, `command.rs`) | `deny_unknown_fields` on `TickEventV1` |
| Digest exclusions | `digest_with_dynamic` hashes tick plus every entity map, including the post-tick worlds, deposits, and shipments | Pause, speed, and camera are not fields of `WorkshopStateV1` |
| Local-reference type confusion | `allocate` rejects a duplicate local id and an id already in state or the batch. `resolve` rejects an absent, forward, or wrong-kind reference (`command.rs`) | Creator rejection, not a committed state |
| ID collision | `allocate` returns `IdentityCollision`. `dispatch_routes` returns `DeterministicFault::IdentityCollision` when `occupied_ids.insert` fails | Typed fault, no second shipment |
| Phase-order drift | `step_state` order is hazards, deliveries, primary industries, processors, route dispatch, then digest and tick increment | Matches the design's six simulation steps. `exact_phase_order_delivers_then_produces_then_dispatches` |
| RulesV1 coupling | `nyon-workshop-core` does not depend on the Classic crate. Workshop step rate in the design is 10 ticks per second | No RulesV1 phase order in `step_state` |

Checkpoint scheduling uses `saturating_sub` (`history.rs`, `maybe_checkpoint`).
That path does not change canonical state. `checkpoint_cadence_cannot_change_replay_digest_or_archive_bytes`
pins the digest. It is not a tick-path finding.

### Finding ledger

| Severity | Reproduction | Test | Repair commit | Rerun |
|---|---|---|---|---|
| none | No reproducible Step 1 defect | The six suites below | none | EXIT 0, 38 passed, 0 failed |

Rerun on this code, one Cargo invocation, exit code read from the command:

```sh
cargo test -p nyon-workshop-core --test simulation --test history --test archive --test two_system_forge --test creator --test pack
```

| Suite | Result |
|---|---|
| `archive` | 3 passed |
| `creator` | 11 passed |
| `history` | 6 passed |
| `pack` | 9 passed |
| `simulation` | 8 passed |
| `two_system_forge` | 1 passed |

**Next for this plan: Task 2 Step 2, the parser and archive attack-surface review.**

The 2026-09-08 sentence that `RequestExport` is constructed nowhere in `src/`
is stale at `b7a69f2`: `ClientRuntime` queues it at `src/app/client_runtime.rs`
(`enqueue(WorkshopAction::RequestExport)`). `RequestImport` and `RequestLoad`
are still the enum and match arms only. That product gap blocks Task 3. It is
not a Step 1 finding, and it does not block Step 2.

## Task 2 Step 2 — Parser and archive attack-surface review (2026-09-30)

Reviewed the Workshop V1 pack parser and archive decoder at `0c1568d` (parent
of the commit that adds this section). Living V2 wire is outside this step.
No reproducible parser or archive defect, so there is no repair commit.

| Surface | Where inspected | Result |
|---|---|---|
| Byte cap before parse | Pack `strict_preflight` checks `MAX_PACK_BYTES` (1,048,576) before `parse_strict_value`. Archive `inspect_archive_wire` checks `MAX_ARCHIVE_BYTES` (16 MiB) before `serde_json::from_slice` | `pack_rejects_oversized_bom_invalid_utf8_trailing_and_depth`; `archive_catalog_inspection_authenticates_only_a_canonical_supported_envelope` |
| Duplicate keys | `StrictValueVisitor` rejects a repeated key at every map depth. Archive decode does not use that visitor; canonical re-encode must equal the input, so a duplicate key cannot survive | `pack_rejects_duplicate_keys_at_root_and_recipe_depth`; pretty-printed archive bytes return `NonCanonical` |
| Depth | Pack preflight rejects nesting above 32 before parse. The design's depth-32 rule is under Validated Data Packs. Archive nesting uses pinned `serde_json` 1.0.151 with no `unbounded_depth` feature: `remaining_depth` starts at 128 and `RecursionLimitExceeded` becomes `ArchiveError::Json` | Pack depth rejection is tested. The archive difference is the spec's pack rule, not a crash |
| Allocation | Both caps run before parse. After a successful archive parse, revision count is refused above `MAX_REVISIONS` (10,000) and branch count above `MAX_BRANCHES` (64) before the graph walks. `IncrementalMaterializer` refuses a revisit or a chain of length `MAX_REVISIONS` | Counts are bounded by the byte cap plus those limits |
| Unknown fields and floats | Pack structs use `deny_unknown_fields`. `visit_f64` rejects floats. Archive wire and payload use `deny_unknown_fields` | `pack_rejects_unknown_fields_floats_and_authority_content` |
| IDs and names | `is_pack_id` is `[a-z][a-z0-9._-]{0,47}`. `is_catalog_id` is `[a-z][a-z0-9_-]{0,31}`. Names are 1 through 64 printable ASCII bytes with no leading, trailing, or doubled space. Descriptions are at most 512 printable ASCII bytes. Branch names use the same printable-ASCII rule | `pack_rejects_bad_ids_names_counts_quantities_and_references` |
| Reference graph | Unknown resources and duplicate catalog ids are typed pack errors. Archive revision ids must be strictly sorted and unique. `validate_revision_graph` color-marks the parent DAG and returns `ArchiveError::Revisions` on a cycle before `validate_branches`. `is_ancestor` has no visited set of its own and runs only after that color pass | Quantity and reference rejections are in the pack suite. `IncrementalMaterializer` repeats the cycle guard on replay |
| Hash domains | Pack hash domain is `b"NYON-WORKSHOP-PACK-V1\0"`. Archive integrity domain is `b"NYON-WORKSHOP-ARCHIVE-INTEGRITY-V1\0"` | Tamper returns `Integrity` from `archive_catalog_hash` |
| Active view | An authenticated archive whose active tick is before the selected branch tick is `ArchiveError::ActiveView` | `authenticated_archive_rejects_active_tick_before_selected_branch_tick` |
| Paths, URLs, scripts | The pack schema has no path, URL, script, texture, shader, or WASM field. Unknown authority fields (`scenarios`, `tick_hz`) are rejected. Printable ASCII names can contain the characters of a URL. Nothing in `pack.rs` fetches or opens them. That is display text, not a loadable asset | `pack_rejects_unknown_fields_floats_and_authority_content` |
| Error redaction | `ArchiveError`, `PackValidationError`, `HistoryError`, `CreatorRejectionV1`, and `DeterministicFault` display fixed phrases, static field names, sizes, or a budget. `ArchiveError::History` is transparent over those fixed phrases. Diagnostics receive `error.to_string()` and do not receive the document bytes | Inspected Display impls. No payload echo |
| Invalid-import atomicity | `RequestImport` builds `ArchiveDecodeJob` before any history assignment. `poll_decode` takes `pending_decode`. `new` `Err`, `poll` `Err`, and `finish` `Err` only push `ArchiveRejected`. `self.history` is assigned only inside `finish()` `Ok`, which also sets `"Imported; not saved"` and marks dirty. `RequestExport` encodes without replacing history | `src/workshop/session.rs` `RequestImport` and `poll_decode` |

`RequestImport` pauses the session before validation. A rejected import leaves the existing history, slot, and dirty flag in place.

### Finding ledger

| Severity | Reproduction | Test | Repair commit | Rerun |
|---|---|---|---|---|
| none | No reproducible Step 2 defect | The pack and archive suites below, plus the active-view unit test | none | EXIT 0; archive 3 passed; pack 9 passed; active-view unit 1 passed; 0 failed |

Rerun on this code. Exit codes were read from each command:

```sh
cargo test -p nyon-workshop-core --test pack --test archive
cargo test -p nyon-workshop-core --lib authenticated_archive_rejects_active_tick_before_selected_branch_tick
```

| Suite | Result |
|---|---|
| `archive` | `test result: ok. 3 passed; 0 failed` |
| `pack` | `test result: ok. 9 passed; 0 failed` |
| `archive::tests::authenticated_archive_rejects_active_tick_before_selected_branch_tick` | `test result: ok. 1 passed; 0 failed` |

**Next for this plan: Task 2 Step 3, storage failure ordering.**

`RequestImport` and `RequestLoad` remain enum and match arms only, so Task 3
stays blocked. Windows and Linux hosts still block Task 5 only. Neither limit
is a Step 2 finding.

## Task 2 Step 3 — Storage failure ordering (2026-09-30)

Reviewed Workshop V1 native and browser storage at `893140b` (parent of the
repair commit). One reproducible defect was repaired in that commit.

| Surface | Where inspected | Result |
|---|---|---|
| Temporary-file locality | `persist_generation` creates the temporary with `NamedTempFile::new_in` on the slot directory | Same directory as `generation-*.archive` |
| Flush and directory sync | `flush`, then `File::sync_all`, then `persist`, then `sync_directory`. On Unix that opens the directory and calls `sync_all` | Present before and after the repair |
| Read-back hash | Before the repair, the descriptor hash was `Sha256::digest(archive)` of the caller buffer. The synced temporary was not read | Finding, repaired. `read_synced_temporary` seeks to the start of the synced file and the commit refuses when those bytes differ from `archive`. The stored hash is the digest of the bytes read back |
| Pointer replacement | `NamedTempFile::persist` renames the temporary onto the generation path. The manifest, which is the generation pointer, is written only after that rename | `native_crash_after_generation_write_leaves_the_atomic_manifest_on_the_old_head` |
| Prior-generation retention | A commit keeps the new descriptor and the previous valid descriptor. A manifest with more than two generations is `CorruptManifest` | `native_store_recovers_the_previous_valid_generation_without_moving_the_head` |
| Generation conflict | `CommitSlot` compares `expected_generation` with the manifest head and returns `StaleGeneration` without writing. Two store instances serialize on `store-v1.lock` | `native_generation_cas_is_atomic_across_store_instances` |
| IndexedDB schema | Database `nyon.workshop.v1`, version 1, object stores `slots`, `archives`, and `packs` | `indexeddb_identity_and_object_store_names_are_frozen` |
| Immutable generation keys | Archive keys are `slot:{id}:generation:{generation:020}` | A new commit writes a new key. The slot reference keeps at most two generations |
| Single-transaction ref updates | `mutate_references` opens one read-write transaction over `slots` and, for archive writes, `archives`. A mutation error calls `abort` | `transaction_model_publishes_only_after_success`; `wasm_source_uses_atomic_transactions_and_bounded_diagnostics` |
| Abort, quota, schema, eviction | The host transaction model publishes the candidate only when no injected failure is pending | `browser_failure_classes_preserve_the_committed_generation`; `promotion_abort_and_quota_failure_preserve_head_and_exact_recovered_predecessor` |
| Unload non-success | Session status becomes `Saved` only from `StoreJobState::Complete` with `SlotCreated` or `SlotCommitted`. The wasm waiter resolves only from IndexedDB `oncomplete` and rejects from `onabort`. No `beforeunload` or `pagehide` path assigns `Saved` | Inspected. The native gate does not execute `src/workshop/store/web/wasm.rs` |

### Finding ledger

| Severity | Reproduction | Test | Repair commit | Rerun |
|---|---|---|---|---|
| The native commit could publish a generation whose synced file bytes did not match the archive it claimed | At `BeforeGenerationPersist` on the second generation write, overwrite the synced temporary with `torn-temporary`. Before the repair the commit did not return `WriteGeneration` / `InvalidData` | `native_commit_refuses_a_synced_temporary_that_does_not_match_its_archive` | this commit | Focused test EXIT 0, `test result: ok. 1 passed; 0 failed`. Native gate below |

The failed observation was the same test before `read_synced_temporary` existed. After the repair, a mismatch returns `WorkshopStoreError::Io { operation: WriteGeneration, kind: InvalidData }`, the generation file is not renamed into place, and a reopened store still loads generation 1.

Native CI gate from `AGENTS.md`, each exit read from that command:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo build --release
```

| Command | Result |
|---|---|
| `cargo fmt --all --check` | EXIT 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | EXIT 0 |
| `cargo test --workspace --all-targets` | EXIT 0. 53 lines of `test result: ok`, 876 passed, 0 failed |
| `cargo build --release` | EXIT 0. `Finished release profile [optimized] target(s)` |

**Next for this plan: Task 2 Step 4, the UI and accessibility review.**

`RequestImport` and `RequestLoad` still block Task 3 only. Windows and Linux
hosts still block Task 5 only. The wasm store still needs the wasm gate and a
browser run; this native gate does not supply that.

## Reproducing this record

```sh
git checkout 60db100
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
cargo build --release
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh          # after the web builds, never before
shasum -a 256 dist/webgpu/nyon.js dist/webgpu/nyon_bg.wasm \
              dist/webgl/nyon.js  dist/webgl/nyon_bg.wasm
```

Read each exit code from its own command. Do not read one through a pipe, a
trailing `echo`, or a background wrapper's status.
