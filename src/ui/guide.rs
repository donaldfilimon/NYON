//! Read-only, offline player guide. The same pages drive pixels and semantics.

use glam::Vec2;

use crate::{app::client_runtime::ClientScreen, engine::primitives::PrimitiveBatch};

use super::{
    accessibility::{SemanticActionId, SemanticNode, SemanticNodeId, SemanticRole, SemanticTree},
    platform::{
        PlatformControl, PlatformIconAction, PlatformRect, PlatformUiAction, PlatformUiFrame,
        apply_platform_control_geometry, icon_for_action,
    },
    workshop_layout::WorkshopLayout,
};

const TOPICS: &[(&str, &[&str])] = &[
    (
        "Choose your game",
        &[
            "Classic Sector is a strategy battle. You command the Union: capture five of seven worlds before a rival does. Galaxy Workshop is a separate creative sandbox: build a galaxy, run production, inspect it and revise. It has no enemy or victory screen.",
            "Living Galaxy lets you observe autonomous societies and inspect their inventories and recorded events. First time? Use Next for a first fleet in Classic, continue to Workshop for your first 4 energy, or read Living Galaxy for a paused simulation experiment. No account, server or agent commands are needed.",
            "Open this guide with F1, the main-menu Player guide button, Workshop's Guide button, or Classic's HELP / H. Simulation time is held while reading. Close or Escape returns to the same session and speed; no save or scenario is replaced.",
        ],
    ),
    (
        "Classic // First fleet",
        &[
            "GOAL: You are Union. Control five of seven worlds to win. Losing every Union world and fleet ends your campaign. Energy funds orders and fleets; defense is the force protecting a world.",
            "FIRST SUCCESS: If the old six-step tutorial is active, click SKIP to unlock the speed controls. Press P to pause if needed. Click a Union world, then hover a different world to preview the launch in the command tray. Do not left-click the destination: that changes your selection.",
            "Read source, destination, strength and any rejection in the tray. A launch sends half the source's current energy as fleet strength and consumes that energy. Source defense stays unchanged. The fleet snapshots the source Hydrosphere for its travel speed.",
            "Press Space with keyboard focus off the HUD, or right-click the destination, to issue the order. Press P to run. Your first result is a moving fleet and lower source energy, not an instant capture. Friendly arrivals reinforce defense; hostile arrivals fight defense.",
            "If an order is rejected, select your own world with enough energy and a different destination. Space activates a focused HUD button instead of launching. Clicking your source clears HUD focus. Use the tray's rejection text as the reason to fix.",
        ],
    ),
    (
        "Workshop // First energy",
        &[
            "GOAL: Make a working galaxy, not a battle fleet. Start with New Workshop for a blank galaxy, or Continue for your save. These instructions also work with new objects in an existing galaxy. Do not replace your save just to learn.",
            "Keep time paused. Click Create system and Apply (defaults are fine). Select that system in the left hierarchy. Create star and Apply. Select that star, then Create world and Apply. Factions and ownership are optional for this first production result.",
            "Select your new world in the hierarchy. Click Place industry. Check World is your new world and Industry is solar-array. Apply. New industries start enabled: do NOT toggle them off. Click Step once while paused.",
            "FIRST SUCCESS: The tick increases by one. Select the world again: the visible Inventory line now shows 4 energy. A solar array makes 4 energy per tick. Save Workshop and wait for Saved and selected for Continue. If the bottom controls are missing, widen the window.",
            "Forms are drafts until Apply. Tab / Shift+Tab moves focus; Enter or Space activates. For a choice, activate repeatedly to cycle. Focus a text or number field and type to replace its value; Backspace edits. Escape cancels the form without applying it.",
        ],
    ),
    (
        "Workshop // Ore to alloy",
        &[
            "On the same world, Create deposit: choose ore and a positive reserve. Place industry again: choose extractor, that world and that linked deposit. Keep your solar-array enabled. Step once: extraction uses 1 energy to make 2 ore, reducing the reserve. Deposits alone do not mine anything.",
            "Place a foundry on that world with no linked deposit. Each operating tick consumes 2 energy and 3 ore to make 1 alloy. With one solar array and extractor, ore needs time to accumulate; not every tick can run the foundry. Select the world and watch Inventory for alloy.",
            "For shipping, build a second system, star and world. Connect lane joins the SYSTEMS. Connect route joins the WORLDS: choose source, destination, ore, and batch units 1. A lane by itself ships nothing. Run 1x until ore reaches the destination; travel is not instantaneous.",
            "Put a solar-array and foundry on the receiving world to turn delivered ore into alloy there. If nothing happens: check speed, industry Enabled, matching deposit, reserves, local energy/ore, lane, source/destination and route batch size. An ion storm reduces lane capacity; leave hazards out of your first factory.",
        ],
    ),
    (
        "Living Galaxy // First simulation",
        &[
            "1 / SELECTION: Open Living Galaxy to start paused in Vale Confluence. Select a world in the diorama, or open Navigator to search names, full identities or entity types. Expand a system and inspect a world, civilization, facility, fleet, route or hazard. Selection changes presentation only; Inspector reads current authority fields and recorded recipe causes.",
            "2 / TIME: Step or the period key advances one boundary while paused. Check the tick and selected inventory before and after. Run/Pause or P toggles time outside text fields. Start at 1x: ten ticks per second. 4x and 20x request faster transport; the visible effective speed can be lower under load. Pause, modal activity and suspension discard timing debt.",
            "3 / CREATION: Pause, choose Creator tools, select System and give it a name. Fields are drafts. Preview validates a private candidate and explains its changes; Apply commits only that current preview. Cancel preserves the galaxy. Removals require the displayed dependent objects and derived references to be reviewed. Editing retained history requires Branch here.",
            "FIRST SUCCESS: Find a committed inventory change or event you can explain. Chronicle shows newest verified rows first; repetitive production and freight are summarized in 100-tick windows. Open a row for exact receipt references. Decision rows show recorded actor, score, outcome and reasons. Ordinary events without actor/entity payloads explicitly retain unknown links.",
            "CAMERA: Galaxy, System and World buttons change scope. Empty-space drag pans; modified drag or the Camera tool orbits. Wheel zooms toward the pointer. F focuses selection and Home fits the current scope when scene focus is active. Camera offers visible equivalents. Removing a focused object returns to its surviving parent. Navigator and Inspector remain available when scene objects overlap.",
            "EXPERIMENT: In History, name a bookmark Before the intervention. Preview a temporary lane hazard, Apply it, then observe receipts rather than predict an unrecorded result. Branch preserves the sibling future. Matching-tick comparison evaluates both histories at one exact tick. Record an experiment only from the completed measured comparison. Notebook labels and observations are separate session metadata.",
            "LEARNING GALAXIES: New galaxy offers paused Vale Confluence, Commerce, Frontier Friction, Interrupted Corridor, First Expansion, Blank and reproducible Seeded starts. Main Vale has no guaranteed first war. Commerce, Frontier and Interrupted are deterministic fixture situations; their rules are unchanged while running. Save your current session before replacing it.",
            "PRESERVE: Save and wait for its durable status. Save & leave completes the archive and notebook save before leaving. Library opens exact saved identities and supports rename, archive, restore and deleting archived saves. Continue resolves its recorded selection; invalid data enters recovery. Transfer first prepares an immutable export snapshot, then exports Galaxy, Catalog or Notebook separately. Browser download confirmation is separate from download handoff. Import displays a validated candidate for explicit Apply or Cancel.",
            "ACCESS: Tab / Shift+Tab focuses controls; Enter or Space activates the focused action. Text fields support cursor movement, selection and editing. PageUp / PageDown reads inspector pages. Open F1 to read with time held, then Close to return to the same speed. A fault stops transport at the last committed authority boundary; read its message. Audio settings expose mute and separate volume channels.",
        ],
    ),
    (
        "Controls // Mouse and keyboard",
        &[
            "MENUS / WORKSHOP: Click a visible control, or Tab / Shift+Tab (also arrow keys) to move focus, then Enter / Space. Select objects in the left hierarchy; the Workshop map is a display, not a direct placement surface. P toggles pause / 1x outside forms. Cmd/Ctrl+S saves. Use Pause, Step once, 1x, 4x, 20x on the timeline.",
            "CLASSIC: Left-click selects. Hover previews a destination; right-click issues launch. Space launches unless a HUD button has focus. Drag empty space or middle-drag to orbit; wheel / pinch zooms; C resets camera. P pauses; [ / ] changes speed. 1 / 2 / 3 selects atmosphere / hydrosphere / topology; Q / E lowers / raises it at an energy cost on your own world.",
            "CLASSIC: S opens Settings, F4 opens the detached Scenario editor, R restarts and loses current battle progress, Escape clears selection. The old observed-action tutorial can be reset in Settings. The guide's Main menu button leaves Classic; it does not save a battle.",
            "CONTROLLERS: No native gamepad bindings are implemented in this build. A controller requires an external mouse/keyboard mapping; there is no supported A/B or stick layout to promise. Classic world selection still needs a pointer. Workshop and this guide expose keyboard controls.",
        ],
    ),
    (
        "Save, exit and graphics",
        &[
            "WORKSHOP: Save Workshop (Cmd/Ctrl+S) persists the current galaxy. Wait for Saved and selected for Continue, with no DIRTY marker, before quitting. Main menu / Escape requests a save; New Workshop or Classic can remain disabled while a save is pending. Continue reopens the selected Workshop save, not a Classic battle. Library, on the main menu and in the Workshop, opens, renames and archives saves, and picks the Continue save.",
            "Undo moves to an earlier recorded creator revision. Editing history creates a branch; it does not erase later work. Select a branch to return to it. Save after choosing the branch you want to continue. Library's Export, Export this galaxy and Export content pack prepare portable copies, but this build cannot write a copy to a file or import one, so Save copy and both import controls stay disabled.",
            "CLASSIC: Scenario SAVE stores the editor's setup, not the current battle. LOAD changes the detached draft; APPLY AND RESTART replaces the battle. CANCEL leaves the running scenario intact. R also restarts the battle. To exit the app, use the window close control or Main menu > Quit (native).",
            "GRAPHICS: The retro map is intentional. GPU/Metal/WebGPU labels describe the rendering backend; they do not mean ray tracing. No ray-tracing option is implemented. Classic's neural advisory is only a scoring hint and cannot play for you. Settings offers contrast and motion controls; Classic also has UI scale and graphics-quality choices.",
        ],
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuideLocation {
    topic: usize,
    sheet: usize,
}

impl GuideLocation {
    pub const fn for_screen(screen: ClientScreen) -> Self {
        Self {
            topic: match screen {
                ClientScreen::ClassicSector => 1,
                ClientScreen::GalaxyWorkshop => 2,
                ClientScreen::GalaxySandbox => 4,
                _ => 0,
            },
            sheet: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuideAction {
    Open,
    Show(GuideLocation),
    Close,
    MainMenu,
}

pub struct GuideFrame {
    pub platform: PlatformUiFrame,
    pub lines: Vec<String>,
    pub columns: usize,
}

impl GuideFrame {
    pub fn draw_body(&self, batch: &mut PrimitiveBatch) {
        for (index, line) in self.lines.iter().enumerate() {
            batch.text(
                Vec2::new(18.0, 134.0 + index as f32 * 22.0),
                1.6,
                [0.86, 0.95, 1.0, 1.0],
                &line.to_ascii_uppercase(),
            );
        }
    }
}

pub fn build_guide_frame(
    location: GuideLocation,
    viewport: Vec2,
    high_contrast: bool,
    focused: Option<&SemanticActionId>,
) -> GuideFrame {
    let viewport = if viewport.is_finite() {
        viewport.max(Vec2::new(320.0, 320.0))
    } else {
        Vec2::new(640.0, 480.0)
    };
    let columns = ((viewport.x - 36.0) / (6.0 * 1.6)).floor() as usize;
    let rows = ((viewport.y - 222.0) / 22.0).floor().max(1.0) as usize;
    let topic = location.topic.min(TOPICS.len() - 1);
    let all_lines = wrap(TOPICS[topic].1, columns);
    let sheets = all_lines.len().div_ceil(rows).max(1);
    let sheet = location.sheet.min(sheets - 1);
    let lines = all_lines
        .into_iter()
        .skip(sheet * rows)
        .take(rows)
        .collect::<Vec<_>>();
    let previous = if sheet > 0 {
        Some(GuideLocation {
            topic,
            sheet: sheet - 1,
        })
    } else if topic > 0 {
        Some(GuideLocation {
            topic: topic - 1,
            sheet: wrap(TOPICS[topic - 1].1, columns)
                .len()
                .div_ceil(rows)
                .saturating_sub(1),
        })
    } else {
        None
    };
    let next = if sheet + 1 < sheets {
        Some(GuideLocation {
            topic,
            sheet: sheet + 1,
        })
    } else if topic + 1 < TOPICS.len() {
        Some(GuideLocation {
            topic: topic + 1,
            sheet: 0,
        })
    } else {
        None
    };
    let mut nodes = vec![SemanticNode::text(
        "guide.body",
        SemanticRole::Text,
        "Instructions",
        lines.join(" "),
    )];
    let mut controls = Vec::new();
    let entries = [
        (
            "guide.previous",
            "Previous",
            previous.map(GuideAction::Show),
        ),
        ("guide.next", "Next", next.map(GuideAction::Show)),
        ("guide.menu", "Main menu", Some(GuideAction::MainMenu)),
        ("guide.close", "Close", Some(GuideAction::Close)),
    ];
    let width = (viewport.x - 40.0) / 4.0;
    for (index, (id, label, action)) in entries.into_iter().enumerate() {
        let Some(action) = action else { continue };
        let action_id = SemanticActionId::new(id);
        nodes.push(SemanticNode::control(
            format!("{id}.node"),
            SemanticRole::Button,
            label,
            label,
            true,
            false,
            action_id.clone(),
        ));
        controls.push(PlatformControl {
            semantic_id: SemanticNodeId::new(format!("{id}.node")),
            action_id: action_id.clone(),
            action: PlatformUiAction::Guide(action),
            bounds: PlatformRect::from_xywh(
                8.0 + index as f32 * (width + 8.0),
                viewport.y - 62.0,
                width,
                48.0,
            ),
            label: label.to_owned(),
            description: label.to_owned(),
            enabled: true,
            selected: false,
            focused: focused == Some(&action_id),
            icon: icon_for_action(PlatformIconAction::Guide(action)),
        });
    }
    let mut semantics = SemanticTree {
        root: SemanticNode::container(
            "guide.application",
            SemanticRole::Application,
            "NYON player guide",
            vec![SemanticNode::container(
                "guide.dialog",
                SemanticRole::Dialog,
                TOPICS[topic].0,
                nodes,
            )],
        ),
        announcements: Vec::new(),
    };
    apply_platform_control_geometry(&mut semantics, &mut controls);
    let logical_focus_order = controls
        .iter()
        .map(|control| control.action_id.clone())
        .collect();
    let mut platform = PlatformUiFrame {
        viewport,
        layout: WorkshopLayout::resolve(viewport, 1.0).expect("safe guide viewport must resolve"),
        background: super::platform::PlatformBackground::Full,
        drawer: None,
        modal: None,
        controls,
        logical_focus_order,
        semantics,
        title: "NYON // Player guide".to_owned(),
        status_lines: vec![
            format!("{} ({}/{})", TOPICS[topic].0, sheet + 1, sheets),
            "Time held // Tab + Enter // Escape closes".to_owned(),
        ],
        sighted_text: Vec::new(),
        visible_nodes: Vec::new(),
        source_records: Vec::new(),
        operational_status: None,
        high_contrast,
    };
    platform.rebuild_visible_nodes();
    GuideFrame {
        platform,
        lines,
        columns,
    }
}

fn wrap(paragraphs: &[&str], columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in paragraphs {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + word.len() > columns {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        lines.push(line);
        lines.push(String::new());
    }
    lines
}
