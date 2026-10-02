# Living dormant-recovery evidence

Status: Proposed implementation detail of the approved recovery experiment. The
accepted rule that creator status changes are creator interventions remains authoritative; the creator addendum documents its X18 event distinction. This adds no gameplay,
command, receipt kind, wire field, reward or experiment-completion claim.

An existing dormant civilization can recover either through an autonomous
settlement or through a reviewed creator change. Autonomous recovery retains
its existing `civilization_reactivated` event. Creator recovery MUST NOT emit
that event: X18 reserves it for autonomous gameplay. Instead, its observation
references the existing exact `creator_intervention` event and revision.

Creator evidence requires an actor that exists before that revision, has Dormant
status and zero owned colonies before the whole revision, has Active status after the whole validated
revision, and holds at least one colony afterward. The referenced bookmark must
still show that actor Active with an owned colony. A newly created civilization,
an unusually Dormant actor already holding a colony, a transient intermediate
colony created and removed in one batch, a name/policy
or inventory edit, ownership without a colony, a stale receipt or a generic
intervention without the transition is insufficient.

This evidence slice does not approve the draft X17 status-recomputation proposal
or change gameplay status rules. It observes the authoritative transition that
already occurs. Evidence must compare actual complete revision boundaries;
intermediate per-operation status changes are not sufficient. An unchanged
actor's colony count cannot establish creator recovery merely because another
actor's edit refreshed a status field.

The authority derives proof from the actual pre-revision and final-revision
states while applying the validated revision, then regenerates it during archive
replay. Evidence validation binds the exact ordinary receipt, event ordinal/ID,
creator revision, existing actor transition and final bookmark state. Replaying
a parent history view at the same tick is not a substitute: for running edits,
that view can include later scheduled phases absent from phase-1 pre-state.
Proof is a derived cache, excluded from canonical state/archive/Notebook bytes.
Cancellation, replay failure and failed install preserve resident metadata.

The current Notebook record shape can reference the exact intervention; it does
not need a new event kind or wire field. UI labels distinguish a creator recovery
observation from an autonomous recovery event, and display the authenticated
actor and colony witness if supplied. Neither certifies broad experiment success
or a general causal law. Existing settlement evidence remains autonomous-only.

Required public-API regressions: genuine paused and queued running recovery;
exact portable replay and reload; transient create/remove batch; newly created
active actor; initially Active actor gaining a colony; unchanged dormant actor
name/inventory edits; ownership-only edit; wrong same-tick cursor; forged event
reference; creator intervention misclassified as autonomous settlement; and
final-state loss of the recovered actor's colony. Existing encodings and
independent vectors remain unchanged. Whole-path work budgeting remains Partial
until the complete revision/replay pipeline meets its visited-record contract.


## Executed implementation checkpoint

Current, bounded host evidence on 2026-10-01: genuine creator recovery portable
validation first failed with Evidence while five exclusion tests passed
(`/tmp/nyon-recovery-evidence-red2.log`, exit 101). After the authority proof and
Notebook consumer integration, six public authority tests passed. The external
Session/native-store journey and unrelated-edit refusal subsequently passed
alongside those six (`/tmp/nyon-recovery-journey-tests3.log`, exit 0, 8/8).

The existing Notebook, evidence and independent additive wire suites pass 17/17
(`/tmp/nyon-recovery-notebook-core-tests.log`, exit 0); the original independent
Living corpus remains unchanged and verifies successfully
(`/tmp/nyon-recovery-vectors.log`, exit 0). Focused strict Clippy initially found
an owned test clone-on-Copy and, after its repair, passed
(`/tmp/nyon-recovery-journey-clippy2.log`, exit 0). Full workspace/native/wasm gates,
live UI, IndexedDB and hardware qualification remain separate evidence.
