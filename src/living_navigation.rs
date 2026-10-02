//! Read-only entity hierarchy and inspector facts. Stable full identities never
//! depend on a visible row number or presentation geometry.
use nyon_workshop_core::living::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LivingEntityKind {
    System,
    Star,
    World,
    Colony,
    Deposit,
    Facility,
    ConstructionJob,
    HullJob,
    Civilization,
    Lane,
    Fleet,
    Hull,
    Route,
    Shipment,
    Hazard,
}
impl LivingEntityKind {
    pub const ALL: [Self; 15] = [
        Self::System,
        Self::Star,
        Self::World,
        Self::Colony,
        Self::Deposit,
        Self::Facility,
        Self::ConstructionJob,
        Self::HullJob,
        Self::Civilization,
        Self::Lane,
        Self::Fleet,
        Self::Hull,
        Self::Route,
        Self::Shipment,
        Self::Hazard,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Star => "Star",
            Self::World => "World",
            Self::Colony => "Colony",
            Self::Deposit => "Deposit",
            Self::Facility => "Facility",
            Self::ConstructionJob => "Construction job",
            Self::HullJob => "Hull job",
            Self::Civilization => "Civilization",
            Self::Lane => "Lane",
            Self::Fleet => "Fleet",
            Self::Hull => "Hull",
            Self::Route => "Route",
            Self::Shipment => "Shipment",
            Self::Hazard => "Hazard",
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LivingEntitySelection {
    pub kind: LivingEntityKind,
    pub id: LivingEntityIdV2,
}
impl LivingEntitySelection {
    pub const fn new(kind: LivingEntityKind, id: LivingEntityIdV2) -> Self {
        Self { kind, id }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNavigatorRow {
    pub selection: LivingEntitySelection,
    pub parent: Option<LivingEntitySelection>,
    pub label: String,
    pub world: Option<LivingEntityIdV2>,
    pub depth: usize,
    pub has_children: bool,
}
fn entity(kind: LivingEntityKind, id: LivingEntityIdV2) -> LivingEntitySelection {
    LivingEntitySelection::new(kind, id)
}
pub fn entity_id(id: LivingEntityIdV2) -> String {
    id.0.iter().map(|byte| format!("{byte:02X}")).collect()
}

pub fn entity_exists(state: &LivingGalaxyStateV2, selected: LivingEntitySelection) -> bool {
    use LivingEntityKind as K;
    macro_rules! has {
        ($rows:expr,$key:ident) => {
            $rows.iter().any(|row| row.$key == selected.id)
        };
    }
    match selected.kind {
        K::System => has!(state.systems.as_slice(), id),
        K::Star => has!(state.stars.as_slice(), id),
        K::World => has!(state.worlds.as_slice(), id),
        K::Colony => has!(state.colonies.as_slice(), world),
        K::Deposit => has!(state.deposits.as_slice(), id),
        K::Facility => has!(state.facilities.as_slice(), id),
        K::ConstructionJob => has!(state.construction_jobs.as_slice(), id),
        K::HullJob => has!(state.hull_jobs.as_slice(), id),
        K::Civilization => has!(state.civilizations.as_slice(), id),
        K::Lane => has!(state.lanes.as_slice(), id),
        K::Fleet => has!(state.fleets.as_slice(), id),
        K::Route => has!(state.routes.as_slice(), id),
        K::Shipment => has!(state.shipments.as_slice(), id),
        K::Hazard => has!(state.hazards.as_slice(), id),
        K::Hull => state
            .fleets
            .as_slice()
            .iter()
            .any(|fleet| has!(fleet.hulls.as_slice(), id)),
    }
}

pub fn navigator_rows(state: &LivingGalaxyStateV2) -> Vec<LivingNavigatorRow> {
    use LivingEntityKind as K;
    let mut rows = Vec::new();
    let mut push = |kind, id, parent, label: String, world| {
        rows.push(LivingNavigatorRow {
            selection: entity(kind, id),
            parent,
            label,
            world,
            depth: 0,
            has_children: false,
        })
    };
    for row in state.systems.as_slice() {
        push(K::System, row.id, None, row.name.as_str().into(), None);
    }
    for row in state.stars.as_slice() {
        push(
            K::Star,
            row.id,
            Some(entity(K::System, row.system)),
            row.name.as_str().into(),
            None,
        );
    }
    for row in state.worlds.as_slice() {
        push(
            K::World,
            row.id,
            Some(entity(K::System, row.system)),
            row.name.as_str().into(),
            Some(row.id),
        );
    }
    for row in state.civilizations.as_slice() {
        push(
            K::Civilization,
            row.id,
            None,
            row.name.as_str().into(),
            state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.owner == Some(row.id))
                .map(|world| world.id),
        );
    }
    for row in state.lanes.as_slice() {
        push(
            K::Lane,
            row.id,
            Some(entity(K::System, row.system_a)),
            format!("Lane {} units", row.distance_units),
            None,
        );
    }
    for row in state.colonies.as_slice() {
        push(
            K::Colony,
            row.world,
            Some(entity(K::World, row.world)),
            "Colony and hub".into(),
            Some(row.world),
        );
    }
    for row in state.deposits.as_slice() {
        push(
            K::Deposit,
            row.id,
            Some(entity(K::World, row.world)),
            format!("{} deposit", row.resource.as_wire_str()),
            Some(row.world),
        );
    }
    for row in state.facilities.as_slice() {
        push(
            K::Facility,
            row.id,
            Some(entity(K::World, row.world)),
            row.definition.as_str().replace('_', " "),
            Some(row.world),
        );
    }
    for row in state.construction_jobs.as_slice() {
        push(
            K::ConstructionJob,
            row.id,
            Some(entity(K::World, row.world)),
            format!("Construct {}", row.definition.as_str()),
            Some(row.world),
        );
    }
    for row in state.hull_jobs.as_slice() {
        push(
            K::HullJob,
            row.id,
            Some(entity(K::World, row.world)),
            format!("Build {}", row.definition.as_str()),
            Some(row.world),
        );
    }
    for row in state.fleets.as_slice() {
        let world = match row.location {
            LivingFleetLocationV2::Docked { world } => world,
            LivingFleetLocationV2::Travelling { origin_world, .. } => origin_world,
        };
        push(
            K::Fleet,
            row.id,
            Some(entity(K::World, world)),
            format!("Fleet / {} hulls", row.hulls.as_slice().len()),
            Some(world),
        );
        for hull in row.hulls.as_slice() {
            push(
                K::Hull,
                hull.id,
                Some(entity(K::Fleet, row.id)),
                hull.definition.as_str().replace('_', " "),
                Some(world),
            );
        }
    }
    for row in state.routes.as_slice() {
        push(
            K::Route,
            row.id,
            Some(entity(K::World, row.source_world)),
            format!("{} route", row.resource.as_wire_str()),
            Some(row.source_world),
        );
    }
    for row in state.shipments.as_slice() {
        push(
            K::Shipment,
            row.id,
            Some(entity(K::World, row.source_world)),
            format!(
                "{} freight / {} units",
                row.resource.as_wire_str(),
                row.units
            ),
            Some(row.source_world),
        );
    }
    for row in state.hazards.as_slice() {
        push(
            K::Hazard,
            row.id,
            Some(entity(K::Lane, row.lane)),
            row.definition.as_str().replace('_', " "),
            None,
        );
    }
    let by_key: BTreeMap<_, _> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.selection, index))
        .collect();
    let parents: BTreeSet<_> = rows.iter().filter_map(|row| row.parent).collect();
    for row in &mut rows {
        row.has_children = parents.contains(&row.selection);
    }
    let mut children: BTreeMap<Option<LivingEntitySelection>, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        children
            .entry(row.parent.filter(|parent| by_key.contains_key(parent)))
            .or_default()
            .push(index);
    }
    for indices in children.values_mut() {
        indices.sort_by(|a, b| {
            (
                rows[*a].selection.kind,
                &rows[*a].label,
                rows[*a].selection.id,
            )
                .cmp(&(
                    rows[*b].selection.kind,
                    &rows[*b].label,
                    rows[*b].selection.id,
                ))
        });
    }
    fn visit(
        parent: Option<LivingEntitySelection>,
        depth: usize,
        rows: &[LivingNavigatorRow],
        children: &BTreeMap<Option<LivingEntitySelection>, Vec<usize>>,
        output: &mut Vec<LivingNavigatorRow>,
    ) {
        if let Some(indices) = children.get(&parent) {
            for index in indices {
                let mut row = rows[*index].clone();
                row.depth = depth;
                let key = row.selection;
                output.push(row);
                visit(Some(key), depth + 1, rows, children, output);
            }
        }
    }
    let mut output = Vec::with_capacity(rows.len());
    visit(None, 0, &rows, &children, &mut output);
    output
}

pub fn visible_navigator_rows(
    state: &LivingGalaxyStateV2,
    query: &str,
    kind: Option<LivingEntityKind>,
    expanded: &BTreeSet<LivingEntitySelection>,
) -> Vec<LivingNavigatorRow> {
    let rows = navigator_rows(state);
    let query = query.to_ascii_lowercase();
    let by_key: BTreeMap<_, _> = rows.iter().map(|row| (row.selection, row.parent)).collect();
    let filtering = !query.is_empty() || kind.is_some();
    let mut included = BTreeSet::new();
    if filtering {
        for row in &rows {
            if kind.is_none_or(|kind| kind == row.selection.kind)
                && (query.is_empty()
                    || format!(
                        "{} {} {}",
                        row.selection.kind.label(),
                        row.label,
                        entity_id(row.selection.id)
                    )
                    .to_ascii_lowercase()
                    .contains(&query))
            {
                let mut key = Some(row.selection);
                while let Some(current) = key {
                    included.insert(current);
                    key = by_key.get(&current).copied().flatten();
                }
            }
        }
    }
    rows.into_iter()
        .filter(|row| {
            if filtering {
                return included.contains(&row.selection);
            }
            let mut parent = row.parent;
            while let Some(key) = parent {
                if !expanded.contains(&key) {
                    return false;
                }
                parent = by_key.get(&key).copied().flatten();
            }
            true
        })
        .collect()
}

pub fn entity_details(
    state: &LivingGalaxyStateV2,
    catalog: &ValidatedLivingCatalogPackV2,
    selected: LivingEntitySelection,
) -> Option<Vec<String>> {
    use LivingEntityKind as K;
    let mut lines = vec![
        format!("{} / {}", selected.kind.label(), entity_id(selected.id)),
        format!("Authoritative state at tick {}", state.tick.0),
    ];
    macro_rules! facts {
        ($collection:expr,$key:ident) => {{
            let row = $collection.iter().find(|row| row.$key == selected.id)?;
            append_facts(&mut lines, serde_json::to_value(row).ok()?);
            row
        }};
    }
    match selected.kind {
        K::System => {
            facts!(state.systems.as_slice(), id);
        }
        K::Star => {
            facts!(state.stars.as_slice(), id);
        }
        K::World => {
            facts!(state.worlds.as_slice(), id);
            for facility in state
                .facilities
                .as_slice()
                .iter()
                .filter(|row| row.world == selected.id)
            {
                lines.push(format!(
                    "Facility {} / {}",
                    facility.definition.as_str(),
                    entity_id(facility.id)
                ));
                facility_details(&mut lines, state, catalog, facility);
            }
            for job in state
                .construction_jobs
                .as_slice()
                .iter()
                .filter(|row| row.world == selected.id)
            {
                lines.push(format!(
                    "Under construction: {} / completes {}",
                    job.definition.as_str(),
                    job.completion_tick.0
                ));
            }
        }
        K::Colony => {
            facts!(state.colonies.as_slice(), world);
        }
        K::Deposit => {
            facts!(state.deposits.as_slice(), id);
        }
        K::ConstructionJob => {
            facts!(state.construction_jobs.as_slice(), id);
            lines.push("Operation state: Under construction".into());
        }
        K::HullJob => {
            facts!(state.hull_jobs.as_slice(), id);
            lines.push("Operation state: Under construction".into());
        }
        K::Fleet => {
            facts!(state.fleets.as_slice(), id);
        }
        K::Hull => {
            let row = state
                .fleets
                .as_slice()
                .iter()
                .flat_map(|fleet| fleet.hulls.as_slice())
                .find(|row| row.id == selected.id)?;
            append_facts(&mut lines, serde_json::to_value(row).ok()?);
        }
        K::Lane => {
            facts!(state.lanes.as_slice(), id);
        }
        K::Shipment => {
            facts!(state.shipments.as_slice(), id);
        }
        K::Hazard => {
            let row = facts!(state.hazards.as_slice(), id);
            lines.push(format!(
                "Hazard interval: {}",
                if state.tick < row.start_tick {
                    "Scheduled"
                } else if state.tick < row.end_tick {
                    "Active"
                } else {
                    "Expired"
                }
            ));
        }

        K::Facility => {
            let row = facts!(state.facilities.as_slice(), id);
            facility_details(&mut lines, state, catalog, row);
        }
        K::Route => {
            let row = facts!(state.routes.as_slice(), id);
            lines.push(format!(
                "Dispatch next due {}; reserve {} {}",
                row.next_due.0,
                row.source_reserve,
                row.resource.as_wire_str()
            ));
            let mut causes = Vec::new();
            if row.suspended {
                causes.push("No valid route: suspended".to_owned());
            }
            let source = state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == row.source_world);
            let destination = state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == row.destination_world);
            if source.is_none_or(|world| world.owner != Some(row.source_owner))
                || destination.is_none_or(|world| world.owner != Some(row.receiver_owner))
            {
                causes
                    .push("No valid route: endpoint owner differs from recorded permission".into());
            }
            if let Some(source) = source {
                let held = match row.resource {
                    LivingResourceV2::Energy => source.inventory.energy,
                    LivingResourceV2::Ore => source.inventory.ore,
                    LivingResourceV2::Alloy => source.inventory.alloy,
                };
                lines.push(format!(
                    "Source inventory {held}; batch {}; reserve {}",
                    row.batch_size, row.source_reserve
                ));
                if held < row.batch_size.saturating_add(row.source_reserve) {
                    causes.push("Waiting for input above the source reserve".into());
                }
            }
            if row.kind == LivingRouteKindV2::Trade
                && !state.agreements.as_slice().iter().any(|agreement| {
                    agreement.kind == LivingAgreementKindV2::Trade
                        && ((agreement.participant_a == row.source_owner
                            && agreement.participant_b == row.receiver_owner)
                            || (agreement.participant_a == row.receiver_owner
                                && agreement.participant_b == row.source_owner))
                        && agreement.start_tick <= state.tick
                        && state.tick < agreement.end_tick
                })
            {
                causes.push("No valid route: no active bilateral trade agreement".into());
            }
            if causes.is_empty() {
                lines.push("Current dispatch prerequisites satisfied; next dispatch attempt remains authoritative".into());
            } else {
                lines.extend(
                    causes
                        .into_iter()
                        .map(|cause| format!("Current dispatch cause: {cause}")),
                );
            }
        }
        K::Civilization => {
            facts!(state.civilizations.as_slice(), id);
            let id = selected.id;
            lines.push("Owned colonies".into());
            for row in state
                .colonies
                .as_slice()
                .iter()
                .filter(|row| row.owner == id)
            {
                lines.push(entity_id(row.world));
            }
            lines.push("Limited intelligence / recorded observations".into());
            for row in state
                .observations
                .as_slice()
                .iter()
                .filter(|row| row.civilization == id)
            {
                append_facts(&mut lines, serde_json::to_value(row).ok()?);
            }
            lines.push("Directed relationships".into());
            for row in state
                .relations
                .as_slice()
                .iter()
                .filter(|row| row.from == id || row.to == id)
            {
                append_facts(&mut lines, serde_json::to_value(row).ok()?);
            }
            lines.push("Agreements and truces".into());
            for row in state
                .agreements
                .as_slice()
                .iter()
                .filter(|row| row.participant_a == id || row.participant_b == id)
            {
                append_facts(&mut lines, serde_json::to_value(row).ok()?);
            }
            lines.push("Wars".into());
            for row in state
                .wars
                .as_slice()
                .iter()
                .filter(|row| row.participant_a == id || row.participant_b == id)
            {
                append_facts(&mut lines, serde_json::to_value(row).ok()?);
            }
        }
    }
    Some(lines)
}
fn append_facts(lines: &mut Vec<String>, value: serde_json::Value) {
    fn walk(lines: &mut Vec<String>, prefix: &str, value: &serde_json::Value) {
        match value {
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    let label = if prefix.is_empty() {
                        key.replace('_', " ")
                    } else {
                        format!("{prefix} / {}", key.replace('_', " "))
                    };
                    walk(lines, &label, value);
                }
            }
            serde_json::Value::Array(values) => {
                if values.is_empty() {
                    lines.push(format!("{prefix}: none"));
                } else {
                    for (index, value) in values.iter().enumerate() {
                        walk(lines, &format!("{prefix} {}", index + 1), value);
                    }
                }
            }
            serde_json::Value::String(value) => lines.push(format!("{prefix}: {value}")),
            other => lines.push(format!("{prefix}: {other}")),
        }
    }
    walk(lines, "", &value);
}
fn facility_details(
    lines: &mut Vec<String>,
    state: &LivingGalaxyStateV2,
    catalog: &ValidatedLivingCatalogPackV2,
    facility: &LivingFacilityV2,
) {
    let Some(world) = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == facility.world)
    else {
        return;
    };
    lines.push(format!(
        "Local energy {} / ore {} / alloy {}; capacity 10000 per resource",
        world.inventory.energy, world.inventory.ore, world.inventory.alloy
    ));
    lines.push(format!(
        "Last due attempt: {:?}; next due {}",
        facility.status,
        facility
            .next_due
            .map_or_else(|| "none".into(), |tick| tick.0.to_string())
    ));
    for reason in facility.blocked_reasons.as_slice() {
        lines.push(format!("Recorded blocked cause: {reason:?}"));
    }
    let mut causes = Vec::new();
    if facility.disabled {
        causes.push("Disabled".to_owned());
    }
    if let Some(definition) = catalog.industry(facility.definition.as_str()) {
        match &definition.effect {
            LivingIndustryEffectV2::Recipe {
                period_ticks,
                deposit_units,
                inputs,
                outputs,
            } => {
                lines.push(format!(
                    "Recipe every {period_ticks} ticks; consumes {deposit_units} deposit units"
                ));
                let held = |resource: &str| match resource {
                    "energy" => world.inventory.energy,
                    "ore" => world.inventory.ore,
                    "alloy" => world.inventory.alloy,
                    _ => 0,
                };
                for input in inputs.as_slice() {
                    let available = held(input.resource.as_str());
                    lines.push(format!(
                        "Input {} {} / held {available}",
                        input.quantity,
                        input.resource.as_str()
                    ));
                    if available < input.quantity {
                        causes.push(format!(
                            "Waiting for input: {} needs {}, holds {available}",
                            input.resource.as_str(),
                            input.quantity
                        ));
                    }
                }
                for output in outputs.as_slice() {
                    lines.push(format!(
                        "Output {} {}",
                        output.quantity,
                        output.resource.as_str()
                    ));
                    let consumed = inputs
                        .as_slice()
                        .iter()
                        .filter(|input| input.resource == output.resource)
                        .map(|input| input.quantity)
                        .fold(0u32, u32::saturating_add);
                    if held(output.resource.as_str())
                        .saturating_sub(consumed)
                        .saturating_add(output.quantity)
                        > 10000
                    {
                        causes.push(format!("Output full: {}", output.resource.as_str()));
                    }
                }
                if *deposit_units > 0
                    && state
                        .deposits
                        .as_slice()
                        .iter()
                        .filter(|row| row.world == world.id)
                        .map(|row| row.remaining_units)
                        .fold(0u64, u64::saturating_add)
                        < u64::from(*deposit_units)
                {
                    causes.push("Deposit depleted".into());
                }
            }
            LivingIndustryEffectV2::HullAssembly { concurrent_jobs } => {
                lines.push(format!("Hull assembly: {concurrent_jobs} concurrent jobs"))
            }
            LivingIndustryEffectV2::Defense {
                damage_per_round, ..
            } => lines.push(format!(
                "Defense: {damage_per_round} damage per combat round"
            )),
        }
    }
    if causes.is_empty() {
        let ready = match catalog
            .industry(facility.definition.as_str())
            .map(|definition| &definition.effect)
        {
            Some(LivingIndustryEffectV2::HullAssembly { .. }) => "Ready for hull orders",
            Some(LivingIndustryEffectV2::Defense { .. }) => "Operational defense",
            _ => "Running when due",
        };
        lines.push(format!("Current prerequisite check: {ready}"));
    } else {
        lines.extend(
            causes
                .into_iter()
                .map(|cause| format!("Current prerequisite cause: {cause}")),
        );
    }
}
