//! Presentation-only creator drafts. Editing never touches authority; the core
//! validates a complete typed operation batch during preview and acceptance.
use std::{collections::BTreeMap, rc::Rc};

use crate::living_navigation::{LivingEntityKind as K, LivingEntitySelection, navigator_rows};
use nyon_workshop_core::living::{LivingCreatorOperationV2, LivingEntityIdV2, LivingGalaxyStateV2};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LivingCreatorTool {
    System,
    Star,
    World,
    Lane,
    Civilization,
    Deposit,
    Facility,
    Construction,
    HullJob,
    Fleet,
    Hull,
    Route,
    Shipment,
    Hazard,
    Inventory,
    Ownership,
    Colony,
    RemoveColony,
    DepositAmount,
    Policy,
    Relation,
    Agreement,
    RemoveAgreement,
    War,
    Peace,
    FleetOrder,
    ShipmentDisposition,
    Remove,
    EditSystem,
    EditStar,
    EditWorld,
    EditLane,
    EditDeposit,
    EditCivilization,
    FacilityEnabled,
    ScrapFacility,
    CancelConstruction,
    CancelHullJob,
    EditFleet,
    EditHull,
    SplitFleet,
    MergeFleets,
    EditRoute,
    RouteSuspended,
    EditHazard,
    CancelHazard,
}

impl LivingCreatorTool {
    pub const ALL: [Self; 46] = [
        Self::System,
        Self::Star,
        Self::World,
        Self::Lane,
        Self::Civilization,
        Self::Deposit,
        Self::Facility,
        Self::Construction,
        Self::HullJob,
        Self::Fleet,
        Self::Hull,
        Self::Route,
        Self::Shipment,
        Self::Hazard,
        Self::Inventory,
        Self::Ownership,
        Self::Colony,
        Self::RemoveColony,
        Self::DepositAmount,
        Self::Policy,
        Self::Relation,
        Self::Agreement,
        Self::RemoveAgreement,
        Self::War,
        Self::Peace,
        Self::FleetOrder,
        Self::ShipmentDisposition,
        Self::Remove,
        Self::EditSystem,
        Self::EditStar,
        Self::EditWorld,
        Self::EditLane,
        Self::EditDeposit,
        Self::EditCivilization,
        Self::FacilityEnabled,
        Self::ScrapFacility,
        Self::CancelConstruction,
        Self::CancelHullJob,
        Self::EditFleet,
        Self::EditHull,
        Self::SplitFleet,
        Self::MergeFleets,
        Self::EditRoute,
        Self::RouteSuspended,
        Self::EditHazard,
        Self::CancelHazard,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "Create system",
            Self::Star => "Create star",
            Self::World => "Create world",
            Self::Lane => "Connect systems",
            Self::Civilization => "Create civilization",
            Self::Deposit => "Create deposit",
            Self::Facility => "Create facility",
            Self::Construction => "Create construction job",
            Self::HullJob => "Create hull job",
            Self::Fleet => "Create fleet",
            Self::Hull => "Create hull",
            Self::Route => "Create freight route",
            Self::Shipment => "Create shipment",
            Self::Hazard => "Schedule hazard",
            Self::Inventory => "Edit world inventory",
            Self::Ownership => "Transfer world ownership",
            Self::Colony => "Create colony hub",
            Self::RemoveColony => "Remove colony hub",
            Self::DepositAmount => "Edit deposit remaining",
            Self::Policy => "Change civilization policy",
            Self::Relation => "Change relationship",
            Self::Agreement => "Create agreement",
            Self::RemoveAgreement => "Remove agreement",
            Self::War => "Force war",
            Self::Peace => "Force peace",
            Self::FleetOrder => "Set fleet order",
            Self::ShipmentDisposition => "Set shipment disposition",
            Self::Remove => "Remove entity",
            Self::EditSystem => "Edit system",
            Self::EditStar => "Edit star",
            Self::EditWorld => "Edit world",
            Self::EditLane => "Edit lane",
            Self::EditDeposit => "Edit deposit",
            Self::EditCivilization => "Edit civilization",
            Self::FacilityEnabled => "Enable or disable facility",
            Self::ScrapFacility => "Scrap facility",
            Self::CancelConstruction => "Cancel construction job",
            Self::CancelHullJob => "Cancel hull job",
            Self::EditFleet => "Edit fleet",
            Self::EditHull => "Edit hull",
            Self::SplitFleet => "Split fleet",
            Self::MergeFleets => "Merge fleets",
            Self::EditRoute => "Edit freight route",
            Self::RouteSuspended => "Suspend or restore route",
            Self::EditHazard => "Edit scheduled hazard",
            Self::CancelHazard => "Cancel hazard",
        }
    }
}

#[derive(Clone, Debug)]
pub struct LivingCreatorDraft {
    pub tool: LivingCreatorTool,
    pub query: String,
    pub fields: BTreeMap<String, String>,
    template: Value,
    input_identity: Rc<()>,
}

impl LivingCreatorDraft {
    pub fn new(
        tool: LivingCreatorTool,
        state: &LivingGalaxyStateV2,
        selected: Option<LivingEntityIdV2>,
    ) -> Self {
        let template = template(tool, state, selected, None);
        let mut fields = BTreeMap::new();
        flatten(&template, "", &mut fields);
        Self {
            tool,
            query: String::new(),
            fields,
            template,
            input_identity: Rc::new(()),
        }
    }
    /// Populate compatible fields from the exact inspected record, never another
    /// record's properties with only its target identity replaced.
    pub fn new_for_selection(
        tool: LivingCreatorTool,
        state: &LivingGalaxyStateV2,
        selected: Option<LivingEntitySelection>,
    ) -> Self {
        let selected =
            selected.filter(|selection| crate::living_navigation::entity_exists(state, *selection));
        let world = selected.and_then(|selection| {
            navigator_rows(state)
                .into_iter()
                .find(|row| row.selection == selection)
                .and_then(|row| row.world)
        });
        let template = template(tool, state, world, selected);
        let mut fields = BTreeMap::new();
        flatten(&template, "", &mut fields);
        Self {
            tool,
            query: String::new(),
            fields,
            template,
            input_identity: Rc::new(()),
        }
    }

    /// Clones identify the same private draft; a replacement constructor starts
    /// a fresh input owner even when field keys and values are identical.
    pub(crate) fn input_identity(&self) -> Rc<()> {
        Rc::clone(&self.input_identity)
    }

    pub fn value(&self, key: &str) -> Option<&str> {
        if key == "query" {
            Some(&self.query)
        } else {
            self.fields.get(key).map(String::as_str)
        }
    }
    pub fn edit(&mut self, key: &str, value: &str) -> Result<(), String> {
        if value.len() > 4096 {
            return Err("creator field exceeds 4096 UTF-8 bytes".to_owned());
        }
        if key == "query" {
            self.query = value.to_owned();
            return Ok(());
        }
        let field = self.fields.get_mut(key).ok_or("creator field is absent")?;
        *field = value.to_owned();
        Ok(())
    }
    pub fn operations(&self) -> Result<Vec<LivingCreatorOperationV2>, String> {
        let mut value = self.template.clone();
        for (key, edited) in &self.fields {
            let leaf = field_mut(&mut value, key).ok_or("creator field shape changed")?;
            *leaf = match leaf {
                Value::String(_) => Value::String(edited.clone()),
                Value::Number(_) | Value::Bool(_) => {
                    serde_json::from_str(edited).map_err(|_| {
                        format!(
                            "{} requires a canonical number or boolean",
                            key.replace('.', " ")
                        )
                    })?
                }
                Value::Null => {
                    if edited.is_empty() || edited == "none" {
                        Value::Null
                    } else if key.ends_with("owner")
                        || key.ends_with("target_world")
                        || key.ends_with("target_fleet")
                    {
                        json!({"type":"existing", "id":edited})
                    } else if key.ends_with("kind") {
                        Value::String(edited.clone())
                    } else {
                        serde_json::from_str(edited).map_err(|_| {
                            format!("{} requires an optional value", key.replace('.', " "))
                        })?
                    }
                }
                Value::Array(_) => {
                    let mut targets = Vec::new();
                    for id in edited.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                        targets.push(json!({"type":"existing", "id":id}));
                    }
                    Value::Array(targets)
                }
                Value::Object(_) => return Err("creator field is not editable".to_owned()),
            };
        }
        if let Value::Array(operations) = &mut value {
            for operation in operations {
                if let Some(cascade) = operation.get_mut("cascade").and_then(Value::as_object_mut) {
                    match cascade.get("type").and_then(Value::as_str) {
                        Some("reject_if_referenced") => {
                            cascade.remove("dependents");
                            cascade.remove("derived");
                        }
                        Some("remove_listed") => {
                            cascade.remove("derived");
                        }
                        _ => {}
                    }
                }
            }
        }
        serde_json::from_value(value).map_err(|error| format!("Creator validation: {error}"))
    }
    pub fn operations_reviewed(
        &self,
        state: &LivingGalaxyStateV2,
    ) -> Result<Vec<LivingCreatorOperationV2>, String> {
        let mut operations = self.operations()?;
        for operation in &mut operations {
            let (target, cascade) = match operation {
                LivingCreatorOperationV2::RemoveEntity { target, cascade }
                | LivingCreatorOperationV2::ScrapFacility { target, cascade } => (target, cascade),
                _ => continue,
            };
            if matches!(
                cascade,
                nyon_workshop_core::living::LivingCascadeDispositionV2::RemoveReviewed { .. }
            ) && let nyon_workshop_core::living::LivingCommandTargetV2::Existing { id } = target
            {
                *cascade = serde_json::from_value(reviewed_cascade(state, *id))
                    .map_err(|error| format!("Removal preview: {error}"))?;
            }
        }
        Ok(operations)
    }
    pub fn matching_tools(&self) -> Vec<LivingCreatorTool> {
        let query = self.query.to_ascii_lowercase();
        LivingCreatorTool::ALL
            .into_iter()
            .filter(|tool| tool.label().to_ascii_lowercase().contains(&query))
            .collect()
    }
}

fn flatten(value: &Value, prefix: &str, fields: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if (prefix.ends_with("cascade") && (key == "derived" || key == "dependents"))
                    || (key == "type" && !prefix.ends_with("cascade"))
                    || key == "local"
                {
                    continue;
                }
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, &path, fields);
            }
        }
        Value::Array(rows) if prefix.is_empty() => {
            for (index, row) in rows.iter().enumerate() {
                flatten(row, &index.to_string(), fields);
            }
        }
        Value::String(value) => {
            fields.insert(prefix.to_owned(), value.clone());
        }
        Value::Null => {
            fields.insert(prefix.to_owned(), "none".to_owned());
        }
        Value::Array(rows) => {
            fields.insert(
                prefix.to_owned(),
                rows.iter()
                    .filter_map(|row| row.get("id").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        _ => {
            fields.insert(prefix.to_owned(), value.to_string());
        }
    }
}
fn field_mut<'a>(value: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut leaf = value;
    for segment in path.split('.') {
        leaf = if let Value::Array(rows) = leaf {
            rows.get_mut(segment.parse::<usize>().ok()?)?
        } else {
            leaf.get_mut(segment)?
        };
    }
    Some(leaf)
}
fn target(id: LivingEntityIdV2) -> Value {
    json!({"type":"existing", "id":id.0.iter().map(|byte| format!("{byte:02x}")).collect::<String>()})
}
fn template(
    tool: LivingCreatorTool,
    state: &LivingGalaxyStateV2,
    selected: Option<LivingEntityIdV2>,
    context: Option<LivingEntitySelection>,
) -> Value {
    let absent = LivingEntityIdV2([0; 16]);
    let world = selected
        .or_else(|| {
            if context.is_none() {
                state.worlds.as_slice().first().map(|row| row.id)
            } else {
                None
            }
        })
        .unwrap_or(absent);
    let other_world = state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id != world)
        .map_or(absent, |row| row.id);
    let owner = context
        .filter(|selection| selection.kind == K::Civilization)
        .map(|selection| selection.id)
        .or_else(|| {
            context.and_then(|selection| {
                state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|fleet| {
                        (selection.kind == K::Fleet && fleet.id == selection.id)
                            || (selection.kind == K::Hull
                                && fleet
                                    .hulls
                                    .as_slice()
                                    .iter()
                                    .any(|hull| hull.id == selection.id))
                    })
                    .map(|fleet| fleet.owner)
            })
        })
        .or_else(|| {
            state
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == world)
                .and_then(|row| row.owner)
        })
        .or_else(|| {
            if context.is_none() {
                state.civilizations.as_slice().first().map(|row| row.id)
            } else {
                None
            }
        })
        .unwrap_or(absent);
    let other_owner = state
        .civilizations
        .as_slice()
        .iter()
        .find(|row| row.id != owner)
        .map_or(absent, |row| row.id);
    let (a, b) = if owner < other_owner {
        (owner, other_owner)
    } else {
        (other_owner, owner)
    };
    let system = context
        .and_then(|selection| {
            let rows = navigator_rows(state);
            let mut key = Some(selection);
            while let Some(selection) = key {
                if selection.kind == K::System {
                    return Some(selection.id);
                }
                key = rows
                    .iter()
                    .find(|row| row.selection == selection)
                    .and_then(|row| row.parent);
            }
            None
        })
        .or_else(|| {
            state
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == world)
                .map(|row| row.system)
        })
        .or_else(|| {
            if context.is_none() {
                state.systems.as_slice().first().map(|row| row.id)
            } else {
                None
            }
        })
        .unwrap_or(absent);
    let other_system = state
        .systems
        .as_slice()
        .iter()
        .find(|row| row.id != system)
        .map_or(absent, |row| row.id);
    let (sa, sb) = if system < other_system {
        (system, other_system)
    } else {
        (other_system, system)
    };
    let fleet_row = context.and_then(|selection| state.fleets.as_slice().iter().find(|row| (selection.kind == K::Fleet && row.id == selection.id) || (selection.kind == K::Hull && row.hulls.as_slice().iter().any(|hull| hull.id == selection.id))))
        .or_else(||state.fleets.as_slice().iter().find(|row|matches!(row.location,nyon_workshop_core::living::LivingFleetLocationV2::Docked{world:at} if at==world))).or_else(||state.fleets.as_slice().first());
    let fleet = fleet_row.map_or(absent, |row| row.id);
    let hull = context
        .filter(|selection| selection.kind == K::Hull)
        .and_then(|selection| {
            fleet_row.and_then(|row| {
                row.hulls
                    .as_slice()
                    .iter()
                    .find(|hull| hull.id == selection.id)
            })
        })
        .or_else(|| fleet_row.and_then(|row| row.hulls.as_slice().first()));
    let world_row = state.worlds.as_slice().iter().find(|row| row.id == world);
    let system_row = state.systems.as_slice().iter().find(|row| row.id == system);
    let star = context
        .filter(|selection| selection.kind == K::Star)
        .and_then(|selection| {
            state
                .stars
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .stars
                .as_slice()
                .iter()
                .find(|row| row.system == system)
                .or_else(|| {
                    if context.is_none() {
                        state.stars.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let lane = context
        .filter(|selection| selection.kind == K::Lane)
        .and_then(|selection| {
            state
                .lanes
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .lanes
                .as_slice()
                .iter()
                .find(|row| row.system_a == system || row.system_b == system)
                .or_else(|| {
                    if context.is_none() {
                        state.lanes.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let deposit = context
        .filter(|selection| selection.kind == K::Deposit)
        .and_then(|selection| {
            state
                .deposits
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .deposits
                .as_slice()
                .iter()
                .find(|row| row.world == world)
                .or_else(|| {
                    if context.is_none() {
                        state.deposits.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let civ = state
        .civilizations
        .as_slice()
        .iter()
        .find(|row| row.id == owner);
    let facility = context
        .filter(|selection| selection.kind == K::Facility)
        .and_then(|selection| {
            state
                .facilities
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .facilities
                .as_slice()
                .iter()
                .find(|row| row.world == world)
                .or_else(|| {
                    if context.is_none() {
                        state.facilities.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let construction = context
        .filter(|selection| selection.kind == K::ConstructionJob)
        .and_then(|selection| {
            state
                .construction_jobs
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .construction_jobs
                .as_slice()
                .iter()
                .find(|row| row.world == world)
                .or_else(|| {
                    if context.is_none() {
                        state.construction_jobs.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let hull_job = context
        .filter(|selection| selection.kind == K::HullJob)
        .and_then(|selection| {
            state
                .hull_jobs
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .hull_jobs
                .as_slice()
                .iter()
                .find(|row| row.world == world)
                .or_else(|| {
                    if context.is_none() {
                        state.hull_jobs.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let route = context
        .filter(|selection| selection.kind == K::Route)
        .and_then(|selection| {
            state
                .routes
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state
                .routes
                .as_slice()
                .iter()
                .find(|row| row.source_world == world || row.destination_world == world)
                .or_else(|| {
                    if context.is_none() {
                        state.routes.as_slice().first()
                    } else {
                        None
                    }
                })
        });
    let hazard = context
        .filter(|selection| selection.kind == K::Hazard)
        .and_then(|selection| {
            state
                .hazards
                .as_slice()
                .iter()
                .find(|row| row.id == selection.id)
        })
        .or_else(|| {
            state.hazards.as_slice().iter().find(|hazard| {
                state.lanes.as_slice().iter().any(|lane| {
                    lane.id == hazard.lane && (lane.system_a == system || lane.system_b == system)
                })
            })
        })
        .or_else(|| {
            if context.is_none() {
                state.hazards.as_slice().first()
            } else {
                None
            }
        });
    let tick = state.tick.0;
    let operation = match tool {
        LivingCreatorTool::System => json!({"type":"create_system","local":0,"name":"New system"}),
        LivingCreatorTool::Star => {
            json!({"type":"create_star","local":0,"system":target(system),"archetype":"main_sequence","name":"New star"})
        }
        LivingCreatorTool::World => {
            json!({"type":"create_world","local":0,"system":target(system),"archetype":"temperate","name":"New world","owner":null,"inventory":{"energy":0,"ore":0,"alloy":0}})
        }
        LivingCreatorTool::Lane => {
            json!({"type":"create_lane","local":0,"system_a":target(sa),"system_b":target(sb),"distance_units":100})
        }
        LivingCreatorTool::Civilization => {
            json!({"type":"create_civilization","local":0,"name":"New civilization","policy":"neutral","status":"dormant"})
        }
        LivingCreatorTool::Deposit => {
            json!({"type":"create_deposit","local":0,"world":target(world),"resource":"ore","remaining_units":20000})
        }
        LivingCreatorTool::Facility => {
            json!({"type":"create_facility","local":0,"world":target(world),"definition":"solar_array","status":"operational","paid_alloy":0,"hit_points":0,"next_due":tick.saturating_add(10)})
        }
        LivingCreatorTool::Construction => {
            json!({"type":"create_construction_job","local":0,"world":target(world),"owner":target(owner),"definition":"foundry","accepted_tick":tick,"completion_tick":tick.saturating_add(200),"paid_alloy":20})
        }
        LivingCreatorTool::HullJob => {
            json!({"type":"create_hull_job","local":0,"world":target(world),"owner":target(owner),"definition":"escort","accepted_tick":tick,"completion_tick":tick.saturating_add(200),"paid_alloy":20,"target_fleet":null})
        }
        LivingCreatorTool::Fleet => {
            return json!([{"type":"create_fleet","local":0,"owner":target(owner),"world":target(world)},{"type":"create_hull","local":1,"fleet":{"type":"local","local":0},"definition":"escort","hit_points":10,"return_credit":false}]);
        }
        LivingCreatorTool::Hull => {
            json!({"type":"create_hull","local":0,"fleet":target(fleet),"definition":"escort","hit_points":10,"return_credit":false})
        }
        LivingCreatorTool::Route => {
            json!({"type":"create_route","local":0,"kind":"internal","source_world":target(world),"destination_world":target(other_world),"resource":"ore","batch_size":10,"cadence_ticks":10,"source_reserve":20,"source_owner":target(owner),"receiver_owner":target(owner),"next_due":tick.saturating_add(10)})
        }
        LivingCreatorTool::Shipment => {
            json!({"type":"create_shipment","local":0,"dispatch_owner":target(owner),"intended_receiver":target(other_owner),"source_world":target(world),"destination_world":target(other_world),"resource":"ore","units":10,"departure_tick":tick,"arrival_tick":tick.saturating_add(10),"disposition":"outbound"})
        }
        LivingCreatorTool::Hazard => {
            json!({"type":"create_hazard","local":0,"definition":"ion_storm","lane":target(lane.map_or(absent, |row| row.id)),"start_tick":tick.saturating_add(10),"end_tick":tick.saturating_add(100)})
        }
        LivingCreatorTool::Inventory => {
            json!({"type":"set_world_inventory","world":target(world),"inventory":{"energy":100,"ore":100,"alloy":120}})
        }
        LivingCreatorTool::Ownership => {
            json!({"type":"set_world_owner","world":target(world),"owner":target(owner)})
        }
        LivingCreatorTool::Colony => {
            json!({"type":"create_colony","world":target(world),"owner":target(owner)})
        }
        LivingCreatorTool::RemoveColony => json!({"type":"remove_colony","world":target(world)}),
        LivingCreatorTool::DepositAmount => {
            json!({"type":"set_deposit_remaining","deposit":target(deposit.map_or(absent, |row| row.id)),"remaining_units":20000})
        }
        LivingCreatorTool::Policy => {
            json!({"type":"set_civilization_policy","civilization":target(owner),"policy":"neutral"})
        }
        LivingCreatorTool::Relation => {
            json!({"type":"set_relation_base","from":target(owner),"to":target(other_owner),"base_disposition":20})
        }
        LivingCreatorTool::Agreement => {
            json!({"type":"set_agreement","kind":"trade","participant_a":target(a),"participant_b":target(b),"start_tick":tick,"end_tick":tick.saturating_add(1000)})
        }
        LivingCreatorTool::RemoveAgreement => {
            json!({"type":"remove_agreement","kind":"trade","participant_a":target(a),"participant_b":target(b)})
        }
        LivingCreatorTool::War => {
            json!({"type":"force_war","participant_a":target(a),"participant_b":target(b),"declarer":target(owner),"start_tick":tick,"end_tick":tick.saturating_add(1000)})
        }
        LivingCreatorTool::Peace => {
            json!({"type":"force_peace","participant_a":target(a),"participant_b":target(b)})
        }
        LivingCreatorTool::FleetOrder => {
            json!({"type":"set_fleet_order","fleet":target(fleet),"kind":null,"target_world":null,"not_before_tick":tick,"lane_path":[]})
        }
        LivingCreatorTool::ShipmentDisposition => {
            json!({"type":"set_shipment_disposition","shipment":target(context.filter(|selection| selection.kind == K::Shipment).map(|selection| selection.id).or_else(||state.shipments.as_slice().first().map(|row|row.id)).unwrap_or(absent)),"disposition":"returning"})
        }
        LivingCreatorTool::Remove => {
            if context.is_some_and(|selection| selection.kind == K::Colony) {
                return json!([{"type":"remove_colony","world":target(world)}]);
            }
            let id = context.map_or(world, |selection| selection.id);
            json!({"type":"remove_entity","target":target(id),"cascade":reviewed_cascade(state,id)})
        }
        LivingCreatorTool::EditSystem => {
            json!({"type":"edit_system","target":target(system),"name":system_row.map_or("System",|row|row.name.as_str())})
        }
        LivingCreatorTool::EditStar => {
            json!({"type":"edit_star","target":target(star.map_or(absent,|row|row.id)),"system":target(star.map_or(system,|row|row.system)),"archetype":star.map_or("main_sequence",|row|row.archetype.as_str()),"name":star.map_or("Star",|row|row.name.as_str())})
        }
        LivingCreatorTool::EditWorld => {
            json!({"type":"edit_world","target":target(world),"system":target(system),"archetype":world_row.map_or("temperate",|row|row.archetype.as_str()),"name":world_row.map_or("World",|row|row.name.as_str())})
        }
        LivingCreatorTool::EditLane => {
            json!({"type":"edit_lane","target":target(lane.map_or(absent,|row|row.id)),"system_a":target(lane.map_or(sa,|row|row.system_a)),"system_b":target(lane.map_or(sb,|row|row.system_b)),"distance_units":lane.map_or(100,|row|row.distance_units)})
        }
        LivingCreatorTool::EditDeposit => {
            json!({"type":"edit_deposit","target":target(deposit.map_or(absent,|row|row.id)),"resource":deposit.map_or(nyon_workshop_core::living::LivingResourceV2::Ore,|row|row.resource),"remaining_units":deposit.map_or(20000,|row|row.remaining_units)})
        }
        LivingCreatorTool::EditCivilization => {
            json!({"type":"edit_civilization","target":target(owner),"name":civ.map_or("Civilization",|row|row.name.as_str()),"policy":civ.map_or("neutral",|row|row.policy.0.as_str())})
        }
        LivingCreatorTool::FacilityEnabled => {
            json!({"type":"set_facility_enabled","target":target(facility.map_or(absent,|row|row.id)),"enabled":facility.is_some_and(|row|row.disabled)})
        }
        LivingCreatorTool::ScrapFacility => {
            let id = facility.map_or(absent, |row| row.id);
            json!({"type":"scrap_facility","target":target(id),"cascade":reviewed_cascade(state,id)})
        }
        LivingCreatorTool::CancelConstruction => {
            json!({"type":"cancel_construction_job","target":target(construction.map_or(absent,|row|row.id))})
        }
        LivingCreatorTool::CancelHullJob => {
            json!({"type":"cancel_hull_job","target":target(hull_job.map_or(absent,|row|row.id))})
        }
        LivingCreatorTool::EditFleet => {
            json!({"type":"edit_fleet","target":target(fleet),"owner":target(fleet_row.map_or(owner,|row|row.owner)),"world":target(fleet_row.and_then(|row|match row.location {nyon_workshop_core::living::LivingFleetLocationV2::Docked{world}=>Some(world),_=>None}).unwrap_or(if context.is_some() {absent} else {world}))})
        }
        LivingCreatorTool::EditHull => {
            json!({"type":"edit_hull","target":target(hull.map_or(absent,|row|row.id)),"definition":hull.map_or("escort",|row|row.definition.as_str()),"hit_points":hull.map_or(10,|row|row.hit_points),"return_credit":hull.is_some_and(|row|row.return_credit)})
        }
        LivingCreatorTool::SplitFleet => {
            let source = if context.is_some_and(|selection| selection.kind == K::Fleet) {
                fleet_row
            } else {
                fleet_row.filter(|row| {
                    row.hulls.as_slice().len() > 1
                        && row.order.is_none()
                        && matches!(
                            row.location,
                            nyon_workshop_core::living::LivingFleetLocationV2::Docked { .. }
                        )
                })
            };
            json!({"type":"split_fleet","local":0,"source":target(source.map_or(fleet,|row|row.id)),"hulls":source.and_then(|row|row.hulls.as_slice().first()).map(|row|vec![target(row.id)]).unwrap_or_default()})
        }
        LivingCreatorTool::MergeFleets => {
            let source = fleet_row.and_then(|selected| {
                state.fleets.as_slice().iter().find(|row| {
                    row.id != selected.id
                        && row.owner == selected.owner
                        && row.location == selected.location
                        && row.order.is_none()
                })
            });
            json!({"type":"merge_fleets","target":target(fleet),"source":target(source.map_or(absent,|row|row.id))})
        }
        LivingCreatorTool::EditRoute => {
            json!({"type":"edit_route","target":target(route.map_or(absent,|row|row.id)),"kind":route.map_or(nyon_workshop_core::living::LivingRouteKindV2::Internal,|row|row.kind),"source_world":target(route.map_or(world,|row|row.source_world)),"destination_world":target(route.map_or(other_world,|row|row.destination_world)),"resource":route.map_or(nyon_workshop_core::living::LivingResourceV2::Ore,|row|row.resource),"batch_size":route.map_or(10,|row|row.batch_size),"cadence_ticks":route.map_or(10,|row|row.cadence_ticks),"source_reserve":route.map_or(20,|row|row.source_reserve),"source_owner":target(route.map_or(owner,|row|row.source_owner)),"receiver_owner":target(route.map_or(owner,|row|row.receiver_owner))})
        }
        LivingCreatorTool::RouteSuspended => {
            json!({"type":"set_route_suspended","target":target(route.map_or(absent,|row|row.id)),"suspended":route.is_none_or(|row|!row.suspended)})
        }
        LivingCreatorTool::EditHazard => {
            json!({"type":"edit_hazard","target":target(hazard.map_or(absent,|row|row.id)),"lane":target(hazard.map_or(absent,|row|row.lane)),"definition":hazard.map_or("ion_storm",|row|row.definition.as_str()),"start_tick":hazard.map_or(tick,|row|row.start_tick.0.max(tick)),"end_tick":hazard.map_or(tick.saturating_add(100),|row|row.end_tick.0.max(tick.saturating_add(1)))})
        }
        LivingCreatorTool::CancelHazard => {
            json!({"type":"cancel_hazard","target":target(hazard.map_or(absent,|row|row.id))})
        }
    };
    json!([operation])
}

fn reviewed_cascade(state: &LivingGalaxyStateV2, id: LivingEntityIdV2) -> Value {
    let dependents = nyon_workshop_core::living::preview_living_removal_v2(state, id)
        .map(|preview| {
            preview
                .dependents
                .into_iter()
                .map(target)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let derived = nyon_workshop_core::living::preview_living_derived_removal_v2(state, id)
        .unwrap_or_default();
    json!({"type":"remove_reviewed","dependents":dependents,"derived":derived})
}

pub fn removal_review_lines(operations: &[LivingCreatorOperationV2]) -> Vec<String> {
    let mut lines = Vec::new();
    for operation in operations {
        match operation {
            LivingCreatorOperationV2::RemoveEntity { .. } => lines.push("Creator removal override: releases reservations and removes reviewed records; no economic refund.".to_owned()),
            LivingCreatorOperationV2::ScrapFacility { .. } => lines.push("Creator scrap override: removes the facility and reviewed dependents; no economic refund.".to_owned()),
            LivingCreatorOperationV2::CancelConstructionJob { .. } | LivingCreatorOperationV2::CancelHullJob { .. } => lines.push("Explicit job cancellation: half of recorded paid alloy is refunded; review the exact projected inventory change.".to_owned()),
            LivingCreatorOperationV2::SetWorldOwner { .. } => lines.push("Ownership transfer requires foreign paid jobs to be explicitly cancelled or removed earlier in this same atomic batch.".to_owned()),
            _=>{},
        }

        let cascade = match operation {
            LivingCreatorOperationV2::RemoveEntity { cascade, .. }
            | LivingCreatorOperationV2::ScrapFacility { cascade, .. } => cascade,
            _ => continue,
        };
        if let nyon_workshop_core::living::LivingCascadeDispositionV2::RemoveReviewed {
            dependents,
            derived,
        } = cascade
        {
            for dependent in dependents {
                if let nyon_workshop_core::living::LivingCommandTargetV2::Existing { id } =
                    dependent
                {
                    lines.push(format!(
                        "Remove dependent {}",
                        id.0.iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ));
                }
            }
            for row in derived {
                if let Ok(Value::Object(fields)) = serde_json::to_value(row) {
                    let kind = fields
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("record")
                        .replace('_', " ");
                    let values = fields
                        .iter()
                        .filter(|(key, _)| key.as_str() != "type")
                        .map(|(key, value)| {
                            format!(
                                "{}: {}",
                                key.replace('_', " "),
                                value
                                    .as_str()
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| value.to_string())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(format!("Remove {kind}; {values}"));
                }
            }
        }
    }
    lines
}
