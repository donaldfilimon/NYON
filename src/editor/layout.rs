use std::collections::BTreeMap;

use glam::Vec2;

use crate::scenario::{ScenarioDraft, ScenarioIssue, ValidationReport};

use super::{
    ConfirmationAction, EditorAction, EditorLayoutKind, EditorRect, EditorSection, EditorState,
    EditorWidget, HEADER_HEIGHT, MIN_TARGET, ROW_GAP, ROW_HEIGHT, WidgetKind, action_id,
    widget_id_for_issue,
};

impl EditorState {
    pub fn rebuild_widgets(&mut self, viewport: Vec2) {
        self.viewport = Vec2::new(finite_extent(viewport.x), finite_extent(viewport.y));
        self.layout_kind = if self.viewport.x >= 1100.0 {
            EditorLayoutKind::Wide
        } else {
            EditorLayoutKind::Compact
        };
        self.synchronize_typed_buffers();
        let report = self.draft.validation_report();
        let actions = [
            (EditorAction::Revert, "REVERT"),
            (EditorAction::FactoryDefaults, "FACTORY DEFAULTS"),
            (EditorAction::RegenerateFromSeed, "REGENERATE FROM SEED"),
            (EditorAction::Load, "LOAD"),
            (EditorAction::Save, "SAVE"),
            (EditorAction::Cancel, "CANCEL"),
            (EditorAction::ApplyAndRestart, "APPLY AND RESTART"),
        ];
        let columns = ((self.viewport.x / 150.0).floor() as usize).clamp(1, actions.len());
        let action_rows = actions.len().div_ceil(columns);
        let footer_height = (action_rows as f32 * ROW_HEIGHT).min(self.viewport.y);
        let footer_top = (self.viewport.y - footer_height).max(HEADER_HEIGHT.min(self.viewport.y));
        let mut widgets = Vec::new();

        append_status_widgets(&mut widgets, self, &report);

        let mut content_top = HEADER_HEIGHT.min(footer_top);
        match self.layout_kind {
            EditorLayoutKind::Wide => {
                let nav_x = 16.0_f32.min(self.viewport.x);
                let nav_width = 208.0_f32.min((self.viewport.x - nav_x).max(0.0));
                append_section_widgets(&mut widgets, self.section, nav_x, content_top, nav_width);
                if self.section == EditorSection::Worlds {
                    let world_top = content_top + 3.0 * (ROW_HEIGHT + ROW_GAP) + ROW_GAP;
                    for index in 0..self.draft.worlds.len() {
                        widgets.push(world_widget(
                            index,
                            &self.draft.worlds[index].name,
                            EditorRect::new(
                                nav_x,
                                world_top + index as f32 * (ROW_HEIGHT + ROW_GAP),
                                nav_width,
                                ROW_HEIGHT,
                            ),
                            self.selected_world == index,
                        ));
                    }
                }

                let preview_x = nav_x + nav_width + 16.0;
                let form_width = 420.0_f32.min(self.viewport.x);
                let form_x = (self.viewport.x - form_width - 16.0).max(preview_x);
                let preview_width = (form_x - preview_x - 16.0).max(0.0);
                widgets.push(preview_widget(
                    &self.draft,
                    self.selected_world,
                    EditorRect::new(
                        preview_x,
                        content_top,
                        preview_width,
                        (footer_top - content_top).max(0.0),
                    ),
                ));
                self.content_rect = EditorRect::new(
                    form_x,
                    content_top,
                    form_width,
                    (footer_top - content_top).max(0.0),
                );
            }
            EditorLayoutKind::Compact => {
                let tab_width = self.viewport.x / 3.0;
                for (index, section) in [
                    EditorSection::Worlds,
                    EditorSection::Rules,
                    EditorSection::Validation,
                ]
                .into_iter()
                .enumerate()
                {
                    widgets.push(section_widget(
                        section,
                        EditorRect::new(
                            index as f32 * tab_width,
                            content_top,
                            tab_width.max(MIN_TARGET.min(self.viewport.x)),
                            ROW_HEIGHT.min((footer_top - content_top).max(0.0)),
                        ),
                        self.section == section,
                    ));
                }
                content_top = (content_top + ROW_HEIGHT + ROW_GAP).min(footer_top);
                if self.section == EditorSection::Worlds {
                    let world_columns = ((self.viewport.x / 80.0).floor() as usize)
                        .clamp(1, self.draft.worlds.len());
                    let world_rows = self.draft.worlds.len().div_ceil(world_columns);
                    let world_width = self.viewport.x / world_columns as f32;
                    for index in 0..self.draft.worlds.len() {
                        let row = index / world_columns;
                        let column = index % world_columns;
                        widgets.push(world_widget(
                            index,
                            &self.draft.worlds[index].name,
                            EditorRect::new(
                                column as f32 * world_width,
                                content_top + row as f32 * ROW_HEIGHT,
                                world_width.max(MIN_TARGET.min(self.viewport.x)),
                                ROW_HEIGHT.min((footer_top - content_top).max(0.0)),
                            ),
                            self.selected_world == index,
                        ));
                    }
                    content_top =
                        (content_top + world_rows as f32 * ROW_HEIGHT + ROW_GAP).min(footer_top);
                    let preview_height = 88.0_f32.min((footer_top - content_top).max(0.0));
                    widgets.push(preview_widget(
                        &self.draft,
                        self.selected_world,
                        EditorRect::new(
                            16.0,
                            content_top,
                            (self.viewport.x - 32.0).max(0.0),
                            preview_height,
                        ),
                    ));
                    content_top = (content_top + preview_height + ROW_GAP).min(footer_top);
                }
                self.content_rect = EditorRect::new(
                    16.0_f32.min(self.viewport.x),
                    content_top,
                    (self.viewport.x - 32.0).max(0.0),
                    (footer_top - content_top).max(0.0),
                );
            }
        }

        let fields = section_field_descriptors(self.section, self.selected_world, &report);
        let content_height = fields.len() as f32 * (ROW_HEIGHT + ROW_GAP);
        self.maximum_scroll = (content_height - self.content_rect.height).max(0.0);
        self.scroll = self.scroll.clamp(0.0, self.maximum_scroll);
        let mut errors = BTreeMap::new();
        for issue in report.issues() {
            if issue.severity == crate::scenario::IssueSeverity::Error {
                errors
                    .entry(widget_id_for_issue(&issue.field))
                    .or_insert_with(|| issue.message.clone());
            }
        }
        for id in &self.parse_errors {
            errors.insert(id.clone(), "INVALID VALUE".into());
        }
        for (index, (id, label, editable)) in fields.into_iter().enumerate() {
            let y = self.content_rect.y + index as f32 * (ROW_HEIGHT + ROW_GAP) - self.scroll;
            let visible = y >= self.content_rect.y
                && y + ROW_HEIGHT <= self.content_rect.y + self.content_rect.height;
            let mut value = self
                .text_buffers
                .get(&id)
                .cloned()
                .unwrap_or_else(|| validation_value(&id, report.issues()).unwrap_or_default());
            if self.focused.as_deref() == Some(id.as_str()) && !self.ime_preedit.is_empty() {
                value.push_str(&self.ime_preedit);
            }
            widgets.push(EditorWidget {
                id: id.clone(),
                kind: WidgetKind::Field,
                rect: EditorRect::new(self.content_rect.x, y, self.content_rect.width, ROW_HEIGHT),
                label,
                value,
                enabled: editable,
                error: errors.get(&id).cloned(),
                focusable: editable,
                visible,
            });
        }

        let cell_width = self.viewport.x / columns as f32;
        for (index, (action, label)) in actions.into_iter().enumerate() {
            let row = index / columns;
            let column = index % columns;
            let y = self.viewport.y - footer_height + row as f32 * ROW_HEIGHT;
            let enabled = action != EditorAction::ApplyAndRestart || self.can_apply();
            widgets.push(EditorWidget {
                id: action_id(action).into(),
                kind: WidgetKind::Action(action),
                rect: EditorRect::new(
                    column as f32 * cell_width,
                    y,
                    cell_width.max(MIN_TARGET.min(self.viewport.x)),
                    ROW_HEIGHT
                        .min(self.viewport.y)
                        .max(MIN_TARGET.min(self.viewport.y)),
                ),
                label: label.into(),
                value: String::new(),
                enabled,
                error: None,
                focusable: true,
                visible: true,
            });
        }

        if let Some(confirmation) = self.confirmation {
            for widget in &mut widgets {
                widget.enabled = false;
                widget.focusable = false;
            }
            append_confirmation_widgets(&mut widgets, self.viewport, confirmation);
        }
        self.widgets = widgets;
    }
}

fn finite_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn append_status_widgets(
    widgets: &mut Vec<EditorWidget>,
    editor: &EditorState,
    report: &ValidationReport,
) {
    widgets.push(EditorWidget {
        id: "status.header".into(),
        kind: WidgetKind::Status,
        rect: EditorRect::new(
            0.0,
            0.0,
            editor.viewport.x,
            HEADER_HEIGHT.min(editor.viewport.y),
        ),
        label: "SCENARIO EDITOR".into(),
        value: format!(
            "{} | {} ERRORS | {} WARNINGS",
            section_label(editor.section),
            report.error_count(),
            report.warning_count()
        ),
        enabled: true,
        error: None,
        focusable: false,
        visible: true,
    });
    if let Some(message) = &editor.store_message {
        widgets.push(EditorWidget {
            id: "status.store".into(),
            kind: WidgetKind::Status,
            rect: EditorRect::new(
                (editor.viewport.x * 0.5).max(0.0),
                0.0,
                editor.viewport.x * 0.5,
                HEADER_HEIGHT.min(editor.viewport.y),
            ),
            label: "STORAGE".into(),
            value: message.to_ascii_uppercase(),
            enabled: true,
            error: None,
            focusable: false,
            visible: true,
        });
    }
}

fn append_section_widgets(
    widgets: &mut Vec<EditorWidget>,
    selected: EditorSection,
    x: f32,
    y: f32,
    width: f32,
) {
    for (index, section) in [
        EditorSection::Worlds,
        EditorSection::Rules,
        EditorSection::Validation,
    ]
    .into_iter()
    .enumerate()
    {
        widgets.push(section_widget(
            section,
            EditorRect::new(
                x,
                y + index as f32 * (ROW_HEIGHT + ROW_GAP),
                width,
                ROW_HEIGHT,
            ),
            selected == section,
        ));
    }
}

const fn section_label(section: EditorSection) -> &'static str {
    match section {
        EditorSection::Worlds => "WORLDS",
        EditorSection::Rules => "RULES",
        EditorSection::Validation => "VALIDATION",
    }
}

fn section_widget(section: EditorSection, rect: EditorRect, selected: bool) -> EditorWidget {
    let focusable = rect.width >= MIN_TARGET && rect.height >= MIN_TARGET;
    EditorWidget {
        id: format!("section.{}", section_label(section).to_ascii_lowercase()),
        kind: WidgetKind::Action(EditorAction::SelectSection(section)),
        rect,
        label: section_label(section).into(),
        value: if selected {
            "SELECTED".into()
        } else {
            String::new()
        },
        enabled: true,
        error: None,
        focusable,
        visible: rect.width > 0.0 && rect.height > 0.0,
    }
}

fn world_widget(index: usize, name: &str, rect: EditorRect, selected: bool) -> EditorWidget {
    let focusable = rect.width >= MIN_TARGET && rect.height >= MIN_TARGET;
    EditorWidget {
        id: format!("world.select.{index}"),
        kind: WidgetKind::Action(EditorAction::SelectWorld(index)),
        rect,
        label: format!("WORLD {index}"),
        value: if selected {
            format!("{name} | SELECTED")
        } else {
            name.into()
        },
        enabled: true,
        error: None,
        focusable,
        visible: rect.width > 0.0 && rect.height > 0.0,
    }
}

fn preview_widget(draft: &ScenarioDraft, index: usize, rect: EditorRect) -> EditorWidget {
    let world = &draft.worlds[index];
    let owner = match world.owner {
        None => "NEUTRAL",
        Some(crate::game::model::Faction::Union) => "UNION",
        Some(crate::game::model::Faction::Helix) => "HELIX",
        Some(crate::game::model::Faction::Choir) => "CHOIR",
    };
    EditorWidget {
        id: "preview.world".into(),
        kind: WidgetKind::Preview,
        rect,
        label: format!("TACTICAL PREVIEW | WORLD {index} | {}", world.name),
        value: format!(
            "{owner} | X {} Y {} | ENERGY {} | DEFENSE {} | FIELDS {}/{}/{}",
            world.x,
            world.y,
            world.energy,
            world.defense,
            world.atmosphere,
            world.hydrosphere,
            world.topology
        ),
        enabled: true,
        error: None,
        focusable: false,
        visible: rect.width > 0.0 && rect.height > 0.0,
    }
}

fn section_field_descriptors(
    section: EditorSection,
    selected_world: usize,
    report: &ValidationReport,
) -> Vec<(String, String, bool)> {
    match section {
        EditorSection::Rules => [
            ("scenario.seed", "SEED", true),
            ("rule.tick_hz", "TICK RATE", false),
            ("rule.win_world_count", "WIN WORLDS", true),
            ("rule.ai_period_ticks", "AI PERIOD", true),
            ("rule.hazard_first_tick", "HAZARD FIRST", true),
            ("rule.hazard_period_ticks", "HAZARD PERIOD", true),
            ("rule.hazard_duration_ticks", "HAZARD DURATION", true),
            ("rule.maximum_energy", "MAX ENERGY", true),
            ("rule.maximum_defense", "MAX DEFENSE", true),
            ("rule.minimum_launch", "MIN LAUNCH", true),
            ("rule.field_raise_cost", "RAISE COST", true),
            ("rule.field_lower_refund", "LOWER REFUND", true),
            ("rule.base_fleet_speed", "BASE SPEED", true),
        ]
        .into_iter()
        .map(|(id, label, editable)| (id.into(), label.into(), editable))
        .collect(),
        EditorSection::Worlds => [
            ("name", "NAME"),
            ("x", "X"),
            ("y", "Y"),
            ("owner", "OWNER"),
            ("energy", "ENERGY"),
            ("defense", "DEFENSE"),
            ("base_output", "BASE OUTPUT"),
            ("base_regeneration", "BASE REGEN"),
            ("atmosphere", "ATMOSPHERE"),
            ("hydrosphere", "HYDROSPHERE"),
            ("topology", "TOPOLOGY"),
        ]
        .into_iter()
        .map(|(suffix, label)| {
            (
                format!("world.{selected_world}.{suffix}"),
                label.into(),
                true,
            )
        })
        .collect(),
        EditorSection::Validation => {
            if report.issues().is_empty() {
                vec![("validation.clean".into(), "VALIDATION".into(), false)]
            } else {
                report
                    .issues()
                    .iter()
                    .enumerate()
                    .map(|(index, issue)| {
                        (
                            format!("validation.{index}"),
                            format!("{:?}", issue.code).to_ascii_uppercase(),
                            false,
                        )
                    })
                    .collect()
            }
        }
    }
}

fn validation_value(id: &str, issues: &[ScenarioIssue]) -> Option<String> {
    if id == "validation.clean" {
        return Some("NO ERRORS OR WARNINGS".into());
    }
    let index = id.strip_prefix("validation.")?.parse::<usize>().ok()?;
    let issue = issues.get(index)?;
    Some(format!("{:?} | {}", issue.severity, issue.message).to_ascii_uppercase())
}

fn append_confirmation_widgets(
    widgets: &mut Vec<EditorWidget>,
    viewport: Vec2,
    confirmation: ConfirmationAction,
) {
    let width = 600.0_f32.min((viewport.x - 32.0).max(0.0));
    let height = 176.0_f32.min(viewport.y.max(0.0));
    let x = ((viewport.x - width) * 0.5).max(0.0);
    let y = ((viewport.y - height) * 0.5).max(0.0);
    widgets.push(EditorWidget {
        id: "confirmation.message".into(),
        kind: WidgetKind::Confirmation,
        rect: EditorRect::new(x, y, width, (height - ROW_HEIGHT).max(0.0)),
        label: "CONFIRM ACTION".into(),
        value: confirmation_message(confirmation).into(),
        enabled: true,
        error: None,
        focusable: false,
        visible: width > 0.0 && height > 0.0,
    });
    let button_width = width * 0.5;
    for (index, (action, label)) in [
        (EditorAction::DismissConfirmation, "KEEP EDITING"),
        (EditorAction::Confirm, "CONFIRM"),
    ]
    .into_iter()
    .enumerate()
    {
        widgets.push(EditorWidget {
            id: action_id(action).into(),
            kind: WidgetKind::Action(action),
            rect: EditorRect::new(
                x + index as f32 * button_width,
                y + height - ROW_HEIGHT,
                button_width.max(MIN_TARGET.min(width)),
                ROW_HEIGHT.min(height),
            ),
            label: label.into(),
            value: String::new(),
            enabled: true,
            error: None,
            focusable: true,
            visible: width > 0.0 && height > 0.0,
        });
    }
}

const fn confirmation_message(action: ConfirmationAction) -> &'static str {
    match action {
        ConfirmationAction::Revert => "DISCARD DRAFT CHANGES AND REVERT?",
        ConfirmationAction::FactoryDefaults => "REPLACE THE DRAFT WITH FACTORY DEFAULTS?",
        ConfirmationAction::RegenerateFromSeed => "REPLACE ALL WORLD EDITS FROM THE SEED?",
        ConfirmationAction::LoadDirty => "DISCARD DRAFT CHANGES AND LOAD THE SAVED SLOT?",
        ConfirmationAction::OverwriteSave => "REPLACE THE OCCUPIED SAVE SLOT?",
        ConfirmationAction::CancelDirty => "DISCARD DRAFT CHANGES AND CLOSE THE EDITOR?",
        ConfirmationAction::ApplyAndRestart => {
            "DISCARD THE RUNNING SESSION AND START THIS SCENARIO?"
        }
    }
}
