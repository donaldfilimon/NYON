# Additive Proposed H3b layout dispatch contract

2026-10-09. Resolves review3b02ea54e977b6fd9c1634a3cc0ac9cc57fda1e0d9029389d4077663cc9ca560 I1/I2 without changing original binding/report/review. Root engineering rulings now approve editor shared1179648/category-only diagnostics with raw source preserved/two-module layout then retained split/narrow parse getter. Implementation still awaits final H3a acceptance/rebind and fresh preparation review. No product/compiler/tests/gate actions.

## Concrete Proposed types

Imports accepted classic_editor as e, scenario as sc, retained_geometry.Rect. These NEW declarations are proposed, not existing callable APIs:

```zig
pub const LayoutError = error{ InvalidWorld, InvalidField, InvalidReport,
    InvalidIssuePath, InvalidFocus, InvalidScroll, ArithmeticOverflow,
    AliasedWorkspace, WorkspaceBusy };
pub const StatusKey = enum { header, store };
pub const Footer = enum { revert, factory_defaults, regenerate_from_seed,
    load, save, cancel, apply_and_restart };
pub const WidgetKey = union(enum) {
    status: StatusKey, section: e.Section, world_choice: u3,
    preview, field: e.FieldId, validation_clean, validation_issue: u8,
    footer: Footer, confirmation_message, dismiss_confirmation, confirm,
};
pub const WidgetKind = enum { status, action, preview, field, confirmation };
pub const FocusKey = union(enum) { field:e.FieldId, action:e.Action };
pub const Descriptor = struct {
    key:WidgetKey, kind:WidgetKind, bounds:Rect,
    enabled:bool, selected:bool, focusable:bool, visible:bool,
};
pub const Workspace = struct { descriptors:[114]Descriptor = undefined,
    borrowed:bool=false };
pub const LayoutInput = struct {
    viewport:[2]f32, section:e.Section, selected_world:u3,
    focused:?FocusKey, scroll:f32, modal:?e.ModalKind,
    can_apply:bool, has_store_status:bool, issue_count:usize,
};
pub const LayoutView = struct {
    widgets:[]const Descriptor, viewport:[2]f32,
    wide:bool, content_rect:Rect, maximum_scroll:f32, scroll:f32,
};
pub const IssueText = union(enum) {
    invalid_value, issue_index:u8,
};
pub const ParseFlags = [90]bool;
pub const FieldError = LayoutError;
```

`layout(input:LayoutInput,workspace:*Workspace) LayoutError!LayoutView`; `releaseView(workspace:*Workspace) void`. Workspace owns descriptors, LayoutView borrows them until releaseView or owner destruction; no owning copy or return of stack-local Workspace. Workspace.borrowed true forbids layout reuse (WorkspaceBusy) until release; contract misuse/double release rejects in all build modes through explicit contract guard. Borrowed view counts are exact, trailing undefined descriptors never read. Layout writes only after complete input/checked arithmetic/alias preflight; errors preserve workspace and borrowed flag, input immutable. All descriptor/key scalars contain no source text/report slices and no tokens. Input by value contains only scalars. No report/draft alias possible in layout itself; integration calls explicit `validateWorkspaceBorrowing(workspace:*const Workspace,report:*const sc.ValidationReport,flags:*const ParseFlags) LayoutError!void` before any layout write while report/flags are live; checked full byte ranges reject overlap with AliasedWorkspace. Read-only report/flags overlap each other is permitted. layout alone has scalar input and no report alias source.

Input selected_world<=6, issue_count<=98, finite scroll>=0 or InvalidScroll (clamp upper bound to computed maximum); viewport source sanitization finite max(v,0)/nonfinite0. Admit finite f32 intermediates or ArithmeticOverflow, no infinity→integer casts: derive footer/world column counts by finite clamped thresholds before cast (above7 thresholds yields7, below1 yields1), so arbitrary finite huge viewport cannot overflow usize conversion. Focus world/field index<=6; global enum is closed. A syntactically valid focus absent from current displayed layout is preserved as remembered identity, not InvalidFocus; InvalidFocus only malformed/out-of-range typed payload. Every count34/114 and checked multiplication/rect endpoint finite preflight before materialization. Pure geometric count<=114: max source Validation111+modal3. Full source arithmetic formulas/order from immutable error-widget binding are authoritative; no new layout policy except checked overflow rejection. [src/editor/layout.rs:14-233,281-519]

`validateReport(report:*const sc.ValidationReport) LayoutError!void` checks len<=98 AND len<=entries.len, each Text.len<=physical48/96 then logical path<=27/message<=73 and UTF-8, exact code/severity/schema. Rules/world path indices canonical decimal0..6; no leading zeros or malformed brackets, no extra suffix; global worlds accepted. Each source-code expected path schema must match26-family table in prior binding, preventing arbitrary enum/path pairing. Read entries by pointer, never `for (... ) |issue|` then escape issue.field.asSlice from copied local value. Validate source Text physical length BEFORE slicing. Actual source-produced validate output satisfies proved bounds; crafted100-entry report rejected whole.

`fieldError(report:*const sc.ValidationReport,flags:*const ParseFlags,id:e.FieldId) FieldError!?IssueText` validates id/world0..6 and full report. It writes nothing, so does not require an implicit Workspace parameter; integration uses the separate explicit alias preflight before layout writes. Parse flag[id.slot()] true returns .invalid_value; otherwise returns earliest Error issue_index whose exact typed mapping matches id. Warning ignored; worlds aggregate matches no field. Rule prefixes and grouped x/atmosphere behavior follow prior table, no mirrored error. Return union owns no slice and survives only as index referring to immutable report identity. `issueText(report:*const sc.ValidationReport,text:IssueText) LayoutError![]const u8` revalidates report, returns frozen literal INVALID VALUE or borrowed pointer directly to report.entries[index].message.asSlice(); caller keeps report alive/immutable until full copy, index check<count. Never retain slice after report/local wrapper expires. `mapIssueField(report:*const sc.ValidationReport,index:usize) LayoutError!?e.FieldId` validates full report and index; global worlds returns null. No leaking borrowed strings from local ScenarioIssue copies.

Narrow Proposed actual EditorState getter `fieldHasParseError(self:*const EditorState,id:FieldId) error{InvalidField}!bool`, checks world.index<=6 then reads `_current.errors[id.slot()]`; no allocation/epoch mutation. Read-only adapter calls it for90 typed catalog values into call-local flags, keeps flags/report alive for synchronous retained prepare copy. Getter edit scope explicitly approved root, future tests owned with layout worker; do not access underscore elsewhere. [classic_editor.zig:8-30,70-78]

## Callable focus/hit/scroll

```zig
pub fn first(view:LayoutView) ?FocusKey;
pub fn next(view:LayoutView,current:?FocusKey,reverse:bool) ?FocusKey;
pub fn hit(view:LayoutView,point:[2]f32) ?FocusKey;
pub fn actionAt(view:LayoutView,key:FocusKey) ?e.Action;
pub fn reveal(view:LayoutView,key:FocusKey) LayoutError!f32;
pub const Scroll = union(enum) { delta:f32, page_up, page_down, home, end };
pub fn scrollTo(view:LayoutView,operation:Scroll) LayoutError!f32;
```

All borrow view for call, no mutation/ownership. first/next use source enabled&&focusable, independent of visible; missing-current forward first/backward last, wraps. Descriptor.field→FocusKey.field; section/world/footer/modal action→exact e.Action. Hit reverse source descriptor order, visible/enabled/focusable with inclusive Rect.contains; nonfinite pointer returns null. actionAt requires actual corresponding descriptor enabled/focusable and action kind, returns no e.Action for editable field. reveal only fields, unchanged scroll for actions/status/preview; source upper/lower formulas return checked finite clamped scroll; unknown displayed key returns original scroll (source reveal no-op). scrollTo nonfinite delta returns original scroll (source scroll_by no-op), finite arithmetic overflow InvalidScroll preserving old, page viewport max44/home0/endmaximum. Source owner applies returned focus/scroll only through accepted EditorState.focusField/focusAction and explicitly assigned public scroll field, then rebuilds immutable layout and increments caller revision. Layout function itself never edits editor/Session/draft/transactions.

## Every nonfocusable action capability adaptation

Typed WidgetKey/static109 identity catalog always preserves source identity. Retained builder sets NodeSpec.action=null whenever source Descriptor.focusable=false, including nonmodal tiny sections/choices and modal disabled underlay. Keep enabled/visible/source roles/name/value/bounds exact. When focusable=true, actual retained action present even disabled Apply; source navigation filters enabled and activation rejects disabled. Field tick_hz/validation readonly have no action. Capability restoration occurs only by accepted current-source resize/layout reprojection, never mutating published H1 frame. validateAction requires actual present frame node action, enabled/focusable plus current revision/owner/modal permission; catalog identity alone grants no capability. H1 does not require remembered base FocusSpec IDs to correspond to current node actions, so base/restore identities can persist while incapable. [src/editor/layout.rs:214-220,310-351; frozen retained_semantics.zig:610-619,363-419]

Future integration fixture publishes complete zero/tiny nonmodal frame with source enabled true/visible flags but sections/world choices action=null, no ActionIsNotFocusable; activation rejects those IDs. Resize to960x600 fresh projection restores source focusable flag/capability, preserving source identity and owner/context/revision rules. Modal publication remains only2 action-bearing nodes; resize while modal never restores underlay actions. Disabled Apply nonmodal remains action-bearing/focusable yet traversal excludes and activation rejects. No source flag rewrite to satisfy H1.

## Descriptor storage and dispatch ownership

NEW layout module/tests first + narrow approved getter/editor test hunks; NEW classic_editor_ui/tests second after layout and actual H3a API acceptance. Root owns additive exports/README gate integration. No transaction/source-model changes. Retained Header embeds Workspace[114 descriptors] within existing cap; Proposed target assertion @sizeOf(Descriptor)<=96 and Workspace overhead<=16 permits114*96+16=10960. Prior Header envelope282296+10960=293256 <=294912, slack1656. Count Workspace descriptor storage once in Header actual @sizeOf allocation/Charge; no second standalone heap or copied descriptor array during retained prepare. Standalone pure tests may own local Workspace, full local stack separately measured. If actual target Descriptor/Workspace/Header exceeds asserted bound, dispatch policy fails review; no cap/budget growth. Static109 identity table target footprint separately reported, no allocator request claimed. Error report and flags call-local actual sizes remain ordinary local storage outside named payload.

Approved shared1179648 remains sufficient for previously conservatively capped Header294912: requested1088720/named1089268 formulas unchanged; actual Header measured lower may reduce request, never fake cap request.900000 lower budget intentionally typed-fails. Category diagnostics parent-approved for H3b with raw original objects preserved; no implicit clears/messageAlloc/errorName. Parent must freeze current accepted source manifest/getter paths and independent literal fixtures before dispatch. Exact new source/API identity cannot bind transient H3a, which remains pending acceptance.

## Frozen inputs

- task-3h3b-editor-error-widget-binding.md: `74cbff6c0d49726e62d711daa8c4d8c311126e1de8c3666c9049be83d2123974`
- task-3h3b-editor-error-widget-review.md: `3b02ea54e977b6fd9c1634a3cc0ac9cc57fda1e0d9029389d4077663cc9ca560`
- task-3h3b-editor-prerequisites.md: `74dffdfa1060d493693338bee540b7f901938ddb3d2ad679059f9cc3e6e80653`
- task-3h3b-editor-semantic-binding.md: `2e1eeaf17c3f79f04c5b7a6f53085c2cb51772c2704f20c4dc9fe6c70d102edf`

No execution proof: target layouts, readonly getter implementation, malformed report/alias/error preflight, literal layout114/counts, tiny semantic publish/resize/OOM, source Current/revision bindings and real host integration remain future qualification.
