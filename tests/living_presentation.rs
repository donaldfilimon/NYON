//! Projection never changes authority and supports arbitrary entity counts.
use glam::Vec2;
use nyon::{
    engine::primitives::PrimitiveBatch, presentation::living::LivingSceneFrame,
    ui::platform::PlatformRect,
};
use nyon_workshop_core::living::builtin_living_sandbox_v2;

#[test]
fn living_projection_preserves_digest_and_draws_every_world_and_lane() {
    let authority = builtin_living_sandbox_v2().unwrap();
    let state = authority.state();
    let digest = state.digest().unwrap();
    for (width, height) in [(390.0, 844.0), (1280.0, 720.0), (1920.0, 1080.0)] {
        let bounds = PlatformRect::from_xywh(20.0, 60.0, width - 40.0, height - 120.0);
        let frame = LivingSceneFrame::extract(state, bounds);
        assert_eq!(frame.worlds.len(), state.worlds.as_slice().len());
        assert_eq!(frame.systems.len(), state.systems.as_slice().len());
        assert_eq!(frame.lanes.len(), state.lanes.as_slice().len());
        for world in &frame.worlds {
            assert!(bounds.contains(world.position));
            assert!(world.position.is_finite());
        }
        for high_contrast in [false, true] {
            let mut batch = PrimitiveBatch::default();
            frame.draw(&mut batch, high_contrast);
            assert!(!batch.vertices().is_empty());
            assert!(
                batch
                    .vertices()
                    .iter()
                    .all(|vertex| vertex.position.iter().all(|value| value.is_finite()))
            );
        }
        assert_eq!(state.digest().unwrap(), digest);
    }
}

#[test]
fn empty_and_single_system_projection_are_finite_and_pickable() {
    let authority = builtin_living_sandbox_v2().unwrap();
    let mut state = authority.state().clone();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 320.0, 300.0);
    let first = state.systems.as_slice()[0].clone();
    state.systems =
        nyon_workshop_core::living::LivingSortedVecV2::new(vec![first.clone()]).unwrap();
    state.worlds = nyon_workshop_core::living::LivingSortedVecV2::new(
        state
            .worlds
            .as_slice()
            .iter()
            .filter(|world| world.system == first.id)
            .cloned()
            .collect(),
    )
    .unwrap();
    let frame = LivingSceneFrame::extract(&state, bounds);
    assert_eq!(frame.systems.len(), 1);
    assert!(frame.systems[0].is_finite());
    for world in &frame.worlds {
        assert_eq!(frame.hit_world(world.position), Some(world.id));
    }
    state.systems = Default::default();
    state.worlds = Default::default();
    let empty = LivingSceneFrame::extract(&state, bounds);
    assert!(empty.worlds.is_empty());
    assert!(empty.lanes.is_empty());
    assert_eq!(empty.hit_world(Vec2::ZERO), None);
}

#[test]
fn topology_changes_do_not_reframe_existing_worlds() {
    let authority = builtin_living_sandbox_v2().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 800.0, 600.0);
    let before = LivingSceneFrame::extract(authority.state(), bounds);
    let mut state = authority.state().clone();
    let mut added = state.systems.as_slice()[0].clone();
    added.id = nyon_workshop_core::living::LivingEntityIdV2([255; 16]);
    let mut systems = state.systems.as_slice().to_vec();
    systems.push(added);
    systems.sort_by_key(|system| system.id);
    state.systems = nyon_workshop_core::living::LivingSortedVecV2::new(systems).unwrap();
    let after = LivingSceneFrame::extract(&state, bounds);
    assert!(!after.worlds.is_empty());
    for world in &after.worlds {
        assert_eq!(
            world.position,
            before
                .worlds
                .iter()
                .find(|prior| prior.id == world.id)
                .unwrap()
                .position
        );
    }
}

#[test]
fn living_navigation_changes_only_the_camera_and_retains_pickable_selection() {
    use nyon::sandbox_session::{SandboxSession, SandboxUiAction};
    let mut session = SandboxSession::open().unwrap();
    let bookmark = session.bookmark().unwrap();
    let selected = session.selected_world().unwrap();
    let bounds = PlatformRect::from_xywh(250.0, 120.0, 600.0, 500.0);
    let galaxy = session.scene_frame(bounds);
    session
        .apply_action(SandboxUiAction::NavigateWorld)
        .unwrap();
    let world = session.scene_frame(bounds);
    assert_ne!(galaxy, world);
    assert_eq!(
        world.hit_world((bounds.min + bounds.max) * 0.5),
        Some(selected)
    );
    assert!(world.worlds.iter().all(|row| bounds.contains(row.position)));
    session
        .apply_action(SandboxUiAction::NavigateSystem)
        .unwrap();
    let system_frame = session.scene_frame(bounds);
    let system = session.camera_scope().entity.unwrap();
    assert!(system_frame.entities.iter().any(|marker| {
        marker.selection.kind == nyon::living_navigation::LivingEntityKind::System
            && marker.selection.id == system
            && marker.position.distance((bounds.min + bounds.max) * 0.5) < 0.01
    }));
    session
        .apply_action(SandboxUiAction::NavigateGalaxy)
        .unwrap();
    assert_eq!(session.scene_frame(bounds), galaxy);
    assert_eq!(session.bookmark().unwrap(), bookmark);
}

#[test]
fn fleet_and_freight_selection_uses_drawn_centers_before_and_after_focus() {
    use nyon::{living_content::LivingContent, living_navigation::LivingEntityKind as K};
    let mut authority = LivingContent::Commerce.open().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let digest = authority.state().digest().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    let mut frame = LivingSceneFrame::extract(authority.state(), bounds);
    for (kind, offset, points) in [
        (K::Fleet, Vec2::new(0.0, -18.0), frame.fleets.clone()),
        (K::Shipment, Vec2::new(0.0, 18.0), frame.freight.clone()),
    ] {
        assert!(!points.is_empty());
        for point in points {
            assert_eq!(frame.hit_entity(point + offset).unwrap().kind, kind);
        }
    }
    let world = authority.state().worlds.as_slice()[0].id;
    frame.focus(bounds, world, 3.0);
    for (kind, offset, points) in [
        (K::Fleet, Vec2::new(0.0, -18.0), frame.fleets.clone()),
        (K::Shipment, Vec2::new(0.0, 18.0), frame.freight.clone()),
    ] {
        for point in points {
            assert_eq!(frame.hit_entity(point + offset).unwrap().kind, kind);
        }
    }
    assert_eq!(authority.state().digest().unwrap(), digest);
}

#[test]
fn stylized_world_surfaces_are_distinct_stable_and_independent_of_ownership() {
    use nyon::presentation::living::LivingWorldAppearance as A;
    let authority = builtin_living_sandbox_v2().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    let digest = authority.state().digest().unwrap();
    let original = LivingSceneFrame::extract(authority.state(), bounds);
    let mut signatures = Vec::new();
    for (slug, appearance) in [
        ("temperate", A::Temperate),
        ("barren", A::Arid),
        ("ice", A::Ice),
        ("gas_giant", A::Gas),
        ("unknown", A::Rocky),
    ] {
        assert_eq!(A::for_archetype(slug), appearance);
        let mut frame = original.clone();
        frame.worlds.truncate(1);
        frame.entities.clear();
        frame.systems.clear();
        frame.lanes.clear();
        frame.worlds[0].owner = None;
        frame.worlds[0].completed_buildings = 0;
        frame.worlds[0].appearance = appearance;
        let seed = frame.worlds[0].surface_seed;
        let mut batch = PrimitiveBatch::default();
        frame.draw(&mut batch, false);
        assert!(batch.vertices().iter().all(|vertex| {
            vertex
                .position
                .iter()
                .chain(vertex.color.iter())
                .all(|value| value.is_finite())
        }));
        signatures.push(batch.vertices().to_vec());
        frame.worlds[0].owner = authority
            .state()
            .civilizations
            .as_slice()
            .first()
            .map(|row| row.id);
        frame.worlds[0].color = [1.0, 0.5, 0.2, 1.0];
        let mut owned = PrimitiveBatch::default();
        frame.draw(&mut owned, false);
        assert_eq!(
            &owned.vertices()[..batch.vertices().len()],
            batch.vertices()
        );
        assert_eq!(frame.worlds[0].surface_seed, seed);
    }
    for (index, left) in signatures.iter().enumerate() {
        for right in signatures.iter().skip(index + 1) {
            assert_ne!(left, right);
        }
    }
    assert_eq!(authority.state().digest().unwrap(), digest);
}

#[test]
fn visual_scope_and_contrast_rendering_preserve_full_identity_picking() {
    use nyon::{
        living_content::LivingContent,
        sandbox_session::{SandboxSession, SandboxUiAction},
    };
    let mut session = SandboxSession::from_authority(LivingContent::Commerce.open().unwrap());
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    let digest = session.state().digest().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    for action in [
        SandboxUiAction::NavigateGalaxy,
        SandboxUiAction::NavigateSystem,
        SandboxUiAction::NavigateWorld,
    ] {
        session.apply_action(action).unwrap();
        let frame = session.scene_frame(bounds);
        for contrast in [false, true] {
            let labels = frame.labels();
            assert_eq!(labels, frame.labels());
            if let Some(selected) = frame.selected_entity
                && frame
                    .entities
                    .iter()
                    .any(|marker| marker.visible && marker.selection == selected)
            {
                assert_eq!(labels.first().unwrap().0, selected);
            }
            let mut batch = PrimitiveBatch::default();
            frame.draw(&mut batch, contrast);
            assert!(!batch.vertices().is_empty());
            assert!(batch.vertices().iter().all(|vertex| {
                vertex
                    .position
                    .iter()
                    .chain(vertex.color.iter())
                    .all(|value| value.is_finite())
            }));
            let mut repeated = PrimitiveBatch::default();
            frame.draw(&mut repeated, contrast);
            assert_eq!(batch.vertices(), repeated.vertices()); // No wall-clock animation or hidden authority interpolation.
            for marker in frame.entities.iter().filter(|marker| marker.visible) {
                let picked = frame.hit_entity(marker.position).unwrap();
                let expected = frame
                    .entities
                    .iter()
                    .filter(|candidate| candidate.visible && candidate.position == marker.position)
                    .map(|candidate| candidate.selection)
                    .min()
                    .unwrap();
                assert_eq!(picked, expected);
            }
        }
    }
    assert_eq!(session.state().digest().unwrap(), digest);
}

#[test]
fn returning_freight_projects_the_actual_reverse_leg_and_waits_at_source() {
    use nyon::{living_content::LivingContent, living_navigation::LivingEntityKind};
    use nyon_workshop_core::living::{
        LivingShipmentDispositionV2 as D, LivingSortedVecV2, LivingTickV2,
    };
    let mut authority = LivingContent::Commerce.open().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let digest = authority.state().digest().unwrap();
    let mut state = authority.state().clone();
    let mut shipments = state.shipments.as_slice().to_vec();
    let manifest = &mut shipments[0];
    manifest.disposition = D::Returning;
    manifest.departure_tick = LivingTickV2(0);
    manifest.arrival_tick = LivingTickV2(400);
    let id = manifest.id;
    let source = manifest.source_world;
    let destination = manifest.destination_world;
    state.shipments = LivingSortedVecV2::new(shipments).unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    let frame = LivingSceneFrame::extract(&state, bounds);
    let from = frame
        .worlds
        .iter()
        .find(|world| world.id == source)
        .unwrap()
        .position;
    let to = frame
        .worlds
        .iter()
        .find(|world| world.id == destination)
        .unwrap()
        .position;
    let cargo = frame
        .entities
        .iter()
        .find(|row| row.selection.id == id && row.selection.kind == LivingEntityKind::Shipment)
        .unwrap();
    assert!(
        cargo
            .position
            .distance(to.lerp(from, 0.25) + Vec2::new(0.0, 18.0))
            < 0.01
    );
    assert!(cargo.direction.dot((from - to).normalize_or_zero()) > 0.99);
    let visual = cargo.freight_visual.unwrap();
    assert_eq!(visual.progress_percent, 25);
    assert_eq!(visual.eta_ticks, 300);
    assert!(!visual.waiting_for_storage);
    state.tick = LivingTickV2(400);
    let mut shipments = state.shipments.as_slice().to_vec();
    shipments[0].disposition = D::WaitingForStorage;
    shipments[0].returning_to_source = true;
    state.shipments = LivingSortedVecV2::new(shipments).unwrap();
    let waiting = LivingSceneFrame::extract(&state, bounds);
    let cargo = waiting
        .entities
        .iter()
        .find(|row| row.selection.id == id && row.selection.kind == LivingEntityKind::Shipment)
        .unwrap();
    assert!(cargo.position.distance(from + Vec2::new(0.0, 18.0)) < 0.01);
    let visual = cargo.freight_visual.unwrap();
    assert_eq!(visual.progress_percent, 100);
    assert_eq!(visual.eta_ticks, 0);
    assert!(visual.waiting_for_storage);
    assert_eq!(authority.state().digest().unwrap(), digest);
}

#[test]
fn freight_resource_shapes_and_selected_progress_use_manifest_fields() {
    use nyon::{living_content::LivingContent, living_navigation::LivingEntityKind};
    use nyon_workshop_core::living::LivingResourceV2 as R;
    let mut authority = LivingContent::Commerce.open().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let digest = authority.state().digest().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    let original = LivingSceneFrame::extract(authority.state(), bounds);
    let manifest = authority.state().shipments.as_slice().first().unwrap();
    let mut cargo = original
        .entities
        .iter()
        .find(|row| {
            row.selection.id == manifest.id && row.selection.kind == LivingEntityKind::Shipment
        })
        .unwrap()
        .clone();
    let visual = cargo.freight_visual.unwrap();
    assert_eq!(visual.resource, manifest.resource);
    assert_eq!(visual.units, manifest.units);
    assert_eq!(
        visual.eta_ticks,
        manifest
            .arrival_tick
            .0
            .saturating_sub(authority.state().tick.0)
    );
    let mut frame = original.clone();
    frame.worlds.clear();
    frame.systems.clear();
    frame.lanes.clear();
    frame.entities.clear();
    frame.selected_entity = Some(cargo.selection);
    let mut glyphs = Vec::new();
    for resource in [R::Alloy, R::Energy, R::Ore] {
        cargo.freight_visual.as_mut().unwrap().resource = resource;
        frame.entities = vec![cargo.clone()];
        let mut batch = PrimitiveBatch::default();
        frame.draw(&mut batch, true);
        glyphs.push(batch.vertices().to_vec());
        assert_eq!(frame.hit_entity(cargo.position), Some(cargo.selection));
        let label = &frame.labels()[0].2;
        assert!(label.contains(resource.as_wire_str()));
        assert!(label.contains(&format!("ETA {} ticks", visual.eta_ticks)));
        assert!(label.contains(&format!("{}%", visual.progress_percent)));
    }
    for (index, glyph) in glyphs.iter().enumerate() {
        for other in glyphs.iter().skip(index + 1) {
            assert_ne!(glyph, other);
        }
    }
    assert_eq!(authority.state().digest().unwrap(), digest);
}
