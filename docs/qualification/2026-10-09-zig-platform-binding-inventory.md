# Task 6 platform binding inventory

Observed 2026-10-09 in canonical `/Users/donaldfilimon/dev/active/NYON`,
`main`, HEAD `bac2b6e313f8034309bd718d3b8cb4e0448ac20f`.

**Current:** source and installed-header/library inventory only. **Proposed:**
the alternatives below. **Unknown:** a selected Zig host architecture and its
runtime behavior. No host/binding/GPU/runtime qualification was performed.
This work writes only this report; it does not alter production code, install,
download, clone, build, touch profile data, or query credentials. Existing
Classic store and Living changes belong to other slices. Source is a live,
shared tree; line references identify this observation, not a frozen release.

## Binding scope and acceptance

[Migration Task 6](../superpowers/plans/2026-10-09-nyon-zig-migration.md)
requires native/browser hosts, full renderer/input/semantics/audio, real native
and IndexedDB stores, Library/Sessions/coordinators/transfers/recovery/Continue,
production content and all 46 creator tools. Task 8 retires Rust production
paths only after actual final Zig replacements are exercised. A terminal host
or macOS window alone cannot close either task. Bindings must be resolved in a
reviewable design before adding production dependencies. The existing Cargo
dependencies do not automatically authorize a new Zig/C/C++ production library.

The binding requirement is the complete matrix, not just the current machine:

| Host | Native backend | Browser paths |
| --- | --- | --- |
| macOS | Metal | Safari and Chrome: natural selection and forced WebGL2 |
| Windows 11 | DX12 | Edge and Chrome: natural selection and forced WebGL2 |
| Ubuntu 24.04 | Vulkan and forced GL | Firefox: natural selection and forced WebGL2 |

Source: [Living qualification §5](../superpowers/specs/2026-09-04-nyon-living-galaxy-qualification.md),
lines 150–159, and migration requirements lines 87–100. Record OS/browser,
GPU/driver, requested/selected backend, final artifact hash, catalog/seed/digest,
screenshots, keyboard/screen-reader, durable reopen, fallback and performance
for each row. Missing machines stay unverified. Header presence establishes no
adapter availability and no cross-compilation/link success.

Required viewports include 723×802 at 100/115/130% and 1280×720, 1440×900,
1920×1080 at 100/130%; retained controls use logical pixels and 44-pixel targets.
Acceptance includes Retina, touch/pinch, keyboard and native/browser screen
reader, muted/reduced-motion equivalents, interrupted/quota/device-full storage,
portable readback, and network-disabled cold new/open/play/edit/branch/save/
export/import. Performance requirements are authority p95 ≤5 ms, native/WebGPU
frame p95 ≤16.7 ms, WebGL2 Low ≤33.3 ms and input p95 ≤100 ms. Endurance is
36,000 mixed/replay ticks and 100,000 ticks with bounded memory. These are
requirements, not measurements (qualification §6, lines 161–178).

## Current Rust source seams

The engine module index is `src/engine/mod.rs:1`: backend, GPU, input, primitives,
quality, render, render_frame, resources, scene_renderer, shader, time,
ui_renderer. There is no separate `src/renderer/` directory. Platform selection
is `src/platform/mod.rs:1`; host composition is native/web, with most lifecycle
and input plumbing currently in `src/app.rs`. `src/ui.rs:5` indexes retained UI,
semantics, clipboard, text editing, Library/Workshop/creator and projection/SDF
adapters. These are decomposition seams, not independent replacement libraries.

| Seam | Exact current source / capabilities | Replacement contract |
| --- | --- | --- |
| Host entry/composition | `src/main.rs:3` Tokio multi-thread host; `src/platform/native.rs:79` native run; `src/platform/web.rs:109` wasm start / `:115` browser run | Native window/event loop stays on main thread. Bind authority, stores, coordinator, audio and transfer before sessions use them. Browser host owns JS event callbacks and async completions. |
| Profile injection | `src/platform/native.rs:40` absolute nonempty `NYON_PROFILE_DIR`, `:52` scenario, `:59` Workshop, `:69` Living, `:73` global coordinator, `:79` audio | Resolve once, support non-UTF8 native absolute paths, retain namespaces, use owned test profiles. No real-profile writes for qualification. |
| Window/lifecycle | `src/app.rs:2079` resumed; `:2130` suspended; `:2229` close/resized/scale/redraw; `:2380` occlusion; `:2394` focus | Suspend clocks/input as required, clear held pointers/keys on lost focus, hold durable close until selected authority/coordinator commits, preserve valid session through surface recovery. |
| Native GPU initialization | `src/engine/gpu.rs:35`, `src/app.rs:471` prepare surface synchronously then schedule adapter/device acquisition; `:514` generation mailbox acceptance | Only the current generation installs resources; stale/error completions cannot replace active resources. Finish configure/renderer work on window thread. No unreviewed assumption that a foreign graphics object is thread-safe. |
| Browser initialization | `src/app.rs:530` fresh per-generation mailbox with `spawn_local`; `src/platform/web.rs:22` graphics ready/failed events | JS Promise completion is asynchronous and owner/generation checked; cleanup old canvas, handlers and resources before fallback. |
| Input/text | `src/app.rs:2237` modifiers/keyboard, `:2243` IME preedit, `:2251` commit; `:2264` pointer, `:2306` pinch, `:2322` wheel, `:2357` touch | Preserve logical positions, touch IDs/cancellation and selection-before-launch policy, composition distinct from committed text, caret/selection, action/focus routing and modal capture. Native IME requires more than keycodes. |
| Semantic tree | `src/ui/accessibility.rs:50` input owner; `:89` roles; `:117` node; `:241` tree; `:315` focus manager | Stable semantic/action IDs, owner-bound event queue, selected/disabled/value/expanded/focus state, descriptions, announcements and modal focus. All supported creator tools must be exposed. |
| Native accessibility | `src/ui/platform_native.rs:18` native adapter; `:129` full AccessKit tree; `src/app.rs:2101` AccessKit winit bridge created before visibility; `:2156` initial tree/action requests | Real OS accessibility adapter with stable identity and action/edit mapping, not an internal semantics-only test. Current Cargo AccessKit is a Rust dependency, not a verified Zig-callable C library. |
| Browser accessibility | `src/ui/platform_web.rs:29` DOM semantic mirror, `:69` owner/signature sync; `:120` caret/focus capture; `:174` node/ARIA/input/button conversion | Preserve DOM roles, live announcements, actual editable inputs, disabled/selected attributes, handlers and UTF-16 caret restoration; no stale editor event may mutate replacement state. |
| Clipboard | `src/ui/clipboard.rs:8` 1 MiB cap; `:120` one worker mailbox, cancellation retains worker lane; `:265` macOS pbpaste/pbcopy; `:143` non-macOS unsupported | Complete owned UTF-8/text selection contracts; failed/stale paste preserves field. Existing Windows/Linux unsupported behavior is a current gap, not full clipboard acceptance. |
| Graphics surface | `src/engine/gpu.rs:75` resize; `:92` recreate; `:182` zero-extent suspension/default config; `src/app.rs:104` surface decisions | Acquire/configure/present, device/surface loss, timeout/outdated/suboptimal/OOM/error policy, extent/format changes and renderer recreation without changing authority. |
| Backend/capability | `src/engine/backend.rs:9`, `src/engine/gpu.rs:115` browser feature isolation / `:136` device acquisition | Metal/DX12/Vulkan/GL/WebGPU/WebGL2Low diagnostic identities remain presentation only. WebGL2 uses downlevel limits and CPU advisory fallback when compute unavailable. Native `prepare` presently requests `Backends::PRIMARY`, despite GLES being a Cargo feature: a forced native GL selector needs explicit design/implementation and evidence. |
| Quality/resources | `src/engine/quality.rs:16` monotonic optional High state, `:43` resolver; `src/engine/scene_renderer.rs:119` pipelines, `:496` surface targets | Required Low before optional High; High failure disables High for that device epoch. High: 4× MSAA/bloom/full particles; Low: 1×/no bloom/quarter particles. Preserve user requested vs effective quality and visual-motion isolation. |
| Renderer | `src/engine/render.rs:46`, `src/engine/scene_renderer.rs:95`, `src/engine/resources.rs`, `src/engine/render_frame.rs`, `src/engine/ui_renderer.rs:57` | Command submission, bounded checked uploads, instanced opaque/transparent draws, canonical back-to-front ties, depth, alpha, R8 SDF atlas/filtering, clipping/panels, postprocess and graceful resource failures. Preserve host/shader structure sizes/offsets. |
| Advisory compute | `src/advisory/gpu.rs:12`, `:55` capability check, `:156` submit and async readback | Storage buffers, compute dispatch and map/readback callbacks; exact request metadata/freshness and parity before publication, monotonic device-epoch CPU fallback. Does not mutate authority. |
| Audio | `src/living_audio.rs:15` settings, `:88` limiter, `:109` host controller; native rodio at `:216`, browser WebAudio at `:272` | Master/ambience/music/effects 0–100 and mute; 82.4/123.5 Hz layers and four cue frequencies; one category per rolling second/four total; stop on presentation change, no-device graceful behavior, browser user gesture resume, persist preferences without affecting digest. Native cue envelope and browser ramp presently differ; preserve/review intentional policy rather than silently assume identical waveforms. |
| Content/assets | `src/ui.rs:27` embedded atlas/metrics/WGSL; `assets/{workshop/core-pack-v1.json,living/core-pack-v2.json}`; `src/living_content.rs`/`living_chronicle.rs`/`ui/guide.rs` | Offline assets/catalog/content, retained license/hash provenance, authentic production templates/seven experiments. No network content dependency and no authority/content substitution. |
| Stores/global Continue | `src/{scenario,preferences}/store.rs`, `src/workshop/store.rs:441`, `src/living_store.rs`, `src/library_selection.rs:214`, host composition above | Bounded requests/typed outcomes, separate authority namespaces, exact-catalog loading, generation CAS, selected slot/branch/history identity and cancellation/abandon behavior. Host workers must be bounded and poll must not block. |
| Portable files | `src/app/transfer.rs:183` handoff outcomes; `src/living_transfer/native.rs:22` bounded read / `:44` durable write; browser `src/living_transfer/web.rs:30` file input/download | Actual picker cancel/import/export, cap-before-read and readback authentication, immutable source, generation owner isolation. Native DurableSaved follows flush/fsync/atomic placement; browser DownloadStarted is handoff only and requires later imported-copy verification for portable qualification. |

The Task 3 app/advisory inventories carry the exact pure clock/camera/editor/
input/controller policies. Task 6 should consume reviewed values/interfaces,
not recalculate commands from rendered widgets or duplicate authority state.

## Shader, text and generated asset boundary

Current graphics dependencies are `wgpu = 30.0.1` and `naga = 30.0.1`
(`Cargo.toml:30,36` and lockfile). Native Cargo features name DX12/GLES/Metal/Vulkan;
browser builds select one of WebGPU or WebGL, disable defaults and emit separate
artifacts (`tools/build-web-webgpu.sh`, `tools/build-web-webgl.sh`). Existing
shader source is WGSL:

| Asset | Host use |
| --- | --- |
| `assets/shaders/primitives.wgsl` | Packed 36-byte `Vertex`, 16-byte logical-pixel globals, primitive kind/ring-width packing |
| `space.wgsl`, `worlds.wgsl`, `routes.wgsl` | Procedural background/depth-tested worlds, instanced transparent routes/fleets/halos/particles |
| `postprocess.wgsl` | Bloom texture/sampler composite |
| `ui_sdf.wgsl` | R8 atlas texture, sampler, UI globals, panel/glyph instanced draws |
| `advisory.wgsl` | Three storage buffers; `@compute @workgroup_size(8)` |

`src/engine/shader.rs:18` currently parses and validates WGSL through Naga before
pipelines. A direct Metal/GL/DX12/Vulkan route must supply equivalent validation
and shader compilation/translation; renaming WGSL or linking Metal cannot do
that. Naga's Rust implementation cannot remain a production runtime if Task 8
requires no residual Rust runtime. Any retained build-time generator needs a
reviewed tooling-only boundary and reproducible generated-artifact checks.

`src/ui.rs:27–36` fixes 1024×1024 R8 atlas, 8,192 glyphs, 512 panels, printable
ASCII Regular/SemiBold and frozen icons. `src/ui.rs:666` rejects non-ASCII text;
it is not a Unicode shaping stack. `src/ui/text_edit.rs` and native/browser
composition handling accept text independently. A future wider name/text
rendering decision must resolve this mismatch explicitly. CoreText/FreeType/
HarfBuzz availability does not authorize changing accepted asset/layout identity.

`assets/ui/PROVENANCE.md` records Inter 4.1/OFL, Lucide frozen SVGs/ISC, fixed
SDF generation parameters and the isolated Rust atlas-builder workspace.
The game embeds generated bytes and metrics; runtime does not fetch fonts.
Preserve `atlas.r8`, `atlas-metrics.json`, `hashes.json` and licenses together;
do not hand-edit generated output. Atlas generation is developer tooling and
requires a separate Task 8 decision if the final delivery language rule also
covers tooling. Current contract tests: `tests/{shaders,renderer_contracts,
ui_assets,advisory_gpu,workshop_accessibility}.rs` and Library transfer suites.
Reading these contracts is not running them; `advisory_gpu` can skip.

## Actual persistence/browser host APIs

Classic filesystem paths are source-defined (`src/scenario/store.rs:87–149`,
`src/preferences/store.rs:93`): macOS Application Support/NYON, Windows APPDATA/
NYON, Linux absolute XDG_DATA_HOME or HOME/.local/share/nyon. Files stay
`scenario-v1.json` and `preferences-v1.json`; caps are 65,536 bytes and 16 KiB.
Primary absence alone permits read-only legacy fallback; error/present invalid
primary wins. Native save uses bounded read, temp file, flush, sync and atomic
placement. Task 2 owns this slice; Task 6 must integrate its reviewed adapter.

Workshop stores are `workshop-v1` (16 slots, 16 MiB archives, 1 MiB packs);
`src/workshop/store/native.rs:769` holds an OS lock, rereads manifest, uses CAS
and durable archive/manifest replacement. Worker/abandon contracts are at
`:688–767`. Living native `src/living_store/native.rs` uses isolated `living-v2`,
alternating generation manifests/head, authenticated payloads, durable staging
journal and recovery/reclamation. Global selection lives in independent
`library-selection-v1.*` records; `src/library_selection/native.rs:78–138`
locks, compares and commits durable selection only after target authority
transaction completion. These multi-file protocols cannot be replaced by a
single generic JSON save operation.

Browser host needs the following real browser APIs; they currently enter through
wasm-bindgen/web-sys/js-sys and inline JS, not a native C ABI:

| API | Source-defined obligation |
| --- | --- |
| WebAssembly import/export + event/timing host | Owned copied byte spans or retained lifetime-safe handles, event-loop yield, allocation failure, generation-tagged Promise completions, bounded work and cleanup. A future custom JS import ABI must replace wasm-bindgen glue explicitly. |
| DOM/canvas | Canvas lifecycle/DPR/CSS extent, pointer/touch/wheel/keyboard/composition/focus, DOM semantics, announcements, file picker gesture and application errors. |
| `navigator.gpu` | Adapter/device requests, canvas context, shader/pipeline/buffers/textures, queue submissions/error/device loss and advisory compute/readback. |
| WebGL2 context | Actual GL2 context + shader/link/resource/draw/fallback/error recovery; WGSL-to-GLSL or reviewed GLSL assets must replace current wgpu translation. No compute assumption. |
| `localStorage` | `nyon.scenario.v1` with `intergalactic-warfare.scenario.v1` fallback (`src/platform/web.rs:17`), `nyon.preferences.v1` with separate legacy key (`src/preferences/store.rs:11`). Access denied/missing/quota errors remain explicit. |
| IndexedDB Workshop | `nyon.workshop.v1`, version 1, `slots`/`archives`/`packs` object stores (`src/workshop/store/web.rs:10`); real schema upgrade, transactional CAS, commit/abort/quota/eviction/recovery. `wasm.rs:1075,1273,1324,1471` bind transactions/open/errors. |
| IndexedDB Living | Separate `nyon.living.v2`, version 1, `living` object store; snapshot compares both recovery generations and head, then atomically puts payloads/manifest/head and deletes garbage (`src/living_store/web.rs:11–69`). Loading authenticates type/length/hash and incremental replay. |
| IndexedDB global coordinator | Separate `nyon.library.v1`, `selection` store; exact snapshot CAS updates recovery record/head together (`src/library_selection/web.rs:7–65`). Authority/selection databases remain separate; coordinator publication ordering and failure recovery matter. |
| Files/Blob/URL | `File.array_buffer`, bounded import, Blob object URL/download, delayed revoke and cleanup (`src/living_transfer/web.rs:30–160`). DownloadStarted is not durable readback. |
| WebAudio | AudioContext/oscillator/gain/resume/disconnect/currentTime (`src/living_audio.rs:272–371`), gesture restrictions/no-device/mute and persisted `nyon.living.audio.v1` settings. |

`web/loader.js:1` defines separate WebGPU/WebGL artifact imports. It supports
`?backend=webgl2`, preflight `navigator.gpu.requestAdapter`, a 20-second graphics
ready/failed handshake, teardown then WebGL2 after WebGPU initialization failure,
and an actionable both-failed status (`:208–263`). New imports may emit different
artifact names, but the loader contract must still qualify actual selected
backend and successful renderer readiness. Artifact import alone is insufficient.
Cold offline acceptance means assets/build output can be served with networking
disabled; service-worker caching/deployment policy was not inspected or selected.
Browser tab/OS termination cannot be assumed to wait for arbitrary async saves;
the design must distinguish application-controlled durable close from crash/tab
termination and exercise interrupted-transaction recovery.

## Installed local binding evidence

Read-only commands: `xcrun --show-sdk-path`, exact framework/header and linkage
stub listings, targeted `rg` declarations, Homebrew include/lib/Cellar listings,
`file -L` on named libraries. No binding test was compiled or called.

The selected SDK is:

`/Applications/Xcode-beta.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk`

Below, `SDK/` means that exact path. SDK stubs are link descriptions, not proof
that a target deployment OS supplies every advertised API. Actual SDK version,
minimum OS, codesigning/distribution and Intel/universal binary policy are
unselected.

| Available local surface | Exact evidence / callable declarations | Boundary and limitation |
| --- | --- | --- |
| ObjC runtime C entrypoints | `SDK/usr/include/objc/{objc,message,runtime}.h`; `SDK/usr/lib/libobjc.tbd`; `runtime.h:262` objc_getClass, `:723` class_addMethod, class allocation/registration declarations | Possible Zig extern bindings/typed message dispatch and classes; native ObjC header syntax is not automatically a complete Zig import. Method ABI, blocks, structs, retention/autorelease and callback lifetime require review. |
| AppKit | `SDK/System/Library/Frameworks/AppKit.framework/{Headers,AppKit.tbd}`; `NSApplication.h:262,356,359` run/sendEvent/nextEvent; `NSTextInputClient.h:37,45` insertText/setMarkedText; `NSAccessibilityProtocols.h:39,193,386,842` tree/press; `NSSavePanel.h:201` async completion | Window/events/IME/accessibility/panels are locally available system APIs. A complete NSTextInputClient/accessibility implementation is still application work. Need owned native tree/focus/actions and main-thread behavior, not just selectors. |
| Metal + QuartzCore | Metal/QuartzCore Headers + `.tbd`; `MTLDevice.h:130` MTLCreateSystemDefaultDevice, `:644` newCommandQueue, `:787` newLibraryWithSource; `CAMetalLayer.h:93,102` drawableSize/nextDrawable | C device creation plus ObjC Metal command/resource/layer protocol. Native Metal is macOS route only. Shader translation and required other native/browser backends remain open. nextDrawable may block/time out; cannot assume negligible main-thread cost. |
| MetalKit | Headers + MetalKit.tbd | Convenience native view available; no view host/runtime was exercised or chosen. |
| CoreText/CoreGraphics/Foundation | Headers + framework `.tbd`; `CoreText/CTLine.h:139,352` attributed-string line creation/typographic bounds | Possible system text metrics/raster/shaping and supporting data/object APIs; current embedded ASCII atlas requires no new font runtime. Not an automatic portable atlas/shaping replacement. |
| AudioToolbox/CoreAudio/AVFoundation | Headers + framework `.tbd`; `AudioToolbox/AudioQueue.h:893,1062,1142,1242` NewOutput/AllocateBuffer/EnqueueBuffer/Start | C AudioQueue supports possible procedural PCM output. Callback ownership, real-time-safe generation, device change and graceful failure remain unimplemented. These are macOS APIs. |
| OpenGL | `SDK/System/Library/Frameworks/OpenGL.framework/Headers/gl3.h`, OpenGL.tbd | Desktop GL declarations available locally; does not qualify Ubuntu forced GL or browser WebGL2, and native context/shader strategy is unselected. |
| SDL3 3.4.18 | `/opt/homebrew/include/SDL3/SDL.h`; `SDL_version.h:47,56,65`; `/opt/homebrew/lib/libSDL3.dylib` → Cellar/sdl3/3.4.18 | Available native C API; inspected dylib is Mach-O arm64. SDL_video.h:1184 CreateWindow; :2013 GetWindowSizeInPixels; :999 GetWindowDisplayScale; SDL_events.h:1310 PollEvent; SDL_keyboard.h:378,550 text input/area; SDL_clipboard.h:104,123; SDL_metal.h:75,103 view/layer; SDL_audio.h:735 audio device; SDL_dialog.h:166,215 async open/save dialogs. Linking it into NYON would add a production dependency and needs approval despite local installation. |
| SDL3 GPU API | `SDL_gpu.h:29` names Metal/Vulkan/D3D12; `:2256` CreateGPUDevice; `:1037–1041` SPIR-V/DXBC/DXIL/MSL/metallib formats | Candidate native renderer seam. No WGSL/WebGPU/WebGL2/GL backend appears in the inspected declared driver/format contract. Requires shader tooling and a separate GL/browser path; not a drop-in wgpu replacement. |
| SDL2 compatibility | `/opt/homebrew/include/SDL2/SDL.h`; `libSDL2.dylib` → Cellar/sdl2-compat/2.32.74 | Available compatibility shim; inspected dylib arm64. Not evidence of independent original SDL2 implementation, and adoption still requires new-dependency approval. |
| FreeType/HarfBuzz | `/opt/homebrew/include/freetype2/ft2build.h`, `include/harfbuzz/hb.h`; named dylibs/static archives; Cellar/freetype/2.14.3 and harfbuzz/14.6.0 | Available C interfaces for a possible future text pipeline; dylibs arm64. New production dependencies need approval. Current atlas does not require runtime rasterization/shaping. |

No matching GLFW, wgpu-native, Dawn/webgpu, Vulkan/MoltenVK, Sokol, miniaudio or
Emscripten names were found in the inspected `/opt/homebrew/include`,
`/opt/homebrew/lib` and targeted Cellar candidates; `/usr/local/include` and
`/usr/local/lib` were absent in the bounded listing. This does not prove these
products are absent everywhere on disk. Cargo contains Rust wgpu/Naga sources
and compiled artifacts, which do not establish a usable native C distribution.
No Windows/Linux SDK/header/library or cross-target package availability was
established by the macOS inspection.

## Bounded alternatives for design review

No alternative is selected or authorized for implementation by this report.

| Proposed alternative | Existing usable starting evidence | Main unresolved cost |
| --- | --- | --- |
| Zig OS-specific hosts + direct graphics per platform | macOS SDK AppKit/Metal/AudioQueue/ObjC C runtime | Full ObjC lifetime/IME/accessibility implementation; separate Windows DX12 and Linux Vulkan/GL hosts; shader validation/translation; browser JS WebGPU/WebGL2. System bindings need no third-party package install, but no portable complete host is proved. |
| Zig host using approved SDL3 native C API + separate renderer/semantic adapters | Installed SDL3 window/events/IME/audio/clipboard/dialog APIs and native metal-layer access | Reproducible pin/link/package/distribution on all hosts; accessibility still needs a complete reviewed adapter; either direct graphics or another approved library. SDL3 GPU route requires translated native shader formats and separate GL/browser implementation. |
| Zig native front end using newly approved portable C graphics/accessibility libraries | No matching local native WebGPU C library or AccessKit C distribution established | Select/pin/provision/review license/ABI/builds first. A Rust-built wgpu-native or Rust accessibility runtime would conflict with unqualified Task 8 no-residual-Rust production wording; cannot declare it acceptable automatically. A C++ graphics library also adds a dependency/build/distribution surface. |
| Zig wasm + project-owned JS browser adapter | Existing loader, DOM mirror behavior, inline IndexedDB transactions and WebAudio/file protocols are source witnesses | Explicit memory/handles/import ABI, event/promise lifecycle and full JS graphics bridge; preserve both browser backends and all semantic/storage/fallback paths. This browser option can pair with either native route. JS platform glue scope under “all production entrypoints Zig” needs a written accepted boundary. |

Keeping existing Rust winit/wgpu/AccessKit/rodio/rfd application code behind a
C facade may be useful comparison scaffolding only if explicitly designed; it
would not satisfy final retirement of Rust production paths. Neither locally
installed third-party libraries nor an early migration prototype grants
standing dependency or implementation-language exceptions.

## Concrete questions before Task 6 implementation dispatch

1. Define the final allowed foreign-code boundary: OS framework FFI, JS platform
   adapter, generated shader tooling, and any C/ObjC shim. What constitutes
   retired Rust production code? Any proposed Rust-built runtime exception must
   be explicit, not hidden behind a C ABI.
2. Select the complete renderer path for Metal/DX12/Vulkan/forced GL/WebGPU/
   WebGL2. Direct API and SDL3 GPU routes need shader formats and reproducible
   tooling; no available library here accepts current WGSL for all six paths.
3. Select native accessibility strategy on all three OSes. Direct AppKit can
   cover macOS; this report establishes no Windows UIA/Linux AT-SPI or portable
   Zig-callable semantic bridge. Sparse Classic semantics remain a known full
   accessibility acceptance gap in the migration plan.
4. Define ownership/thread/async ABI and generation cancellation before wiring
   renderer, stores, clipboard, file dialogs or semantic edits. Native callbacks
   must not retain temporary Zig slices; browser growth/copy/Promise handles
   must not expose stale memory. Resource cleanup must remain bounded.
5. Preserve store crash/CAS contracts across Zig filesystem primitives on all
   OSes, including Unicode/non-UTF8 paths, file locks, atomic replacement,
   directory sync/recovery and bounded worker lanes. Browser commits complete
   only on transaction completion, not request success. Check cross-tab CAS,
   quota, eviction, blocked upgrade and interrupted coordinator publication.
6. Review shader host packing, numeric matrix conventions, texture/depth/blend
   formats, MSAA/resource fallback and logical-vs-physical sizing independently
   of any new backend. Optional compute advisory must fail back to CPU with
   exact metadata/freshness/parity policy.
7. Decide text coverage against current accepted names/creator text. The atlas
   is ASCII while editing/composition is broader; installing a shaper does not
   itself resolve visual/caret/semantics/layout parity.
8. Define bundle/link delivery: target OS minimums/architectures, dynamic
   dependency packaging, licenses, offline artifacts, local system-framework
   access and reproducible shader/atlas identities. Homebrew local paths cannot
   be the sole delivered runtime resolution strategy.
9. Allocate real qualification hosts and name device/browser versions. Source
   and artifact/static gates do not replace GPU parity, screen reader, touch,
   storage interruption/readback, performance, endurance or offline tests.

This inventory is a design prerequisite. It selects no binding, changes no
runtime behavior and closes no Task 6 or Task 8 acceptance row.
