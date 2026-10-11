# Proposed Task3I1 second chunk: normalized key and command edges

2026-10-09. PREPARED, NOT DISPATCHED. This second `classic_input.zig` / `classic_input_tests.zig` chunk follows independently accepted owner/revision/queue foundation implementation. It adds value-only key normalization, mapping and action-edge state. It does not redefine foundation ownership APIs, dispatch Session effects, implement guide/editor navigation, accumulate text, or deliver host events. `zig/src/classic_input.zig` is currently absent; no implementation is presumed. Fresh parent review precedes dispatch.

## Concrete proposed callable surface

```zig
pub const NamedKey = enum {
    f1, f4, tab, enter, space, escape,
    arrow_up, arrow_down, arrow_left, arrow_right,
    page_up, page_down, backspace, delete, home, end, other,
};
pub const LogicalKey = union(enum) {
    named: NamedKey, character: []const u8, unsupported: void,
};
pub const KeyState = enum { pressed, released };
pub const Modifiers = struct {
    shift: bool = false, control: bool = false,
    alt: bool = false, super: bool = false,
};
pub const NormalizedKeyEvent = struct {
    key: LogicalKey, state: KeyState, repeat: bool,
    modifiers: Modifiers, text: ?[]const u8 = null,
};
pub const NormalizedIme = union(enum) {
    preedit: []const u8, commit: []const u8,
};
pub const Action = enum {
    field1, field2, field3, decrease, increase, launch, pause,
    speed_down, speed_up, camera_reset, settings, restart, help,
    cancel, scenario,
};
pub const NavigationAction = enum {
    tab_forward, tab_backward, up, down, left, right,
    page_up, page_down, activate, escape, backspace, delete, home, end,
};
pub fn validateKeyEvent(event: NormalizedKeyEvent) error{InvalidUtf8}!void;
pub fn validateIme(event: NormalizedIme) error{InvalidUtf8}!void;
pub fn actionForKey(key: LogicalKey) ?Action;
pub fn navigationForKey(key: LogicalKey, modifiers: Modifiers) ?NavigationAction;
pub fn editorPrintableText(text: []const u8) error{InvalidUtf8}!bool;
pub const InputState = struct {
    // Two fixed 15-action boolean sets; no retained event/text/key values.
    held: [15]bool = [_]bool{false} ** 15,
    pressed: [15]bool = [_]bool{false} ** 15,
    pub fn init() InputState;
    pub fn setAction(self: *InputState, action: Action, down: bool) void;
    pub fn isHeld(self: *const InputState, action: Action) bool;
    pub fn takePressedOnce(self: *InputState, action: Action) bool;
    pub fn clearTransient(self: *InputState) void;
    pub fn clearAll(self: *InputState) void;
    pub fn finishFrame(self: *InputState) void;
};
```

Names/layout above are Proposed engineering choices. Implementation asserts action count15 and uses exhaustive enum indices. No allocation, token retain, queue enqueue, owner stamp or platform import occurs in these helpers. Zig fields remain mutable by convention; callers use methods. Foundation declarations stay under their previously reviewed contracts.

Logical character is a complete borrowed UTF8 string, not one byte, Unicode scalar or physical key code. Host can supply multiple scalars, combining sequences or an unsupported logical category; do not truncate to its first codepoint. Named other and unsupported map to null. `validateKeyEvent` validates the entire character and optional text independently; `validateIme` validates complete payload. Validation failures precede any later router state/effect. Direct mapping helpers require already validated input; they perform exact ASCII comparisons and retain nothing. All borrowed slices remain valid only through the synchronous call. A future consumer which stores text must copy it under its own already reviewed ownership/admission contract. No queue3840/4096 capacity becomes an editor, character or IME limit.

Modifiers are the full currently observed modifier snapshot supplied by the caller; only shift changes Tab mapping. Classic command mappings do not suppress control/alt/super combinations or invent command-key clipboard behavior. `repeat` is preserved independently of pressed/released and held state; this layer neither discards repeats nor synthesizes releases. Host synthetic-event filtering, physical/logical conversion, IME enabled/disabled and preedit selection/caret ranges stay host concerns. The source Classic preedit handler ignores its range; this pure carrier intentionally forwards text only. Preedit and commit are distinct from optional key-event text; no automatic duplicate suppression or composition protocol is invented.

## Exact mapping and edge semantics

`src/app.rs:2520` maps the WHOLE character string after ASCII lowercase: `1/2/3`=>field1/2/3, `q/e`=>decrease/increase, `p`=>pause, `[/]`=>speed_down/up, `c`=>camera_reset, `s`=>settings, `r`=>restart, `h`=>help. ASCII upper/lower letters match; `qq`, accented/fullwidth letters, Unicode case-fold equivalents and character-space do not match. Named space=>launch, escape=>cancel, f4=>scenario; named enter has no gameplay Action. F1 is separate router precedence, never a mapped command edge.

`src/app.rs:2636` maps named Tab using shift to backward/forward; arrows=>direction, PageUp/Down=>page_up/down, Enter/Space=>activate, Escape/Backspace/Delete/Home/End to their corresponding navigation values. All other keys=>null. Mapping yields a value only; it does not decide whether a mode accepts that value.

`src/engine/input.rs:55`: setAction(down=true) sets held and creates one pressed bit ONLY on false=>true transition; down=false clears held ONLY. In particular release does not erase a pending pressed edge. `takePressedOnce` returns the pending bit then clears it, preserving held. Multiple presses/releases before consumption coalesce as a set, not a count. `clearTransient` clears every pressed bit but preserves held; `finishFrame` delegates to that. `clearAll` clears both sets. No raw key identity/reference counts are added: two logical aliases mapping to one Action share the source action-level held bit, and either release clears it. This reproduces source rather than silently upgrading it to per-physical-key state.

Repeat metadata does not gate setAction. In Playing's ordinary command path the source calls setAction then takePressed on every mapped event; repeated press while held cannot create another edge. A first observed repeat when held is false CAN create an edge (for example after clearAll); the source has no blanket `!repeat` guard there. F1/H use explicit nonrepeat precedence guards in the router instead. Navigation/text repeats can be accepted by their mode; do not apply gameplay edge suppression globally.

## Source-bound later router integration obligations

`src/app/input_router.rs:524–686` establishes this ordered protocol; I3 owns its implementation, not this chunk:

1. Pressed nonrepeat named F1 toggles guide before any mode/text/command handling and returns. Guide presence consumes all remaining key events; only pressed events with mapped navigation are forwarded (including repeats). Opening guide clears action held/pressed state as source `InputState::default()` does (`src/app.rs:1626–1642`).
2. Classic editor consumes releases. On press, present text with no Unicode control scalar and focusedAcceptsText is inserted BEFORE navigation and returns; repeat and modifiers do not add a typing veto. Even present empty text satisfies the source control test and consumes this branch when a text field accepts it. If absent, contains a control, or field does not accept text, try navigation then return. Control predicate is Unicode Cc: U+0000–001F and U+007F–009F; not byte<32 and not all format/combining characters. Validate UTF8 before scalar iteration.
3. Settings consumes all keys; pressed navigation only, repeats included. Backward/left/up=>previous, forward/right/down=>next, Activate=>focused control, Escape=>close; other navigation values are ignored (`src/app/input_router.rs:442`). No game command edge mutation through Settings/editor/guide branches.
4. Playing pressed nonrepeat whole ASCII-case-insensitive character H opens guide and returns. Pressed Tab moves HUD focus, including repeat; Enter/Space activates HUD only when the actual current focus index exists, including repeat. An unfocused Enter maps to no command; unfocused named Space becomes launch. Remaining mapped press/releases run setAction and takePressedOnce on press. Character H repeat falls through to the source Help action path rather than acquiring a new special repeat veto. Focus traversal includes disabled controls; activation filtering stays Session/consumer ownership (`zig/src/classic_session.zig:300`, `:322`).
5. Finish-frame clears transient edges only; host focus loss/suspend clears all (`src/app.rs:2134`, `:2410`). Do not clear held at every frame or mutate state on earlier consumed branches to compensate for lost release events. Later transitions requiring clearAll must be explicitly caller-bound to source.

Keyboard launch calls actual Session.keyboardAction(.launch), whose command implementation uses hovered_world, not contextual touch target (`zig/src/classic_commands.zig:147`). Other Action values are routing intents; they are not all accepted by commands.CommandAction, which has only launch/increase/decrease. I3 must map to actual typed Session/Core effects without inventing an all-actions Session signature. No Classic gamepad or mute action is added.

IME source path is separate (`src/app.rs:2243–2265`): no guide, Classic editor present => preedit updates composition or commit inserts and clears composition. It does not apply the key-text no-control test to commits. Confirmation rejection belongs editor. Stable Zig EditorState.setPreedit/commitIme validate UTF8 and preserve existing epoch on failed allocation (`zig/src/classic_editor.zig:167`, `:177`); this chunk does not import or call those methods, own composition, fake host support, or impose truncation. Later router invokes accepted editor APIs synchronously with complete data and typed errors.

## Independent literal qualification jury for later implementation

- Exhaust all15 Action values and all mapping rows, ASCII uppercase equivalents, named-space versus character-space, unfocused Enter null, F1 null, named other/unsupported null; `é`, `Ｑ`, `qq`, `q`+combining accent and multi-scalar input remain unmapped complete strings. Source buffers unchanged.
- Every named navigation row; Shift+Tab backward, unshifted Tab forward even with control/alt/super; arrows/Page/Home/End independent of modifiers. All character inputs return null navigation.
- Fresh false held/pressed; press=>held true/take true then false; duplicate down after consumed edge=>false; release=>held false/repress=>new true. Release BEFORE take preserves the pending edge; several press/release cycles without take coalesce to one. Distinct actions independent; aliases share one action bit. finishFrame/clearTransient preserves held and prevents new edge until release; clearAll resets held and allows a subsequent down, even if the carrier repeat flag is true.
- Full UTF8 and text validation: malformed continuation/overlong/truncated sequences rejected before state; valid combining/emoji/multiple scalars retained exactly at call boundary. Printable test accepts empty, ASCII space, combining scalar, emoji and format U+200D; rejects newline, NUL, DEL and U+0085 anywhere in otherwise printable text, with full-string scalar scan. Borrowed temporary poisoning after return cannot affect InputState because it stores only bits.
- Synthetic I3 precedence witnesses: F1 press nonrepeat beats printable text in editor, repeat/release do not toggle; guide consumes mapped gameplay keys/releases; editor printable Space inserts rather than navigation Activate, control-containing text falls through to navigation, empty-present text consumes while absent text permits navigation; Settings arrows wrap and pressed repeats navigate; Playing Tab repeats navigate, focused Enter/Space activates, disabled activation stays consumer policy, unfocused Enter no-op versus named Space one launch edge; H nonrepeat opens guide while repeated H follows ordinary mapping path. Command modifiers do not add a suppression rule.
- Separate IME carrier preserves a preedit/commit payload beyond semantic queue capacity and multi-byte boundaries exactly; this tests forwarding/borrowing only. Later actual editor fixture confirms commit clears preedit, confirmation refusal and allocation-failure preservation. No keyboard/IME duplicate suppression fixture is presented as source behavior.

Pure mapping/edge fixtures qualify this second chunk only. Router/editor/guide and host witnesses stay unexecuted later obligations. Parent must freeze the accepted first-chunk APIs/source and ensure no ownership collision before dispatch; no gate or compiler runs during preparation. Queue8×4096 remains Proposed, with no host delivery/drop/backpressure disposition.

## Frozen read-only identities

|Source/document|SHA256|
|---|---|
|task-3i-routing-intake.md|d67931577d8a9d0a3174748e8f844a4aec26649e79642e027d2de42a5319be5e|
|task-3i-routing-brief.md|d2a5702b16fbbc3bf8145d94dbb011ab770ade972c9376483222f6ef99481e7e|
|task-3i-routing-brief-fix1-review.md|2dab0d66b560a5adf8440c8fe2ca9522eb2916485fffcae11961ca16b6e13816|
|src/engine/input.rs|f092eb304bc44c121fe1d101a5c814b5dd1d92450820fd524ec1cbb5ad390566|
|src/app/input_router.rs|966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a|
|src/app.rs|67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2|
|src/editor.rs|069c2325e7db1c92529045740885b143892f4217ae26063e62ecae8ec7775439|
|zig/src/classic_commands.zig|2b148fe0962af83308718ab20d4d216cdd04b30d086a210167591e3a1dae932b|
|zig/src/classic_session.zig|747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568|
|zig/src/classic_editor.zig|aea58f15ff8e5a4291e9e8b3692ca6068813fb333525fe29e157f073ac2de26a|
