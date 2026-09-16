# NYON player manual

NYON has two separate modes. Neither needs an account, an MCP server, or an AI agent.

| Mode | What you are trying to do | Your first success |
| --- | --- | --- |
| Classic Sector | Command the Union and control **five of seven worlds** before a rival does. | Issue an order and watch your first fleet travel. |
| Galaxy Workshop | Create a galaxy, run an economy, inspect results, revise and save. There is **no enemy or victory screen**. | Build a solar array and make **4 energy**. |

## Help and getting into the game

Open **Player guide (F1)** on the main menu or in Workshop. In Classic, click **HELP**, press **H**, or press **F1**. On Macs configured to use media keys, F1 may require Fn/Globe. The on-screen button does not.

The guide holds simulation time while you read. Click Previous/Next to turn pages, or use Tab / Shift+Tab and Enter. Escape, F1, or Close returns to the same session, draft and speed without replacing or saving anything. The guide's Main menu button is a separate, deliberate exit from the current screen. No time catches up after closing the guide.

Classic's small **TUTORIAL … PAUSED** overlay is an optional six-step introduction in the real match, not a separate training game. During its first step, **START HERE** points to one of your Union worlds. Click that world or the START HERE button; the button is also reachable with Tab and Enter. Ownership is not communicated by color alone.

**Just want to play? Click SKIP at the top right.** This removes the tutorial and resumes **the same match**, normally at 1x (or at your previous running speed if you restarted guidance). It does not regenerate the galaxy. Use **P** or **PLAY/PAUSE** afterward. Settings > **RESET GUIDANCE** restarts the observed-action tutorial and pauses the same match.

## Classic Sector: launch your first fleet

1. Choose **Classic Sector** on the main menu. Click **SKIP** if the tutorial is active. Press **P** to pause while planning if the match is running.
2. **Left-click a Union world**. The default Union worlds are cyan, but check the ownership/selection information, not just color. You cannot order an enemy or neutral world.
3. **Hover a different world** to preview the order. Read SOURCE, TARGET, STRENGTH and the readiness/rejection in the command tray. Do not left-click the destination: with a mouse, that changes your selection.
4. **Right-click the destination** to issue the order, or press **Space** while hovering it with keyboard focus off the HUD. The on-screen LAUNCH control issues a valid retained preview; a mouse destination is cleared when the pointer leaves the scene, so right-click/Space is the reliable mouse path. With touch, tap your source, tap a destination, then tap LAUNCH.
5. If paused, press **P** to run. A fleet travels between worlds and your source's energy drops. This visible fleet is the first success. Arrival is not instant; friendly arrivals reinforce defense, while hostile arrivals fight the destination's defense.

You win by controlling five worlds. Losing all Union worlds and fleets ends your campaign. A launch sends half the source world's current energy as fleet strength and consumes that energy, leaving its defense unchanged. The fleet snapshots the source's Hydrosphere level when launched; that level affects its travel speed. Defense protects a world, and friendly arrivals reinforce its defense. Do not assume sending a fleet guarantees a capture. Watch enemy movement and reinforce threatened worlds.

### Classic controls

| Input | Action |
| --- | --- |
| Left-click | Select a world. Clicking your source also clears HUD keyboard focus. |
| Hover a different world | Preview a launch from your selected world. |
| Right-click destination | Issue a launch if the simulation accepts it. |
| Space | Launch the current preview, **unless a HUD button has focus**, in which case it activates that button. |
| Tab / Shift+Tab; Enter | Focus and activate HUD controls. The ordinary world-selection path still needs a pointer; the first tutorial's START HERE also has a keyboard button. |
| P; `[` / `]` | Pause/resume; lower/raise speed through 0x, 1x, 2x, 4x. Tutorial must be completed or skipped first. |
| 1 / 2 / 3 | Select atmosphere / hydrosphere / topology. |
| Q / E | Lower/raise the selected field on your own world, when the energy/rules allow it. |
| Drag empty space; middle-drag | Orbit the camera. Dragging does not launch. |
| Wheel / pinch; C | Zoom; reset camera. |
| S | Settings, including UI scale, reduced motion, high contrast, graphics quality and RESET GUIDANCE. |
| HELP / H / F1 | Player guide. |
| F4 / SCENARIO | Detached scenario editor. |
| R | Restart the current scenario. **Current battle progress is lost.** |
| Escape / CLEAR | Clear selection during play. Escape does not quit the app. |

**An order does nothing?** Read its rejection in the tray. Check that the source is yours, the destination differs, and the source has enough energy. If time is stopped, skip/complete the tutorial, then resume. If Space activates an unrelated button, click the source world again to release HUD focus.

The neural advisory is a scoring hint, not an autopilot. It does not issue commands or determine outcomes.

## Galaxy Workshop: make your first 4 energy

You can follow these steps with new objects in your existing Workshop. A fresh **New Workshop** is empty; **Continue** opens the selected Workshop save. Do not replace a valued save just to try the guide.

Keep time **paused** while editing. Use the left-hand hierarchy to select objects. The map displays the galaxy; it is not a click-to-place editor. In each creator form, verify the parent/world choice before applying: defaults can pick an existing object when none is selected.

1. **Create system > Apply.** Default names and coordinates are fine for this first exercise.
2. Select that system in the hierarchy. **Create star > Apply.** Its System field must refer to your new system.
3. Select the star. **Create world > Apply.** Check System and Primary star. This world begins with an empty inventory.
4. Select the world. **Place industry**: choose that World and **solar-array**, with no linked deposit. **Apply**.
5. New industries start **enabled**. Do not use Set industry enabled just because you placed one: the default toggle would turn an enabled industry off.
6. Click **Step once** while paused. The tick increases by one. Select the world again and read the visible **Inventory: 4 energy** status line. That is your first production success.
7. Click **Save Workshop**. Wait for **Saved and selected for Continue**, a generation number, and no DIRTY marker.

A faction, ownership, ore deposit, lane and route are **not required** to make solar energy. They are additional tools, not missing prerequisites for the first success.

### From energy to ore to alloy

Use the built-in NYON Core pack. Its recipes are:

| Industry | Needs | Produces per operating tick |
| --- | --- | --- |
| solar-array | No resource input | 4 energy |
| extractor | 1 energy on its world; linked ore deposit with enough reserves | 2 ore; consumes 2 reserve units |
| foundry | 2 energy and 3 ore on its world | 1 alloy |

1. Select your solar-powered world and **Create deposit** with resource **ore** and a positive reserve (default 1000 is fine). A deposit is a finite reserve, not an inventory of mined ore.
2. **Place industry** with **extractor**, your world and its matching **linked deposit**. Step once. Energy is spent, ore enters the world's inventory, and deposit reserves fall.
3. **Place industry** with **foundry**, the same world, and no linked deposit. Keep solar and extractor enabled. Advance a few ticks. Select the world and look for **alloy** in Inventory.

One extractor supplies 2 ore per tick, but the foundry needs 3. The foundry therefore cannot necessarily operate every tick. Industry waits when inputs are insufficient; simply placing a foundry does not conjure ore or power.

### Ship between systems

1. Build another system, star and world. Keep names distinct so parent and endpoint choices are recognizable.
2. **Connect lane** between the two **systems**.
3. **Connect route** from the mining **world** to the receiving **world**; choose **ore** and start with **batch units 1**. Ensure the source retains some ore to ship (a source foundry also consumes ore).
4. Run **1x**. The route dispatches shipments and they take time to arrive. A lane alone transports nothing. Select the receiving world to watch its inventory grow.
5. Place a **solar-array** and **foundry** on the receiving world if you want to turn delivered ore into alloy there. Energy is local to each world unless you explicitly ship it too.

For the first factory, omit hazards. An ion storm reduces lane capacity and can stop a small-capacity route from moving useful quantities.

### Workshop controls and editing

| Input | Action |
| --- | --- |
| Click | Activate a tool/control or select an object in the hierarchy. |
| Tab / Shift+Tab or arrow keys | Move keyboard focus through the available controls. |
| Enter / Space | Activate the focused control. In a choice field, cycle to the next value. |
| Type in a focused text/number field | Replace its value; subsequent typing appends. Backspace edits. There is no general text-selection/caret editor here. |
| Apply / Cancel | Submit the valid draft / leave without applying it. Invalid drafts cannot be applied. |
| Escape | Cancel a creator/removal dialog; otherwise return to Main menu and request a save. In the guide it closes only the guide. |
| P outside forms | Toggle paused / 1x. |
| Pause, Step once, 1x, 4x, 20x | Timeline controls. 1x is 10 simulation ticks per second. |
| Cmd+S on Mac; Ctrl+S elsewhere | Save Workshop. Prefer the visible Save control while editing a form. |
| F1 / Player guide | Read instructions without simulation time advancing. |

Use a wide window for the current Workshop layout. At narrow widths some timeline controls are not shown; widening the window restores them. The visual hierarchy currently shows a bounded number of entries, so a large galaxy can expose more objects through accessibility than fit in the visual panel. The guide itself wraps and paginates at compact sizes. These are current UI limitations, not extra controls you have missed.

The inspector model includes inventories, deposits, industries, routes, shipments and hazards. The selected world's inventory now has a normal visible status line; the full detailed inspector is currently exposed through accessibility, not a complete visual table.

**Nothing is producing or arriving?** Check: time is running or you used Step once; industry is Enabled; the extractor has a matching nonempty deposit; the world has energy and ore; lane connects the correct systems; route points in the intended direction; source has enough for the batch; and travel time has passed. Select the **world**, not the industry, to read Inventory.

## Saving, undo and leaving safely

**Workshop:** Save Workshop writes your current Workshop state. Wait for **Saved and selected for Continue** and no DIRTY indicator. Autosave/exit handling exists, but this visible confirmation is the clearest checkpoint. Main menu requests a durable transition; replacement routes can stay disabled until persistence finishes. **Continue is for Workshop saves**, not battle saves. If saving reports a failure, keep the app open and retain the working session; do not assume an earlier generation contains recent changes.

**Library:** open it from the main menu, or from the Workshop: the **Library** button in the top bar, or on a narrow window the **Library** button at the top of the Navigator drawer. Opening it leaves the Workshop exactly as it was, unsaved changes included, and **Close** or Escape returns to it. Select a save to **Rename**, **Archive** or **Unarchive** it; each asks first, and Archive says whether it will clear Continue. Names use 1 to 64 printable characters with single spaces between words. The resident Workshop's own save cannot be archived. **Open** loads the selected save into the Workshop and makes it the Continue save; it is unavailable while the open Workshop has unsaved changes, and for the save that is already open. If the save changed since the list was read, the Library says so and **Refresh** lists it again. If its latest generation is invalid, the Library offers **Open previous**, which opens the last valid one and replaces the invalid one when it saves. **Use for Continue** makes the selected save the one Continue opens, without opening it now; it is unavailable while a Workshop is open, because the open Workshop decides Continue until you leave it. Exporting a save, and importing or exporting files, are shown but not available yet.

**Undo and branches:** Undo moves to an earlier creator revision while paused. Editing a historical revision creates a branch rather than deleting later work. Branch buttons select a history branch. Save after choosing the state/branch you want Continue to reopen. Do not treat Undo as “erase everything since the last frame.” There are no visible archive import/export controls in this build.

**Classic:** Scenario SAVE stores the editor's setup, **not a resumable battle**. LOAD changes the detached draft; APPLY AND RESTART starts that setup afresh. CANCEL leaves the active scenario intact. R restarts the current scenario and loses battle progress. Settings/preferences are saved separately.

To leave Classic for the product menu, open HELP and choose **Main menu**. To quit the native app, use **Main menu > Quit** or close its window. Save and wait first in Workshop. Browser players should save before closing the tab; browser storage is scoped to the current origin/profile and is not a portable backup.

## Controllers, accessibility and graphics

**There are no native gamepad bindings in this build.** No Xbox/PlayStation button layout or stick mapping is implemented. An external tool can map a controller to mouse/keyboard input, but such mappings have not been qualified here. Use mouse/keyboard for the documented paths. Classic's ordinary world targeting remains pointer-based.

Workshop and the new guide provide a shared keyboard/pointer action model and native/browser semantic information. Actual screen-reader behavior is a separate platform acceptance check; the presence of those semantics does not establish complete accessibility in every screen. The first Classic marker uses text, a line and a ring rather than relying only on cyan color, and its button supports keyboard selection.

The retro presentation is intentional. **Metal, WebGPU and WebGL2 labels identify rendering backends, not ray-tracing modes. There is no implemented ray-tracing option.** Classic's GPU advisory, when available, is separate from rendering and still only a hint. Use contrast/motion settings for readability; Classic also exposes UI scale and graphics-quality choices.

## Validation boundary

This manual describes current source behavior. The first-energy/first-alloy exercise, guide layout, Classic START HERE selection and Skip behavior have automated regression coverage in `tests/player_guide.rs`. Source/build checks do not prove live native or browser usability. Test a newly built app after saving and quitting the older build; do not run two processes against the same Workshop save.
