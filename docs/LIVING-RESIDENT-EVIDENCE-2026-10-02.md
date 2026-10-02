# Living resident staged-boundary integration

Status: Current resident integration; Partial whole-frame responsiveness, 2026-10-02.

The canonical main checkout retains its existing uncommitted work. This slice
adds resident ownership/polling in `src/sandbox_session.rs`, an exact-capture
freshness accessor on `LivingStepJobV2`, and a synchronous compatibility drain
for `ClientRuntime::step_sandbox`. Interactive Step now requests work; update
publishes it after preparation. Timing and transfer/guide tests await that
publication where they previously assumed synchronous UI action completion.

The four new resident regressions passed, alongside four matching existing unit
tests (`cargo test --lib resident_`: 8 passed). The complete final-source native
gate passed **1,415 tests across 101 result suites**, zero failures or ignored
tests. Source identities below remained unchanged through the remaining gates.

| Documented gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Exit 0 |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | Exit 0 |
| `cargo test --workspace --all-targets` | Exit 0; 1,415 passed |
| `cargo build --release` | Exit 0 |
| Separate fuzz workspace formatting | Exit 0 |
| Separate fuzz workspace locked all-target Clippy | Exit 0 |
| `./tools/build-web.sh` (WebGPU and WebGL2) | Exit 0 |
| Root wasm library check | Exit 0 |
| Workshop-core wasm check | Exit 0 |
| Root wasm library strict Clippy | Exit 0 |
| `./tools/check-workshop.sh` | Exit 0 |
| Independent original/additive V2 vector verification scripts | Both exit 0 |

The first full test attempts identified old synchronous-publication assumptions
in the visibility and player-guide tests. Those assertions now wait for staged
publication while retaining their exact timing/state expectations. The final
full gate above includes these corrections.

Parity compares complete V2 state, ordinary receipt, decision receipt, event-facts
receipt and encoded archive bytes through tick 55 at record budgets 1, 64 and
1024. The accepted-tail regression changes authority admission during private
preparation, then compares its restarted publication with the synchronous
reference. Cancellation preserves the committed bookmark and produces no cached
receipt or fault. Repeated Step inputs publish exactly one boundary; saturated
Run requests publish no more than twenty boundaries after elapsed-time debt is
dropped. Each update uses no more than 1024 counted step-job record visits.

Whole-frame responsiveness is Partial. The staged executor still explicitly
finishes due intents, delegated creator work, validation, sealing and journal
construction synchronously. Reclamation and presentation caches also remain
synchronous. The resident path limits this suffix to one boundary per update;
these operations are outside the counted record budget. The integration does
not establish the 5 ms target, allocator latency, capacity performance, live
native/browser behavior, persistence or accessibility acceptance. No commit,
push or PR was requested or performed.

## Combined-checkout handoff

The concurrently authorized completion chat owns validator/state-retirement work;
this slice owns resident integration and its publication-timing tests. Its
`validation-resident-retry` checkpoint independently passed the combined gates,
including vertex validation and all 17 additive vector modes. At handoff, all
**516 files** in its frozen source manifest matched current content; none changed.
The manifest SHA256 is
`c4182dfc115513ab088fafb855c0a311ee6bac2de95f6ceae90d6de3f0820727`.

The retained manifest and gate receipts are under
`.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/checkpoints/validation-resident-retry/`.
Future state-retirement or other source edits supersede these results for the
changed candidate. No locks were removed, other work reverted, or live browser
sessions started. V3 design and live acceptance remain separate from V2 gates.

## Verified source identities

```text
68d115cbce041b793a7b9d7ba01601e51a160785d980305e253ed6ae6c930a7f  src/sandbox_session.rs
098964971787165c6ddbf0d7127cf5a4c4a76450a53b3734503bbcfb332235ce  src/app/client_runtime.rs
966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a  src/app/input_router.rs
1bbd6141aa971984145d2ee48fdd38f567680361fc6ca247dc28f58708236795  src/app/client_runtime/living_notebook_jobs.rs
4fc8b921867809b561eb040b31141acc2ea2156f7f506af7e378c512516607a2  crates/nyon-workshop-core/src/living/simulation.rs
376e54ae85672d8ce1483906cada16ae789b30a25d9cd25b34f039d0d312a98d  tests/galaxy_sandbox.rs
cd952ebcc9dbffef7975aad9887a41ce0f1bf16eda5d9506d5b555bcf74bfa29  tests/living_runtime.rs
b64501e99a879b304af10c063664f8f0771f1c4505eeba26f45d6674a96c0d0a  tests/living_qualification.rs
a3c70c13e1c46f50e1e69b08f9b47a89af4671dd24ec55ddd64d6a863a43c39d  tests/player_guide.rs
```
