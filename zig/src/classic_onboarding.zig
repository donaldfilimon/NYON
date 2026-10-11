//! Presentation-only guidance; no authority, storage, clock or frame dependency.
pub const Step = enum { select_union_world, preview_launch, launch_fleet, tune_field, read_advisory, open_scenario_editor };
pub const Event = enum { union_world_selected, launch_previewed, fleet_launched, field_tuned, advisory_read, scenario_editor_opened };
pub const Progress = struct { current: usize, total: usize };
pub const State = struct {
    index: ?u3,
    pub fn init(completed: bool) State {
        return .{ .index = if (completed) null else 0 };
    }
    pub fn active(self: State) bool {
        return self.index != null;
    }
    pub fn step(self: State) ?Step {
        return if (self.index) |i| @fromBackingInt(@intCast(i)) else null;
    }
    pub fn progress(self: State) Progress {
        return .{ .current = if (self.index) |i| @as(usize, i) + 1 else 6, .total = 6 };
    }
    /// Returns true only on the final matching transition, never intermediate progress.
    pub fn observe(self: *State, event: Event) bool {
        const i = self.index orelse return false;
        const expected: Event = switch (self.step().?) {
            .select_union_world => .union_world_selected,
            .preview_launch => .launch_previewed,
            .launch_fleet => .fleet_launched,
            .tune_field => .field_tuned,
            .read_advisory => .advisory_read,
            .open_scenario_editor => .scenario_editor_opened,
        };
        if (event != expected) return false;
        self.index = if (i == 5) null else i + 1;
        return i == 5;
    }
    pub fn skip(self: *State) bool {
        const was_active = self.active();
        self.index = null;
        return was_active;
    }
    pub fn reset(self: *State) void {
        self.index = 0;
    }
};
