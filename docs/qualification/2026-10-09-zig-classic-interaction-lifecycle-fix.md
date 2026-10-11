# Proposed Task3I2 additive caller lifecycle correction

2026-10-09. Preparation only; original binding/report/review remain immutable. Sole review M1 is corrected below. This document supersedes only the generic caller lifecycle sentence at `task-3i2-interaction-dispatch-binding.md:68`; pure gesture/cancel semantics, original fixtures and numeric proposals remain otherwise unchanged. Fresh scoped review required before dispatch.

## Exact source-bound override

Replace the original “Caller guide-open/reset/focus-loss/suspend clears controller under source rules” claim with this precise protocol:

- `src/app.rs:1628` GuideAction::Open **only when player_guide is absent** replaces `self.interaction` with `InteractionController::default()` at line1640 and resets InputState at1641. This clears every contact/baseline. Open while already present is the no-op Open arm at1661; Show, Close and MainMenu do not themselves cancel/replace interaction in their arms (`src/app.rs:1643`). Existing guide-open cancellation must not be broadened to every guide action or every rebuild.
- `src/app.rs:2394` WindowEvent::Focused(false) cancels interaction at2411 after clearing input and Living drag/touch state. Focused(true) does not cancel interaction. This is a direct caller action, not a property of pure controller mode detection.
- `src/app.rs:2369` TouchPhase::Cancelled clears the **whole** Classic interaction controller at2376, independent of the canceled touch ID. It also clears separate Living contact/drag state. No per-contact cancellation or synthetic end action is introduced.
- `src/app.rs:2130` suspended callback clears InputState/frame/GPU resources and calls Classic on_suspend; it **preserves the app-owned interaction contacts and pinch baseline**. `src/app/core.rs:923` on_suspend only clears the Classic fixed clock. Do not cancel interaction as a source-parity implementation of suspend. A separate later Focused(false) or TouchPhase::Cancelled event can clear contacts under its own rule; suspend does not imply either event was delivered.
- `src/app.rs:2380` WindowEvent::Occluded(true) clears separate Living drag/touch state and invalidates frame/sandbox suspension state; it does not cancel Classic `self.interaction`. Occluded(false) does not cancel it either. Do not infer Classic cancellation from another controller's clear operation.

Audit evidence: focused `rg` across `src/app.rs` and `src/app/input_router.rs` finds exactly one direct default replacement at1640 and direct cancellations at2376 and2411. Generic “reset” has no standalone source-parity meaning here and is removed. Source default construction at App initialization is initial empty state, not a claim all restart, camera reset, settings changes, shell transitions or frame rebuilds clear existing contacts. Those pathways receive no new cancellation policy from this package. Pure `cancel()` still clears all contacts and baseline, repeat cancel remains harmless, and explicit caller initialization starts empty.

No guide product module, transient UI or root/README source was read. Above references are unchanged Rust app caller observations, not final future guide/host seam acceptance.

## Independent fixtures and evidence limits

- Pure controller fixture: begin a pair, establish baseline, cancel=>count0/no baseline; second cancel unchanged; subsequent end/update old IDs=>null. This qualifies pure cancel semantics only.
- Later synthetic caller fixture: guide Open with absent guide clears admitted pair/held keys; Open while already present, Show and Close have no additional cancel from their own branch. A MainMenu transition may have separately wired effects; do not infer its entire outcome from the guide arm alone.
- Later synthetic caller fixture: Focused(false) clears all Classic contacts; Focused(true) preserves them; one TouchPhase::Cancelled clears every contact and suppresses all later release gestures.
- Later source-parity caller fixture: suspend with admitted contacts preserves controller entries/owners/last positions/moved/baseline while clearing InputState and clock debt; occlusion likewise does not clear this Classic controller. No release action is fabricated by suspend/occlusion. A following actual focus-loss cancellation then clears contacts under that distinct event.
- Actual native/browser event ordering, missing releases, suspension/resume delivery and cancellation propagation require separate host lifecycle qualification. These fixtures are Proposed and unexecuted. Preserved contacts across suspend is verified source behavior, not a recommendation that a new host omit cancellation; changing it deliberately requires reviewed policy authority and cannot be labeled source parity.

## Boundaries and pending disposition

max_contacts=8 and original action/entry/controller size ceilings remain Proposed in this artifact; parent numeric disposition is pending separately. Source still has unbounded contact storage. This correction neither accepts a new touch/device policy nor chooses ContactLimit drop/backpressure/retry, admission versus focus side effects, raw-ID normalization, or finite-derived-output handling. The review records a separate parent queue-capacity ruling; this lifecycle fix changes no queue contract. Final3H/guide/router capability/source rebinding and host lifecycle acceptance remain later obligations.

## Frozen evidence

|Evidence|SHA256|
|---|---|
|task-3i2-interaction-dispatch-review.md|bbc608b44b22c7beea7bcf45355a26b602dcc807ffa5be678efa2ac3787ea190|
|task-3i2-interaction-dispatch-binding.md|378b2300aa874b3a16317999be49daf736ac54e213607e95f48121f9df93e768|
|task-3i2-interaction-dispatch-binding-report.md|9a052f05f4267f4001a972807a9f98f2fd4a7cea024a1654f05192191f7849f5|
|src/app.rs|67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2|
|src/app/core.rs|2ac51de5a8110c76c96c6665483812ea7869d9a33f05a4731d2abeb7c910befa|
