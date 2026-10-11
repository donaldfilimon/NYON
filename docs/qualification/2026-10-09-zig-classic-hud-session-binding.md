# Proposed H3a accepted Session adapter binding

2026-10-09. Read-only preparation. Source hashes below are observed identities, not new implementation acceptance. Parent must compare with accepted manifests before dispatch. Existing H3a FIX1 binding/report remain untouched while their review runs. No product/compiler/tests/gate/dependency/assets/toolchain work. Source `zig/src/classic_controller.zig` does not exist; actual intent controller is classic_commands.zig, advisory controller separate. No path/API was invented to cover the failed lookup.

## Exact admitted immutable data mapping

| ProjectionSource field | Actual source retrieval and ownership |
|---|---|
| mode | Session.core.mode():AppMode; playing/settings admitted, editing_scenario returns typed WrongProjectionMode, no editor projection |
| logical viewport | core.viewport():math.Vec2; source finite component max1/nonfinite1x1 policy in builder, no physical conversion |
| preferences | core.preferences():UserPreferencesV1 value; ui_scale,motion,high_contrast,graphics_quality,onboarding_completed; presentation() is only presentation subset, not substitute |
| speed/paused | core.speed():GameSpeed; multiplier():u8, paused iff multiplier0; no independent pause boolean source |
| selected field/help | core.selected_field/core.help_visible are named public struct fields, no getter exists; underscore fields are internal and must not be read |
| selected/source/target names | commands.commandTray(&session.core):Tray owns optional WorldName values, not heap names; commands.commandTarget chooses hovered_world orelse touch_command_target; by-ID scan of campaign().worlds preserves names, no assumed array indexing by ID |
| launch enabled/strength/status | same Tray.launch_enabled,launch_strength:?Strength,status frozen literal; commands.previewContextualLaunch returns ?CommandRejection!CommandPreview; no new command enqueue |
| decrease/increase | commands.fieldControl(core,.decrease/.increase):FieldControl enabled and literal tooltip; no mutation |
| onboarding | session.onboarding public State field; step():?Step and progress():Progress; current1..6,total6; no public Session onboarding getter exists |
| source focus | session.controls(null):Controls fixed18 array, items() borrowed; session.focusIndex(items):?usize maps retained action/settings index; copy typed action by value while Controls local alive, not slice pointer |
| HUD action | session.hud_focus public field or controls[index].action after focusIndex; missing-current display null does not mutate hidden retained focus identity |
| Settings focus | session.settings.focusIndex():usize through public settings field, clamp/wrap owned by existing model |
| pointer | no pointer-position accessor or retained pointer in Session/AppCore; explicit caller-provided optional logical pointer only, immutable during prepare; not fabricated from camera |
| hovered name | core.hovered_world public field, scan core.campaign().worlds by ID, canonical WorldName copy; tooltip source fallback only if no control tooltip |
| recovery | session.recoveryFailure():?RecoveryFailure, call-borrowed pointer variants; synchronously map into fixed adapter scratch or static literal before prepare; never retain failure pointer |
| owner/revision | no existing Session retained TokenRef/context/revision accessors; caller supplies trusted borrowed CurrentStamp from actual owner; no Session generation invented |
| marker | **Excluded from first actual H3a slice**, controls(null); no marker world/action/bounds until accepted H3c selectable-safe-area binding |

These accessors/fields are Current source observations, read-only. CampaignView is value worlds[7] plus borrowed fleet slice; H3a only inspects world/name values and does not retain fleet slice. Session/core live owning structs are not copied. Only admitted field values and copied full strings escape into owned H3a Header. [classic_app.zig:84-122,141-144,225-280; classic_session.zig:35-65,266-299; classic_commands.zig:11-19,102-155; classic_onboarding.zig:5-18; classic_settings.zig:4-29]

AppCore underscore members explicitly internal. Minimal Proposed parent-owned seam if direct public nonunderscore fields are considered unsuitable: one read-only `Session.hudProjectionState()` returning value mode/preferences/viewport/speed/field/help/onboarding/typedfocus plus Tray and two FieldControl values, and optional hovered WorldName. No dynamic allocation, authority mutation, platform imports, event queue or source backreference. This is Proposed API, not callable today. Simpler first implementation uses existing public fields and functions inside H3a-owned adapter `projectionSource(session:*const Session,pointer:?Vec2,stamp:CurrentStamp)` and needs no Session API mutation. Parent must rule ownership of adapter file and choose one seam; no human approval is implied by this engineering routing decision.

## Complete recovery vocabulary and priority

Current recoveryFailure priority is general, then any resource lane, then tick, then preferences. Projection preserves this priority and does not iterate lower-priority hidden failures or clear any failure. `general.command` is DispatchError=8 CommandRejection plus ModeRefused,OutOfMemory,MissingSource,MissingTarget. The8 rejection outputs use actual commands.rejectionLabel, identical Rust strings. Additional closed Proposed text: ModeRefused=`COMMAND UNAVAILABLE IN CURRENT MODE`; OutOfMemory=`COMMAND MEMORY ALLOCATION FAILED`; MissingSource=`SELECT A SOURCE WORLD`; MissingTarget=`SELECT A TARGET WORLD`. Existing commandResult does not install missing-source/target/mode failures into general_failure, but exhaustive type mapper covers them without asserting they are normally stored. [classic_session.zig:62-65,104-110; classic_commands.zig:134-156; src/app/core.rs:1021-1041]

`general.admission` exhaustive3: Validation=`SCENARIO VALIDATION FAILED`; InvalidInitialState=`SCENARIO INITIAL STATE INVALID`; StaleFingerprint=`SCENARIO FINGERPRINT IS STALE`. All three are Proposed deterministic ASCII, no Rust exact-string parity claim. [scenario.zig:35]

`resource` potentially has preferences:?Allocator.Error,editor:?anyerror,command:?Allocator.Error simultaneously. Output Proposed fixed composite in lane order preferences/editor/command, omit absent lanes, join exact ` | `: `PREFERENCE MEMORY ALLOCATION FAILED`; `EDITOR OPERATION FAILED`; `COMMAND MEMORY ALLOCATION FAILED`. Editor anyerror maps all possible values to that one closed message; never @errorName. Maximum all3 literal sum plus6 separators<=128 (literal length assertion required in mapper fixtures); fixed scratch[256] sufficient. This is deliberate Proposed category projection; original typed anyerror and raw storage detail remain in owning Session/Store and are not deleted, rewritten or purported copied in full. Parent must accept this category-only policy explicitly before dispatch, or choose full original detail with existing16384 complete-byte admission; do not silently discard detail under a source-parity claim. [classic_session.zig:19,62-65]

`tick` has cause allocator OutOfMemory and tick:u64. Proposed exact `SIMULATION MEMORY ALLOCATION FAILED AT TICK <decimal>` with full tick0..maxu64 decimal20, fixed prefix length<=64, total<=84. No fractional/f32 formatting. Does not call core.recover(); recover() currently clears failure and clock only, not implicit acknowledgement during projection. [classic_app.zig:51,280-282,318-323]

`preferences.codec` exhaustive4: too_large uses existing exact `preferences JSON exceeds the 16384-byte limit`; version uses existing exact `unsupported preferences format version`; ui_scale uses existing exact `unsupported UI scale percentage <u16 decimal>` bound5; json Proposed `invalid preferences JSON` without arbitrary @errorName. Existing messageAlloc appends errorName dynamically; do not invoke it from bounded H3a adapter. [classic_store.zig:626-646; preferences.zig:65-70]

`preferences.store` exhaustive5 FailureKind: io Proposed `PREFERENCE STORAGE FAILED`; oversized_slot existing format `preference storage slot exceeds the <u64 decimal>-byte limit` with full actual max_bytes checked usize on64-bit<=20; before_persist Proposed `PREFERENCE STORAGE FAILED BEFORE ATOMIC PERSIST`; browser Proposed `BROWSER PREFERENCE STORAGE FAILED`; unavailable_path Proposed `PREFERENCE STORAGE PATH UNAVAILABLE`. Record must be preferences for PreferencesFailure; inconsistent scenario record returns typed InvalidRecoveryRecord before publication rather than mislabeling it. Raw message/cause/cleanup_error remain in original owning StoreFailure; bounded category view explicitly excludes them. Actual private record.label() spelling is `preference` singular; adapter uses this frozen source literal rather than calling private label(). Verify exact oversized-slot parity in future literal fixture. [classic_store.zig:12-35,35-62,626-646]

Only top-priority recovery display selected per projection; same unchanged failure is retained each rebuild, no automatic once-only consumption. Announcements are separate Proposed occurrence state, advance only after successful transition publication; Rust emits empty announcements. Persistent failing operation remains until existing owner clears/replaces it, no invented ack action. `Session.restart` clears general_failure after success, editor closed/applied clears it, persist stages replacement preference failure; core.recover explicitly clears tick. Do not add clearing to prepare. [classic_session.zig:71-84,246-253,343-350; classic_app.zig:318-323]

All closed literal category messages above <=128, numeric formatted recovery<=128, composite<=128; source arbitrary recovery lane16384 remains available only for explicit complete caller strings. Adapter outputs use existing256 bounded formatter scratch plus admitted Header arena, no messageAlloc allocation. New scratch source values are call-local; Header preparation copies complete selected recovery before return. No queue/event/payload added, token retain remains one preparation Header handle in accepted transition slot ledger. First-slice marker exclusion lowers Playing maximum18 to17 controls and67 to64 nodes; retain common72-node/18-action caps as explicit3-node/one-action future slack, do not count nonexistent marker as Current.

## Sufficient pure-slice prerequisites and later integration

Pure semantic H3a can implement prepare/publish/freshness with explicit immutable ProjectionSource and caller stamps without real host queue or3I. Required before this pure dispatch: parent confirms frozen accepted H1 + accepted Session/commands/3G3 identities below, accepts category recovery mapping and parent/worker adapter ownership, freezes absent classic_ui.zig/tests and root export ownership, fresh independent review, Header/layout assertions in future implementation. Session-backed adapter qualification uses existing functions and value data with null marker, caller pointer/stamp; no host event accepted. H2a accepted metadata can be referenced; active H2b exports/layout/materialization remain pending. Pure retained publication must not claim sighted text rendering parity until H2b accepted/materialization coordinated atomicity is qualified.

Later3I owns actual source revision advancement/stale dispatch gating, queue layout/counts, holds/modal owner lifetimes, guide/help transition and host pointer feeds; parent must bind those actual seams before host integration, not hold pure semantic tests hostage to invented queue. H3c later owns marker projection and safe area. Category mapper must not import editor implementation or expand HUD to tactical/advisory f32 content; that surface remains separate. Contemporary future fixtures independently enumerate every recovery tag/subcase including anyerror category and simultaneous3 resource lanes, rejected inconsistent record; raw detail unchanged after projection and source poisoning; priority masking; all numeric0/max widths; no implicit recover/persist; canonical fingerprint/commands unchanged. These are future fixtures, not run results.

## Source identity freeze

| Observed source | SHA-256 |
|---|---|
| zig/src/classic_session.zig | `747b27b98b15030931f214507568a9ced462121f47591b59db674c9383fd9568` |
| zig/src/classic_app.zig | `2d418ba49cdfb85d2ffd8d172cefc563b1b4cf87bb601729eb4045f791e2f1be` |
| zig/src/classic_commands.zig | `2b148fe0962af83308718ab20d4d216cdd04b30d086a210167591e3a1dae932b` |
| zig/src/classic_onboarding.zig | `f534774a32e1c00b4986f846cadad1d73ca950cba1b34445466ae4e9d956971a` |
| zig/src/classic_settings.zig | `f66701119b9fb9870a39aa3dea225d9b705c8711dceaf8aa2ee3437a003efd11` |
| zig/src/classic_advisory_controller.zig | `1f75aae850e711b198aead65bb876b1c328a442d980cd9bef3bedd4169fae032` |
| zig/src/classic_advisory_projection.zig | `fb092241c606b9686e5dcc4c30bf561fcfa0ba7656e7f0d29053279da3964877` |
| zig/src/preferences.zig | `18d2d90d4e57baa0e75d126e947e5002450b4e17909ee877cf4d8bfbbb6bdce2` |
| zig/src/classic_store.zig | `1552a5fe97b76780b191f6357edf4cfe2366c368af9ace6ef0443c1c7081e7ea` |
| zig/src/scenario.zig | `2df5624269ba8b467280fb232edc335ddd45e91d0e76efc344f0c32bccb4e4d5` |
| zig/src/model.zig | `86db58f389d4b0b1e415a363b7e32c8694837f7bd23d4b3b9635cee66daed20e` |
| zig/src/simulation.zig | `7d52a2d00251ab356dd258194bec1e0798969305650fe3e94aeae27bc0eb17fe` |
| src/app/core.rs | `2ac51de5a8110c76c96c6665483812ea7869d9a33f05a4731d2abeb7c910befa` |
| src/presentation/ui.rs | `b38b0b4f0c78c5de6e807116239018351d55538e2c5f2faba355828744517ae6` |

Gaps: accepted-manifest comparison pending; category projection needs parent engineering ruling; actual H2b and later host/3I/H3c integration remain pending. No game completion/font/assets/runtime acceptance.
