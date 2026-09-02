use nyon::advisory::gpu::GpuAdvisoryError;
use nyon::advisory::{
    AdvisoryBackend, AdvisoryController, AdvisoryMetadata, AdvisoryTrigger, CompletionDisposition,
    MODEL_VERSION, cpu_scores, evaluate_features, features_for_world, packed_features,
    packed_weights, parity_matches, priority_world,
};
use nyon::game::model::{
    ActiveHazard, Campaign, DEFAULT_SEED, Energy, Faction, FieldKind, Fleet, FleetId, HazardKind,
    RulesV1, Strength, Tick, WorldId,
};

fn campaign() -> Campaign {
    Campaign::new(DEFAULT_SEED, RulesV1::default())
}

fn fleet(id: u64, faction: Faction, destination: u8, strength: u64) -> Fleet {
    Fleet {
        id: FleetId(id),
        faction,
        source: WorldId(6),
        destination: WorldId(destination),
        strength: Strength(strength),
        route_length: 1,
        progress: 0,
        speed_per_second: 1,
        hydrosphere_level: 0,
        movement_remainder: 0,
    }
}

#[test]
fn extracts_all_twelve_features_and_packs_seven_worlds() {
    let mut state = campaign();
    state.worlds[0].owner = Some(Faction::Union);
    state.worlds[0].defense = Strength(50_000);
    state.worlds[0].energy = Energy(125_000);
    state.worlds[0].base_output_per_second = Energy(1_500);
    state.worlds[0].base_regeneration_per_second = Strength(250);
    state.worlds[0].fields.atmosphere = 8;
    state.worlds[0].fields.hydrosphere = 6;
    state.worlds[0].fields.topology = 4;
    state.active_hazard = Some(ActiveHazard {
        event_index: 0,
        kind: HazardKind::IonStorm,
        affected_field: FieldKind::Hydrosphere,
        start: Tick(0),
        end: Tick(10),
    });
    state.fleets = vec![
        fleet(0, Faction::Union, 0, 10_000),
        fleet(1, Faction::Union, 0, 15_000),
        fleet(2, Faction::Helix, 0, 20_000),
        fleet(3, Faction::Choir, 0, 30_000),
    ];
    assert_eq!(
        features_for_world(&state, WorldId(0)),
        [1.0, 0.0, 0.5, 0.5, 0.5, 0.5, 0.8, 0.3, 0.4, 0.25, 0.5, 0.0]
    );
    assert_eq!(
        &packed_features(&state)[..12],
        &features_for_world(&state, WorldId(0))
    );
    for world in &mut state.worlds {
        if world.owner == Some(Faction::Union) {
            world.owner = None;
        }
    }
    assert_eq!(features_for_world(&state, WorldId(0))[11], 1.0);
}

#[test]
fn exact_weights_finite_clamp_parity_and_tie_break_are_stable() {
    let weights = packed_weights();
    assert_eq!(
        weights,
        [
            -0.60, 0.55, -0.80, 0.0, 0.35, -0.20, 0.10, 0.10, -0.10, 0.20, -0.60, -0.40, 0.80,
            -0.80, -0.70, 0.15, 0.15, 0.05, 0.0, 0.0, 0.0, -0.40, 1.00, -0.20, 0.0, 0.0, -0.10,
            0.15, 0.55, 0.25, 0.20, 0.20, 0.20, 0.0, -0.10, -0.15, -0.20, 0.20, -0.35, 0.0, 0.0,
            0.0, 0.0, 0.25, 0.0, 0.60, -0.35, -0.90, 0.50, 0.10, -0.25, 0.55, 0.45, 0.25, 0.20,
            0.25, 0.10,
        ]
    );
    assert!(evaluate_features([f32::NAN; 12]).is_finite());
    assert!((0.0..=1.0).contains(&evaluate_features([f32::INFINITY; 12])));
    assert_eq!(priority_world(&[0.5; 7]), WorldId(0));
    assert!(parity_matches(&[0.5; 7], &[0.500_02; 7]));
    assert!(!parity_matches(&[0.5; 7], &[0.6; 7]));
}

#[test]
fn factory_default_score_bits_are_a_reviewed_golden() {
    let bits = cpu_scores(&campaign()).map(f32::to_bits);
    assert_eq!(
        bits,
        [
            1_058_173_887,
            1_060_329_379,
            1_059_226_841,
            1_054_688_186,
            1_056_209_868,
            1_054_665_716,
            1_057_420_253,
        ]
    );
}

#[test]
fn controller_schedules_coalesces_and_accepts_only_the_newest_request() {
    let mut state = campaign();
    let mut controller = AdvisoryController::new(7, true);
    let first = controller
        .request_if_due(&state, 10, AdvisoryTrigger::Initialization)
        .unwrap();
    assert_eq!(first.snapshot.backend, AdvisoryBackend::Cpu);
    let first_gpu = first.gpu_request.unwrap();
    assert!(
        controller
            .request_if_due(&state, 10, AdvisoryTrigger::Tick)
            .is_none()
    );
    state.next_tick = Tick(1);
    assert!(
        controller
            .request_if_due(&state, 11, AdvisoryTrigger::MaterialEvent)
            .unwrap()
            .gpu_request
            .is_none()
    );
    assert!(controller.has_queued_request());
    assert!(
        controller
            .request_if_due(&state, 11, AdvisoryTrigger::Tick)
            .is_none()
    );
    state.next_tick = Tick(30);
    controller
        .request_if_due(&state, 12, AdvisoryTrigger::Tick)
        .unwrap();
    let stale = controller.complete_gpu(first_gpu.metadata, Ok(first_gpu.cpu_scores));
    assert_eq!(stale.disposition, CompletionDisposition::Stale);
    let newest = stale.gpu_request.unwrap();
    assert_eq!(newest.metadata.source_tick, Tick(30));
    assert_eq!(newest.metadata.source_fingerprint, 12);
    let accepted = controller.complete_gpu(newest.metadata, Ok(newest.cpu_scores));
    assert_eq!(accepted.disposition, CompletionDisposition::Accepted);
    assert_eq!(accepted.snapshot.unwrap().backend, AdvisoryBackend::WebGpu);
}

#[test]
fn stale_metadata_is_dropped_and_failure_disables_only_current_epoch() {
    let mut state = campaign();
    let mut controller = AdvisoryController::new(4, true);
    let request = controller
        .request_if_due(&state, 20, AdvisoryTrigger::Restart)
        .unwrap()
        .gpu_request
        .unwrap();
    let base = request.metadata;
    let mut stale_metadata = [base; 5];
    stale_metadata[0].device_epoch += 1;
    stale_metadata[1].request_id += 1;
    stale_metadata[2].model_version += 1;
    stale_metadata[3].source_tick.0 += 1;
    stale_metadata[4].source_fingerprint += 1;
    for wrong in stale_metadata {
        assert_eq!(
            controller
                .complete_gpu(wrong, Ok(request.cpu_scores))
                .disposition,
            CompletionDisposition::Stale
        );
    }
    assert!(controller.has_in_flight_request());
    let failed = controller.complete_gpu(request.metadata, Ok([1.0; 7]));
    assert_eq!(failed.disposition, CompletionDisposition::Fallback);
    assert_eq!(
        failed.snapshot.unwrap().backend,
        AdvisoryBackend::CpuFallback
    );
    assert!(!controller.gpu_enabled());
    state.next_tick = Tick(1);
    assert!(
        controller
            .request_if_due(&state, 21, AdvisoryTrigger::MaterialEvent)
            .unwrap()
            .gpu_request
            .is_none()
    );
    controller.set_device_epoch(5, true);
    assert!(controller.gpu_enabled());
    let restarted = controller
        .request_if_due(&state, 22, AdvisoryTrigger::Restart)
        .unwrap();
    assert_eq!(restarted.gpu_request.unwrap().metadata.device_epoch, 5);
}

#[test]
fn gpu_runtime_error_falls_back_to_the_newest_coalesced_cpu_result() {
    let mut state = campaign();
    let mut controller = AdvisoryController::new(9, true);
    let first = controller
        .request_if_due(&state, 30, AdvisoryTrigger::Initialization)
        .unwrap()
        .gpu_request
        .unwrap();
    state.next_tick = Tick(1);
    controller
        .request_if_due(&state, 31, AdvisoryTrigger::MaterialEvent)
        .unwrap();
    let outcome = controller.complete_gpu(
        first.metadata,
        Err(GpuAdvisoryError::Map("injected readback failure".into())),
    );
    assert_eq!(outcome.disposition, CompletionDisposition::Fallback);
    let fallback = outcome.snapshot.unwrap();
    assert_eq!(fallback.backend, AdvisoryBackend::CpuFallback);
    assert_eq!(fallback.metadata.source_tick, Tick(1));
    assert_eq!(fallback.metadata.source_fingerprint, 31);
    assert!(!controller.gpu_enabled());
}

#[test]
fn matching_faulty_old_completion_disables_epoch_before_newer_stale_drop() {
    for faulty_scores in [[f32::NAN; 7], [1.0; 7]] {
        let mut state = campaign();
        let mut controller = AdvisoryController::new(10, true);
        let oldest = controller
            .request_if_due(&state, 60, AdvisoryTrigger::Initialization)
            .unwrap()
            .gpu_request
            .unwrap();
        state.next_tick = Tick(1);
        let newest = controller
            .request_if_due(&state, 61, AdvisoryTrigger::MaterialEvent)
            .unwrap()
            .snapshot;

        let outcome = controller.complete_gpu(oldest.metadata, Ok(faulty_scores));
        assert_eq!(outcome.disposition, CompletionDisposition::Fallback);
        assert!(outcome.gpu_request.is_none());
        let fallback = outcome.snapshot.unwrap();
        assert_eq!(fallback.backend, AdvisoryBackend::CpuFallback);
        assert_eq!(fallback.metadata, newest.metadata);
        assert_eq!(fallback.scores, newest.scores);
        assert!(!controller.has_in_flight_request());
        assert!(!controller.has_queued_request());
        assert!(!controller.gpu_enabled());
    }
}

#[test]
fn device_failure_without_in_flight_work_republishes_retained_cpu_for_the_epoch() {
    let state = campaign();
    let mut controller = AdvisoryController::new(12, false);
    let cpu = controller
        .request_if_due(&state, 40, AdvisoryTrigger::Initialization)
        .unwrap()
        .snapshot;
    let failure = controller.fail_device_epoch(GpuAdvisoryError::Device("device lost".into()));
    assert_eq!(failure.disposition, CompletionDisposition::Fallback);
    let fallback = failure.snapshot.unwrap();
    assert_eq!(fallback.backend, AdvisoryBackend::CpuFallback);
    assert_eq!(fallback.metadata, cpu.metadata);
    assert_eq!(fallback.scores, cpu.scores);
    assert!(!controller.has_in_flight_request());
    assert!(!controller.has_queued_request());

    controller.set_device_epoch(12, true);
    assert!(
        !controller.gpu_enabled(),
        "same-epoch availability cannot reverse a device failure"
    );
}

#[test]
fn device_failure_discards_gpu_work_but_new_epoch_can_retry_newest_cpu_snapshot() {
    let mut state = campaign();
    let mut controller = AdvisoryController::new(20, true);
    controller
        .request_if_due(&state, 50, AdvisoryTrigger::Initialization)
        .unwrap();
    state.next_tick = Tick(1);
    let newest = controller
        .request_if_due(&state, 51, AdvisoryTrigger::MaterialEvent)
        .unwrap()
        .snapshot;
    assert!(controller.has_in_flight_request());
    assert!(controller.has_queued_request());

    let failure = controller.fail_device_epoch(GpuAdvisoryError::Device("queue lost".into()));
    let fallback = failure.snapshot.unwrap();
    assert_eq!(fallback.backend, AdvisoryBackend::CpuFallback);
    assert_eq!(fallback.metadata, newest.metadata);
    assert_eq!(fallback.scores, newest.scores);
    assert!(!controller.has_in_flight_request());
    assert!(!controller.has_queued_request());
    assert!(!controller.gpu_enabled());

    controller.set_device_epoch(21, true);
    assert!(controller.gpu_enabled());
    let retry = controller
        .request_if_due(&state, 51, AdvisoryTrigger::Restart)
        .unwrap()
        .gpu_request
        .unwrap();
    assert_eq!(retry.metadata.device_epoch, 21);
}

#[test]
fn failed_epoch_keeps_cpu_fallback_visible_until_a_new_epoch() {
    let mut state = campaign();
    let mut controller = AdvisoryController::new(30, true);
    controller
        .request_if_due(&state, 70, AdvisoryTrigger::Initialization)
        .unwrap();
    controller.fail_device_epoch(GpuAdvisoryError::Device("device removed".into()));

    state.next_tick = Tick(1);
    let same_epoch = controller
        .request_if_due(&state, 71, AdvisoryTrigger::MaterialEvent)
        .unwrap();
    assert_eq!(same_epoch.snapshot.backend, AdvisoryBackend::CpuFallback);
    assert!(same_epoch.gpu_request.is_none());
    assert!(!controller.gpu_enabled());

    controller.set_device_epoch(31, true);
    let new_epoch = controller
        .request_if_due(&state, 71, AdvisoryTrigger::Restart)
        .unwrap();
    assert_eq!(new_epoch.snapshot.backend, AdvisoryBackend::Cpu);
    assert!(new_epoch.gpu_request.is_some());
    assert!(controller.gpu_enabled());
    assert!(controller.has_in_flight_request());
}

#[test]
fn advisory_source_has_no_simulation_authority_type() {
    let source = concat!(
        include_str!("../src/advisory/mod.rs"),
        include_str!("../src/advisory/gpu.rs")
    );
    let forbidden = ["Game", "Command"].concat();
    assert!(!source.contains(&forbidden));
    let metadata = AdvisoryMetadata {
        device_epoch: 1,
        request_id: 2,
        model_version: MODEL_VERSION,
        source_tick: Tick(3),
        source_fingerprint: 4,
    };
    assert_eq!(metadata.model_version, 1);
}
