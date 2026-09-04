# NYON Living Galaxy: experience, interface, and diorama

Date: 2026-09-04

Status: Proposed; not an implemented-feature list

Parent: [Product and architecture](2026-09-04-nyon-living-galaxy-design.md)

## 1. The experience at every scale

The player is a galaxy architect, not an invisible civilization with unlimited resources. Creation is free; autonomous societies work within the economy. The interface distinguishes a civilization decision, a natural/scheduled event, and a creator intervention in every relevant message.

### Moment to moment

Select a world, understand its condition, preview an intervention, commit it, and see the real response. Selecting never launches a fleet or silently changes ownership. A blank-area drag never issues a command. A creator preview identifies affected objects, validation errors, dependencies, and the planned application boundary.

The default view is calm enough to observe. Ships and trade reveal activity without flooding the display. Important changes have a persistent textual explanation; the player need not notice a transient animation to understand what happened.

### First five minutes

Open **Vale Confluence**, paused, from a starter card that describes its situation. Offer **Explore while paused** and **Run**. A dismissible three-item guide introduces selection, time, and creation; it does not lock any tool. Its unscripted Main branch has no guaranteed first war. The bounded Commerce, Frontier Friction, and Interrupted Corridor fixture branches supply deterministic learning witnesses without changing their rules at runtime.

Begin framed on Aster Vale and a nearby resource corridor. Its briefing identifies a real production bottleneck. The first suggested experiment is adding power or correcting a supply route. Show the actual affected industry, its recipe, and the before/after inventory change. Guidance completion follows authoritative events, not a timer or a scripted success animation.

### First fifteen minutes

Observe a civilization choose a construction job, open a route, or settle a world. Select that event to reveal the actor's recorded reason and the relevant objects. Create a bookmark named **Before the intervention**, introduce a temporary lane hazard, and inspect the downstream shortage.

Pause, branch, and try a different intervention. Compare both branches at the same simulation tick. The view reports stockpiles, ownership, relationship states, and delivered freight differences; it does not claim that one experiment establishes a general causal law.

### First hour

Build a larger connected galaxy, follow a civilization, observe an agreement and a conflict in the appropriate scenario branches, preserve two meaningful histories, and export a galaxy worth sharing. A quiet session summary offers continued observation, branch exploration, or save-and-leave. There is no forced ending.

## 2. Worlds and scenario content

### Vale Confluence starter

The first-run curated starter has **four systems, eight worlds, and three civilizations**. This differs intentionally from the larger procedural default in the rules document. Two worlds belong to each civilization and two are unsettled frontier worlds. Use six direct lanes arranged to provide one short disputed corridor and one longer alternate route.

- **Vale Cooperative:** trader policy; reliable supply and negotiated access. Home: Aster Vale.
- **Nacre Cartographers:** expansionist policy; reaches unclaimed worlds and accepts longer supply chains. Home: Nacre.
- **Crucible Directorate:** guardian policy; secures industry and nearby corridors. Home: Crucible.

These are policy presets, not scripted destinies or moral labels. Seeded behavior and recorded interventions determine outcomes. No starting treaty or unit violates the rules catalog. The starter is validated content, not a privileged code path.

The initial session has no forced war. The starter offers recorded scenario branches demonstrating **Commerce**, **Frontier Friction**, and **Interrupted Corridor**. Each is a versioned validated fixture with its exact seed, initial manifest, catalog hash, and expected receipt witnesses. Commerce must establish a trade agreement by tick 100 and unload its first cross-civilization freight by tick 200. Frontier Friction starts with fresh mutual observations, relations -40, and an eligible assembled attack; it must declare war by tick 100, produce hostile arrival and combat receipts by tick 200, and keep simulating through tick 500. Interrupted Corridor schedules its declared ion storm before tick 50 and must show a reduced or zero dispatch and downstream production wait by tick 200. If the rules cannot produce these bounded witnesses, the fixture fails; the client never injects them as scripted story events.

An additional validated **First Expansion** fixture isolates autonomous construction/scouting/settlement. It has seed `0x4E594F4E5F455850`, two systems joined by a distance-100 lane, one founding world and one unowned frontier world, and one Neutral-policy civilization. The home begins with hub, solar, extractor, shipyard, 100 energy, 100 ore, 120 alloy, 20000 ore deposit units, and one fleet of one scout plus two escorts; it deliberately lacks a foundry and ark. The frontier has no owner/facility/fleet. The expected witnesses are: foundry job accepted at 50 and completed at 250; scout exploration ordered at 50, departed at 51, arrived/observed the frontier at 61; ark job accepted at 100 and completed at 300; settlement ordered at 300, departed at 301, arrived at 311, and claim completed at 361. No other civilizations or hazards exist. The fixture is valid only if replay produces this exact sequence and state digests recorded with the fixture.

### Other starting points

- **Blank Galaxy:** no actors or entities, with a contextual create-system invitation.
- **Seeded Galaxy:** 12 systems, 24 worlds, four civilizations by default; reproducible seed and validated generation options.
- **Library:** saved Living Galaxy sessions plus clearly labeled legacy Workshop archives and Classic scenarios.

No capability is locked behind the starter or a tutorial. Creating a 1-world diorama and simulating a large sector use the same tools and rules.

### Optional Experiments notebook

Suggested experiments are evidence-backed bookmarks, not missions that award power:

1. Establish a two-system alloy supply chain.
2. Observe settlement without creator intervention.
3. Open a trade agreement and inspect real deliveries.
4. Disrupt a corridor and identify affected industries.
5. Observe a war, then branch an alternative diplomatic setup.
6. Recover a dormant civilization through settlement or intervention.
7. Export, import, and reproduce a selected state.

Each completion links to its branch, tick, and supporting receipt. The notebook is hideable and imposes no failure timer, resource reward, repeat requirement, or artificial progression grind.

## 3. Unified interface

### Workspace regions

| Region | Contents and responsibility |
| --- | --- |
| Top bar | Galaxy name, version, save state, branch selector, command search, settings |
| Tool rail | Select, Create, Civilizations, Connections, Hazards, Layers |
| Navigator | Searchable/collapsible hierarchy, entity filters, reveal-selection action |
| Diorama | Galaxy/system/world focus, contextual labels, camera controls and selection |
| Inspector | Overview, Civilization, Economy, Connections, Environment, History |
| Transport | Pause/Run, Step, 1x/4x/20x, tick, History |
| Chronicle drawer | Filtered events, followed entities, reasons, optional pause-on-event |

Creation controls are contextual rather than permanently showing every operation. Selecting a system suggests adding a star/world; selecting a world suggests industry/deposits/ownership; selecting a civilization suggests policies/relations. A searchable all-commands palette remains available.

The inspector renders every applicable section from the same model used for semantics. Economy rows show inputs, outputs, local inventory, capacity, next completion, source reserve, and an explicit operation state: Running, Disabled, Waiting for input, Output full, Deposit depleted, Under construction, or No valid route. Several causes may be shown together; a misleading single green Enabled badge is insufficient.

A civilization panel shows owned colonies, known frontier, current policies, planned actions, relationships, active agreements, wars/truces, and recorded decision reasons. A creator can inspect both omniscient world state and the civilization's limited intelligence without confusing them.

### Responsive behavior

Choose layout using effective width `logical_width / ui_scale`.

| Effective width | Behavior |
| --- | --- |
| At least 1200 | Navigator about 240 units and inspector about 320 units around a resizable scene |
| 900-1199 | Collapsed navigator rail; docked inspector; one-click navigator overlay |
| Below 900 | Full-width scene; one mutually exclusive Navigator, Inspector, or Create sheet |

At short heights, panel bodies scroll while their title and primary action remain visible. Compact layout keeps transport reachable through a compact speed menu instead of dropping buttons. At 723x802 with 115% or 130% scale, drawers replace fixed side columns.

Every entity, branch, field, error, and action remains reachable. Use scroll/virtualization rather than fixed `.take(N)` lists. Long names have measured ellipsis in rows and full wrapped text in details. Errors and destructive confirmations wrap without hiding decisive words. A drawer's closing action remains visible even at maximum scale.

### Typography and editing

Reuse bundled Inter Regular/SemiBold and the SDF UI renderer. Default sizes before user scaling: body 15, metadata 13, section title 18, line height about 1.35. Use sentence case. Measure glyph advances for wrapping, alignment, clipping, and hit areas; do not estimate width from character count.

The first release is English-first and retains the existing validated printable-ASCII, 1-64-byte object-name contract, including no leading/trailing spaces or doubled spaces at commit. Reject an insertion/paste transaction containing unsupported characters atomically, preserving the previous draft and caret and showing an ASCII-safe error. Ordinary invalid intermediate ASCII text remains editable until commit. Do not pass unsupported draft characters into the ASCII-only SDF renderer. No silent transliteration, lossy save, or whole-frame failure. Broader Unicode font coverage is a separate internationalization change, not an implied capability of the current atlas.

Fields support caret movement, selection, select-all, copy/paste, replacement, Backspace/Delete, Home/End, and conventional word navigation. Numeric fields preserve invalid intermediate text until commit. Choices open labeled selectable lists with search for large sets; repeated cycling is not the only control.

Minimum action target is 44x44 logical units before user scaling. Drawn bounds, hit tests, clipping, focus bounds, and semantic bounds derive from one layout. Offscreen controls cannot intercept pointer actions. Logical keyboard/accessibility navigation retains every row identity: expand ancestors, scroll/materialize the requested row, then focus and activate it through the same validated path. Add semantic geometry, expanded-state, and reveal support to both adapters; virtualization must not become another truncated hierarchy.

## 4. Camera and spatial interaction

Three semantic zoom levels share a stable selected entity:

- **Galaxy:** systems, civilization presence, connections, aggregate traffic, and conflict markers.
- **System:** stars, orbital bands, worlds, settlements, local logistics, and inbound fleets.
- **World focus:** a larger stylized globe, colony cluster, industry markers, stockpiles, and incoming activity. It is not a walkable terrain map.

Use an oblique camera with default pitch 55 degrees, bounded pitch 35-70 degrees and yaw within 45 degrees either side of its scope orientation. Reuse those existing bounds as presentation defaults, not Classic authority settings.

Click selects; double-click focuses; empty-space drag pans; the Camera tool or modified drag orbits; wheel/pinch zooms toward the pointed context. **F** focuses selection and **Home** fits the current scope when scene focus is active. Visible buttons provide every equivalent. Breadcrumbs return to System or Galaxy.

Fit on first load, explicit request, or recovery from an invalid target. Do not auto-fit every frame or move the camera when a civilization expands. Remember framing by scope within the session. Removing the focused object returns to its surviving parent or galaxy scope without losing the document.

Spatial selection and creation always have Navigator/Inspector alternatives. Canvas creation uses a ghost preview, snapping to validated coordinates, and explicit Apply/Cancel; dragging does not commit by itself. Multi-object dependent changes preview as one atomic creator batch.

## 5. Visual identity and rendering

The art direction is a miniature cosmos with tactile silhouettes and a restrained palette: midnight blue space, warm-white labels, muted blue-gray secondary information, cyan selection, amber attention, and reserved red for severe events or destructive actions. Civilization colors do not double as generic selection colors.

### Visual vocabulary

| Object | Treatment and gameplay meaning |
| --- | --- |
| Star | Distinct temperature-family hue and silhouette, soft halo, readable core without bloom |
| Rocky world | Faceted land and crater accents; actual resource/industry overlays kept separate |
| Ocean world | Broad ocean bands and islands; cosmetic archetype unless a rules field explicitly says otherwise |
| Ice world | Pale fractured caps with high-contrast outlines |
| Gas world | Layered atmospheric bands and optional rings; settlement represented by orbital platforms |
| Colony | Small cluster plus civilization crest; cluster size maps to completed buildings, not invented population |
| Industry | Recognizable small marker and explicit blocked/running state |
| Lane | Thin structural connection with stable endpoints |
| Freight | Directional dashed route, resource glyph, selected shipment progress and ETA |
| Fleet | Scout/ark/escort silhouettes, hull-count badge, crest, selected order and destination |
| Conflict | Local impact markers, contested outline, and inspectable combat summary |
| Hazard | Pattern, icon, affected lane, start/end time, and actual effect |

A world's appearance is derived from a stable presentation seed and archetype, not mutable ownership. Ownership changes crest/trim/pattern, not the underlying planet. Existing accepted archetypes remain valid. New archetypes are bundled, validated V2 catalog content and do not introduce unapproved habitability or population rules.

Layer order is explicit: background, galaxy geometry, logistics/fleets, world labels, panels, text, focus/modals. Only intentionally modal scrims cover the scene. The first repair must remove the full-window Workshop overlay occlusion rather than compensating with brighter planets.

At overview scale, aggregate decorative traffic and declutter labels in order: selected, focused, important event, primary entity, secondary entity. Aggregation does not delete authoritative selection or inspector access. Visual orbit interpolation cannot alter orbital state, arrivals, or AI perception.

Use a variable-capacity diorama rendering path separate from Classic's exactly-seven-world path. Reuse shader validation, device lifecycle, batching, and optional-quality patterns. Preserve the existing 36-byte compatibility vertex ABI; new mesh/instance ABIs are independent and explicitly asserted.

High quality may add soft bloom, atmospheric rims, fine shading, and limited ambient particles. Low removes those in that order, never removing labels, selected outlines, travel direction, ownership patterns, or hazard information. All essential rendering works without compute shaders on WebGL2.

## 6. Chronicle, history, and experimentation

Chronicle entries use actual event receipts with actor, entity links, tick, and provenance. Examples: **Vale delivered 10 ore to Crucible**, **Nacre settled an unowned world**, **Creator changed a relationship**, or **Nacre ark returned: unescorted noncombatant**. Only show a reason if it was recorded; never fabricate motives from names or animation.

Aggregate repetitive production/freight into 100-tick summaries. Keep settlement, treaties, war, occupation, dormant civilizations, and creator overrides individually accessible. Default display is the latest 500 summaries; earlier ranges are paged or reconstructed through bounded replay. Rendering does not retain an unlimited in-memory log.

Follow an entity, filter a civilization, or pause on selected event categories. Auto-pause stops at the completed tick boundary that produced the event; it never interrupts a half-committed tick. At high simulation speed, both sounds and notifications are coalesced.

History shows readable operations and times rather than only opaque hashes. Each creator revision stores its acceptance boundary, parent, and command; state at a requested history view is materialized by replaying that command set through the requested tick. Browsing the past pauses and shows **Viewing history**, **Return to present**, and **Branch here**. Undo at tick T removes the latest creator revision from the view and materializes the counterfactual parent state at the same T; it does not pretend to restore the raw pre-command moment. Redo at T restores that revision and must recover its former digest at T. Stepping back in time is separate explicit history navigation. Branch comparison labels the chosen common tick and changed creator inputs.

Before an experiment, record a bookmark with branch, revision cursor, tick, and digest. After running and pausing, record the same four fields. The counterfactual digest after Undo at the later tick is its own oracle; do not confuse it with the earlier bookmark. Fixtures carry all three expected states so acceptance cannot pass against an unspecified digest.

## 7. Saving, transfer, and recovery

Use distinct labels: Unsaved changes, Saving, Saved locally, and Save failed. **Saved** requires successful durable commit, not merely queued work. Keep the save generation and detailed error code in expandable diagnostics.

The Library supports new, open, rename, archive, export, and deliberate capacity reclamation. Archive is reversible removal from the ordinary list, not deletion, and archived items still count toward slot/pack caps. An archived, nonactive galaxy exposes **Delete permanently** only after showing its name/ID, dependent pack, Continue status, irreversibility, and an export recommendation; confirmation must type its displayed name. A pack can be permanently deleted only when archived and unreferenced by every retained galaxy, including archived galaxies. Pack deletion lists blockers rather than cascading.

If the deletion target is global Continue, first compare-and-swap the Library coordinator to unset. Failure changes nothing. Then delete the authority-local slot. A later delete failure leaves the still-recoverable slot in Library with Continue unset and offers explicit reselection; it never deletes the active in-memory session or chooses a different slot. Successful permanent deletion emits a durable receipt and frees exactly one capacity entry. Continue otherwise returns to the last explicitly selected valid authority/slot/branch from the version-independent Library coordinator; it never resolves competing V1/V2 selections by recency or enumeration order. A missing or invalid target opens Library recovery with the recorded authority and error instead of silently choosing another galaxy. Its **Content packs** view imports, inspects, exports, archives, and permanently deletes eligible standalone validated packs.

Galaxy import is choose archive, bounded envelope/integrity preview, then resolve its exact catalog hash. A missing catalog pauses at **Choose matching pack**; validate and register that standalone pack before replay. A mismatch rejects and never substitutes the built-in pack. Then replay/preview counts and **Import as new galaxy**. Every import creates a new slot; overwrite-import is outside this release. Complete durable new-slot commit and Continue-selection persistence before replacing the active session. Validation, storage, or selection failure preserves the prior session and draft; a committed but unselected new slot remains recoverable in Library.

**Export Galaxy** prepares two separately named canonical files: catalog pack and archive. The panel reports Prepared, Handed to platform, and Confirmed written independently for each. Importing the pair through visible controls must restore exact catalog identity and history. The archive references, but does not embed, its catalog. Do not claim browser download completion from a click alone.

Help, file dialogs, and destructive confirmation hold time without later catch-up. Ordinary inspector panels do not pause unless requested. Holds use session-epoch-scoped tokens. Closing/cancelling a dialog restores prior focus and speed only if the same session remains active, every nested hold has ended, and no intervening explicit pause changed the user's intent. Opened/imported sessions always start paused and never inherit the previous session's speed. A native quit waits for required durability or shows a recoverable failure. Browser users receive honest local-storage and download limitations.

## 8. Accessibility and audio

Stable semantic action IDs route pointer, keyboard, native accessibility, and browser semantics through the same commands. Region navigation, normal tree arrow keys, visible focus, modal focus trapping/restoration, and a navigable event list are required. Shortcuts are suppressed during text editing and only operate the focused scope.

Reduced motion removes camera easing, decorative orbital movement, parallax, pulses, shake, and nonessential particles. Static positions and text still report every authoritative event. High contrast combines readable colors with crests, shapes, line styles, and patterns.

Audio is supporting presentation: restrained ambient layers, soft selection/construction cues, and non-startling hazard/conflict signals. Provide master, ambience, music, and effects controls, with mute and persisted levels. Missing audio hardware must not block loading or simulation. Browser audio starts only after a user gesture; replay/import does not emit a backlog of sounds.

Cap event effects to four starts per wall-clock second, coalesce repeated category events, and stop stale loops on scene/session changes. This uses wall time only in the audio presentation layer. No essential information is audio-only. Bundle licensed/provenance-recorded assets or original procedural cues; require no remote fetch or voice generation.

## 9. Acceptance

The [qualification document](2026-09-04-nyon-living-galaxy-qualification.md) defines exact evidence. At minimum: readable 723x802 at 115% and 130%, all three zoom levels, last hierarchy row, more than four branches, complete inspector, normal editing behavior, keyboard-only creation/save/history/import, native and browser semantics, and no scene occlusion.

These are proposed release requirements, not claims about the currently running app.
