# Task 3f — Classic CPU advisory and controller inventory

Date: 2026-10-09. **Current:** source inventory only. **Proposed:** Zig subtasks
and acceptance below. No implementation, tests, builds, GPU execution or browser
qualification were performed. Only this report was written for this dispatch.
Workshop/Living authority, workers and advisory are out of scope.

## Actual source and isolation

The complete Classic advisory source is `src/advisory/mod.rs` (448 lines) and
`src/advisory/gpu.rs` (272 lines). Model, feature extraction, CPU reference,
controller and readiness getters are all in `mod.rs`; there are no separate
Classic advisor/model/reference/worker files. The GPU shader is
`assets/shaders/advisory.wgsl`. Primary acceptance witnesses are
`tests/advisory.rs`, `tests/advisory_gpu.rs`, `tests/shaders.rs:110-118`, and the
Classic AppCore integration at `tests/app.rs:125-140`.

The advisory accepts `&Campaign`, explicit canonical source fingerprint and
read-only feature/snapshot values. It has no Simulation reference, GameCommand
type, queue mutation, RNG or campaign mutation (`advisory/mod.rs:5`, `148-177`,
`311-413`). The source exclusion test pins absence of GameCommand
(`tests/advisory.rs:338-352`); port acceptance should also prove digest/queue
isolation dynamically. AppCore alone chooses when to request, using
`simulation.state()` and `canonical_fingerprint()` (`app/core.rs:1005-1013`).
**Source fingerprint is canonical campaign state, not ScenarioFingerprint.**
The scenario fingerprint separately seeds visual presentation.

The neural scorer is a presentation hint, not the Classic faction AI. There is
**no advisory search tree, route planner, minimax, sampling, beam, depth or search
budget**. The only nearest-distance scan is a feature. There is **no CPU worker
or CPU cancellation token**: CPU scores compute synchronously in request creation
and immediately publish. GPU mapping is asynchronous but separate from the
platform's asynchronous GPU-device initialization. Preserve these distinctions
instead of inventing a background architecture.

## Exact feature, model, ordering and capacity contracts

`MODEL_VERSION=1`, `FEATURE_COUNT=12`, `HIDDEN_COUNT=4`,
`SCORE_COUNT=WORLD_COUNT=7`, `PACKED_FEATURE_COUNT=84`,
`PACKED_WEIGHT_COUNT=57`, `CADENCE_TICKS=30` (`advisory/mod.rs:7-13`). Fixed arrays
are values, not dynamic allocations. The controller retains one latest CPU
request, one published snapshot, one in-flight request and one overwriteable
queued request (`93-103`). Those are distinct ownership copies, not four worker
lanes. AppCore additionally holds one pending GPU request ready for host dispatch
(`app/core.rs:170-171`).

`features_for_world` indexes the seven-world array by WorldId and returns twelve
zeros for an invalid ID (`advisory/mod.rs:311-314`). For a valid world the features
are, in exact order (`348-364`):

| Index | Feature and normalization |
| --- | --- |
| 0 | Union owner indicator |
| 1 | Helix or Choir owner indicator; neutral is zero |
| 2 | defense / 100,000 |
| 3 | energy / 250,000 |
| 4 | base output per second / 3,000 |
| 5 | base regeneration per second / 500 |
| 6 | Atmosphere / 10, halved if that field is the active hazard's affected field |
| 7 | Hydrosphere / 10, same hazard adjustment |
| 8 | Topology / 10, same hazard adjustment |
| 9 | Sum of inbound Union fleet strength / 100,000 |
| 10 | Sum of inbound non-Union fleet strength / 100,000 |
| 11 | Minimum squared distance to any Union-owned world / 200,000,000; no Union world defaults to 1 |

Normalization `unit(value, fallback)` clamps finite values to [0,1], otherwise
returns fallback (`442-448`). Most feature fallback is zero; distance fallback
is one. The active hazard's affected field is used directly, irrespective of
hazard kind or inspecting start/end tick; the simulation owns its active validity
(`315-323`). Resource denominators are literal constants, not scenario maxima.

Inbound sums use saturating u64 addition and scan **all** fleets destined for
the world (`324-336`). This must not use presentation's truncated 512-fleet
scene list. There is no fleet count cap in advisory: Campaign owns a dynamic
fleet vector (`game/model.rs:268-278`). Positions widen to i128 before squared
distance, reduce with f64 minimum, divide in f64 and cast to f32 (`337-363`).
Union worlds include themselves, so their distance feature is zero. No travel
path/speed or fleet progress enters this feature.

World score/feature packing order is numeric WorldId 0..6, with 12 contiguous
features per world (`367-375`, `409-413`). Existing source computes packed
features and CPU scores independently, extracting features twice per request
(`173-177`). A port can reuse a feature value within the same immutable request
if byte/score parity is proved, but must not introduce stale caching or truncate
fleets. Complexity is linear in all fleets per world plus seven-world distance
scan; CPU neural evaluation itself is fixed work.

Exact f32 weight literals (`advisory/mod.rs:15-31`):

```text
W1[0] = -0.60,  0.55, -0.80, 0.00, 0.35, -0.20, 0.10, 0.10, -0.10,  0.20, -0.60, -0.40
W1[1] =  0.80, -0.80, -0.70, 0.15, 0.15,  0.05, 0.00, 0.00,  0.00, -0.40,  1.00, -0.20
W1[2] =  0.00,  0.00, -0.10, 0.15, 0.55,  0.25, 0.20, 0.20,  0.20,  0.00, -0.10, -0.15
W1[3] = -0.20,  0.20, -0.35, 0.00, 0.00,  0.00, 0.00, 0.25,  0.00,  0.60, -0.35, -0.90
B1    =  0.50,  0.10, -0.25, 0.55
W2    =  0.45,  0.25,  0.20, 0.25
B2    =  0.10
```

Weights pack W1 row-major at offsets 0..47, B1 48..51, W2 52..55, B2 56
(`377-389`). Evaluate sanitizes each feature with `unit(value,0)`, starts each
hidden activation from B1, adds products in feature order, applies ReLU, then
starts output from B2 and adds four hidden products in order; final score is
`unit(score,0)` (`392-406`). Preserve f32 intermediates and operation order;
do not introduce fused multiply-add or reassociation while porting bit goldens.
Zig float behavior needs explicit verification in Debug and ReleaseSafe on
target, not an assumption based on identical decimal constants.

Priority chooses highest sanitized score with strict greater-than, so a tie
keeps the lowest numeric world ID (`415-426`). It does not filter Union, neutral
or hostile worlds. Nonfinite priority input becomes zero.

Factory-default score bits are reviewed test goldens
(`tests/advisory.rs:92-105`):

```text
[1058173887, 1060329379, 1059226841, 1054688186,
 1056209868, 1054665716, 1057420253]
```

GPU eligibility requires every score finite and within [0,1]
(`advisory/mod.rs:436-439`), then pairwise CPU/GPU parity:

```text
abs(cpu - gpu) <= 1e-5 + 5e-5 * max(abs(cpu), abs(gpu))
```

Both inputs must be finite (`428-433`). Accepted GPU scores select their own
priority; Current controller does **not** require priority equality with CPU.
A near tie can therefore choose another world within tolerance. Do not add an
unreported priority-equality admission rule or replace CPU bit goldens with
GPU-tolerance goldens.

## Controller lifecycle, freshness and fallback order

Metadata is the full value tuple `{device_epoch:u64, request_id:u64,
model_version:u32, source_tick:Tick, source_fingerprint:u64}`
(`advisory/mod.rs:40-47`). Request owns metadata, 84 features and seven independent
CPU scores. Snapshot owns seven scores, priority, metadata and backend tag
(`49-62`). No borrow of Campaign survives `request_if_due`. Published returns a
borrow of the controller-owned snapshot; RequestOutcome and CompletionOutcome
return owned values/copies (`72-90`, `144-146`). Zig should retain that separation
and make any borrowed snapshot lifetime explicit.

### Creation and cadence

Initialization/Restart/MaterialEvent requests are unconditional. Tick requests
are due only when `campaign.next_tick % 30 == 0`, the tick differs from
`last_cadence_tick`, and latest metadata did not already request that tick.
`last_cadence_tick` updates only for an admitted Tick trigger
(`148-163`). Rejected cadence consumes no request ID and does not replace data.
The metadata records **next tick at request time**, which is the post-step state
when AppCore schedules after simulation stepping.

Every admitted request uses current request ID, wrapping increments it,
retains latest metadata/latest CPU, and immediately publishes CPU output.
Backend tag is CpuFallback only if current epoch has previously been disabled;
unsupported/unavailable compute alone uses Cpu (`165-186`). If GPU enabled and
no flight exists, own one in-flight copy and return a dispatch request. Otherwise
replace the single queued request with newest; never append an unbounded backlog
(`188-200`). Requesting CPU output must not wait for adapter mapping/readiness.

### Completion decision order — preserve exactly

`complete_gpu` (`203-235`) executes in this order:

1. No in-flight request → Stale, no snapshot or dispatch.
2. Full metadata differs from in-flight metadata → Stale, **leave flight/queue
   intact**; do not cancel someone else's matching request.
3. Take the matching in-flight request.
4. Error result, nonfinite/out-of-range score or parity mismatch → disable
   current epoch and fall back immediately.
5. Only then check metadata against latest requested tuple, current epoch and
   model version. If obsolete → Stale, do not overwrite published CPU; promote
   newest queued request to in-flight and return it if GPU remains enabled.
6. Otherwise publish accepted GPU scores with WebGpu backend; then return the
   next queued request if any.

The fault-before-newest-check ordering is intentional and tested by
`matching_faulty_old_completion_disables_epoch_before_newer_stale_drop`
(`tests/advisory.rs:224-249`). A matching older request that fails disables the
epoch, even though a newer CPU snapshot exists; fallback republishes that
newest retained CPU snapshot. An unrelated old-epoch completion with mismatching
metadata cannot disable the current epoch.

Freshness is **latest requested metadata**, not equality with live campaign
state on every intermediate tick. During a noncadence, nonmaterial tick the
latest requested source can lag the campaign while still being an eligible GPU
completion. That is consistent with displaying SOURCE TICK. Do not add a live
fingerprint check that discards all delayed results between cadence ticks.

`fail_device_epoch` disables current epoch, clears flight and queue and republishes
latest CPU as CpuFallback, even when no flight existed (`238-258`). Without a
latest CPU request it returns Fallback with no snapshot. `fallback_for_epoch`
uses latest CPU when present, otherwise the matching taken request
(`261-276`). Future same-epoch requests still compute/publish CPU fallback.

`set_device_epoch` on a **different** epoch clears disabled flag/flight/queue;
same epoch only updates availability and cannot undo previous failure
(`122-134`). It does not itself recompute, clear latest CPU/published, reset
request IDs or cadence. AppCore resets its pending dispatch slot and schedules
a Restart request after calling it (`app/core.rs:931-934`). Initialization starts
epoch 0 unavailable, but immediately publishes CPU (`app/core.rs:203-208`).

### Readiness, cancellation and workers

Controller readiness is `gpu_enabled`, `has_in_flight_request`,
`has_queued_request`, `published`; AppCore's UPDATING is flight-or-queue, not CPU
work (`advisory/mod.rs:132-145`; `app/core.rs:573-579`). There is no explicit CPU
ready flag, request cancellation method, GPU map cancel, worker join or timeout
inside this controller. “Cancel” must mean retiring publication ownership,
clearing epoch-bound work or dropping the host adapter as actually supported,
not guaranteed cancellation of already submitted device work.

AppCore suspend/resume clear clock only; host suspend drops GPU advisory/device
and initialization resources (`app.rs:2130-2142`). A new installed device epoch
clears core request ownership and re-requests CPU/GPU. Guide holds and nonClassic
screens stop regular advisory polling because poll/submit occurs only on Classic
with guide absent (`app.rs:1903-1906`); queued callback completion may still
arrive. Task 6 must preserve eventual drain/replacement without calling that an
advisory CPU worker cancellation guarantee.

## Direct Classic call-site contracts

| Caller | Inputs, state transition and reference |
| --- | --- |
| AppCore initialization | epoch 0 unavailable; Initialization using immutable state and canonical fingerprint (`app/core.rs:203-208`) |
| After successful Simulation.step | MaterialEvent if any accepted command result or any event; otherwise Tick cadence (`805-817`). Queued intent alone does not request (`767-787`; `tests/app.rs:125-138`). |
| Apply/restart | Restart request after replacing/resetting active simulation (`app/core.rs:904-920`) |
| Set GPU epoch | Clear pending host request, change availability/epoch, request Restart (`931-934`) |
| Schedule | Take only returned optional GPU request into single pending dispatch slot (`1005-1013`) |
| Host take/complete | `take_gpu_request` consumes the pending value; completion may return a new dispatch which AppCore stores (`937-950`) |
| Device error | Clear pending dispatch and controller failure (`953-956`) |
| AppCore scene extraction | Borrow snapshot into `build_scene_frame` (`377-387`); immutable score priority projection (`presentation/scene.rs:242-245`) |
| App frame | Draw advisory with const campaign, snapshot and updating flag (`app.rs:763-769`) |
| Readable panel | ADVISORY ONLY; INITIALIZING without snapshot; priority name with UNKNOWN fallback, score to one decimal percent, source tick, CPU/WEBGPU/CPU FALLBACK, UPDATING/CURRENT (`game/view.rs:493-556`) |
| Advisory acknowledge | Enabled HUD target triggers AdvisoryRead onboarding only (`presentation/ui.rs:655-669`; `app/core/session.rs:50-52`). The actual panel's text remains in primitive content; UI batching skips duplicate text (`presentation/ui.rs:325-331`). |
| Source-tick helper | No snapshot maps to Tick(0) (`app/core.rs:1017-1018`); not a freshness validator |

`AdvisoryBackend::WebGpu` is the compute publication tag **even on native Metal,
Vulkan or DX12** (`advisory/mod.rs:229`). Render BackendKind is a different
diagnostic (`engine/backend.rs:8-15`, `47-59`). Do not rewrite the scoring backend
tag based on graphics adapter label or make backend selection alter truth.

## GPU and native/browser adapter boundary for Task 6

This portion records the interoperability contract needed by Task 3's pure
controller; it does not assign GPU production implementation to Task 3.

GPU availability is optional: adapter must expose COMPUTE_SHADERS;
`GpuAdvisory.new` returns `Ok(None)` without support, and validates WGSL before
creating pipeline (`advisory/gpu.rs:55-71`). Host explicitly skips GPU advisory
for WebGl2Low; native GL is not automatically classed low and still uses the
compute-capability check (`app.rs:607-622`; `engine/backend.rs:33-40`, `58-59`).
Initialization errors log and use CPU, not a fatal scoring failure. Device epoch
increments upon GPU installation; core requests anew. Browser graphics-ready
notification reports renderer/backend readiness, not GPU advisory parity
(`app.rs:619-639`). User graphics Low preference does not independently disable
GPU scoring in these call sites; actual adapter capability does.

Buffers: features 336 bytes, weights 228, scores and readback 28; all tightly
packed f32, shader bindings 0 features read, 1 weights read, 2 scores write
(`advisory/gpu.rs:12-15`, `73-124`). Shader arrays 84/57/7 and one workgroup of
8 invocations, with invocation 7 guarded (`assets/shaders/advisory.wgsl:1-38`).
Hidden biases and output weights share the exact offsets described above. CPU
sanitizes feature input; shader does not independently sanitize raw arbitrary
NaN features. It receives controller-produced packed features, so do not expose
an unchecked external feature upload API.

`GpuAdvisory` owns cloned device/queue, pipeline/bind group, buffers,
Arc<Mutex<VecDeque<MapSignal>>> completion mailbox and one optional metadata
flight (`gpu.rs:42-52`). Request is borrowed during submission; feature bytes
are uploaded and metadata copied into the callback (`156-197`). Busy rejects a
second submission without overwrite. It dispatches one workgroup, copies scores,
maps readback on submit, and callback only enqueues metadata/map success or
failure. No callback publishes a score or touches simulation.

Native `poll` calls nonblocking `device.poll(Poll)`; wasm omits that call and
depends on browser callbacks (`gpu.rs:200-210`). No signal yields None; map/lock/
poll errors retain correlated metadata when possible so the controller can fall
back. If no metadata is available, errors return at outer Result and host calls
`fail_device_epoch`. Successful read copies seven little-endian f32 values,
drops mapping view and unmaps before completion is returned (`211-253`). Current
callback mailbox has no explicit numeric cap; one in-flight submission limits
ordinary production to one callback before next submit. Theoretical retained
callbacks after failures are not an established bounded-worker queue guarantee.
Port should use one owned completion slot or document its exact bounded mailbox
policy, while preserving the one-flight contract.

Host takes pending dispatch, fails epoch if backend missing, and drops adapter on
submit error (`app.rs:2037-2052`). Poll outer error also fails epoch and drops
adapter; completion errors pass through matching-metadata controller handling
(`2055-2070`). A matching error completion can leave host adapter allocated but
controller-disabled; disabled controller emits no further work until new epoch.
No source timeout exists; the GPU integration **test** has a ten-second deadline
(`tests/advisory_gpu.rs:65-74`). Do not describe that as a runtime watchdog.

Platform GPU initialization is distinct: native prepares window/surface on main
thread then uses Tokio acquisition plus generation mailbox
(`app.rs:472-529`); wasm uses `spawn_local` and per-generation Rc/RefCell mailbox
(`531-574`). Native cancel aborts initialization task; browser replaces mailbox
and rejects stale generation rather than aborting the local future
(`643-655`). Neither implies CPU advisory worker support. Task 6 must keep init
generation separate from scoring device_epoch/request metadata.

## Proposed bounded Zig subtask ordering and acceptance

These expand the 3f dependency in `task-3-app-inventory.md`. Parent owns shared
`root.zig`, build/CLI/check integration; Task 1 owns model/simulation/codecs;
Task 6 owns GPU/event/backends. Proposed source files must not import platform,
storage, editor, Simulation or GameCommand into the pure scoring/controller
island. No production dependency addition is needed for the pure work.

### 3f.1 — Feature extraction and exact reference model

Own `zig/src/classic_advisory_features.zig`,
`classic_advisory_model.zig`, `classic_advisory_reference_tests.zig`.
Export by-value fixed-array `featuresForWorld(*const Campaign, WorldId)`,
`packedFeatures`, `packedWeights`, `evaluateFeatures`, `cpuScores`,
`priorityWorld`, `parityMatches`, score validity and model constants. CPU compute
uses no allocator and borrows Campaign only for synchronous extraction.

- [ ] Twelve-feature vector `[1,0,.5,.5,.5,.5,.8,.3,.4,.25,.5,0]` from
      `tests/advisory.rs:32-68`, including no-Union distance 1 and invalid ID 0s.
- [ ] All seven world packing rows and exact 57 weight bits/offsets.
- [ ] Default seven score bits above in Debug/ReleaseSafe; additional independent
      feature vectors and scripted Rust/Zig state score comparisons, not
      regenerated goldens from the Zig evaluator.
- [ ] Saturating inbound sums, all fleets including beyond 512, neutral/rival
      ownership, hazard field effects, distance widening/default/self-distance.
- [ ] Nonfinite clamping, finite range, tie-lowest-ID, parity inside/outside and
      exact boundary formula; CPU priority may differ from eligible near-tie GPU.
- [ ] Campaign fingerprint and pending commands unchanged after every query;
      no command type import or mutable authority capability.

### 3f.2 — Owned request/controller state and full disposition table

Own `zig/src/classic_advisory.zig`, `classic_advisory_controller_tests.zig`.
Define fixed-value Metadata/Request/Snapshot/Backend/Trigger/RequestOutcome/
CompletionOutcome matching the source. Export init, epoch change, readiness,
requestIfDue, completeGpu(metadata, typed result), failDeviceEpoch and const
published view. Keep optional flight/queued/latest CPU inline; no heap/backlog.

- [ ] Initialization/restart/material always request; Tick at multiples of 30
      only once and skipped after another same-tick request; cadence rejection
      consumes no ID. Wrapping ID retained across restarts/epoch replacement.
- [ ] CPU publishes before optional GPU completion; newest CPU visible while
      GPU flight runs; queued latest replaces intermediate queued requests.
- [ ] Perturb each metadata field: stale leaves original flight intact.
- [ ] Matching valid older completion is stale and dispatches newest queued;
      accepted newest returns GPU-tagged snapshot.
- [ ] Matching old error/NaN/out-of-range/parity mismatch disables epoch before
      latest check and republishes latest CPU, clearing flight and queue.
- [ ] Device failure with/without flight/latest CPU, sticky same-epoch disable,
      new-epoch retry; unavailable-but-never-failed stays CPU instead of fallback.
- [ ] Freshness admits still-latest requested source between nonmaterial ticks;
      no comparison against unrequested live fingerprint added.
- [ ] Borrowed published view documented as invalid after next controller
      mutation; owned Request/Outcome survives campaign teardown.

### 3f.3 — AppCore adapter and readable projection

Own `zig/src/classic_advisory_session.zig`,
`classic_advisory_projection.zig`, `classic_advisory_app_tests.zig`.
Interface consumes immutable simulation observation plus tick material flag;
exports CPU/published/updating, one takeable GPU dispatch and correlated
completion handler. Parent composes into Classic AppCore without transferring
Simulation ownership. Readable projection exposes priority label/score/source
tick/backend/updating; scene consumes score/priority value only.

- [ ] Epoch-0 CPU initialization and every apply/restart/dev-epoch refresh.
- [ ] Enqueue is not material refresh; accepted tick command or event is;
      rejected command-only report falls back to Tick cadence.
- [ ] UI/scene queries and advisory-read onboarding change no authority state;
      Acknowledge action progresses guidance rather than dispatching gameplay.
- [ ] Pending take consumes once; completion-promoted next request installs
      once; failDeviceEpoch clears pending as well as controller flight/queue.
- [ ] All readable states/source-tick/UNKNOWN name behavior and priority score
      projection; GPU compute tag kept separate from renderer BackendKind.
- [ ] Same command traces have identical canonical fingerprints with CPU-only,
      successful GPU, mismatch, unavailable GPU and injected stale completion.

### 3f.4 — Task 6 GPU adapter interoperability

Task 6 owns production adapter, shader compilation/upload/readback and real
native/browser effects. Task 3 owns `classic_advisory_adapter_contract_tests.zig`
using fake submission/map results to prove controller/host contract; final file
location coordinated with Task 6 before integration. Use fixed request/completion
values so no Campaign pointer escapes to device callbacks. Agree explicit
mailbox and teardown ownership before implementing.

- [ ] Exact f32 buffers 336/228/28 bytes, bindings 0/1/2, shader sizes/offsets,
      workgroup 8 and eighth-invocation guard; validate shader before pipeline.
- [ ] Busy never overwrites submitted features or metadata; callback returns
      correlated completion only; mapped bytes copied before unmap/reuse.
- [ ] Native poll nonblocking; browser progresses through its real callbacks;
      WebGL2/compute-unsupported paths stay CPU and produce no dispatch.
- [ ] Submit/poll/map failures preserve latest CPU; old generation/epoch cannot
      publish into new adapter. Drop/cancel evidence distinguished from actual
      device-work cancellation.
- [ ] Device initialization readiness and generation distinct from advisory
      parity acceptance; native-render label distinct from WebGpu score tag.

### 3f.5 — Differential and live qualification

Run pure Zig reference/controller/AppCore suites in Debug and ReleaseSafe, then
fresh integrated Zig gate. Rust comparison commands are focused
`cargo test --test advisory`, `cargo test --test app`,
`cargo test --test shaders`, and
`cargo test --test advisory_gpu -- --nocapture`, using repo toolchain/gates.
Tests cited here were inspected, not executed in this task.

- [ ] Exact score-bit/model/feature oracle remains independent of GPU tolerance.
- [ ] Review all controller transitions from `tests/advisory.rs:109-335` plus
      no-flight, same-epoch availability, ID-wrap and delayed latest-source cases.
- [ ] Genuine native GPU parity and readable fallback runtime evidence; native
      GPU test SKIP for no adapter/compute is explicitly **unverified GPU parity**,
      not a passing proof (`tests/advisory_gpu.rs:19-36`).
- [ ] Real browser WebGPU callback/readback parity; forced WebGL2 CPU behavior;
      guide/mode hold, suspend/resume/device recreation and stale teardown path.
- [ ] Source/queue/digest authority isolation across these journeys; native-only
      build/test cannot establish wasm or real callback behavior.

Task 3f pure completion does not close Task 6 GPU/native/browser acceptance.
The CPU reference and controller are a small, fixed-storage dependency that
should precede scene/UI composition; real adapter work remains a later shared
host integration.

## Key integration concerns

1. Keep CPU synchronous/immediately available, all 12 features and reviewed
   f32 bits. No training/search/CPU worker exists to migrate.
2. Do not feed advisory the scene's truncated fleets or scenario fingerprint.
3. Preserve matching-fault-before-latest-stale decision order and newest CPU
   fallback; wrong metadata must not consume current flight.
4. Freshness means latest requested state, not live state on every tick.
5. Same-epoch availability cannot re-enable failed compute; new epoch plus
   Restart request can. Keep controller flight, core pending slot and GPU
   adapter flight ownership synchronized.
6. CPU-only unavailable and CPU fallback after a failure are different labels;
   accepted compute's WebGpu tag is independent of native render backend.
7. GPU/drop/init generation behavior, browser callbacks and genuine adapter
   parity are Task 6 evidence; do not convert a headless adapter SKIP into proof.
