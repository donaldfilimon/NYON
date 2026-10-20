# Seeded galaxy standalone qualification — 2026-10-11

Status: **Current focused implementation and validation; Partial shared integration.**
This is strategic metadata only. Independent Task 5 source review is pending at
executor freeze. No full-game, persistence, UI, browser, or 512-economy acceptance
is implied.

## Authority and source identity

Approved specification SHA256:
`5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d`.
Approved plan SHA256:
`fe29311846a3a569a1a736ec958e8d6c257c7465b9b2b3662e38a21327184658`.
Compiler: `0.18.0-dev.146+35accc06e`, `/Users/donaldfilimon/.zvm/bin/zig`.

The evidence directory is
`.superpowers/sdd/2026-10-11-nyon-zig-seeded-galaxy/task-5-evidence/`.
`entry-baseline.json` preserves the content/hash/absence of all 26 dispatched
paths; all matched dispatch at entry. `freeze.json` records exact final source
hashes, `owned.diff` the bounded source/document diff, and `review-package.tar`
the frozen files and logs. These describe uncommitted source, not HEAD.
`validation.json` and `final-validation.json` retain command argv, cwd, start/end
and raw process exit status; logs are separate files, never pipeline exit codes.

## Current focused results

- Behavioral RED: placeholder CLI returned `NotImplemented`; the supplied
  maximum journey failed, then all four initial behavior tests failed. Logs:
  `red-cli.log`, `red-cli-all.log`. The first green run exposed a test argument
  slice error (creator accidentally included selection); corrected the slice
  to exercise the intended null policy. No production expectation was relaxed.
- Format check: exit 0. Final aggregate: **48/48 Debug and 48/48 ReleaseSafe**.
  CLI: **10/10 Debug and 10/10 ReleaseSafe** (includes imported tests).
- Small seed0, 2/4/2, budget1, ruler2: exit0; 177-byte export/decode verified;
  digest `d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1`.
- Maximum seed-u64-max, 64/512/16, budget256: exit0; 11,455-byte export/decode
  verified; digest
  `8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30`;
  bridge returns `CapacityUnsupported`. Seed0 maximum is independently pinned
  to `9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852`.
- Invalid executable argument `--budget 0`: exit1, exactly one stdout ERR row.
  Standard Zig main also reports `error: InvalidArguments` to stderr.
- Strict decimal grammar, unknown/missing/duplicate flags, overflow and dimension
  relationships are rejected. Creator/null, ruler2, combined1 and absent/invalid
  selections are tested. No filesystem/profile flags exist.
- Output includes exactly the ten specified heading kinds, all 64 systems,
  512 planets and 16 civilizations at maximum, and the terminal scope row.
  Small output fails at every byte boundary and maximum output at every row
  newline; every failure releases all requested heap. All four CLI admission
  allocation failures are exhausted, followed by a successful fifth run.
- Eight independent integer recurrence seeds exercise create/decode schedules
  including 0/1/7/256/4096 and repeated observational zeros; every exported byte
  and digest is compared with independent literals. Small/max work formula is
  `3*R + 3 + ceil((L+9)/64)`, plus one decode header unit: 30/31 and 1959/1960.
  Fresh-token cancellation/retry retains identical literals. The aggregate also
  includes predecessor exhaustive maximum creation/decode cancellation cuts,
  allocation failures, malformed wires and zero poll/query allocation witnesses.
- Both 11,455-byte literal fixtures copied into qualification were recomputed
  with the existing independent Python reference and matched byte-for-byte;
  see `literal-verification.json`. Python is optional developer evidence, not
  part of CLI runtime or final delivery dependencies.

## Ownership and memory accounting

CLI uses public Setup create/poll/take and copied record queries, then begins
owned-input decode of its copied export on the same controller. It compares
all decoded wire bytes and digest. The decoded handle is destroyed before any
later admission. Every owning handle and Setup receives an immediate `defer`.
Policy is checked before output. The small bridge validates Config only, using
explicit local 64/256/64/4096 and global4096 capacities; no v0 gameplay is created.

| Quantity | Bytes | Interpretation |
| --- | ---: | --- |
| Output allocation | 29,952 | One detached manifest's requested heap |
| Scratch allocation | 16,592 | Candidate scratch including owned decode input |
| Active candidate requested heap | 46,544 | Two allocations |
| Candidate reservation | 47,590 | Includes control envelope and alignment |
| Detached reservation | 30,983 | Output, handle/control allowance and alignment |
| CLI retained output + candidate requested heap | 76,496 | Measured by tracking allocator |
| Replacement reservation | 78,573 | One resident plus candidate, below 196,608 |
| Two detached outputs requested heap | 59,904 | Transient validation pair |
| Two detached reservations | 61,966 | Below 131,072; no third owner is admitted |
| CLI export buffer | 16,384 | Caller stack scratch, outside Setup reservation |
| CLI streaming stdout buffer | 8,192 | Caller stack scratch, outside Setup reservation |
| One copied wire record | 256 | CLI and `test_support.collectWire`, not 64 records |
| Copied system/planet/civilization values | 16 / 24 / 12 | Caller scratch |
| CLI bridge capacities | 32 | Four explicit capacity values, caller scratch |
| CLI digest/config | 32 / 24 | Copied caller values |
| Benchmark timing arrays | 30,400 | 1,900 i96 samples, padded to 16 bytes each |
| Benchmark ceiling input/output buffer | 16,384 / 8,192 | Instrumentation caller scratch |

CLI tests use an allocating output writer whose allocation is separate from
Setup accounting; production streams. Process argv/arena, allocator metadata,
other scalar locals, library formatting frames and OS resources are not measured
RSS or an exact compiled stack peak. They are not represented as zero cost.
The reservation limits govern Setup plus retained manifest ownership, not total
process memory. Benchmark begin paths attempted 600 allocations; all 1,600
successful nonzero polls plus ceiling-cancel polls attempted zero allocations;
remaining requested heap was zero. Copied query zero-attempt proof comes from
focused tests, separate from timing samples.

## Named-host timing evidence

Host: **Mac17,2, Apple M5, arm64, 24 GiB RAM**; macOS **27.2 (26B5101f)**.
`hardware.json` contains the live command results. Optimization: ReleaseSafe.
The final run occurred during a concurrent repository gate on this host; no
exclusive machine scheduling or cross-hardware performance claim is made.
Run: `zig run zig/src/seeded_galaxy_tests.zig -O ReleaseSafe`.
The aggregate's standalone main is qualification instrumentation, not game startup.
Final raw measurements: `performance-release-final.log`.

| Calls measured | Samples | p95 (ns) | Maximum (ns) |
| --- | ---: | ---: | ---: |
| Create nonzero poll, budget256, 100 maximum jobs | 800 | 4,500 | 91,000 |
| Decode nonzero poll, budget256, 100 maximum jobs | 800 | 4,084 | 4,167 |
| Create admission including allocation | 100 | 11,750 | 36,000 |
| Decode admission including allocation + 11,455-byte copy | 100 | 10,625 | 40,500 |
| Decode admission including allocation + 16,384-byte copy | 100 | 8,458 | 43,375 |

Both measured poll sets meet proposed p95<=2,000,000ns/max<=8,000,000ns targets.
Samples bracket each call with the monotonic awake clock and compute
`before.durationTo(after).raw.toNanoseconds()` after attaching clock identity.
The 16,384-byte input is deliberately invalid wire: admission copies it, then
explicit cancellation retires it outside timing samples. It is evidence for
bounded admission-copy cost, not successful decoding of a 16,384-byte manifest.
Allocation and copy costs are combined in admission measurements, never charged
to or reported as poll work. The deterministic unit tests establish work bounds;
wall-clock evidence cannot replace them. The older `wire_tests` timing prints
and earlier warm-up logs are diagnostic only; this table names the final explicit
qualification run. Target-hardware/UI responsiveness acceptance remains pending.

## Shared integration and remaining acceptance

Existing `./tools/check.sh` from `zig/`, with pinned compiler first in PATH,
finished **exit1**, 13:20:39–13:23:54 UTC. Both native suites passed **628/628**;
planet CLI passed. The gate stopped at
`tools/check-classic-store-cli.py:177`, `AssertionError: source changed during
CLI qualification`. The subsequent surface CLI check was not reached. The
script's source-stability guard compares every Zig source and selected tools,
not just the exercised Classic code. This is failed source-stability evidence,
not a witnessed Classic behavioral failure or a seeded failure.

`gate-source-mtimes.json` records concurrent shared-owner writes, including
`coordinated_galaxy/wire_vectors_tests.zig` at13:23:31 UTC and expanded-session
allocator ledger files at13:22:22/31. The owned Task 5 qualification edit occurred
at13:20:45 DURING the whole gate, before its final focused checks. That file is
a guard input: `snapshot()` lines19–23 includes all `zig/src/**/*.zig`, all
`zig/tools/*.py`, `check.sh`, `build.zig` and `.zigversion`. The Classic script
starts later than the whole gate; the owned edit is not proof it changed during
that particular snapshot window. The guard does not preserve its before/after
maps or start timestamp; the exact drift set responsible for the assertion is
unknown. Neither solely external causation nor sole Task 5 causation is claimed. No shared source was repaired, reverted or retested into a misleading
whole-gate PASS. Raw failure is retained in `shared-gate.log` and validation JSON.

The root test closure does not import the seeded aggregate. Even a green existing
gate therefore supplies no integrated seeded runtime/build acceptance. All
changes here remain within the assigned strategic CLI/test/report paths. Shared
coordinator, root, main, build, check, AGENTS and profile edits were preserved.

The integrating owner must explicitly acquire `zig/src/root.zig`,
`zig/src/main.zig`, `zig/build.zig`, `zig/tools/check.sh` and any coordinated
acceptance aggregate before editing them. Concrete handoff:

1. Import the strategic tests into the required root closure; expose the facade
   and CLI command; add standalone CLI/build qualification to the required gate.
2. Read the then-current finished v0 creation/replay API. Compare bridged/manual
   identical Config through that unmodified path, requiring **exact local and
   global genesis bytes and heads**. Config-only validation here is not that proof.
3. Preserve the strategic digest as a separate association, without changing v0
   identity/genesis bytes or asserting that v0 replay verifies strategic metadata.
4. Rerun focused Debug/ReleaseSafe plus the full Zig gate and review exact sources.
5. Complete runtime game/native/browser/hardware acceptance and the expanded
   storage/coordinator scale, founding, fleets, civilization strategy and saves.

Actual v0 genesis/head equivalence is **Partial/pending handoff**, not verified
here. Full NYON delivery and Rust-source removal are outside this file ownership;
no Rust source or user data was deleted. Independent source review is required
before marking the standalone task accepted.
