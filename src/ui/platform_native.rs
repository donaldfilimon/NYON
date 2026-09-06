//! AccessKit conversion and action routing for native builds.
//!
//! This adapter owns only stable semantic-node identities and the shared typed
//! action queue. `App` owns the `accesskit_winit::Adapter` because that bridge
//! must be created against the real (still invisible) window.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use accesskit::{Action, Live, Node, NodeId, Rect, Role, TreeId, TreeInfo, TreeUpdate};

use super::accessibility::{
    AnnouncementKind, SemanticActionId, SemanticNode, SemanticNodeId, SemanticRole, SemanticTree,
};

const FIRST_NODE_ID: u64 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct NativeSemanticAdapter {
    tree: Option<SemanticTree>,
    ids: BTreeMap<SemanticNodeId, NodeId>,
    action_nodes: BTreeMap<SemanticActionId, NodeId>,
    node_actions: BTreeMap<NodeId, SemanticActionId>,
    focused_action: Option<SemanticActionId>,
    actions: VecDeque<SemanticActionId>,
    next_node_id: u64,
}

impl Default for NativeSemanticAdapter {
    fn default() -> Self {
        Self {
            tree: None,
            ids: BTreeMap::new(),
            action_nodes: BTreeMap::new(),
            node_actions: BTreeMap::new(),
            focused_action: None,
            actions: VecDeque::new(),
            next_node_id: FIRST_NODE_ID,
        }
    }
}

impl NativeSemanticAdapter {
    pub fn sync(&mut self, tree: &SemanticTree, focused: Option<&SemanticActionId>) {
        self.tree = Some(tree.clone());
        self.focused_action = focused.cloned();
    }

    pub fn tree(&self) -> Option<&SemanticTree> {
        self.tree.as_ref()
    }

    pub fn request_action(&mut self, action: SemanticActionId) {
        self.actions.push_back(action);
    }

    pub fn request_node_action(&mut self, node: NodeId) -> bool {
        let Some(action) = self.node_actions.get(&node).cloned() else {
            return false;
        };
        self.request_action(action);
        true
    }

    pub fn action_for_node(&self, node: NodeId) -> Option<&SemanticActionId> {
        self.node_actions.get(&node)
    }

    pub fn action_node(&self, action: &SemanticActionId) -> Option<NodeId> {
        self.action_nodes.get(action).copied()
    }

    pub fn focus_action_for_node(&mut self, node: NodeId) -> Option<SemanticActionId> {
        let action = self.node_actions.get(&node).cloned()?;
        self.focused_action = Some(action.clone());
        Some(action)
    }

    pub fn drain_actions(&mut self) -> impl Iterator<Item = SemanticActionId> + '_ {
        self.actions.drain(..)
    }

    /// Produces a complete, internally consistent tree. AccessKit accepts full
    /// updates after initialization, and the bounded Workshop semantic model is
    /// small enough that this keeps removal semantics simple and auditable.
    pub fn full_tree_update(&mut self) -> TreeUpdate {
        let tree = self.tree.clone().unwrap_or_else(fallback_semantic_tree);
        self.action_nodes.clear();
        self.node_actions.clear();

        let mut nodes = Vec::new();
        let modal_actions = modal_action_scope(&tree.root);
        let root = self.convert_node(&tree.root, &mut nodes, modal_actions.as_ref());
        let mut announcement_ids = Vec::new();
        for (index, announcement) in tree.announcements.iter().enumerate() {
            let semantic_id = SemanticNodeId::new(format!(
                "native.announcement.{}.{}.{index}",
                match announcement.kind {
                    AnnouncementKind::Status => "status",
                    AnnouncementKind::Error => "error",
                },
                announcement.code
            ));
            let id = self.stable_id(semantic_id);
            let mut node = Node::new(match announcement.kind {
                AnnouncementKind::Status => Role::Status,
                AnnouncementKind::Error => Role::Alert,
            });
            node.set_label(announcement.message.clone());
            node.set_description(format!("NYON status code {}", announcement.code));
            node.set_live(match announcement.kind {
                AnnouncementKind::Status => Live::Polite,
                AnnouncementKind::Error => Live::Assertive,
            });
            nodes.push((id, node));
            announcement_ids.push(id);
        }
        if !announcement_ids.is_empty()
            && let Some((_, root_node)) = nodes.iter_mut().find(|(id, _)| *id == root)
        {
            let mut children = root_node.children().to_vec();
            children.extend(announcement_ids);
            root_node.set_children(children);
        }

        let focus = self
            .focused_action
            .as_ref()
            .and_then(|action| self.action_nodes.get(action).copied())
            .unwrap_or(root);
        TreeUpdate {
            nodes,
            tree: Some(TreeInfo {
                root,
                toolkit_name: Some("NYON".to_owned()),
                toolkit_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
            tree_id: TreeId::ROOT,
            focus,
        }
    }

    fn convert_node(
        &mut self,
        semantic: &SemanticNode,
        output: &mut Vec<(NodeId, Node)>,
        modal_actions: Option<&BTreeSet<SemanticActionId>>,
    ) -> NodeId {
        let id = self.stable_id(semantic.id.clone());
        let child_ids = semantic
            .children
            .iter()
            .map(|child| self.convert_node(child, output, modal_actions))
            .collect::<Vec<_>>();
        let mut node = Node::new(accesskit_role(semantic.role));
        if !semantic.name.is_empty() {
            node.set_label(semantic.name.clone());
        }
        if !semantic.description.is_empty() {
            node.set_description(semantic.description.clone());
        }
        if let Some(value) = &semantic.value {
            node.set_value(value.clone());
        }
        if let Some(bounds) = semantic.bounds {
            node.set_bounds(Rect {
                x0: f64::from(bounds.min[0]),
                y0: f64::from(bounds.min[1]),
                x1: f64::from(bounds.max[0]),
                y1: f64::from(bounds.max[1]),
            });
        }
        if !semantic.visible {
            node.set_hidden();
        }
        if !child_ids.is_empty() {
            node.set_children(child_ids);
        }
        if !semantic.enabled {
            node.set_disabled();
        }
        if semantic.selected {
            node.set_selected(true);
        }
        if let Some(level) = semantic.hierarchy_level {
            node.set_level(level.saturating_sub(1) as usize);
        }
        if let Some(action) = &semantic.action_id {
            if semantic.visible && modal_actions.is_none_or(|actions| actions.contains(action)) {
                node.add_action(Action::Click);
                node.add_action(Action::Focus);
                if matches!(
                    semantic.role,
                    SemanticRole::TextInput | SemanticRole::SpinButton
                ) {
                    node.add_action(Action::SetValue);
                }
                self.action_nodes.insert(action.clone(), id);
                self.node_actions.insert(id, action.clone());
            } else {
                node.set_disabled();
            }
        }
        if semantic.role == SemanticRole::Dialog {
            node.set_modal();
        }
        output.push((id, node));
        id
    }

    fn stable_id(&mut self, semantic: SemanticNodeId) -> NodeId {
        if let Some(id) = self.ids.get(&semantic) {
            return *id;
        }
        let id = NodeId(self.next_node_id);
        self.next_node_id = self
            .next_node_id
            .checked_add(1)
            .expect("bounded semantic node identity space is not exhaustible");
        self.ids.insert(semantic, id);
        id
    }
}

fn modal_action_scope(root: &SemanticNode) -> Option<BTreeSet<SemanticActionId>> {
    fn find_dialog(node: &SemanticNode) -> Option<&SemanticNode> {
        if node.role == SemanticRole::Dialog {
            return Some(node);
        }
        node.children.iter().find_map(find_dialog)
    }

    fn collect(node: &SemanticNode, actions: &mut BTreeSet<SemanticActionId>) {
        if let Some(action) = &node.action_id {
            actions.insert(action.clone());
        }
        for child in &node.children {
            collect(child, actions);
        }
    }

    let dialog = find_dialog(root)?;
    let mut actions = BTreeSet::new();
    collect(dialog, &mut actions);
    Some(actions)
}

fn fallback_semantic_tree() -> SemanticTree {
    SemanticTree {
        root: SemanticNode::container(
            "native.application",
            SemanticRole::Application,
            "NYON",
            Vec::new(),
        ),
        announcements: Vec::new(),
    }
}

const fn accesskit_role(role: SemanticRole) -> Role {
    match role {
        SemanticRole::Application => Role::Application,
        SemanticRole::Toolbar => Role::Toolbar,
        SemanticRole::Button => Role::Button,
        SemanticRole::Tree => Role::Tree,
        SemanticRole::TreeItem => Role::TreeItem,
        SemanticRole::Region => Role::Pane,
        SemanticRole::Group => Role::Group,
        SemanticRole::Heading => Role::Heading,
        SemanticRole::Text => Role::Paragraph,
        SemanticRole::Status => Role::Status,
        SemanticRole::Alert => Role::Alert,
        SemanticRole::Dialog => Role::Dialog,
        SemanticRole::List => Role::List,
        SemanticRole::ListItem => Role::ListItem,
        SemanticRole::Option => Role::ListBoxOption,
        SemanticRole::Checkbox => Role::CheckBox,
        SemanticRole::TextInput => Role::TextInput,
        SemanticRole::SpinButton => Role::SpinButton,
    }
}
