# NYON full game and UI/UX discovery — 2026-10-19

Status: Discovery, not an approved implementation design. Rust parity-first recovery remains active; tests and CI remain paused. New product requirements do not retroactively qualify existing ports.

## User decisions

- Preserve all creator, ruler and combined capabilities and expose distinct world modes, using Minecraft-like mode choice as the reference. Do not force one role into the only default experience.
- Fully3D cube worlds with gravity shifting between faces; all modes remain available.

Interpretation to confirm through interaction design: six connected faces of a cube planet each have local down; edge crossing changes the gravity frame. The user did not yet specify a character controller, first/third person, voxel terrain, survival mechanics, multiplayer or a physics model. Minecraft was a mode/world reference, not approval to invent all of its mechanics.

## Already-established requirements

Creator/ruler/combined roles; connected cube planets; cities/economy; strategic diplomacy/logistics/research/ecology/fleets; tactical ground combat; durable branches/replay/Continue; native/browser delivery; accessible coherent controls and explicit provenance. Current Classic/Living/Workshop screens and tutorials are retained behavior, not final product architecture.

## Experience refinement — 21:30 EDT

User requests extensive customization across direct and strategic control, a cube that does not itself rotate, and a cube world described as expanded from its center. Working interpretation: both control experiences are available, the world has a stable orientation, and player/camera motion provides navigation. Exact world geometry, terrain representation and customization permissions are not yet specified. Do not infer an implemented avatar or physics engine.

## Gravity decision — 21:31 EDT

User authorizes supporting all gravity choices and delegates the best default. Selected design default: face-perpendicular gravity, smooth camera transitions across edges, stable cube orientation. Center-directed gravity is an optional per-world model. This is a product design decision, not a compiled or qualified physics implementation. Terrain representation, edge geometry, gravity-switch lifecycle and controller behavior require the later written design.

## Delivery decision — 21:32 EDT

User selected complete single-player first, including all world modes and customization. Multiplayer requires a separate later design; do not introduce networking, shared-world clocks or remote ownership into the initial recovery.

## Terrain decision — 21:32 EDT

User selected both editable block-level terrain and city building: digging/tunneling/building plus larger structures, roads and infrastructure. This expands scope beyond the recovered two-dimensional surface grid. A new versioned volumetric terrain design is required, with explicit relationships between block coordinates, face/cell authority, construction footprints, gravity and persistence. Existing surface/wire contracts must not silently change.

## Population decision — 21:34 EDT

User requests both population-level simulation with visible residents and persistent individual citizens. Treat these as explicit simulation profiles, recorded in world metadata and replay rules. Camera zoom must not silently change canonical simulation semantics. Detailed citizen mode requires separate ownership, needs/movement/job/storage and capacity contracts; representative residents must not be presented as persistent citizens. Population-level remains the recommended initial implementation/default, not a deletion of individual simulation scope. Profile conversion during an existing world is not yet specified.

## Integrated first-loop direction — 21:35 EDT

User requests combining the best of explorer-to-founder, civilization ruler and creative world builder. Proposed integrated first journey: choose a customizable preset/mode; start near a functioning settlement; explore across cube faces; dig/build/reshape terrain; expand the settlement; switch to strategic city/civilization management; inspect autonomous outcomes; use explicitly permitted creator interventions; save/quit/resume the same world/history. The functioning-settlement default and exact tutorial beats are design synthesis, not new user-approved numerical mechanics. Empty/creative starts and productive ruler capitals remain available presets.

## UI discovery format — 21:36 EDT

User declined the browser visual companion; continue text-only and do not offer it again unless the user raises it. No mockup server/browser was launched. Proposed layout baseline: compact exploration HUD, contextual strategic city/civilization panels and an explicitly marked creator workspace; configurable panel placement/scale, shortcuts and information density. This is a proposed synthesis for the later UI specification, not delivered interface behavior.

## Customization and failure decisions — 21:37 EDT

User requests all failure styles as customization: recover/rebuild in a continuing world, checkpoint restoration and harsh permanent loss. Selected default: recover/rebuild. Exact loss consequences and restoration/branch ownership still require specification. Do not silently rewind the world on ordinary respawn.

Standing interpretation of repeated user direction: customization is broad; prefer coherent selectable presets/defaults rather than repeatedly asking which optional feature to exclude. New world creation should expose Quick Start presets and an advanced Customize path for role/mode, terrain, gravity, simulation profile, difficulty, controls and UI layout. Avoid requiring every setting before initial play. This UI flow is proposed for review.

## Visual-theme decision — 21:38 EDT

User requests all three proposed visual identities: stylized living cube worlds; grounded frontier civilization; bold alien sandbox. Support them as visual theme presets. Selected default is stylized living cubes with rich natural colors and restrained science-fiction UI. Appearance/layout changes alone must not silently alter canonical terrain, collision, gameplay or history. No visual companion is authorized; discussion remains text-only.

## Proposed experience foundation (presented for correction)

- World creation: Quick Start/Customize, creator/ruler/combined modes, terrain/gravity settings, citizen simulation and failure rules; reusable configurations.
- Exploration/building: selectable first/third person, digging/tunneling/block construction plus larger structures; stable cube orientation and connected face transitions.
- Governance: strategic control in the same world; inspectors explain shortages/jobs/housing/production/freight/diplomacy/combat and blocked actions.
- Creator workspace: explicit permission/provenance, previews, branches and comparisons; never a concealed ruler bypass.
- Persistence: save/quit/Continue exact state, configurable interface layouts/themes, failed operations preserve active state.

One world and history underpin these views. This synthesis is awaiting user correction/approval; it is not an implemented UI or approval of unstated first/third-person mechanics.

## Galactic-first correction — 21:39 EDT

User clarifies that every discussed behavior needs customization options, while the primary game is a galactic sandbox at huge scale. Do not frame the product as a single cube-world builder with optional galaxy content. Direct exploration, terrain/city building, ruling and creator tools are ways to inhabit/manage one larger persistent galactic sandbox. The earlier one-world journey is a local introductory slice, not the complete product scope.

Scale navigation must support universe/galaxy/system/world/surface/city and relevant tactical/entity views, with search, selection continuity, clear scope and persistent history. Huge scale is a user requirement, but no numeric first-release capacity or unbounded-simulation performance claim is established.

## Universe direction — 21:41 EDT

User requests combining the best of procedurally expanding regions, an enormous finite galaxy and multiple finite galaxies. Proposed synthesis, awaiting review: one persistent universe with finite seeded galaxies, generated region by region as explored; additional galaxies can be discovered/generated beyond the starting galaxy. Universe generation presets can select finite-single-galaxy, finite-multi-galaxy or expanding-universe behavior. Expose size/density/biome/abundance/civilization/discovery settings through presets and advanced customization.

Each galaxy retains a meaningful spatial map and frontier; discovered identity/content stays stable across revisits, saves and generator versions. Persist changes rather than regenerating modified regions. New-generation settings and physical region/capacity rules need a written contract; no claim of infinite resident memory or unlimited active simulation. Rendering detail streams independently from the saved simulation profile; camera movement cannot silently change canonical simulation semantics.

This enlarges the future architecture beyond the retained seeded-manifest profile, whose limits/identity remain preserved. The galactic sandbox should be decomposed into universe generation/streaming, volumetric cube terrain/gravity, resident simulation, role/control/UI, and city/strategy/combat/durable delivery designs. The initial universe experience contract is the first design section; dependencies are explicit rather than one giant implementation plan.

## Current question

Review the proposed hybrid universe structure and default galactic navigation/discovery behavior before writing the universe specification. No universe-generation, terrain or avatar implementation is authorized by this design response alone.

## Later design questions (ask one at a time)

World mode permissions and switching; gravity/edge traversal and camera comfort; building/terrain interaction; city autonomy and direct control; objectives/progression; conflict and loss; navigation across scales; default information density; critical-event time behavior; onboarding/help/accessibility; art/audio/performance and platform input parity. These are discovery topics, not decided features.

## Implementation boundary

Do not implement new avatar/gravity/voxel/UI mechanics until their architecture and written design are reviewed. Keep parity ports distinguishable from newly invented gameplay. Split the expanded experience into dependent subsystem designs and plans after user choices stabilize.
