//! Read-only, deterministic neural scoring for campaign inspection.

pub mod gpu;

use crate::game::model::{Campaign, Faction, FieldKind, Tick, WORLD_COUNT, WorldId};

pub const MODEL_VERSION: u32 = 1;
pub const FEATURE_COUNT: usize = 12;
pub const HIDDEN_COUNT: usize = 4;
pub const SCORE_COUNT: usize = WORLD_COUNT;
pub const PACKED_FEATURE_COUNT: usize = FEATURE_COUNT * SCORE_COUNT;
pub const PACKED_WEIGHT_COUNT: usize = HIDDEN_COUNT * FEATURE_COUNT + HIDDEN_COUNT * 2 + 1;
pub const CADENCE_TICKS: u64 = 30;

const W1: [[f32; FEATURE_COUNT]; HIDDEN_COUNT] = [
    [
        -0.60, 0.55, -0.80, 0.0, 0.35, -0.20, 0.10, 0.10, -0.10, 0.20, -0.60, -0.40,
    ],
    [
        0.80, -0.80, -0.70, 0.15, 0.15, 0.05, 0.0, 0.0, 0.0, -0.40, 1.00, -0.20,
    ],
    [
        0.0, 0.0, -0.10, 0.15, 0.55, 0.25, 0.20, 0.20, 0.20, 0.0, -0.10, -0.15,
    ],
    [
        -0.20, 0.20, -0.35, 0.0, 0.0, 0.0, 0.0, 0.25, 0.0, 0.60, -0.35, -0.90,
    ],
];
const B1: [f32; HIDDEN_COUNT] = [0.50, 0.10, -0.25, 0.55];
const W2: [f32; HIDDEN_COUNT] = [0.45, 0.25, 0.20, 0.25];
const B2: f32 = 0.10;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvisoryBackend {
    Cpu,
    WebGpu,
    CpuFallback,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdvisoryMetadata {
    pub device_epoch: u64,
    pub request_id: u64,
    pub model_version: u32,
    pub source_tick: Tick,
    pub source_fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdvisorySnapshot {
    pub scores: [f32; SCORE_COUNT],
    pub priority: WorldId,
    pub metadata: AdvisoryMetadata,
    pub backend: AdvisoryBackend,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdvisoryRequest {
    pub metadata: AdvisoryMetadata,
    pub features: [f32; PACKED_FEATURE_COUNT],
    pub cpu_scores: [f32; SCORE_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvisoryTrigger {
    Initialization,
    Restart,
    MaterialEvent,
    Tick,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequestOutcome {
    pub snapshot: AdvisorySnapshot,
    pub gpu_request: Option<AdvisoryRequest>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompletionDisposition {
    Accepted,
    Stale,
    Fallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompletionOutcome {
    pub disposition: CompletionDisposition,
    pub snapshot: Option<AdvisorySnapshot>,
    pub gpu_request: Option<AdvisoryRequest>,
}

#[derive(Debug)]
pub struct AdvisoryController {
    device_epoch: u64,
    gpu_available: bool,
    gpu_disabled_epoch: Option<u64>,
    next_request_id: u64,
    latest: Option<AdvisoryMetadata>,
    latest_cpu: Option<AdvisoryRequest>,
    published: Option<AdvisorySnapshot>,
    in_flight: Option<AdvisoryRequest>,
    queued: Option<AdvisoryRequest>,
    last_cadence_tick: Option<Tick>,
}

impl AdvisoryController {
    pub fn new(device_epoch: u64, gpu_available: bool) -> Self {
        Self {
            device_epoch,
            gpu_available,
            gpu_disabled_epoch: None,
            next_request_id: 0,
            latest: None,
            latest_cpu: None,
            published: None,
            in_flight: None,
            queued: None,
            last_cadence_tick: None,
        }
    }

    pub fn set_device_epoch(&mut self, device_epoch: u64, gpu_available: bool) {
        if device_epoch != self.device_epoch {
            self.device_epoch = device_epoch;
            self.gpu_disabled_epoch = None;
            self.in_flight = None;
            self.queued = None;
        }
        self.gpu_available = gpu_available;
    }

    pub fn gpu_enabled(&self) -> bool {
        self.gpu_available && self.gpu_disabled_epoch != Some(self.device_epoch)
    }

    pub fn has_in_flight_request(&self) -> bool {
        self.in_flight.is_some()
    }

    pub fn has_queued_request(&self) -> bool {
        self.queued.is_some()
    }

    pub fn published(&self) -> Option<&AdvisorySnapshot> {
        self.published.as_ref()
    }

    pub fn request_if_due(
        &mut self,
        campaign: &Campaign,
        source_fingerprint: u64,
        trigger: AdvisoryTrigger,
    ) -> Option<RequestOutcome> {
        if trigger == AdvisoryTrigger::Tick {
            let tick = campaign.next_tick;
            if !tick.0.is_multiple_of(CADENCE_TICKS)
                || self.last_cadence_tick == Some(tick)
                || self.latest.is_some_and(|latest| latest.source_tick == tick)
            {
                return None;
            }
            self.last_cadence_tick = Some(tick);
        }

        let metadata = AdvisoryMetadata {
            device_epoch: self.device_epoch,
            request_id: self.next_request_id,
            model_version: MODEL_VERSION,
            source_tick: campaign.next_tick,
            source_fingerprint,
        };
        self.next_request_id = self.next_request_id.wrapping_add(1);
        let request = AdvisoryRequest {
            metadata,
            features: packed_features(campaign),
            cpu_scores: cpu_scores(campaign),
        };
        self.latest = Some(metadata);
        self.latest_cpu = Some(request.clone());
        let backend = if self.gpu_disabled_epoch == Some(self.device_epoch) {
            AdvisoryBackend::CpuFallback
        } else {
            AdvisoryBackend::Cpu
        };
        let snapshot = snapshot_for(&request, backend, request.cpu_scores);
        self.published = Some(snapshot.clone());

        let gpu_request = if !self.gpu_enabled() {
            None
        } else if self.in_flight.is_none() {
            self.in_flight = Some(request.clone());
            Some(request)
        } else {
            self.queued = Some(request);
            None
        };
        Some(RequestOutcome {
            snapshot,
            gpu_request,
        })
    }

    pub fn complete_gpu(
        &mut self,
        metadata: AdvisoryMetadata,
        result: Result<[f32; SCORE_COUNT], gpu::GpuAdvisoryError>,
    ) -> CompletionOutcome {
        let Some(in_flight) = self.in_flight.as_ref() else {
            return stale_outcome(None);
        };
        if in_flight.metadata != metadata {
            return stale_outcome(None);
        }
        let request = self.in_flight.take().expect("checked above");
        let scores = match result {
            Ok(scores) => scores,
            Err(_) => return self.fallback_for_epoch(request),
        };
        if !valid_scores(&scores) || !parity_matches(&request.cpu_scores, &scores) {
            return self.fallback_for_epoch(request);
        }
        if self.latest != Some(metadata)
            || metadata.device_epoch != self.device_epoch
            || metadata.model_version != MODEL_VERSION
        {
            return stale_outcome(self.take_next_gpu_request());
        }

        let snapshot = snapshot_for(&request, AdvisoryBackend::WebGpu, scores);
        self.published = Some(snapshot.clone());
        CompletionOutcome {
            disposition: CompletionDisposition::Accepted,
            snapshot: Some(snapshot),
            gpu_request: self.take_next_gpu_request(),
        }
    }

    /// Disables advisory compute for the current device epoch and republishes
    /// the newest independently computed CPU scores, even when no request is
    /// currently mapped. A same-epoch availability change cannot clear this
    /// failure; only `set_device_epoch` with a new epoch permits another try.
    pub fn fail_device_epoch(&mut self, error: gpu::GpuAdvisoryError) -> CompletionOutcome {
        log::warn!("GPU advisory disabled for device epoch: {error}");
        self.gpu_disabled_epoch = Some(self.device_epoch);
        self.in_flight = None;
        self.queued = None;
        let snapshot = self
            .latest_cpu
            .as_ref()
            .map(|request| snapshot_for(request, AdvisoryBackend::CpuFallback, request.cpu_scores));
        if let Some(snapshot) = &snapshot {
            self.published = Some(snapshot.clone());
        }
        CompletionOutcome {
            disposition: CompletionDisposition::Fallback,
            snapshot,
            gpu_request: None,
        }
    }

    fn fallback_for_epoch(&mut self, request: AdvisoryRequest) -> CompletionOutcome {
        self.gpu_disabled_epoch = Some(self.device_epoch);
        let fallback_request = self.latest_cpu.clone().unwrap_or(request);
        self.in_flight = None;
        self.queued = None;
        let snapshot = snapshot_for(
            &fallback_request,
            AdvisoryBackend::CpuFallback,
            fallback_request.cpu_scores,
        );
        self.published = Some(snapshot.clone());
        CompletionOutcome {
            disposition: CompletionDisposition::Fallback,
            snapshot: Some(snapshot),
            gpu_request: None,
        }
    }

    fn take_next_gpu_request(&mut self) -> Option<AdvisoryRequest> {
        if !self.gpu_enabled() {
            self.queued = None;
            return None;
        }
        let next = self.queued.take();
        self.in_flight = next.clone();
        next
    }
}

fn stale_outcome(gpu_request: Option<AdvisoryRequest>) -> CompletionOutcome {
    CompletionOutcome {
        disposition: CompletionDisposition::Stale,
        snapshot: None,
        gpu_request,
    }
}

fn snapshot_for(
    request: &AdvisoryRequest,
    backend: AdvisoryBackend,
    scores: [f32; SCORE_COUNT],
) -> AdvisorySnapshot {
    AdvisorySnapshot {
        priority: priority_world(&scores),
        scores,
        metadata: request.metadata,
        backend,
    }
}

pub fn features_for_world(campaign: &Campaign, world_id: WorldId) -> [f32; FEATURE_COUNT] {
    let Some(world) = campaign.worlds.get(usize::from(world_id.0)) else {
        return [0.0; FEATURE_COUNT];
    };
    let hazard_field = campaign.active_hazard.map(|hazard| hazard.affected_field);
    let adjusted_field = |field: FieldKind, level: u8| {
        let multiplier = if hazard_field == Some(field) {
            0.5
        } else {
            1.0
        };
        unit(f32::from(level) * multiplier / 10.0, 0.0)
    };
    let mut union_inbound = 0_u64;
    let mut hostile_inbound = 0_u64;
    for fleet in campaign
        .fleets
        .iter()
        .filter(|fleet| fleet.destination == world_id)
    {
        if fleet.faction == Faction::Union {
            union_inbound = union_inbound.saturating_add(fleet.strength.0);
        } else {
            hostile_inbound = hostile_inbound.saturating_add(fleet.strength.0);
        }
    }
    let min_distance = campaign
        .worlds
        .iter()
        .filter(|candidate| candidate.owner == Some(Faction::Union))
        .map(|candidate| {
            let dx = i128::from(world.position.x) - i128::from(candidate.position.x);
            let dy = i128::from(world.position.y) - i128::from(candidate.position.y);
            (dx * dx + dy * dy) as f64
        })
        .reduce(f64::min);

    [
        f32::from(world.owner == Some(Faction::Union)),
        f32::from(matches!(world.owner, Some(Faction::Helix | Faction::Choir))),
        unit(world.defense.0 as f32 / 100_000.0, 0.0),
        unit(world.energy.0 as f32 / 250_000.0, 0.0),
        unit(world.base_output_per_second.0 as f32 / 3_000.0, 0.0),
        unit(world.base_regeneration_per_second.0 as f32 / 500.0, 0.0),
        adjusted_field(FieldKind::Atmosphere, world.fields.atmosphere),
        adjusted_field(FieldKind::Hydrosphere, world.fields.hydrosphere),
        adjusted_field(FieldKind::Topology, world.fields.topology),
        unit(union_inbound as f32 / 100_000.0, 0.0),
        unit(hostile_inbound as f32 / 100_000.0, 0.0),
        unit(
            (min_distance.unwrap_or(200_000_000.0) / 200_000_000.0) as f32,
            1.0,
        ),
    ]
}

pub fn packed_features(campaign: &Campaign) -> [f32; PACKED_FEATURE_COUNT] {
    let mut packed = [0.0; PACKED_FEATURE_COUNT];
    for world_index in 0..SCORE_COUNT {
        let start = world_index * FEATURE_COUNT;
        packed[start..start + FEATURE_COUNT]
            .copy_from_slice(&features_for_world(campaign, WorldId(world_index as u8)));
    }
    packed
}

pub fn packed_weights() -> [f32; PACKED_WEIGHT_COUNT] {
    let mut packed = [0.0; PACKED_WEIGHT_COUNT];
    let mut cursor = 0;
    for row in W1 {
        packed[cursor..cursor + FEATURE_COUNT].copy_from_slice(&row);
        cursor += FEATURE_COUNT;
    }
    packed[cursor..cursor + HIDDEN_COUNT].copy_from_slice(&B1);
    cursor += HIDDEN_COUNT;
    packed[cursor..cursor + HIDDEN_COUNT].copy_from_slice(&W2);
    cursor += HIDDEN_COUNT;
    packed[cursor] = B2;
    packed
}

pub fn evaluate_features(features: [f32; FEATURE_COUNT]) -> f32 {
    let features = features.map(|value| unit(value, 0.0));
    let mut hidden = [0.0; HIDDEN_COUNT];
    for hidden_index in 0..HIDDEN_COUNT {
        let mut activation = B1[hidden_index];
        for feature_index in 0..FEATURE_COUNT {
            activation += W1[hidden_index][feature_index] * features[feature_index];
        }
        hidden[hidden_index] = activation.max(0.0);
    }
    let score = hidden
        .iter()
        .zip(W2)
        .fold(B2, |sum, (activation, weight)| sum + activation * weight);
    unit(score, 0.0)
}

pub fn cpu_scores(campaign: &Campaign) -> [f32; SCORE_COUNT] {
    std::array::from_fn(|index| {
        evaluate_features(features_for_world(campaign, WorldId(index as u8)))
    })
}

pub fn priority_world(scores: &[f32; SCORE_COUNT]) -> WorldId {
    let mut best_index = 0;
    let mut best_score = unit(scores[0], 0.0);
    for (index, score) in scores.iter().copied().enumerate().skip(1) {
        let score = unit(score, 0.0);
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    WorldId(best_index as u8)
}

pub fn parity_matches(cpu: &[f32; SCORE_COUNT], gpu: &[f32; SCORE_COUNT]) -> bool {
    cpu.iter().zip(gpu).all(|(&cpu, &gpu)| {
        cpu.is_finite()
            && gpu.is_finite()
            && (cpu - gpu).abs() <= 1.0e-5 + 5.0e-5 * cpu.abs().max(gpu.abs())
    })
}

fn valid_scores(scores: &[f32; SCORE_COUNT]) -> bool {
    scores
        .iter()
        .all(|score| score.is_finite() && (0.0..=1.0).contains(score))
}

fn unit(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}
