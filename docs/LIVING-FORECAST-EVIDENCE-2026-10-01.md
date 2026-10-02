# Living scheduled forecast checkpoint

Date: 2026-10-01. Status: **Partial executor implementation**.
Parent: `LIVING-STEP-ORCHESTRATION.md` and the October sandbox program.

## Implemented change

The private forecast-input job copies and filters actor-visible forecast state
under the shared 1..=1024 visited-record budget. Ownership lookups, foreign record
removal and index cleanup participate in that budget. Explicit observations of
an unowned world override older route knowledge; unseen rival stock is hidden.

The private scheduled-forecast job composes that input job with lifecycle/arrival,
production/repair and dispatch jobs across all 200 forecast boundaries. Repair
shortages, due facilities, blocked input facts, trade permissions, route searches,
hazards, projected inventories and metadata retirement are cursor stages. Search
index retirement also visits records individually. Prepared state stays private.

The compatibility forecast adapter still drains these jobs synchronously. Actor
candidate generation/reservation, complete boundary validation/sealing/history,
resident pending transport and bounded owner reclamation remain outstanding.
This checkpoint does not establish authority latency, input latency or memory
acceptance. The private result retains its final state for subsequent composition.

## Focused verification

The initial scheduled-forecast first-poll regression failed with exit 101 before
implementation. The final focused forecast run exited 0: **13 passed, 0 failed,
0 ignored, 150 filtered**. Seven regressions were added across input preparation
and complete forecast execution. Logs: `/tmp/nyon-scheduled-forecast-red.log` and
`/tmp/nyon-scheduled-forecast-final-tests.log`.

Budgets 1, 64 and 1024 produce identical inventories, deficits, idle-facility lists,
complete final state and total visit counts against the retained reference phases.
Coverage includes hidden inventories, ownership loss, explicit unowned knowledge,
paid construction, cross-system freight, bilateral trade expiry, ion storms, repair
shortages, cancellation, incomplete finish, invalid budgets and horizon overflow.
The source state remains unchanged throughout private preparation.

## Current source and gate evidence

The frozen manifest includes 506 source/test/asset/tool files at
`/tmp/nyon-forecast-source-manifest.json`, SHA256
`8b989710afa2f1eef391363f0a2787532c59a13fcede428fe863da11ac21bc02`.
Documentation is outside that source freeze. Formatting, strict workspace
all-targets/all-features Clippy, original V2 vector verification, all 17 additive
vector modes and `python3 tools/validate-vertex-layout.py` exited 0.

Full workspace tests exited 0: **1,360 passed across 101 suites, zero failed or
ignored**. The frozen manifest was rechecked with zero changed inputs. Native release, independent fuzz formatting/locked strict lint, both backend
builds, root/core wasm checks, strict wasm lint and Workshop artifact checks all
exited 0. `python3 tools/cross-build-fingerprint.py --skip-tests` also exited 0;
this is static artifact/facade evidence, not live native/wasm canonical-byte parity. Result
index: `/tmp/nyon-scheduled-forecast-gate-results.txt`. An earlier interrupted
workspace driver supplied no terminal pass and is excluded from current evidence.

The dispatch checkpoint's 1,353-test and browser tick-1 journeys belong to its
previous source identity. They remain historical evidence in
`LIVING-DISPATCH-EVIDENCE-2026-10-01.md`, not runtime qualification of this change.

## Native runtime observation, 2026-10-02

Native release SHA256: `3d38fae15e3e08b3de5df8fe17d5b3af25b7b5dc4a14856621a28440bc13f127`.
A separate ignored qualification bundle at
`target/qualification/NYON Forecast Checkpoint 1360.app` binds successfully through
computer use. Its executable is a copy of this release artifact; no production
packaging or signing acceptance is implied.

A new First Expansion galaxy ran from tick 0 to 149, paused, then Step crossed
150. Displayed Founding World inventories were energy 212, ore 157, alloy 21,
with two fleets. Save reported “Galaxy and Continue selection saved”. Quit and
fresh launch reopened tick 150 paused with the same displayed inventories and
“Opened exact saved Living galaxy and notebook”. The entity inspector was usable
and showed next operation 160. This is a live native visible/semantic journey,
not a full canonical-byte comparison, 1000-tick reproduction or timing acceptance.
The preexisting saved galaxy was preserved; the qualification save uses a new slot.

A screenshot was displayed inline, but no durable screenshot file was produced.
It shows clipped left save-status text and awkward word/number wrapping in the
sidebar; full visual containment remains unqualified. On fresh background launch,
transport controls were disabled. Inspector opened, but the later Close/Raise
observation failed with ScreenCaptureKit error -3801 (“The user declined TCCs for
application, window, display capture”). Focus restoration and transport after
reopen therefore remain unverified; do not classify this as a confirmed authority
fault or silently request broader OS access.

## Artifact identities and browser observations, 2026-10-02

| Artifact | Bytes | SHA256 |
| --- | ---: | --- |
| Native `target/release/nyon` | 16209200 | `3d38fae15e3e08b3de5df8fe17d5b3af25b7b5dc4a14856621a28440bc13f127` |
| WebGPU loader | 117809 | `15b0ff8b7ab3feaea4b714feb6d874af03316d19065b7f0c77e0c8b62e004f64` |
| WebGPU wasm | 11171936 | `30f0a40c0487bf5f50bf64e42147c649b908ed45f376945e8b3ea741c4f7da09` |
| WebGL2 loader | 115744 | `180cb16e47eb584edc5ecbd402c35cbf15b26731e5028b36afd354742d517173` |
| WebGL2 wasm | 13091824 | `09f67fa90aae20f4f80b435f9edb4781539fc80baa2ecde30cfebf268cc617aa` |

Live WebGPU initialized successfully. A separate new First Expansion galaxy
advanced by native browser AX Step from0 to1 and2, then Run reached158 with two
fleets and Founding World energy212/ore157/alloy21. Repeated Playwright semantic
clicks caused no visible advance; the semantic mirror is offscreen and this does
not alone prove a product input defect. A subsequent AX batch failed on its
shadow-root target. While running, AX returned unnamed buttons whereas a DOM
snapshot retained labels. Screenshot/keyboard follow-up lost the tab binding;
the tab inventory then became empty. No WebGPU save/reopen claim for this new slot.

A fresh forced WebGL2 tab reported `WEBGL2 LOW initialized successfully`.
New First Expansion → Step → Save → reload preserved tick1 paused with Founding
World energy100/ore100/alloy120 and one fleet. Reload reported the exact saved
Living galaxy/notebook had opened. Screenshot succeeded inline at the browser's
ordinary 1086×610 surface, without a durable file. No full viewport matrix or
pixel containment qualification is implied. The completed tab, own localhost
server and qualification native process were closed. Existing saved slots remain.
