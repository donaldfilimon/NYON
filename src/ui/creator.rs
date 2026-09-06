//! Client-only editable drafts for recorded Workshop creator operations.
//!
//! Draft strings deliberately live outside `nyon-workshop-core`: incomplete
//! text is presentation state, while `to_batch` is the only boundary that
//! constructs a typed, authority-ready command.

use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, EntityId, GalaxyPointV1, ObjectName,
    ObjectRefV1, WorkshopStateV1, WorkshopTick, model::EntityKind,
};

const MAX_DRAFT_VALUE_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CreatorTool {
    CreateFaction,
    CreateSystem,
    CreateStar,
    CreateWorld,
    ConnectLane,
    CreateDeposit,
    SetOwner,
    PlaceIndustry,
    ConnectRoute,
    SetIndustryEnabled,
    RenameObject,
    RemoveObject,
    ScheduleHazard,
    CancelHazard,
}

impl CreatorTool {
    pub const ALL: [Self; 14] = [
        Self::CreateFaction,
        Self::CreateSystem,
        Self::CreateStar,
        Self::CreateWorld,
        Self::ConnectLane,
        Self::CreateDeposit,
        Self::SetOwner,
        Self::PlaceIndustry,
        Self::ConnectRoute,
        Self::SetIndustryEnabled,
        Self::RenameObject,
        Self::RemoveObject,
        Self::ScheduleHazard,
        Self::CancelHazard,
    ];

    pub const fn for_operation(operation: &CreatorOpV1) -> Self {
        match operation {
            CreatorOpV1::CreateFaction { .. } => Self::CreateFaction,
            CreatorOpV1::CreateSystem { .. } => Self::CreateSystem,
            CreatorOpV1::CreateStar { .. } => Self::CreateStar,
            CreatorOpV1::CreateWorld { .. } => Self::CreateWorld,
            CreatorOpV1::ConnectLane { .. } => Self::ConnectLane,
            CreatorOpV1::CreateDeposit { .. } => Self::CreateDeposit,
            CreatorOpV1::SetOwner { .. } => Self::SetOwner,
            CreatorOpV1::PlaceIndustry { .. } => Self::PlaceIndustry,
            CreatorOpV1::ConnectRoute { .. } => Self::ConnectRoute,
            CreatorOpV1::SetIndustryEnabled { .. } => Self::SetIndustryEnabled,
            CreatorOpV1::RenameObject { .. } => Self::RenameObject,
            CreatorOpV1::RemoveObject { .. } => Self::RemoveObject,
            CreatorOpV1::ScheduleHazard { .. } => Self::ScheduleHazard,
            CreatorOpV1::CancelHazard { .. } => Self::CancelHazard,
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::CreateFaction => "create-faction",
            Self::CreateSystem => "create-system",
            Self::CreateStar => "create-star",
            Self::CreateWorld => "create-world",
            Self::ConnectLane => "connect-lane",
            Self::CreateDeposit => "create-deposit",
            Self::SetOwner => "set-owner",
            Self::PlaceIndustry => "place-industry",
            Self::ConnectRoute => "connect-route",
            Self::SetIndustryEnabled => "set-industry-enabled",
            Self::RenameObject => "rename-object",
            Self::RemoveObject => "remove-object",
            Self::ScheduleHazard => "schedule-hazard",
            Self::CancelHazard => "cancel-hazard",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::CreateFaction => "Create faction",
            Self::CreateSystem => "Create system",
            Self::CreateStar => "Create star",
            Self::CreateWorld => "Create world",
            Self::ConnectLane => "Connect lane",
            Self::CreateDeposit => "Create deposit",
            Self::SetOwner => "Set owner",
            Self::PlaceIndustry => "Place industry",
            Self::ConnectRoute => "Connect route",
            Self::SetIndustryEnabled => "Enable or disable industry",
            Self::RenameObject => "Rename object",
            Self::RemoveObject => "Remove object",
            Self::ScheduleHazard => "Schedule hazard",
            Self::CancelHazard => "Cancel hazard",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::CreateFaction => "Add a visual ownership faction.",
            Self::CreateSystem => "Add a named system at integer galaxy coordinates.",
            Self::CreateStar => "Add a catalog star to a system.",
            Self::CreateWorld => "Add an orbiting catalog world to a system.",
            Self::ConnectLane => "Connect two systems with a direct logistics lane.",
            Self::CreateDeposit => "Add a finite resource reserve to a world.",
            Self::SetOwner => "Assign or clear a visual ownership faction.",
            Self::PlaceIndustry => "Place a catalog industry on a world.",
            Self::ConnectRoute => "Dispatch a resource between two worlds.",
            Self::SetIndustryEnabled => "Change whether an industry runs.",
            Self::RenameObject => "Change the printable name of an object.",
            Self::RemoveObject => "Review dependencies, then remove one object.",
            Self::ScheduleHazard => "Schedule a deterministic lane hazard.",
            Self::CancelHazard => "Cancel a scheduled or active hazard.",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorFieldKind {
    Text,
    Integer,
    Choice,
}

pub(crate) fn edited_creator_text(
    kind: CreatorFieldKind,
    current: &str,
    text: Option<&str>,
    backspace: bool,
    replace: bool,
) -> Option<String> {
    if kind == CreatorFieldKind::Choice {
        return None;
    }
    if backspace {
        let mut value = if replace {
            String::new()
        } else {
            current.to_owned()
        };
        if !replace {
            value.pop();
        }
        return Some(value);
    }
    let text = text?;
    let acceptable = !text.is_empty()
        && text.is_ascii()
        && !text.chars().any(char::is_control)
        && (kind == CreatorFieldKind::Text
            || text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || byte == b'-'));
    acceptable.then(|| {
        if replace {
            text.to_owned()
        } else {
            let mut value = current.to_owned();
            value.push_str(text);
            value
        }
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorChoice {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorDraftField {
    pub id: &'static str,
    pub label: &'static str,
    pub value: String,
    pub kind: CreatorFieldKind,
    pub choices: Vec<CreatorChoice>,
}

impl CreatorDraftField {
    pub fn display_value(&self) -> String {
        self.choices
            .iter()
            .find(|choice| choice.value == self.value)
            .map_or_else(|| self.value.clone(), |choice| choice.label.clone())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorDraft {
    pub tool: CreatorTool,
    pub expected_cursor: Option<nyon_workshop_core::RevisionId>,
    pub expected_tick: WorkshopTick,
    pub fields: Vec<CreatorDraftField>,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CreatorDraftError {
    #[error("creator draft has no field named {0}")]
    UnknownField(String),
    #[error("{0} is not valid: {1}")]
    InvalidField(&'static str, &'static str),
    #[error("{0} is not one of the available choices")]
    InvalidChoice(&'static str),
    #[error("creator draft does not match its selected tool")]
    ToolMismatch,
}

impl CreatorDraft {
    pub fn from_batch(
        state: &WorkshopStateV1,
        batch: &CreatorBatchV1,
    ) -> Result<Self, CreatorDraftError> {
        let operation = batch.operations.as_slice();
        let [operation] = operation else {
            return Err(CreatorDraftError::ToolMismatch);
        };
        let tool = CreatorTool::for_operation(operation);
        let mut fields = Vec::new();
        match operation {
            CreatorOpV1::CreateFaction {
                name, color_rgb, ..
            } => {
                fields.push(text("name", "Name", name.as_str()));
                fields.push(integer("red", "Red", color_rgb[0]));
                fields.push(integer("green", "Green", color_rgb[1]));
                fields.push(integer("blue", "Blue", color_rgb[2]));
            }
            CreatorOpV1::CreateSystem { name, position, .. } => {
                fields.push(text("name", "Name", name.as_str()));
                fields.push(integer("x", "Galaxy X", position.x.get()));
                fields.push(integer("y", "Galaxy Y", position.y.get()));
            }
            CreatorOpV1::CreateStar {
                system,
                name,
                archetype_id,
                ..
            } => {
                fields.push(entity_choice(
                    "system",
                    "System",
                    state,
                    EntityKind::System,
                    Some(*system),
                    false,
                ));
                fields.push(text("name", "Name", name.as_str()));
                fields.push(catalog_choice(
                    "archetype",
                    "Star archetype",
                    archetype_id,
                    &["yellow-dwarf"],
                ));
            }
            CreatorOpV1::CreateWorld {
                system,
                primary,
                name,
                archetype_id,
                orbit_radius_milli_au,
                orbit_period_ticks,
                phase_millidegrees,
                ..
            } => {
                fields.push(entity_choice(
                    "system",
                    "System",
                    state,
                    EntityKind::System,
                    Some(*system),
                    false,
                ));
                fields.push(entity_choice(
                    "primary",
                    "Primary star",
                    state,
                    EntityKind::Star,
                    Some(*primary),
                    false,
                ));
                fields.push(text("name", "Name", name.as_str()));
                fields.push(catalog_choice(
                    "archetype",
                    "World archetype",
                    archetype_id,
                    &["rocky-world"],
                ));
                fields.push(integer(
                    "orbit-radius",
                    "Orbit radius (milli-AU)",
                    *orbit_radius_milli_au,
                ));
                fields.push(integer(
                    "orbit-period",
                    "Orbit period (ticks)",
                    *orbit_period_ticks,
                ));
                fields.push(integer(
                    "phase",
                    "Phase (millidegrees)",
                    *phase_millidegrees,
                ));
            }
            CreatorOpV1::ConnectLane { a, b, .. } => {
                fields.push(entity_choice(
                    "a",
                    "System A",
                    state,
                    EntityKind::System,
                    Some(*a),
                    false,
                ));
                fields.push(entity_choice(
                    "b",
                    "System B",
                    state,
                    EntityKind::System,
                    Some(*b),
                    false,
                ));
            }
            CreatorOpV1::CreateDeposit {
                world,
                resource_id,
                reserve_units,
                ..
            } => {
                fields.push(entity_choice(
                    "world",
                    "World",
                    state,
                    EntityKind::World,
                    Some(*world),
                    false,
                ));
                fields.push(catalog_choice(
                    "resource",
                    "Resource",
                    resource_id,
                    &["energy", "ore", "alloy"],
                ));
                fields.push(integer("reserve", "Reserve units", *reserve_units));
            }
            CreatorOpV1::SetOwner { target, faction } => {
                fields.push(filtered_entity_choice(
                    "target",
                    "Object",
                    state,
                    ownable,
                    Some(*target),
                    false,
                ));
                fields.push(entity_choice(
                    "faction",
                    "Faction",
                    state,
                    EntityKind::Faction,
                    *faction,
                    true,
                ));
            }
            CreatorOpV1::PlaceIndustry {
                world,
                definition_id,
                linked_deposit,
                ..
            } => {
                fields.push(entity_choice(
                    "world",
                    "World",
                    state,
                    EntityKind::World,
                    Some(*world),
                    false,
                ));
                fields.push(catalog_choice(
                    "definition",
                    "Industry",
                    definition_id,
                    &["solar-array", "extractor", "foundry"],
                ));
                fields.push(entity_choice(
                    "deposit",
                    "Linked deposit",
                    state,
                    EntityKind::Deposit,
                    *linked_deposit,
                    true,
                ));
            }
            CreatorOpV1::ConnectRoute {
                source,
                destination,
                resource_id,
                batch_units,
                ..
            } => {
                fields.push(entity_choice(
                    "source",
                    "Source world",
                    state,
                    EntityKind::World,
                    Some(*source),
                    false,
                ));
                fields.push(entity_choice(
                    "destination",
                    "Destination world",
                    state,
                    EntityKind::World,
                    Some(*destination),
                    false,
                ));
                fields.push(catalog_choice(
                    "resource",
                    "Resource",
                    resource_id,
                    &["energy", "ore", "alloy"],
                ));
                fields.push(integer("batch", "Batch units", *batch_units));
            }
            CreatorOpV1::SetIndustryEnabled { industry, enabled } => {
                fields.push(entity_choice(
                    "industry",
                    "Industry",
                    state,
                    EntityKind::Industry,
                    Some(*industry),
                    false,
                ));
                fields.push(bool_choice("enabled", "Enabled", *enabled));
            }
            CreatorOpV1::RenameObject { target, name } => {
                fields.push(filtered_entity_choice(
                    "target",
                    "Object",
                    state,
                    named,
                    Some(*target),
                    false,
                ));
                fields.push(text("name", "Name", name.as_str()));
            }
            CreatorOpV1::RemoveObject { target } => {
                fields.push(filtered_entity_choice(
                    "target",
                    "Object",
                    state,
                    removable,
                    Some(*target),
                    false,
                ));
            }
            CreatorOpV1::ScheduleHazard {
                lane,
                hazard_id,
                start_tick,
                duration_ticks,
                ..
            } => {
                fields.push(entity_choice(
                    "lane",
                    "Lane",
                    state,
                    EntityKind::Lane,
                    Some(*lane),
                    false,
                ));
                fields.push(catalog_choice(
                    "hazard",
                    "Hazard",
                    hazard_id,
                    &["ion-storm"],
                ));
                fields.push(integer("start", "Start tick", start_tick.0));
                fields.push(integer("duration", "Duration (ticks)", *duration_ticks));
            }
            CreatorOpV1::CancelHazard { hazard } => {
                fields.push(entity_choice(
                    "hazard",
                    "Hazard",
                    state,
                    EntityKind::Hazard,
                    Some(*hazard),
                    false,
                ));
            }
        }
        Ok(Self {
            tool,
            expected_cursor: batch.expected_cursor,
            expected_tick: batch.expected_tick,
            fields,
        })
    }

    pub fn field(&self, id: &str) -> Option<&CreatorDraftField> {
        self.fields.iter().find(|field| field.id == id)
    }

    pub fn set_text(
        &mut self,
        id: &str,
        value: impl Into<String>,
    ) -> Result<(), CreatorDraftError> {
        let value = value.into();
        if value.len() > MAX_DRAFT_VALUE_BYTES || !value.is_ascii() {
            return Err(CreatorDraftError::InvalidField(
                "field",
                "use at most 128 ASCII bytes",
            ));
        }
        let field = self
            .fields
            .iter_mut()
            .find(|field| field.id == id)
            .ok_or_else(|| CreatorDraftError::UnknownField(id.to_owned()))?;
        if field.kind == CreatorFieldKind::Choice {
            return Err(CreatorDraftError::InvalidField(
                field.label,
                "activate it to choose a value",
            ));
        }
        field.value = value;
        Ok(())
    }

    pub fn cycle(&mut self, id: &str, direction: i8) -> Result<(), CreatorDraftError> {
        let field = self
            .fields
            .iter_mut()
            .find(|field| field.id == id)
            .ok_or_else(|| CreatorDraftError::UnknownField(id.to_owned()))?;
        if field.kind != CreatorFieldKind::Choice || field.choices.is_empty() {
            return Err(CreatorDraftError::InvalidField(
                field.label,
                "is not a choice",
            ));
        }
        let current = field
            .choices
            .iter()
            .position(|choice| choice.value == field.value)
            .unwrap_or(0);
        let len = field.choices.len();
        let next = if direction < 0 {
            current.checked_sub(1).unwrap_or(len - 1)
        } else {
            (current + 1) % len
        };
        field.value.clone_from(&field.choices[next].value);
        Ok(())
    }

    pub fn to_batch(&self) -> Result<CreatorBatchV1, CreatorDraftError> {
        let local = BatchLocalId(1);
        let operation = match self.tool {
            CreatorTool::CreateFaction => CreatorOpV1::CreateFaction {
                local,
                name: self.name("name")?,
                color_rgb: [
                    self.number("red")?,
                    self.number("green")?,
                    self.number("blue")?,
                ],
            },
            CreatorTool::CreateSystem => CreatorOpV1::CreateSystem {
                local,
                name: self.name("name")?,
                position: GalaxyPointV1::new(self.number("x")?, self.number("y")?).map_err(
                    |_| {
                        CreatorDraftError::InvalidField(
                            "Galaxy position",
                            "coordinates are out of range",
                        )
                    },
                )?,
            },
            CreatorTool::CreateStar => CreatorOpV1::CreateStar {
                local,
                system: self.entity("system")?,
                name: self.name("name")?,
                archetype_id: self.catalog("archetype")?,
            },
            CreatorTool::CreateWorld => CreatorOpV1::CreateWorld {
                local,
                system: self.entity("system")?,
                primary: self.entity("primary")?,
                name: self.name("name")?,
                archetype_id: self.catalog("archetype")?,
                orbit_radius_milli_au: self.number("orbit-radius")?,
                orbit_period_ticks: self.number("orbit-period")?,
                phase_millidegrees: self.number("phase")?,
            },
            CreatorTool::ConnectLane => CreatorOpV1::ConnectLane {
                local,
                a: self.entity("a")?,
                b: self.entity("b")?,
            },
            CreatorTool::CreateDeposit => CreatorOpV1::CreateDeposit {
                local,
                world: self.entity("world")?,
                resource_id: self.catalog("resource")?,
                reserve_units: self.number("reserve")?,
            },
            CreatorTool::SetOwner => CreatorOpV1::SetOwner {
                target: self.entity("target")?,
                faction: self.optional_entity("faction")?,
            },
            CreatorTool::PlaceIndustry => CreatorOpV1::PlaceIndustry {
                local,
                world: self.entity("world")?,
                definition_id: self.catalog("definition")?,
                linked_deposit: self.optional_entity("deposit")?,
            },
            CreatorTool::ConnectRoute => CreatorOpV1::ConnectRoute {
                local,
                source: self.entity("source")?,
                destination: self.entity("destination")?,
                resource_id: self.catalog("resource")?,
                batch_units: self.number("batch")?,
            },
            CreatorTool::SetIndustryEnabled => CreatorOpV1::SetIndustryEnabled {
                industry: self.entity("industry")?,
                enabled: self.value("enabled")? == "true",
            },
            CreatorTool::RenameObject => CreatorOpV1::RenameObject {
                target: self.entity("target")?,
                name: self.name("name")?,
            },
            CreatorTool::RemoveObject => CreatorOpV1::RemoveObject {
                target: self.entity("target")?,
            },
            CreatorTool::ScheduleHazard => CreatorOpV1::ScheduleHazard {
                local,
                lane: self.entity("lane")?,
                hazard_id: self.catalog("hazard")?,
                start_tick: WorkshopTick(self.number("start")?),
                duration_ticks: self.number("duration")?,
            },
            CreatorTool::CancelHazard => CreatorOpV1::CancelHazard {
                hazard: self.entity("hazard")?,
            },
        };
        Ok(CreatorBatchV1 {
            expected_cursor: self.expected_cursor,
            expected_tick: self.expected_tick,
            operations: vec![operation],
        })
    }

    fn value(&self, id: &str) -> Result<&str, CreatorDraftError> {
        let field = self
            .field(id)
            .ok_or_else(|| CreatorDraftError::UnknownField(id.to_owned()))?;
        if field.kind == CreatorFieldKind::Choice
            && !field
                .choices
                .iter()
                .any(|choice| choice.value == field.value)
        {
            return Err(CreatorDraftError::InvalidChoice(field.label));
        }
        Ok(&field.value)
    }

    fn name(&self, id: &str) -> Result<ObjectName, CreatorDraftError> {
        ObjectName::new(self.value(id)?).map_err(|_| {
            CreatorDraftError::InvalidField(
                "Name",
                "use 1-64 printable ASCII bytes with single interior spaces",
            )
        })
    }

    fn catalog(&self, id: &str) -> Result<CatalogId, CreatorDraftError> {
        CatalogId::new(self.value(id)?)
            .map_err(|_| CreatorDraftError::InvalidField("Catalog choice", "is invalid"))
    }

    fn number<T>(&self, id: &str) -> Result<T, CreatorDraftError>
    where
        T: std::str::FromStr,
    {
        let field = self
            .field(id)
            .ok_or_else(|| CreatorDraftError::UnknownField(id.to_owned()))?;
        field
            .value
            .parse()
            .map_err(|_| CreatorDraftError::InvalidField(field.label, "enter an integer in range"))
    }

    fn entity(&self, id: &str) -> Result<ObjectRefV1, CreatorDraftError> {
        self.optional_entity(id)?.ok_or_else(|| {
            let label = self.field(id).map_or("Entity", |field| field.label);
            CreatorDraftError::InvalidField(label, "choose an object")
        })
    }

    fn optional_entity(&self, id: &str) -> Result<Option<ObjectRefV1>, CreatorDraftError> {
        let value = self.value(id)?;
        if value.is_empty() {
            return Ok(None);
        }
        let quoted = serde_json::Value::String(value.to_owned());
        let entity = serde_json::from_value::<EntityId>(quoted)
            .map_err(|_| CreatorDraftError::InvalidField("Object", "has an invalid identifier"))?;
        Ok(Some(ObjectRefV1::Existing(entity)))
    }
}

fn text(id: &'static str, label: &'static str, value: &str) -> CreatorDraftField {
    CreatorDraftField {
        id,
        label,
        value: value.to_owned(),
        kind: CreatorFieldKind::Text,
        choices: Vec::new(),
    }
}

fn integer(id: &'static str, label: &'static str, value: impl ToString) -> CreatorDraftField {
    CreatorDraftField {
        id,
        label,
        value: value.to_string(),
        kind: CreatorFieldKind::Integer,
        choices: Vec::new(),
    }
}

fn bool_choice(id: &'static str, label: &'static str, value: bool) -> CreatorDraftField {
    CreatorDraftField {
        id,
        label,
        value: value.to_string(),
        kind: CreatorFieldKind::Choice,
        choices: vec![
            CreatorChoice {
                value: "true".to_owned(),
                label: "Yes".to_owned(),
            },
            CreatorChoice {
                value: "false".to_owned(),
                label: "No".to_owned(),
            },
        ],
    }
}

fn catalog_choice(
    id: &'static str,
    label: &'static str,
    selected: &CatalogId,
    values: &[&str],
) -> CreatorDraftField {
    CreatorDraftField {
        id,
        label,
        value: selected.as_str().to_owned(),
        kind: CreatorFieldKind::Choice,
        choices: values
            .iter()
            .map(|value| CreatorChoice {
                value: (*value).to_owned(),
                label: catalog_label(value),
            })
            .collect(),
    }
}

fn entity_choice(
    id: &'static str,
    label: &'static str,
    state: &WorkshopStateV1,
    kind: EntityKind,
    selected: Option<ObjectRefV1>,
    optional: bool,
) -> CreatorDraftField {
    filtered_entity_choice(
        id,
        label,
        state,
        |candidate| candidate == Some(kind),
        selected,
        optional,
    )
}

fn filtered_entity_choice(
    id: &'static str,
    label: &'static str,
    state: &WorkshopStateV1,
    predicate: impl Fn(Option<EntityKind>) -> bool,
    selected: Option<ObjectRefV1>,
    optional: bool,
) -> CreatorDraftField {
    let mut choices = Vec::new();
    if optional {
        choices.push(CreatorChoice {
            value: String::new(),
            label: "None".to_owned(),
        });
    }
    for entity in all_entities(state) {
        if predicate(state.entity_kind(entity)) {
            choices.push(CreatorChoice {
                value: entity_hex(entity),
                label: entity_label(state, entity),
            });
        }
    }
    let value = selected
        .and_then(|reference| match reference {
            ObjectRefV1::Existing(entity) => Some(entity_hex(entity)),
            ObjectRefV1::Local(_) => None,
        })
        .unwrap_or_default();
    CreatorDraftField {
        id,
        label,
        value,
        kind: CreatorFieldKind::Choice,
        choices,
    }
}

fn all_entities(state: &WorkshopStateV1) -> impl Iterator<Item = EntityId> + '_ {
    state
        .factions
        .keys()
        .chain(state.systems.keys())
        .chain(state.stars.keys())
        .chain(state.worlds.keys())
        .chain(state.lanes.keys())
        .chain(state.deposits.keys())
        .chain(state.industries.keys())
        .chain(state.routes.keys())
        .chain(state.shipments.keys())
        .chain(state.hazards.keys())
        .copied()
}

fn entity_hex(entity: EntityId) -> String {
    entity.0.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn entity_label(state: &WorkshopStateV1, entity: EntityId) -> String {
    if let Some(value) = state.factions.get(&entity) {
        return format!("Faction: {}", value.name);
    }
    if let Some(value) = state.systems.get(&entity) {
        return format!("System: {}", value.name);
    }
    if let Some(value) = state.stars.get(&entity) {
        return format!("Star: {}", value.name);
    }
    if let Some(value) = state.worlds.get(&entity) {
        return format!("World: {}", value.name);
    }
    let short = entity_hex(entity).chars().take(8).collect::<String>();
    format!(
        "{} {short}",
        state
            .entity_kind(entity)
            .map_or("Object".to_owned(), |kind| format!("{kind:?}"))
    )
}

fn catalog_label(value: &str) -> String {
    value
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_ascii_uppercase().to_string() + chars.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ownable(kind: Option<EntityKind>) -> bool {
    matches!(
        kind,
        Some(EntityKind::System | EntityKind::Star | EntityKind::World | EntityKind::Industry)
    )
}

fn named(kind: Option<EntityKind>) -> bool {
    matches!(
        kind,
        Some(EntityKind::Faction | EntityKind::System | EntityKind::Star | EntityKind::World)
    )
}

fn removable(kind: Option<EntityKind>) -> bool {
    !matches!(kind, None | Some(EntityKind::Shipment))
}

#[cfg(test)]
mod tests {
    use super::{CreatorFieldKind, edited_creator_text};

    #[test]
    fn native_keyboard_edits_replace_append_and_retain_invalid_intermediate_numbers() {
        assert_eq!(
            edited_creator_text(CreatorFieldKind::Text, "System 1", Some("E"), false, true,),
            Some("E".to_owned())
        );
        assert_eq!(
            edited_creator_text(CreatorFieldKind::Text, "E", Some("m"), false, false),
            Some("Em".to_owned())
        );
        assert_eq!(
            edited_creator_text(CreatorFieldKind::Integer, "0", Some("-"), false, true),
            Some("-".to_owned())
        );
        assert_eq!(
            edited_creator_text(CreatorFieldKind::Integer, "12", Some("x"), false, false),
            None
        );
        assert_eq!(
            edited_creator_text(CreatorFieldKind::Integer, "12", None, true, false),
            Some("1".to_owned())
        );
    }
}
