# Living completion ledger — 2026-10-08

Status: **Partial**. This milestone strengthens V2 preparation, storage recovery
and player transport. Due-intent execution and boundary sealing still have a
synchronous suffix. No V3 mechanics are approved or implemented by this milestone.

## Implemented and reviewed

- Applied revision-ID preparation is counted, retains accepted journal order,
  and sorts receipt IDs without losing duplicates. Sealed preparation exposes
  immutable borrowed state and receipt accessors.
- Reviewed removals compare the exact ordered cascade witness before mutation
  through counted stages. Creator dispatch no longer constructs its synchronous
  creator bridge. Original nested allocations remain owned for retirement.
- Speed changes cancel private work while preserving accepted commands.
  Effective pace measures the trailing one-second publication window and reports
  recent update caps.
- `NYON_PROFILE_DIR` isolates app-owned native persistence. Unset preserves
  existing defaults; empty or relative explicit roots fail startup.
- Living and Continue IndexedDB callbacks catch synchronous mutation failures,
  abort provisional transactions, and preserve the original error through
  exactly-once settlement.
- Active Living save failures preserve the resident, accepted work and screen,
  release operation holds, and expose the exact failure and retry guidance.
  Bootstrap and inactive failures retain their existing recovery policy.
- Sighted storage status uses a bounded printable-ASCII projection, measured
  wrapping and ellipsis. Multiline/Unicode failures cannot reject the entire UI
  batch; full raw semantic text and diagnostics remain available. The narrow
  footer can truncate sighted retry guidance, so full error usability is open.
- Trade Notebook persistence tests identify the pending stage and retain exact
  save/replay assertions with a 120-second deadlock guard. That guard is not a
  latency acceptance threshold.

## Automated evidence — Current

Final source candidate passed all documented native gates: formatting, strict
workspace/all-target/all-feature Clippy, **1,805 workspace tests** across 101
summaries (zero failures or ignored tests), and release build. Toolchain remains
the intentionally refreshed `nightly-2026-10-08`; dependency pins are preserved.

The separate fuzz workspace passed formatting and locked compile/lint checks;
fuzz execution was not qualified. Both backend web builds, root and core wasm
checks, wasm Clippy and `check-workshop.sh` passed. Independent V2 vectors,
vertex/shader layout validation, static cross-build fingerprint checks and
**25 Python tests** passed. The fingerprint invocation used `--skip-tests` and
is static evidence only. The callback regression exercises 13 scenarios against
the actual inline JavaScript; mock tests do not establish real quota exhaustion.

Raw commands, exit codes, counts, source identities and independent slice reviews
are retained locally under the ignored execution ledger
`.superpowers/sdd/2026-10-02-living-sealing-and-replay/`. Compilation and tests are
distinct from the live observations below.

## Browser evidence — Current within this scope

Fresh isolated origins on port 18767 loaded the final rebuilt artifacts. A real
IndexedDB `put` callback was instrumented to throw `QuotaExceededError` at the
Living head write after provisional payload/manifest writes. Its message included
LF, CR, tab and non-ASCII text. The active galaxy remained at tick 2, full error
and retry guidance remained in semantics, and no escaped window error or
UI-composition warning was observed during that failure. The six-record store
retained head `3`, saved tick 1, and whole-store SHA-256
`b6f6b6337ec95a4cede6d6ea016fdccf69a49177f33ceede65dbdc9d5b2e4d31`.
Restoring the original API and retrying saved tick 2; fresh reload reopened it.
This is injected real-API failure evidence, not genuine origin quota exhaustion.

Production Vale Confluence was stepped to tick 100, saved and freshly reopened
on WebGPU and forced WebGL2 Low. Each canonical archive was compared byte for
byte with the native pure-authority production-content reference:

| Property | All three authorities |
| --- | --- |
| Final digest | `7869140a62c25ebfd2ef4bc495f53e274b0465041cfde54ace5e742cb348cfb8` |
| Archive bytes | 15,088 |
| Archive SHA-256 | `0c92f4fedf3365b4049f1755eaaf33de2cbcb977da950ef1526a6ffc656cba20` |

The native reference uses production authority and content; it is not an
independent encoder oracle. Screenshots were inspected:
[quota feedback](2026-10-08/living-quota-feedback.png),
[WebGPU tick 100](2026-10-08/living-webgpu-tick100.png), and
[WebGL2 Low tick 100](2026-10-08/living-webgl2-tick100.png).
Their different background colors are not a claim of rendering parity.
Keyboard focus plus Enter exercised Step and Save, without qualifying complete
Tab navigation, screen-reader behavior, touch or pinch.

## Native evidence — Current within this scope

The exact final release binary ran in a uniquely identified local qualification
app wrapper with an isolated absolute profile. Metal initialized on Apple M5 and
presented a frame. Native Step reached tick 1; Save acknowledged completion, and
the actual profile contained the 15,082-byte canonical archive with SHA-256
`2643732dd34c59a4a2521d5003cc8a01ebc9ec155d94f2367ccd7433980286ce`,
matching the native authority and browser tick-1 reference. Fresh app launch
reopened tick 1 with “Opened exact saved Living galaxy and notebook.” The owned
qualification app was closed afterward. Empty and relative profile checks exited
1 with the expected startup error. The wrapper is test infrastructure, not
distribution, signing or packaging qualification. A Retina visual sample and
pointer activation do not qualify the full native input/accessibility surface.

## Remaining work — Partial or Proposed

**V2 engine Partial:** bounded due-intent composition/reservations, candidate
validation, hashing, receipt/journal sealing, terminal Ready and borrowed
exactly-once extraction, shared stepping/replay/restore/fork/comparison execution,
durable cleanup, and the fully shared 1,024-visit update budget remain open.
The next production slice is complete due-intent composition wired into Step,
preserving the independent synchronous orchestration oracle.

**Player qualification Partial:** all-tool cancel/stale/durable fresh-Continue
journeys, remaining experiment witnesses, real quota exhaustion and generation
conflicts, portable transfers, offline reopening, complete native/browser
keyboard/focus/accessibility/Retina/touch/pinch acceptance, capacity p95,
endurance and memory qualification remain open. Host paging was observed;
these runs do not establish performance acceptance. No service worker was
registered in the earlier inspected browser profile; offline reopening remains
unverified.

**V3 Proposed:** the dated repair-eligibility clarification preserves the
original proposal. Its independent transition fixture, remaining initialization
and corpus coverage, complete-rules review and explicit rules approval precede
separate V3 implementation. No implicit V2 conversion is approved.

## Delivery

This verified prerequisite milestone is prepared for a local commit on canonical
`main`. Whole-program completion is not claimed. Push remains unauthorized;
remote CI has not qualified this candidate.
