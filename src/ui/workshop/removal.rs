//! Removal confirmation model and the authoritative blocker scan, split out of
//! `workshop.rs`.

use std::collections::BTreeSet;

use nyon_workshop_core::{CreatorBatchV1, CreatorOpV1, EntityId, ObjectRefV1, WorkshopStateV1};

use crate::workshop::session::{WorkshopAction, WorkshopSessionSnapshot};

use super::{
    RemovalBlocker, RemovalConfirmation, WorkshopControl, WorkshopUiIntent, entity_display_name,
};

pub(super) fn build_removal_confirmation(
    snapshot: &WorkshopSessionSnapshot,
    target: EntityId,
) -> RemovalConfirmation {
    let blockers = removal_blockers(&snapshot.state, target)
        .into_iter()
        .map(|entity| RemovalBlocker {
            entity,
            label: entity_display_name(&snapshot.state, entity),
        })
        .collect::<Vec<_>>();
    let can_remove = blockers.is_empty();
    RemovalConfirmation {
        target,
        target_label: entity_display_name(&snapshot.state, target),
        blockers,
        cancel_control: WorkshopControl::new(
            "remove.cancel",
            "Cancel removal",
            "Close this confirmation without changing the Workshop.",
            true,
            false,
            WorkshopUiIntent::CloseRemovalConfirmation,
        ),
        confirm_control: WorkshopControl::new(
            "remove.confirm",
            "Remove object",
            if can_remove {
                "Submit one explicit non-cascading removal batch."
            } else {
                "Remove the listed dependencies before removing this object."
            },
            can_remove,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Submit(CreatorBatchV1 {
                expected_cursor: snapshot.active_view.view_cursor,
                expected_tick: snapshot.active_view.tick,
                operations: vec![CreatorOpV1::RemoveObject {
                    target: ObjectRefV1::Existing(target),
                }],
            })),
        ),
    }
}

pub fn removal_blockers(state: &WorkshopStateV1, target: EntityId) -> Vec<EntityId> {
    let mut found = BTreeSet::new();
    if state.factions.contains_key(&target) {
        found.extend(
            state
                .owners
                .iter()
                .filter_map(|(entity, owner)| (*owner == target).then_some(*entity)),
        );
    }
    for (id, star) in &state.stars {
        if star.system == target {
            found.insert(*id);
        }
    }
    for (id, world) in &state.worlds {
        if world.system == target || world.primary == target {
            found.insert(*id);
        }
    }
    for (id, lane) in &state.lanes {
        if lane.a == target || lane.b == target {
            found.insert(*id);
        }
    }
    for (id, deposit) in &state.deposits {
        if deposit.world == target {
            found.insert(*id);
        }
    }
    for (id, industry) in &state.industries {
        if industry.world == target || industry.linked_deposit == Some(target) {
            found.insert(*id);
        }
    }
    for (id, route) in &state.routes {
        if route.source == target || route.destination == target {
            found.insert(*id);
        }
    }
    if let Some(lane) = state.lanes.get(&target) {
        for (id, route) in &state.routes {
            let Some(source) = state.worlds.get(&route.source) else {
                continue;
            };
            let Some(destination) = state.worlds.get(&route.destination) else {
                continue;
            };
            if (source.system == lane.a && destination.system == lane.b)
                || (source.system == lane.b && destination.system == lane.a)
            {
                found.insert(*id);
            }
        }
    }
    for (id, shipment) in &state.shipments {
        if shipment.route == target || shipment.source == target || shipment.destination == target {
            found.insert(*id);
        }
    }
    for (id, hazard) in &state.hazards {
        if hazard.lane == target {
            found.insert(*id);
        }
    }
    found.into_iter().collect()
}
