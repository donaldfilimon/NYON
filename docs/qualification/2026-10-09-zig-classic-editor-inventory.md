# Task 3C detached editor seams

Date: 2026-10-09. **Current:** Rust/Zig source contracts and a standalone Rust parser characterization (exit 0). **Proposed:** headless Zig ownership/API boundaries. Only this report and authorized disposable parser scratch were written; no NYON product source, dependencies, build gates, profiles, commits or transactions were changed. Real widgets/navigation remain 3H; store/confirmation transactions remain 3D.

## Original, current draft and exact field catalog

Rust owns a detached ORIGINAL clone and a separate current draft, plus all field buffers, parse-error IDs, focus, section/world selection, IME preedit and confirmation captures (`src/editor.rs:138-183`). Saving/loading current draft does not redefine ORIGINAL or the active campaign; revert clones ORIGINAL (`src/editor.rs:473-479,549-552,616-618`; `tests/scenario_editor.rs:229-239`). Editor construction accepts a draft without validating it. Zig must permit semantic-invalid detached drafts and arbitrary draft names (`src/scenario/mod.rs:16-53`; `zig/src/scenario.zig:37-48,91-116`).

There are **90 text buffers = 13 globals + 7 × 11 world fields**, including **one read-only buffer**, hence 89 editable fields (`src/editor.rs:740-813`; `src/editor/layout.rs:385-429`). Do not equate all 90 buffers with currently rendered/focusable widgets.

| Exact ID(s), with `i=0..6` | Typed destination / parser |
| --- | --- |
| `scenario.seed` | u64, exactly 16 UTF-8 bytes then radix-16 parser |
| `rule.tick_hz` | u32 display only; no text assignment arm and disabled/nonfocusable widget |
| `rule.win_world_count` | u8 decimal |
| `rule.ai_period_ticks`, `rule.hazard_first_tick`, `rule.hazard_period_ticks`, `rule.hazard_duration_ticks`, `rule.maximum_energy`, `rule.maximum_defense`, `rule.minimum_launch`, `rule.field_raise_cost`, `rule.field_lower_refund`, `rule.base_fleet_speed` | u64 decimal |
| `world.i.name` | Unbounded Rust String assignment; any valid UTF-8 text is accepted into the draft before semantic validation |
| `world.i.x`, `world.i.y` | i32 decimal |
| `world.i.owner` | Exact `NEUTRAL`, `UNION`, `HELIX`, `CHOIR`; no trimming/case folding |
| `world.i.energy`, `world.i.defense`, `world.i.base_output`, `world.i.base_regeneration` | u64 decimal |
| `world.i.atmosphere`, `world.i.hydrosphere`, `world.i.topology` | u8 decimal |

Assignments/parser arms are `src/editor.rs:816-899`; concrete Rust types are `src/scenario/mod.rs:16-27`, `src/game/model.rs:150-162`; Zig equivalents are `zig/src/scenario.zig:37-48`, `zig/src/model.zig:81-94`. Seed formatting is uppercase width 16; integers use decimal formatting; owner formatting uses those four exact words (`src/editor.rs:740-810`).

`focus_widget` requires an exact matching currently built widget that is enabled and focusable; visibility is not required because focus reveal may scroll to it. Unknown, absent-from-current-section, wrong selected-world, read-only tick, or modal-disabled background IDs return UnknownWidget and preserve focus (`src/editor.rs:303-313`; `src/editor/layout.rs:150-186,216-222,385-449`; `tests/scenario_editor.rs:243-293`). A focused action/section/world control has no text buffer, so text edits are inert and `focused_accepts_text` is false (`src/editor.rs:287-300,360-369,418-423,433-437`).

Private `apply_world_text_value` splits dots, parses the world index as usize and reads only the first three components; it does not reject trailing components. This is **not** public widget-ID acceptance: the exact widget/buffer lookup prevents aliases such as `world.+0.energy` and `world.0.energy.extra` reaching that helper (`src/editor.rs:303-309,421-425,727-731,841-855`). Proposed 3C FieldId catalog should match only the canonical 90 IDs and never create a buffer for an unknown ID. A narrow `fieldText(id)` query returns null for unknown IDs; `focusField(id)` returns UnknownWidget unless the exact editable field belongs to the active section/selected world. Tick remains queryable but cannot receive focus. Full action/widget focus remains 3H.

## Parser characterization and semantic boundaries

Rust source uses `value.parse::<T>()` without trimming and seed uses `value.len()==16` followed by `u64::from_str_radix(value,16)` (`src/editor.rs:818-824,892-899`). Installed Rust stdlib source was absent; a disposable harness directly exercised those operations and String::pop using the checkout's live toolchain:

```text
rustc 1.101.0-nightly (1d81eb4ad 2026-10-07)
active override: nightly-2026-10-08-aarch64-apple-darwin
rustc /private/tmp/nyon-editor-parser.M0WT5f/parse.rs -o /private/tmp/nyon-editor-parser.M0WT5f/parse
/private/tmp/nyon-editor-parser.M0WT5f/parse > /private/tmp/nyon-editor-parser.M0WT5f/output.txt
combined compile/run command: exit 0
rustc /private/tmp/nyon-editor-parser.M0WT5f/zeroes.rs -o /private/tmp/nyon-editor-parser.M0WT5f/zeroes
/private/tmp/nyon-editor-parser.M0WT5f/zeroes
additional long-leading-zero assertions: exit 0
```

Harness seed function was exactly `if s.len()!=16 { Err("byte_length") } else { u64::from_str_radix(s,16).map_err(|_| "parse") }`; decimal probes called `parse::<u8/u64/i32>()`. These are parser measurements, not NYON integration-test results.

| Probe / class | Measured acceptance |
| --- | --- |
| `+0`, `00`, `01`, `+01` | All three decimal types accept; value 0 or 1 |
| Long leading-zero decimal ending in 1; negative equivalent | u8/u64/i32 accept unsigned spelling as 1; i32 accepts negative spelling as -1 (additional harness assertions) |
| `-0`, `-00`, `-1` | i32 accepts; u8/u64 reject, including unsigned negative zero |
| Empty or sign-only `+` | Reject |
| Leading/trailing ASCII space, tab/newline, `1_0`, `0x10`, fullwidth/Arabic digits | All decimal types reject |
| `255`, `256` | u8 accepts 255, rejects 256; wider types accept both |
| `18446744073709551615`, `18446744073709551616` | u64 accepts max, rejects max+1 |
| `2147483647`, `2147483648`, `-2147483648`, `-2147483649` | i32 accepts max/min, rejects adjacent overflow |
| `0123456789abcdef`, `0123456789ABCDEF` | Both seed strings accept, value 81985529216486895 |
| `+123456789abcdef`, `+00000000000000F` | **16-byte seed strings with plus + 15 hex digits accept**; latter is 15 |
| `FFFFFFFFFFFFFFFF`, lowercase equivalent, sixteen zeroes | Accept u64 max / zero |
| 15/17-byte seed, `0x` prefix, underscore, whitespace, negative seed | Reject; length is checked before radix parsing |
| Sixteen fullwidth zero characters | 48 bytes, rejected by byte length |

Use explicit ASCII grammar and checked destination arithmetic in Zig; do not blindly substitute canonical `scenario.parseSeed/parseU64`, which require uppercase seed and reject plus/leading-zero decimal spelling (`zig/src/scenario.zig:552-560`). Do not introduce a digit-count cap that rejects arbitrarily many leading zeroes accepted by Rust integer parsing. Numeric type overflow is a parse error; gameplay limits are later semantic validation.

For example u8 field level 255 parses and changes the typed draft, then fails the semantic 0..10 limit; coordinate -1/i32 max parse and fail the 0..10000 limit. Win-world count parses u8 then must be 2..7. Other rule relationships, caps, hazard/production/regeneration/fleet-speed arithmetic and initial-faction conditions remain the existing validator's responsibility (`zig/src/scenario.zig:288-364`). Draft names, including empty, lowercase, controls, Unicode or names over 24 bytes, must be retained; `WorldName.init`'s canonical restrictions belong to validation/admission, not text insertion (`src/editor.rs:856-859`; `zig/src/scenario.zig:318`; `zig/src/model.zig:11-29`). Zig input should explicitly require valid UTF-8 because Rust `&str` guarantees it; do not silently repair invalid bytes or impose a canonical-name cap on buffers/preedit.

## Typed values, errors, focus and IME

Failed parsing keeps the exact buffer and the prior typed value, and sets that field's parse-error bit; successful parsing updates the typed value and removes the bit (`src/editor.rs:727-736,892-899`). Every rebuild synchronizes **all non-error buffers** from typed values, so valid `+01` becomes `1`, valid signed negative zero becomes `0`, and lowercase/plus seed becomes uppercase 16-digit hex immediately. Invalid text survives section/focus/layout rebuilds (`src/editor.rs:719-724`; `src/editor/layout.rs:14-22`). Do not leave successful raw spelling permanently in the Zig buffer.

`isDirty = typed_draft_content != ORIGINAL_content || any_parse_error`; preedit/focus/modal/buffer spelling alone do not make it dirty. `validationReport()` describes only typed semantic validation and can be valid while invalid text exists. `canApply` requires both no parse errors and no semantic errors; warnings alone permit apply. Candidate extraction fails InvalidText before semantic validation (`src/editor.rs:222-238`; `zig/src/scenario.zig:175-193`). Layout merges semantic errors by mapped field ID, then overrides with exact `INVALID VALUE` for a parse error; warnings do not become field errors (`src/editor/layout.rs:154-184`; `src/editor.rs:902-917`).

Replace sets the entire focused buffer, insert appends at its end, and backspace removes the last **Unicode scalar**, not one byte or a grapheme cluster (`src/editor.rs:410-455`). Measured String::pop trace: `A🪐e◌́ → A🪐e → A🪐 → A → empty`; the combining mark is removed separately. There is no selection/caret-range model in current source.

Preedit is stored separately, does not alter typed draft/buffer/error flags, and is appended only to the focused widget's displayed value. Commit appends committed UTF-8 to the buffer and then clears preedit, including when no text field is focused (`src/editor.rs:241-260`; `src/editor/layout.rs:169-176`; `tests/scenario_editor.rs:125-159,296-312`). Section/world/focus changes do **not** clear preedit; do not invent that reset. Same-section selection is inert; a different section clears focus and resets scroll. Any in-range world selection, including selecting the same index, sets Worlds, changes the selected index, clears focus and resets scroll; out-of-range selection is inert (`src/editor.rs:262-280`).

3C may store those value transitions and exact field/action focus identities. Actual `focusFirst/focusNext`, widget order, disabled control policy, pointer hit testing, reverse wrapping, geometry, clipping, focus reveal and scrolling must come from real 3H widgets. A made-up 90-field tab order would not match Rust's section/world/action catalog (`src/editor.rs:316-408`; `tests/scenario_editor.rs:102-121,243-293`). Keep focused-text ownership available to later input routing so printable Space belongs to text before action activation (`src/app/input_router.rs:629-644`; `tests/scenario_editor.rs:147-175`).

## Allocation/lifetime seam

Proposed `EditorState` owns immutable `_original: ScenarioDraft`, a current epoch containing `_draft: ScenarioDraft`, `[90][]const u8` buffers, `[90]bool` parse flags and preedit, plus section/selected-world/focus value identities. Use one canonical FieldId catalog rather than maps and dynamically duplicated ID strings. Clone both drafts from an explicitly borrowed input; build all buffers before returning init. Equality must compare seed/rules/world numeric fields, ownership and **name bytes**, ignoring arena/allocator/pointer identity. Clones have distinct name allocations (`zig/src/scenario.zig:91-100`).

Use the current draft arena for the current epoch's names/buffers/preedit. For a fallible edit or current-draft replacement, stage a fresh epoch with a stable base allocator: clone/create candidate draft, allocate every resulting text/preedit slice, perform the parse/type update and normalization on the candidate, then swap and deinit the previous epoch only after all preparation succeeds. An invalid parse is still a successful edit committing raw text plus an error bit; OOM preserves every prior typed value/buffer/flag/focus/preedit/modal slot. Whole-epoch replacement bounds memory by immutable ORIGINAL plus current content and one candidate; it prevents unbounded accumulation of superseded names/text while allowing arbitrarily long live editable text. Read-only/value focus changes need no fresh epoch unless canonical buffer synchronization actually needs replacement.

Do not save `draft.arena.allocator()` in a field or use it as the child allocator of another independently movable owner: installed Zig master constructs `.ptr = arena`; moving the containing editor/candidate leaves that allocator context pointing to the old struct address. Obtain arena allocators transiently inside a method, and clone with the stable base allocator. Repeated `dupe` into one live arena followed by freeing old text does not solve retention: free ignores non-most-recent allocations (`/Users/donaldfilimon/.zvm/master/lib/std/heap/ArenaAllocator.zig:34-43,608-623`). `ScenarioDraft.clone`, `toDraft`, generation and regeneration already prepare owned replacements and unwind on failure; regeneration only protects its draft, so build replacement editor buffers before committing it to the resident editor (`zig/src/scenario.zig:79-108,215-219`).

Expose immutable DraftView values without an arena handle, with name slices borrowed from the owning draft; buffer/preedit slices expire on the next current-epoch mutation/replacement/deinit. ORIGINAL view names remain valid until editor deinit. ValidationReport is a value with inline issue text; call `issues()` on a retained report, not a temporary whose returned slice would escape (`zig/src/scenario.zig:160-193`). Candidate validation/extraction allocates nothing in the current Zig API, so do not invent OOM tests for that operation. A validated ScenarioV1 capture is a self-contained value; pending save must own encoded bytes independently of the mutable editor epoch (`zig/src/scenario.zig:111-116,208-219,236-250`; `src/editor.rs:582-607`). Never retain a pointer to EditorState/current draft/arena in an embedded controller or capture.

## Narrow modal contract for 3D

Keep one modal/capture owner composed by value with the editor, for example `EditorSession { editor: EditorState, modal: ModalState }`; controller calls pass `*EditorState` and `*ModalState` per call. Modal kinds are exactly Revert, FactoryDefaults, RegenerateFromSeed, LoadDirty, OverwriteSave, CancelDirty and ApplyAndRestart (`src/editor.rs:49-57`). 3D owns requests, stores, captures and disposal; it never copies focus/text/parse state. The value capsule can hold optional owned save bytes and optional validated ScenarioV1, with dismissal releasing/clearing both. There must be no second independent modal boolean or stored back-pointer.

3C mutation calls require the modal gate explicitly: active modal blocks section/world changes, insert, backspace, set-preedit and commit-IME. **Replace-focused-text is different:** if a modal is active, dismiss it and clear both captures, then return **without applying the replacement**, even with no focused field. Preserve focus, existing buffers, parse flags and preedit (`src/editor.rs:245-280,410-416,429-446,621-625`; `tests/scenario_editor.rs:328-344`). 3H disables all background focusable controls and supplies the two modal actions. 3D installs captures only after successful preparation, owns failed/consumed confirmation policy and revalidates/equality-checks apply capture; 3C must expose candidate extraction and atomic current-draft replacement, not execute load/save/apply/revert/default/regenerate requests itself (`src/editor/layout.rs:216-222,481-504`; `src/editor.rs:578-684`).

## Minimal meaningful 3C tests

1. Exact 90-entry/89-editable catalog and initial formatting; all seven worlds/11 suffixes; read-only tick and unknown/aliased/out-of-range IDs never create buffers, change focus or mutate typed state. Headless field focus enforces current section/selected world; leave real widget traversal tests for 3H.
2. Parser table above across concrete u8/u64/i32 fields and seed; include arbitrarily many leading zeroes, uppercase/lowercase/plus seed, owner exact-case checks, type overflow versus semantic limits. Successful spelling normalizes; invalid spelling persists while typed value stays unchanged.
3. Arbitrary names over 24 bytes and Unicode remain editable/typed, produce semantic errors without parse errors, and clone independently. Dirty/canApply/validation remain distinct; warning-only drafts can apply; repairing text can return to ORIGINAL equality/clean state.
4. UTF-8 replace/append/backspace, combining-mark versus emoji scalar removal, empty backspace, unfocused/action-focus no-op, invalid-byte admission preservation. IME preedit is visual-only, commit clears it, and section/world/focus changes preserve it.
5. Modal guards for all mutators, especially replacement dismissing captures without replacing text. Verify fields/focus/preedit unchanged, captured save bytes freed once, pending apply cleared and no stale payload returned; transaction outcomes remain 3D tests.
6. Inject every init/epoch edit/atomic draft replacement allocation failure: preserve original/current bytes, all buffers/flags, focus, section/world and preedit; clean candidate allocations; retry once; deinit without leaks. Repeated long-to-short name/preedit/buffer edits must not retain historical epochs. Move initialized editor/session before edits to expose saved allocator/back-pointer bugs.
7. Detached current-draft replacement resets buffers/parse flags only after preparation succeeds and preserves ORIGINAL. Candidate extraction rejects invalid text first. Keep active campaign/queue fingerprint unchanged throughout editor-only edits (`tests/scenario_editor.rs:125-159,229-239,328-360`; `tests/usability.rs:531-542`).
