//! Pure fixed-value lifecycle; no authority or device ownership.
const m = @import("model.zig");
const t = @import("classic_advisory_types.zig");
const f = @import("classic_advisory_features.zig");
const neural = @import("classic_advisory_model.zig");
pub const Backend = enum { Cpu, WebGpu, CpuFallback };
pub const Trigger = enum { Initialization, Restart, MaterialEvent, Tick };
pub const Disposition = enum { Accepted, Stale, Fallback };
pub const DeviceError = error{ DeviceFailure, SubmitFailure, PollFailure, MapFailure };
pub const Metadata = struct {
    device_epoch: u64,
    request_id: u64,
    model_version: u32,
    source_tick: m.Tick,
    source_fingerprint: u64,
    pub fn eql(a: Metadata, b: Metadata) bool {
        return a.device_epoch == b.device_epoch and a.request_id == b.request_id and a.model_version == b.model_version and a.source_tick == b.source_tick and a.source_fingerprint == b.source_fingerprint;
    }
};
pub const Request = struct { metadata: Metadata, features: [84]f32, cpu_scores: [7]f32 };
pub const Snapshot = struct { scores: [7]f32, priority: m.WorldId, metadata: Metadata, backend: Backend };
pub const RequestOutcome = struct { snapshot: Snapshot, gpu_request: ?Request };
pub const CompletionOutcome = struct { disposition: Disposition, snapshot: ?Snapshot = null, gpu_request: ?Request = null };
pub const Controller = struct {
    device_epoch: u64,
    gpu_available: bool,
    disabled_epoch: ?u64 = null,
    next_request_id: u64 = 0,
    latest: ?Metadata = null,
    latest_cpu: ?Request = null,
    publication: ?Snapshot = null,
    in_flight: ?Request = null,
    queued: ?Request = null,
    last_cadence_tick: ?m.Tick = null,
    pub fn init(epoch: u64, available: bool) Controller {
        return .{ .device_epoch = epoch, .gpu_available = available };
    }
    pub fn setDeviceEpoch(self: *Controller, epoch: u64, available: bool) void {
        if (epoch != self.device_epoch) {
            self.device_epoch = epoch;
            self.disabled_epoch = null;
            self.in_flight = null;
            self.queued = null;
        }
        self.gpu_available = available;
    }
    pub fn gpuEnabled(self: *const Controller) bool {
        return self.gpu_available and self.disabled_epoch != self.device_epoch;
    }
    pub fn hasInFlightRequest(self: *const Controller) bool {
        return self.in_flight != null;
    }
    pub fn hasQueuedRequest(self: *const Controller) bool {
        return self.queued != null;
    }
    /// Borrow expires on owner mutation, move or destruction. Copy to retain.
    pub fn published(self: *const Controller) ?*const Snapshot {
        return if (self.publication) |*s| s else null;
    }
    pub fn requestIfDue(self: *Controller, observation: t.Observation, fingerprint: u64, trigger: Trigger) ?RequestOutcome {
        if (trigger == .Tick) {
            const tick = observation.next_tick;
            if (tick % t.CADENCE_TICKS != 0 or self.last_cadence_tick == tick or (if (self.latest) |latest| latest.source_tick == tick else false)) return null;
            self.last_cadence_tick = tick;
        }
        const metadata: Metadata = .{ .device_epoch = self.device_epoch, .request_id = self.next_request_id, .model_version = t.MODEL_VERSION, .source_tick = observation.next_tick, .source_fingerprint = fingerprint };
        self.next_request_id +%= 1;
        const request: Request = .{ .metadata = metadata, .features = f.packedFeatures(observation), .cpu_scores = neural.cpuScores(observation) };
        self.latest = metadata;
        self.latest_cpu = request;
        const snapshot = snapshotFor(request, if (self.disabled_epoch == self.device_epoch) .CpuFallback else .Cpu, request.cpu_scores);
        self.publication = snapshot;
        var dispatch: ?Request = null;
        if (self.gpuEnabled()) {
            if (self.in_flight == null) {
                self.in_flight = request;
                dispatch = request;
            } else self.queued = request;
        }
        return .{ .snapshot = snapshot, .gpu_request = dispatch };
    }
    pub fn completeGpu(self: *Controller, metadata: Metadata, result: DeviceError![7]f32) CompletionOutcome {
        const flight = self.in_flight orelse return .{ .disposition = .Stale };
        if (!flight.metadata.eql(metadata)) return .{ .disposition = .Stale };
        self.in_flight = null;
        const scores = result catch return self.fallbackForEpoch(flight);
        if (!neural.scoresValid(scores) or !neural.parityMatches(flight.cpu_scores, scores)) return self.fallbackForEpoch(flight);
        if (!(if (self.latest) |latest| latest.eql(metadata) else false) or metadata.device_epoch != self.device_epoch or metadata.model_version != t.MODEL_VERSION) return .{ .disposition = .Stale, .gpu_request = self.takeNextGpuRequest() };
        const snapshot = snapshotFor(flight, .WebGpu, scores);
        self.publication = snapshot;
        return .{ .disposition = .Accepted, .snapshot = snapshot, .gpu_request = self.takeNextGpuRequest() };
    }
    pub fn failDeviceEpoch(self: *Controller, device_error: DeviceError) CompletionOutcome {
        const error_name = @errorName(device_error);
        _ = error_name;
        self.disabled_epoch = self.device_epoch;
        self.in_flight = null;
        self.queued = null;
        const snapshot = if (self.latest_cpu) |request| snapshotFor(request, .CpuFallback, request.cpu_scores) else null;
        if (snapshot) |value| self.publication = value;
        return .{ .disposition = .Fallback, .snapshot = snapshot };
    }
    fn fallbackForEpoch(self: *Controller, taken: Request) CompletionOutcome {
        self.disabled_epoch = self.device_epoch;
        const request = self.latest_cpu orelse taken;
        self.in_flight = null;
        self.queued = null;
        const snapshot = snapshotFor(request, .CpuFallback, request.cpu_scores);
        self.publication = snapshot;
        return .{ .disposition = .Fallback, .snapshot = snapshot };
    }
    fn takeNextGpuRequest(self: *Controller) ?Request {
        if (!self.gpuEnabled()) {
            self.queued = null;
            return null;
        }
        const next = self.queued;
        self.queued = null;
        self.in_flight = next;
        return next;
    }
};
fn snapshotFor(request: Request, backend: Backend, scores: [7]f32) Snapshot {
    return .{ .scores = scores, .priority = neural.priorityWorld(scores), .metadata = request.metadata, .backend = backend };
}
