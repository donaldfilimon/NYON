# Workshop V1 Star Visuals — Live Acceptance — 2026-09-04

## Verdict

**ACCEPTED FOR THE WORKSHOP V1 BASELINE.** Fresh artifacts render the upgraded Workshop star marker on native Metal, browser WebGPU, and forced browser WebGL2. The native failure that originally showed only the system ring is no longer reproducible after routing filled discs through the shared full-width kind-2 ring path.

## Exact artifacts exercised

- Native: `cargo build --release --all-features`, then the resulting `target/release/nyon` was copied into an isolated temporary `.app`, stripped of inherited quarantine metadata, ad-hoc signed, verified with `codesign --verify --deep --strict`, and launched as a new process. The previously exported app and its stale binary were not used as evidence.
- Browser WebGPU: both browser bundles were rebuilt with `./tools/build-web.sh`; the natural `http://127.0.0.1:8000/web/` route was reloaded after the rebuild.
- Browser WebGL2: the same rebuilt shell was reloaded through `http://127.0.0.1:8000/web/?backend=webgl2`, forcing the WebGL2 bundle and low-quality scene path.
- Static/browser contract gate: `./tools/check-workshop.sh` passed after both rebuilt bundle timestamps were current.

## Live observations

### Native Metal

- The semantic diagnostics reported `Graphics backend: METAL`.
- A saved one-system/one-star Workshop loaded successfully at generation 7.
- In high-contrast mode, the star displayed a luminous opaque core, a surrounding halo, and the outer system ring. This is the exact configuration that previously displayed only the system ring.
- Selecting `Star 1` through the native accessibility tree changed the semantic selection to `Star 1` and visibly added the separate outer selection ring without replacing the core or halo.
- Creating `World 1` through the typed native creator dialog completed successfully, exposed the new world in the semantic outliner, and selected the new immutable branch. The accepted source tests separately pin the world's shared full-width-disc primitive; this live scene was not treated as proof of a visually separated orbit because the default 1,000 milli-AU orbit projects extremely close to its star at the current scale.
- The create operation selected the saved Workshop as generation 8. No destructive store operation was performed.

### Browser WebGPU

- Reloaded status: `WEBGPU initialized successfully.`
- The live 814×861 canvas displayed the luminous star core, halo, and system ring in the compact layout after opening the saved Workshop.
- The first frame presented successfully.
- The browser again logged the pre-existing optional High scene-pipeline internal error-scope timeout and explicitly continued with Low graphics. Therefore this run qualifies the WebGPU Low fallback, not the optional High scene pipeline.

### Browser WebGL2

- Reloaded status: `WEBGL2 LOW initialized successfully. forced by ?backend=webgl2`.
- The live canvas displayed the same luminous core, halo, and surrounding system ring after opening the saved Workshop.
- Browser logs contained initialization, the ANGLE Metal renderer adapter, and first-frame presentation at info level only; no warning or error was recorded for the rebuilt WebGL2 run.

## Source and controller gates

- Independent source review: approved with no open P0, P1, or P2 findings.
- Controller-focused tests: Workshop presentation 5/5, Workshop UI 23/23, renderer contracts 8/8, and shader tests 9/9.
- Implementer full suite: 364 passed, 0 failed.
- Warnings-denied workspace Clippy, wasm Clippy, exclusive WebGPU and WebGL2 target checks, formatting, diff checks, and Naga shader validation passed.
- Rebuilt browser static/contracts gate passed.

## Evidence boundary

- This acceptance qualifies the Workshop star slice and its Metal filled-disc compatibility repair. It does not qualify the optional browser WebGPU High scene pipeline, pixel-perfect equality across backends, the forthcoming sighted inspector, SDF text/icon work, the Library workflow, or Living Galaxy rendering.
- Screenshots used during the run were temporary acceptance evidence and were not added to the repository.
