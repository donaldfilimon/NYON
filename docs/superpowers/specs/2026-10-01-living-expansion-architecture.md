# Living Galaxy expansion architecture

Date: 2026-10-01. Status: **Proposed written specification for review**.
Parent: `docs/superpowers/plans/2026-10-01-sandbox-completion-and-expansion.md`.

## Product contract

Deliver autonomous societies with population needs, specialization and research;
deterministic evolving ecology; negotiated diplomacy and objective-driven
conflict. Include creator interventions, explanations, experiments, immutable
history, saves and portable reproduction for every subsystem. The creator is
omniscient and can override conditions/policies; autonomous actors only consume
their permitted observations. No player faction, mandatory victory or forced
terminal state is introduced.

This architecture specification is not approval of numerical rules. Focused
society, ecology and diplomacy specifications must define their exact formulas,
phase order, catalog/wire fields and independent vectors before code for those
mechanics is accepted. Existing V2 completion work remains authorized by its
approved contracts and proceeds separately.

## Version and authority isolation

Use **Living V3** for stateful expansion mechanics. Preserve V2 code, catalog,
wire/hash domains and archive interpretation. An old save always opens through
its matching engine. Do not append population/research/alliances/environmental
clocks to V2 canonical state, invent omitted-default V2 mechanics or change V2
replay based on an application setting.

Implement the new pure authority in a separate `nyon-living-core` workspace crate.
Its dependencies may use the already-approved serialization/hash/error libraries
but never the root application, wgpu, winit, Tokio, browser/storage APIs or the
Workshop/V2 authority. Reuse patterns and engine rendering rather than converting
legacy IDs or making V3 depend on a frozen V2 state machine.

Each domain owns its entity/branch/revision IDs, catalog hashes, state digests,
receipt digests, commands and archive types. Canonical domains include an explicit
V3 marker; the focused wire specification defines their literal bytes and ordered
fields before deriving expected vectors independently.

No migration is part of first delivery. V2 libraries remain visible and usable.
V3 starts from validated V3 content or a new V3 document. A later conversion tool
would create a separate new document, identify losses and retain the original;
it requires its own specification and authorization.

## Core execution and publication

The Proposed core schema now binds Actor32 source selection, decoder stages, optional minimal tick-zero profile, derived future-Truce account and exact history/Notebook operators. Scoring §Conditional private B/F operator defines private scenario dependencies and14 capacity accounts; unsupported outcomes never become authority evidence. Running acceptance stages its exact phase1 prefix Revision/capture and durable mark, while the complete Receipt seals only after queue plus later phases. Aggregate creator events report operation-order mutations; they are not an instruction stream. Historical counters/forks and Notebook ordinals follow schema, with no new command/queue/header fields. Public0/>1024 rejects invalid_range; internal exhausted0 continues.


One unpublished job owns a complete candidate and cumulative event facts.
Capture immutable authority inputs without deep copying. Copy/index/hash/phase
work, forecasts, validation, encoding and journal preparation share a bounded
record-visit budget. V3 also uses 1..=1024 visits per poll; returning Pending is
not permission to do an uncounted full-state traversal first.

Ecology §Boundary order is the exact11-step authority order, including guarded
phase5 costs before phase6 needs. Phase7 captures every actor's permitted planning
records once. Phase8/9 share that immutable input and one central ledger; protected
category0 plans precede other material admissions and older pending offers.
Scoring §§1–8 fixes the200-boundary forecast, imminent<=100 gate, four categories,
policy weights, complete identity, force selection and stable merge/ties. Neither
negotiation nor reservations recapture/rescore the actor's proposed candidates.
Exact full candidate/audit declarations are in complete-schema's integrated sections.
ActorInputContext hashes only permitted planning context; outer full State/revision
provenance is authenticated separately through CandidateAudit/Receipt. The trace
contains eligible complete identities only; central attempted denial reasons have
a fixed separate first-failure order. Tagged economy/offer/war components preserve
exact explanations without inventing a generic utility bucket.

Prepared success/failure tokens have private constructors. Publication verifies
exact capture ownership, queue/revision cursor, allocator marks and resident
generation before moving owners. No fallible work or capacity-sized destruction
occurs inside that fence. Retired owners are reclaimed by a bounded queue.
Faults preserve the preceding valid state and accepted queue; ordinary invalid
submissions consume neither IDs nor resources. Cancellation never publishes a
private candidate.

Actor proposals use one immutable information-limited snapshot. Central
reservation decides conflicts in the specified stable order. Population, food,
energy, industry slots, route capacity, research funding and fleet assets cannot
be spent twice by economy, ecology adaptation and military plans. Own paid accounts
are physical local stock, never forecast output or unmanifested promises. Due
Food/energy holds guard discretionary production/labs/repair/military supply before
needs; separate hull/building reservations remain explicit. Four tiers are imminent
needs recovery, recovery, obligation and ordinary. Strict category0 gate is the
user-selected requirement; other numeric/coordination choices remain Proposed.

An actor's War candidate contains only its own objective/funding witness. Central
admission attaches the defender's objective from its own same-tick private phase7
capture before atomic War installation. Private defender selection affects only
that later outcome, not declarer inputs/identity/order. Only the installed public
objective is disclosed; authority/private audits retain exact capture bindings.

## Session, controls and projection interfaces

Keep version-specific Sessions. The root sandbox facade routes operations to
their matching Session; it does not expose an untyped mutable state union.
Read-only projection may normalize display values into entity references that
retain the authority discriminator and full identifier bytes. Entity selection
from one version can never be dispatched into another version's command builder.

Semantic actions carry resident ownership and draft/modal context exactly as
current Living input does. Switching versions retires queued input, clipboard
completion, audio cues, comparisons and transfer jobs belonging to the old
resident. Read-only selection/layers/audio never enqueue authority commands.

Inspectors show population trend/needs, industry inputs and blocked causes,
specialization/workforce, research funding/progress/unlocks, environmental
conditions/warnings, treaty terms/obligations and war objectives/readiness.
Omniscient facts and actor observations have distinct labels; observations show
source and timestamp. Decision explanations reference recorded candidates,
scores/rejections and facts rather than reconstructing a reason from final state.

Creator drafts preserve invalid intermediate text. Preview describes exact
before/after values, dependencies, cascades, refunds/dispositions and application
boundary. Free creator interventions remain distinct from actor-paid decisions.
Every new operation supports typed validation, atomic application, provenance,
undo/redo, exact history restoration and save/transfer.

## Library, storage and portable reproduction

Route files by explicit recognized top-level kind and version before constructing
an engine. Unknown versions fail visibly and preserve the resident document.
Exact V3 catalogs are required; matching a name is insufficient. Never use a V2
pack to interpret V3 state.

Store V3 slots/packs in a distinct namespace with generation CAS and neutral
Continue selection naming the exact authority/version, slot, branch and view.
Extend the coordinator through a separately versioned record; old valid V1/V2
selections remain read-only fallback when the newer coordinator is absent.
An existing corrupt/errored newer record cannot silently fall back to selecting
another galaxy. Explicit recovery retains the preceding independently valid
generation and names what the user can recover.

V3 portable reproduction includes exact catalog, archive and Notebook evidence.
Import into a new slot after bounded validation/replay. Export owns one coherent
capture and checks freshness before each handoff. Native durable completion and
browser download handoff stay distinct. Cancellation racing a won commit must
report actual durability and reconcile selection rather than claim no mutation.

Notebook references include exact domain, branch, revision, tick, state/receipt
digest and event identity. Validation replays the selected history. Comparing
different rules versions is out of scope; a V3 experiment compares retained V3
branches at one tick and reports measured differences without universal causal
claims.

## Acceptance and review package

Require independently derived V3 canonical vectors, invalid/unknown/truncated
files, duplicate/dangling IDs, stale/cancelled jobs, arithmetic/capacity failure,
simultaneous scarcity, hidden/stale intelligence, lifecycle/capture/removal and
complete creator cascades. Compare uninterrupted, saved/replayed and transferred
results; vary poll budgets and render cadences without changing digests.

Each subsystem includes deterministic content witnesses, ordinary visible and
semantic operation, evidence-backed experiment, save/Continue and export/import.
Repeat native/WebGPU/WebGL2/accessibility/offline/endurance/performance acceptance
for exact artifacts. No proposed mechanic becomes Current merely because its
state type or unit test exists.

Written review must settle: independent V3 crate and no automatic migration;
coordinator compatibility; exact resource/clock interactions; population and
research limits; ecological schedules; treaty obligations/intelligence; objective
resolution and logistics. These are the next focused specification deliverables,
not implementation choices delegated to an agent.


## Integrated review corrections, 2026-10-02

Known environmental forecasts require exact observed event targets, intervals,
creator-edited factors and original observer/timestamp; their bounded visibility
records are schema correction I1, not permission to scan hidden root schedules.
Snapshot, audit, utility and creator-order semantics follow schema corrections
I2–I5/scoring; coordinator reads enforce derived504-byte cap before parse (I6).
These changes are Proposed, preserve V2 separation, and require new integrated
vector/qualification evidence rather than relabelling baseline subset success.


## Integrated round2 environmental scope correction (Proposed)

Complete-schema §Integrated correction I1 is the normative environmental knowledge
registry. Complete timestamped system/lane assertions include empty negative
coverage; choose newest scope across all permitted source worlds before flattening
positive event IDs. Select newest per-event child over retained winning positives
INCLUDING expired children, then filter the chosen child by expiry, then check128-
fact capture limits; never resurrect a losing older copy on expiry. Scope history
is unchanged by filtering; source refresh/eviction is separate knowledge loss. Equal ticks use lowest
original observer then canonical scope bytes, without own-source priority. Missing,
partial or unrelated scope data grants no absence evidence. Shared/request copies
preserve scope bytes and timestamps. Forecasts use only winning permitted scopes;
hidden cancellations retain stale facts until authorized supersession or expiry.
Fresh schedule observation requires an incident directly visible world; own fleet
traversal alone does not refresh schedules. This explicitly Proposed visibility
refinement replaces round1's fleet-only read and changes observation/capture wire
bytes, not balance tables, combat sight or movement. Empty winners enter actor
snapshot hashes; complete observation carriers and scopes obey schema bounds.
Old-baseline corpora remain historical, and full integrated gap6 remains Partial.
