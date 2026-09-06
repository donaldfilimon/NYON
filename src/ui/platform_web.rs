//! DOM semantic mirror for the browser client.

use std::{
    cell::RefCell,
    collections::{BTreeSet, VecDeque},
    rc::Rc,
};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement};

use super::accessibility::{
    AnnouncementKind, SemanticActionId, SemanticNode, SemanticRole, SemanticTree,
};

pub const SEMANTIC_MIRROR_ID: &str = "nyon-semantic-mirror";

/// The keyboard position `sync` must hand back after it rebuilds the mirror.
///
/// The mirror signature covers node values, so every accepted keystroke
/// rebuilds every element. Without this the browser drops focus and the caret
/// after one character, which makes incremental keyboard editing impossible.
struct FocusRestore {
    node_id: String,
    selection: Option<(u32, u32)>,
}

pub struct DomSemanticMirror {
    document: Document,
    root: Element,
    signature: String,
    callbacks: Vec<Closure<dyn FnMut(Event)>>,
    actions: Rc<RefCell<VecDeque<SemanticActionId>>>,
    edits: Rc<RefCell<VecDeque<(SemanticActionId, String)>>>,
}

impl DomSemanticMirror {
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("document unavailable"))?;
        let root = if let Some(existing) = document.get_element_by_id(SEMANTIC_MIRROR_ID) {
            existing
        } else {
            let root = document.create_element("div")?;
            root.set_attribute("id", SEMANTIC_MIRROR_ID)?;
            root.set_attribute(
                "style",
                "position:fixed;left:-10000px;top:0;width:1px;height:1px;overflow:hidden;",
            )?;
            document
                .body()
                .ok_or_else(|| JsValue::from_str("document body unavailable"))?
                .append_child(&root)?;
            root
        };
        Ok(Self {
            document,
            root,
            signature: String::new(),
            callbacks: Vec::new(),
            actions: Rc::new(RefCell::new(VecDeque::new())),
            edits: Rc::new(RefCell::new(VecDeque::new())),
        })
    }

    pub fn sync(&mut self, tree: &SemanticTree) -> Result<(), JsValue> {
        let signature = format!("{tree:?}");
        if self.signature == signature {
            return Ok(());
        }
        tree.validate()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let restore = self.capture_focus();
        self.root.set_text_content(None);
        self.callbacks.clear();
        let modal_actions = modal_action_scope(&tree.root);
        append_node(
            &self.document,
            &self.root,
            &tree.root,
            &self.actions,
            &self.edits,
            &mut self.callbacks,
            modal_actions.as_ref(),
        )?;
        for (index, announcement) in tree.announcements.iter().enumerate() {
            let element = self.document.create_element("div")?;
            element.set_attribute("id", &format!("nyon-announcement-{index}"))?;
            element.set_attribute(
                "role",
                match announcement.kind {
                    AnnouncementKind::Status => "status",
                    AnnouncementKind::Error => "alert",
                },
            )?;
            element.set_attribute(
                "aria-live",
                match announcement.kind {
                    AnnouncementKind::Status => "polite",
                    AnnouncementKind::Error => "assertive",
                },
            )?;
            element.set_attribute("data-nyon-code", announcement.code)?;
            element.set_text_content(Some(&announcement.message));
            self.root.append_child(&element)?;
        }
        self.restore_focus(restore.as_ref());
        self.signature = signature;
        Ok(())
    }

    /// Records the focused mirror element and its caret before teardown.
    ///
    /// Selection offsets are read only from real inputs; every other focused
    /// control restores position alone.
    fn capture_focus(&self) -> Option<FocusRestore> {
        let active = self.document.active_element()?;
        if !self.root.contains(Some(&active)) {
            return None;
        }
        let node_id = active.id();
        if node_id.is_empty() {
            return None;
        }
        let selection = active.dyn_ref::<HtmlInputElement>().and_then(|input| {
            let start = input.selection_start().ok().flatten()?;
            let end = input.selection_end().ok().flatten()?;
            Some((start, end))
        });
        Some(FocusRestore { node_id, selection })
    }

    /// Returns focus to the rebuilt element carrying the same stable node id.
    ///
    /// This deliberately never selects the whole value. Select-all belongs to
    /// entry (the click handler), not to a rebuild the user did not ask for.
    fn restore_focus(&self, restore: Option<&FocusRestore>) {
        let Some(restore) = restore else {
            return;
        };
        let Some(element) = self.document.get_element_by_id(&restore.node_id) else {
            return;
        };
        if let Some(html) = element.dyn_ref::<HtmlElement>() {
            let _ = html.focus();
        }
        if let (Some((start, end)), Some(input)) =
            (restore.selection, element.dyn_ref::<HtmlInputElement>())
        {
            // The rebuilt element carries the authoritative value, which can be
            // shorter than the caret we saved when a draft edit was rejected.
            let length = u32::try_from(input.value().chars().count()).unwrap_or(u32::MAX);
            let _ = input.set_selection_range(start.min(length), end.min(length));
        }
    }

    pub fn drain_actions(&mut self) -> impl Iterator<Item = SemanticActionId> + '_ {
        self.actions
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn drain_edits(&mut self) -> impl Iterator<Item = (SemanticActionId, String)> + '_ {
        self.edits
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>()
            .into_iter()
    }
}

fn append_node(
    document: &Document,
    parent: &Element,
    node: &SemanticNode,
    actions: &Rc<RefCell<VecDeque<SemanticActionId>>>,
    edits: &Rc<RefCell<VecDeque<(SemanticActionId, String)>>>,
    callbacks: &mut Vec<Closure<dyn FnMut(Event)>>,
    modal_actions: Option<&BTreeSet<SemanticActionId>>,
) -> Result<(), JsValue> {
    let tag = if matches!(
        node.role,
        SemanticRole::TextInput | SemanticRole::SpinButton
    ) {
        "input"
    } else if node.action_id.is_some() {
        "button"
    } else if matches!(node.role, SemanticRole::TreeItem | SemanticRole::ListItem) {
        "li"
    } else {
        "div"
    };
    let element = document.create_element(tag)?;
    let modal_allowed = node
        .action_id
        .as_ref()
        .is_none_or(|action| modal_actions.is_none_or(|actions| actions.contains(action)));
    let interactable = node.enabled && modal_allowed && node.visible;
    element.set_attribute("id", node.id.as_str())?;
    element.set_attribute("role", semantic_role(node.role))?;
    element.set_attribute("aria-label", &node.name)?;
    if let Some(bounds) = node.bounds {
        element.set_attribute(
            "data-nyon-bounds",
            &format!(
                "{},{},{},{}",
                bounds.min[0], bounds.min[1], bounds.max[0], bounds.max[1]
            ),
        )?;
    }
    if !node.description.is_empty() {
        element.set_attribute("title", &node.description)?;
    }
    if let Some(value) = &node.value {
        element.set_attribute("value", value)?;
        element.set_attribute("aria-valuetext", value)?;
        if node.role == SemanticRole::SpinButton {
            if value.parse::<i128>().is_ok() {
                element.set_attribute("aria-valuenow", value)?;
            }
            element.set_attribute("inputmode", "numeric")?;
        }
    }
    element.set_attribute("aria-disabled", if interactable { "false" } else { "true" })?;
    if node.role == SemanticRole::Dialog {
        element.set_attribute("aria-modal", "true")?;
    }
    if node.selected {
        element.set_attribute("aria-selected", "true")?;
        if node.role == SemanticRole::Checkbox {
            element.set_attribute("aria-checked", "true")?;
        }
    } else if node.role == SemanticRole::Checkbox {
        element.set_attribute("aria-checked", "false")?;
    }
    if let Some(level) = node.hierarchy_level {
        element.set_attribute("aria-level", &level.to_string())?;
    }
    if !node.visible {
        element.set_attribute("aria-hidden", "true")?;
        element.set_attribute("tabindex", "-1")?;
    }
    if let Some(action_id) = &node.action_id {
        element.set_attribute("data-nyon-action", action_id.as_str())?;
        if !modal_allowed || !node.visible {
            element.set_attribute("tabindex", "-1")?;
            element.set_attribute("aria-hidden", "true")?;
        }
        if !matches!(
            node.role,
            SemanticRole::TextInput | SemanticRole::SpinButton
        ) {
            element.set_text_content(Some(&node.name));
        }
        if !interactable {
            element.set_attribute("disabled", "")?;
        } else {
            let action_id = action_id.clone();
            let actions = Rc::clone(actions);
            let preserve_native_editing = matches!(
                node.role,
                SemanticRole::TextInput | SemanticRole::SpinButton
            );
            let callback = Closure::wrap(Box::new(move |event: Event| {
                if preserve_native_editing {
                    if let Some(input) = event
                        .target()
                        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                    {
                        input.select();
                    }
                } else {
                    event.prevent_default();
                }
                actions.borrow_mut().push_back(action_id.clone());
            }) as Box<dyn FnMut(_)>);
            element.add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())?;
            callbacks.push(callback);
            if matches!(
                node.role,
                SemanticRole::TextInput | SemanticRole::SpinButton
            ) {
                let action_id = node.action_id.clone().expect("editable nodes have actions");
                let edits = Rc::clone(edits);
                let callback = Closure::wrap(Box::new(move |event: Event| {
                    let Some(input) = event
                        .target()
                        .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                    else {
                        return;
                    };
                    // The rebuilt element is seeded from authoritative state and
                    // `sync` restores the caret, so the live value is the truth.
                    edits
                        .borrow_mut()
                        .push_back((action_id.clone(), input.value()));
                }) as Box<dyn FnMut(_)>);
                element
                    .add_event_listener_with_callback("input", callback.as_ref().unchecked_ref())?;
                callbacks.push(callback);
            }
        }
    } else if !node.name.is_empty() {
        let heading = document.create_element("span")?;
        heading.set_text_content(Some(&node.name));
        element.append_child(&heading)?;
    }
    for child in &node.children {
        append_node(
            document,
            &element,
            child,
            actions,
            edits,
            callbacks,
            modal_actions,
        )?;
    }
    parent.append_child(&element)?;
    Ok(())
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

const fn semantic_role(role: SemanticRole) -> &'static str {
    match role {
        SemanticRole::Application => "application",
        SemanticRole::Toolbar => "toolbar",
        SemanticRole::Button => "button",
        SemanticRole::Tree => "tree",
        SemanticRole::TreeItem => "treeitem",
        SemanticRole::Region => "region",
        SemanticRole::Group => "group",
        SemanticRole::Heading => "heading",
        SemanticRole::Text => "note",
        SemanticRole::Status => "status",
        SemanticRole::Alert => "alert",
        SemanticRole::Dialog => "dialog",
        SemanticRole::List => "list",
        SemanticRole::ListItem => "listitem",
        SemanticRole::Option => "option",
        SemanticRole::Checkbox => "checkbox",
        SemanticRole::TextInput => "textbox",
        SemanticRole::SpinButton => "spinbutton",
    }
}
