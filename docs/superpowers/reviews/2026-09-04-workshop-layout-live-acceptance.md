# Workshop Layout Live Acceptance — 2026-09-04

## Outcome

**PASS for the blank-canvas and responsive-chrome repair.** Fresh native, natural-WebGPU, and forced-WebGL2 runs established that the repaired Workshop canvas is no longer hidden by an opaque application overlay and that the persistent compact controls remain sighted and usable after the two live-found overlap repairs.

This acceptance closes only the live evidence gap attached to the scoped Foundation Task 1 layout repair. It does not close the separate missing-star visual, incomplete sighted inspector, SDF typography/icon, or broader Living Galaxy rendering work.

## Build and static evidence

- `./tools/build-web.sh` completed for both pinned backend artifacts after the final Rust frame-composition repair.
- `./tools/check-workshop.sh` passed after the rebuild:
  - renderer contracts: 8 passed
  - shader contracts: 8 passed
  - Workshop web contracts: 9 passed
- Controller-focused `cargo test --test workshop_ui --test workshop_web --test renderer_contracts` passed 39 tests before the final rebuild.
- The independent layout re-review approved both follow-up repairs with no P0-P2 source finding.

## Native observation

- `cargo run --release` launched the current native binary and reported the `METAL` backend inside the Workshop diagnostics.
- Continue loaded an authoritative Workshop save; the scene canvas remained visible between the side chrome and timeline.
- Creating a star and then a world through the real creator dialogs updated hierarchy rows, revision history, digest, generation, and Continue selection.
- The center system/world presentation marker remained in the scene rather than disappearing behind the platform overlay.
- Selecting the world updated the semantic inspector and removal action.
- This run also reconfirmed two separate open product gaps: a created star has no distinct visual instance, and the sighted inspector exposes far less information than the semantic model.

## Natural WebGPU observation

- `http://127.0.0.1:8000/web/` reported `WEBGPU initialized successfully` from the freshly rebuilt artifact.
- The browser emitted one bounded warning that optional High scene pipelines did not resolve their internal error scope and the device epoch continued with Low graphics. Startup and input remained operational; this is not evidence that High graphics rendered.
- At the default 816-pixel-wide in-app viewport, New Workshop entered Compact mode with a full-width scene, persistent Create/Navigator controls, Main Menu, Guide, timeline, and Save controls.
- Before the follow-up fixes, live inspection found the ready diagnostic covering Create/Navigator and then found the product title drawing through those controls. Those observations drove the two reviewed repairs.
- After rebuilding and reloading, the ready diagnostic remained semantic but no longer painted over the app, and Compact/Medium suppressed the sighted product title. All four persistent top controls were unobscured.
- With an explicit 723x802 browser viewport, the same four controls remained unobscured, the Creator drawer opened, its paged tool grid was usable, and its Previous/Next/Close controls remained reachable above the timeline.
- In High Contrast at 723x802, creating a system through the real compact creator dialog produced a clearly sighted white ring/core marker on the full-width black canvas.
- The temporary exact viewport override was reset after the check.

## Forced WebGL2 observation

- `http://127.0.0.1:8000/web/?backend=webgl2` reported `WEBGL2 LOW initialized successfully` from the freshly rebuilt forced-fallback artifact.
- A fresh reload after the final repair showed the shell controls and no ready diagnostic overlay.
- Browser console inspection returned no warning or error entries for this forced-WebGL2 run.
- This proves only the observed startup/shell path and backend selection. It does not substitute for a complete WebGL2 gameplay, persistence, or accessibility matrix.

## Remaining visual/product gaps

- Star entities are present in authority and semantics but have no distinct scene instance.
- Normal-mode system/world markers are intentionally minimal and need the planned contrast/selection/visual-hierarchy pass; final graphics quality is not accepted by this layout check.
- The sighted inspector does not yet expose the detailed entity facts available in the semantic tree.
- Platform chrome still uses the primitive bitmap text path; SDF typography and explicit icons remain planned.
- Real VoiceOver behavior, browser assistive-technology interaction, native 723x802 window resizing at 115%/130%, and long-session persistence remain separate acceptance layers.
