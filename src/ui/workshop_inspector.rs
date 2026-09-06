//! Catalog-backed, presentation-only Workshop inspector derivation.

use nyon_workshop_core::{
    CatalogId, EntityId, ValidatedCatalogPackV1, WorkshopStateV1, model::EntityKind,
    pack::QuantityV1,
};

use super::{
    accessibility::SemanticNodeId,
    workshop::{
        InspectorModel, InspectorRow, InspectorSection, WorkshopControl, WorkshopUiIntent,
        entity_display_name, fixed_hex, hazard_status, is_ownable, is_removable, short_hex,
    },
};

pub(super) fn build_inspector(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorModel {
    let title = selected
        .map(|entity| {
            state.industries.get(&entity).map_or_else(
                || entity_display_name(state, entity),
                |industry| {
                    format!(
                        "{} {}",
                        catalog
                            .and_then(|pack| pack.industry(&industry.definition_id))
                            .map_or(industry.definition_id.as_str(), |definition| definition
                                .name()),
                        short_hex(&entity.0)
                    )
                },
            )
        })
        .unwrap_or_else(|| "Galaxy overview".to_owned());
    let mut sections = vec![
        status_section(state, selected, catalog),
        inventory_section(state, selected, catalog),
        deposit_section(state, selected, catalog),
        industry_section(state, selected, catalog),
        route_section(state, selected, catalog),
        shipment_section(state, selected, catalog),
        hazard_section(state, selected, catalog),
    ];
    assign_inspector_semantic_ids(&mut sections, selected);
    let remove_control = selected
        .filter(|entity| is_removable(state.entity_kind(*entity)))
        .map(|entity| {
            WorkshopControl::new(
                format!("remove.open.{}", fixed_hex(&entity.0)),
                "Review removal",
                "Show every dependency that must be removed first.",
                true,
                false,
                WorkshopUiIntent::OpenRemovalConfirmation(entity),
            )
        });
    InspectorModel {
        selected,
        title,
        sections,
        remove_control,
    }
}

fn status_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let mut rows = vec![row("Authoritative tick", state.tick.0)];
    if let Some(entity) = selected {
        rows.push(row("Entity ID", fixed_hex(&entity.0)));
        let kind = state.entity_kind(entity);
        rows.push(row(
            "Kind",
            kind.map_or("Unknown".to_owned(), |value| format!("{value:?}")),
        ));
        if is_ownable(kind) {
            rows.push(row("Owner", owner_name(state, entity)));
        }
        match kind {
            Some(EntityKind::Faction) => {
                let faction = &state.factions[&entity];
                rows.extend([
                    row("Name", &faction.name),
                    row(
                        "Color",
                        format!(
                            "#{:02X}{:02X}{:02X}",
                            faction.color_rgb[0], faction.color_rgb[1], faction.color_rgb[2]
                        ),
                    ),
                    row(
                        "Owned objects",
                        state
                            .owners
                            .values()
                            .filter(|owner| **owner == entity)
                            .count(),
                    ),
                ]);
            }
            Some(EntityKind::System) => {
                let system = &state.systems[&entity];
                rows.extend([
                    row("Name", &system.name),
                    row(
                        "Position",
                        format!("{}, {}", system.position.x.get(), system.position.y.get()),
                    ),
                    row(
                        "Stars",
                        state
                            .stars
                            .values()
                            .filter(|star| star.system == entity)
                            .count(),
                    ),
                    row(
                        "Worlds",
                        state
                            .worlds
                            .values()
                            .filter(|world| world.system == entity)
                            .count(),
                    ),
                    row(
                        "Connected lanes",
                        state
                            .lanes
                            .values()
                            .filter(|lane| lane.a == entity || lane.b == entity)
                            .count(),
                    ),
                ]);
            }
            Some(EntityKind::Star) => {
                let star = &state.stars[&entity];
                let (archetype, detail) = catalog
                    .and_then(|pack| pack.star_archetype_metadata(&star.archetype_id))
                    .unwrap_or((star.archetype_id.as_str(), "Catalog metadata unavailable"));
                rows.extend([
                    row("Name", &star.name),
                    row("System", entity_display_name(state, star.system)),
                    row("Archetype", archetype),
                    row("Archetype ID", &star.archetype_id),
                    row("Catalog detail", detail),
                ]);
            }
            Some(EntityKind::World) => {
                let world = &state.worlds[&entity];
                let (archetype, detail) = catalog
                    .and_then(|pack| pack.world_archetype_metadata(&world.archetype_id))
                    .unwrap_or((world.archetype_id.as_str(), "Catalog metadata unavailable"));
                rows.extend([
                    row("Name", &world.name),
                    row("System", entity_display_name(state, world.system)),
                    row("Primary", entity_display_name(state, world.primary)),
                    row("Archetype", archetype),
                    row("Archetype ID", &world.archetype_id),
                    row("Catalog detail", detail),
                    row(
                        "Orbit radius",
                        format!("{} milli-AU", world.orbit_radius_milli_au),
                    ),
                    row(
                        "Orbit period",
                        format!("{} ticks", world.orbit_period_ticks),
                    ),
                    row(
                        "Orbit phase",
                        format!("{} millidegrees", world.phase_millidegrees),
                    ),
                ]);
            }
            Some(EntityKind::Lane) => {
                let lane = &state.lanes[&entity];
                rows.extend([
                    row(
                        "Endpoints",
                        format!(
                            "{} to {}",
                            entity_display_name(state, lane.a),
                            entity_display_name(state, lane.b)
                        ),
                    ),
                    row("Distance", format!("{} units", lane.distance_units)),
                    row(
                        "Hazards",
                        state
                            .hazards
                            .values()
                            .filter(|hazard| hazard.lane == entity)
                            .count(),
                    ),
                ]);
            }
            Some(EntityKind::Deposit) => {
                let deposit = &state.deposits[&entity];
                let (resource, detail) = resource_metadata(catalog, &deposit.resource_id);
                rows.extend([
                    row("World", entity_display_name(state, deposit.world)),
                    row("Resource", resource),
                    row("Resource ID", &deposit.resource_id),
                    row("Catalog detail", detail),
                    row("Reserve", format!("{} units", deposit.reserve_units)),
                ]);
            }
            Some(EntityKind::Industry) => {
                let industry = &state.industries[&entity];
                let definition = catalog.and_then(|pack| pack.industry(&industry.definition_id));
                rows.extend([
                    row("World", entity_display_name(state, industry.world)),
                    row(
                        "Definition",
                        definition.map_or(industry.definition_id.as_str(), |value| value.name()),
                    ),
                    row("Definition ID", &industry.definition_id),
                    row(
                        "Catalog detail",
                        definition
                            .map_or("Catalog metadata unavailable", |value| value.description()),
                    ),
                    row(
                        "Industry type",
                        definition
                            .map_or("Unknown".to_owned(), |value| format!("{:?}", value.kind())),
                    ),
                    row(
                        "Duration",
                        definition.map_or("Unknown".to_owned(), |value| {
                            format!("{} ticks", value.duration_ticks())
                        }),
                    ),
                    row(
                        "Inputs",
                        definition.map_or("Unknown".to_owned(), |value| {
                            format_quantities(catalog, value.inputs())
                        }),
                    ),
                    row(
                        "Outputs",
                        definition.map_or("Unknown".to_owned(), |value| {
                            format_quantities(catalog, value.outputs())
                        }),
                    ),
                    row(
                        "Linked deposit",
                        industry
                            .linked_deposit
                            .map_or("None".to_owned(), |deposit| {
                                entity_display_name(state, deposit)
                            }),
                    ),
                    row(
                        "Operation",
                        industry_operation_status(state, industry, definition, catalog),
                    ),
                ]);
            }
            Some(EntityKind::Route) => {
                let route = &state.routes[&entity];
                let (resource, detail) = resource_metadata(catalog, &route.resource_id);
                let active = active_route_hazards(state, entity, catalog);
                let effective = effective_route_capacity(route.batch_units, &active);
                rows.extend([
                    row("Source", entity_display_name(state, route.source)),
                    row("Destination", entity_display_name(state, route.destination)),
                    row("Resource", resource.clone()),
                    row("Resource ID", &route.resource_id),
                    row("Catalog detail", detail),
                    row(
                        "Batch capacity",
                        format!("{} {resource} per dispatch", route.batch_units),
                    ),
                    row(
                        "Effective capacity",
                        format!("{effective} {resource} per dispatch"),
                    ),
                    row(
                        "Active hazards",
                        if active.is_empty() {
                            "None".to_owned()
                        } else {
                            active
                                .iter()
                                .map(|hazard| hazard.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        },
                    ),
                    row(
                        "In-flight shipments",
                        state
                            .shipments
                            .values()
                            .filter(|shipment| shipment.route == entity)
                            .count(),
                    ),
                ]);
            }
            Some(EntityKind::Shipment) => {
                let shipment = &state.shipments[&entity];
                let (resource, _) = resource_metadata(catalog, &shipment.resource_id);
                rows.extend([
                    row("Route", entity_display_name(state, shipment.route)),
                    row("Source", entity_display_name(state, shipment.source)),
                    row(
                        "Destination",
                        entity_display_name(state, shipment.destination),
                    ),
                    row("Resource", resource),
                    row("Resource ID", &shipment.resource_id),
                    row("Units", shipment.units),
                    row("Dispatched", format!("tick {}", shipment.dispatched_tick.0)),
                    row("Arrival", format!("tick {}", shipment.arrival_tick.0)),
                    row(
                        "Ticks remaining",
                        shipment.arrival_tick.0.saturating_sub(state.tick.0),
                    ),
                ]);
            }
            Some(EntityKind::Hazard) => {
                let hazard = &state.hazards[&entity];
                let definition = catalog.and_then(|pack| pack.hazard(&hazard.hazard_id));
                let end = hazard.start_tick.0.saturating_add(hazard.duration_ticks);
                let ratio = definition.map_or("Unknown".to_owned(), |value| {
                    let (numerator, denominator) = value.route_capacity_ratio();
                    format!("{numerator}/{denominator} of normal")
                });
                rows.extend([
                    row("Lane", entity_display_name(state, hazard.lane)),
                    row(
                        "Hazard",
                        definition.map_or(hazard.hazard_id.as_str(), |value| value.name()),
                    ),
                    row("Hazard ID", &hazard.hazard_id),
                    row(
                        "Catalog detail",
                        definition
                            .map_or("Catalog metadata unavailable", |value| value.description()),
                    ),
                    row("Start", format!("tick {}", hazard.start_tick.0)),
                    row("Duration", format!("{} ticks", hazard.duration_ticks)),
                    row("End", format!("tick {end}")),
                    row("Status", hazard_status(hazard, state.tick.0)),
                    row("Route capacity", ratio),
                ]);
            }
            None => {}
        }
    } else {
        if let Some(catalog) = catalog {
            rows.extend([
                row("Catalog", catalog.title()),
                row("Catalog ID", catalog.pack_id()),
                row("Catalog version", catalog.pack_version()),
            ]);
        }
        rows.extend([
            row("Factions", state.factions.len()),
            row("Systems", state.systems.len()),
            row("Stars", state.stars.len()),
            row("Worlds", state.worlds.len()),
            row("Lanes", state.lanes.len()),
            row("Deposits", state.deposits.len()),
            row("Industries", state.industries.len()),
            row("Routes", state.routes.len()),
            row("Shipments", state.shipments.len()),
            row("Hazards", state.hazards.len()),
        ]);
    }
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Status",
        rows,
    }
}

fn inventory_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let rows = selected
        .and_then(|entity| state.worlds.get(&entity))
        .map(|world| {
            world
                .inventory
                .iter()
                .map(|(resource, units)| {
                    row_with_key(
                        format!("resource.{}", resource.as_str()),
                        resource_metadata(catalog, resource).0,
                        *units,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Inventory",
        rows,
    }
}

fn deposit_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let rows = state
        .deposits
        .iter()
        .filter(|(id, deposit)| {
            selected == Some(**id) || selected == Some(deposit.world) || selected.is_none()
        })
        .map(|(id, deposit)| {
            row_with_key(
                format!("entity.{}", fixed_hex(&id.0)),
                format!(
                    "{} {}",
                    resource_metadata(catalog, &deposit.resource_id).0,
                    short_hex(&id.0)
                ),
                format!("{} reserve units", deposit.reserve_units),
            )
        })
        .collect();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Deposits",
        rows,
    }
}

fn industry_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let rows = state
        .industries
        .iter()
        .filter(|(id, industry)| {
            selected == Some(**id) || selected == Some(industry.world) || selected.is_none()
        })
        .map(|(id, industry)| {
            row_with_key(
                format!("entity.{}", fixed_hex(&id.0)),
                format!(
                    "{} {}",
                    catalog
                        .and_then(|pack| pack.industry(&industry.definition_id))
                        .map_or(industry.definition_id.as_str(), |definition| definition
                            .name()),
                    short_hex(&id.0)
                ),
                if industry.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                },
            )
        })
        .collect();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Industries",
        rows,
    }
}

fn route_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let selected_kind = selected.and_then(|entity| state.entity_kind(entity));
    let rows = state
        .routes
        .iter()
        .filter(|(id, route)| match (selected, selected_kind) {
            (None, _) => true,
            (Some(entity), Some(EntityKind::Route)) => **id == entity,
            (Some(entity), Some(EntityKind::World)) => {
                route.source == entity || route.destination == entity
            }
            (Some(entity), Some(EntityKind::System)) => {
                state
                    .worlds
                    .get(&route.source)
                    .is_some_and(|world| world.system == entity)
                    || state
                        .worlds
                        .get(&route.destination)
                        .is_some_and(|world| world.system == entity)
            }
            _ => false,
        })
        .map(|(id, route)| {
            row_with_key(
                format!("entity.{}", fixed_hex(&id.0)),
                format!(
                    "{} {}",
                    resource_metadata(catalog, &route.resource_id).0,
                    short_hex(&id.0)
                ),
                format!(
                    "{} to {}; {} units",
                    entity_display_name(state, route.source),
                    entity_display_name(state, route.destination),
                    route.batch_units
                ),
            )
        })
        .collect();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Routes",
        rows,
    }
}

fn shipment_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let rows = state
        .shipments
        .iter()
        .filter(|(id, shipment)| {
            selected.is_none()
                || selected == Some(**id)
                || selected == Some(shipment.route)
                || selected == Some(shipment.source)
                || selected == Some(shipment.destination)
        })
        .map(|(id, shipment)| {
            row_with_key(
                format!("entity.{}", fixed_hex(&id.0)),
                format!(
                    "{} {}",
                    resource_metadata(catalog, &shipment.resource_id).0,
                    short_hex(&id.0)
                ),
                format!(
                    "{} units; tick {} to {}",
                    shipment.units, shipment.dispatched_tick.0, shipment.arrival_tick.0
                ),
            )
        })
        .collect();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Shipments",
        rows,
    }
}

fn hazard_section(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> InspectorSection {
    let rows = state
        .hazards
        .iter()
        .filter(|(id, hazard)| {
            selected.is_none() || selected == Some(**id) || selected == Some(hazard.lane)
        })
        .map(|(id, hazard)| {
            row_with_key(
                format!("entity.{}", fixed_hex(&id.0)),
                format!(
                    "{} {}",
                    catalog
                        .and_then(|pack| pack.hazard(&hazard.hazard_id))
                        .map_or(hazard.hazard_id.as_str(), |definition| definition.name()),
                    short_hex(&id.0)
                ),
                hazard_status(hazard, state.tick.0),
            )
        })
        .collect();
    InspectorSection {
        semantic_id: SemanticNodeId::new("inspector.section.pending"),
        heading: "Hazards",
        rows,
    }
}

fn row(label: impl Into<String>, value: impl ToString) -> InspectorRow {
    let label = label.into();
    row_with_key(semantic_slug(&label), label, value)
}

fn row_with_key(
    stable_key: impl Into<String>,
    label: impl Into<String>,
    value: impl ToString,
) -> InspectorRow {
    InspectorRow {
        semantic_id: SemanticNodeId::new("inspector.row.pending"),
        label: label.into(),
        value: value.to_string(),
        stable_key: stable_key.into(),
    }
}

fn assign_inspector_semantic_ids(sections: &mut [InspectorSection], selected: Option<EntityId>) {
    let owner = selected.map_or_else(
        || "overview".to_owned(),
        |entity| format!("entity.{}", fixed_hex(&entity.0)),
    );
    for section in sections {
        let section_slug = semantic_slug(section.heading);
        section.semantic_id = SemanticNodeId::new(format!("inspector.section.{section_slug}"));
        for row in &mut section.rows {
            row.semantic_id = SemanticNodeId::new(format!(
                "inspector.{owner}.{section_slug}.{}",
                row.stable_key
            ));
        }
    }
}

fn semantic_slug(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut separator = false;
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(char::from(byte.to_ascii_lowercase()));
            separator = false;
        } else {
            separator = true;
        }
    }
    if slug.is_empty() {
        "item".to_owned()
    } else {
        slug
    }
}

fn owner_name(state: &WorkshopStateV1, entity: EntityId) -> String {
    state
        .owners
        .get(&entity)
        .map_or("Unowned".to_owned(), |owner| {
            entity_display_name(state, *owner)
        })
}

fn resource_metadata<'a>(
    catalog: Option<&'a ValidatedCatalogPackV1>,
    resource: &'a CatalogId,
) -> (String, String) {
    catalog
        .and_then(|pack| pack.resource_metadata(resource))
        .map_or_else(
            || {
                (
                    resource.to_string(),
                    "Catalog metadata unavailable".to_owned(),
                )
            },
            |(name, description)| (name.to_owned(), description.to_owned()),
        )
}

fn format_quantities(
    catalog: Option<&ValidatedCatalogPackV1>,
    quantities: &[QuantityV1],
) -> String {
    if quantities.is_empty() {
        return "None".to_owned();
    }
    quantities
        .iter()
        .map(|quantity| {
            format!(
                "{} {}",
                quantity.quantity,
                resource_metadata(catalog, &quantity.resource_id).0
            )
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

fn industry_operation_status(
    state: &WorkshopStateV1,
    industry: &nyon_workshop_core::model::IndustryV1,
    definition: Option<&nyon_workshop_core::pack::IndustryDefinitionV1>,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> String {
    if !industry.enabled {
        return "Disabled".to_owned();
    }
    let Some(definition) = definition else {
        return "Enabled; catalog definition unavailable".to_owned();
    };
    let Some(world) = state.worlds.get(&industry.world) else {
        return "Enabled; world unavailable".to_owned();
    };
    let missing = definition
        .inputs()
        .iter()
        .filter_map(|input| {
            let available = world
                .inventory
                .get(&input.resource_id)
                .copied()
                .unwrap_or(0);
            (available < input.quantity).then(|| {
                format!(
                    "{} {}",
                    input.quantity - available,
                    resource_metadata(catalog, &input.resource_id).0
                )
            })
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return format!("Enabled; waiting for {}", missing.join(" + "));
    }
    if matches!(
        definition.kind(),
        nyon_workshop_core::CatalogDefinitionKind::Extractor
    ) {
        let reserve = industry
            .linked_deposit
            .and_then(|deposit| state.deposits.get(&deposit))
            .map(|deposit| deposit.reserve_units);
        if reserve == Some(0) {
            return "Enabled; linked deposit depleted".to_owned();
        }
        if reserve.is_none() {
            return "Enabled; linked deposit unavailable".to_owned();
        }
    }
    let duration = definition.duration_ticks();
    let next_tick = if state.tick.0.is_multiple_of(duration) {
        state.tick.0
    } else {
        state
            .tick
            .0
            .saturating_add(duration - state.tick.0 % duration)
    };
    format!("Enabled; ready at tick {next_tick}")
}

struct ActiveRouteHazard {
    name: String,
    numerator: u64,
    denominator: u64,
}

fn active_route_hazards(
    state: &WorkshopStateV1,
    route_id: EntityId,
    catalog: Option<&ValidatedCatalogPackV1>,
) -> Vec<ActiveRouteHazard> {
    let Some(route) = state.routes.get(&route_id) else {
        return Vec::new();
    };
    let Some(source) = state.worlds.get(&route.source) else {
        return Vec::new();
    };
    let Some(destination) = state.worlds.get(&route.destination) else {
        return Vec::new();
    };
    if source.system == destination.system {
        return Vec::new();
    }
    let lane = state.lanes.iter().find_map(|(id, lane)| {
        ((lane.a == source.system && lane.b == destination.system)
            || (lane.a == destination.system && lane.b == source.system))
            .then_some(*id)
    });
    let Some(lane) = lane else {
        return Vec::new();
    };
    state
        .hazards
        .values()
        .filter(|hazard| {
            hazard.lane == lane
                && !hazard.cancelled
                && hazard.start_tick.0 <= state.tick.0
                && state.tick.0 < hazard.start_tick.0.saturating_add(hazard.duration_ticks)
        })
        .filter_map(|hazard| {
            let definition = catalog?.hazard(&hazard.hazard_id)?;
            let (numerator, denominator) = definition.route_capacity_ratio();
            Some(ActiveRouteHazard {
                name: definition.name().to_owned(),
                numerator,
                denominator,
            })
        })
        .collect()
}

fn effective_route_capacity(base: u64, hazards: &[ActiveRouteHazard]) -> u64 {
    hazards.iter().fold(base, |capacity, hazard| {
        u64::try_from(
            u128::from(capacity) * u128::from(hazard.numerator) / u128::from(hazard.denominator),
        )
        .unwrap_or(u64::MAX)
    })
}
