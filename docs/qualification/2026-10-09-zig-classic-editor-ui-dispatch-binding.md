# H3b retained editor UI dispatch binding

2026-10-09. Proposed next sequential slice, no implementation authority. Frozen six-path pure layout/getter package matches current bytes and is NOW parent-accepted: independent review b68b58381af7b8b8eb89757ffe3e67900d961dd7278b2630cff97f635c566e57 PASS; existing parent gate terminal0/374Debug/67identities/no drift. Root/README final six-path baseline is now frozen. This preparation ran no gate. Parent rulings in ignored progress.md:modalnull adaptation,1179648 scoped budget,closed diagnostic categories,split layout/UI+narrow getter,tiny nonmodal nullactions read at lines380-396. H3a accepted actual UI APIs are identity-bound; no stale sourceabsence statement inherited from original preparation.

## Actual callable dependencies and source snapshots

Actual classic_editor_layout provides LayoutInput,Workspace,LayoutView,WidgetKey,FocusKey,ParseFlags,layout/releaseView/validateWorkspaceBorrowing/validateReport/fieldError/issueText/first/next/hit/actionAt/reveal/scrollTo. Workspace3196/Descriptor28 measured frozen package, not Header storage. ValidationReport16808/ParseFlags90 are real local/source data. Error union LayoutError has InvalidWorld,InvalidField,InvalidReport,InvalidIssuePath,InvalidFocus,InvalidScroll,ArithmeticOverflow,AliasedWorkspace,WorkspaceBusy. Actual EditorState exposes current/original DraftView byvalue,fieldText(textID),preedit,fieldHasParseError(FieldId),validationReport/canApply/hasParseErrors/isDirty; named section/selected_world/focus/scroll/modal.kind are public. No underscore data access (`classic_editor_layout.zig:1-27,185-220,364-441`; classic_editor.zig:73-144,206-238).

Session.editor is public Transactions; Transactions.session:?EditorState,status:Status,action_failure:?Failure are actual named publicfields. No Session editor getter is invented. Core.mode()/viewport() actual public. No generation/resident/draft/hold getter exists in EditorState,Core or Transactions. Caller supplies trusted source revision/full OwnerSpec and immutable UIthread source lease, forbids all epoch/source/status/modal/focus/world/scroll mutations until prepare returns. Revision must increment for every presentation-affecting change, including preedit,not just draft identity. Token/hold/context ownership remains external currentcoordinator (`classic_session.zig:35-44`; classic_editor_transactions.zig:7-19`; classic_app.zig:141,241`).

```zig
pub const CurrentStamp = struct { revision:u64, owner:H1.OwnerSpec };
pub const SavedBaseFocus = struct {
    order:[]const layout.FocusKey, restore:?layout.FocusKey,
};
pub const FocusInput = struct {
    current:?layout.FocusKey,
    saved_base:?SavedBaseFocus = null,
};
pub const Source = struct {
    editor:*const editor_model.EditorState,
    report:*const scenario.ValidationReport,
    parse_flags:*const layout.ParseFlags,
    viewport:[2]f32,
    store_status:*const transactions.Status,
    action_failure:?*const transactions.Failure,
    focus:FocusInput,
    // Explicit arbitrary caller diagnostic lane, never inferred from raw OS cause.
    raw_status:?[]const u8 = null,
    raw_action_error:?[]const u8 = null,
};
pub const PrepareError = H1.FrameError || layout.LayoutError || error{
    NoEditor, WrongProjectionMode, InvalidOwner, InvalidSource,
    InputLimit, TextLimit, SnapshotLimit, InvalidRecoveryRecord,
};
pub const PublishError = H1.FrameError || error{
    EmptyProjection, BudgetMismatch, StaleProjection,
};
pub const ActionError = error{
    InvalidUtf8, StaleProjection, UnknownAction, DisabledAction,
    InactiveModal, InvalidSource,
};
pub const Target = union(enum){field:editor_model.FieldId,action:editor_model.Action};
pub fn prepareEditor(allocator:Allocator,budget:*H1.PreparationBudget,
    workspace:*layout.Workspace,source:Source,current:CurrentStamp)
    PrepareError!PreparedEditor;
pub fn prepareEditorFromSession(allocator:Allocator,budget:*H1.PreparationBudget,
    workspace:*layout.Workspace,session:*const session_model.Session,
    focus:FocusInput,current:CurrentStamp) PrepareError!PreparedEditor;
pub fn publishEditor(slot:*H1.FrameSlot,published_revision:*?u64,
    prepared:*const PreparedEditor,current:CurrentStamp) PublishError!void;
pub fn validateEditorTarget(view:H1.FrameView,published_revision:u64,
    current:CurrentStamp,current_modal:?editor_model.ModalKind,
    id:H1.ActionId) ActionError!Target;
```

All module/API names above NEW Proposed; actual H1 types/errors propagate LimitExceeded/ReferenceOverflow unchanged. Extra TextLimit checks totalH1text,SnapshotLimit unique Headerarena,InputLimit localcounts/announcements; InvalidOwner rejects missingdraft/context128/nonadmittedowner. InvalidReport uses actual layoutlogical/physical/schema validation, no ad-hoc alternate100profile. Source.raw_* when supplied replace categoryDISPLAY for that lane only, require exactUTF8 and fullbytecount; never free/reset original typedfailure.

Sessionwrapper verifies coremode editing_scenario and editor.session nonnull; gets actualEditor pointer without copyingowner. It creates ONE local report=editor.validationReport(),ONE ParseFlags90 via fieldHasParseError for all90catalogentries, and local DraftView only while needed by prepare. It passes pointer to original localreport/flags to pure function synchronously; cannot return Source/LayoutView/issueText slices borrowing locals. Workspace alias validation before layout writes and report/flags physical validation before anyslice. Typed status/action pointers refer actual Transactions lifetime. Category formatter scratch512+decimal20 local plus actualreport16808+flags90+Workspace3196 externalcallerstorage are named populations; DraftView/Source/FocusInput/CurrentStamp/callframes/transitive stack require futureactualsize. No hidden secondreport/Workspace/currentepochclone, no allocating candidate/messageAlloc/copyCurrent/validation installation. Pure source callers may supply alreadyexistingreport/flags instead of wrapperlocaldata.

## Header lifetime, full copied source, exact allocations

PreparedEditor move-only nullable stable heap pointer, deinit/input/readgeometry borrow semantics match accepted H1/H3a ownership model. Header one actual @sizeOf allocation with NodeSpec480=76800,children479usize=3832,ActionId32=512,Announcement8=320,context128,arena196608,metadata<=4096 =>old prospective282296 envelope. Workspace not embedded: caller3196 reused while unborrowed,layout views copied immediately into NodeSpec and released before returning. No report16808 or ParseFlags90 embedded. Metadata includes allocator,budget,Charge,resident+draft retainedhandles/flags,revision,hold,counts,offsets,base/modal/focused/restore slots and layoutmetrics. ActualtargetHeader<=294912 mandatory; no padded envelope proof substitutes actualtargetlayout. Independent mutable sourceEpoch lifetime ends after prepare; every outputname/value/description/context becomes copied Headerarena or static moduleliteral before return.

Preflight read-only Source validation, report/schema,flags/catalog,fullUTF8,layoutinput scalar counts and all exact copied lengths. Use actual layout(workspace),hold/release view with defer on everypath; while viewborrowed no workspaceoverwrite. Scalar text/count pass makes no source mutation/allocation and does not retain slices beyondcall. If modal needs base validation, use same Workspace nonmodal pass,copy only bounded static IDs/counts into scalar metadata,release,then modalpass; never two simultaneousWorkspaceviews or descriptorarrays. After all count/UTF8/localcap preflight reserve exactHeaderbytes as .frame_scratch,then allocator.createHeader;OOM cancelsreservation. CommitactualrequestCharge immediately; initializeHeader safely; replay accepted deterministic source/layout/text formatting into Header with checked limits beforewrites. On failure destroyHeader before copiedCharge.release; source/workspace ownership intact. Retainresident AFTER completecopy, then draft AFTERresident with checked overflow. Failure secondretain releasesfirst,freeHeader,releaseCharge. Distinct two caller reference slots required; no globalregistry or Coretokeninvented. Prepareddeinit releasesboth once, savesCharge/allocator beforefree. Stablebudget outlivesHeader/frame. Sourceimmutable lease coverspreflight/replay; no numerically inferred provenance.

Publication validates nonemptyHeader, samebudgetpointer slot.budget,capturedrevision plus input.owner.matches(current.owner) including resident,draft,hold and fullUTF8context. H1replace synchronously builds/copies candidate; only after successfulreplace trustedoptionalrevision=current.revision. Every failure preserves oldframe/backing/focus/trustedscalar and source,preparedHeader alive for retry/cancel. Nullpublication admitsnotarget. No redundant owningpublishedstamp or arbitrarysideactionmap. Headercontext copy128 bounded independenttext/IDarenas; fullarbitraryUTF8 no normalization. Context/modalactivity are trustedcallerprotocol, not discovered from arbitraryequalbytes.

## Static109 identities and exact DFS mapping

Statictable109=90FieldId+3sections+7worldchoices+7footer+2modal. Fields canonicalsourcekeys scenario.seed,rule.<GlobalField exceptseed>,world.<0..6>.<WorldField>. Nonfields canonicalkeys section.worlds/rules/validation,world.select.<0..6>,footer.revert/factory_defaults/regenerate_from_seed/load/save/cancel/apply_and_restart,confirmation.dismiss/confirm. These footer/modal keys are NEW retainedidentityschema,not asserted literalRustIDs. ID `a/editor/<key>` maps statictypedTarget; nodeIDs `c/editor/<key>/{primary,label,value,error}`. Tickreadonly tableidentity preserved without editablecapability. Static table `{id:[]constu8,target:Target,editable:bool}` owns no heap/ref; actualsizeof109table and stringliteralfootprint separatelytargetmeasured,not duplicated Header. IDs bounded64node/48action, fullretainedstaticidentity independentrawtext.

Consume layoutdescriptor order/statusheader+optionalstore/sections/worldchoices/preview/activefields/footer/modal. FlatDFS: rootApplication -> editorregion -> statusregion(statuswidgets triplets),navregion(section+choice triplets),previewregion(previewtriplet whenpresent),contentregion(activefieldtriplets or validationlist(issuegroups)),footerregion(footertriplets),recoveryregion(actionAlert),then confirmationDialog(message+twoactiontriplets). Instantiateonlyneededregions; tenmaximumcontainerIDs static. Never interleave latechildren of earliercontainer; each group children appendedcontiguously,then rebuildadjacency finalsourceorder. All parentindices backward/childedgeN-1; actualH1preflight+independentliteralDFS fixtures mandatory. Layoutbounds copiedexactly, no geometry policy duplicated or newRectformatting.

Uniformwidget primary+labelText+valueText=3. Fieldrole text_input(name),spin_button numeric,optionowner;tick_hztextreadonly. Fieldprimaryvalue andvaluechild identical fullsourcebytevalue;labelchild staticlabel. FieldError returns parseINVALIDVALUE beforefirstmappedError; primarydescription+Alertchild copyerror twice. Validationrow list_item+codeText+messageText+fieldText=4, sourcecodeCamelCaseuppercaseclosedmap and ERROR/WARNINGseparator copies accounted below. Cleanrow3. Status/preview3. Confirmationmessage3+twoactiontriplets plusDialogcontainer. Independent actionfailure Alertprimary+Textchild2, orphanpreedit Status1 only when no focusededitablefield. ExactN=3*W+issueRows+fieldErrors+containers+orphan+actionFailureNodes; W<=114,issueRows<=98,containers<=10,orphan<=1,actionFailureNodes<=2 ->validationmax453,edges452. Forfieldsections issuesRows0/fieldErrors<=13 and smallerW bound; allprofiles<=480/479. This additive completeformula corrects prior452sketch without capgrowth; potential453 onlyconservative separatelyattainableconditions,not claimedsourcewidgets453.

Every source-nonfocusable action node.action=null, including tinysection/worldchoices and allmodalunderlay. Source enabled/visible/focusflags retained; field/clippedsemantics retainedcomplete evenvisiblefalse. NonmodaldisabledApply focusable/action stays,notenabledtraversal/activation. Underlaynodeenabled=false/focusable=false/actionnull duringmodal. Noemptytextomission, noclippingnodepopulation. Resizefreshlayoutrestores admittedcapabilities,not directmutateH1frame. Hits reverse sourcevisibleenabledfocusable; retainedeventrouting future must honor samepriority ratherthan arbitraryDFSfirsthit.

## Focus, modal and current target capability

Nonmodalorder source enabled+focusable descriptors,includinginvisiblefields, <=28; actions separate<=28includingdisabledApply. SavedBaseFocus is explicitcallborrowed remembered complete sourceorder/restore from BEFOREmodal; modal sourceEditor.focus alone doesnotprove this list. Modal requires saved_base present:validateuniqueclosedkeys,<=28,exactcurrenteditorowner/draft association bycallerlease,restorememberorclear under sourcefallback. Copy staticIDs into Header32 totalbase+modal<=30. Modalactiveorder dismiss,confirm two;underlay IDs remainbase/restore despite nullnodeactions. Sourcefocusedcurrent modal key must dismissorconfirm; missingcurrent picksfirst,reversefallback handledactual layoutnext. Nonmodalwellformedabsentfocusedkey becomes sourcefirsteligible (explicitsourcefallback), malformedtypedworld7 InvalidFocus;no borrowedIDslice. HeaderFocusSpec base remembered/modal2/focused/restoreraw<=34;H1scratch544 actual versuscommon1056 conservativeallowance. A statictable/baseorder never dispatches absentnodecapability.

validateEditorTarget checks idUTF8,trustedrevision,currentcompleteview.owner,closedtypedIDactualemittednodeaction/equality,enabled/focusable,thenmodalpolicy: current_modalnonnull permits ONLYconfirmationdismiss/confirm with currentconfirmationcontext;underlay inactive. NonmodalrejectmodalIDs,readonly/tinynulls unknown. ReturnfieldTarget is focus/editcapabilityonly; actual focusField takes canonical textID from existingeditorcatalog, notFieldId inventedAPI. ReturnactionTarget invokes no transaction. Latercaller verifiescurrenteditor/source lease beforedispatchEditor/Transactions.dispatch;confirmationone-shot captures alreadyowned source, projector never clones/begins/consumes them. Modalreason/context must be caller revisionbound;no blindconfirmafterbufferschange. actualTransactions.confirm clearscapsule BEFOREattempt, failureconsumption remains sourcepolicy (`classic_editor_transactions.zig:154-188`).

## Full UTF8 and formatter accounting

Inputs remain unbounded UTF8. Actual90buffers/preedit arewholeepochcopies;invalidnumericspelling may beararbitrarilylong validUTF8 and must displayexactly. Wire65536JSON is noinput/outputcap. Readonlytick retainsactualfieldText,not canonicalregeneration. Onlyactivefields copied,plusallsevenchoice names+preview selectedname wherepresent. Focusedfielddisplay=rawfield followed IMEpreedit directly with ZERO separator bytes, exactly value.push_str(&self.ime_preedit) when sourcefocusedIDmatches andpreeditnonempty (src/editor/layout.rs:174-175); orphanpreedit completeStatus ifnofocusededitablefield. No composition separator or normalization. Sourcepreeditnot truncated/deleted onprojectionlimit.

Exact preflight maintains separate uniqueHeadercopiesU and H1retainedcopiesT. For eachwidget copyname/description/value once peractualsemanticfield,includingprimary/childduplications;sharedHeaderpointerstillH1doublecopied. Worldchoice rawname twice plusselectedsuffix;previewrawname twice;fieldrawvalue twice;focusedcombined raw+IME twice;store rawstatus twice;actionrawerror twice;orphanpreeditonce;fieldError twice. Validationissuefullmap copiesfieldpathTWICE,messageTWICE,codeTWICE,severity3,separators3;98*(2*27+2*73+2*24+3*7+3*3)=27244. Parse/groupedmapping/errorprecedence actuallayoutfieldError;no pathsynthesizedfromusertext. Source validation98callsitebound/physical100/text48/96/path27/message73 provedprerequisite,not attained98 claim. validateReport rejectscraftedoversized schemabeforecopy,fullsourcepreserved.

Fixedtypedformats seeduppercase16hex,u64decimal20,i32decimal11,u8decimal3,ownermax7;previewnumeric119;sheetno f32text. Acceptedprerequisitefixed4226<=8192 coversliteral/separator/scalar/closeddiagnostic lanes;fieldErrors1898+validation27244+fixed4226+ann768=34136,leaves228008 variableH1text within262144. Conservative8192allowance retains38102subtotal/residual224042. ExactT decidesadmission,neitherresidualnormalizesrawstrings norcapindividualinput. U<=196608 independently; T<=262144 independently;violations SnapshotLimit/TextLimit preservefullinput/oldframe. No hashingdedupe hiddenmap: implement fixedboundedcopyreferences/offsets inmetadata4096 orcopyeachdistinctsource-lane once conservatively,prove actualU and repeatedtext. Ifmetadata4096 cannotfitactualreferences,dispatchblockednot capgrowth.

Typedstore/actioncategory mapping exactacceptedprerequisiteclosedliterals<=128;sourceStatussuccessNO SAVED SCENARIO/SCENARIO LOADED INTO DRAFT/SCENARIO SAVED unchanged. StorestatusandactionAlert independent; nonscenariostorage record InvalidRecoveryRecord;typedcause/rawOSdetails nevercleared ormessageAlloc/errorName used. ExplicitSource.raw_* completeadmission optional forfullrawdisplay,UTF8countedno128fiction. Existing Sessionresource recoverycategory source can be separatelyprojected onlyif exactboundedlane/count assigned;this source has no invented thirdrecoveryinput or silentlylosttypedfailure.

## Numeric profiles and independent failure tests

Limits480nodes/479edges/49152IDs/262144text/64focus/8ann/128context/442368owned. Headercap294912,actualnewTarget.pending;callerWorkspace3196 notHeader,budgetpopulationHeaderframe_scratch exactrequest. Scopedshared1179648 appliesunchanged;old394456+candidate394456+Header294912+DFS3840+focus1056=1088720,slack90928 requested. Namedwrapper report16808+flags90+workspace3196+formatter532=20626 outsideheapcharge;actualoptionalrevision16fromacceptedH3a,targethandle8 expectedassert,draftview/source/stackstaticcatalogmetadataUNKNOWN;no wholeprocessclaim. Conservative900000 cannotadmitthis upperprofile, but mandatoryfailure uses independently measuredactualpeakP exact/P-1,not demand900000fail forsmallerrealfixture. OneHeaderlifetimecontract independentofbudgetfailure;no secondpreparedHeader overlappingunaccounted.

FutureREDmust runnablebehavior:90fieldcatalogreadonly and109IDs;modalonlytwoactualactions/nullunderlay+savedbase;tinynonmodalpublishnullactions/resizecaprestore;literalDFS/453formula;fullrawinvalidfield/preedit/worldnames repeatedcopies exact/one-lessT/U;typed98/100 schema;independentall7reasons/footerorders/26families;fullsourcepointerpreservation. OOM stages are REALHeaderallocation,H1candidate/arena/DFS/focus allocations,checkedresidentthen draftretains separately;recordeveryactualrequestordinal,syntheticlimit+actualOOM,oldframe+revision+inputbuffers+sourcecapsulepreserved,retry/deinitledgerzeroexceptresident. SameTokennewbufferrevisionstale rejects;tokenoldreferences noABA;UIthreadcleanup. Sourcecategorypolicyboundtestsindependentliteral expectedstrings. Host stale recoverychannels notclaimedavailable: currentprovenance focusField/backspace/replaceFocusedText/modalcancel onlyfutureexplicitrouting,notoldpixelactivation. No rendering/H2font/assets/nativebrowser/queueacceptance.

## Remaining dispatch decisions

Rootscopedbudget/category/module/getter/modal/tiny rulings alreadyexplicit;do notaskagain. Acceptedlayout/getter source six identities and root/README baseline now rebound below. ActualHeader/static109table/copyoffset/targetsizequalification and currentfocusmodal/savedbasecoordinatorownership must freeze beforeimplementation. Sessionresourceerror visual treatment vsrootrecovery presentation requires exactscoped decision; sourcewrapper cannotquietly inventorclear it. Announcements actualsourceRustempty; defaultempty, futureclosedoccurrencevocabulary mustreviewbeforeemission (no arbitrarycaller8events). Actualhostmodalcontext/revision-generationlease remains3I integration prerequisite,not purebuildercompletiongate when explicittrustedcallerfixtures supplyit. This proposal grants no productionownership or testexecutionauthority.

## Accepted layout qualification rebind

Existing durable docs/qualification/2026-10-09-zig-task3h3b-layout-gate.json SHA60bce1289f075f806c814a97f2b62b8a946e0a1fa6602fe4508ed7f7fdd29f96 read: terminal0,374Debug,67identities,emptydrift. Parent reports ReleaseSafe testbuild0/57CLI/323codec/144000ticks2038events; these are reported acceptance evidence,not commands run here. Durable independent review docs/qualification/2026-10-09-zig-task3h3b-layout-review.md SHAb68b58381af7b8b8eb89757ffe3e67900d961dd7278b2630cff97f635c566e57 matches. Fullsix final-source hashes in report remain matched,including rootf40c00f8bbf465b9e0cc6ee3c18c5e45e132a7e460b60ea61dcfef422caa147f and READMEb493a31ea91d43d2732c15b8bc59d42131b451191bc55110b41236b0729dea72. This closes prior actual-layout/getter acceptance prerequisite only.
