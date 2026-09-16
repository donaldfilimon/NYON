//! Source-owned semantic projection decisions for platform presentation.

use std::collections::BTreeMap;

use super::{
    accessibility::{SemanticNode, SemanticNodeId, SemanticRect, SemanticRole, SemanticTree},
    platform::PlatformRect,
    platform_sdf::{
        PlatformTextOverflow, PlatformTextRole, PlatformVisibleNodeRecord, PlatformVisibleNodeState,
    },
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

/// The single dialog every modal derivation must agree on.
///
/// Presentation, the focus trap's action list, and the modal-identity key are all
/// projections of the *same* dialog. They used to answer that question three separate
/// ways — `modal_presentation` took the first `Dialog` depth-first while the focus trap
/// and the identity key read `creator_form` first and `removal_confirmation` second —
/// and nothing kept the three in step.
///
/// `WorkshopUiModel` can hold both modals at once, and `src/app.rs` does not prevent it
/// (`OpenCreatorForm` does not clear `pending_removal`, `OpenRemovalConfirmation` does
/// not clear `active_creator`). In that state the old code handed the focus trap to the
/// dialog that was *not* rendered, which
/// `focus_trap_follows_the_rendered_dialog_when_both_modals_are_open` pins.
///
/// **No user path into that state has been demonstrated**, and the evidence points the
/// other way: the frame's own trap disables every control outside the modal, and
/// `ui_focus` restricts the Tab order to the modal's actions, so neither pointer nor
/// keyboard appears to reach the second open. Treat this as a latent inconsistency that
/// is now impossible by construction, not as a fixed user-visible bug.
///
/// The durable win is generalization: a dialog type named nowhere in
/// `platform/workshop.rs` now gets a focus trap because it is in the tree.
///
/// Route every modal projection through here so the three cannot drift apart again.
pub(crate) fn modal_dialog(tree: &SemanticTree) -> Option<&SemanticNode> {
    tree.nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)
}

/// Focus-trap action IDs for the active modal, in depth-first order.
///
/// Derived from the semantic tree rather than from named model fields, so a modal type
/// that renders at all also gets a focus trap. Nodes without an `action_id` (body text,
/// validation alerts) are skipped, which reproduces the previous hand-built order for
/// both existing dialogs.
pub(crate) fn modal_action_ids(
    tree: &SemanticTree,
) -> Option<Vec<super::accessibility::SemanticActionId>> {
    fn collect(node: &SemanticNode, output: &mut Vec<super::accessibility::SemanticActionId>) {
        if let Some(action) = &node.action_id {
            output.push(action.clone());
        }
        for child in &node.children {
            collect(child, output);
        }
    }

    let dialog = modal_dialog(tree)?;
    let mut actions = Vec::new();
    for child in dialog.children.iter() {
        collect(child, &mut actions);
    }
    Some(actions)
}

/// Identity key for the active modal, used only to detect that the modal changed.
///
/// The dialog's `name` carries the discriminating content the previous key encoded by
/// hand (`"{title} editor"` for the creator, `"Remove {target}"` for removal), so a
/// different tool or a different removal target still produces a different key.
pub(crate) fn modal_identity(tree: &SemanticTree) -> Option<String> {
    let dialog = modal_dialog(tree)?;
    Some(format!("{}:{}", dialog.id, dialog.name))
}

pub(crate) fn modal_presentation(
    tree: &SemanticTree,
    layout: &WorkshopLayout,
) -> Option<ModalPresentation> {
    let dialog = modal_dialog(tree)?;
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

#[cfg(test)]
mod modal_projection_tests {
    use super::*;
    use crate::ui::accessibility::SemanticActionId;

    fn control(id: &str, action: &str) -> SemanticNode {
        SemanticNode::control(
            id,
            SemanticRole::Button,
            action,
            "",
            true,
            false,
            SemanticActionId::new(action),
        )
    }

    fn tree_with(dialogs: Vec<SemanticNode>) -> SemanticTree {
        SemanticTree {
            root: SemanticNode::container(
                "test.application",
                SemanticRole::Application,
                "test",
                dialogs,
            ),
            announcements: Vec::new(),
        }
    }

    /// The generalization the hand-built list could not provide: a dialog type nobody
    /// named in `platform/workshop.rs` still gets a focus trap, in tree order.
    #[test]
    fn modal_action_ids_covers_a_dialog_type_no_model_field_names() {
        let tree = tree_with(vec![SemanticNode::container(
            "workshop.export-dialog",
            SemanticRole::Dialog,
            "Export galaxy",
            vec![
                SemanticNode::text("export.body", SemanticRole::Text, "Body", "choose a format"),
                control("export.control.format", "export.format"),
                control("export.control.cancel", "export.cancel"),
                control("export.control.confirm", "export.confirm"),
            ],
        )]);

        assert_eq!(
            modal_action_ids(&tree),
            Some(vec![
                SemanticActionId::new("export.format"),
                SemanticActionId::new("export.cancel"),
                SemanticActionId::new("export.confirm"),
            ]),
            "actions must follow tree order and skip nodes carrying no action"
        );
    }

    /// Presentation, the focus trap and the identity key must describe one dialog.
    /// When two dialogs are present the tree order decides, and all three agree because
    /// they share `modal_dialog`.
    #[test]
    fn every_modal_projection_describes_the_same_dialog() {
        let tree = tree_with(vec![
            SemanticNode::container(
                "workshop.removal-dialog",
                SemanticRole::Dialog,
                "Remove Kepler Yard",
                vec![
                    control("removal.control.cancel", "remove.cancel"),
                    control("removal.control.confirm", "remove.confirm"),
                ],
            ),
            SemanticNode::container(
                "workshop.creator-dialog",
                SemanticRole::Dialog,
                "Deposit editor",
                vec![
                    control("creator.control.cancel", "creator.cancel"),
                    control("creator.control.submit", "creator.submit"),
                ],
            ),
        ]);

        let dialog = modal_dialog(&tree).expect("a dialog is present");
        assert_eq!(dialog.id.as_str(), "workshop.removal-dialog");
        assert_eq!(
            modal_action_ids(&tree),
            Some(vec![
                SemanticActionId::new("remove.cancel"),
                SemanticActionId::new("remove.confirm"),
            ]),
            "the focus trap must belong to the dialog that is rendered, not to whichever \
             model field is inspected first"
        );
        assert_eq!(
            modal_identity(&tree).as_deref(),
            Some("workshop.removal-dialog:Remove Kepler Yard")
        );
    }

    /// The identity key exists to notice that the modal changed; a different removal
    /// target must not read as the same modal.
    #[test]
    fn modal_identity_discriminates_between_targets() {
        let one = tree_with(vec![SemanticNode::container(
            "workshop.removal-dialog",
            SemanticRole::Dialog,
            "Remove Kepler Yard",
            vec![control("removal.control.cancel", "remove.cancel")],
        )]);
        let two = tree_with(vec![SemanticNode::container(
            "workshop.removal-dialog",
            SemanticRole::Dialog,
            "Remove Vela Foundry",
            vec![control("removal.control.cancel", "remove.cancel")],
        )]);
        assert_ne!(modal_identity(&one), modal_identity(&two));
    }

    #[test]
    fn no_dialog_means_no_modal_projection() {
        let tree = tree_with(vec![SemanticNode::container(
            "workshop.navigator",
            SemanticRole::Group,
            "Navigator",
            vec![control("nav.control.select", "nav.select")],
        )]);
        assert!(modal_dialog(&tree).is_none());
        assert!(modal_action_ids(&tree).is_none());
        assert!(modal_identity(&tree).is_none());
    }
}
