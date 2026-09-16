//! Shell (main menu / guide) platform frames and fallback screens, split out of `platform.rs`.

use super::{
    PlatformBackground, PlatformControl, PlatformIconAction, PlatformRect, PlatformUiAction,
    PlatformUiFrame, ShellUiAction, apply_platform_control_geometry, icon_for_action,
    install_platform_batches, push_guide_control, safe_viewport,
};
use crate::app::client_runtime::ClientScreen;
use crate::app::client_runtime::MainMenuCapability;
use crate::app::client_runtime::MainMenuRoute;
use crate::engine::backend::BackendKind;
use crate::engine::primitives::PrimitiveBatch;
use crate::preferences::UserPreferencesV1;
use crate::ui::accessibility::SemanticActionId;
use crate::ui::accessibility::SemanticNode;
use crate::ui::accessibility::SemanticNodeId;
use crate::ui::accessibility::SemanticRole;
use crate::ui::accessibility::SemanticTree;
use crate::ui::workshop_layout::WorkshopLayout;
use glam::Vec2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformFallbackCode {
    Capacity,
    Geometry,
    Atlas,
}

impl PlatformFallbackCode {
    pub fn text(self) -> &'static str {
        match self {
            Self::Capacity => "UI TEXT UNAVAILABLE: CAPACITY",
            Self::Geometry => "UI TEXT UNAVAILABLE: GEOMETRY",
            Self::Atlas => "UI TEXT UNAVAILABLE: ATLAS",
        }
    }
}

pub(super) fn fallback_bounds(code: PlatformFallbackCode) -> PlatformRect {
    PlatformRect::from_xywh(8.0, 8.0, code.text().len() as f32 * 6.0, 7.0)
}

pub(super) fn draw_fallback(batch: &mut PrimitiveBatch, code: PlatformFallbackCode) {
    batch.text(Vec2::splat(8.0), 1.0, [1.0, 1.0, 1.0, 1.0], code.text());
}

/// Installs Guide chrome and its fixed separate body. No callback or ordinary
/// platform frame can extend the validated Workshop primitive overlay here.
///
/// ```compile_fail,E0308
/// use nyon::{engine::primitives::PrimitiveBatch, ui::{AtlasMetrics, UiBatch,
///     UiBatchError, platform::{PlatformUiFrame, install_guide_batches}}};
/// let _: fn(&PlatformUiFrame, &AtlasMetrics, &mut UiBatch, &mut PrimitiveBatch)
///     -> Result<(), UiBatchError> = install_guide_batches;
/// ```
///
/// ```compile_fail,E0061
/// use nyon::{engine::primitives::PrimitiveBatch, ui::{AtlasMetrics, UiBatch,
///     guide::GuideFrame, platform::install_guide_batches}};
/// fn arbitrary_body(guide: &GuideFrame, metrics: &AtlasMetrics,
///     ui: &mut UiBatch, overlay: &mut PrimitiveBatch) {
///     install_guide_batches(guide, metrics, ui, overlay, |_: &mut PrimitiveBatch| {});
/// }
/// ```
///
/// ```compile_fail,E0432
/// use nyon::ui::platform::install_platform_batches_with_body;
/// ```
pub fn install_guide_batches(
    guide: &crate::ui::guide::GuideFrame,
    metrics: &crate::ui::AtlasMetrics,
    ui: &mut crate::ui::UiBatch,
    overlay: &mut PrimitiveBatch,
) -> Result<(), crate::ui::UiBatchError> {
    install_platform_batches(&guide.platform, metrics, ui, overlay)?;
    guide.draw_body(overlay);
    Ok(())
}

pub struct ShellPlatformInput<'a> {
    pub screen: ClientScreen,
    pub capabilities: &'a [MainMenuCapability],
    pub credits_visible: bool,
    pub recovery_message: Option<&'a str>,
    pub continue_available: bool,
    pub backend: Option<BackendKind>,
    pub preferences: UserPreferencesV1,
    pub viewport: Vec2,
    pub focused: Option<&'a SemanticActionId>,
}

pub fn build_shell_platform_frame(input: ShellPlatformInput<'_>) -> PlatformUiFrame {
    let viewport = safe_viewport(input.viewport);
    let layout = WorkshopLayout::resolve(viewport, input.preferences.ui_scale.factor())
        .expect("safe shell viewport must resolve");
    let mut controls = Vec::new();
    let mut nodes = Vec::new();
    let title;
    let status_lines;

    match input.screen {
        ClientScreen::MainMenu if input.credits_visible => {
            title = "NYON // Credits".to_owned();
            status_lines = vec![
                "NYON Galaxy Workshop".to_owned(),
                "Developed by Donald Filimon".to_owned(),
                "Offline deterministic creative sandbox".to_owned(),
            ];
            push_shell_control(
                &mut controls,
                &mut nodes,
                "shell.credits.close",
                ShellUiAction::DismissCredits,
                "Close credits",
                "Return to the main menu.",
                centered_button(viewport, 0),
                true,
                false,
                input.focused,
            );
        }
        ClientScreen::MainMenu => {
            title = "NYON // Galaxy Workshop".to_owned();
            status_lines = vec![
                "Shape. Observe. Iterate. Share.".to_owned(),
                "Offline local-first sandbox".to_owned(),
            ];
            for (index, capability) in input.capabilities.iter().enumerate() {
                let (label, description) = menu_copy(capability.route);
                push_shell_control(
                    &mut controls,
                    &mut nodes,
                    &format!("shell.menu.{}", menu_slug(capability.route)),
                    ShellUiAction::Menu(capability.route),
                    label,
                    description,
                    centered_button(viewport, index),
                    capability.enabled,
                    false,
                    input.focused,
                );
            }
            push_guide_control(
                &mut controls,
                &mut nodes,
                PlatformRect::from_xywh(viewport.x - 164.0, 8.0, 156.0, 44.0),
                input.focused,
            );
        }
        ClientScreen::Settings => {
            title = "NYON // Settings".to_owned();
            status_lines = vec![format!(
                "Graphics backend: {}",
                input.backend.map_or("NOT INITIALIZED", BackendKind::label)
            )];
            let scale_label = format!("UI scale: {}%", input.preferences.ui_scale.percent());
            push_shell_control(
                &mut controls,
                &mut nodes,
                "shell.settings.scale",
                ShellUiAction::CycleUiScale,
                &scale_label,
                "Cycle UI scale through 85%, 100%, 115%, and 130%.",
                centered_button(viewport, 0),
                true,
                false,
                input.focused,
            );
            let settings = [
                (
                    "shell.settings.motion",
                    ShellUiAction::ToggleReducedMotion,
                    "Reduced motion",
                    "Reduce nonessential camera and presentation motion.",
                    input.preferences.motion == crate::presentation::MotionPreference::Reduced,
                ),
                (
                    "shell.settings.contrast",
                    ShellUiAction::ToggleHighContrast,
                    "High contrast",
                    "Use the high-contrast presentation palette.",
                    input.preferences.high_contrast,
                ),
                (
                    "shell.settings.close",
                    ShellUiAction::CloseSettings,
                    "Close settings",
                    "Return to the previous product screen.",
                    false,
                ),
            ];
            for (index, (id, action, label, description, selected)) in
                settings.into_iter().enumerate()
            {
                push_shell_control(
                    &mut controls,
                    &mut nodes,
                    id,
                    action,
                    label,
                    description,
                    centered_button(viewport, index + 1),
                    true,
                    selected,
                    input.focused,
                );
            }
        }
        ClientScreen::Loading => {
            title = "NYON // Loading".to_owned();
            status_lines = vec!["Validating the explicitly selected Workshop save".to_owned()];
        }
        ClientScreen::RecoverableError => {
            title = "NYON // Recovery".to_owned();
            status_lines = vec![
                input
                    .recovery_message
                    .unwrap_or("A recoverable Workshop error occurred")
                    .to_owned(),
            ];
            let mut index = 0;
            if input.continue_available {
                push_shell_control(
                    &mut controls,
                    &mut nodes,
                    "shell.recovery.continue",
                    ShellUiAction::ContinueRecovery,
                    "Continue recovered save",
                    "Open the explicitly selected previous valid generation.",
                    centered_button(viewport, index),
                    true,
                    false,
                    input.focused,
                );
                index += 1;
            }
            push_shell_control(
                &mut controls,
                &mut nodes,
                "shell.recovery.dismiss",
                ShellUiAction::DismissRecovery,
                "Dismiss recovery",
                "Return without replacing the current valid session.",
                centered_button(viewport, index),
                true,
                false,
                input.focused,
            );
        }
        // Screens this builder never draws. The Library joined them when route
        // design task 8 gave it its own frame; its old placeholder here would
        // otherwise build a second Close control beside the real one.
        ClientScreen::ClassicSector | ClientScreen::GalaxyWorkshop | ClientScreen::Library => {
            title = "NYON".to_owned();
            status_lines = Vec::new();
        }
    }

    let mut semantics = SemanticTree {
        root: SemanticNode::container(
            "shell.application",
            SemanticRole::Application,
            &title,
            vec![SemanticNode::container(
                "shell.controls",
                SemanticRole::Group,
                "Product controls",
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
    let mut frame = PlatformUiFrame {
        viewport,
        layout,
        background: PlatformBackground::Full,
        drawer: None,
        modal: None,
        controls,
        logical_focus_order,
        semantics,
        title,
        status_lines,
        sighted_text: Vec::new(),
        visible_nodes: Vec::new(),
        source_records: Vec::new(),
        operational_status: None,
        high_contrast: input.preferences.high_contrast,
    };
    frame.rebuild_visible_nodes();
    frame
}

#[allow(clippy::too_many_arguments)]
pub(super) fn push_shell_control(
    controls: &mut Vec<PlatformControl>,
    nodes: &mut Vec<SemanticNode>,
    id: &str,
    action: ShellUiAction,
    label: &str,
    description: &str,
    bounds: PlatformRect,
    enabled: bool,
    selected: bool,
    focused: Option<&SemanticActionId>,
) {
    let action_id = SemanticActionId::new(id);
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("{id}.node")),
        action_id: action_id.clone(),
        action: PlatformUiAction::Shell(action),
        bounds,
        label: label.to_owned(),
        description: description.to_owned(),
        enabled,
        selected,
        focused: focused == Some(&action_id),
        icon: icon_for_action(PlatformIconAction::Shell(action)),
    });
    nodes.push(SemanticNode::control(
        format!("{id}.node"),
        if selected {
            SemanticRole::Checkbox
        } else {
            SemanticRole::Button
        },
        label,
        description,
        enabled,
        selected,
        action_id,
    ));
}

pub(super) fn centered_button(viewport: Vec2, index: usize) -> PlatformRect {
    PlatformRect::from_xywh(
        viewport.x * 0.5 - 180.0,
        150.0 + index as f32 * 58.0,
        360.0,
        48.0,
    )
}

pub(super) fn menu_copy(route: MainMenuRoute) -> (&'static str, &'static str) {
    match route {
        MainMenuRoute::NewWorkshop => ("New Workshop", "Create a blank deterministic galaxy."),
        MainMenuRoute::Continue => (
            "Continue",
            "Open the last explicitly selected validated Workshop save.",
        ),
        MainMenuRoute::ClassicSector => (
            "Classic Sector",
            "Play the frozen deterministic RulesV1 sector.",
        ),
        MainMenuRoute::Settings => ("Settings", "Open presentation and accessibility settings."),
        MainMenuRoute::Credits => ("Credits", "Show developer and product credits."),
        #[cfg(not(target_arch = "wasm32"))]
        MainMenuRoute::Quit => ("Quit", "Exit NYON."),
    }
}

pub(super) fn menu_slug(route: MainMenuRoute) -> &'static str {
    match route {
        MainMenuRoute::NewWorkshop => "new-workshop",
        MainMenuRoute::Continue => "continue",
        MainMenuRoute::ClassicSector => "classic-sector",
        MainMenuRoute::Settings => "settings",
        MainMenuRoute::Credits => "credits",
        #[cfg(not(target_arch = "wasm32"))]
        MainMenuRoute::Quit => "quit",
    }
}
