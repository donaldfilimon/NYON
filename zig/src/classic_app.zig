//! Headless Classic authority, camera and effective presentation composition.
//! Platform hosts remain separate. No report allocation escapes advance.
const std = @import("std");
const m = @import("model.zig");
const scenario = @import("scenario.zig");
const sim = @import("simulation.zig");
const prefs = @import("preferences.zig");
const advisory = @import("classic_advisory_controller.zig");
const observation = @import("classic_advisory_types.zig");
const camera_model = @import("classic_camera.zig");
const math = @import("classic_math.zig");
const scene = @import("classic_scene.zig");
const picking = @import("classic_picking.zig");
const FixedClock = @import("classic_clock.zig").FixedClock;

pub const EnqueueError = error{ModeRefused} || sim.CommandRejection || std.mem.Allocator.Error;
pub const Enqueued = struct { sequence: m.CommandSequence, preview_valid: bool };

pub const MAX_STEPS_PER_PASS: usize = 15;
pub const GameSpeed = enum {
    paused,
    normal,
    fast,
    very_fast,

    pub fn multiplier(self: GameSpeed) u8 {
        return switch (self) {
            .paused => 0,
            .normal => 1,
            .fast => 2,
            .very_fast => 4,
        };
    }
    pub fn faster(self: GameSpeed) GameSpeed {
        return switch (self) {
            .paused => .normal,
            .normal => .fast,
            .fast, .very_fast => .very_fast,
        };
    }
    pub fn slower(self: GameSpeed) GameSpeed {
        return switch (self) {
            .paused, .normal => .paused,
            .fast => .normal,
            .very_fast => .fast,
        };
    }
};
pub const AppMode = enum { playing, editing_scenario, settings };
pub const SpeedSelection = enum { changed, unchanged, onboarding_rejected };
pub const TickFailure = struct { cause: std.mem.Allocator.Error, tick: m.Tick };
pub const FrameStatus = union(enum) {
    advanced,
    paused,
    mode_hold,
    onboarding_hold,
    failure_hold: TickFailure,
    gameplay_finished,
};
pub const TickSummary = struct {
    tick: m.Tick,
    accepted_commands: usize,
    event_count: usize,
    pub fn material(self: TickSummary) bool {
        return self.accepted_commands != 0 or self.event_count != 0;
    }
};
pub const FrameOutcome = struct {
    status: FrameStatus,
    completed_steps: usize = 0,
    material_steps: usize = 0,
    accepted_commands: usize = 0,
    event_count: usize = 0,
    summaries: [MAX_STEPS_PER_PASS * 4]TickSummary = undefined,

    /// Borrows inline value summaries, never TickReport memory.
    pub fn ticks(self: *const FrameOutcome) []const TickSummary {
        return self.summaries[0..self.completed_steps];
    }
};

/// Deeply read-only projection: fleet storage is borrowed until the next
/// mutation/restart/deinit, while worlds and all other fields are value copies.
pub const CampaignView = struct {
    rules_version: u32,
    generator_version: u32,
    seed: u64,
    next_tick: m.Tick,
    phase: m.Phase,
    worlds: [m.WORLD_COUNT]m.World,
    fleets: []const m.Fleet,
    active_hazard: ?m.ActiveHazard,
    next_hazard_index: u64,
    next_fleet_id: m.FleetId,
};

/// Move-only by convention, like Simulation: do not copy a live owning session
/// and deinit both copies. Underscore fields are internal; use the projections.
pub const AppCore = struct {
    _viewport: math.Vec2 = .{ .x = 1440, .y = 900 },
    _camera: camera_model.CameraState = camera_model.CameraState.init(.{ .x = 1440, .y = 900 }),
    _presentation: prefs.PresentationPreferences = .{},
    _advisory: advisory.Controller = advisory.Controller.init(0, false),
    _pending_gpu_request: ?advisory.Request = null,
    _allocator: std.mem.Allocator,
    _active_scenario: scenario.ScenarioV1,
    _simulation: sim.Simulation,
    _preferences: prefs.UserPreferencesV1,
    _clock: FixedClock,
    _mode: AppMode = .playing,
    _speed: GameSpeed,
    _resume_speed: GameSpeed = .normal,
    _onboarding_hold: bool,
    _failure: ?TickFailure = null,
    _next_sequence: m.CommandSequence = 0,
    selected_world: ?m.WorldId = null,
    hovered_world: ?m.WorldId = null,
    touch_command_target: ?m.WorldId = null,
    selected_field: m.FieldKind = .atmosphere,
    help_visible: bool = false,
    visual_time: f32 = 0,

    pub fn init(allocator: std.mem.Allocator, candidate: *const scenario.ScenarioV1, initial_preferences: prefs.UserPreferencesV1) scenario.AdmissionError!AppCore {
        const admitted = try scenario.admit(candidate);
        const simulation = try sim.Simulation.fromScenario(allocator, &admitted);
        var core: AppCore = .{
            ._allocator = allocator,
            ._active_scenario = admitted,
            ._simulation = simulation,
            ._preferences = initial_preferences,
            ._presentation = initial_preferences.presentation(),
            ._clock = FixedClock.init(admitted.rules.tick_hz, MAX_STEPS_PER_PASS) catch unreachable,
            ._speed = if (initial_preferences.onboarding_completed) .normal else .paused,
            ._onboarding_hold = !initial_preferences.onboarding_completed,
        };
        core._camera.setMotion(core._presentation.motion);
        core.scheduleAdvisory(.Initialization);
        return core;
    }

    pub fn viewport(self: *const AppCore) math.Vec2 {
        return self._viewport;
    }
    pub fn setViewport(self: *AppCore, value: math.Vec2) void {
        if (!value.isFinite()) return;
        self._viewport = .{ .x = @max(value.x, 0), .y = @max(value.y, 0) };
        self._camera.setViewport(self._viewport);
    }
    pub fn zoomCamera(self: *AppCore, value: f32) void {
        self._camera.zoom(value);
    }
    /// Synchronous extraction consumes fleet/snapshot borrows here. Returned
    /// populated prefixes and matrices are owned values.
    pub fn sceneFrame(self: *const AppCore) scene.SceneFrame {
        const value = self.campaign();
        const advice = if (self.advisorySnapshot()) |snapshot| scene.SceneAdvisory.fromSnapshot(snapshot) else null;
        return scene.buildScene(.{ .worlds = value.worlds, .fleets = value.fleets, .active_hazard = value.active_hazard }, self.activeScenario().fingerprint, self.clockAlpha(), .{ .selected_world = self.selected_world, .hovered_world = self.hovered_world }, advice, self.visual_time, &self._camera, self._presentation);
    }
    pub fn pickSceneWorld(self: *const AppCore, pointer: math.Vec2, hud_consumed: bool) ?m.WorldId {
        const frame = self.sceneFrame();
        const ray = picking.Ray.fromLogicalPointer(pointer, self._viewport, self._camera.current_matrices.inverse_view_projection) orelse return null;
        var worlds: [m.WORLD_COUNT]picking.PickWorld = undefined;
        for (frame.worlds, 0..) |world, i| worlds[i] = .{ .id = world.id, .position = world.position, .radius = world.radius };
        return if (picking.pickAfterHud(hud_consumed, ray, &worlds)) |hit| hit.world else null;
    }

    pub fn resetCamera(self: *AppCore) void {
        self._camera.reset();
    }
    pub fn presentation(self: *const AppCore) prefs.PresentationPreferences {
        return self._presentation;
    }
    pub fn camera(self: *const AppCore) camera_model.CameraState {
        return self._camera;
    }
    pub fn orbitCamera(self: *AppCore, yaw: f32, pitch: f32) void {
        self._camera.orbit(yaw, pitch);
    }
    pub fn setPresentationPreferences(self: *AppCore, value: prefs.PresentationPreferences) void {
        self._presentation = value;
        self._camera.setMotion(value.motion);
    }

    /// Borrow expires on mutation, move or destruction; copy to retain.
    pub fn advisorySnapshot(self: *const AppCore) ?*const advisory.Snapshot {
        return self._advisory.published();
    }
    pub fn advisoryUpdating(self: *const AppCore) bool {
        return self._advisory.hasInFlightRequest() or self._advisory.hasQueuedRequest();
    }
    pub fn advisorySourceTick(self: *const AppCore) m.Tick {
        return if (self.advisorySnapshot()) |snapshot| snapshot.metadata.source_tick else 0;
    }
    pub fn setGpuEpoch(self: *AppCore, epoch: u64, available: bool) void {
        self._pending_gpu_request = null;
        self._advisory.setDeviceEpoch(epoch, available);
        self.scheduleAdvisory(.Restart);
    }
    pub fn takeGpuRequest(self: *AppCore) ?advisory.Request {
        const request = self._pending_gpu_request;
        self._pending_gpu_request = null;
        return request;
    }
    pub fn completeGpu(self: *AppCore, metadata: advisory.Metadata, result: advisory.DeviceError![7]f32) advisory.CompletionOutcome {
        const outcome = self._advisory.completeGpu(metadata, result);
        if (outcome.gpu_request) |request| self._pending_gpu_request = request;
        return outcome;
    }
    pub fn failDeviceEpoch(self: *AppCore, failure: advisory.DeviceError) advisory.CompletionOutcome {
        self._pending_gpu_request = null;
        return self._advisory.failDeviceEpoch(failure);
    }
    fn scheduleAdvisory(self: *AppCore, trigger: advisory.Trigger) void {
        const state = &self._simulation.state;
        const value: observation.Observation = .{ .next_tick = state.next_tick, .worlds = state.worlds, .fleets = state.fleets.items, .active_hazard = state.active_hazard };
        if (self._advisory.requestIfDue(value, self.canonicalFingerprint(), trigger)) |outcome| {
            if (outcome.gpu_request) |request| self._pending_gpu_request = request;
        }
    }

    pub fn deinit(self: *AppCore) void {
        self._simulation.deinit();
        self.* = undefined;
    }
    pub fn activeScenario(self: *const AppCore) *const scenario.ScenarioV1 {
        return &self._active_scenario;
    }
    pub fn campaign(self: *const AppCore) CampaignView {
        const state = &self._simulation.state;
        return .{ .rules_version = state.rules_version, .generator_version = state.generator_version, .seed = state.seed, .next_tick = state.next_tick, .phase = state.phase, .worlds = state.worlds, .fleets = state.fleets.items, .active_hazard = state.active_hazard, .next_hazard_index = state.next_hazard_index, .next_fleet_id = state.next_fleet_id };
    }
    pub fn rules(self: *const AppCore) m.RulesV1 {
        return self._simulation.rules;
    }
    pub fn canonicalFingerprint(self: *const AppCore) u64 {
        return self._simulation.canonicalFingerprint();
    }
    pub fn replayIdentity(self: *const AppCore) ?scenario.ReplayIdentity {
        return self._simulation.replayIdentity();
    }
    pub fn mode(self: *const AppCore) AppMode {
        return self._mode;
    }
    pub fn speed(self: *const AppCore) GameSpeed {
        return self._speed;
    }
    pub fn resumeSpeed(self: *const AppCore) GameSpeed {
        return self._resume_speed;
    }
    pub fn nextSequence(self: *const AppCore) m.CommandSequence {
        return self._next_sequence;
    }
    pub fn previewCommand(self: *const AppCore, command: m.GameCommand) sim.CommandRejection!sim.CommandPreview {
        return self._simulation.previewCommand(command);
    }
    pub fn pendingCommandCount(self: *const AppCore) usize {
        return self._simulation.pending.items.len;
    }
    /// Admission consumes sequence only after authority append succeeds.
    /// Gameplay-invalid previews still queue for canonical tick rejection.
    pub fn enqueueGameCommand(self: *AppCore, command: m.GameCommand) EnqueueError!Enqueued {
        if (self._mode != .playing) return error.ModeRefused;
        const preview_valid = if (self.previewCommand(command)) |_| true else |_| false;
        const sequence = self._next_sequence;
        try self._simulation.enqueueNext(sequence, command);
        self._next_sequence +%= 1;
        return .{ .sequence = sequence, .preview_valid = preview_valid };
    }
    pub fn preferences(self: *const AppCore) prefs.UserPreferencesV1 {
        return self._preferences;
    }
    /// Durable settings only. Onboarding completion must preserve the effective
    /// presentation owner; settings application calls the distinct setter.
    pub fn setPreferences(self: *AppCore, value: prefs.UserPreferencesV1) void {
        self._preferences = value;
    }
    pub fn onboardingHeld(self: *const AppCore) bool {
        return self._onboarding_hold;
    }
    pub fn recoverableStatus(self: *const AppCore) ?TickFailure {
        return self._failure;
    }
    pub fn clockAlpha(self: *const AppCore) f32 {
        return self._clock.alpha();
    }
    pub fn setSpeed(self: *AppCore, value: GameSpeed) SpeedSelection {
        if (self._onboarding_hold and value != .paused) return .onboarding_rejected;
        if (self._speed == value) return .unchanged;
        self._speed = value;
        if (value != .paused) self._resume_speed = value;
        self._clock.clear();
        return .changed;
    }
    /// Later onboarding owns progress and calls this explicit hold seam.
    /// Releasing the hold resumes the remembered speed like Rust completion.
    pub fn setOnboardingHold(self: *AppCore, active: bool) void {
        if (self._onboarding_hold == active) return;
        self._onboarding_hold = active;
        self._clock.clear();
        if (active) {
            _ = self.setSpeed(.paused);
        } else if (self._speed == .paused) {
            _ = self.setSpeed(self._resume_speed);
        }
    }
    pub fn setMode(self: *AppCore, value: AppMode) void {
        if (self._mode == value) return;
        self._mode = value;
        self._clock.clear();
    }
    pub fn onSuspend(self: *AppCore) void {
        self._clock.clear();
    }
    pub fn onResume(self: *AppCore) void {
        self._clock.clear();
    }
    /// Explicit retry authorization. No tick is retried until new clock work.
    pub fn recover(self: *AppCore) bool {
        if (self._failure == null) return false;
        self._failure = null;
        self._clock.clear();
        return true;
    }

    pub fn advance(self: *AppCore, delta_ns: u128) FrameOutcome {
        self._camera.update(delta_ns);
        if (self._presentation.motion == .full) {
            // Match Duration::as_secs_f32's whole/fraction conversions. Divide
            // before conversion so even maximum u128 nanoseconds remain finite.
            const seconds = @as(f32, @floatFromInt(delta_ns / 1_000_000_000)) + @as(f32, @floatFromInt(delta_ns % 1_000_000_000)) / 1_000_000_000.0;
            self.visual_time = @mod(self.visual_time + seconds, 4096.0);
        }
        const held: ?FrameStatus = if (self._failure) |failure| .{ .failure_hold = failure } else if (self._mode != .playing) .mode_hold else if (self._onboarding_hold) .onboarding_hold else if (self._speed == .paused) .paused else if (self._simulation.state.phase == .finished) .gameplay_finished else null;
        if (held) |status| {
            self._clock.clear();
            return .{ .status = status };
        }
        var frame: FrameOutcome = .{ .status = .advanced };
        for (0..self._speed.multiplier()) |_| {
            self._clock.pushFrame(delta_ns);
            while (self._clock.takeStep()) {
                if (self._simulation.state.phase == .finished) {
                    self._clock.clear();
                    frame.status = .gameplay_finished;
                    return frame;
                }
                var report = self._simulation.step() catch |cause| {
                    const failure: TickFailure = .{ .cause = cause, .tick = self._simulation.state.next_tick };
                    self._failure = failure;
                    self._clock.clear();
                    frame.status = .{ .failure_hold = failure };
                    return frame;
                };
                defer report.deinit(self._allocator);
                var accepted: usize = 0;
                for (report.command_results.items) |result| {
                    if (result == .accepted) accepted += 1;
                }
                const summary: TickSummary = .{ .tick = report.tick, .accepted_commands = accepted, .event_count = report.events.items.len };
                frame.summaries[frame.completed_steps] = summary;
                frame.completed_steps += 1;
                frame.accepted_commands += accepted;
                frame.event_count += summary.event_count;
                if (summary.material()) frame.material_steps += 1;
                self.scheduleAdvisory(if (summary.material()) .MaterialEvent else .Tick);
                if (self._simulation.state.phase == .finished) {
                    self._clock.clear();
                    frame.status = .gameplay_finished;
                    return frame;
                }
            }
        }
        return frame;
    }

    /// Builds checked inline scenario/simulation/clock before touching resident
    /// ownership. Current admission and construction perform no allocations.
    pub fn replaceScenario(self: *AppCore, candidate: *const scenario.ScenarioV1) scenario.AdmissionError!void {
        const admitted = try scenario.admit(candidate);
        const replacement = try sim.Simulation.fromScenario(self._allocator, &admitted);
        const clock = FixedClock.init(admitted.rules.tick_hz, MAX_STEPS_PER_PASS) catch unreachable;
        self._simulation.deinit();
        self._simulation = replacement;
        self._active_scenario = admitted;
        self._clock = clock;
        self._mode = .playing;
        self._failure = null;
        self._next_sequence = 0;
        self.selected_world = null;
        self.hovered_world = null;
        self.touch_command_target = null;
        self.selected_field = .atmosphere;
        self._speed = if (self._onboarding_hold) .paused else .normal;
        self._resume_speed = .normal;
        self.help_visible = false;
        self.resetCamera();
        self.visual_time = 0;
        self.scheduleAdvisory(.Restart);
    }
    pub fn restart(self: *AppCore) scenario.AdmissionError!void {
        try self.replaceScenario(&self._active_scenario);
    }
};
