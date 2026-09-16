//! Focus-order control collection and the semantic tree projection, split out
//! of `workshop.rs`.

use crate::{
    ui::{
        accessibility::{
            AnnouncementKind, SemanticAnnouncement, SemanticNode, SemanticRole, SemanticTree,
        },
        creator::CreatorFieldKind,
    },
    workshop::session::{WorkshopDiagnosticCode, WorkshopSessionSnapshot},
};

use super::{
    BranchChoice, CreatorFormModel, CreatorToolControl, InspectorModel, OutlinerEntry,
    RemovalConfirmation, SaveStatusModel, TimelineModel, WorkshopControl, WorkshopDiagnosticsModel,
    WorkshopPresentationPreferences, fixed_hex, safe_store_status,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn all_controls<'a>(
    menu: &'a WorkshopControl,
    tools: &'a [CreatorToolControl],
    outliner: &'a [OutlinerEntry],
    inspector: &'a InspectorModel,
    timeline: &'a TimelineModel,
    branches: &'a [BranchChoice],
    save: &'a SaveStatusModel,
    removal: Option<&'a RemovalConfirmation>,
    preferences: &'a WorkshopPresentationPreferences,
    creator: Option<&'a CreatorFormModel>,
) -> Vec<&'a WorkshopControl> {
    let mut controls = vec![menu];
    controls.extend(tools.iter().map(|tool| &tool.control));
    controls.extend(outliner.iter().map(|entry| &entry.control));
    if let Some(control) = &inspector.remove_control {
        controls.push(control);
    }
    controls.extend(&timeline.controls);
    controls.extend(branches.iter().map(|branch| &branch.control));
    controls.push(&save.save_control);
    controls.push(&save.library_control);
    controls.push(&preferences.reduced_motion_control);
    controls.push(&preferences.high_contrast_control);
    if let Some(removal) = removal {
        controls.push(&removal.cancel_control);
        controls.push(&removal.confirm_control);
    }
    if let Some(creator) = creator {
        controls.extend(creator.fields.iter().map(|field| &field.control));
        controls.push(&creator.cancel_control);
        controls.push(&creator.submit_control);
    }
    controls
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_semantic_tree(
    snapshot: &WorkshopSessionSnapshot,
    menu: &WorkshopControl,
    tools: &[CreatorToolControl],
    outliner: &[OutlinerEntry],
    inspector: &InspectorModel,
    timeline: &TimelineModel,
    branches: &[BranchChoice],
    save: &SaveStatusModel,
    diagnostics: &WorkshopDiagnosticsModel,
    removal: Option<&RemovalConfirmation>,
    preferences: &WorkshopPresentationPreferences,
    creator: Option<&CreatorFormModel>,
) -> SemanticTree {
    let tool_nodes = tools
        .iter()
        .map(|tool| semantic_control("tool", &tool.control, SemanticRole::Button))
        .collect();
    let mut outliner_nodes = outliner
        .iter()
        .map(|entry| {
            let mut node = semantic_control("outliner", &entry.control, SemanticRole::TreeItem);
            node.hierarchy_level = Some(entry.depth + 1);
            node
        })
        .collect::<Vec<_>>();
    if outliner_nodes.is_empty() {
        outliner_nodes.push(SemanticNode::text(
            "outliner.empty",
            SemanticRole::Text,
            "No galaxy objects",
            "Use the creator tools to begin shaping the galaxy.",
        ));
    }

    let mut inspector_nodes = inspector
        .sections
        .iter()
        .map(|section| {
            let rows = section
                .rows
                .iter()
                .map(|row| {
                    SemanticNode::text(
                        row.semantic_id.as_str(),
                        SemanticRole::Text,
                        &row.label,
                        &row.value,
                    )
                })
                .collect();
            SemanticNode::container(
                section.semantic_id.as_str(),
                SemanticRole::Group,
                section.heading,
                rows,
            )
        })
        .chain(
            inspector
                .remove_control
                .iter()
                .map(|control| semantic_control("inspector", control, SemanticRole::Button)),
        )
        .collect::<Vec<_>>();
    inspector_nodes.insert(
        0,
        SemanticNode::text(
            "inspector.title",
            SemanticRole::Heading,
            format!("Inspector: {}", inspector.title),
            "",
        ),
    );

    let timeline_nodes = timeline
        .controls
        .iter()
        .map(|control| semantic_control("timeline", control, SemanticRole::Button))
        .collect();
    let branch_nodes = branches
        .iter()
        .map(|branch| semantic_control("branches", &branch.control, SemanticRole::Option))
        .collect();
    let save_nodes = vec![
        SemanticNode::text(
            "save.status",
            SemanticRole::Status,
            "Save status",
            format!(
                "{}; {}; generation {}",
                save.status,
                if save.dirty {
                    "unsaved changes"
                } else {
                    "clean"
                },
                save.generation
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
        ),
        semantic_control("save", &save.save_control, SemanticRole::Button),
        semantic_control("save", &save.library_control, SemanticRole::Button),
    ];
    let diagnostic_nodes = vec![
        SemanticNode::text(
            "diagnostics.backend",
            SemanticRole::Text,
            "Graphics backend",
            &diagnostics.backend,
        ),
        SemanticNode::text(
            "diagnostics.catalog",
            SemanticRole::Text,
            "Catalog hash",
            &diagnostics.catalog_hash,
        ),
        SemanticNode::text(
            "diagnostics.digest",
            SemanticRole::Text,
            "State digest",
            &diagnostics.state_digest,
        ),
        SemanticNode::text(
            "diagnostics.tick",
            SemanticRole::Text,
            "Authoritative tick",
            diagnostics.tick.to_string(),
        ),
    ];
    let preference_nodes = vec![
        semantic_control(
            "preferences",
            &preferences.reduced_motion_control,
            SemanticRole::Checkbox,
        ),
        semantic_control(
            "preferences",
            &preferences.high_contrast_control,
            SemanticRole::Checkbox,
        ),
    ];

    let mut children = vec![
        semantic_control("workshop", menu, SemanticRole::Button),
        SemanticNode::container(
            "workshop.tools",
            SemanticRole::Toolbar,
            "Creator tools",
            tool_nodes,
        ),
        SemanticNode::container(
            "workshop.outliner",
            SemanticRole::Tree,
            "Galaxy hierarchy",
            outliner_nodes,
        ),
        SemanticNode::container(
            "workshop.inspector",
            SemanticRole::Region,
            format!("Inspector: {}", inspector.title),
            inspector_nodes,
        ),
        SemanticNode::container(
            "workshop.timeline",
            SemanticRole::Toolbar,
            format!("Timeline at tick {}", timeline.tick),
            timeline_nodes,
        ),
        SemanticNode::container(
            "workshop.branches",
            SemanticRole::List,
            "History branches",
            branch_nodes,
        ),
        SemanticNode::container(
            "workshop.save",
            SemanticRole::Region,
            "Workshop save",
            save_nodes,
        ),
        SemanticNode::container(
            "workshop.diagnostics",
            SemanticRole::Region,
            "Workshop diagnostics",
            diagnostic_nodes,
        ),
        SemanticNode::container(
            "workshop.preferences",
            SemanticRole::Group,
            "Presentation accessibility",
            preference_nodes,
        ),
    ];

    if let Some(removal) = removal {
        let mut removal_nodes = removal
            .blockers
            .iter()
            .enumerate()
            .map(|(index, blocker)| {
                SemanticNode::text(
                    format!("removal.blocker.{index}"),
                    SemanticRole::ListItem,
                    &blocker.label,
                    format!("Blocking entity {}", fixed_hex(&blocker.entity.0)),
                )
            })
            .collect::<Vec<_>>();
        removal_nodes.push(semantic_control(
            "removal",
            &removal.cancel_control,
            SemanticRole::Button,
        ));
        removal_nodes.push(semantic_control(
            "removal",
            &removal.confirm_control,
            SemanticRole::Button,
        ));
        children.push(SemanticNode::container(
            "workshop.removal-dialog",
            SemanticRole::Dialog,
            format!("Remove {}", removal.target_label),
            removal_nodes,
        ));
    }

    if let Some(creator) = creator {
        let mut creator_nodes = creator
            .fields
            .iter()
            .map(|field| {
                let mut node = semantic_control(
                    "creator-field",
                    &field.control,
                    match field.kind {
                        CreatorFieldKind::Text => SemanticRole::TextInput,
                        CreatorFieldKind::Integer => SemanticRole::SpinButton,
                        CreatorFieldKind::Choice => SemanticRole::Option,
                    },
                );
                node.value = Some(field.value.clone());
                node
            })
            .collect::<Vec<_>>();
        creator_nodes.extend(creator.preview.iter().enumerate().map(|(index, line)| {
            SemanticNode::text(
                format!("creator.preview.{index}"),
                SemanticRole::Text,
                "Creator batch preview",
                line,
            )
        }));
        if let Some(message) = &creator.validation_message {
            creator_nodes.push(SemanticNode::text(
                "creator.validation",
                SemanticRole::Alert,
                "Creator form needs attention",
                message,
            ));
        }
        creator_nodes.push(semantic_control(
            "creator",
            &creator.cancel_control,
            SemanticRole::Button,
        ));
        creator_nodes.push(semantic_control(
            "creator",
            &creator.submit_control,
            SemanticRole::Button,
        ));
        children.push(SemanticNode::container(
            "workshop.creator-dialog",
            SemanticRole::Dialog,
            format!("{} editor", creator.title),
            creator_nodes,
        ));
    }

    SemanticTree {
        root: SemanticNode::container(
            "workshop.application",
            SemanticRole::Application,
            "NYON Galaxy Workshop",
            children,
        ),
        announcements: semantic_announcements(snapshot),
    }
}

fn semantic_control(prefix: &str, control: &WorkshopControl, role: SemanticRole) -> SemanticNode {
    SemanticNode::control(
        format!("{prefix}.control.{}", control.action_id.as_str()),
        role,
        &control.label,
        &control.description,
        control.enabled,
        control.selected,
        control.action_id.clone(),
    )
}

fn semantic_announcements(snapshot: &WorkshopSessionSnapshot) -> Vec<SemanticAnnouncement> {
    let mut announcements = snapshot
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let (code, message) = safe_diagnostic(diagnostic.code);
            SemanticAnnouncement {
                kind: AnnouncementKind::Error,
                code,
                message: message.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    announcements.push(SemanticAnnouncement {
        kind: AnnouncementKind::Status,
        code: "workshop-store-status",
        message: safe_store_status(&snapshot.store.status).to_owned(),
    });
    announcements
}

fn safe_diagnostic(code: WorkshopDiagnosticCode) -> (&'static str, &'static str) {
    match code {
        WorkshopDiagnosticCode::ActionQueueFull => (
            "action-queue-full",
            "The Workshop action could not be queued.",
        ),
        WorkshopDiagnosticCode::CreatorRejected => (
            "creator-rejected",
            "The creator edit was rejected. Review its fields and dependencies.",
        ),
        WorkshopDiagnosticCode::HistoryRejected => (
            "history-rejected",
            "The history action is unavailable in the current Workshop state.",
        ),
        WorkshopDiagnosticCode::SimulationFault => (
            "simulation-fault",
            "The deterministic simulation paused after a recoverable fault.",
        ),
        WorkshopDiagnosticCode::ArchiveRejected => (
            "archive-rejected",
            "The Workshop archive could not be validated.",
        ),
        WorkshopDiagnosticCode::StoreRejected => (
            "store-rejected",
            "The Workshop storage operation did not complete.",
        ),
        WorkshopDiagnosticCode::StoreProtocol => (
            "store-protocol",
            "The Workshop storage adapter returned an unexpected result.",
        ),
    }
}
