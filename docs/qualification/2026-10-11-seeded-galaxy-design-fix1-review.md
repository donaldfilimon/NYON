# Seeded galaxy design: scoped fix 1 review

Date: 2026-10-11. Verdict: **F1 and F2 addressed; proceed to detailed planning**.
No new Important or Critical regression found in the changed contracts.

Reviewed exact specification SHA256:
`5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d`.
This supersedes the two blocking findings for this revision only. The original
`2026-10-11-seeded-galaxy-design-review.md` remains unchanged as raw evidence.

## F1 — Addressed

Specification lines **135–166** now assign the lane and checked job/generation
counters to a persistent session-owned Setup. Tokens carry an owner namespace;
the caller must provide distinct, never-reused namespaces across the session's
controllers and recreation. This is an explicit caller contract. The pure
module validates equality; it does not claim to discover process-wide uniqueness.
Calls require live controller storage, and closed controllers return Shutdown.

Ready retains the lane; take/release reopen it. Taken/released/foreign tokens
return StaleJob. Shutdown and detached Manifest destruction have separate owned
allocation responsibilities and bounded cleanup. Allocation failure can consume
reserved counters without admitting a job, which prevents accidental reuse.

Lines **178–190** distinguish the candidate's 131,072-byte ceiling from a detached
65,536-byte resident. The replacement peak is explicitly **196,608 requested
bytes**; after take, at most two detached outputs total 131,072 bytes until the
superseded resident is destroyed. The session must perform that destruction
before admitting another candidate. This resolves the original retained-output
accounting gap without pretending arbitrary callers' extra controllers are free.

## F2 — Addressed

Lines **168–176, 195–198** separate create preallocation config validation from
decode's length admission, allocation and owned snapshot. Decoder schema and
recomputation work occur during poll. The former requirement to validate decoded
config before its input allocation no longer applies.

The complete admission snapshot is an explicit synchronous exception capped at
16,384 bytes, with no decoding/hashing/recomputation in that copy. Its byte count
and latency must be reported separately. No borrowed input survives begin, while
poll retains incremental validation/hash charges and zero-allocation obligations.

## Nonblocking precision for the detailed plan

- Freeze the full admission precedence among Shutdown, Busy, excessive input
  length and IdExhausted, and name the excessive-length error. Lines 150–151
  already require Busy before counter reservation; the remaining combinations
  should have explicit tests. This is error reporting precision, not evidence of
  authority mutation or unbounded work.
- Read line **222**, "Begin can reject only the byte ceiling immediately," as
  the input-validation rule. The detailed interface should say this explicitly:
  closed/busy/exhausted/allocation failures can also occur during begin.
- Express "existing state" in line **200** as resident authority/manifest and
  admitted lane state, consistent with the deliberate consumed-counter rule at
  lines 149–150. Counter consumption on allocation failure is already explicit.

These do not require another human approval pause. The current delegation record
at `2026-10-11-delegated-game-design-authority.md:5–17` permits controller design
and plan decisions while retaining written artifacts and independent review.
This review does not represent Donald personally reviewing this artifact.

## Proof obligations retained

The implementation plan must include owner/lane/token reuse and exhaustion,
controller recreation, maximum replacement memory, every allocation/cancellation
cut point, caller input mutation, admission copy cost, and exact malformed-wire
failure witnesses. The checked-in independent oracle and literal fixtures,
seed-max/boundary cases, budget/build-mode equivalence and exact v0 bridge
equivalence remain mandatory. The earlier reference-wire calculation is not an
implementation result and was not rerun in this scoped contract review.

Large metadata generation still cannot establish full-scale playable authority.
The integrated large-galaxy storage/coordinator design, unified clock/history,
mandatory cities/combat/strategy/save systems and all final hosts remain required.
No implementation, runtime, hardware, loaded-memory, timing or delivery claim is
closed by this review.

## Review hygiene

The new report path was checked absent before creation. The reviewed hash was
confirmed from disk. Only this report was written; the raw review, specification,
product source, active coordinator ledger and other authors' work were preserved.
Report whitespace checks passed. No builds, tests, agents or commits were run.
