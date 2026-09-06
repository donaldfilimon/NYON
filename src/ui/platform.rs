//! Shared layout and primitive rendering for the product shell and Workshop.

use glam::Vec2;

use crate::ui::UiIcon;
use crate::workshop::session::WorkshopAction;
use crate::{
    app::client_runtime::{ClientScreen, MainMenuCapability, MainMenuRoute},
    engine::{backend::BackendKind, primitives::PrimitiveBatch},
    preferences::UserPreferencesV1,
    presentation::workshop::WorkshopSceneFrame,
    ui::{
        accessibility::{
            SemanticActionId, SemanticNode, SemanticNodeId, SemanticRect, SemanticRole,
            SemanticTree,
        },
        platform_inspector::{apply_platform_sighted_text_geometry, build_inspector_sighted_text},
        platform_projection::{build_source_records, collect_semantic_sources, modal_presentation},
        platform_sdf::{
            PlatformTextOverflow, PlatformTextRole, PlatformVisibleNodeRecord,
            PlatformVisibleNodeState,
        },
        workshop::{WorkshopControl, WorkshopUiIntent, WorkshopUiModel},
        workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
        workshop_view::{NavigatorSection, WorkshopDrawer, WorkshopViewAction, WorkshopViewState},
    },
};

pub use super::platform_inspector::PlatformSightedText;

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

fn fallback_bounds(code: PlatformFallbackCode) -> PlatformRect {
    PlatformRect::from_xywh(8.0, 8.0, code.text().len() as f32 * 6.0, 7.0)
}

fn draw_fallback(batch: &mut PrimitiveBatch, code: PlatformFallbackCode) {
    batch.text(Vec2::splat(8.0), 1.0, [1.0, 1.0, 1.0, 1.0], code.text());
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
    guide: &super::guide::GuideFrame,
    metrics: &super::AtlasMetrics,
    ui: &mut super::UiBatch,
    overlay: &mut PrimitiveBatch,
) -> Result<(), super::UiBatchError> {
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
        ClientScreen::ClassicSector | ClientScreen::GalaxyWorkshop => {
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

pub fn build_workshop_platform_frame(
    model: &WorkshopUiModel,
    viewport: Vec2,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    let viewport = safe_viewport(viewport);
    let layout =
        WorkshopLayout::resolve(viewport, 1.0).expect("safe Workshop viewport must resolve");
    build_workshop_platform_frame_with_layout(model, layout, focused)
}

pub fn build_workshop_platform_frame_with_layout(
    model: &WorkshopUiModel,
    layout: WorkshopLayout,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    build_workshop_platform_frame_for_view(model, layout, &WorkshopViewState::default(), focused)
}

pub fn build_workshop_platform_frame_for_view(
    model: &WorkshopUiModel,
    layout: WorkshopLayout,
    view: &WorkshopViewState,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    let viewport = layout.viewport.max;
    let scale = layout.ui_scale;
    let mut controls = Vec::new();
    push_workshop_control(
        &mut controls,
        &model.menu_control,
        PlatformRect::from_xywh(
            (layout.top_bar.max.x - 154.0 * scale).max(layout.top_bar.min.x + 8.0),
            layout.top_bar.min.y + (layout.top_bar.height() - 44.0) * 0.5,
            146.0 * scale,
            44.0,
        ),
        focused,
    );

    if super::workshop_view::wide_navigation_fits(&layout) {
        let left = layout
            .left_panel
            .expect("wide Workshop has a navigator panel");
        let gap = 4.0;
        let tool_width = ((left.width() - 16.0 - gap) * 0.5).max(44.0);
        let tool_height = 44.0;
        let tools_top = left.min.y + 318.0;
        for (visible_index, item) in model.tool_palette.iter().enumerate() {
            let column = visible_index % 2;
            let row = visible_index / 2;
            push_workshop_control(
                &mut controls,
                &item.control,
                PlatformRect::from_xywh(
                    left.min.x + 8.0 + column as f32 * (tool_width + gap),
                    tools_top + row as f32 * 48.0,
                    tool_width,
                    tool_height,
                ),
                focused,
            );
        }
        let projection =
            super::workshop_view::wide_outliner_projection(&layout, model.tool_palette.len())
                .expect("roomy Wide navigation has a resolved outliner projection");
        let window = view.outliner_window(model, &layout);
        for (visible_index, index) in window.range().enumerate() {
            let entry = &model.outliner[index];
            let indent = entry.depth as f32 * 12.0;
            push_workshop_control(
                &mut controls,
                &entry.control,
                PlatformRect::from_xywh(
                    left.min.x + 8.0 + indent,
                    projection.top + visible_index as f32 * projection.row_stride,
                    (left.width() - 16.0 - indent).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
    } else {
        let button_height = 44.0;
        let button_y = if layout.mode == WorkshopLayoutMode::Wide {
            layout.left_panel.unwrap().min.y + 8.0
        } else {
            layout.top_bar.min.y + (layout.top_bar.height() - button_height) * 0.5
        };
        push_view_control(
            &mut controls,
            WorkshopViewAction::OpenCreator,
            "Create",
            PlatformRect::from_xywh(8.0, button_y, 108.0, button_height),
            focused,
            view.open_drawer == Some(WorkshopDrawer::Creator),
        );
        push_view_control(
            &mut controls,
            WorkshopViewAction::OpenNavigator,
            "Navigator",
            if layout.mode == WorkshopLayoutMode::Medium {
                let rail = layout
                    .left_panel
                    .expect("medium Workshop has a navigator rail");
                PlatformRect::from_xywh(rail.center().x - 22.0, rail.min.y + 8.0, 44.0, 44.0)
            } else {
                PlatformRect::from_xywh(122.0, button_y, 118.0, button_height)
            },
            focused,
            view.open_drawer == Some(WorkshopDrawer::Navigator),
        );
    }

    let mut timeline_x = layout.bottom_bar.min.x + 8.0;
    let save_width = if layout.mode == WorkshopLayoutMode::Compact {
        112.0
    } else {
        0.0
    };
    for control in model.timeline.controls.iter().skip(view.timeline_start) {
        let width = if control.action_id.as_str().contains("redo") {
            112.0 * scale
        } else {
            82.0 * scale
        };
        if timeline_x + width > layout.bottom_bar.max.x - 8.0 - save_width {
            break;
        }
        push_workshop_control(
            &mut controls,
            control,
            PlatformRect::from_xywh(
                timeline_x,
                layout.bottom_bar.min.y + (layout.bottom_bar.height() - 44.0) * 0.5,
                width,
                44.0,
            ),
            focused,
        );
        timeline_x += width + 4.0 * scale;
    }

    if let Some(right) = layout.right_panel {
        let right_x = right.min.x + 8.0;
        let branch_gap = 6.0;
        let branch_width = ((right.width() - 16.0 - branch_gap) * 0.5).max(44.0);
        let branch_capacity = super::workshop_view::docked_branch_capacity(&layout);
        let branch_window = super::virtual_list::VisibleWindow::new(
            model.branches.len(),
            branch_capacity,
            view.branch_start,
        );
        for (visible_index, index) in branch_window.range().enumerate() {
            let branch = &model.branches[index];
            push_workshop_control(
                &mut controls,
                &branch.control,
                PlatformRect::from_xywh(
                    right_x,
                    right.min.y + 12.0 + visible_index as f32 * (44.0 + 4.0),
                    branch_width,
                    44.0,
                ),
                focused,
            );
        }
        push_workshop_control(
            &mut controls,
            &model.save.save_control,
            PlatformRect::from_xywh(
                right_x + branch_width + branch_gap,
                right.min.y + 12.0,
                branch_width,
                44.0,
            ),
            focused,
        );
        if view.open_drawer.is_none() {
            for (index, (action, label)) in [
                (
                    WorkshopViewAction::ScrollInspectorPrevious,
                    "Inspector previous",
                ),
                (WorkshopViewAction::ScrollInspectorNext, "Inspector next"),
            ]
            .into_iter()
            .enumerate()
            {
                let gap = 4.0;
                let width = (right.width() - 16.0 - gap) * 0.5;
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        right_x + index as f32 * (width + gap),
                        right.max.y - 148.0,
                        width,
                        44.0,
                    ),
                    focused,
                    false,
                );
            }
        }
        if let Some(remove) = &model.inspector.remove_control {
            push_workshop_control(
                &mut controls,
                remove,
                PlatformRect::from_xywh(
                    right_x,
                    right.max.y - 52.0,
                    (right.width() - 16.0).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
        for (index, preference) in [
            &model.preferences.reduced_motion_control,
            &model.preferences.high_contrast_control,
        ]
        .into_iter()
        .enumerate()
        {
            push_workshop_control(
                &mut controls,
                preference,
                PlatformRect::from_xywh(
                    right_x + index as f32 * (((right.width() - 20.0) * 0.5).max(44.0) + 4.0),
                    right.max.y - 100.0,
                    ((right.width() - 20.0) * 0.5).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
    } else {
        push_workshop_control(
            &mut controls,
            &model.save.save_control,
            PlatformRect::from_xywh(
                layout.bottom_bar.max.x - save_width - 8.0,
                layout.bottom_bar.min.y + (layout.bottom_bar.height() - 44.0) * 0.5,
                save_width,
                44.0,
            ),
            focused,
        );
    }

    let drawer = (!super::workshop_view::wide_navigation_fits(&layout)
        || view.navigator_section == NavigatorSection::Status)
        .then_some(view.open_drawer)
        .flatten()
        .map(|_| layout.drawer_sheet);
    if let (Some(drawer), Some(open_drawer)) = (drawer, view.open_drawer) {
        controls.retain(|control| !drawer.overlaps(control.bounds));
        push_view_control(
            &mut controls,
            WorkshopViewAction::CloseDrawer,
            "Close",
            PlatformRect::from_xywh(drawer.max.x - 108.0, drawer.min.y + 8.0, 100.0, 44.0),
            focused,
            false,
        );
        let body_top = drawer.min.y + 60.0;
        if open_drawer == WorkshopDrawer::Navigator {
            let tab_gap = 4.0;
            let columns = if drawer.width() < 480.0 { 2 } else { 4 };
            let tab_width =
                (drawer.width() - 24.0 - tab_gap * (columns - 1) as f32) / columns as f32;
            for (index, (action, label, selected)) in [
                (
                    WorkshopViewAction::ShowHierarchy,
                    "Hierarchy",
                    view.navigator_section == NavigatorSection::Hierarchy,
                ),
                (
                    WorkshopViewAction::ShowBranches,
                    "Branches",
                    view.navigator_section == NavigatorSection::Branches,
                ),
                (
                    WorkshopViewAction::ShowInspector,
                    "Inspector",
                    view.navigator_section == NavigatorSection::Inspector,
                ),
                (
                    WorkshopViewAction::ShowStatus,
                    "Status",
                    view.navigator_section == NavigatorSection::Status,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        drawer.min.x + 8.0 + (index % columns) as f32 * (tab_width + tab_gap),
                        body_top + (index / columns) as f32 * 52.0,
                        tab_width,
                        44.0,
                    ),
                    focused,
                    selected,
                );
            }
        }
        let body_top = if open_drawer == WorkshopDrawer::Navigator {
            drawer.min.y + super::workshop_view::navigator_header_height(&layout)
        } else {
            body_top
        };
        let body_bottom = drawer.max.y - 56.0;
        match open_drawer {
            WorkshopDrawer::Creator => {
                let window = view.creator_window(model, &layout);
                let gap = 4.0;
                let width = (drawer.width() - 24.0 - gap) * 0.5;
                for (visible_index, index) in window.range().enumerate() {
                    let column = visible_index % 2;
                    let row = visible_index / 2;
                    let y = body_top + row as f32 * 48.0;
                    if y + 44.0 > body_bottom {
                        break;
                    }
                    push_workshop_control(
                        &mut controls,
                        &model.tool_palette[index].control,
                        PlatformRect::from_xywh(
                            drawer.min.x + 8.0 + column as f32 * (width + gap),
                            y,
                            width,
                            44.0,
                        ),
                        focused,
                    );
                }
            }
            WorkshopDrawer::Navigator => match view.navigator_section {
                NavigatorSection::Status => {}
                NavigatorSection::Hierarchy => {
                    let window = view.outliner_window(model, &layout);
                    for (visible_index, index) in window.range().enumerate() {
                        let entry = &model.outliner[index];
                        let indent = entry.depth as f32 * 12.0;
                        let y = body_top + visible_index as f32 * 48.0;
                        if y + 44.0 > body_bottom {
                            break;
                        }
                        push_workshop_control(
                            &mut controls,
                            &entry.control,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0 + indent,
                                y,
                                (drawer.width() - 16.0 - indent).max(44.0),
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
                NavigatorSection::Branches => {
                    let window = view.branch_window(model, &layout);
                    for (visible_index, index) in window.range().enumerate() {
                        let y = body_top + visible_index as f32 * 48.0;
                        if y + 44.0 > body_bottom {
                            break;
                        }
                        push_workshop_control(
                            &mut controls,
                            &model.branches[index].control,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0,
                                y,
                                drawer.width() - 16.0,
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
                NavigatorSection::Inspector => {
                    for (index, preference) in [
                        &model.preferences.reduced_motion_control,
                        &model.preferences.high_contrast_control,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        push_workshop_control(
                            &mut controls,
                            preference,
                            PlatformRect::from_xywh(
                                drawer.min.x
                                    + 8.0
                                    + index as f32 * ((drawer.width() - 20.0) * 0.5 + 4.0),
                                drawer.max.y - 148.0,
                                (drawer.width() - 20.0) * 0.5,
                                44.0,
                            ),
                            focused,
                        );
                    }
                    if let Some(remove) = &model.inspector.remove_control {
                        push_workshop_control(
                            &mut controls,
                            remove,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0,
                                drawer.max.y - 100.0,
                                drawer.width() - 16.0,
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
            },
        }
        push_view_control(
            &mut controls,
            WorkshopViewAction::ScrollPrevious,
            "Previous",
            PlatformRect::from_xywh(drawer.min.x + 8.0, drawer.max.y - 48.0, 108.0, 44.0),
            focused,
            false,
        );
        push_view_control(
            &mut controls,
            WorkshopViewAction::ScrollNext,
            "Next",
            PlatformRect::from_xywh(drawer.max.x - 116.0, drawer.max.y - 48.0, 108.0, 44.0),
            focused,
            false,
        );
    }
    if let Some(dialog) = &model.removal_confirmation {
        let center = layout.canvas.center();
        let y = center.y + 64.0;
        push_workshop_control(
            &mut controls,
            &dialog.cancel_control,
            PlatformRect::from_xywh(center.x - 154.0, y, 148.0, 40.0),
            focused,
        );
        push_workshop_control(
            &mut controls,
            &dialog.confirm_control,
            PlatformRect::from_xywh(center.x + 6.0, y, 148.0, 40.0),
            focused,
        );
    }
    if let Some(dialog) = &model.creator_form {
        let center = layout.canvas.center();
        let start_y =
            (center.y - dialog.fields.len() as f32 * 17.0 - 48.0).max(layout.canvas.min.y + 16.0);
        for (index, field) in dialog.fields.iter().enumerate() {
            push_workshop_control(
                &mut controls,
                &field.control,
                PlatformRect::from_xywh(
                    center.x - 210.0,
                    start_y + index as f32 * 34.0,
                    420.0,
                    30.0,
                ),
                focused,
            );
        }
        let y = start_y + dialog.fields.len() as f32 * 34.0 + 10.0;
        push_workshop_control(
            &mut controls,
            &dialog.cancel_control,
            PlatformRect::from_xywh(center.x - 154.0, y, 100.0, 40.0),
            focused,
        );
        push_workshop_control(
            &mut controls,
            &dialog.submit_control,
            PlatformRect::from_xywh(center.x - 42.0, y, 196.0, 40.0),
            focused,
        );
    }

    let modal_content = modal_presentation(model, &layout);
    let mut modal_actions = model
        .creator_form
        .as_ref()
        .map(|dialog| {
            dialog
                .fields
                .iter()
                .map(|field| field.control.action_id.clone())
                .chain([
                    dialog.cancel_control.action_id.clone(),
                    dialog.submit_control.action_id.clone(),
                ])
                .collect::<Vec<_>>()
        })
        .or_else(|| {
            model.removal_confirmation.as_ref().map(|dialog| {
                vec![
                    dialog.cancel_control.action_id.clone(),
                    dialog.confirm_control.action_id.clone(),
                ]
            })
        });
    let modal = modal_content.as_ref().map(|content| content.bounds);
    if let Some(modal_actions) = modal_actions.as_ref() {
        for control in &mut controls {
            if !modal_actions.contains(&control.action_id) {
                control.enabled = false;
            }
        }
    }

    let sighted_text = build_inspector_sighted_text(model, &layout, view, &controls);
    let mut semantics = model.semantics.clone();
    if let Some(content) = &modal_content {
        let actions = modal_actions.as_mut().unwrap();
        let visible = content
            .visible_rows(view.modal_start(model))
            .collect::<Vec<_>>();
        controls.retain(|control| {
            !content
                .rows
                .iter()
                .any(|row| row.action.as_ref() == Some(&control.action_id))
                || visible
                    .iter()
                    .any(|(row, _)| row.action.as_ref() == Some(&control.action_id))
        });
        for control in &mut controls {
            if let Some((_, bounds)) = visible
                .iter()
                .find(|(row, _)| row.action.as_ref() == Some(&control.action_id))
            {
                control.bounds = *bounds;
            } else if actions.contains(&control.action_id) {
                let right = matches!(
                    control.action_id.as_str(),
                    "creator.submit" | "remove.confirm"
                );
                let width = (content.bounds.width() - 32.0) * 0.5;
                control.bounds = PlatformRect::from_xywh(
                    content.bounds.min.x + 12.0 + if right { width + 8.0 } else { 0.0 },
                    content.bounds.max.y - content.footer_height - 8.0,
                    width,
                    content.footer_height,
                );
            }
        }
        let dialog = semantics
            .root
            .children
            .iter_mut()
            .find(|node| node.role == SemanticRole::Dialog)
            .unwrap();
        dialog.children.push(SemanticNode::text(
            content.title_id.as_str(),
            SemanticRole::Heading,
            &content.title,
            "",
        ));
        if content.scrolling {
            // Existing drawer scroll actions are owned by this dialog while it is active.
            controls.retain(|control| {
                !matches!(
                    control.action_id.as_str(),
                    "view.scroll-previous" | "view.scroll-next"
                )
            });
            for (action, label, x) in [
                (
                    WorkshopViewAction::ScrollPrevious,
                    "Previous",
                    content.bounds.min.x + 12.0,
                ),
                (
                    WorkshopViewAction::ScrollNext,
                    "Next",
                    content.bounds.max.x - 120.0,
                ),
            ] {
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        x,
                        content.bounds.max.y - content.footer_height - 56.0,
                        108.0,
                        40.0,
                    ),
                    focused,
                    false,
                );
                let id = action.action_id();
                actions.push(id.clone());
                dialog.children.push(SemanticNode::control(
                    format!("platform.{id}"),
                    SemanticRole::Button,
                    label,
                    "Scroll modal content",
                    true,
                    false,
                    id,
                ));
            }
        }
    }
    if model.creator_form.is_none() && model.removal_confirmation.is_none() {
        push_guide_control(
            &mut controls,
            &mut semantics.root.children,
            PlatformRect::from_xywh(
                (layout.top_bar.max.x - 304.0 * scale).max(layout.top_bar.min.x + 8.0),
                layout.top_bar.min.y + (layout.top_bar.height() - 44.0) * 0.5,
                142.0 * scale,
                44.0,
            ),
            focused,
        );
    }

    let mut status_lines = vec![
        format!(
            "Tick {} // {} // {} revisions",
            model.diagnostics.tick, model.diagnostics.backend, model.diagnostics.revision_count
        ),
        format!(
            "Save: {}{} // generation {}",
            model.save.status,
            if model.save.dirty { " // DIRTY" } else { "" },
            model
                .save
                .generation
                .map_or_else(|| "none".to_owned(), |value| value.to_string())
        ),
    ];
    if let Some(dialog) = &model.creator_form {
        status_lines.push(format!("Creator: {}", dialog.title));
        status_lines.extend(dialog.preview.iter().take(1).cloned());
        if let Some(message) = &dialog.validation_message {
            status_lines.push(format!("Needs attention: {message}"));
        }
    } else if let Some(dialog) = &model.removal_confirmation {
        status_lines.push(if dialog.blockers.is_empty() {
            format!("Confirm removal of {}", dialog.target_label)
        } else {
            format!(
                "Removal blocked by {} dependent objects",
                dialog.blockers.len()
            )
        });
    } else if !model.inspector.title.is_empty() {
        status_lines.push(format!("Selected: {}", model.inspector.title));
        if let Some(inventory) = model
            .inspector
            .sections
            .iter()
            .find(|section| section.heading == "Inventory" && !section.rows.is_empty())
        {
            status_lines.push(format!(
                "Inventory: {}",
                inventory
                    .rows
                    .iter()
                    .take(3)
                    .map(|row| format!("{} {}", row.value, row.label))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ));
        } else if model.outliner.is_empty() {
            status_lines.push("Start: Create system > star > world. Player guide: F1".to_owned());
        }
    }

    apply_platform_control_geometry(&mut semantics, &mut controls);
    apply_platform_sighted_text_geometry(&mut semantics, &sighted_text, &layout, view);
    let source_records = build_source_records(
        &mut semantics,
        &layout,
        view,
        modal_content
            .as_ref()
            .map(|content| (content, view.modal_start(model))),
    );
    let mut logical_focus_order = modal_actions.clone().unwrap_or_else(|| {
        controls
            .iter()
            .filter(|control| matches!(control.action, PlatformUiAction::WorkshopView(_)))
            .map(|control| control.action_id.clone())
            .chain(model.focus_order().iter().cloned())
            .collect::<Vec<_>>()
    });
    if controls
        .iter()
        .any(|control| control.action_id.as_str() == "guide.open")
    {
        logical_focus_order.push(SemanticActionId::new("guide.open"));
    }
    let mut seen = std::collections::BTreeSet::new();
    logical_focus_order.retain(|action| seen.insert(action.clone()));
    let mut frame = PlatformUiFrame {
        viewport,
        layout,
        background: PlatformBackground::ChromeOnly,
        drawer,
        modal,
        controls,
        logical_focus_order,
        semantics,
        title: "NYON // Galaxy Workshop".to_owned(),
        status_lines,
        sighted_text,
        visible_nodes: Vec::new(),
        source_records,
        operational_status: None,
        high_contrast: model.preferences.high_contrast,
    };
    frame.rebuild_visible_nodes();
    frame
}

fn push_view_control(
    controls: &mut Vec<PlatformControl>,
    action: WorkshopViewAction,
    label: &str,
    bounds: PlatformRect,
    focused: Option<&SemanticActionId>,
    selected: bool,
) {
    let action_id = action.action_id();
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("platform.{}", action_id.as_str())),
        action_id: action_id.clone(),
        action: PlatformUiAction::WorkshopView(action),
        bounds,
        label: label.to_owned(),
        description: format!("{label} Workshop drawer"),
        enabled: true,
        selected,
        focused: focused == Some(&action_id),
        icon: icon_for_action(PlatformIconAction::WorkshopView(action)),
    });
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopMarkerLayer {
    SystemRing,
    StarHalo,
    StarCore,
    StarSelectionRing,
    World,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkshopMarkerWitness {
    pub layer: WorkshopMarkerLayer,
    pub source_index: usize,
    pub center: Vec2,
    pub radius: f32,
    pub color: [f32; 4],
    pub ring_thickness: Option<f32>,
    pub packed_shape: u32,
    pub vertex_span: [usize; 2],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkshopSceneCoverage {
    pub system_markers: Vec<Vec2>,
    pub star_markers: Vec<Vec2>,
    pub world_markers: Vec<Vec2>,
    pub marker_witnesses: Vec<WorkshopMarkerWitness>,
}

pub fn draw_workshop_scene(
    scene: &WorkshopSceneFrame,
    layout: &WorkshopLayout,
    batch: &mut PrimitiveBatch,
    high_contrast: bool,
) -> WorkshopSceneCoverage {
    let canvas_min = layout.canvas.min;
    let canvas_max = layout.canvas.max;
    let points = scene
        .systems
        .iter()
        .map(|system| Vec2::new(system.position_radius[0], system.position_radius[1]))
        .collect::<Vec<_>>();
    let (center, scale) = fit_points(&points, canvas_min, canvas_max);
    let project = |point: Vec2| (point - center) * scale + (canvas_min + canvas_max) * 0.5;

    for lane in &scene.lanes {
        let from = project(Vec2::new(lane.from_to_x[0], lane.from_to_y[0]));
        let to = project(Vec2::new(lane.from_to_x[1], lane.from_to_y[1]));
        batch.line(
            from,
            to,
            lane.color_width[3].max(1.0),
            [
                lane.color_width[0],
                lane.color_width[1],
                lane.color_width[2],
                0.9,
            ],
        );
    }
    let mut system_markers = Vec::with_capacity(scene.systems.len());
    let mut star_markers = Vec::with_capacity(scene.stars.len());
    let mut marker_witnesses =
        Vec::with_capacity(scene.systems.len() + scene.stars.len() * 3 + scene.worlds.len());
    for (source_index, system) in scene.systems.iter().enumerate() {
        let point = project(Vec2::new(
            system.position_radius[0],
            system.position_radius[1],
        ));
        let radius = marker_radius(
            system.position_radius[3],
            0.12,
            if high_contrast { 18.0 } else { 16.0 },
        );
        system_markers.push(point);
        let ring_thickness = if high_contrast { 3.0 } else { 2.0 };
        let vertex_start = batch.vertices().len();
        batch.ring(point, radius, ring_thickness, system.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::SystemRing,
            source_index,
            center: point,
            radius,
            color: system.color,
            ring_thickness: Some(ring_thickness),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }

    let projected_stars = scene
        .stars
        .iter()
        .enumerate()
        .map(|(source_index, star)| {
            let point = project(Vec2::new(star.position_radius[0], star.position_radius[1]));
            let visual_scale = (star.position_radius[3] / 0.075).clamp(0.5, 2.0);
            (source_index, star, point, visual_scale)
        })
        .collect::<Vec<_>>();
    for (source_index, star, point, visual_scale) in &projected_stars {
        let radius = if high_contrast { 10.5 } else { 9.5 } * visual_scale;
        let color = [star.color[0], star.color[1], star.color[2], 0.28];
        star_markers.push(*point);
        let vertex_start = batch.vertices().len();
        batch.disc(*point, radius, color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarHalo,
            source_index: *source_index,
            center: *point,
            radius,
            color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for (source_index, star, point, visual_scale) in &projected_stars {
        let radius = if high_contrast { 5.5 } else { 4.75 } * visual_scale;
        let vertex_start = batch.vertices().len();
        batch.disc(*point, radius, star.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarCore,
            source_index: *source_index,
            center: *point,
            radius,
            color: star.color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for (source_index, star, point, visual_scale) in &projected_stars {
        if star.flags[0] == 0 {
            continue;
        }
        let radius = if high_contrast { 14.0 } else { 12.75 } * visual_scale;
        let color = if high_contrast {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [1.0, 0.92, 0.32, 1.0]
        };
        let ring_thickness = if high_contrast { 3.0 } else { 2.25 };
        let vertex_start = batch.vertices().len();
        batch.ring(*point, radius, ring_thickness, color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarSelectionRing,
            source_index: *source_index,
            center: *point,
            radius,
            color,
            ring_thickness: Some(ring_thickness),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    let mut world_markers = Vec::with_capacity(scene.worlds.len());
    for (source_index, world) in scene.worlds.iter().enumerate() {
        let point = project(Vec2::new(
            world.position_radius[0],
            world.position_radius[1],
        ));
        let radius = marker_radius(world.position_radius[3], 0.05, 3.5);
        world_markers.push(point);
        let vertex_start = batch.vertices().len();
        batch.disc(point, radius, world.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::World,
            source_index,
            center: point,
            radius,
            color: world.color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for shipment in &scene.shipments {
        let point = project(Vec2::new(
            shipment.position_units[0],
            shipment.position_units[1],
        ));
        batch.disc(point, 3.0, shipment.color);
    }
    for entry in scene.semantic_entries.iter().filter(|entry| {
        matches!(
            entry.role,
            crate::presentation::workshop::WorkshopSemanticRole::System
        )
    }) {
        let point = project(Vec2::new(entry.bounds[0], entry.bounds[1]));
        batch.text(
            point + Vec2::new(10.0, -16.0),
            1.0,
            if high_contrast {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.58, 0.82, 1.0, 1.0]
            },
            &truncate_ascii(&entry.label.to_ascii_uppercase(), 18),
        );
    }
    WorkshopSceneCoverage {
        system_markers,
        star_markers,
        world_markers,
        marker_witnesses,
    }
}

fn marker_radius(source_radius: f32, reference_radius: f32, pixel_radius: f32) -> f32 {
    let scale = if source_radius.is_finite() && source_radius > 0.0 {
        (source_radius / reference_radius).clamp(0.5, 2.0)
    } else {
        1.0
    };
    pixel_radius * scale
}

fn push_workshop_control(
    controls: &mut Vec<PlatformControl>,
    control: &WorkshopControl,
    bounds: PlatformRect,
    focused: Option<&SemanticActionId>,
) {
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("platform.{}", control.action_id.as_str())),
        action_id: control.action_id.clone(),
        action: PlatformUiAction::Workshop(control.action_id.clone()),
        bounds,
        label: control.label.clone(),
        description: control.description.clone(),
        enabled: control.enabled,
        selected: control.selected,
        focused: focused == Some(&control.action_id),
        icon: icon_for_action(PlatformIconAction::Workshop(&control.intent)),
    });
}

#[allow(clippy::too_many_arguments)]
fn push_shell_control(
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

fn centered_button(viewport: Vec2, index: usize) -> PlatformRect {
    PlatformRect::from_xywh(
        viewport.x * 0.5 - 180.0,
        150.0 + index as f32 * 58.0,
        360.0,
        48.0,
    )
}

fn menu_copy(route: MainMenuRoute) -> (&'static str, &'static str) {
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

fn menu_slug(route: MainMenuRoute) -> &'static str {
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

fn safe_viewport(viewport: Vec2) -> Vec2 {
    if viewport.is_finite() {
        viewport.max(Vec2::new(640.0, 480.0))
    } else {
        Vec2::new(640.0, 480.0)
    }
}

fn fit_points(points: &[Vec2], min: Vec2, max: Vec2) -> (Vec2, f32) {
    if points.is_empty() {
        return (Vec2::ZERO, 1.0);
    }
    let mut low = points[0];
    let mut high = points[0];
    for point in points.iter().copied().filter(|point| point.is_finite()) {
        low = low.min(point);
        high = high.max(point);
    }
    let span = (high - low).max(Vec2::splat(1.0));
    let available = (max - min - Vec2::splat(80.0)).max(Vec2::splat(1.0));
    (
        (low + high) * 0.5,
        (available.x / span.x).min(available.y / span.y),
    )
}

fn truncate_ascii(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}
