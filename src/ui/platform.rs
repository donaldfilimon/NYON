//! Shared layout and primitive rendering for the product shell and Workshop.
pub use super::platform_inspector::PlatformSightedText;

use crate::app::client_runtime::MainMenuRoute;
use crate::engine::primitives::PrimitiveBatch;
use crate::ui::UiIcon;
use crate::ui::accessibility::SemanticActionId;
use crate::ui::accessibility::SemanticNode;
use crate::ui::accessibility::SemanticNodeId;
use crate::ui::accessibility::SemanticRect;
use crate::ui::accessibility::SemanticRole;
use crate::ui::accessibility::SemanticTree;
use crate::ui::platform_projection::collect_semantic_sources;
use crate::ui::platform_sdf::PlatformTextOverflow;
use crate::ui::platform_sdf::PlatformTextRole;
use crate::ui::platform_sdf::PlatformVisibleNodeRecord;
use crate::ui::platform_sdf::PlatformVisibleNodeState;
use crate::ui::workshop::WorkshopUiIntent;
use crate::ui::workshop_layout::WorkshopLayout;
use crate::ui::workshop_layout::WorkshopLayoutMode;
use crate::ui::workshop_view::WorkshopViewAction;
use crate::workshop::session::WorkshopAction;
use glam::Vec2;

mod shell;
mod workshop;

pub use shell::{
    PlatformFallbackCode, ShellPlatformInput, build_shell_platform_frame, install_guide_batches,
};
use shell::{draw_fallback, fallback_bounds};
pub use workshop::{
    WorkshopMarkerLayer, WorkshopMarkerWitness, WorkshopSceneCoverage,
    build_workshop_platform_frame, build_workshop_platform_frame_for_view,
    build_workshop_platform_frame_with_layout, draw_workshop_scene,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl PlatformRect {
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            max: Vec2::new(x + width.max(0.0), y + height.max(0.0)),
        }
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.is_finite() && point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    pub fn width(self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    pub fn height(self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let intersection = Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        (intersection.width() > 0.0 && intersection.height() > 0.0).then_some(intersection)
    }

    pub fn contains_rect(self, other: Self) -> bool {
        other.min.cmpge(self.min).all() && other.max.cmple(self.max).all()
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.intersection(other)
            .is_some_and(|rect| rect.width() > 0.01 && rect.height() > 0.01)
    }

    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

impl From<PlatformRect> for SemanticRect {
    fn from(value: PlatformRect) -> Self {
        Self {
            min: value.min.to_array(),
            max: value.max.to_array(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformBackground {
    Full,
    ChromeOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShellUiAction {
    Menu(MainMenuRoute),
    CloseSettings,
    CycleUiScale,
    ToggleReducedMotion,
    ToggleHighContrast,
    DismissCredits,
    DismissRecovery,
    ContinueRecovery,
    ReturnToMainMenu,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformUiAction {
    Shell(ShellUiAction),
    Workshop(SemanticActionId),
    WorkshopView(WorkshopViewAction),
    Guide(super::guide::GuideAction),
}

/// Typed construction source for the frozen icon vocabulary. Workshop actions
/// retain their originating intent here instead of decoding semantic ID strings.
pub enum PlatformIconAction<'a> {
    Shell(ShellUiAction),
    Workshop(&'a WorkshopUiIntent),
    WorkshopView(WorkshopViewAction),
    Guide(super::guide::GuideAction),
}

pub fn icon_for_action(action: PlatformIconAction<'_>) -> Option<UiIcon> {
    match action {
        PlatformIconAction::Shell(action) => icon_for_shell_action(action),
        PlatformIconAction::Workshop(intent) => icon_for_workshop_intent(intent),
        PlatformIconAction::WorkshopView(action) => icon_for_view_action(action),
        PlatformIconAction::Guide(action) => match action {
            super::guide::GuideAction::Open => Some(UiIcon::Help),
            super::guide::GuideAction::Close => Some(UiIcon::Close),
            super::guide::GuideAction::Show(_) | super::guide::GuideAction::MainMenu => None,
        },
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformControl {
    pub semantic_id: SemanticNodeId,
    pub action_id: SemanticActionId,
    pub action: PlatformUiAction,
    pub bounds: PlatformRect,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub selected: bool,
    pub focused: bool,
    pub icon: Option<UiIcon>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformUiFrame {
    pub viewport: Vec2,
    pub layout: WorkshopLayout,
    pub background: PlatformBackground,
    pub drawer: Option<PlatformRect>,
    pub modal: Option<PlatformRect>,
    pub controls: Vec<PlatformControl>,
    pub logical_focus_order: Vec<SemanticActionId>,
    pub semantics: SemanticTree,
    pub title: String,
    pub status_lines: Vec<String>,
    pub sighted_text: Vec<PlatformSightedText>,
    pub visible_nodes: Vec<PlatformVisibleNodeRecord>,
    pub(crate) source_records: Vec<PlatformVisibleNodeRecord>,
    pub(crate) operational_status: Option<String>,
    pub high_contrast: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct PlatformTextRun {
    origin: Vec2,
    scale: f32,
    value: String,
}

impl PlatformTextRun {
    fn bounds(&self) -> PlatformRect {
        PlatformRect::from_xywh(
            self.origin.x,
            self.origin.y,
            self.value.len() as f32 * 6.0 * self.scale,
            7.0 * self.scale,
        )
    }
}

impl PlatformUiFrame {
    /// Adds presentation status through the shared semantic/sighted source.
    /// Task 3 uses this for the durable-exit pending message before installation.
    pub fn append_status_line(&mut self, value: impl Into<String>) {
        let value = value.into();
        let insertion = self.status_lines.len().min(3);
        self.status_lines.insert(insertion, value.clone());
        self.operational_status = Some(value);
        self.rebuild_visible_nodes();
    }

    /// Reconciles post-build focus changes into controls and visible witnesses.
    pub fn reconcile_focused(&mut self, focused: Option<&SemanticActionId>) {
        for control in &mut self.controls {
            control.focused = focused == Some(&control.action_id);
        }
        self.rebuild_visible_nodes();
    }

    pub fn hit_test(&self, point: Vec2) -> Option<&PlatformControl> {
        self.controls
            .iter()
            .find(|control| control.enabled && control.bounds.contains(point))
    }

    pub fn action(&self, action_id: &SemanticActionId) -> Option<&PlatformUiAction> {
        self.controls
            .iter()
            .find(|control| control.enabled && &control.action_id == action_id)
            .map(|control| &control.action)
    }

    pub fn focus_order(&self) -> Vec<SemanticActionId> {
        self.logical_focus_order.clone()
    }

    fn title_run(&self) -> Option<PlatformTextRun> {
        (self.background == PlatformBackground::Full
            || self.layout.mode == WorkshopLayoutMode::Wide)
            .then(|| PlatformTextRun {
                origin: self.layout.top_bar.min + Vec2::new(18.0, 14.0),
                scale: 2.0,
                value: self.title.to_ascii_uppercase(),
            })
    }

    fn status_runs(&self) -> Vec<PlatformTextRun> {
        let origin = if self.background == PlatformBackground::ChromeOnly {
            self.layout
                .left_panel
                .map_or(self.layout.top_bar.min, |panel| panel.min)
                + Vec2::new(12.0, 16.0)
        } else {
            Vec2::new(18.0, 66.0)
        };
        let limit = if self.operational_status.is_some()
            || self.background == PlatformBackground::Full
            || self.layout.mode == WorkshopLayoutMode::Wide
        {
            4
        } else {
            0
        };
        self.status_lines
            .iter()
            .take(limit)
            .enumerate()
            .map(|(index, line)| PlatformTextRun {
                origin: origin + Vec2::new(0.0, index as f32 * 15.0),
                scale: 1.25,
                value: line.to_ascii_uppercase(),
            })
            .collect()
    }

    pub fn title_bounds(&self) -> Option<PlatformRect> {
        self.title_run().map(|run| {
            let mut bounds = PlatformRect::from_xywh(
                run.origin.x,
                run.origin.y,
                self.viewport.x - run.origin.x - 18.0,
                18.0 * self.layout.ui_scale * 1.35,
            );
            for control in &self.controls {
                if control.bounds.overlaps(bounds) && control.bounds.min.x > bounds.min.x {
                    bounds.max.x = bounds.max.x.min(control.bounds.min.x - 8.0);
                }
            }
            bounds
        })
    }

    pub fn sighted_text_bounds(&self) -> Vec<PlatformRect> {
        self.title_run()
            .into_iter()
            .chain(self.status_runs())
            .map(|run| run.bounds())
            .chain(self.sighted_text.iter().map(|text| text.bounds))
            .collect()
    }

    /// Rebuilds presentation witnesses from source-owned frame/control records.
    /// These witnesses are deliberately excluded from Workshop authority and persistence.
    pub(crate) fn rebuild_visible_nodes(&mut self) {
        self.semantics.root.children.retain(|node| {
            node.id.as_str() != "platform.title"
                && !node.id.as_str().starts_with("platform.status.")
        });
        let mut records = self.source_records.clone();
        if let Some(bounds) = self.title_bounds() {
            let id = SemanticNodeId::new("platform.title");
            let mut node = SemanticNode::text(
                id.as_str(),
                SemanticRole::Heading,
                &self.title,
                "Current screen",
            );
            node.bounds = Some(bounds.into());
            self.semantics.root.children.push(node);
            records.push(PlatformVisibleNodeRecord {
                semantic_id: id,
                action_id: None,
                display_text: self.title.clone(),
                semantic_name: self.title.clone(),
                semantic_description: "Current screen".to_owned(),
                semantic_value: None,
                role: PlatformTextRole::SectionTitle,
                overflow: PlatformTextOverflow::SingleLineEllipsis,
                bounds,
                clip: Some(self.layout.top_bar),
                state: PlatformVisibleNodeState {
                    enabled: true,
                    ..Default::default()
                },
                icon: None,
                prewrapped_lines: None,
            });
        }
        let workshop_frame = self.semantics.root.id.as_str() == "workshop.application";
        for (index, run) in self
            .status_runs()
            .into_iter()
            .enumerate()
            .filter(|(index, _)| {
                !workshop_frame
                    || self
                        .operational_status
                        .as_ref()
                        .is_some_and(|status| self.status_lines.get(*index) == Some(status))
            })
        {
            let id = SemanticNodeId::new(format!("platform.status.{index}"));
            let bounds = if workshop_frame {
                // Pending operational status is presentation over the map, outside
                // control chrome, including compact layouts without a left panel.
                let canvas = self.layout.canvas;
                PlatformRect::from_xywh(
                    canvas.min.x + 8.0,
                    canvas.min.y + 8.0,
                    canvas.width() - 16.0,
                    (81.0 * self.layout.ui_scale).min(canvas.height() - 16.0),
                )
            } else {
                let line_height = 15.0 * self.layout.ui_scale * 1.35;
                PlatformRect::from_xywh(
                    run.origin.x,
                    66.0 + index as f32 * line_height,
                    self.viewport.x - run.origin.x - 18.0,
                    line_height,
                )
            };
            let mut node = SemanticNode::text(
                id.as_str(),
                SemanticRole::Status,
                &self.status_lines[index],
                "Platform status",
            );
            node.bounds = Some(bounds.into());
            self.semantics.root.children.push(node);
            records.push(PlatformVisibleNodeRecord {
                semantic_id: id,
                action_id: None,
                display_text: self.status_lines[index].clone(),
                semantic_name: self.status_lines[index].clone(),
                semantic_description: "Platform status".to_owned(),
                semantic_value: None,
                role: PlatformTextRole::Status,
                overflow: if workshop_frame {
                    PlatformTextOverflow::Wrap
                } else {
                    PlatformTextOverflow::SingleLineEllipsis
                },
                bounds,
                clip: Some(bounds),
                state: PlatformVisibleNodeState {
                    enabled: true,
                    ..Default::default()
                },
                icon: None,
                prewrapped_lines: None,
            });
        }
        records.extend(self.controls.iter().map(|control| {
            let variable_label = self
                .semantics
                .node(control.semantic_id.as_str())
                .is_some_and(|node| {
                    matches!(
                        node.role,
                        SemanticRole::TreeItem | SemanticRole::TextInput | SemanticRole::Option
                    )
                });
            PlatformVisibleNodeRecord {
                semantic_id: control.semantic_id.clone(),
                action_id: Some(control.action_id.clone()),
                display_text: control.label.clone(),
                semantic_name: control.label.clone(),
                semantic_description: control.description.clone(),
                semantic_value: control.selected.then(|| "selected".to_owned()),
                role: PlatformTextRole::Control,
                overflow: if variable_label {
                    PlatformTextOverflow::SingleLineEllipsis
                } else {
                    PlatformTextOverflow::Wrap
                },
                bounds: if variable_label {
                    PlatformRect::from_xywh(
                        control.bounds.min.x + 3.0,
                        control.bounds.min.y + 3.0,
                        (control.bounds.width() - 6.0).max(0.0),
                        (control.bounds.height() - 6.0).max(0.0),
                    )
                } else {
                    control.bounds
                },
                clip: Some(control.bounds),
                state: PlatformVisibleNodeState {
                    enabled: control.enabled,
                    selected: control.selected,
                    focused: control.focused,
                },
                icon: control.icon,
                prewrapped_lines: None,
            }
        }));
        if self.layout.mode == WorkshopLayoutMode::Medium
            && let Some(control) = self.controls.iter().find(|control| {
                matches!(
                    control.action,
                    PlatformUiAction::WorkshopView(WorkshopViewAction::OpenNavigator)
                )
            })
        {
            records.push(PlatformVisibleNodeRecord {
                semantic_id: control.semantic_id.clone(),
                action_id: Some(control.action_id.clone()),
                display_text: control.label.clone(),
                semantic_name: control.label.clone(),
                semantic_description: control.description.clone(),
                semantic_value: None,
                role: PlatformTextRole::Control,
                overflow: PlatformTextOverflow::SingleLineEllipsis,
                bounds: PlatformRect::from_xywh(
                    self.controls
                        .iter()
                        .find(|control| {
                            control.action_id == WorkshopViewAction::OpenCreator.action_id()
                        })
                        .map_or(8.0, |control| control.bounds.max.x + 8.0),
                    self.layout.top_bar.min.y + 8.0,
                    112.0,
                    28.0,
                ),
                clip: Some(self.layout.top_bar),
                state: PlatformVisibleNodeState {
                    enabled: control.enabled,
                    selected: control.selected,
                    focused: control.focused,
                },
                icon: None,
                prewrapped_lines: None,
            });
        }
        records.extend(self.sighted_text.iter().map(|text| {
            let display_text = text.lines.concat();
            let role = match text.kind {
                super::workshop::InspectorTextKind::Title
                | super::workshop::InspectorTextKind::SectionHeading => {
                    PlatformTextRole::SectionTitle
                }
                super::workshop::InspectorTextKind::Fact => PlatformTextRole::Body,
            };
            PlatformVisibleNodeRecord {
                semantic_id: text.semantic_id.clone(),
                action_id: None,
                display_text,
                semantic_name: text.label.clone(),
                semantic_description: String::new(),
                semantic_value: None,
                role,
                overflow: PlatformTextOverflow::Wrap,
                bounds: text.bounds,
                clip: Some(text.clip),
                state: PlatformVisibleNodeState {
                    enabled: true,
                    ..Default::default()
                },
                icon: None,
                prewrapped_lines: Some(text.lines.clone()),
            }
        }));
        let semantic_sources = collect_semantic_sources(&self.semantics);
        for record in &mut records {
            if let Some(source) = semantic_sources.get(&record.semantic_id) {
                record.semantic_name.clone_from(&source.name);
                record.semantic_description.clone_from(&source.description);
                record.semantic_value.clone_from(&source.value);
            }
        }
        let modal_ids = self.modal_semantic_ids().unwrap_or_default();
        if !modal_ids.is_empty() {
            records.retain(|record| modal_ids.contains(&record.semantic_id));
        }
        let geometry = records
            .iter()
            .map(|record| (record.semantic_id.clone(), record.bounds.into()))
            .collect::<std::collections::BTreeMap<_, SemanticRect>>();
        if workshop_frame {
            for node in self
                .semantics
                .nodes_depth_first()
                .into_iter()
                .filter(|node| {
                    node.visible
                        && presentable_semantic_role(node.role)
                        && (modal_ids.is_empty() || modal_ids.contains(&node.id))
                })
            {
                assert!(
                    geometry.contains_key(&node.id),
                    "visible presentable semantic source {} lacks an explicit materialization record",
                    node.id
                );
            }
            apply_presented_semantic_scope(
                &mut self.semantics.root,
                &geometry,
                (!modal_ids.is_empty()).then_some(&modal_ids),
            );
        }
        apply_missing_semantic_geometry(&mut self.semantics.root, &geometry);
        self.visible_nodes = records;
    }

    pub(crate) fn modal_semantic_ids(&self) -> Option<std::collections::BTreeSet<SemanticNodeId>> {
        self.modal?;
        self.semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.role == SemanticRole::Dialog)
            .map(|dialog| {
                let mut ids = std::collections::BTreeSet::new();
                collect_semantic_ids(dialog, &mut ids);
                ids
            })
    }

    /// Compatibility entry point for primitive-only callers. Chrome is SDF.
    pub fn draw(&self, batch: &mut PrimitiveBatch) {
        let mut overlay = PlatformPrimitiveOverlay::default();
        if self.append_primitive_overlay(&mut overlay).is_ok() {
            *batch = overlay.batch;
        } else {
            batch.clear();
        }
    }

    pub fn append_primitive_overlay(
        &self,
        overlay: &mut PlatformPrimitiveOverlay,
    ) -> Result<(), super::UiBatchError> {
        for control in self
            .controls
            .iter()
            .filter(|control| control.focused && control.enabled)
        {
            let start = overlay.batch.vertices().len();
            draw_focus(&mut overlay.batch, control.bounds);
            overlay.witnesses.push(PlatformDecorationWitness {
                kind: PlatformDecorationKind::Focus,
                bounds: control.bounds,
                owning_node: Some(control.semantic_id.clone()),
                vertex_span: [start, overlay.batch.vertices().len()],
            });
        }
        overlay.validate(self)
    }
}

fn collect_semantic_ids(node: &SemanticNode, ids: &mut std::collections::BTreeSet<SemanticNodeId>) {
    ids.insert(node.id.clone());
    for child in &node.children {
        collect_semantic_ids(child, ids);
    }
}

fn apply_missing_semantic_geometry(
    node: &mut SemanticNode,
    geometry: &std::collections::BTreeMap<SemanticNodeId, SemanticRect>,
) {
    if node.bounds.is_none()
        && let Some(bounds) = geometry.get(&node.id)
    {
        node.bounds = Some(*bounds);
    }
    for child in &mut node.children {
        apply_missing_semantic_geometry(child, geometry);
    }
}

fn apply_presented_semantic_scope(
    node: &mut SemanticNode,
    geometry: &std::collections::BTreeMap<SemanticNodeId, SemanticRect>,
    modal_scope: Option<&std::collections::BTreeSet<SemanticNodeId>>,
) {
    fn apply(
        node: &mut SemanticNode,
        geometry: &std::collections::BTreeMap<SemanticNodeId, SemanticRect>,
        modal_scope: Option<&std::collections::BTreeSet<SemanticNodeId>>,
    ) -> bool {
        let mut contains_modal_node = false;
        for child in &mut node.children {
            contains_modal_node |= apply(child, geometry, modal_scope);
        }

        if let Some(scope) = modal_scope {
            let admitted = scope.contains(&node.id);
            if admitted {
                if presentable_semantic_role(node.role) {
                    node.visible = geometry.contains_key(&node.id);
                }
            } else if contains_modal_node {
                // A hidden or disabled ancestor removes the admitted dialog from
                // browser and native accessibility trees. Keep only the structural
                // path to the dialog exposed; sibling background branches remain
                // hidden and disabled below.
                node.visible = true;
                node.enabled = true;
            } else {
                node.visible = false;
                node.enabled = false;
            }
            admitted || contains_modal_node
        } else {
            if presentable_semantic_role(node.role) {
                node.visible = geometry.contains_key(&node.id);
            }
            false
        }
    }

    apply(node, geometry, modal_scope);
}

fn presentable_semantic_role(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Button
            | SemanticRole::TreeItem
            | SemanticRole::Heading
            | SemanticRole::Text
            | SemanticRole::Status
            | SemanticRole::Alert
            | SemanticRole::ListItem
            | SemanticRole::Option
            | SemanticRole::Checkbox
            | SemanticRole::TextInput
            | SemanticRole::SpinButton
    )
}

/// Only decorations currently emitted by the platform are admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformDecorationKind {
    Focus,
    Fallback(PlatformFallbackCode),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformDecorationWitness {
    pub kind: PlatformDecorationKind,
    pub bounds: PlatformRect,
    pub owning_node: Option<SemanticNodeId>,
    pub vertex_span: [usize; 2],
}

#[derive(Default)]
pub struct PlatformPrimitiveOverlay {
    pub batch: PrimitiveBatch,
    pub witnesses: Vec<PlatformDecorationWitness>,
}

impl PlatformPrimitiveOverlay {
    /// Exact, contiguous ownership plus actual finite geometry is mandatory.
    /// Reconstructing permitted decorations also rejects panels disguised as focus.
    pub fn validate(&self, frame: &PlatformUiFrame) -> Result<(), super::UiBatchError> {
        let invalid = || super::UiBatchError::NonFiniteGeometry;
        let viewport = PlatformRect {
            min: Vec2::ZERO,
            max: frame.viewport,
        };
        let mut cursor = 0;
        let mut owners = std::collections::BTreeSet::new();
        for witness in &self.witnesses {
            let [start, end] = witness.vertex_span;
            if start != cursor
                || end <= start
                || end > self.batch.vertices().len()
                || !witness.bounds.min.is_finite()
                || !witness.bounds.max.is_finite()
                || !viewport.min.is_finite()
                || !viewport.max.is_finite()
                || witness.bounds.width() <= 0.0
                || witness.bounds.height() <= 0.0
                || !viewport.contains_rect(witness.bounds)
            {
                return Err(invalid());
            }
            let mut expected = PrimitiveBatch::default();
            match witness.kind {
                PlatformDecorationKind::Focus => {
                    let owner = witness.owning_node.as_ref().ok_or_else(invalid)?;
                    if !owners.insert(owner) {
                        return Err(invalid());
                    }
                    let control = frame
                        .controls
                        .iter()
                        .find(|control| &control.semantic_id == owner)
                        .ok_or_else(invalid)?;
                    if !control.enabled
                        || !control.focused
                        || control.bounds != witness.bounds
                        || control.bounds.width() < 4.0
                        || control.bounds.height() < 4.0
                        || !frame
                            .visible_nodes
                            .iter()
                            .any(|node| &node.semantic_id == owner && node.state.focused)
                    {
                        return Err(invalid());
                    }
                    draw_focus(&mut expected, witness.bounds);
                }
                PlatformDecorationKind::Fallback(code) => {
                    if self.witnesses.len() != 1
                        || witness.owning_node.is_some()
                        || witness.bounds != fallback_bounds(code)
                    {
                        return Err(invalid());
                    }
                    draw_fallback(&mut expected, code);
                }
            }
            let vertices = &self.batch.vertices()[start..end];
            if vertices != expected.vertices()
                || vertices.iter().any(|vertex| {
                    !witness.bounds.contains(Vec2::from_array(vertex.position))
                        || vertex
                            .color
                            .iter()
                            .chain(vertex.local.iter())
                            .any(|value| !value.is_finite())
                        || vertex.shape != 0
                })
            {
                return Err(invalid());
            }
            cursor = end;
        }
        if cursor != self.batch.vertices().len() {
            return Err(invalid());
        }
        Ok(())
    }
}

fn draw_focus(batch: &mut PrimitiveBatch, bounds: PlatformRect) {
    let min = bounds.min + Vec2::ONE;
    let max = bounds.max - Vec2::ONE;
    let color = [0.95, 0.58, 0.12, 1.0];
    for (from, to) in [
        (min, Vec2::new(max.x, min.y)),
        (Vec2::new(max.x, min.y), max),
        (max, Vec2::new(min.x, max.y)),
        (Vec2::new(min.x, max.y), min),
    ] {
        batch.line(from, to, 2.0, color);
    }
}

/// Shared App installation boundary. Both destinations are replaced together;
/// this presentation-only helper cannot mutate session, store, or semantic state.
pub fn install_platform_batches(
    frame: &PlatformUiFrame,
    metrics: &super::AtlasMetrics,
    ui: &mut super::UiBatch,
    overlay: &mut PrimitiveBatch,
) -> Result<Vec<PlatformDecorationWitness>, super::UiBatchError> {
    let composed = super::platform_sdf::build_platform_ui_batch(frame, metrics).and_then(|sdf| {
        let mut primitive = PlatformPrimitiveOverlay::default();
        frame.append_primitive_overlay(&mut primitive)?;
        Ok((sdf.batch, primitive))
    });
    match composed {
        Ok((sdf, primitive)) => {
            (*ui, *overlay) = (sdf, primitive.batch);
            Ok(primitive.witnesses)
        }
        Err(error) => {
            let code = match &error {
                super::UiBatchError::Capacity { .. }
                | super::UiBatchError::PanelCapacity { .. } => PlatformFallbackCode::Capacity,
                super::UiBatchError::NonFiniteGeometry => PlatformFallbackCode::Geometry,
                super::UiBatchError::UnsupportedCharacter(_)
                | super::UiBatchError::MissingEntry(_) => PlatformFallbackCode::Atlas,
            };
            let mut fallback = PlatformPrimitiveOverlay::default();
            draw_fallback(&mut fallback.batch, code);
            fallback.witnesses.push(PlatformDecorationWitness {
                kind: PlatformDecorationKind::Fallback(code),
                bounds: fallback_bounds(code),
                owning_node: None,
                vertex_span: [0, fallback.batch.vertices().len()],
            });
            if fallback.validate(frame).is_err() {
                fallback.batch.clear();
            }
            (*ui, *overlay) = (super::UiBatch::default(), fallback.batch);
            Err(error)
        }
    }
}

pub(crate) fn apply_platform_control_geometry(
    tree: &mut SemanticTree,
    controls: &mut [PlatformControl],
) {
    use std::collections::{BTreeMap, BTreeSet};

    let existing = tree
        .nodes_depth_first()
        .into_iter()
        .filter_map(|node| node.action_id.clone())
        .collect::<BTreeSet<_>>();
    for control in controls.iter() {
        if !existing.contains(&control.action_id) {
            tree.root.children.push(SemanticNode::control(
                format!("platform.{}", control.action_id.as_str()),
                SemanticRole::Button,
                &control.label,
                &control.description,
                control.enabled,
                control.selected,
                control.action_id.clone(),
            ));
        }
    }

    let identities = tree
        .nodes_depth_first()
        .into_iter()
        .filter_map(|node| {
            node.action_id
                .clone()
                .map(|action| (action, node.id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    for control in controls.iter_mut() {
        if let Some(id) = identities.get(&control.action_id) {
            control.semantic_id = id.clone();
        }
    }
    let geometry = controls
        .iter()
        .map(|control| (control.action_id.clone(), control.bounds.into()))
        .collect::<BTreeMap<_, SemanticRect>>();
    fn apply(node: &mut SemanticNode, geometry: &BTreeMap<SemanticActionId, SemanticRect>) {
        if let Some(action) = &node.action_id {
            node.bounds = geometry.get(action).copied();
            node.visible = node.bounds.is_some();
        }
        for child in &mut node.children {
            apply(child, geometry);
        }
    }
    apply(&mut tree.root, &geometry);
}

fn push_guide_control(
    controls: &mut Vec<PlatformControl>,
    nodes: &mut Vec<SemanticNode>,
    bounds: PlatformRect,
    focused: Option<&SemanticActionId>,
) {
    let action_id = SemanticActionId::new("guide.open");
    nodes.push(SemanticNode::control(
        "guide.open.node",
        SemanticRole::Button,
        "Player guide (F1)",
        "Objectives, controls, first successes and saving. Holds simulation time while reading.",
        true,
        false,
        action_id.clone(),
    ));
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new("guide.open.node"),
        action_id: action_id.clone(),
        action: PlatformUiAction::Guide(super::guide::GuideAction::Open),
        bounds,
        label: "Player guide (F1)".to_owned(),
        description: "How to play NYON".to_owned(),
        enabled: true,
        selected: false,
        focused: focused == Some(&action_id),
        icon: icon_for_action(PlatformIconAction::Guide(super::guide::GuideAction::Open)),
    });
}

fn icon_for_shell_action(action: ShellUiAction) -> Option<UiIcon> {
    match action {
        ShellUiAction::Menu(MainMenuRoute::Continue) => Some(UiIcon::Load),
        ShellUiAction::CloseSettings
        | ShellUiAction::DismissCredits
        | ShellUiAction::DismissRecovery => Some(UiIcon::Close),
        ShellUiAction::ContinueRecovery => Some(UiIcon::Check),
        ShellUiAction::CycleUiScale
        | ShellUiAction::ToggleReducedMotion
        | ShellUiAction::ToggleHighContrast => Some(UiIcon::Settings),
        ShellUiAction::Menu(MainMenuRoute::Settings) => Some(UiIcon::Settings),
        ShellUiAction::ReturnToMainMenu
        | ShellUiAction::Menu(
            MainMenuRoute::NewWorkshop | MainMenuRoute::ClassicSector | MainMenuRoute::Credits,
        ) => None,
        #[cfg(not(target_arch = "wasm32"))]
        ShellUiAction::Menu(MainMenuRoute::Quit) => None,
    }
}

fn icon_for_view_action(action: WorkshopViewAction) -> Option<UiIcon> {
    match action {
        WorkshopViewAction::OpenCreator => Some(UiIcon::Crosshair),
        WorkshopViewAction::OpenNavigator => Some(UiIcon::Zoom),
        WorkshopViewAction::CloseDrawer => Some(UiIcon::Close),
        WorkshopViewAction::ShowInspector => Some(UiIcon::Zoom),
        WorkshopViewAction::ShowStatus => Some(UiIcon::Zoom),
        WorkshopViewAction::ScrollPrevious
        | WorkshopViewAction::ScrollNext
        | WorkshopViewAction::ScrollInspectorPrevious
        | WorkshopViewAction::ScrollInspectorNext
        | WorkshopViewAction::ScrollTimelinePrevious
        | WorkshopViewAction::ScrollTimelineNext
        | WorkshopViewAction::ShowHierarchy
        | WorkshopViewAction::ShowBranches => None,
    }
}

fn icon_for_workshop_intent(intent: &WorkshopUiIntent) -> Option<UiIcon> {
    match intent {
        WorkshopUiIntent::Dispatch(WorkshopAction::Pause) => Some(UiIcon::Pause),
        WorkshopUiIntent::Dispatch(WorkshopAction::Resume(_)) => Some(UiIcon::Speed),
        WorkshopUiIntent::Dispatch(WorkshopAction::StepOnce) => Some(UiIcon::Speed),
        WorkshopUiIntent::Dispatch(WorkshopAction::RequestSave) => Some(UiIcon::Save),
        WorkshopUiIntent::Dispatch(WorkshopAction::RequestLoad(_)) => Some(UiIcon::Load),
        WorkshopUiIntent::Dispatch(WorkshopAction::Submit(_)) => Some(UiIcon::Check),
        WorkshopUiIntent::CloseRemovalConfirmation | WorkshopUiIntent::CloseCreatorForm => {
            Some(UiIcon::Close)
        }
        WorkshopUiIntent::OpenCreatorForm { .. } | WorkshopUiIntent::SelectEntity(_) => {
            Some(UiIcon::Crosshair)
        }
        WorkshopUiIntent::SetReducedMotion(_) | WorkshopUiIntent::SetHighContrast(_) => {
            Some(UiIcon::Settings)
        }
        WorkshopUiIntent::Dispatch(
            WorkshopAction::Undo
            | WorkshopAction::Redo(_)
            | WorkshopAction::SelectBranch(_)
            | WorkshopAction::RequestExport
            | WorkshopAction::RequestImport(_),
        )
        | WorkshopUiIntent::OpenRemovalConfirmation(_)
        | WorkshopUiIntent::ReturnToMainMenu
        | WorkshopUiIntent::EditCreatorField { .. } => None,
    }
}

fn safe_viewport(viewport: Vec2) -> Vec2 {
    if viewport.is_finite() {
        viewport.max(Vec2::new(640.0, 480.0))
    } else {
        Vec2::new(640.0, 480.0)
    }
}
