# Recovery review Finding 3: what is already covered, what is untestable, and what the finding misnames

Date: 2026-09-08

Status: Proposal. This narrows and corrects the scope of Finding 3 in
`docs/superpowers/reviews/2026-09-04-workshop-recovery-review.md`, which carries
`Status: Open acceptance evidence`. It does not close it. The browser half of
that finding remains genuinely open.

Evidence base: every claim below was re-verified by hand against `main` on
2026-09-08. Where an automated survey reported something, it was re-checked
before being written here.

## The finding, and what has changed under it

Finding 3 says live browser IndexedDB recovery-promotion is untested: the
wasm-only `promote_recovered_slot` path is exercised only through a host-side
memory model, never the real IndexedDB transaction code. That top-level status
is correct and stays correct.

Four things about its detail are not.

### 1. Its own suggested test is already half-satisfied

Finding 3 asks for "an aborted/quota-denied promotion case proving the prior
reference and archive remain intact." That case exists at host-model level:
`tests/workshop_store_web.rs:138`,
`promotion_abort_and_quota_failure_preserve_head_and_exact_recovered_predecessor`.
It creates a generation, commits an authority-invalid head, injects an abort and
then a quota denial on the promote request, and asserts the head is unmoved and
the previous generation's bytes come back byte-exact.

The review never cites `tests/workshop_store_web.rs`. That file landed in
`fc38672`, after the review was written, so this is drift rather than an error —
but the finding now reads as though nothing covers the case. Only the browser
half is open.

### 2. "Conflict" is not a distinct failure mode

The finding lists "quota, abort, schema, unavailable storage, conflict, and
eviction," quoting the design spec. The code has exactly five IndexedDb variants
(`src/workshop/store.rs:219-227`): `IndexedDbUnavailable`,
`IndexedDbSchemaMismatch`, `IndexedDbTransactionAborted`, `IndexedDbQuotaDenied`,
`IndexedDbEvicted`. There is no conflict variant. A concurrency conflict surfaces
either as `StaleGeneration` when the head moves, or as `IndexedDbSchemaMismatch`
when the predecessor descriptor vanishes between the pre-read and the write
transaction — a conflict wearing a schema label.

Listing "conflict" alongside the others implies a sixth typed failure a browser
test could observe by name. It cannot.

### 3. "Schema change" describes code that does not exist

`INDEXED_DB_VERSION` is `1` (`src/workshop/store/web.rs:11`), the open path
rejects any other version outright, and the accepted design says unknown versions
are rejected rather than migrated. There is no migration code.

So "schema change" as an untested recovery behavior implies untested code. The
honest narrowing is **schema-mismatch rejection**, which is testable, rather than
schema migration, which does not exist and is deliberately not going to.

### 4. The larger gap the finding does not name

`map_js_error` (`src/workshop/store/web.rs:1463`) is the sole place any browser
failure becomes a typed variant, and it has **zero execution coverage in any
environment**. The host model never runs it — the model returns whichever variant
the test asked for, so it proves the store's reaction to a variant, never the
classification that produces one. Its catch-all converts every unrecognized
DOMException to `IndexedDbTransactionAborted`.

This matters most on the realistic quota path. A Chrome quota failure typically
does not throw synchronously: the `put` request succeeds, the request errors
later, and the transaction aborts. Whether that surfaces as `IndexedDbQuotaDenied`
or falls through to the catch-all depends on the DOMException carried on the
transaction, which nothing has ever observed.

## The host model asserts a strictly weaker contract, not a partial one

This is the distinction the finding blurs, and it is the reason no amount of
host-side work closes it.

The model proves one property: a failed mutation publishes nothing. The wasm
implementation holds that property *and* several the model does not implement and
therefore cannot fail — it re-reads and re-hashes the predecessor blob before
promoting, re-checks the compare-and-swap inside the write transaction, and
validates the JSON reference-record schema. The memory store finds the
predecessor by generation number and never re-reads its bytes, and its error
precedence differs: an archived slot with a bad recovered generation yields
`ArchivedSlot` on the host and `NoValidGeneration` in the browser.

Any interim claim should therefore read "atomicity contract shared with the
memory implementation," never "IndexedDB conformance."

## What can honestly be produced, and what cannot

**Without a browser.** Negative and property tests for the JSON reference-record
decode/validate layer and for `map_js_error` are reachable in principle, though
both are wasm-gated and private today, so exposing them is an implementation
decision rather than a free test. Broader model coverage of the promote
rejection paths — unknown slot, archived slot, stale generation — is free.

**With a browser, if a harness is built.** There is none: no Playwright, no
Puppeteer, no `wasm-bindgen-test`, and the sole dev-dependency is `pollster`. The
existing `node --eval` harness in `tests/workshop_web.rs` cannot be extended
here, because Node has no IndexedDB. The wasm also exposes only a `start`
entry point, so nothing can drive the store from script — a browser journey has
to drive the product UI and inject corruption through console IndexedDB writes.
Transaction abort, schema-mismatch rejection, and the pre-read versus
write-transaction race are all genuinely reachable that way, and the race is one
the host model structurally cannot reach.

**Not at all, on any machine, honestly.** Quota exhaustion cannot be induced from
page script; the DevTools quota control is manual, and if someone drives it that
is manually assisted evidence, not automation. Eviction cannot be induced —
deleting an archive row proves the code's response to a missing blob and must be
labelled as exactly that, never as observed eviction. Note the asymmetry worth
recording either way: a promote whose predecessor is missing at pre-read returns
`NoValidGeneration`, not `IndexedDbEvicted`.

## Two observations that are not defects but bear on the above

There is no `delete` anywhere in `src/workshop/store/web.rs`. Every commit and
every promotion writes a new archive row and truncates the reference list to two,
so dereferenced blobs stay in the object store permanently. The design says
preserve the last two valid generations and never mandates deletion, so this is
not a spec violation — but origin storage grows monotonically with commit count,
which bears directly on the quota question above.

`browser_shell_exposes_visible_semantic_recovery_status` in
`tests/workshop_web.rs` is misleadingly named for this purpose. It asserts the
markup of the graphics-startup live region, not the recovery screen. Citing it as
recovery evidence would be false.
