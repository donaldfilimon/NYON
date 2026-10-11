# Task 4A independent finite-input publication review

2026-10-09. Verdict: PASS for the finite expected-input publication gate. Parent freeze decision remains Pending. This review approves no product implementation, runtime result, release qualification, or complete Workshop corpus.

Reviewed report SHA-256: `db9a896bb49f843d47bb71d47cf86bb7541a1af28d9409b51e5646755a579b85`.
Reviewed candidate manifest SHA-256: `ecc2f925e1f3c958dc3655a5bd6d8f4cd8593ad38b651716e8e093cba402e424`.

## Findings

Critical: none. Important: none. Minor: none.

Current: independently re-read all 24 payloads against the original source bytes selected by each locator. The 13 fenced cases select the named opening fence and following literal row; two state-boundary cases select the exact `STATE_PAYLOAD ID ` prefix; nine dynamic-view cases decode the exact `literal_dynamic_payload_hex ID ` row. No author extraction/verification script was executed or used as the oracle. IDs, lengths and hashes also match the corrected inclusion table. [task-4a-compatibility-manifest-proposal.md:48-78; docs/qualification/fixtures/2026-10-09-workshop-foundation-v1/manifest.json:227]

Current: all 18 preimages equal the literal 23-byte `NYON-WORKSHOP-STATE-V1` plus NUL domain followed by the directly retrieved payload; payload files exclude the display newline. All payload/preimage byte counts and SHA-256 values match the manifest. Nine descriptor files match their original inclusive line intervals byte for byte, preserving case names and formulas. Exact inventory is 53 files: 24 payloads, 18 preimages, nine descriptors, manifest and README, with no additional expected files. [manifest.json:227; manifest.json:711; task-4a-compatibility-manifest-proposal.md:80-93]

Current: all 31 manifest source/provenance bindings rehash to the bound byte counts and SHA-256. Parent ruling bytes and its preserved snapshot also rehash correctly; the ruling equals snapshot line244. This binds accepted scope and materialization authorization, not a prior freeze. [manifest.json:31; task-4a-freeze-evidence/parent-ledger-snapshot.md:244; task-4a-freeze-report.md:3]

Current classification is honest: raw schema and fragment representability remain distinct from handwritten valid construction/traced ticks; executed admission, receipts and replay are not claimed. JSON descriptors remain future consumers outside semantic-only initial4A. Constructor/ownership/allocation-failure/differential/Debug/ReleaseSafe witnesses and later namespaces/catalog/admission/runtime residuals remain unexecuted. One shared map-ordering path is a condition of this finite slice; independently ordered paths need additional reviewed literals. [manifest.json:5; manifest.json:261; manifest.json:334; manifest.json:727; manifest.json:885; manifest.json:895; manifest.json:902; README.md:5]

## Independent check evidence

Actual command: `python3 - <<'PY'` with a reviewer-authored inline Python stdlib check (`pathlib`, `json`, `hashlib`, `re`) run from the tool, exit0. It directly compared sources, original proposal table IDs/hashes, all payloads, preimages, descriptor intervals, 31 fingerprints, exact inventory and parent ruling snapshot. Output: `PASS: pinned manifest/report; 31 source bindings; 24 direct source payloads and unchanged IDs/table hashes; 18 domain-NUL preimages; 9 unchanged descriptor intervals; exact 53-file set; parent ruling snapshot line244`.

The first inline checker exited1 because my checker initially treated the proposal locator as a payload row rather than its named opening fence. Inspection showed `task-4a-independent-fixture-proposal.md:51` is ` ```STATE_EMPTY` (without the leading space shown here); the corrected checker asserts the named fence then retrieves the next row. This was a reviewer checker error, not a candidate defect. Both actual command/exit results remain in the tool transcript.

Unknown/unverified: historical action absence cannot be proven by this finite document check; no product code, build, test, dependencies, assets, commit or runtime execution was performed in this review. Source derivation semantic correctness relies on the already reviewed proposals and reviews; this fresh gate independently establishes publication fidelity, not a new implementation oracle. No claim of freeze approval is made.

Only this review file was written. Candidate and production sources were not changed.
