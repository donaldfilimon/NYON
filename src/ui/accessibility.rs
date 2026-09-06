//! Platform-neutral accessibility primitives for NYON user interfaces.
//!
//! The renderer, DOM adapter, and native accessibility adapter all consume the
//! same immutable semantic tree. Focus state is deliberately kept outside the
//! authoritative Workshop state.

use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticNodeId(String);

impl SemanticNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SemanticNodeId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticActionId(String);

impl SemanticActionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SemanticActionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SemanticRole {
    Application,
    Toolbar,
    Button,
    Tree,
    TreeItem,
    Region,
    Group,
    Heading,
    Text,
    Status,
    Alert,
    Dialog,
    List,
    ListItem,
    Option,
    Checkbox,
    TextInput,
    SpinButton,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    pub id: SemanticNodeId,
    pub role: SemanticRole,
    pub name: String,
    pub description: String,
    pub value: Option<String>,
    pub enabled: bool,
    pub selected: bool,
    pub focusable: bool,
    pub hierarchy_level: Option<u32>,
    pub action_id: Option<SemanticActionId>,
    pub bounds: Option<SemanticRect>,
    pub visible: bool,
    pub children: Vec<SemanticNode>,
}

impl SemanticNode {
    pub fn container(
        id: impl Into<String>,
        role: SemanticRole,
        name: impl Into<String>,
        children: Vec<Self>,
    ) -> Self {
        Self {
            id: SemanticNodeId::new(id),
            role,
            name: name.into(),
            description: String::new(),
            value: None,
            enabled: true,
            selected: false,
            focusable: false,
            hierarchy_level: None,
            action_id: None,
            bounds: None,
            visible: true,
            children,
        }
    }

    pub fn text(
        id: impl Into<String>,
        role: SemanticRole,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: SemanticNodeId::new(id),
            role,
            name: name.into(),
            description: description.into(),
            value: None,
            enabled: true,
            selected: false,
            focusable: false,
            hierarchy_level: None,
            action_id: None,
            bounds: None,
            visible: true,
            children: Vec::new(),
        }
    }

    pub fn control(
        id: impl Into<String>,
        role: SemanticRole,
        name: impl Into<String>,
        description: impl Into<String>,
        enabled: bool,
        selected: bool,
        action_id: SemanticActionId,
    ) -> Self {
        Self {
            id: SemanticNodeId::new(id),
            role,
            name: name.into(),
            description: description.into(),
            value: None,
            enabled,
            selected,
            focusable: true,
            hierarchy_level: None,
            action_id: Some(action_id),
            bounds: None,
            visible: true,
            children: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnnouncementKind {
    Status,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticAnnouncement {
    pub kind: AnnouncementKind,
    pub code: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticTree {
    pub root: SemanticNode,
    pub announcements: Vec<SemanticAnnouncement>,
}

impl SemanticTree {
    pub fn nodes_depth_first(&self) -> Vec<&SemanticNode> {
        fn visit<'a>(node: &'a SemanticNode, output: &mut Vec<&'a SemanticNode>) {
            output.push(node);
            for child in &node.children {
                visit(child, output);
            }
        }

        let mut nodes = Vec::new();
        visit(&self.root, &mut nodes);
        nodes
    }

    pub fn node(&self, id: &str) -> Option<&SemanticNode> {
        self.nodes_depth_first()
            .into_iter()
            .find(|node| node.id.as_str() == id)
    }

    pub fn validate(&self) -> Result<(), SemanticTreeError> {
        if self.root.role != SemanticRole::Application {
            return Err(SemanticTreeError::RootIsNotApplication);
        }
        let mut node_ids = BTreeSet::new();
        let mut action_ids = BTreeSet::new();
        for node in self.nodes_depth_first() {
            if !node_ids.insert(node.id.clone()) {
                return Err(SemanticTreeError::DuplicateNodeId(node.id.clone()));
            }
            if let Some(action) = &node.action_id {
                if !node.focusable {
                    return Err(SemanticTreeError::ActionIsNotFocusable(node.id.clone()));
                }
                if !action_ids.insert(action.clone()) {
                    return Err(SemanticTreeError::DuplicateActionId(action.clone()));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SemanticTreeError {
    #[error("the semantic tree root must have the application role")]
    RootIsNotApplication,
    #[error("duplicate semantic node identifier {0}")]
    DuplicateNodeId(SemanticNodeId),
    #[error("duplicate semantic action identifier {0}")]
    DuplicateActionId(SemanticActionId),
    #[error("semantic node {0} has an action but is not focusable")]
    ActionIsNotFocusable(SemanticNodeId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputModality {
    Pointer,
    Keyboard,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModalFocusScope {
    order: Vec<SemanticActionId>,
    restore: Option<SemanticActionId>,
}

/// Stable, cyclic focus order with a single modal trap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FocusManager {
    base_order: Vec<SemanticActionId>,
    focused: Option<SemanticActionId>,
    modal: Option<ModalFocusScope>,
}

impl FocusManager {
    pub fn new(order: impl IntoIterator<Item = SemanticActionId>) -> Self {
        let base_order = stable_unique(order);
        let focused = base_order.first().cloned();
        Self {
            base_order,
            focused,
            modal: None,
        }
    }

    pub fn focused(&self) -> Option<&SemanticActionId> {
        self.focused.as_ref()
    }

    pub fn base_order(&self) -> &[SemanticActionId] {
        &self.base_order
    }

    pub fn active_order(&self) -> &[SemanticActionId] {
        self.modal
            .as_ref()
            .map_or(&self.base_order, |scope| &scope.order)
    }

    pub fn replace_base_order(&mut self, order: impl IntoIterator<Item = SemanticActionId>) {
        self.base_order = stable_unique(order);
        if self.modal.is_none()
            && self
                .focused
                .as_ref()
                .is_none_or(|focused| !self.base_order.contains(focused))
        {
            self.focused = self.base_order.first().cloned();
        }
    }

    pub const fn modal_is_open(&self) -> bool {
        self.modal.is_some()
    }

    /// Synchronize the presented scope without replacing a modal's restore target.
    pub fn replace_active_order(
        &mut self,
        order: impl IntoIterator<Item = SemanticActionId>,
    ) -> Result<(), FocusError> {
        let order = stable_unique(order);
        if let Some(scope) = &mut self.modal {
            if order.is_empty() {
                return Err(FocusError::EmptyModal);
            }
            if self
                .focused
                .as_ref()
                .is_none_or(|focused| !order.contains(focused))
            {
                self.focused = order.first().cloned();
            }
            scope.order = order;
        } else {
            self.replace_base_order(order);
        }
        Ok(())
    }

    pub fn request_focus(&mut self, action: &SemanticActionId) -> bool {
        if self.active_order().contains(action) {
            self.focused = Some(action.clone());
            true
        } else {
            false
        }
    }

    pub fn move_next(&mut self) -> Option<&SemanticActionId> {
        self.move_by(1)
    }

    pub fn move_previous(&mut self) -> Option<&SemanticActionId> {
        self.move_by(-1)
    }

    pub fn open_modal(
        &mut self,
        order: impl IntoIterator<Item = SemanticActionId>,
    ) -> Result<&SemanticActionId, FocusError> {
        if self.modal.is_some() {
            return Err(FocusError::ModalAlreadyOpen);
        }
        let order = stable_unique(order);
        let first = order.first().cloned().ok_or(FocusError::EmptyModal)?;
        self.modal = Some(ModalFocusScope {
            order,
            restore: self.focused.clone(),
        });
        self.focused = Some(first);
        Ok(self
            .focused
            .as_ref()
            .expect("modal focus was just installed"))
    }

    pub fn close_modal(&mut self) -> Option<&SemanticActionId> {
        let scope = self.modal.take()?;
        self.focused = scope
            .restore
            .filter(|action| self.base_order.contains(action))
            .or_else(|| self.base_order.first().cloned());
        self.focused.as_ref()
    }

    fn move_by(&mut self, direction: isize) -> Option<&SemanticActionId> {
        let order = self.active_order();
        if order.is_empty() {
            self.focused = None;
            return None;
        }
        let current = self
            .focused
            .as_ref()
            .and_then(|focused| order.iter().position(|action| action == focused))
            .unwrap_or(0);
        let next = if direction >= 0 {
            (current + 1) % order.len()
        } else if current == 0 {
            order.len() - 1
        } else {
            current - 1
        };
        let action = order[next].clone();
        self.focused = Some(action);
        self.focused.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FocusError {
    #[error("a modal focus scope is already open")]
    ModalAlreadyOpen,
    #[error("a modal focus scope must contain an actionable control")]
    EmptyModal,
}

fn stable_unique(items: impl IntoIterator<Item = SemanticActionId>) -> Vec<SemanticActionId> {
    let mut seen = BTreeSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(item.clone()))
        .collect()
}
