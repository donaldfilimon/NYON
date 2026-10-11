# H3b error-to-widget and pure layout binding

2026-10-09. Source-only prerequisite closure. Existing H3a/H3b documents and product files immutable. Rust private widget_id_for_issue is actual oracle, not a public API callable from Zig. New pure layout API below is Proposed and must be reviewed/assigned before implementation.

## Exact association of every validator family

Function behavior: rules.<rest> maps rule.<rest>; worlds[<index>]<suffix> splits once at first ], strips all leading dots from suffix, remaps position→x and fields→atmosphere, then emits world.<index>.<suffix>; everything else returns original field string. It does not validate index, catalog membership or numeric canonicality. Proposed typed adapter covers admitted validator schemas, rejects malformed crafted reports explicitly instead of inventing arbitrary field IDs. [src/editor.rs:902-918]

| Families (all26) | Source path | Widget key | Association |
|---|---|---|---|
| TickRate | rules.tick_hz | rule.tick_hz | Error retained even readonly |
| WinWorldCount | rules.win_world_count | rule.win_world_count | Error |
| AiPeriod | rules.ai_period_ticks | rule.ai_period_ticks | Error |
| HazardPeriod | rules.hazard_period_ticks | rule.hazard_period_ticks | Error |
| HazardDuration | rules.hazard_duration_ticks | rule.hazard_duration_ticks | Error |
| HazardBoundaryArithmetic | rules.hazard_first_tick | rule.hazard_first_tick | Error |
| EnergyCap | rules.maximum_energy | rule.maximum_energy | Error |
| DefenseCap | rules.maximum_defense | rule.maximum_defense | Error |
| MinimumLaunch | rules.minimum_launch | rule.minimum_launch | Error |
| FieldCostRelation | rules.field_raise_cost | rule.field_raise_cost | Error; no mirrored refund field error |
| FleetSpeedArithmetic | rules.base_fleet_speed | rule.base_fleet_speed | Error |
| WorldName | worlds[i].name | world.i.name | Error |
| WorldCoordinate | worlds[i].position | world.i.x | Error; y gets no shared error |
| WorldFieldLevel | worlds[i].fields | world.i.atmosphere | Error; hydrosphere/topology no mirrored error |
| WorldEnergy | worlds[i].energy | world.i.energy | Error |
| WorldDefense | worlds[i].defense | world.i.defense | Error |
| ProductionArithmetic | worlds[i].base_output | world.i.base_output | Error |
| RegenerationArithmetic | worlds[i].base_regeneration | world.i.base_regeneration | Error |
| DuplicatePosition | worlds[i].position | world.i.x | Error, first source Error wins over later mapped same key |
| DuplicateName | worlds[i].name | world.i.name | Warning, no field error |
| ClosePosition | worlds[i].position | world.i.x | Warning, no field error |
| MissingUnion | worlds | worlds | Error stored association but no matching field widget |
| InitialVictory | worlds | worlds | Error stored association but no matching field widget |
| MissingHelix | worlds | worlds | Warning, no field error |
| MissingChoir | worlds | worlds | Warning, no field error |
| NoLaunchableUnion | worlds | worlds | Warning, no field error |

Source algorithm iterates ordered report, inserts only Error severity with entry.or_insert: first mapped Error text wins. Then every parse-error ID overwrites with exact INVALID VALUE. Active field descriptor performs lookup by exact widget ID; unmatched global `worlds` issues stay present in Validation rows/status counts but never attach to arbitrary field/preview. An issue for unselected world maps its real field but does not attach selected-world field. Rules/worlds only show corresponding section; absence of field widget never deletes issue from complete Validation section. Source grouped coordinate/field errors are intentionally single targets x/atmosphere. No eager spreading to every related scalar. [src/editor/layout.rs:150-183; src/scenario/mod.rs:311-564]

Pure Proposed `fieldError(report,parseFlags,FieldId) ?IssueText` returns parse INVALID VALUE first, otherwise earliest Error with mapping equal requested source key; no allocation/owned error map required. It uses actual ordered report and selected typed ID. Rejected crafted path never silently associates. `FieldId.world.index` is u3 and may7: reject before slot access. Minimal EditorState.fieldHasParseError getter still Proposed/parent-owned, existing aggregate getter is insufficient for exact per-field override. Existing fieldText/preedit/current/section/selected_world/focus/scroll/canApply are available, `_current` remains internal. [classic_editor.zig:8-30,70-78,98-101,141-144,206-224]

## Exact pure layout contract

Proposed new classic_editor_layout.zig/tests owns `layout(LayoutInput,workspace) LayoutError!LayoutView`: immutable typed source section/world/focus/scroll/modal/canApply, viewport and issue count/store-status presence; output bounded114 widget descriptors and content_rect,maximum_scroll,clamped_scroll. Text is attached by later retained builder, not allocated or duplicated in geometric layout. Caller-owned descriptor workspace actual size asserted/charged by integration; no layout heap/Session mutation. Stable widget keys typed FieldId/section/world/footer/issue index/status/preview/confirmation, never generated user keys. Pure traversal/hit/reveal functions return typed focus or new scroll, caller owns mutation/reprojection. Descriptors can be generated directly in eventual Header workspace to avoid second heap snapshot; actual combined Header layout must count them before acceptance.

Viewport finite max(v,0), nonfinite0 for editor, unlike HUD1x1. Wide iff width>=1100. Header56,row44,gap8,min target44. Footer columns clamp(floor(width/150),1,7),rows ceil(7/columns),footer_height=min(rows*44,height),footer_top=max(height-footer_height,min(56,height)). Content starts min(56,footer_top). [src/editor/layout.rs:14-40,227-233]

Wide: navx=min16,width;navw=min208,max(width-navx,0); sections at y=content_top+i*52. Worlds choices start content_top+3*52+8, at52 intervals. Previewx=navx+navw+16;formw=min420,width;formx=max(width-formw-16,previewx);previeww=max(formx-previewx-16,0). Preview every section, height=max(footer_top-content_top,0). Content rect formx/content_top/formw/height. [layout.rs:40-88]

Compact: tabs x=i*width/3,width=max(width/3,min44,width),height=min44,max(footer_top-content_top,0); advance content_top=min(top+52,footer_top). Worlds choices cols clamp(floor(width/80),1,7),rows ceil7/cols,cellwidth=width/cols,max minimum source width, y=top+row*44; source choice height=min44,remaining at initial top (not per-row recomputation). Advance top by rows*44+8 capped footer; previewx16,width=max(width-32,0),height=min88,remaining; advance height+8 capped footer. Rules/Validation omit compact preview. Contentx=min16,width,w=max(width-32,0),height=max(footer_top-top,0). [layout.rs:89-147]

Fields order11/13/issue rows source; content_height=count*52,maximum_scroll=max(content_height-contenth,0); clamp scroll0..maximum. Field y=contenty+i*52-scroll,width=contentw,h44. Visible iff FULL row y>=contenty and y+44<=contenty+contenth; source does not additionally require positive width. Fields editable/focusable from descriptor; readonly tick_hz/validation false. Footer y=height-footer_height+row*44 (distinct from footer_top in tiny windows),x=col*cellwidth,w=max(cellwidth,min44,width),h=max(min44,height,min44,height) simplifying to min44,height; visible true even zero viewport. [layout.rs:150-212]

Sections/choices focusable iff rect.width>=44 AND height>=44, visible iff both positive. Preview visible iff both positive, nonfocusable. Modal disables all underlay flags then appends message+two buttons; retained mapping removes their node actions as accepted FIX1 adaptation. Modalw=min600,max(width-32,0),h=min176,maxheight0,center x/y max0; message h=max(h-44,0); buttons y=center_y+h-44 (can negative),w=max(w/2,min44,w),height=min44,h. Source buttons focusable true regardless visibility; enabled true. Bounds converted with accepted XYWH clamping, finite negative positions permitted. Do not claim minimum44 at arbitrary tiny viewport. [layout.rs:310-383,470-506]

Focus order all enabled AND focusable, regardless field visible. Missing current next first/previous last; wrap exact. Hit reverse widget order visible/enabled/focusable plus inclusive rect containment. Activation only Action kind; focusing field does not emit footer action. Reveal skips Actions/Status/Preview; if field above content, scroll=max(scroll-(contenty-widgety),0); else if bottom below content bottom,scroll=min(scroll+widgetbottom-contentbottom,maxscroll); rebuild. Source Confirmation message is nonfocusable; modal action focus need not reveal. Scroll finite delta clamp after addition; source Home0/Endmax, PageUp/-max(viewportheight,44),PageDown positive. Proposed checked finite intermediate failure preserves old scroll instead of introducing NaN corruption; this explicit engineering guard needs reviewed tests. [src/editor.rs:303-407,458-470,685-707]

## Small sequential ownership split and fixtures

First implement pure layout + exact issue mapping in NEW classic_editor_layout.zig/tests, parent narrowly owns/assigns readonly parse getter classic_editor.zig and root test imports; no transaction logic. Independent literals:1440x900 wide Worlds nav16/208,preview240/748,form1004/420,contenty56/h800,footer top856; first world choice y220; first fieldy56;11field scrollmax0. Rules13*52=676 fits800. Validation98*52=5096,maxscroll4296.960x600 compact Worlds tabs56..100,world row108..152,preview160..248,form256/h256,footer512,11field content572/maxscroll316.320x220 footer top56/content0, four footer rows begin44 (source footer y≠footer_top); first row source cellw160; compact controls height0 where source computes it. NaN/Inf0 viewport,1099/1100,scroll extremes, clipped fields reachable/reveal, reverse overlap hit, modal all7 reasons, disabled Apply capability/traversal distinction. Expected values handwritten from formulas, never product-generated.

Issue fixtures all26 families and path/schema mappings; first coordinate and later duplicate both map x, coordinate wins; Warning never wins; parse x overrides INVALID VALUE; grouped field anomaly only atmosphere; global worlds no matching field; readonly tick attaches error but no edit; wrong selected world no bleed; crafted index7 rejects before array slot. Source text/error buffers preserved by pure layout. No compiler/tests executed here.

Then retained editor semantics in NEW classic_editor_ui.zig/tests, after layout/helper accepted and actual H3a final APIs frozen: consumes exact descriptors/errors, maps complete Header/frames/modal capabilities and owner/revision lifetime, no host transaction dispatch. Parent rules1179648 budget and category diagnostics/getter/module ownership before dispatch. If layout adds descriptor arrays not present in prior Header, recompute exact Header size and shared peak; existing cap294912 cannot grow silently. H2/host input/rendering/stale recovery remain later integration. No transient H3a rebind.

## Source identities

- src/editor.rs: `069c2325e7db1c92529045740885b143892f4217ae26063e62ecae8ec7775439`
- src/editor/layout.rs: `568d1ca1bffe48a98b08c928263b7e0198aa197363d39945240d5df8fe3125a7`
- src/scenario/mod.rs: `66aa18d9938b53bbfb6ddb23067a164b686f380ec6a6d9d282d548c4fbf6cc85`
- zig/src/classic_editor.zig: `3c93ed1f9a1d526b4ba293c3b9e9a004ad1e41ddb59f7b617b9f98a29ba1d07c`
- zig/src/classic_editor_transactions.zig: `e4776d63c6f81c8d33371ecf5ee7db5ee59b85095226869f304ef2c0a20d3913`

Gaps: actual new API/target layouts/OOM evidence, parent scope/budget/category/getter rulings, accepted actual H3a rebind and later host/H2 integration. Source error-path mapping is now explicit; no product or whole-game acceptance.
