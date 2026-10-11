# H3d guide dispatch correction

2026-10-09. Proposed additive correction, not product dispatch. Original source binding/report and independent review remain immutable. This document supersedes only I1 shared-budget accounting, I2 published navigation lifetime, I3 callable error/focus contracts and M1 raw scratch derivation from original binding. Root selected these engineering policies; fresh review and actual accepted API rebind still precede implementation.

## Trusted published navigation, without owning stamp

```zig
pub const Location = struct { topic: u8, sheet: u16 };
pub const GuidePublication = struct {
    revision: ?u64 = null,
    previous: ?Location = null,
    next: ?Location = null,
};
pub const PrepareError = H1.FrameError || error{
    InvalidGeometry, ArithmeticOverflow, InvalidOwner,
};
pub const PublishError = H1.FrameError || error{
    CurrentMismatch, BudgetMismatch, InvalidPublication,
};
pub const ActionError = error{
    InvalidUtf8, NoPublication, CurrentMismatch, UnknownAction,
    InactiveAction, InvalidPublication,
};
pub fn prepareGuide(allocator: Allocator, budget: *H1.PreparationBudget,
    input: Input) PrepareError!PreparedGuide;
pub fn publishGuide(slot: *H1.FrameSlot, publication: *GuidePublication,
    prepared: *const PreparedGuide, current: CurrentStamp) PublishError!void;
pub fn validateGuideAction(view: H1.FrameView,
    publication: *const GuidePublication, current: CurrentStamp,
    id: H1.ActionId) ActionError!Action;
```

Input and CurrentStamp retain original binding's call-borrowed contracts; this is no claim these proposed functions exist. H1.FrameError includes FocusError (OutOfMemory,LimitExceeded,InvalidUtf8,InvalidFocus,EmptyModal,ModalAlreadyOpen), TokenRetainError ReferenceOverflow and geometry/topology errors InvalidBounds,RootIsNotApplication,DuplicateNodeId,DuplicateActionId,ActionIsNotFocusable,InvalidTopology (frozen retained_semantics.zig:288,556). Budget.reserve LimitExceeded and retain ReferenceOverflow propagate unchanged. Original proposed PreparationLimit/RetainOverflow names are removed; no undisclosed remapping. All limits and checked-byte additions use LimitExceeded; ArithmeticOverflow only identifies viewport-to-count arithmetic outside those H1 resource counts. InvalidOwner is complete source owner admission rejection; CurrentMismatch identifies a captured/current ownership or revision disagreement, not automatic detection of caller mutation.

Caller allocates one GuidePublication in stable coordinator storage. Its lifetime spans displayed guide frames, survives PreparedGuide.deinit and contains no pointers, owning context, tokens, retained references or heap map. No current location is needed: only displayed Previous/Next exact normalized destinations are required. Header retains candidate previous/next until publication. After checking complete captured/current owner, revision, Header bounds, and identical stable budget pointer, publish constructs candidate whole record `{revision=current.revision,previous=prepared.previous,next=prepared.next}`. It calls synchronous slot.replace; ONLY after successful return assigns publication.*=candidate. Every failure leaves both old frame and every old metadata field unchanged; candidate Header remains caller-owned for retry/cancel. Same UI thread immutable caller lease prevents any observation between synchronous replacement and scalar-record assignment. No callback, host event queue or concurrent reader is established here.

Validation first validates event action UTF8, requires nonnull trusted publication.revision, matches current revision and actual view.owner's resident/draft identity,hold and full copied context, and requires current guide owner context. Then locate exact canonical static action and actual emitted frame node; require node.action exact equality,enabled=true,focusable=true and active guide modal ownership under later coordinator policy. Only after capability/freshness acceptance resolve guide.previous/next from trusted record; absent destination is InvalidPublication (never infer it from mutable Session state or event payload). guide.menu -> main_menu and guide.close -> close are static. Events supply action ID only, never a Location. Record is caller-trusted, cannot be replaced by arbitrary event metadata. A stale frame is rejected even if draft token remains same. Header destruction cannot invalidate any record field.

Coordinator clears entire publication to initial null value only after successful replacement by a non-guide frame; failed close/navigation preserves displayed guide and record. On slot destruction clear the record as well. It must not clear before destination commit. The publication belongs to slot coordinator, not Session private storage invented here. Future actual target measures @sizeOf(Location),@sizeOf(?Location),@sizeOf(?u64),@sizeOf(GuidePublication), asserting record<=64 bytes. Actual size, not a claimed8-byte optional revision, enters caller fixed-payload ledger. This fixed record has zero allocator request and no reference retain; ordinary stack/coordinator overhead remains unknown beyond the enumerated profile.

## Focus admission and independent raw q

Source build compares caller focused against each actually emitted control; unknown or absent action focuses none (`src/ui/guide.rs:193-224`). Proposed input validates focused UTF8 first: malformed -> InvalidUtf8 without modifying frame/source. Well-formed unknown or endpoint-omitted guide ID becomes null, preserving source equality behavior. Present known ID is selected from canonical module-static action identity and copied into Header descriptors by canonical identity, never retaining input slice bytes. A caller may end the Input.focused borrow after prepare returns. No normalization or prefix matching.

FocusSpec uses only base order in source slot order, modal=null,restore=null; full-screen guide's owner routing is coordinator policy, not an invented H1 nested-modal order. base<=4,focused-present<=1,raw q<=5 independently of unique<=4. H1 focus scratch<=5*16=80,DFS<=24*8=192,actual helper<=272. Original288 allowance remains explicit16-byte padding, not q4 derivation. Caller order is Header canonical array; no second caller order allocation. Future tests cover four controls focused,unknown,well-formed omitted endpoint,malformed input and post-input-lifetime publication (frozen retained_semantics.zig:367-375).

## Shared stable budget and every transition

Root scoped engineering ceiling1179648 is the system/editor shared PreparationBudget. HUD196608 remains a local qualification profile, not a second interchangeable budget for the same slot. Guide preparation receives exactly the same stable budget object used by FrameSlot; identity is preserved across every consumer transition. Proposed BudgetMismatch check is caller adapter comparison before replacing, not an existing H1 error. Never silently allocate a guide budget, auto-grow900000/98304, or free old displayed resident early. Actual caller with smaller budget may type-fail LimitExceeded and preserves old display/source. Original98304 proves only guide-to-guide arithmetic and is removed as a cross-consumer admission claim.

Guide local Header cap32768 and local candidate max_owned32768 remain separate from shared ceiling. Header one actual @sizeOf(Header) request remains frame_scratch with reserve-before-alloc/cancel-on-OOM/commit-exact/free-before-charge.release and checked resident/draft retain unwinds. Stable budget outlives all Header/frame allocations. H1 helper peaks source accepted records; no token allocation is hidden in these retained frame terms.

|Transition|Old resident|Destination candidate|Destination Header cap|DFS + raw focus allowance|Requested peak|Shared slack|
|---|---:|---:|---:|---:|---:|---:|
|Guide -> guide|24872|24872|32768|288|82800|1096848|
|HUD -> guide|74344|24872|32768|288|132272|1047376|
|Editor -> guide|394456|24872|32768|288|452384|727264|
|Guide -> HUD|24872|74344|36864|896|136976|1042672|
|Guide -> editor|24872|394456|294912|4896|719136|460512|

Guide conservative R24872 retains original padded byte proof; HUD74344/Header36864/helper896 from frozen H3a final dispatch binding, requested-accounting section; editor394456/Header294912/helper4896 from frozen H3b semantic binding numeric-admission section. These are reviewed proposed population maxima, not measurements of transient H3a or implemented H3b. Closing source Main menu eventually needs its destination frame's own complete resident/Header/helper bound: that consumer is not admitted by this table and remains an explicit integration prerequisite. Root's shared1179648 ruling does not prove an unbound Main menu consumer fits.

At most one preparation Header concurrently: deinit the old PreparedGuide after success or cancel before preparing another consumer. Published GuidePublication permits this without losing navigation. Old H1 resident remains in slot through destination prepare and replace failure. If a future caller intentionally keeps old Header during destination preparation, add its actual32768 upper bound to the relevant table (Guide->HUD169744,Guide->editor751904); such overlap is outside this first implementation lifetime contract and cannot be assumed free. Similarly failed Header retained for retry prevents another prepare until cancellation. Static10591 corpus is binary data, publication<=64 caller fixed storage separate, no new owning token registry. Full stack/call descriptor/allocator-bookkeeping envelope remains unknown and must be reported separately. PreparationBudget totals include all live resident/staged/header/DFS/focus charges, with measured actual request sizes; named caps cannot replace those ledgers.

## Narrow future ownership and fixtures

Only future pure classic_guide.zig plus tests and minimal root/README exports are contemplated, with no authority granted now. Session guide state, saved complete focus, guide owner context, root-held immutable lease, key/pointer/accessibility routing, Main menu destination ownership and whole advance/advisory suppression remain later3I/new-owner work. Existing Session exposes no source guide state API. Pure callers can qualify projection, trusted publication and rollback using explicit fixtures before host exists. Accepted H2/H3a product API rebind must be separately frozen; no transient implementation is adopted.

Mandatory future independent fixtures: publish pageA then destroy Header then validate Next returns A's stored destination, not mutable pageB; failed replacement preserves all three publication fields and backing identity; success updates all fields; endpoint omitted control rejects activation even with forged nonnull trusted-test destination; current owner/revision stale rejects before Show resolution; publication null after accepted leave; caller input focused backing destroyed; four-focus q5 actual80; each open/close transition ledger; old Header contract violation distinguished from normal peak; BudgetMismatch and actual900000 request type-failure when disallowed overlap; real/synthetic OOM every allocation ordinal and ReferenceOverflow retain unwinds. Actual target record/Header layouts and all-ordinal populations must be asserted, not compiler result guessed. No compiler/test/gate ran during this preparation. Fresh parent review remains required.
