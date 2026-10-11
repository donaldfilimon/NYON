# Task 3I2 lifecycle correction scoped review

2026-10-09. Preparation-only re-review of independent finding M1 and newly introduced lifecycle statements. Original binding, report and review remain immutable.

## Verdict and disposition

**M1: CLOSED by the additive correction. Specification compliance: PASS for the scoped correction. Quality: PASS for the scoped correction.** No new findings. This closes the prior lifecycle wording defect; it does not qualify an implementation, approve a contact-capacity/host policy or broaden the original independent review.

## Source verification

- `task-3i2-interaction-lifecycle-fix.md:9` correctly restricts the direct guide replacement to Open while `player_guide` is absent (`src/app.rs:1628`, replacement at1640). Already-present Open is an empty arm; Show, Close and MainMenu do not directly cancel/replace interaction in their arms. The correction appropriately distinguishes separately wired transition effects from the local arm.
- `task-3i2-interaction-lifecycle-fix.md:10` correctly binds whole-controller cancellation to Focused(false) (`src/app.rs:2411`), without attributing cancellation to Focused(true).
- `task-3i2-interaction-lifecycle-fix.md:11` correctly describes the reviewed TouchPhase::Cancelled branch as cancelling the whole controller (`src/app.rs:2376`), without introducing per-ID cancellation or release gestures.
- `task-3i2-interaction-lifecycle-fix.md:12` correctly preserves contacts and pinch baseline through `suspended` (`src/app.rs:2130`); that callback clears input/frame/GPU state, and Classic `on_suspend` clears only clock debt (`src/app/core.rs:923`). It does not imply a later focus or cancellation event was delivered.
- `task-3i2-interaction-lifecycle-fix.md:13` correctly distinguishes separate Living drag/touch clearing on occlusion (`src/app.rs:2380`) from Classic interaction ownership. Generic reset attribution is removed; initialization and pure cancel remain separate explicit operations.

The proposed fixtures distinguish pure cancel, synthetic caller dispatch and actual host event ordering. The correction does not add cancellation policy to settings, restart, shell transition or frame rebuild pathways. No new source-path collision, admission rollback or gesture-semantic change is introduced by this additive override.

## Evidence and limits

Read the complete correction/report and relevant unchanged Rust guide, touch, focus, occlusion, suspend and Classic clock callbacks. Focused search confirmed the direct interaction default replacement at1640 and cancellations at2376/2411, separate from initialization at361. Read-only commands exited 0. Reverified correction/report, original binding/report/review and app/core source identities against their supplied hashes.

Correction SHA-256: `f175549001a2383a4548b933b05f24155a0f93e84724948f016ac670c1bb1db8`.

Correction report SHA-256: `54fe2582d4b7b8945a5dac15c0f6a57950eae4f6cc8362a26f83092d70863a95`.

Actual host lifecycle ordering/delivery and the proposed eight-contact storage/admission policy remain unqualified. No fixtures, compiler, tests or gate were run. Only this new review file was written; no product, Git, dependencies, assets, transient guide/root README or subagents were used or changed.
