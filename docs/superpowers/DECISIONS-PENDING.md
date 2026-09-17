# NYON: decisions awaiting the owner

Date: 2026-09-08 23:1x
Scope: everything the 2026-09-08/09 session surfaced that an agent should not decide.
Last reconciled 2026-09-09 02:1x.

**None of these block each other.** *(2026-09-17: the "four block, five do not" count
here predates entries 9 and 10. Entry 9 is resolved; the titles marked BLOCKS name the
entries that still hold downstream work.)* Each
entry states what it costs to decide *late*, because that is the only thing that
makes ordering them meaningful — several are free to confirm now and expensive to
overrule after the next task consumes them.

---

## 1. Genesis manifest wire schema — BLOCKS Living Galaxy Task 3c

`.superpowers/sdd/wild-snuggling-treehouse/gap-6-genesis-manifest-schema.md`

**The rules spec publishes no wire schema for the genesis manifest at all** (verified:
zero matches), while referencing it normatively throughout — "the validated source of
replay truth", feeding `genesis_manifest_digest_32` into the root-branch formula.

3c's method *is* authoring `minimal-genesis.json` by hand from spec text. There is
none. Authoring from `genesis.rs`'s invented envelope would produce a vector that can
only ever agree with the code — the exact failure independent derivation exists to
prevent.

Six decisions, with drafted text and a recommendation. The central one is settled in
my reading by an approved design document: the generator *constructs* the manifest, so
it is the generator's output and materialization is projection, not expansion. The
rest are genuinely open — chiefly whether the manifest is a standalone document at
all, given it *is* an archive field and mirroring the pack's envelope would triple
version fields inside every archive.

**Cost of deciding late:** nothing is frozen. A rewrite of an unwritten document.
**Cost of not deciding:** 3c cannot start.

## 2. Creator operation schema is frozen with no published table

`.superpowers/sdd/wild-snuggling-treehouse/gap-7-provenance-and-event-kinds.md`

**28 creator operations are frozen into identity with no published table anywhere.**
Decoding `command_bytes_hex` shows field names — `local`, `system_a`,
`distance_units` — that are pure `command.rs` decisions, and **`revision_id` hashes
those bytes**. This is the same defect as the entity-kind registry before Task 4
published it, at roughly seven times the surface.

The fix is a spec publication beside the entity-kind and phase tables, not a code
change.

**Cost of deciding late: this is the expensive one.** Ratifying costs nothing.
Overruling any single field name costs re-deriving every affected `revision_id` with
`tools/living-v2-vectors.py` and every digest downstream of it — and that cost grows
with each task that consumes these bytes.

*Note: this entry was originally filed against `provenance` and the 26 event kinds.
The Task 4 review measured those and found them low stakes; the operation schema is
the real freeze. The relocation is recorded in the gap file.*

## 3. Record field order has no upstream authority

Section 10 publishes the state's 24-field order but **no field list for any individual
record**, so record order cannot be independently derived. `model.rs` says so itself
and declares itself the authority.

`2b7bfbc` pins *drift* with a mutation-verified 200-key canonical sequence, and
`4fc7f4b` pins the genesis envelope the same way. **Those pins cannot say the order is
correct, only that it has not changed** — which is all they claim.

**Cost of deciding late:** publishing record field lists later is a documentation act
if the orders match what is frozen, and a re-derivation if they do not.

## 4. `LivingFacilityStatusV2` has no never-attempted variant

Two variants, `Operational` and `Blocked`, documented as "outcome of the most recent
due attempt" — and **at genesis there has been no attempt**. The genesis fixture labels
never-run facilities `Operational` because it is the only honest choice available, so
the type cannot distinguish "ran and succeeded" from "never ran". The 3b review
confirmed the gap is real and that `Operational` is the right stand-in.

**Cost of deciding late:** adding a variant is a format break requiring new vectors.
Cheap now, a re-derivation later.

## 5. `intent_ordinal` 0 sits outside the published registry — carries to Task 5

Task 4 published an intent registry running 1..16. The `ai_fleet_phase_8` corpus row
carries `intent_ordinal` **0** at phase 8, and the dispatch-ordinal exception at
`rules.md:355` covers route dispatch at **phase 9**, not intent generation. So one row
carries an unpublished value.

Pre-existing; publishing the table is what made it visible. **When a typed intent
lands it must not be resolved by editing the vector.**

## 6. Baseline review Finding 10 — a release obligation

The one Finding-7 residual that is a decision rather than a repair.

## 7. `construction_jobs` and `hull_jobs` are bounded by nothing

`06873f6` documented this rather than enforcing it, because section 1 assigns
reservation accounting to the creator-operation path and the civilizations plan homes
it in `civilization.rs`. Enforcing it in `model.rs` too would put one rule in two
places.

**Today a state with 2,048 facilities plus 2,048 outstanding construction jobs
validates.** The decision is whether that stays a `civilization.rs` obligation.

## 8. Facility `hit_points` is never range-checked while hull `hit_points` is

Found by the 3b review: `model.rs:1432-1437` never range-checks facility `hit_points`
while `check_fleets` rejects `hull.hit_points == 0` — **which is the only reason the
genesis fixture's `hit_points: 0` validates at all**. An asymmetry, not obviously
intentional.

## 9. The Library design has no task for the main-menu entry control — RESOLVED

**Resolved 2026-09-16 by `3b0e229`** ("open the Library from the main menu; fit the menu
at 480"). Kept below as the record of the gap.

`docs/superpowers/specs/2026-09-08-workshop-library-route-design.md`, recorded in its own
task list.

The design's twelve tasks include no main-menu entry control, while addendum §2 requires
main-menu entry — task 10 is the **in-Workshop** control only. Task 6 satisfies the
constraint **at the runtime seam only**: `open_library()` succeeds from `MainMenu` and is
tested there, but **nothing routes to it**, so the screen is currently unreachable in the
product from the main menu.

Task 6 was right not to add `MainMenuRoute::Library` unbriefed — it pulls in `menu_copy`,
`menu_slug`, icon arms and every `menu_capabilities()` assertion, and the design assigns
entry controls to their own tasks.

**The decision is where it goes:** task 8's dispatch, or a new task. **Cost of deciding
late: the two-slice ship plan delivers a screen no user can open**, and the omission is
invisible until someone goes looking for the control.

*Added 2026-09-09 02:1x. It was missing from this document's first version, found by
re-checking rather than by anything surfacing it — which is the drift this file exists to
prevent, occurring in this file.*

## 10. Native dialog dependency for transfer adapters — BLOCKS Task 11's real adapters

`docs/superpowers/spikes/2026-09-17-native-dialog-macos.md`

The addendum §8 spike ran its macOS half on 2026-09-17: `rfd` 0.17.2 builds beside
winit 0.30.12 with one `raw-window-handle`, is MIT, and the durable write sequence
(temp, `F_FULLFSYNC`, rename, parent sync) works. Windows, Linux/portal, packaging and a
manual picker run are still unproven, and the report lists three trade-offs to decide
(two objc2 generations on macOS, a `windows-sys` duplicate, portal-only vs a `gtk3`
fallback).

**Cost of deciding late:** Task 11's native adapter cannot start; the protocol and test
adapter are already in place, so nothing already built changes.

---

## What is NOT waiting on you

*Updated 2026-09-17.* Phase 2, the Library screen, is landed except its real transfer
adapters: design tasks 0-10 and 12 are on `main`, the main-menu entry is `3b0e229`, task
11's transfer protocol and test adapter are `7abec08`, and all four transfer-strip routes
are landed (Export this galaxy, Export content pack and Import content pack in `58ca424`;
Import galaxy, which is task 12's Import archive, in `2cafa03`), exercised only through the
scripted test adapter. Not waiting on you: Living Galaxy authority Task 5 (the creator
queue and atomic boundary orchestration). Still waiting on you: task 11's native and
browser adapters (entry 10), without which no import and no Save copy is live in the
product, and authority Task 3c (entry 1).

## The ceiling, restated so it is not rediscovered

**Artifact-qualified is the honest terminal state on this machine.** Nine of the
fourteen live-matrix rows need Windows 11 or Ubuntu hosts that do not exist here, and
the qualification plan forbids substituting compilation for live evidence. Runtime-
and release-qualified are unreachable **by design, not by omission** — which the
plan's own Task 6 Step 4 explicitly permits committing.
