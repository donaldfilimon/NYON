# H3c pure marker implementation dispatch binding

2026-10-09. Proposed NEXT sequential pure slice, not current implementationauthority. Sole current productworker retainededitorUI; source/root/README preserved. Parentaccepted ChoiceB/immutablelease/HUDrebind/attach/textfix reviewed preparation and progressmarkerattach ruling retained; exact113text accounting supersedes112, but no retainedattachment implementation in this first slice.

## Narrow ownership and actual numeric APIs

Future ownership ONLY NEW zig/src/classic_start_marker.zig and classic_start_marker_tests.zig, minimalrootexport/testimport and narrowREADME AFTER retainededitorUI acceptance/freshbaseline. Puremodule imports classic_math/model/classic_camera/classic_scene/classic_picking/classic_onboarding/retained_geometry only; no classic_ui,Session,Core,adapter,host. NEW snapshotadapter and actualUIgeometrygetter/flags/attach/actionmapping are a later separate slice. No protected math/camera/pick/source/store/RustOracle/assets/dependency changes.

Actualmath Vec2/Vec3/Vec4 fields f32;Mat4 columns[4]Vec4,mulVec4/isFinite/identity; strict staged arithmetic. CameraMatrices fields view/projection/view_projection/inverse_view_projection and isFinite. CameraState.init(Vec2) byvalue, logicalviewport min1, defaultyaw0/pitch55/distance15/FOV45/aspectclamp.1..10/near.1/far100. SceneWorld idu8,positionVec3,radiusf32 plus presentationfields; SceneFrame.worlds[7],cameraCameraMatrices; buildScene takes byvalueSceneCampaign{worlds[7]World,fleets[]constFleet,active_hazard},fingerprint,alpha,SceneView,advice,visualtime,*constCameraState,PresentationPreferences. Ray.fromLogicalPointer(pointer,viewport,inverse)?Ray uses ZOnear0/far1 andnormalizesdirection; pickWorld(ray,[]constPickWorld)?WorldHit picksnearestdistance/IDtie. Existing sphereDistance assumesunitdirection algebra; projectoruses ONLY normalizedRay helper, no manualnonunitRayinput API (classic_math.zig:3-29,114-145; classic_camera.zig:15-45,104-120;classic_scene.zig:16-45,95-117;classic_picking.zig:5-50).

## Exact proposed types and callable error contract

```zig
pub const ZoneKind = enum { control, tray, help, guidance, tooltip };
pub const Zone = struct { kind:ZoneKind, bounds:Rect, enabled:bool };
pub const OpaqueEdges = struct { left:f32, right:f32, top:f32, bottom:f32 };
pub const Input = struct {
    step:?onboarding.Step,
    playing:bool,
    viewport:math.Vec2,
    camera:camera_model.CameraMatrices,
    scene_worlds:*const [7]scene.SceneWorld,
    worlds:*const [7]model.World,
    safe:Rect,
    edges:OpaqueEdges,
    settings:bool,
    zones:[]const Zone,
};
pub const Mode = enum { scene_linked, action_only };
pub const Marker = struct {
    world:model.WorldId,
    name:model.WorldName,
    point:math.Vec2,
    label:Rect,
    mode:Mode,
    connector_start:?math.Vec2,
    ring_radius:?f32,
};
pub const Absent = enum { not_initial, not_playing, no_label_candidate };
pub const Result = union(enum) { marker:Marker, absent:Absent };
pub const InputError = error {
    InvalidWorldSet, InvalidWorldName, InvalidViewport, InvalidCamera,
    InvalidBounds, InvalidEdges, ZoneLimit, ArithmeticOverflow,
};
pub const ProjectError = InputError;
pub fn project(input:Input) InputError!Result;
```

Rect aliases accepted retained_geometry.Rect. Everynewname Proposeduntilimplementation. Typedoptionalstage enum onlysourceinitial select_union_world admitsmarker. WorldnamesinlineboundedMAX24 actualWorldName, no returnedsourceString/borrow. No allocator/resultpointer/callback/workspace/token/currentgeneration argument. Puredata numeric seam cannotproveproductionprovenance or inverseconsistency; actualcaller adapterimmutablelease providesthat later. NoUIimportcycle. Fullread/read sourcealias permitted: functionwritesnoinput/outputbacking, returnsownedfixedvalue. Callerimmutablesources remainlive untilreturn; inlineResult safeafterarrayreuse/deinit. No Coretoken/revisionreread fictitiousAPI.

Validation BEFORE any candidate scan/return requires: zones.len<=26 beforeiterate; world/model andscene IDsets eachuniquecomplete0..6,rangecheckbeforeanylookup (WorldId actualu8,notu3;id7rejectInvalidWorldSet). LookupcampaignbyactualID boundedscan,notassumesamearrayorder. Checkeachname.len1..24 beforeasSlice; WorldName.init completeASCII/spacingadmission,malformedlen/UTF8/noncanonical ->InvalidWorldName. Thisvalidates namecopywithoutreadinguninitializedtail. ReadONLYusedSceneWorldgeometry(id/position/radius),notunusedcolor/fieldenergy orfleetcounts; positionsfinite,radiusfinitepositive andradius squaredfinite else InvalidWorldSet. All4 cameraMatrices finite ->elseInvalidCamera; finitearbitrarymatrixrelationship ACCEPTEDsyntheticinput, not SourceMismatch/provenance detection.

Viewportfinitepositive required, negative/zero/nonfiniteInvalidViewport; pure doesnotnormalize rawviewport. Lateradapter suppliesactualnormalizedHUD/camera logicalviewport>=1, no fallbackmatrixswap. Rects fullfiniteordered viafromBounds; elseInvalidBounds includingnonfinite directRect. Edgesfinite>=0 elseInvalidEdges; edgeslargerthanviewport allowed forcollapsedpolicy, not guessedmaxcap. Safe nondegenerate axes mustbe withinviewport; collapsed axes permittedatfinitelegacycollapsedmidpoint evenoutsideviewport (tinyHUD), yieldingno_label_candidate because180x54 insetintervalempty. This explicittiny admission differs malformedinvertedRect, which rejects. Zones orderedfinite, maypartlyoutsideviewport/zeroarea asactualHUDcontrolspermit; noinventedclipping. Validateallinputfirst eveninactiveflags: malformedinputnotmaskedbyabsence; wellformednotinitial->not_initial,wellformednotplaying ORsettings->not_playing. Addedtypedinputadmission is Proposedengineeringguard, not Rustvalidation claim.

## Deterministic ChoiceB selection and geometry

Iteration exactscene_worlds sourceorder. ForUnioncampaignowner .union_faction candidates: clip=camera.view_projection.mulVec4(position.xyz,w1). Skip nonfiniteclip or clip.w<=0 exactlyRust, includingfinite-matrix overflow producingnonfiniteclip. ndc=clip.xyz/clip.w; projectedpoint x=(ndcx+1)*viewport.x*.5,y=(1-ndcy)*viewport.y*.5. Skipnonfinitepoint orpoint outside inclusive0..viewport. NO ndczdepthclip,NO wabs/epsilon policy,NO camera authority change. A generic finiteinconsistentmatrix mayyieldabsent/actiononly,neverprovenanceerror.

Source labelmin x=clamp(point.x-90,8,max(viewport.x-188,8)); y=min(max(point.y-92,190),max(viewport.y-180,190)). Nearest full180x54 label safeinset1:allowedmin=safe.min+1,allowedmax=safe.max-1-(180,54). Emptyintervalskipcandidate; deterministiccomponentclamp sourcemin tointerval,thencheckedRectfromXYWH. Any finiteendpointarithmeticoverflow ArithmeticOverflow; no silentNaN/clamprepair. Labels mustfitviewport andsafeinset; nondegeneratepolicyboundschecked. No alternatepositionsearch/textsize/capgrowth. Labelrejectenabledcontrol/tray/help CLOSEDoverlap-or-touch; disabledcontrol/guidance/tooltip POSITIVEarea intersection. Useexactedgecomparisons forclosed test,Rect.intersection!=null forpositive; doNOTRect.overlaps .01threshold.

Firstsourcecandidate withadmittedlabel WINS immediately evenaction_only. Point/ring eligibility onlydecidesmode, neverpreferslaterUnion. Sourcepointconsumed ifsettings/nonfinite, inclusiveopaqueedges x<=left/x>=w-right/y<=top/y>=h-bottom,enabledcontrolcontains ORtray/helpcontains. Guidance/tooltip/disabledcontrol do notconsume point. scene_linked additionallypoint andringboundingRect(point±22) whollyinsideSafe,ringboundsclearzones undersamekind/touchrules, and acceptedRay.fromLogicalPointer+pickWorld SAME7scene spheres selectswinnerID. BuildlocalPickWorld[7]id/position/radius only, no campaign/reorder. PickWorldtie choosesIDnotmarkersourceorder. Invalid/unprojectablenumerics yieldingnullRay/pick meanaction_only,not sourceprovenanceerror. scene_linked ring_radius22/connector_start=label.min+(90,54); action_only BOTHnull. pointkeptnumericdiagnostic,notclaimedscenetappable. Connectorwholelinevisibilitynotguaranteedbyendpointpolicy;futurematerializerclipping separate. No camera pan/zoom/focus/restartmutation.

NoUnion/outviewport/wbehind/blockedlabel/too-small safe->no_label_candidate. No universalavailability promise. SourceinputWorldsarrays/camera/zonebounds pureprojection doesnotestablishactualhostclickpriorities; lateractualsnapshot+publishedclosedcapabilitiesdo.

## Independent default12 and synthetic fixtures

Defaulttestcorpus sourcegenerator seed0x4947_5731_2026_0902,soleUnionID0 ASTER VALE (1037,5027)->scene(-3.963,0,.027),energy80000,defense43735,radius.31+.032+.026241=.368241. Otherpositions sourcefrozenvectors:8793/2818,8517/7827,4811/1320,4905/8955,3746/3709,5925/6136;ownersHelix/Choir/nullothers (`generator.zig:5-25,80-105`). Useactualgenerator.generate/.deinit andactualCameraState.init/buildScene/pickwithoutmutatingacceptedmodules. Currentonboardinginitialstep,playingtrue,helpfalse,pointerabsent. Testzones are independentliteral/formulaHUDfixturebuiltfromacceptedH3ageometry source,NOTfutureadapterorUIimports:16 present pre-marker primary controls (17 maximum if help/reset-guidance is enabled) withonboarding3speeddisabled,launchdisabled,cleardisabled,SKIPpresent,fieldadjustenabledasactualselected-nonefalse,advisorytrue,resetguidanceabsent;tray/guidancepresent,tooltipabsent. SourceUIDefaultselectedfieldatmosphere. Scale baseclamp(min(w/1440,h/900),.72,1.15)*factor thenclamp.60..1.50,legacymax.72. AllliteralHUDformulas/zoneenableflags tracedacceptedclassic_ui mainBounds/buildPlaying/trayRect/guidanceRect,not productprojectexpected.

Expectedindependentanalyticnumbers approxnotf32execution:723x802point(105.464,402.429),sourcelabel(15.464,310.429);85/100 labelx173.8,115x199.72,130x225.64 withsamey,modeaction_only becausepointoutsideSafe.960x600point(288.452,301.069),sourcelabel(198.452,209.069);85/100 finalx198.452,115x199.72,130x225.64;predictedscene_linked afteractualf32ray/zoneproof.1440x900point(432.678,451.604),label(342.678,359.604) allfourfactorcases,predictedscene_linked. Sourceanalyticdepth15-.027*cos55 andcot22.5 formulaindependentlyderivespositions,not productreturnedgoldens (acceptedmarkerpolicycorrection table). Futuref32fixtures useliteralapproxcoordinateswith documentedtolerance .02px andfixedmode/ID/name/labelbounds/sameworldpickassertions; actualdefaultvectors/radii checkedliteral. No “actual12passed” claimnow. Realf32defaultfixturemustestablishadmission andexactring/connectornullability allcases; ifpredictionfails do notregenerateexpectedfromprojector—characterizeagainstactualsourcepolicyandparentreview.

Synthetic7sphere identitymatrices:ID0Unionposition(0,0,.8),r.1; ID1nonUnion(0,0,.3),r.1,IDs2..6atx10,20,30,40,50/z.8,admittednamesB..G. Centerpoint720/450,label630/358;normalizedidentityrayorigin0/direction+z picksID1 .2 beforeID0 .7 =>winner0action_only BOTHhidden. SeparateclearID0center(0,0,0),r.1 withothersoffray yieldspositiveexitdistance.1 andscene_linked. LaterID2Union(.4,0,.8),firstID0occluded stillwinsaction_only evenifID2scene_linked;sourceorderreorderedarray changesfirstcandidate sourceprioritywithoutIDtieabuse. Actualnormalizedray test inversefinite diag(1,1,2,1) unprojectsfarz2 butfromLogicalPointer normalizesdirection+z, same .2/.7; nevercallSphereDistance withmanualdirectionlength2 andpretendunitformula general. Finiteprojection/inverseinconsistent acceptednumeric fixture, not provenancefraud.

Additionalhandfixtures inclusivepointedge,clipw0/negative/NaN skip,ndczoutside stillsourcecandidate,zoneexacttouchenabledrejectvsdisabledallowed/positiveoverlapreject,ringocclusionpointpickhideboth,emptyinsetwidth<182/height<56absence,exact182x56fittouchesinset acceptedwithoutzones,zero/negativeviewporterror,malformedworld7/missingduplicateIDs/nameUTF8/len25,zero/negradius/nonfinitebounds/camera/edges,26zonesadmit27ZoneLimitbeforeiterate,f32overflowlabelmath typed, sourcearraysbyteunchanged onerror/success and inlineoutputname survivesbackingreuse. Fullaliasread/read tests onlyvalidinitializedtypedobjects; no invalidpointerfabrication. Rayunprojectsingularfiniteinverse->nullRay/action_only permitted; zero sourceprojection clipw0absence. No unitrayAPI assumptionhidden.

## Resource, target gates and contemporary evidence

Pureprojectheaprequests0 bycompleteAPI/sourceinspection: noallocator/ArrayList/dupe/format/stringbuilder, tokens0/events0, atmost7candidate*26zone tests and7picks. No OOMerror orfakefailingallocatorloop forallocationfreefunction. FixedlocalPickWorld[7],Result,Mat4/scalargeometry locals realnamedstorage, actualtargetsize/stackcharacterization mandatory;Marker<=128 compileassert includes inlineWorldName,optional tags/padding;Resulttargetsize reportedseparatelynotassumed128. Zone26 storage iscallerborrowed, notcopied persistentHeader; Input byvalueCameraMatrices256 isrealcallerpayload. No sharedPreparationBudget/Headerchange inthisslice. No fullprocess/transitivestackbound inferredfromoneassert.

FutureworkercontemporaryREDfamilies runnablebehavior forselection,sourceprojection/w/z,clamp/touch,actiononlypick,typedvalidation/resourcepurity;expectedoracles literal/sourceanalyticalnotgeneratedsameimplementation. ActualDebug/ReleaseSafe/ReleaseFastfocusedtests/fmt/freshindependentreview+parentwholegate requiredafterauthorizedproductdispatch. Existingmath/pickingbehavior tests reusedasdependencies,noteditingRustoracle. Parentmarker113retainedtext/closedsevenIDs/attachpreflight remainlaterintegrationacceptance,notclaimedimplementedpure. Currentretainededitorworker remainssolemutatoruntilacceptance/freshminimalrootREADMEbaseline.

## Frozen source identities and dispatch residuals

|Source|SHA256|
|---|---|
|`zig/src/classic_math.zig`|`e0b5188b08ab78a4c32a348ba0398150ab2b8ac0541a8fe4984b9e4d60913186`|
|`zig/src/classic_camera.zig`|`ac5931069c0f65c44eaad0e093dc668ef61756a5ff19cdf5f69b02dfcbd0c268`|
|`zig/src/classic_scene.zig`|`e022c8961168b072f36cd26d0a84f9cb88bd17d904689df9d2614982c6f4ce11`|
|`zig/src/classic_picking.zig`|`e9dc79171f9f3906549642df0d53c3076a52b4427eb3a9ba375aab2541b0c219`|
|`zig/src/model.zig`|`86db58f389d4b0b1e415a363b7e32c8694837f7bd23d4b3b9635cee66daed20e`|
|`zig/src/generator.zig`|`96ed3c2490d3e2b8c69c9b7921fda0a5d6e962a8679773bc63cb971b146daf39`|
|`zig/src/classic_onboarding.zig`|`f534774a32e1c00b4986f846cadad1d73ca950cba1b34445466ae4e9d956971a`|
|`zig/src/retained_geometry.zig`|`f6dc55befc4e84fddfe516f7d8b72f1304a918412cba836e1194c034ba4aaece`|
|`src/ui/start_marker.rs`|`db5a7611b7acce6e873a76a219ac8a49c2532e86864ffc1041373f4fa90c5923`|
|`src/app/core.rs`|`2ac51de5a8110c76c96c6665483812ea7869d9a33f05a4731d2abeb7c910befa`|

Originalacceptedmarkerpackages remainimmutable. No rootpolicyambiguity identifiedforpureprojector;structuralcallerlease/provenance is explicitnumericseamcondition, not newownergetter. Actualtarget/12defaultf32/independentfixtures/gates qualifyimplementationduringauthorizedslice;laterGeometryAccessor/HudSnapshot/adapter/attach/SessioncurrentUnion/hostrender/materialization/focusrouting separatelypending. This preparationran no compiler/tests/gate or sourcewrites.
