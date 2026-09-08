//! Source-owned semantic projection decisions for platform presentation.

use std::collections::BTreeMap;

use super::{
    accessibility::{SemanticNode, SemanticNodeId, SemanticRect, SemanticRole, SemanticTree},
    platform::PlatformRect,
    platform_sdf::{
        PlatformTextOverflow, PlatformTextRole, PlatformVisibleNodeRecord, PlatformVisibleNodeState,
    },
    workshop::WorkshopUiModel,
    workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
    workshop_view::{NavigatorSection, WorkshopDrawer, WorkshopViewState},
};

pub(crate) struct ModalBodyRow {
    pub id: SemanticNodeId,
    pub action: Option<super::accessibility::SemanticActionId>,
    pub text: String,
    pub height: f32,
}

pub(crate) struct ModalPresentation {
    pub bounds: PlatformRect,
    pub body: PlatformRect,
    pub title: String,
    pub title_id: SemanticNodeId,
    pub rows: Vec<ModalBodyRow>,
    pub scrolling: bool,
    pub footer_height: f32,
}

pub(crate) fn modal_presentation(
    model: &WorkshopUiModel,
    layout: &WorkshopLayout,
) -> Option<ModalPresentation> {
    let dialog = model
        .semantics
        .nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)?;
    let width = layout.canvas.width().min(640.0);
    let metrics = super::AtlasMetrics::embedded().expect("embedded atlas is valid");
    let mut rows = Vec::new();
    for node in dialog.children.iter() {
        if node.action_id.is_some() {
            if matches!(
                node.role,
                SemanticRole::TextInput | SemanticRole::SpinButton | SemanticRole::Option
            ) {
                rows.push(ModalBodyRow {
                    id: node.id.clone(),
                    action: node.action_id.clone(),
                    text: String::new(),
                    height: 44.0,
                });
            }
            continue;
        }
        rows.extend(wrapped_source_rows(
            node,
            width - 24.0,
            layout.ui_scale,
            &metrics,
        ));
    }
    let title_height = 18.0 * layout.ui_scale * 1.35;
    let body_height = rows.iter().map(|row| row.height).sum::<f32>();
    let footer_height = if width < 400.0 { 60.0 } else { 44.0 };
    let fixed_height = title_height + 16.0 + footer_height + 16.0;
    let scrolling = body_height + fixed_height > layout.canvas.height();
    let height = (body_height + fixed_height).min(layout.canvas.height());
    let bounds = PlatformRect::from_xywh(
        layout.canvas.center().x - width * 0.5,
        layout.canvas.center().y - height * 0.5,
        width,
        height,
    );
    let body = PlatformRect::from_xywh(
        bounds.min.x + 12.0,
        bounds.min.y + 8.0 + title_height,
        width - 24.0,
        height - fixed_height - if scrolling { 48.0 } else { 0.0 },
    );
    Some(ModalPresentation {
        bounds,
        body,
        title: dialog.name.clone(),
        title_id: SemanticNodeId::new(format!("{}.title", dialog.id)),
        rows,
        scrolling,
        footer_height,
    })
}

impl ModalPresentation {
    pub(crate) fn previous_page_start(&self, current: usize) -> usize {
        let current = current.min(self.rows.len());
        let mut start = current.saturating_sub(1);
        while start > 0 && self.visible_rows(start - 1).count() >= current - (start - 1) {
            start -= 1;
        }
        start
    }

    pub(crate) fn visible_rows(
        &self,
        start: usize,
    ) -> impl Iterator<Item = (&ModalBodyRow, PlatformRect)> {
        let mut y = self.body.min.y;
        self.rows
            .iter()
            .skip(start.min(self.rows.len().saturating_sub(1)))
            .map(move |row| {
                let bounds =
                    PlatformRect::from_xywh(self.body.min.x, y, self.body.width(), row.height);
                y += row.height;
                (row, bounds)
            })
            .take_while(|(_, bounds)| self.body.contains_rect(*bounds))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SemanticSourceRecord {
    pub id: SemanticNodeId,
    pub name: String,
    pub description: String,
    pub value: Option<String>,
}

pub(crate) fn collect_semantic_sources(
    tree: &SemanticTree,
) -> BTreeMap<SemanticNodeId, SemanticSourceRecord> {
    tree.nodes_depth_first()
        .into_iter()
        .map(|node| {
            let source = SemanticSourceRecord {
                id: node.id.clone(),
                name: node.name.clone(),
                description: node.description.clone(),
                value: node.value.clone(),
            };
            (source.id.clone(), source)
        })
        .collect()
}

pub(crate) const SUMMARY_SOURCES: [&str; 6] = [
    "save.status",
    "diagnostics.backend",
    "diagnostics.catalog",
    "diagnostics.digest",
    "diagnostics.tick",
    "outliner.empty",
];

pub(crate) struct SummaryPresentation {
    pub body: PlatformRect,
    pub rows: Vec<ModalBodyRow>,
}

pub(crate) fn summary_presentation(
    tree: &SemanticTree,
    layout: &WorkshopLayout,
    view: &WorkshopViewState,
) -> Option<SummaryPresentation> {
    let body = if view.open_drawer == Some(WorkshopDrawer::Navigator)
        && view.navigator_section == NavigatorSection::Status
    {
        let header = super::workshop_view::navigator_header_height(layout);
        PlatformRect::from_xywh(
            layout.drawer_sheet.min.x + 8.0,
            layout.drawer_sheet.min.y + header,
            layout.drawer_sheet.width() - 16.0,
            layout.drawer_sheet.height() - header - 56.0,
        )
    } else if layout.mode == WorkshopLayoutMode::Wide && view.open_drawer.is_none() {
        let left = layout.left_panel?;
        let top = if super::workshop_view::wide_navigation_fits(layout) {
            8.0
        } else {
            60.0
        };
        PlatformRect::from_xywh(
            left.min.x + 8.0,
            left.min.y + top,
            left.width() - 16.0,
            (left.height() - top - 8.0).min(302.0),
        )
    } else {
        return None;
    };
    let metrics = super::AtlasMetrics::embedded().expect("embedded atlas is valid");
    let rows = SUMMARY_SOURCES
        .into_iter()
        .filter_map(|id| tree.node(id))
        .flat_map(|node| wrapped_source_rows(node, body.width(), layout.ui_scale, &metrics))
        .collect();
    Some(SummaryPresentation { body, rows })
}

fn wrapped_source_rows(
    node: &SemanticNode,
    width: f32,
    scale: f32,
    metrics: &super::AtlasMetrics,
) -> Vec<ModalBodyRow> {
    let font = 15.0 * scale;
    let mut rows = Vec::new();
    let mut line = String::new();
    for character in source_display_text(node).chars() {
        let mut candidate = line.clone();
        candidate.push(character);
        if !line.is_empty()
            && metrics
                .measure_text(font, super::FontWeight::Regular, &candidate)
                .expect("validated Workshop text")
                .advance
                > width - 2.0 * super::platform_sdf::text_ink_padding(scale)
        {
            rows.push(ModalBodyRow {
                id: node.id.clone(),
                action: None,
                text: std::mem::take(&mut line),
                height: font * 1.35,
            });
        }
        line.push(character);
    }
    rows.push(ModalBodyRow {
        id: node.id.clone(),
        action: None,
        text: line,
        height: font * 1.35,
    });
    rows
}

fn materialize_rows(
    tree: &mut SemanticTree,
    rows: &[ModalBodyRow],
    body: PlatformRect,
    start: usize,
    records: &mut Vec<PlatformVisibleNodeRecord>,
) {
    let mut y = body.min.y;
    for row in rows.iter().skip(start.min(rows.len().saturating_sub(1))) {
        let bounds = PlatformRect::from_xywh(body.min.x, y, body.width(), row.height);
        if !body.contains_rect(bounds) {
            break;
        }
        y += row.height;
        if row.action.is_some() {
            continue;
        }
        if let Some(record) = records
            .last_mut()
            .filter(|record| record.semantic_id == row.id)
        {
            record.display_text.push_str(&row.text);
            record
                .prewrapped_lines
                .as_mut()
                .unwrap()
                .push(row.text.clone());
            record.bounds.max.y = record.bounds.min.y
                + (record.prewrapped_lines.as_ref().unwrap().len() - 1) as f32 * row.height
                + row.height;
            set_geometry(&mut tree.root, &row.id, record.bounds.into());
        } else {
            materialize(tree, row.id.as_str(), bounds, body, records);
            let record = records.last_mut().unwrap();
            record.display_text.clone_from(&row.text);
            record.prewrapped_lines = Some(vec![row.text.clone()]);
        }
    }
}

pub(crate) fn build_source_records(
    tree: &mut SemanticTree,
    layout: &WorkshopLayout,
    view: &WorkshopViewState,
    modal_content: Option<(&ModalPresentation, usize)>,
) -> Vec<PlatformVisibleNodeRecord> {
    let mut records = Vec::new();
    for source in SUMMARY_SOURCES {
        set_materialized(&mut tree.root, source, false);
    }
    if let Some(summary) = summary_presentation(tree, layout, view) {
        let start = if view.open_drawer == Some(WorkshopDrawer::Navigator)
            && view.navigator_section == NavigatorSection::Status
        {
            view.summary_start
        } else {
            0
        };
        materialize_rows(tree, &summary.rows, summary.body, start, &mut records);
    }
    if let Some((content, start)) = modal_content {
        for row in &content.rows {
            set_materialized(&mut tree.root, row.id.as_str(), false);
        }
        let title = PlatformRect::from_xywh(
            content.bounds.min.x + 12.0,
            content.bounds.min.y + 8.0,
            content.bounds.width() - 24.0,
            18.0 * layout.ui_scale * 1.35,
        );
        materialize(tree, content.title_id.as_str(), title, title, &mut records);
        if let Some(record) = records.last_mut() {
            record.overflow = PlatformTextOverflow::SingleLineEllipsis;
        }
        materialize_rows(tree, &content.rows, content.body, start, &mut records);
    }
    records
}

fn set_materialized(node: &mut SemanticNode, id: &str, visible: bool) {
    if node.id.as_str() == id {
        node.visible = visible;
    }
    for child in &mut node.children {
        set_materialized(child, id, visible);
    }
}

fn materialize(
    tree: &mut SemanticTree,
    id: &str,
    bounds: PlatformRect,
    clip: PlatformRect,
    records: &mut Vec<PlatformVisibleNodeRecord>,
) {
    let Some(source) = tree.node(id).cloned() else {
        return;
    };
    set_geometry(&mut tree.root, &source.id, bounds.into());
    let display_text = source_display_text(&source);
    records.push(PlatformVisibleNodeRecord {
        semantic_id: source.id,
        action_id: source.action_id,
        display_text,
        semantic_name: source.name,
        semantic_description: source.description,
        semantic_value: source.value,
        role: match source.role {
            SemanticRole::Status => PlatformTextRole::Status,
            SemanticRole::Alert => PlatformTextRole::Alert,
            SemanticRole::Heading => PlatformTextRole::SectionTitle,
            _ => PlatformTextRole::Body,
        },
        overflow: PlatformTextOverflow::Wrap,
        bounds,
        clip: Some(clip),
        state: PlatformVisibleNodeState {
            enabled: source.enabled,
            selected: source.selected,
            focused: false,
        },
        icon: None,
        prewrapped_lines: None,
    });
}

fn source_display_text(source: &SemanticNode) -> String {
    let mut display_text = source.name.clone();
    if let Some(value) = source.value.as_deref().filter(|value| !value.is_empty()) {
        display_text.push_str(": ");
        display_text.push_str(value);
    }
    if !source.description.is_empty()
        && source.description != source.value.as_deref().unwrap_or_default()
    {
        if !display_text.is_empty() {
            display_text.push_str(" - ");
        }
        display_text.push_str(&source.description);
    }
    display_text
}

fn set_geometry(node: &mut SemanticNode, id: &SemanticNodeId, bounds: SemanticRect) {
    if &node.id == id {
        node.bounds = Some(bounds);
        node.visible = true;
    }
    for child in &mut node.children {
        set_geometry(child, id, bounds);
    }
}
