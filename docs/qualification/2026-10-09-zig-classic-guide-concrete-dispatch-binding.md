# H3d concrete pure retained guide dispatch binding

2026-10-09. Proposed nextpuredispatch closure; acceptedsource/correction/addendum/reviews/rootGuidePublication andshared1179648 contracts unchangedexcept explicitbodyblankseparatorbound correction below. No guideproductauthority now. RetainededitorUIsolemutator; markerpureacceptedprepawaitsitssequentialslice. No currentroot/README/transienteditorAPI adoption.

## Closed corpus and source location

NEWclassic_guide.zig must copy all34paragraphs andseven titles VERBATIMfromfrozenRustsrc/ui/guide.rs:15-86, module-staticUTF8ASCIIliteralstorage. Workshop/Living contents remainread-only guideprose, notnewmodeimplementation. No file/network/URIload, asset ordependency. TopicdigestdefinitionUTF8title +oneNUL +paragraphUTF8joinedbyNUL,no trailingNUL. Exactorderedcorpus identities:

|Index|Title|Paragraphs|Paragraphbytes|TopicdigestSHA256|
|---:|---|---:|---:|---|
|0|Choose your game|3|787|`bad44dedc44e86e597b7bd52c27b12f01118c2d1fda81309edcb011f8e45be1d`|
|1|Classic // First fleet|5|1229|`414023a844ac91eda7e9715ba3989acef0e8019fbc66a1e6e76aaa9ec5ed73bc`|
|2|Workshop // First energy|5|1204|`91ee81434df53f34ba11c9ba96e94229f7475ac60be447640c867827d0e4da5c`|
|3|Workshop // Ore to alloy|4|1138|`b20469cff510cb2d5efb91ed7ba947a9da74ed804d45a09cad7cd75e3e972065`|
|4|Living Galaxy // First simulation|9|3548|`f9d55420aec90ad907a34730eb6ac9c0d26fcf2ab915b4c3f5ac2d7687f5dcbf`|
|5|Controls // Mouse and keyboard|4|1225|`3079105e03a893fe0215aa6073d2f0e2da37d699630608f567008a5e45125136`|
|6|Save, exit and graphics|4|1460|`9d9c75154db348ccaa1dbbb2b196a6056e05ff57d2081740bf5b808e59f6a821`|

Totalparagraph10591bytes,title172bytes,payloadliteral10763bytes (excludescompilerNUL/padding/interning). Actualcompilerstaticfootprint/TOPICdescriptor/strings reportedduringimplementation;do notclaim10763entirebinarysize. Sevenmaxparagraphlengths306/280/265/313/532/359/429; globalmax532. Maxactivetopic521words/9paragraphs/530lines; alltopics1686words+34blanks=1720conservative. Digestexpectedcorpus mustindependentlycompareRustsource literals,not generateexpectedfromZigproduct. Originalguide source/reporthash remainsbasis; no contentediting/shortening.

```zig
pub const Topic = enum(u3) {
    choose_game,classic_first_fleet,workshop_first_energy,
    workshop_ore_alloy,living_first_simulation,controls,save_exit_graphics,
};
pub const Location = struct { topic:u8, sheet:u16 };
pub const Source = struct {
    location:Location, viewport:math.Vec2,
    high_contrast:bool, focused:?H1.ActionId = null,
};
pub const CurrentStamp = struct { revision:u64, owner:H1.OwnerSpec };
pub const Input = struct { source:Source,current:CurrentStamp };
pub const Action = union(enum) { show:Location,close,main_menu };
pub const GuidePublication = struct {
    revision:?u64=null, previous:?Location=null, next:?Location=null,
};
pub const PrepareError = H1.FrameError || error{
    InvalidGeometry,ArithmeticOverflow,InvalidOwner,
};
pub const PublishError = H1.FrameError || error{
    CurrentMismatch,BudgetMismatch,InvalidPublication,
};
pub const ActionError = error{
    InvalidUtf8,NoPublication,CurrentMismatch,UnknownAction,
    InactiveAction,InvalidPublication,
};
pub fn prepareGuide(allocator:Allocator,budget:*H1.PreparationBudget,
    input:Input) PrepareError!PreparedGuide;
pub fn publishGuide(slot:*H1.FrameSlot,publication:*GuidePublication,
    prepared:*const PreparedGuide,current:CurrentStamp) PublishError!void;
pub fn validateGuideAction(view:H1.FrameView,
    publication:*const GuidePublication,current:CurrentStamp,
    id:H1.ActionId) ActionError!Action;
```

Theseactualfuturecallables supersedeinitialinlineInputshape only; noSessionguidegetter/fields. Rawlocationtopicclamp6;sheetclampcomputedactualsheets-1;headernormalizedTopic. InitialClassiclocation1/0,Workshop2/0,Living4/0,other0/0 remainscallerroute choice,not nonexistentZigClientScreenAPI. H1FrameError propagationactualLimitExceeded/ReferenceOverflow/OutOfMemory unchanged. Tokenowner canincluderesidentandoptionaldraft;contextfullUTF8<=128,hold/currentrevision suppliedtrustedcaller. No normalization/hiddenownerstamp.

## Bounded source wrapping and geometry API

```zig
pub const WordSpan = struct { paragraph:u8,start:u16,len:u16 };
pub const LineSpan = struct {
    first_word:u16,word_count:u16,upper_start:u16,upper_len:u16,
};
pub const GuideGeometry = struct {
    revision:u64,viewport:math.Vec2,topic:Topic,location:Location,
    sheets:u16,nominal_columns:f32,nominal_rows:f32,
    effective_columns:u16,effective_rows:u16,high_contrast:bool,
};
pub const LineView = struct { text:[]constu8,position:math.Vec2,scale:f32 };
pub const PreparedGuide = struct {
    header:?*Header=null,
    pub fn deinit(self:*PreparedGuide) void;
    pub fn input(self:*const PreparedGuide)
        error{EmptyProjection}!H1.FrameInput;
    pub fn geometry(self:*const PreparedGuide)
        error{EmptyProjection}!GuideGeometry;
    pub fn bodyLines(self:*const PreparedGuide)
        error{EmptyProjection}![]const LineSpan;
    pub fn line(self:*const PreparedGuide,index:usize)
        error{EmptyProjection,InvalidLine}!LineView;
};
```

BodyLines isselectedpageprefix/slice ofHeaderLineSpan530, notlocalarray; line(i) returnsbyvalueLineView referencingHeaderuppercasearena span,position18,134+i*22,scale1.6. Callerborrows expireHeaderdeinit; handlemove nevermovesheapheader. inputallnodes/text/focus/context Header/staticborrows,geometrybyvalue no owningstamp. No report/Workspace/secondlineviewarray.

Viewportanynonfinitecomponent->640x480;finitecomponentmax320 includingnegative. Source nominalcolumns=floor((w-36)/(6*1.6)),nominalrows=max(1,floor((h-222)/22)),strictf32/finitecheckedBEFOREconversion. Effectivecolumns=min(nominalcolumns,532),effectiverows=min(nominalrows,530), castonlyboundednonnegativevalues. Thesearesemanticwrap/page equivalentcaps,notclaimedfullsourceusizecolumns whenhugeviewport:everyparagraph<=532bytes andeverytopic<=530lines, sohighercolumn/rowcannotchangewrapping/sheets/pages. Exposednominalfloats retainactualsourcecomputedcountswithoutunboundedintconversion. No floatgeometryformatting. Decimalpagecount<=530 needs3digits;source Title33 +space/parens/slash+6digits<=43locationstatus.

Wrap byactualsplitwhitespace/byte-wordlength, single-spacejoin, never splitword. Everyparagraphendsoneemptyline. Previoustopicwrap-countscalarpass usesNOsecondWord/Linearray; afterreleasinganyreusedstage, fillactiveHeaderWord521/Line530. Wordfirst/count boundu16, paragraph0..8,offset/len<=532. Geometry/alltext/countpreflight scalarbeforeallocation; previousdest sameTopic sheet-1 elseprevTopic lastsheet atSAMEeffectivecolumns/rows; nextsamesheet+1 elsenextTopic0. First/lastendpoint omittedcontrol, no disabledplaceholder. Sheet*rows arithmeticchecked beforeindex;selectedcountbounded530. Sourceblanklines retained, evenfinalblank andpagecutblank.

Footer sourcefourfixedslots:width=(w-40)/4,x=8+i*(width+8),y=h-62,height48. Presentcontrols onlybutslotindicesnevercompact. Ordinaryminimum320viewport yields70width/48height (atleast44target), not44height substitution. CheckedRectfromXYWH/canonicalfiniteendpoints; ifarithmeticnonfinite ArithmeticOverflow, ifpositive sourcewidth/48losesrepresentableextent at hugefinitecoordinate InvalidGeometry. This is explicitProposedfalliblegeometryguardforunusableextremeinput,notassertedRusterrorparity. Bodypositionindicesbounded530avoidsf32overflow. No sourceviewportcollapse atordinarynegative/zero/nonfiniteinputbecauseminimum320/fallback; corruptedbounds/overflow rejectedwithoutpublication, no roundingallpositions.

## Exact retained tree, roles, complete copy lanes

FlatDFS sourceIDs preserved:

1 guide.application Application nameNYON player guide,boundsnull,parentnull.
2 guide.dialog Dialog nameselectedtopicTitle,boundsnull,parent0.
3 guide.title Text nameNYON // Player guide,parentDialog,boundsnull.
4 guide.location Text nameexact `<title> (<sheet+1>/<sheets>)`,parentDialog.
5 guide.held Text nameTime held // Tab + Enter // Escape closes,parentDialog.
6 guide.body Text nameInstructions,descriptionFULLpreserved-case selectedlines.join(" "),value=null,parentDialog.
7 guide.body.value Text nameempty,valueFULLuppercase selectedbodyjoin,parentBody.
8 onwardforeachpresentbutton fixedorderPrevious,Next,Mainmenu,Close: primary guide.previous.node/guide.next.node/guide.menu.node/guide.close.node roleButton,namelabel,descriptionSAMElabel,valueempty,boundssourcefooter,enabledtrue,selectedfalse,focusabletrue,canonicalactionguide.previous/guide.next/guide.menu/guide.close,parentDialog. Immediatechildren `<action>.value` Text namelabel and `<action>.description` Text namelabel; noaction,focusablefalse,sourcebounds sameprimary.

Allnonbuttons enabledtrue/selectedfalse/focusablefalse/actionnull/visibletrue;title/location/held/bodyboundsnullbecauseoriginalsemantictext hasnocontrolgeometry. Everyunuseddescriptionempty/value=null exceptexplicitbodyvalue;nodeshierarchy/expandednull. No extraunderlay/editor/HUDnodes retainedinthisguideframe. Atmost7+4*3=19nodes/18edges, localcaps24/23. ExactDFS bodychildbeforefirstbutton, eachbuttonchildrenbefore nextsibling; adjacencyrebuiltsourceorderparents/childrange/reciprocalN-1. ActionIDliteralUTF8 notnormalized. SourceplatformannouncementsEMPTY; Headerannouncementszero-length, no codes/occurrences invented. Sighteduppercase laneHeaderbodyline spans separatefrompreserved semanticdescription, no H2materialization authority.

ADD TEXTBOUND CORRECTION: full normalizedtopicjoin byte CEILING isparagraphbytes+2*paragraphs-1. Source split_whitespace removes original whitespace runs and rejoins words with single spaces; normalized paragraph bytes cannot exceed original paragraph bytes. Wrap adds no byte growth beyond replacing inter-word spaces with joined-line separators; each paragraph contributes its mandatory blank line. Thus 3548+18-1=3565 is a conservative Living ceiling, not an assertion that a product-generated display string has exactly3565bytes. Original verbatim corpus remains10591bytes. Prior3557singlebody,7114dualbody,and7478aggregate bounds are explicitly superseded: singlebody3565,dual7130,complete U7173 and T7438 below. These are independently source-derived bounds, not implementation goldens. Selectedpagesubsequencejoin cannotexceedfulltopicjoin. Bothpreservedanduppercase lanes<=3565, dual<=7130. Formattinglocation<=43,allotherlabelsstaticmoduleliterals;HeaderU<=7130+43=7173<8192arena,context128separatearray. Headerstoresno completeunselected-topicbufferor duplicatejoinedtemporary.

T exhaustiveH1copy multiplicities with conservative lane maxima: guide.body.description<=3565 once,guide.body.value.valueuppercase<=3565once;Instructions12;app17;topicTitle<=33;platformtitle19;location<=43;held40;buttonnames/descriptions/children4copiesperlabel<=4*4*9=144 =>boundedtotal7438 (7130+12+17+33+19+43+40+144) <=original512fixedallowance7642 <16384. Thesecountsaredisjointno thirdbodycopy. DirectHeaderwritespreservedjointhenuppercasefromcopiedfirstlane;LineSpanupper_start/len pointintoupperlane afterseparatorsincludingblankzero-lengthspans. No dynamiclabels/rawcallertext beyondcontext/focusedID,staticContentno truncation. ASCIIcaseconversiondoesnotnormalizeoriginalsemantictext. U/Tpreflight exact andHeaderchildsourcecopy no hiddenbuffer.

## Exact Header and resource accounting

Headerfields: Allocator,budgetpointer,Charge,residentTokenRef,optionaldraftTokenRef,retainedbools,hold:?u64,revision:u64,copiedcontext[128],context_len;NodeSpec[24],childrenusize[23],focusActionId[4],WordSpan[521],LineSpan[530],arena[8192];metadata(counts/selectedstart/count/wordcount/linecount/arenaused/bodyoffsets/lens/location/sheets/previousnext/focused/geometry). No owningpublicationstamp,109table/report/workspace/heapmap/secondHeader. WordSpan<=16,LineSpan<=16/Metadata<=1024 targetassertduringimplementation. Conservativefixedsubtotalword+lines16816+nodes3840+children184+focus64+arena8192+context128+metadata1024=30248<=32768. MetadataINCLUDESAllocator/Charge/tokenhandles andallscalaroffsets;actualsizeofHeader<=32768 mandatory,not sourcepayload arithmetic promotedcompilermeasurement. Optionaltoken andGuidePublicationtags/paddingmeasureactualtarget. Preparedhandle8targetassert;publication<=64targetassert ownsnoallocator/token/context.

Preflightcompletegeometry/count/UTF8/context/focus/staticliterals beforeHeaderreservation. budget.reserve(.frame_scratch,@sizeOf(Header)) BEFOREallocator.create;OOMcancel;commitactualrequestCharge;Headercopiedcontext/staticspans/directarena populatedunderimmutableinputlease;retainresidentAFTERcopythenoptionaldraft,checkedReferenceOverflow andexactunwind. FreeHeaderbeforeCharge.releaseusinglocalsavedcharge,release acquiredtokensonlyonce. StablebudgetoutlivesallHeader/frameobjects. Sourcefocusedborrowvaliduntilreturn;afterthatcanonicalknownpresentID selectedmodule-staticcopy,unknown/omittedwellformed->null,malformedUTF8->InvalidUtf8. Focusbasepresentbuttons<=4,modal/restore=null,focused<=1,rawq5/80focus+DFS192=272 (288padded). No guidefirstfallback.

Limitsnodes24/child23/IDs4096/text16384/focus4/ann0/context128/owned32768 unchanged. ConservativeguideR24872 fromacceptedpriorcapacityformula stillcoverscorrectedT becausefulltextCAP16384unchanged;localHeader32768 separatecandidate32768 andSYSTEMstable1179648sharedbudget. Acceptedfivecapacitypeaks82800/132272/452384/136976/719136 unchanged; guide->HUDactualcurrentHeader33232<=36864,guide->editorprospectiveHeader294912 untilactiveworkerfreezesacceptedAPI/gate. SameFrameSlotbudgetobjectusedallconsumers, no98304claim/autogrow/freeoldframeearly. OldHeaderdeinitbeforeotherprepare callerlifetimecontract;illegaloverlapnotpuretypedbudgetfailure. Alllivecommitted/pendingcharges countonce, no reservationdoublecount. IndependentlymeasuredpositiveactualP exact/P-1 deterministicrequesttrace failures, no900000mandatoryguidefailure. Mainmenu destination unboundlaterintegration.

## Durable publication, action and caller guide ownership

CallerownsonefixedGuidePublication acrossdisplayedguideframes. Publish checksnonnullHeader (`InvalidPublication`),samebudgetpointer, capturedCurrentStamprevision/fullOwnerSpecmatchescurrent;constructwholecandidate `{revision=current.revision,previous=header.previous,next=header.next}`. SynchronousFrameSlot.replace thenONLYsuccesspublication.*=candidate; failureoldframe/backing/focus/entireoldrecord/sourceunchanged,preparedHeaderretryalive. SerialUIthreadimmutablecallerlease forbidsobserver/sourceinterleaving acrossreplace+wholeassignment. HeaderdeinitaftersuccessdoesnotloseShowdestinations.

validate eventUTF8 first,nonnulltrustedpublicationrevision,matchcurrentrevision+actualFrameView.owner(resident/draft/hold/context),exactemittedcanonicalenabled/focusablecurrentNodeSpec.action,thenresolvesprev/next fromtrustedrecord;nonnullmaprequiredforactualendpointactions. Returnmenu/close static. Verifytrustedlocationtopic<=6/sheet<530forstorednavigation beforeShow; malformedrecordInvalidPublication,neverparseeventLocation ormutableSessionpage. Publicationcallertrustedfromsuccessfulpublish,notarbitraryeventsidepayload. Tokenrefequality/UTF8contextmatch cannotdetectactiveguideownership;callerexplicitprecondition is currentdisplayedfullshellguide/currentguideownercontext&hold and nohighermodaldispatchowner. No actualSession/Coreguidegetter/schema/lockexistsorclaimed. Purefixturesconstructtrustedownercurrentvalues; doNOTfakehostcoordinator/queues/ownershipdetectiontests.

Caller clearswholepublicationinitialnull ONLYaftersuccessfulnon-guidedestinationreplace orslotdestroy;failedleavekeepsrecord/frame. PurefunctiondoesnotmutateGuideState/sourceSelectedSpeed/savedFocus. Later3I/rootownsfullshellreplacement(priorityoverHUD/editor),completepremodalfocuscapture/restore,sourceentryclearsinputedges/touches/drags,entireClassicCore.advance ANDadvisorypoll/submitsuppression whileguide,clockbaselineentry/exitpreservingselectedspeed. advance(0) orspeedpaused insufficientbecausecamera/visualupdate precedesholds. CurrentAPI suppliesno guideSessionfield/revisiongetter/pausecommand; no newtransactions. Mainmenu transition/noSave sourcebehavior remainslaterintegration. Purestage qualifiescontent/frame/record independentlyofhost.

## Independent fixtures, evidence and remaining dispatch seams

Literalcorpus7digests/34paragraphs/10591bytes/172titlebytes/allsourcewrapping;handone two/three columns3->one,two,blank,three,blank;longwordabcdefkeepswhole. FullLivingtopic withhugetalladmittedviewport/bodyjoin<=3565blankseparators (widthmustchoosecolumn532equivalent;sourcefullnormalizationindependentliteral oracle),dual7130/U7173bound/T7438bound. Corpuswordspans/linebounds/selectedblank offsets/uppercaseoriginalcase roundtripindependentexpectedliteral,not productgeneratedgolden.

Geometry320x320nominalcol29/rows4/footer70,x8/86/164/242,y258;640x480col62/rows11/footer150,x8/166/324/482,y418;topic0firstomitsPreviousbutNextslot1;finaltopiclastomitsNext;crossprevusespreviousownlastsheet;resizenormalizedlocation/map;hugefinitecountclampingbeforecast/footercollapsereject;NaN/Infwholefallback,negativefinite320. Knownfocusinputbackingdestroyedafterprepare/unknownomittednull/malformedreject;4buttonq5,actual19flatDFSroot/child/presentactions/annempty.

PublishA/destroyHeader/mutableSourceB/validateNextstillAmap;failedH1replacementpreserveswhole3fieldrecord/currentbacking/source;owner/revision/malformedtrustedmap/eventunknownreject;currentcontextcallerpreconditionexplicit. ActualHeader/metadata/WordSpan/LineSpan/GuidePublication/handle/staticfootprint/eachrequestedallocation andretains/OOMordinal qualifications duringimplementation,allmodes/contemporaryrunnableREDsourcebehavior. Steadyrepeatpub/destruction ledger/sourcepurity/oldtokenretention/noABA.UIthreadcleanup. Noallocationtestsforstaticparagraphsource;realHeader/H1allocationfailurestages only.

FutureNEWclassic_guide/tests plusminimalrootREADME sequentialafterpreviousproductacceptance; exacteditorUIcurrentacceptedrebindonlyAFTERfreeze, no currenttransientadoption. H3aUIstableacceptedHeader33232,reviewbb0...gate0/358/65historical, layoutaccepted374/67noteditorUIgate. H2sourcewrapfontmetadata doesn'treplaceguide6*1.6sourcegeometry;actualfont/materialization/host later. No concrete additionalrootpolicychoice requested; correctedblanktextboundfitsunchangedcaps,extremerepresentabilityguard explicitlyProposed for freshreview. Source/root/currentworkerpreserved;no compiler/test/gate duringprep.
