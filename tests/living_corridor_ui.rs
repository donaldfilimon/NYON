//! Exact current-boundary observations, not a causal experiment completion.
use nyon::{
    living_content::LivingContent,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
};
use nyon_workshop_core::living::*;

#[test]
fn actual_restricted_route_can_record_exact_portable_locators_without_commands() {
    let mut session =
        SandboxSession::from_authority(LivingContent::InterruptedCorridor.open().unwrap());
    for _ in 0..149 {
        session.step_once().unwrap();
        if !session.authority().last_corridor_restrictions().is_empty() {
            break;
        }
    }
    assert!(!session.authority().last_corridor_restrictions().is_empty());
    let before = session.bookmark().unwrap();
    let witnesses = session.authority().last_corridor_restrictions().to_vec();
    session.apply_action(A::RecordExperiment(3)).unwrap();
    assert_eq!(session.bookmark().unwrap(), before);
    let evidence = session.notebook_evidence_snapshot().unwrap();
    assert_eq!(evidence.records.len(), 1);
    let record = &evidence.records[0];
    assert_eq!(
        record.experiment,
        LivingNotebookExperimentKindV2::CorridorImpact
    );
    assert_eq!(
        record.receipt_digest,
        session.authority().last_receipt().unwrap().digest
    );
    let attempts = record.route_attempts.as_ref().unwrap();
    assert!(!attempts.is_empty());
    for attempt in attempts {
        assert!(
            witnesses
                .iter()
                .any(|proof| proof.route() == attempt.route && proof.hazard().id == attempt.hazard)
        );
    }
    assert!(
        attempts
            .windows(2)
            .all(|pair| (pair[0].route, pair[0].hazard) < (pair[1].route, pair[1].hazard))
    );
}

#[test]
fn unrelated_initial_boundary_rejects_corridor_record_without_notebook_or_authority_changes() {
    let mut session = SandboxSession::open().unwrap();
    session.step_once().unwrap();
    let before = session.bookmark().unwrap();
    let notes = session.notebook_snapshot();
    let evidence = session.notebook_evidence_snapshot();
    assert!(session.apply_action(A::RecordExperiment(3)).is_err());
    assert_eq!(session.bookmark().unwrap(), before);
    assert_eq!(session.notebook_snapshot(), notes);
    assert_eq!(session.notebook_evidence_snapshot(), evidence);
}

fn fixture() -> LivingGalaxyAuthorityV2 {
    fixture_with_energy(100)
}
fn fixture_with_energy(destination_energy: u32) -> LivingGalaxyAuthorityV2 {
    let base = builtin_first_expansion_v2().unwrap();
    let mut state = base.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let source = state.colonies.as_slice()[0].world;
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.id != source)
        .unwrap()
        .id;
    let mut worlds = state.worlds.as_slice().to_vec();
    for w in &mut worlds {
        w.owner = Some(actor);
        w.inventory = LivingInventoryV2 {
            energy: if w.id == source {
                100
            } else {
                destination_energy
            },
            ore: if w.id == source { 100 } else { 0 },
            alloy: 0,
        };
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut colony = state.colonies.as_slice()[0].clone();
    colony.world = destination;
    state.colonies =
        LivingSortedVecV2::new(vec![state.colonies.as_slice()[0].clone(), colony]).unwrap();
    let other = builtin_living_sandbox_v2().unwrap();
    let mut foundry = other
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|f| f.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    foundry.id = LivingEntityIdV2([0xa1; 16]);
    foundry.world = destination;
    foundry.next_due = Some(LivingTickV2(20));
    state.facilities = LivingSortedVecV2::new(vec![foundry]).unwrap();
    state.fleets = Default::default();
    state.routes = LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
        id: LivingEntityIdV2([0xa2; 16]),
        kind: LivingRouteKindV2::Internal,
        source_world: source,
        destination_world: destination,
        resource: LivingResourceV2::Ore,
        batch_size: 1,
        cadence_ticks: 10,
        source_reserve: 20,
        source_owner: actor,
        receiver_owner: actor,
        next_due: LivingTickV2(11),
        suspended: false,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: LivingEntityIdV2([0xa3; 16]),
        definition: LivingSlugV2::new("ion_storm").unwrap(),
        lane: state.lanes.as_slice()[0].id,
        start_tick: LivingTickV2(10),
        end_tick: LivingTickV2(150),
    }])
    .unwrap();
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: 2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa8; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("corridor_notebook_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}

#[test]
fn validated_corridor_pair_survives_install_and_links_exact_observations() {
    use glam::Vec2;
    let mut session = SandboxSession::from_authority(fixture());
    for _ in 0..11 {
        session.step_once().unwrap();
    }
    assert_eq!(
        session.authority().last_corridor_restrictions()[0].actual_dispatched_units(),
        0
    );
    session.apply_action(A::RecordExperiment(3)).unwrap();
    let attempt = session.bookmark().unwrap();
    for _ in 11..20 {
        session.step_once().unwrap();
    }
    session.apply_action(A::RecordExperiment(3)).unwrap();
    let wait = session.bookmark().unwrap();
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(session.authority()).unwrap(),
        session.authority().catalog(),
    )
    .unwrap();
    let (bookmarks, experiments) = session.notebook_snapshot();
    let notebook = LivingNotebookV2::seal_with_evidence(
        &archive,
        bookmarks,
        experiments,
        session.notebook_evidence_snapshot(),
    )
    .unwrap();
    let mut replay =
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive).unwrap();
    while replay.poll(17).unwrap() != LivingArchiveReplayStatusV2::Ready {}
    let validated = replay.finish().unwrap();
    assert_eq!(validated.corridor_observations().len(), 1);
    assert_eq!(
        validated.corridor_observations()[0].attempt_view(),
        attempt.view
    );
    assert_eq!(validated.corridor_observations()[0].wait_view(), wait.view);
    let before = session.bookmark().unwrap();
    let generation = session.notebook_generation();
    session.accept_notebook_validation(&validated).unwrap();
    assert_eq!(session.bookmark().unwrap(), before);
    assert_eq!(session.notebook_generation(), generation);
    session.install_notebook(validated).unwrap();
    session.apply_action(A::NotebookPicker).unwrap();
    session.apply_action(A::ExperimentDetails(Some(3))).unwrap();
    let mut text = String::new();
    let mut links = Vec::new();
    let mut link_actions = Vec::new();
    for _ in 0..100 {
        let frame = session.frame(Vec2::new(800., 480.), false);
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.fact.corridor")
            {
                text.push_str(&control.label);
            }
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.corridor.open")
            {
                links.push(control.label.clone());
                if let nyon::ui::platform::PlatformUiAction::Sandbox(action) = &control.action {
                    link_actions.push((control.label.clone(), *action));
                }
            }
        }
        if !frame
            .controls
            .iter()
            .any(|c| c.action_id.as_str() == "sandbox.notebook.next" && c.enabled)
        {
            break;
        }
        session.apply_action(A::InspectorPage(false)).unwrap();
    }
    assert!(
        text.contains("Restricted dispatch and matching later input wait observed"),
        "{text}"
    );
    assert!(text.contains("actually dispatched 0 units"), "{text}");
    assert!(text.contains("does not establish sole causation"), "{text}");
    assert!(
        links
            .iter()
            .any(|s| s == "Open verified corridor dispatch bookmark")
    );
    assert!(
        links
            .iter()
            .any(|s| s == "Open verified corridor input wait bookmark")
    );
    assert_eq!(session.bookmark().unwrap(), before);
    for (label, action) in link_actions {
        let expected = if label.contains("input wait") {
            wait
        } else {
            attempt
        };
        session.apply_action(action).unwrap();
        for _ in 0..1000 {
            session.update(std::time::Duration::ZERO).unwrap();
            if session.bookmark().unwrap() == expected {
                break;
            }
        }
        assert_eq!(session.bookmark().unwrap(), expected, "{label}");
    }
}
