//! Pure rational Classic clock. The host supplies elapsed nanoseconds.
pub const NANOS_PER_SECOND: u128 = 1_000_000_000;
pub const MAX_FRAME_DELTA_NS: u128 = 250_000_000;

pub const FixedClock = struct {
    tick_hz: u32,
    scaled_accumulator: u128 = 0,
    pending_steps: usize = 0,
    max_steps: usize,

    /// Invalid public configuration is an error in every optimization mode.
    pub fn init(tick_hz: u32, max_steps: usize) error{InvalidTickRate}!FixedClock {
        if (tick_hz == 0) return error.InvalidTickRate;
        return .{ .tick_hz = tick_hz, .max_steps = max_steps };
    }

    pub fn pushFrame(self: *FixedClock, delta_ns: u128) void {
        const accepted = @min(delta_ns, MAX_FRAME_DELTA_NS);
        self.scaled_accumulator +|= accepted *| @as(u128, self.tick_hz);
        const whole = self.scaled_accumulator / NANOS_PER_SECOND;
        self.scaled_accumulator %= NANOS_PER_SECOND;
        const available = self.max_steps -| self.pending_steps;
        self.pending_steps +|= @intCast(@min(whole, @as(u128, available)));
    }

    pub fn takeStep(self: *FixedClock) bool {
        if (self.pending_steps == 0) return false;
        self.pending_steps -= 1;
        return true;
    }

    pub fn alpha(self: *const FixedClock) f32 {
        return @as(f32, @floatFromInt(self.scaled_accumulator)) / @as(f32, @floatFromInt(NANOS_PER_SECOND));
    }

    pub fn clear(self: *FixedClock) void {
        self.scaled_accumulator = 0;
        self.pending_steps = 0;
    }
};
