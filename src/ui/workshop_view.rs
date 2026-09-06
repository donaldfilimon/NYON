//! Persistent, presentation-only viewport state for Workshop navigation.

use nyon_workshop_core::EntityId;

use crate::ui::{
    accessibility::{SemanticActionId, SemanticNodeId},
    workshop::WorkshopUiModel,
    workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
};

use super::platform_inspector::wrapped_line_count;
use super::virtual_list::VisibleWindow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopDrawer {
    Creator,
    Navigator,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NavigatorSection {
    #[default]
    Hierarchy,
    Branches,
    Inspector,
    Status,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopViewAction {
    OpenCreator,
    OpenNavigator,
    CloseDrawer,
    ScrollPrevious,
    ScrollNext,
    ScrollInspectorPrevious,
    ScrollInspectorNext,
    ShowHierarchy,
    ShowBranches,
    ShowInspector,
    ShowStatus,
}

impl WorkshopViewAction {
    pub fn action_id(self) -> SemanticActionId {
        SemanticActionId::new(match self {
            Self::OpenCreator => "view.open-creator",
            Self::OpenNavigator => "view.open-navigator",
            Self::CloseDrawer => "view.close-drawer",
            Self::ScrollPrevious => "view.scroll-previous",
            Self::ScrollNext => "view.scroll-next",
            Self::ScrollInspectorPrevious => "view.inspector-scroll-previous",
            Self::ScrollInspectorNext => "view.inspector-scroll-next",
            Self::ShowHierarchy => "view.show-hierarchy",
            Self::ShowBranches => "view.show-branches",
            Self::ShowInspector => "view.show-inspector",
            Self::ShowStatus => "view.show-status",
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkshopViewState {
    pub open_drawer: Option<WorkshopDrawer>,
    pub creator_start: usize,
    pub outliner_start: usize,
    pub branch_start: usize,
    pub timeline_start: usize,
    pub summary_start: usize,
    pub inspector_start: usize,
    /// Wrapped line within `inspector_start` at the current responsive width.
    pub inspector_line_offset: usize,
    pub(crate) inspector_selection: Option<EntityId>,
    pub(crate) inspector_width_bits: u32,
    pub navigator_section: NavigatorSection,
    pub(crate) modal_row: usize,
    modal_context: Option<String>,
}

impl WorkshopViewState {
    pub(crate) fn reset_modal(&mut self) {
        self.modal_row = 0;
        self.modal_context = None;
    }

    fn reconcile_modal(&mut self, model: &WorkshopUiModel) {
        let context = modal_context(model);
        if self.modal_context != context {
            self.modal_row = 0;
            self.modal_context = context;
        }
    }

    pub(crate) fn modal_start(&self, model: &WorkshopUiModel) -> usize {
        if self.modal_context == modal_context(model) {
            self.modal_row
        } else {
            0
        }
    }

    pub fn apply(
        &mut self,
        action: WorkshopViewAction,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
    ) {
        self.reconcile_modal(model);
        if let Some(content) = super::platform_projection::modal_presentation(model, layout) {
            if action == WorkshopViewAction::ScrollNext {
                let next = self.modal_row + content.visible_rows(self.modal_row).count().max(1);
                if next < content.rows.len() {
                    self.modal_row = next;
                }
            } else if action == WorkshopViewAction::ScrollPrevious {
                self.modal_row = content.previous_page_start(self.modal_row);
            }
            return;
        }
        match action {
            WorkshopViewAction::OpenCreator => self.open_drawer = Some(WorkshopDrawer::Creator),
            WorkshopViewAction::OpenNavigator => self.open_drawer = Some(WorkshopDrawer::Navigator),
            WorkshopViewAction::CloseDrawer => self.open_drawer = None,
            WorkshopViewAction::ShowHierarchy => {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Hierarchy;
            }
            WorkshopViewAction::ShowBranches => {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Branches;
            }
            WorkshopViewAction::ShowInspector => {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Inspector;
                self.inspector_start = 0;
                self.inspector_line_offset = 0;
                self.remember_inspector_context(model, layout);
            }
            WorkshopViewAction::ShowStatus => {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Status;
                self.summary_start = 0;
            }
            WorkshopViewAction::ScrollPrevious | WorkshopViewAction::ScrollNext => {
                let delta = if action == WorkshopViewAction::ScrollNext {
                    1
                } else {
                    -1
                };
                match self.open_drawer {
                    Some(WorkshopDrawer::Creator) => {
                        let mut window = self.creator_window(model, layout);
                        window.scroll_rows(delta * 2);
                        self.creator_start = window.start;
                    }
                    Some(WorkshopDrawer::Navigator) => match self.navigator_section {
                        NavigatorSection::Hierarchy => {
                            let mut window = self.outliner_window(model, layout);
                            window.scroll_rows(delta);
                            self.outliner_start = window.start;
                        }
                        NavigatorSection::Branches => {
                            let mut window = self.branch_window(model, layout);
                            window.scroll_rows(delta);
                            self.branch_start = window.start;
                        }
                        NavigatorSection::Inspector => {
                            self.scroll_inspector(model, layout, delta);
                        }
                        NavigatorSection::Status => {
                            let content = super::platform_projection::summary_presentation(
                                &model.semantics,
                                layout,
                                self,
                            )
                            .unwrap();
                            self.summary_start = self
                                .summary_start
                                .saturating_add_signed(delta)
                                .min(content.rows.len().saturating_sub(1));
                        }
                    },
                    None if layout.right_panel.is_some() => {
                        self.scroll_inspector(model, layout, delta);
                    }
                    None => {}
                }
            }
            WorkshopViewAction::ScrollInspectorPrevious
            | WorkshopViewAction::ScrollInspectorNext => {
                let delta = if action == WorkshopViewAction::ScrollInspectorNext {
                    1
                } else {
                    -1
                };
                self.scroll_inspector(model, layout, delta);
            }
        }
    }

    pub fn reveal_action(
        &mut self,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
        action: &SemanticActionId,
    ) -> bool {
        self.reconcile_modal(model);
        if let Some(content) = super::platform_projection::modal_presentation(model, layout) {
            if let Some(index) = content
                .rows
                .iter()
                .position(|row| row.action.as_ref() == Some(action))
            {
                if !content
                    .visible_rows(self.modal_row)
                    .any(|(row, _)| row.action.as_ref() == Some(action))
                {
                    self.modal_row = index;
                }
                return true;
            }
            return matches!(
                action.as_str(),
                "creator.cancel"
                    | "creator.submit"
                    | "remove.cancel"
                    | "remove.confirm"
                    | "view.scroll-next"
                    | "view.scroll-previous"
            );
        }
        if let Some(index) = model
            .tool_palette
            .iter()
            .position(|tool| tool.control.action_id == *action)
        {
            if !wide_navigation_fits(layout) {
                self.open_drawer = Some(WorkshopDrawer::Creator);
            }
            let mut window = self.creator_window(model, layout);
            window.reveal(index);
            self.creator_start = window.start;
            return true;
        }
        if let Some(index) = model
            .outliner
            .iter()
            .position(|entry| entry.control.action_id == *action)
        {
            if !wide_navigation_fits(layout) {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
            }
            self.navigator_section = NavigatorSection::Hierarchy;
            let mut window = self.outliner_window(model, layout);
            window.reveal(index);
            self.outliner_start = window.start;
            return true;
        }
        if let Some(index) = model
            .timeline
            .controls
            .iter()
            .position(|control| control.action_id == *action)
        {
            self.timeline_start = index;
            return true;
        }
        if let Some(index) = model
            .branches
            .iter()
            .position(|branch| branch.control.action_id == *action)
        {
            if layout.mode == WorkshopLayoutMode::Compact {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Branches;
            }
            let mut window = self.branch_window(model, layout);
            window.reveal(index);
            self.branch_start = window.start;
            return true;
        }
        if model
            .inspector
            .remove_control
            .as_ref()
            .is_some_and(|control| control.action_id == *action)
            || model.preferences.reduced_motion_control.action_id == *action
            || model.preferences.high_contrast_control.action_id == *action
        {
            if layout.mode == WorkshopLayoutMode::Compact {
                self.open_drawer = Some(WorkshopDrawer::Navigator);
                self.navigator_section = NavigatorSection::Inspector;
            }
            return true;
        }
        if model.menu_control.action_id == *action || model.save.save_control.action_id == *action {
            return true;
        }
        if model.removal_confirmation.as_ref().is_some_and(|dialog| {
            dialog.cancel_control.action_id == *action
                || dialog.confirm_control.action_id == *action
        }) || model.creator_form.as_ref().is_some_and(|dialog| {
            dialog
                .fields
                .iter()
                .any(|field| field.control.action_id == *action)
                || dialog.cancel_control.action_id == *action
                || dialog.submit_control.action_id == *action
        }) {
            return true;
        }
        false
    }

    pub fn reveal_semantic(
        &mut self,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
        semantic_id: &SemanticNodeId,
    ) -> bool {
        self.reconcile_modal(model);
        if let Some(content) = super::platform_projection::modal_presentation(model, layout) {
            if let Some(index) = content.rows.iter().position(|row| row.id == *semantic_id) {
                self.modal_row = index;
                return true;
            }
            return false;
        }
        if super::platform_projection::SUMMARY_SOURCES.contains(&semantic_id.as_str()) {
            self.open_drawer = Some(WorkshopDrawer::Navigator);
            self.navigator_section = NavigatorSection::Status;
            let content =
                super::platform_projection::summary_presentation(&model.semantics, layout, self)
                    .unwrap();
            if let Some(index) = content.rows.iter().position(|row| row.id == *semantic_id) {
                self.summary_start = index;
                return true;
            }
            return false;
        }
        let Some(index) = model.inspector.text_record_index(semantic_id) else {
            return false;
        };
        if layout.mode == WorkshopLayoutMode::Compact {
            self.open_drawer = Some(WorkshopDrawer::Navigator);
            self.navigator_section = NavigatorSection::Inspector;
        }
        self.inspector_start = index;
        self.inspector_line_offset = 0;
        self.remember_inspector_context(model, layout);
        true
    }

    pub fn creator_window(
        &self,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
    ) -> VisibleWindow {
        VisibleWindow::new(
            model.tool_palette.len(),
            drawer_row_capacity(layout).saturating_mul(2),
            self.creator_start,
        )
    }

    pub fn outliner_window(
        &self,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
    ) -> VisibleWindow {
        VisibleWindow::new(
            model.outliner.len(),
            wide_outliner_projection(layout, model.tool_palette.len()).map_or_else(
                || drawer_row_capacity(layout),
                |projection| projection.capacity,
            ),
            self.outliner_start,
        )
    }

    pub fn branch_window(&self, model: &WorkshopUiModel, layout: &WorkshopLayout) -> VisibleWindow {
        VisibleWindow::new(
            model.branches.len(),
            if layout.mode == WorkshopLayoutMode::Compact
                || (self.open_drawer == Some(WorkshopDrawer::Navigator)
                    && self.navigator_section == NavigatorSection::Branches
                    && !wide_navigation_fits(layout))
            {
                drawer_row_capacity(layout)
            } else {
                docked_branch_capacity(layout)
            },
            self.branch_start,
        )
    }

    pub fn inspector_window(
        &self,
        model: &WorkshopUiModel,
        _layout: &WorkshopLayout,
    ) -> VisibleWindow {
        VisibleWindow::new(
            model.inspector.text_record_count(),
            usize::from(model.inspector.text_record_count() > 0),
            self.inspector_start,
        )
    }

    fn scroll_inspector(&mut self, model: &WorkshopUiModel, layout: &WorkshopLayout, delta: isize) {
        if !self.inspector_context_matches(model, layout) {
            self.inspector_start = 0;
            self.inspector_line_offset = 0;
        }
        self.remember_inspector_context(model, layout);
        let maximum_start = model.inspector.text_record_count().saturating_sub(1);
        self.inspector_start = self.inspector_start.min(maximum_start);
        let drawer_inspector = self.open_drawer == Some(WorkshopDrawer::Navigator)
            && self.navigator_section == NavigatorSection::Inspector;
        for _ in 0..delta.unsigned_abs() {
            if delta >= 0 {
                let line_count = model
                    .inspector
                    .text_records()
                    .get(self.inspector_start)
                    .map_or(1, |record| {
                        wrapped_line_count(record, layout, drawer_inspector)
                    });
                if self.inspector_line_offset + 1 < line_count {
                    self.inspector_line_offset += 1;
                } else if self.inspector_start < maximum_start {
                    self.inspector_start += 1;
                    self.inspector_line_offset = 0;
                }
            } else if self.inspector_line_offset > 0 {
                self.inspector_line_offset -= 1;
            } else if self.inspector_start > 0 {
                self.inspector_start -= 1;
                self.inspector_line_offset = model
                    .inspector
                    .text_records()
                    .get(self.inspector_start)
                    .map_or(0, |record| {
                        wrapped_line_count(record, layout, drawer_inspector).saturating_sub(1)
                    });
            }
        }
    }

    pub(crate) fn inspector_context_matches(
        &self,
        model: &WorkshopUiModel,
        layout: &WorkshopLayout,
    ) -> bool {
        self.inspector_selection == model.inspector.selected
            && self.inspector_width_bits == inspector_width(layout, self).to_bits()
    }

    fn remember_inspector_context(&mut self, model: &WorkshopUiModel, layout: &WorkshopLayout) {
        self.inspector_selection = model.inspector.selected;
        self.inspector_width_bits = inspector_width(layout, self).to_bits();
    }
}

fn modal_context(model: &WorkshopUiModel) -> Option<String> {
    if let Some(dialog) = &model.creator_form {
        Some(format!("creator:{:?}", dialog.tool))
    } else {
        model
            .removal_confirmation
            .as_ref()
            .map(|dialog| format!("removal:{:?}", dialog.target))
    }
}

pub(crate) fn wide_navigation_fits(layout: &WorkshopLayout) -> bool {
    layout.mode == WorkshopLayoutMode::Wide
        && layout.left_panel.is_some_and(|left| {
            left.height()
                >= 318.0 + super::workshop::CreatorTool::ALL.len().div_ceil(2) as f32 * 48.0 + 54.0
        })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WideOutlinerProjection {
    pub top: f32,
    pub row_stride: f32,
    pub capacity: usize,
}

pub(crate) fn wide_outliner_projection(
    layout: &WorkshopLayout,
    tool_count: usize,
) -> Option<WideOutlinerProjection> {
    if !wide_navigation_fits(layout) {
        return None;
    }
    let left = layout.left_panel?;
    let top = left.min.y + 318.0 + tool_count.div_ceil(2) as f32 * 48.0 + 8.0;
    let row_stride = 46.0;
    let capacity = ((left.max.y - top - 8.0) / row_stride).floor().max(0.0) as usize;
    Some(WideOutlinerProjection {
        top,
        row_stride,
        capacity,
    })
}

fn inspector_width(layout: &WorkshopLayout, view: &WorkshopViewState) -> f32 {
    let drawer = view.open_drawer == Some(WorkshopDrawer::Navigator)
        && view.navigator_section == NavigatorSection::Inspector
        && layout.mode != WorkshopLayoutMode::Wide;
    if drawer {
        layout.drawer_sheet.width()
    } else {
        layout.right_panel.map_or(0.0, |panel| panel.width())
    }
}

pub fn drawer_row_capacity(layout: &WorkshopLayout) -> usize {
    let body_height =
        (layout.drawer_sheet.height() - navigator_header_height(layout) - 56.0).max(44.0);
    (body_height / 48.0).floor().max(1.0) as usize
}

pub(crate) fn docked_branch_capacity(layout: &WorkshopLayout) -> usize {
    layout.right_panel.map_or(0, |right| {
        (((right.height() - 148.0 - 94.0 - 12.0) / 48.0).floor() as usize).clamp(1, 4)
    })
}

pub(crate) fn navigator_header_height(layout: &WorkshopLayout) -> f32 {
    if layout.drawer_sheet.width() < 480.0 {
        164.0
    } else {
        112.0
    }
}
