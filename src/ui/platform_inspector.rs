//! Sighted Workshop inspector layout and deterministic line virtualization.

use std::collections::BTreeMap;

use crate::ui::{
    AtlasMetrics, FontWeight,
    accessibility::{SemanticNode, SemanticNodeId, SemanticRect, SemanticTree},
    workshop::{InspectorTextKind, InspectorTextRecord, WorkshopUiModel},
    workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
    workshop_view::{NavigatorSection, WorkshopDrawer, WorkshopViewState},
};

use super::platform::{PlatformControl, PlatformRect};

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformSightedText {
    pub semantic_id: SemanticNodeId,
    pub kind: InspectorTextKind,
    pub label: String,
    pub value: Option<String>,
    pub lines: Vec<String>,
    pub bounds: PlatformRect,
    pub clip: PlatformRect,
}

#[derive(Clone, Copy, Debug)]
struct InspectorGeometry {
    container: PlatformRect,
    content: PlatformRect,
    footer_top: f32,
}

pub(crate) fn build_inspector_sighted_text(
    model: &WorkshopUiModel,
    layout: &WorkshopLayout,
    view: &WorkshopViewState,
    controls: &[PlatformControl],
) -> Vec<PlatformSightedText> {
    let Some(geometry) = inspector_geometry(layout, view, controls) else {
        return Vec::new();
    };
    let title = model.inspector.title_record();
    let title_display = title.label.to_ascii_uppercase();
    let title_lines = wrap_measured(
        &title_display,
        geometry.content.width(),
        title.kind,
        layout.ui_scale,
    );
    let title_line_height = line_height(title.kind, layout.ui_scale);
    let title_height = title_lines.len() as f32 * title_line_height + 3.0;
    let title_bounds = PlatformRect::from_xywh(
        geometry.content.min.x,
        geometry.content.min.y,
        geometry.content.width(),
        title_height,
    );
    if title_bounds.max.y > geometry.footer_top {
        return Vec::new();
    }

    let mut visible = vec![PlatformSightedText {
        semantic_id: title.semantic_id,
        kind: title.kind,
        label: title.label,
        value: title.value,
        lines: title_lines,
        bounds: title_bounds,
        clip: geometry.container,
    }];
    let body = PlatformRect::from_xywh(
        geometry.content.min.x,
        title_bounds.max.y + 6.0,
        geometry.content.width(),
        (geometry.footer_top - title_bounds.max.y - 6.0).max(0.0),
    );
    if body.height() <= 0.0 {
        return visible;
    }

    let records = model.inspector.text_records();
    let context_matches = view.inspector_context_matches(model, layout);
    let start = if context_matches {
        view.inspector_start.min(records.len().saturating_sub(1))
    } else {
        0
    };
    let mut y = body.min.y;
    for (record_index, source) in records.iter().enumerate().skip(start) {
        let display = display_text(source);
        let wrapped = wrap_measured(&display, body.width(), source.kind, layout.ui_scale);
        let skip = if context_matches && record_index == start {
            view.inspector_line_offset
                .min(wrapped.len().saturating_sub(1))
        } else {
            0
        };
        let line_height = line_height(source.kind, layout.ui_scale);
        let remaining = (body.max.y - y - 3.0).max(0.0);
        let capacity = (remaining / line_height).floor() as usize;
        if capacity == 0 {
            break;
        }
        let lines = wrapped
            .iter()
            .skip(skip)
            .take(capacity)
            .cloned()
            .collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }
        let height = lines.len() as f32 * line_height + 3.0;
        let bounds = PlatformRect::from_xywh(body.min.x, y, body.width(), height);
        debug_assert!(body.contains_rect(bounds));
        visible.push(PlatformSightedText {
            semantic_id: source.semantic_id.clone(),
            kind: source.kind,
            label: source.label.clone(),
            value: source.value.clone(),
            lines,
            bounds,
            clip: geometry.container,
        });
        y = bounds.max.y;
        if skip + capacity < wrapped.len() {
            break;
        }
    }
    visible
}

pub(crate) fn wrapped_line_count(
    record: &InspectorTextRecord,
    layout: &WorkshopLayout,
    drawer_inspector: bool,
) -> usize {
    let region = if drawer_inspector && layout.mode != WorkshopLayoutMode::Wide {
        Some(layout.drawer_sheet)
    } else {
        layout.right_panel
    };
    region.map_or(1, |region| {
        wrap_measured(
            &display_text(record),
            (region.width() - 16.0).max(0.0),
            record.kind,
            layout.ui_scale,
        )
        .len()
        .max(1)
    })
}

pub(crate) fn apply_platform_sighted_text_geometry(
    tree: &mut SemanticTree,
    sighted_text: &[PlatformSightedText],
    layout: &WorkshopLayout,
    view: &WorkshopViewState,
) {
    let geometry = sighted_text
        .iter()
        .map(|record| (record.semantic_id.clone(), record.bounds.into()))
        .collect::<BTreeMap<_, SemanticRect>>();
    let inspector_bounds = inspector_container(layout, view).map(Into::into);
    fn apply(
        node: &mut SemanticNode,
        geometry: &BTreeMap<SemanticNodeId, SemanticRect>,
        inspector_bounds: Option<SemanticRect>,
    ) {
        if node.id.as_str() == "workshop.inspector" {
            node.bounds = inspector_bounds;
            node.visible = inspector_bounds.is_some();
        } else if node.id.as_str().starts_with("inspector.") && node.action_id.is_none() {
            node.bounds = geometry.get(&node.id).copied();
            node.visible = node.bounds.is_some();
        }
        for child in &mut node.children {
            apply(child, geometry, inspector_bounds);
        }
    }
    apply(&mut tree.root, &geometry, inspector_bounds);
}

fn inspector_geometry(
    layout: &WorkshopLayout,
    view: &WorkshopViewState,
    controls: &[PlatformControl],
) -> Option<InspectorGeometry> {
    let container = inspector_container(layout, view)?;
    let content_x = container.min.x + 8.0;
    let content_width = (container.width() - 16.0).max(0.0);
    let top = controls
        .iter()
        .filter(|control| container.overlaps(control.bounds) && is_top_control(control))
        .map(|control| control.bounds.max.y)
        .fold(container.min.y, f32::max)
        + 8.0;
    let footer_top = controls
        .iter()
        .filter(|control| container.overlaps(control.bounds) && is_footer_control(control))
        .map(|control| control.bounds.min.y)
        .fold(container.max.y, f32::min)
        - 8.0;
    (content_width > 0.0 && footer_top > top).then_some(InspectorGeometry {
        container,
        content: PlatformRect::from_xywh(content_x, top, content_width, footer_top - top),
        footer_top,
    })
}

fn inspector_container(layout: &WorkshopLayout, view: &WorkshopViewState) -> Option<PlatformRect> {
    let drawer_inspector = view.open_drawer == Some(WorkshopDrawer::Navigator)
        && view.navigator_section == NavigatorSection::Inspector
        && layout.mode != WorkshopLayoutMode::Wide;
    if drawer_inspector {
        Some(layout.drawer_sheet)
    } else {
        layout.right_panel
    }
}

fn is_top_control(control: &PlatformControl) -> bool {
    let id = control.action_id.as_str();
    id.starts_with("branch.")
        || id == "save.commit"
        || id == "view.close-drawer"
        || id.starts_with("view.show-")
}

fn is_footer_control(control: &PlatformControl) -> bool {
    let id = control.action_id.as_str();
    id.starts_with("view.inspector-scroll-")
        || id == "view.scroll-previous"
        || id == "view.scroll-next"
        || id.starts_with("preferences.")
        || id.starts_with("remove.")
}

fn display_text(record: &InspectorTextRecord) -> String {
    match (&record.kind, &record.value) {
        (InspectorTextKind::Fact, Some(value)) => format!("{}: {value}", record.label),
        _ => record.label.clone(),
    }
    .to_ascii_uppercase()
}

fn line_height(kind: InspectorTextKind, ui_scale: f32) -> f32 {
    font_size(kind) * ui_scale * 1.35
}

fn font_size(kind: InspectorTextKind) -> f32 {
    match kind {
        InspectorTextKind::Title | InspectorTextKind::SectionHeading => 18.0,
        InspectorTextKind::Fact => 15.0,
    }
}

fn font_weight(kind: InspectorTextKind) -> FontWeight {
    match kind {
        InspectorTextKind::Title | InspectorTextKind::SectionHeading => FontWeight::SemiBold,
        InspectorTextKind::Fact => FontWeight::Regular,
    }
}

fn wrap_measured(value: &str, width: f32, kind: InspectorTextKind, ui_scale: f32) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }
    let metrics = AtlasMetrics::embedded().expect("embedded UI atlas metrics are valid JSON");
    let size = font_size(kind) * ui_scale;
    let weight = font_weight(kind);
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in value.chars() {
        let mut candidate = line.clone();
        candidate.push(character);
        let advance = metrics
            .measure_text(size, weight, &candidate)
            .expect("Workshop display strings are validated printable ASCII")
            .advance;
        if !line.is_empty()
            && advance > width - 2.0 * super::platform_sdf::text_ink_padding(ui_scale)
        {
            lines.push(std::mem::take(&mut line));
        }
        line.push(character);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
