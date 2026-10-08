# Current Proposed empty-root fork history

This is an independently derived **Proposed FORK-HIST author corpus**. It extends
the independently admitted empty-root HIST-C journey by one separately accepted
fork and one paused edit. It does not implement an authority decoder or qualify
general history, production jobs, runtime imports, durability or the six expansion
gaps. V1/V2 and all accepted parent corpora remain unchanged. Human adoption of
the Proposed V3 mechanics remains absent.

From this directory:

```sh
python3 derive.py generate
python3 derive.py verify
```

Only the Python standard library is used. Neither command imports or executes
the CAT, GEN or parent HIST helper, or reads Rust serializer results as an oracle.
The admitted Catalog, empty Genesis and parent Archive/Notebook canonical bytes
are immutable dependencies. The parent history is replayed from its submitted
Revision commands/captures; its asserted States, Receipts and hashes are not
substituted for that replay.

## Exact checkpoint and source authority

`derive.py` pins and checks all dependency hashes before deriving and again
before writing/checking output. A changed input stops the command. Pins include:

- Current eight-document freeze manifest
  `d0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c`;
  coordinated current-source manifest2
  `bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f`.
  All eight live documents and their immutable copies must match the manifest.
- Complete schema `92a4a11240830d16ca6fd037979678fecc7dd6503b8f9a6d53774cd6765ba243`;
  integrated review `cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65`
  and Anchor rereview `947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf`.
- Accepted CAT five files and GEN six files, including corrected GEN helper
  `f7e4a4b0f09397c9fe67b7d8d38a871e82ae5f44aa9971dc6fc67fe4edb35751`
  and coverage `1a37cb6494f4c0faadebf2979ba346da7e746715329e0410f9d486667cf81f9a`.
  CAT review `d556d3230b34a167c997bbbfb58e6a6308e517d5fa006b23c806dfb6f0ded9f6`,
  GEN review `b7167d88507540d7f4b5a19ceda44fe9ecf0425944cd64b7c065e23f0eaec042`
  and GEN rereview `d993d81664fcb1b5f53bf55946a66065b26209959588c89f1ffeb36e423c0464`.
- Corrected HIST approved checkpoint2 manifest
  `e344d098fe6b0fe4752226ccfd22d31cea387bc3a47a9227301c08a5d240c046`.
  Every listed live file and its checkpoint copy is checked: eight parent files,
  original report, full review, correction report and targeted rereview.
  Full review `9ffc1653cff9f2fa63d26a4c27ff72dee6565356b00096c9a1356b29d04cbfa8`;
  targeted approval `389b39c54b7dcfc4a5611416a3b7544aa4bde9d36f23f1186c84f68a3aeca05a`.
- Fork preflight `10dfd1de4b44600848d81d6cd905a185ec8317d24710a8d146b45c452fd4ca60`.

The full machine-readable roster is in `predicate-coverage.json`; source line
citations below refer to the frozen complete schema and wire documents. Review
pins authenticate the dependency checkpoint; they do not approve this new corpus.

## Complete connected journey

The accepted empty Genesis contains all30 State fields, no entity allocation and
no initial Receipt. Parent paused S1 creates System Alpha; S2 edits it Beta;
ordinary boundary1 retains R2 and changes only tick. This empty authority has no
star/world/actor/facility/fleet/route/negotiation/combat work or ordinary facts.
Parent replay authenticates all three genuine original publications.

At retained root(R1,0), copy exact parent State P. Separately accept F1 from
document branch high-water0; accepted high-water remains2. Fork identity binds
original parent digest D1, while child logical State changes **only**
`branch_sequence=1`. Its `next_sequence=1` remains historical. No fork Revision,
Receipt, Event or Anchor is created. Child head is inherited root-owned R1;
cross-branch ancestry is required, not an invalid same-branch reference.

Prepare a paused child edit using published child(R1,0), child-adjusted digest
Df0, pending tail0 and phase1 pre-operation capture. Document-global next accepted
ordinal is3, despite the copied historical next_sequence1. Edit inherited System
to Gamma, with one exact Alpha→Gamma systems Change, operation_index0, empty
resources and passenger_delta string0. Set next_sequence3 after mutations;
branch_sequence1/tick0 persist. The inherited System ID retains its original
root/S1/op0/kind1/local1 creator origin. Child edit allocates no entity.

One direct creator_intervention, phase1/ordinal0/provenanceR3, is emitted. Seal
the Receipt's ID-free payload, then derive its Event ID. No cascade or additional
semantic/resource fact is invented. The complete Archive preserves root headR2/
tip1 and all original Revisions/Anchors, appends child headR3/tip0, keeps marks3/1
and selects child final tick0. The active tick need not equal maximum sibling tip.
All retained root and child tips replay before evidence acceptance.

The four genuine whole-Receipt Anchors authenticate root R1/t0, root R2/t0,
root R2/t1 and child R3/t0. Revisions sort by full32 ID, Branches by ID and Anchors
by the full declared key; chronology does not determine their input order.
Notebook preserves original root future Bookmark1 byte-for-byte, appends child
Bookmark2 with mark3 and reattaches to the exact new Archive integrity. Transfer
embeds exact Catalog/Archive/Notebook and a reachable selected view.

Alpha/Beta/Gamma and labels are fixture choices. Grammar-valid alternatives are
not rejected merely for differing from them. Empty Genesis and this ancestry
shape delimit coverage; richer supported configurations are not invalid wire.
[schema2355–2419,2422–2454,2460–2495,2497–2511,2581–2659,2692–2697]

## Independent frame formulas and exported values

`D(name)` is literal `NYON_LIVING_V3_` plus name plus one NUL. `H` is SHA256,
`C` canonical declared-order JSON, `L` u32le byte length plus bytes, and
`OptRevision` byte0 or byte1 plus32 bytes. Fixed integers are little endian.

```text
Fork = H(D(FORK_BRANCH) || root16 || parent16 || OptRevision(R1) ||
         tick0:u64 || parentD1:32 || ordinal1:u64), first16 bytes
Revision = H(D(REVISION) || root16 || branch16 || OptRevision(parent) ||
             tick:u64 || accepted:u64 || paused0:u8 ||
             L(C(capture)) || L(C(command))), all32 bytes
State = H(D(STATE) || C({format_version:3,rules_version:3,
                        catalog_hash:PACK,state:State}))
Receipt = H(D(RECEIPT) || C({tick,applied_revisions,
                            events:ID-free-payloads,state_digest}))
Event = H(D(EVENT) || Receipt32 || tick:u64 || phase:u16 || ordinal:u32),
        first16 bytes
Archive/Notebook/Transfer = H(their own D || C(complete record excluding
                                            only own final integrity member))
```

No VIEW domain or standalone HistoryView digest is introduced. `trace.json`
exports root, original creator issuance, every retained State/Revision/Receipt/
Event frame, fork-only State and fork frame, private acceptance/Envelope audit,
and all three outer preimages. Trace/private Envelope are audit data, not
portable fields or proof of a running host queue. Fork-only Bookmark authenticates
its wrapped State and can exist without Receipt evidence.

| Value | Hex |
| --- | --- |
| PACK | `9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8` |
| Root B0 | `4108c47943c15900dc896dc7661af625` |
| Inherited System | `a3247be8687b819f40bc4f251178dcba` |
| Full fork digest | `8a0d15991ea6fdf630109a6603b6aa4af79fffdccb3aaf74b9367e45861c1e11` |
| Child Bf | `8a0d15991ea6fdf630109a6603b6aa4a` |
| Parent D1 | `0bdc104397aff67312a7c49584acd282d18b80abf67e8befa93e861b04449c0a` |
| Child fork-only Df0 | `6433299e6111d22b8d7d989094f06a17137ca5a9766d3e4b74670463030a4ee0` |
| Child R3 | `7fcd12da54ef52dd6e10c94aba14a502afbb1696056db076bb4e33ba5aa91805` |
| Child final Df1 | `6c718de3a578a813915c2ef0821adc4f7ab545aeb894b86f96f76476dd595151` |
| Child Receipt Qf | `3d021f51c3aee43bb6aaa30181ca5bbea6b3e0c32877ab01816dd051ee214f03` |
| Child Event | `7adc528eecece00bc36f0511f7653848` |

The base Revision array's chronological ordinals are `[2,1,3]` in actual ID sort.
The distinction between parentD1 and childDf0 is verified from literal frames,
not assumed from a historical fork example.

## Cases and strength

`vectors.json` contains568 complete raw-document cases, with immutable attachment
Archive bytes where standalone Notebook admission requires context. Expectations
are specified first, then checked; the helper's private error ordering does not
create new diagnostic policy.

| Strength | Count | Meaning |
| --- | ---: | --- |
| complete_fork_history_admission |12| Three base documents; three Unicode Name/label documents; Transfer root-future/root-parent/fork-only selections; Notebook mark holes; extra fork-only Bookmark; Archive root-active selection. Complete admission of this selected family, including replayed attachment context. |
| exact_stage_error_path |520| 505 single required-member deletions,9 outer header constant faults,3 own integrity faults and3 descending input-array order faults. |
| exact_stage_error_only |3| Leading whitespace on complete Archive/Notebook/Transfer; stage6 noncanonical_bytes, no guessed containing path/offset. |
| rejection_only |33| Coherently resealed/re-framed replay, header, mark, Notebook or selected-view/evidence inconsistencies; no authoritative stage/error/path claim. |

Transfer's required `/catalog` deletion is covered. Catalog **interior**
`/catalog/…` deletions are excluded here because accepted CAT-C owns those
predicates; the containing Catalog member is never excluded by prefix accident.
Missing option fields are still required-field faults; null is a value, not
permission to omit a member. The declaration ledger covers380 paths and records
required fields, scalar/tag/key declarations and the bounded arrays relevant to
this narrow shape. It does not claim boundary-cap negative coverage for every
declared cap or unsupported collection.

Semantic negative examples include parent digest replaced with child digest,
wrong fork tick/cursor/ordinal, lost root future, R3 following root futureR2,
global S2 reuse, child capture reading original root digest or leaked sibling
next_sequence2, wrong phase/cursor/branch, false reviewed before/operation index/
target, wrong final State digest, mark regression, stale Notebook attachment,
Bookmark digest using parent/pre-edit State, unreachable child tick, forged
fork-only Anchor and embedded header faults. Fork and affected Revision IDs are
re-framed and outer seals updated without repairing the false input facts.

Eight additional trace-binding mutations cover wrong fork-only or final State
headers, invented fork Receipt, wrong child Receipt State/digest and wrong Event
ID. They assert only disagreement with independent exact replay, not portable
wire errors. They are outside the568 raw-document count.

## Admission ledger and explicit limitations

`predicate-coverage.json` records16 journey predicate groups plus source links,
pins,380 declaration paths, case strengths and unsupported matrices. Two in-memory
scope probes classify the already admitted parent with no fork and a later child
tip1 as unsupported coverage, without declaring either malformed wire. Some groups
describe derived private trace/Envelope construction; they do not add portable
admission members. Every exported positive has independently replayable complete
Archive context. Below-limit valid arrays exercise capacity admission; maximum
capacity, recursive identity scans and bounded work are not proven here.

The author helper materializes JSON recursively and buffers output. It is not a
production parser/job engine. It does not qualify UTF8 offsets, syntax versus
depth first-fault ordering, earliest duplicate decoded-member offsets,
stage7–10 multifault priority, bounded memory, visits, polls, cancellation or
publication ownership. Its internal Rejected fields for rejection-only cases
are not authoritative expected diagnostics. Unsupported-family results are
coverage classifications, never an invented wire error.

This family authenticates the shared inherited System creator origin. It does
not implement the general streaming retained-history identity scanner. Generic
Branch collision origin/tag and entity-versus-Branch namespace diagnostics remain
unbound in the frozen source; no new Branch allocation kind or synthetic
cross-namespace collision result is inferred. Combined fork/edit failure,
accepted abandoned queues, more branches, child ordinary evolution, later child
allocations, removals/cascades, inhabited bootstrap, experiments/Evidence and
native/browser durability remain outside this corpus. No initial Receipt,
portable queue, trusted decoded State/cache or verification flag is invented.

`generate` and `verify` are author-corpus checks. Independent review is required
before admitting this checkpoint; they are no evidence of production V3 support.
