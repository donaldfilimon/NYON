## Design Document Review: I3 contact-domain FIX1

### Summary
Specification **PASS** and quality **PASS at preparation level** for the concrete contact-domain protocol. Original Major1 is **addressed**: representation, admission, irreversible invalidation, stale/retry behavior, raw release drainage and retirement are now selected rather than deferred. No new critical/major finding; one minor lifecycle wording issue remains. This is scoped review, not root policy adoption, implementation acceptance or permission to claim whole C readiness.

2026-10-09. Only this assigned review was written. No compiler/test/fmt/gate, product/index/ledger/agent mutation, delegation or commit. A projection/guide scope remains unaffected; C still requires root disposition, frozen predecessors, actual implementation review and cumulative qualification.

### Issue 1: New cancel_all spelling is described as an existing accepted boundary
- **Severity**: minor
- **Section**: Whole cancel, FIX1 line66
- **Description**: “only existing accepted boundaries ... explicit normalized cancel_all” includes a spelling absent from the actual classic_input.zig and both accepted router bindings. The authoritative lifecycle correction defines absent-guide Open, Focused(false) and whole TouchPhase.Cancelled and explicitly removes generic reset as a source-parity meaning. Pure controller.cancel exists, but that does not establish an independent normalized cancel_all route or its authority. The rest of FIX1 correctly labels its new block/release policies Proposed for root disposition; this clause should be equally precise.
- **Suggestion**: State that cancel_all is a proposed local spelling for an already accepted whole-cancel event, naming that event, or remove the independent route. If a new externally callable cancellation policy is intended, label it Proposed and include its exact admission/dispatch scope in root disposition rather than presenting it as already accepted. No existing Input export should be invented.
- **Status**: open

### Original Major1 closure

The original finding was reread verbatim and matches FIX1 line7. FIX1 lines13–34 select eight scalar records: raw ID, active projected resident root ticket index/generation, optional projected draft ticket index/generation, canonical13-byte context, optional hold presence/value, occupied and latched inert state. This supplies the representation missing from original binding lines53/72/82. Hidden draft retention is explicitly distinct from projected draft identity. The 50 scalar-byte arithmetic is correct; <=64 each/<=512 aggregate are proposed ceilings, not executed layout measurements. There is no retained TokenRef, registry address, action slice or additional reference ledger consumption.

Actual SlotTicket fields and checked nonwrapping generation advance exist (classic_input.zig:27,40–47). reserve increments before successful admission, cancel frees without decrement, and reuse increments again. A scalar index/generation comparison within the pinned registry lifetime therefore detects retired root-slot reuse without accessing the old token. No reconstructed old ticket is checked/released and no private registry getter is assumed. Full teardown is required before coordinator/address reuse; no global ABA guarantee is claimed.

Lines38–42 define mandatory transaction invalidation before old-root retirement, including actual partial committed effects, while rejecting preadmission failures without invalidation. Inertness remains latched through A->B->A even if context later matches. Same-domain focus/camera/text/viewport revisions do not invalidate admission merely because revision advances; current publication/event revision freshness remains independently required. This resolves both cross-domain action replay and erroneous own-revision revocation from Major1.

Lines46–52 define one-to-one ID/count synchronization using public hasContact/contactCount, infallible association installation after actual begin and no sidecar writes on pure admission errors. The no-mixed-domain caller policy blocks ALL new admissions while any inert/mismatching contact remains; thus pure beginPinch cannot pair a new-domain touch with old-domain contacts. Existing old contacts may drain or be cleared by an independently authorized whole cancellation. A full live population reaches actual finite/duplicate/ContactLimit checks; no free sidecar slot is required to discover the actual ninth rejection. This is fixed capacity8; no generic zero-capacity coordinator branch is introduced. The pure generic zero-capacity behavior remains unchanged.

Lines56–64 select actual single-call update/end behavior. Unknown IDs and invariant mismatches are distinguished. Stale same-domain updates preserve pure state and can resume after same-domain retry; inert contacts never resume. No-authority end drains known contacts even under Stale, inertness, LeavingClassic or revision exhaustion, discards every gesture and retires its sidecar exactly once. Current end retires despite null gestures or later source/OOM error. This avoids ghost contacts without bypassing checked revision for Session/Core mutations. Typed consumed-release evidence remains distinct from gesture handling and actual Session/Commands results.

Actual interaction.end removes the contact first and, for pinch, clears baseline and converts surviving pinch owners to moved tap_only (classic_interaction.zig:137–160). FIX1 leaves those survivor records occupied/stamped and preserves inertness; it does not invent per-contact cancel or automatic pairing. Once old-domain contacts all drain, no old pure contact/baseline survives to couple with newly admitted touches. Subsequent same-domain pairing follows the real controller's beginPinch policy, including survivor handling. Nonfinite end still uses last-finite coordinates; nonfinite derived orbit retains pure progress while rejecting camera/source effects.

Suspend/occlusion preservation and irreversible scalar invalidation are compatible: invalidation changes dispatch authority without resetting contacts/baseline. Authorized whole cancel clears both structures synchronously. Terminal release drainage changes pure bookkeeping only; LeavingClassic remains inert for source effects, and teardown empties both structures before root/registry destruction. No extra refs, ownership sidecar, headroom consumption or global generation counter appears.

### Strengths
- Concrete transition/admission/update/end ordering closes the original representation and lifetime deferral without widening product file ownership or relying on private interaction state.
- Explicit no-mixed-domain policy resolves pure pinch leakage without broadening cancellation on source replacement; it is clearly Proposed for root adoption.
- Revision-exhaustion release-only exception is explicit and preserves source/revision state while releasing bounded contact occupancy.
- Required independent fixtures cover ticket reuse, root address reuse without old dereference, A->B->A inertness, identical action/new domain, own revision, post-admission OOM/retry, null and nonfinite release, pinch survivors, ID reuse, count synchronization, accepted lifecycle and storage/ref ceilings. They remain unexecuted obligations.

### Frozen identities and evidence limits

Fresh read/hash verification:

|Artifact|SHA256|
|---|---|
|FIX1|cdedbcfe1111be4618654ac4f96363b33f2e5028f2ce244604088a6b3858b324|
|Appended binding report|c56eb34cda56177930140a03efbde61d1ff936a9127f3612862669705d6abcaf|
|Original assigned review|e0b0df3370c98a5c82871f74f312ee8561b82e81036a15d61eaf7f8b21a718b2|
|classic_input.zig|9e02aaff2b09abbcacf5f0fa1912c4e25ca3803e7d9043ae0eb83fe8c6c91a45|
|classic_interaction.zig|1f47633662151e7bc4127682420854d19a9ebb21d2d7d86e0b3751d7dd2d4910|

Read FIX1/report/original finding and only the named source sections: SlotTicket/registry reserve/cancel/release, actual controller begin/queries/update/end/cancel, live TokenRef.sameIdentity and authoritative lifecycle correction. Read/hash commands exited0. Focused cancel_all retrieval exited1 (no matches in actual input or the two accepted router bindings); this is absence evidence, not a failed test. No intermediate UI setter source was read in this re-review. No layout/compiler/runtime/host/remote behavior is newly verified; original review's geometry assessment remains unchanged.
