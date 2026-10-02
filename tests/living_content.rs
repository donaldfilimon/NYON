//! Approved content is ordinary validated genesis, with real receipt witnesses.
use nyon::living_content::LivingContent;
use nyon_workshop_core::living::{LivingEventKindV2, LivingEventProvenanceV2};

#[test]
fn vale_blank_and_seeded_have_approved_shapes_and_reproducible_identities() {
    let vale = LivingContent::ValeConfluence.open().unwrap();
    let state = vale.state();
    assert_eq!(state.systems.as_slice().len(), 4);
    assert_eq!(state.worlds.as_slice().len(), 8);
    assert_eq!(state.civilizations.as_slice().len(), 3);
    assert_eq!(state.lanes.as_slice().len(), 6);
    assert_eq!(
        state
            .worlds
            .as_slice()
            .iter()
            .filter(|world| world.owner.is_none())
            .count(),
        2
    );
    assert!(state.wars.as_slice().is_empty());
    assert_eq!(state.tick.0, 0);
    for (name, policy) in [
        ("Vale Cooperative", "trader"),
        ("Nacre Cartographers", "expansionist"),
        ("Crucible Directorate", "guardian"),
    ] {
        assert!(
            state
                .civilizations
                .as_slice()
                .iter()
                .any(|row| row.name.as_str() == name && row.policy.0.as_str() == policy)
        );
    }
    let mut blank = LivingContent::Blank.open().unwrap();
    assert!(blank.state().worlds.as_slice().is_empty());
    assert!(blank.state().systems.as_slice().is_empty());
    assert!(blank.state().civilizations.as_slice().is_empty());
    for _ in 0..500 {
        blank.step().unwrap();
    }
    for seed in [0, 1, 0x4e594f4e5f47414c, u64::MAX] {
        let first = LivingContent::Seeded(seed).open().unwrap();
        let second = LivingContent::Seeded(seed).open().unwrap();
        assert_eq!(first.state().systems.as_slice().len(), 12);
        assert_eq!(first.state().worlds.as_slice().len(), 24);
        assert_eq!(first.state().civilizations.as_slice().len(), 4);
        assert_eq!(
            first.state().digest().unwrap(),
            second.state().digest().unwrap()
        );
        assert_eq!(first.branch(), second.branch());
    }
    assert_ne!(
        LivingContent::Seeded(1)
            .open()
            .unwrap()
            .state()
            .digest()
            .unwrap(),
        LivingContent::Seeded(2)
            .open()
            .unwrap()
            .state()
            .digest()
            .unwrap()
    );
}

#[test]
fn commerce_witnesses_negotiation_and_cross_civilization_delivery_within_deadlines() {
    let mut authority = LivingContent::Commerce.open().unwrap();
    assert!(authority.state().agreements.as_slice().is_empty());
    let route = authority.state().routes.as_slice()[0].clone();
    assert_ne!(route.source_owner, route.receiver_owner);
    let mut agreement = None;
    let mut delivery = None;
    for _ in 0..200 {
        let receipt = authority.step().unwrap();
        for event in &receipt.events {
            assert!(matches!(
                event.provenance,
                LivingEventProvenanceV2::Autonomous { .. }
            ));
            match event.kind {
                LivingEventKindV2::AgreementFormed => {
                    agreement.get_or_insert(receipt.tick.0);
                }
                LivingEventKindV2::ShipmentDelivered => {
                    delivery.get_or_insert(receipt.tick.0);
                }
                _ => {}
            }
        }
    }
    assert!(
        agreement.is_some_and(|tick| tick <= 100),
        "agreement witness absent"
    );
    assert!(
        delivery.is_some_and(|tick| tick <= 200),
        "delivery witness absent"
    );
}

#[test]
fn frontier_witnesses_war_hostile_arrival_combat_and_survives_tick_500() {
    let mut authority = LivingContent::FrontierFriction.open().unwrap();
    let mut war = None;
    let mut arrival = None;
    let mut combat = None;
    for _ in 0..500 {
        let receipt = authority.step().unwrap();
        for event in receipt.events {
            match event.kind {
                LivingEventKindV2::WarDeclared => {
                    war.get_or_insert(receipt.tick.0);
                }
                LivingEventKindV2::FleetArrived
                    if !authority.state().wars.as_slice().is_empty() =>
                {
                    arrival.get_or_insert(receipt.tick.0);
                }
                LivingEventKindV2::CombatRound => {
                    combat.get_or_insert(receipt.tick.0);
                }
                _ => {}
            }
        }
    }
    assert!(war.is_some_and(|tick| tick <= 100), "war witness absent");
    assert!(
        arrival.is_some_and(|tick| tick <= 200),
        "hostile arrival witness absent"
    );
    assert!(
        combat.is_some_and(|tick| tick <= 200),
        "combat witness absent"
    );
    assert_eq!(authority.state().tick.0, 500);
}

#[test]
fn interrupted_corridor_witnesses_scheduled_hazard_reduced_freight_and_production_wait() {
    let mut authority = LivingContent::InterruptedCorridor.open().unwrap();
    let route = authority.state().routes.as_slice()[0].clone();
    let mut hazard = None;
    let mut reduced = false;
    let mut blocked = false;
    for _ in 0..200 {
        let receipt = authority.step().unwrap();
        if receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::HazardStarted)
        {
            hazard.get_or_insert(receipt.tick.0);
        }
        if receipt.tick.0 >= 100 && receipt.tick.0 < 150 {
            reduced |= authority
                .state()
                .shipments
                .as_slice()
                .iter()
                .any(|shipment| {
                    shipment.source_world == route.source_world
                        && shipment.destination_world == route.destination_world
                        && shipment.units < route.batch_size
                });
        }
        if receipt.tick.0 == 120 {
            blocked = authority
                .state()
                .facilities
                .as_slice()
                .iter()
                .any(|facility| {
                    facility.world == route.destination_world
                        && facility.definition.as_str() == "foundry"
                        && !facility.blocked_reasons.as_slice().is_empty()
                });
        }
    }
    assert!(hazard.is_some_and(|tick| tick < 50));
    assert!(reduced, "storm did not reduce dispatch");
    assert!(blocked, "downstream foundry did not wait");
}
