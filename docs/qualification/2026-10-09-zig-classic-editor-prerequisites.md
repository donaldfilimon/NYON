# H3b editor prerequisite source proof

2026-10-09. Read-only synthesis. Prior FIX1 binding2e1eeaf17c3f79f04c5b7a6f53085c2cb51772c2704f20c4dc9fe6c70d102edf and its preparation PASS remain unchanged. No transient H3a implementation read/rebound as accepted. This document proves source-callsite/literal bounds and proposes unresolved narrow adapter choices; no implementation/compiler/gate evidence.

## Validation:100 storage versus98 emitted-callsite bound

Both validators have10 preworld checks,7 independent world checks*7, at most6 duplicate-position+6 duplicate-name insertions,21 unordered close pairs,6 trailing checks:10+49+12+21+6=98. Zig static comment estimates9 per-world and100 capacity, but actual duplicate at world0 impossible because predecessor slice empty; correcting that two-slot overestimate yields98.100 storage is safety slack2, not possible issue count proof. Exact26 code-to-field/message families below exhaust every push site; dynamic formatting substitutes only a single index0..6 (duplicates reference0..5), never invalid user name. The corresponding Rust cases use same issue-code family, literal suffix/messages and evaluation order; Zig .err maps Rust Error, .warning maps Warning. [zig/src/scenario.zig:288-364; src/scenario/mod.rs:311-564]

| Code family | Full field schema | Exact message or index-range template |
|---|---|---|
|tick_rate|rules.tick_hz|tick rate must remain 60|
|win_world_count|rules.win_world_count|win world count must be between 2 and 7|
|ai_period|rules.ai_period_ticks|AI period must be nonzero|
|hazard_period|rules.hazard_period_ticks|hazard period must be nonzero|
|hazard_duration|rules.hazard_duration_ticks|hazard duration must be nonzero and no greater than its period|
|hazard_boundary_arithmetic|rules.hazard_first_tick|the first two hazard boundaries must fit u64|
|energy_cap|rules.maximum_energy|maximum energy must be nonzero|
|defense_cap|rules.maximum_defense|maximum defense must be nonzero|
|minimum_launch|rules.minimum_launch|minimum launch must be between 1 and half maximum energy|
|field_cost_relation|rules.field_raise_cost|refund must not exceed raise cost, and raise cost must fit the energy cap|
|missing_union|worlds|at least one world must belong to the Union|
|initial_victory|worlds|no faction may begin at the win threshold|
|missing_helix|worlds|no world belongs to the Helix|
|missing_choir|worlds|no world belongs to the Choir|
|no_launchable_union|worlds|the Union has no source able to launch|
|fleet_speed_arithmetic|rules.base_fleet_speed|worst-case hydrosphere arithmetic must fit|
|world_name|worlds[0..6].name|world name is not canonical uppercase ASCII|
|world_coordinate|worlds[0..6].position|world coordinates must be between 0 and 10000|
|world_field_level|worlds[0..6].fields|field levels must be between 0 and 10|
|world_energy|worlds[0..6].energy|starting energy exceeds its cap|
|world_defense|worlds[0..6].defense|starting defense exceeds its cap|
|production_arithmetic|worlds[0..6].base_output|worst-case production arithmetic does not fit|
|regeneration_arithmetic|worlds[0..6].base_regeneration|worst-case regeneration arithmetic does not fit|
|duplicate_position|worlds[1..6].position|world position duplicates worlds[0..5]|
|duplicate_name|worlds[1..6].name|world name duplicates worlds[0..5]|
|close_position|worlds[1..6].position|world is within 250 sectors of worlds[0..5]|

The literal maximum message is73 bytes, `refund must not exceed raise cost, and raise cost must fit the energy cap`; longest field `worlds[6].base_regeneration`27. All literals ASCII. Duplicate/close maximum after one decimal index substitution remains below73. Rule fields<=27; `rules.hazard_duration_ticks`26, longest world path27. Uppercase source rendering preserves ASCII byte length. Source code uppercase CamelCase max24 HazardBoundaryArithmetic; use explicit closed source-code-name table for Zig snake tags, never uppercase tagName to produce mismatching underscores. Severity ERROR5/WARNING7. Thus admitted source-produced reports always fit Zig Text48/Text96 and100 records; source validator output is not arbitrary externally supplied report, so this callsite proof justifies max98/27/73 for actual validate output. Public mutable crafted ValidationReport can violate98/27/73 while fitting physical100/48/96; adapter must check and reject typed ValidationProjectionLimit, preserve full report/source/frame, never truncate. [zig/src/scenario.zig:151-198,288-364; src/scenario/mod.rs:94-127,311-564]

98 is a conservative emission bound, not claimed attainable. Conditions conflict: duplicate positions prevent close-pair warning for those pairs; missing factions conflicts with some victory counts; production overflow formula may not reach arbitrary upper families under typed widths. No satisfying draft achieving98 was computed or claimed. Arithmetic proof permits tighter future bound but does not need it. Cross-language predicate correspondence checked: Zig u128 intermediates hold u64*15000/10000 and sums; Rust checked u128 never overflows these typed inputs; regen checked u64+1000 matches Zig explicit maxu64 check; fleet effective-speed u64 check matches Rust conversion. Both use most recent earlier duplicate index; Zig scan overwrites previous as index increases, Rust BTreeMap.insert replaces. Order10 rules→per-world7 then two duplicates→close pairs→six tails is identical. [same validator locations]

## Fixed copied-text envelope correction/proof

8192 remains sufficient; the prior generic114 widget envelope is replaced with disjoint conservative fixed-lane4226 bound. Every raw name/buffer/preedit/store detail remains a separately counted variable lane. Report issue copied-text27244 and per-active-field error copy1898 remain separate, not included below. Complete fixed fields have source scalar limits16hex seed,20u64,11i32,3u8 and owner7; no editor f32 text formatting. [classic_editor.zig:245-268; src/editor/layout.rs:235-279,340-458]

| Fixed lane, counting every projected copy | Bound bytes |
|---|---:|
| Up to34 nonissue widget labels including3 modal widgets, each actual static label<=32 copied primary+Text |2176|
| Header status complete section/error/warning formatter <=64, duplicated value |128|
| World-choice selected suffix7*11 and section SELECTED3*8, conservative rounding |140|
| Preview full numeric owner/value119 maximum, copied twice |238|
| Preview label fixed prefix<=64, copied twice; raw name separately |128|
| Confirmation message<=64, copied twice; separate from CONFIRM ACTION label |128|
| Clean store success max26 bytes, duplicated value |52|
| Root/container fixed names collectively<=64, copied at most twice |128|
| Active typed field formatted values13*20, copied twice (seed16/u8/i32/owner below20) |520|
| Clean validation NO ERRORS OR WARNINGS22, copied twice |44|
| Proposed orphan-preedit status fixed label<=16, copied twice |32|
| Proposed bounded store/action diagnostic outputs2 lanes*128*2 copies |512|
| Total fixed copied bytes |4226|

All static widget labels enumerated source table: SCENARIO EDITOR,STORAGE,WORLDS,RULES,VALIDATION,WORLD <digit>,SEED,TICK RATE,WIN WORLDS,AI PERIOD,HAZARD FIRST,HAZARD PERIOD,HAZARD DURATION,MAX ENERGY,MAX DEFENSE,MIN LAUNCH,RAISE COST,LOWER REFUND,BASE SPEED,NAME,X,Y,OWNER,ENERGY,DEFENSE,BASE OUTPUT,BASE REGEN,ATMOSPHERE,HYDROSPHERE,TOPOLOGY,REVERT,FACTORY DEFAULTS,REGENERATE FROM SEED,LOAD,SAVE,CANCEL,APPLY AND RESTART,CONFIRM ACTION,KEEP EDITING,CONFIRM. Max label20 REGENERATE FROM SEED or APPL... all<=32. Validation code labels excluded because counted separately in27244; preview variable label separated into fixed prefix+raw name; no name hidden in label allowance. [src/editor/layout.rs:24-32,235-279,281-519]

Preview119 exact formula: owner7 + ` | X `5+i32 eleven + ` Y `3+i32 eleven + ` | ENERGY `10+u64 twenty + ` | DEFENSE `11+u64 twenty + ` | FIELDS `10+threeu8 nine+two slash2 =119. Header status max section VALIDATION10+two3-byte separators+two2-byte counts+ERRORS7/WARNINGS9 suffix spacing below64. Exact source uses ASCII uppercase for status.store and validation values, no Unicode case folding; complete raw UTF-8 byte count retained. World-choice ` | SELECTED`11. Every7 confirmation message literal listed prior FIX1 source binding fits64. [src/editor/layout.rs:251-277,340-383,452-519]

Corrected complete bounded subtotal27244+1898+4226+768 announcements=34136; output262144 leaves228008 for ALL repeated raw name/invalid-buffer/preedit/full-detail output. Retaining8192 as fixed admission allowance leaves original38102/residual224042 conservative slack3966. This proof does not authorize allocating additional formatting buffers: direct bounded Header arena writes and512+20 named scratch remain requirement. Raw valid typed buffer may already exceed minimal typed format if accepted source retains alternate spelling; actual fieldText is always counted exact, never replaced solely by this numeric bound. Fixed numeric520 is conservative extra allowance, not a cap/normalization of raw text.

## Diagnostic policy and parse accessor proposal

Actual Transactions.Status none(no row),no_saved_scenario exact NO SAVED SCENARIO17,loaded SCENARIO LOADED INTO DRAFT26,saved SCENARIO SAVED14,failure(codec/storage). Action Failure is independent and must retain its semantic Alert, not mask existing store row. Rust raw store_error.to_string is unbounded and ASCII-uppercase display. Proposed H3b policy, requiring parent engineering ruling: source success strings exact; closed failure category display<=128 while raw failure objects remain intact, following approved H3a category principle but not claiming that approval automatically extends H3b. [classic_editor_transactions.zig:7-18,58-76; src/editor.rs:528-616]

Closed mapping covers StoreStatusFailure.codec and Failure.codec: too_large=`SCENARIO JSON EXCEEDS THE 65536-BYTE LIMIT`; json=`INVALID SCENARIO JSON`; version=`UNSUPPORTED SCENARIO FORMAT VERSION`; canonical_number=`INVALID CANONICAL SCENARIO NUMBER`; validation=`SCENARIO VALIDATION FAILED`. Do not copy arbitrary canonical_number.field/kind into fixed formatter; retain original object/source and issue nodes, category view explicitly excludes those details. StoreStatusFailure.storage kind io=`SCENARIO STORAGE FAILED`,oversized_slot=`SCENARIO STORAGE SLOT EXCEEDS THE <usize decimal>-BYTE LIMIT`,before_persist=`SCENARIO STORAGE FAILED BEFORE ATOMIC PERSIST`,browser=`BROWSER SCENARIO STORAGE FAILED`,unavailable_path=`SCENARIO STORAGE PATH UNAVAILABLE`; verify record=scenario else typed InvalidRecoveryRecord. Failure invalid_text=`INVALID TEXT`,confirmation_active=`CONFIRMATION ACTIVE`,no_confirmation=`NO CONFIRMATION`,stale_confirmation=`STALE CONFIRMATION`,validation=`SCENARIO VALIDATION FAILED`,admission three=`SCENARIO VALIDATION FAILED`/`SCENARIO INITIAL STATE INVALID`/`SCENARIO FINGERPRINT IS STALE`. Resource anyerror from Session remains separately approved H3a category owner, not overwritten here. No messageAlloc/errorName, no clearStatus/recover/dispatch in projection. If parent prefers complete raw error detail, use exact full-byte variable admission instead of categories;4226 bound still safely reserves512 but cannot pretend raw detail has128 cap. [scenario.zig:15-35; classic_store.zig:35-62; classic_editor_transactions.zig:7-18]

Minimal read-only getter Proposed exactly `EditorState.fieldHasParseError(self:*const EditorState,id:FieldId) error{InvalidField}!bool`: reject world.index>=7 (u3 can7), then return `_current.errors[id.slot()]`; global enum exhaustive valid. No epoch clone/mutation/allocator, no model dependency/new storage, unknown text IDs handled existing lookup before typed call. New product seam needs parent explicitly assign classic_editor.zig narrow getter and independent tests or provide separate accepted adapter ownership; current H3b worker proposed ownership cannot silently edit editor. Error association requires first Error validator issue per source widget_id_for_issue plus parse-error override INVALID VALUE; field-path mapping rules->rule and worlds[i]->world.i with fields group mapped to source exact child policy, must read complete widget_id_for_issue before implementing. [classic_editor.zig:8-30,70-78; src/editor/layout.rs:153-165]

Proposed NEW classic_editor_layout.zig owns pure geometry/widget intermediate/scroll/focus reveal and field ID map, classic_editor_layout_tests.zig independent literals. Separate retained classic_editor_ui.zig/tests owns stable Header/semantics/confirmation adaptation; parent must decide whether this two-module division or one combined owned module is dispatch scope before work. Existing classic_editor_layout absent; no API rebind guessed. Layout never synchronizes or edits epoch, source values already admitted. Actual H3a common Header/publication APIs await final acceptance; do not import transient classic_ui source during preparation.

## Resource implication and identity freeze

Existing Header cap294912/static arrays plus arena, proposed budget1179648 arithmetic unchanged: requested R394456+C394456+Header294912+DFS3840+focus1056=1088720; named532+8handle+8u64value=1089268.900000 undershoots requested peak188720; must typed-fail, cannot silently increase. Parent budget/category/ownership rulings pending. Full Header/Charge/static catalog footprint, ?u64/discriminator/padding/call-local/report stack and allocator overhead remain unmeasured outside named payload; future compiler/assertion/allocator receipts required.

| Observed source | SHA-256 |
|---|---|
|zig/src/scenario.zig|`2df5624269ba8b467280fb232edc335ddd45e91d0e76efc344f0c32bccb4e4d5`|
|zig/src/classic_editor.zig|`3c93ed1f9a1d526b4ba293c3b9e9a004ad1e41ddb59f7b617b9f98a29ba1d07c`|
|zig/src/classic_editor_transactions.zig|`e4776d63c6f81c8d33371ecf5ee7db5ee59b85095226869f304ef2c0a20d3913`|
|zig/src/classic_session.zig|`747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568`|
|zig/src/model.zig|`86db58f389d4b0b1e415a363b7e32c8694837f7bd23d4b3b9635cee66daed20e`|
|src/scenario/mod.rs|`66aa18d9938b53bbfb6ddb23067a164b686f380ec6a6d9d282d548c4fbf6cc85`|
|src/editor/layout.rs|`568d1ca1bffe48a98b08c928263b7e0198aa197363d39945240d5df8fe3125a7`|
|src/editor.rs|`069c2325e7db1c92529045740885b143892f4217ae26063e62ecae8ec7775439`|

Latest accepted H1b3/H2b manifest must bind exact Session hash at dispatch, never historical3E alone. Remaining blockers: parent budget/category/module/getter ownership rulings; fresh prerequisite review; accepted actual H3a API/source rebind; exact source error-to-widget path mapping; actual Header/static/local layout and OOM allocation evidence; H2 materialization/host stale-recovery/event integration separate. Source-callsite98/27/73 and conservative fixed8192 sufficiency are established by this current source synthesis, not executed product tests or proof98 attainable.
