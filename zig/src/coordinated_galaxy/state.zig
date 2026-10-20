//! Fixed coordinator reservation and independent allocation ownership ledger.
const std = @import("std");
const t = @import("types.zig");
const planet = @import("planet.zig");

pub const requested_bytes = 12_582_912;
pub const planet_requested_bytes = 8_642_560;
pub const maximum_requested_bytes = 47_153_152;
pub const CoordinatorReservation = extern struct {
    history: [4096][2048]u8,
    staged: [512][2048]u8,
    scratch: [2_097_152]u8 align(16),
    metadata: [1_048_576]u8 align(16),
};

pub const DescriptorHeader = struct {
    tick: t.Tick = @fromBackingInt(0),
    revision: t.Revision = @fromBackingInt(0),
    last_sequence: t.CommandSequence = @fromBackingInt(0),
    policy: ?t.SessionPolicy = null,
    history_count: u16 = 0,
    history_head: [32]u8 = @splat(0),
    accepted: u16 = 0,
    rejected: u16 = 0,
};
pub const Lease = struct {
    generation: u64 = 0,
    descriptor_slot: u1 = 0,
    descriptor_generation: u64 = 0,
    live: bool = false,
};
const initial_free_leases: [64]u8 = blk: {
    var result: [64]u8 = undefined;
    for (&result, 0..) |*slot, index| slot.* = @intCast(index);
    break :blk result;
};

// All pointers retained by the public handle target this reserved allocation.
// Later coordinator job/replay controls also belong in this reservation.
pub const Metadata = struct {
    descriptors: [2]t.Descriptor = undefined,
    descriptor_generations: [2]u64 = .{ 1, 1 },
    owner: ?Storage = null,
    owner_generation: u64 = 0,
    committed_descriptor: ?u1 = null,
    obsolete_descriptor: ?u1 = null,
    preparing_descriptor: ?u1 = null,
    descriptor_header: DescriptorHeader = .{},
    prepared_roots: [4]?t.PreparedRoot = .{ null, null, null, null },
    descriptor_cursor: u8 = 0,
    descriptor_ready: bool = false,
    descriptor_work: u64 = 0,
    leases: [64]Lease = @splat(.{}),
    free_leases: [64]u8 = initial_free_leases,
    free_lease_count: u8 = 64,
    descriptor_leases: [2]u8 = .{ 0, 0 },
    lane_live: bool = false,
    acquisition_closed: bool = false,
    faulted: bool = false,
    commands: [256][1024]u8 = undefined,
    command_lengths: [256]u16 = undefined,
    command_planets: [256]t.PlanetId = undefined,
    command_order: [256]u8 = undefined,
    command_sequences: [256]u64 = undefined,
    published_sequence: t.CommandSequence = @fromBackingInt(0),
    history_lengths: [4096]u16 = undefined,
    staged_lengths: [512]u16 = undefined,
    builders: [4]?planet.PlanetBuilder = .{ null, null, null, null },
    planets: [4]?planet.PlanetAuthority = .{ null, null, null, null },
};
comptime {
    std.debug.assert(@sizeOf(CoordinatorReservation) == requested_bytes);
    std.debug.assert(@offsetOf(CoordinatorReservation, "staged") == 8_388_608);
    std.debug.assert(@offsetOf(CoordinatorReservation, "scratch") == 9_437_184);
    std.debug.assert(@offsetOf(CoordinatorReservation, "metadata") == 11_534_336);
    std.debug.assert(@sizeOf(Metadata) + @alignOf(Metadata) - 1 < 1_048_576);
    std.debug.assert(requested_bytes + 4 * planet_requested_bytes == maximum_requested_bytes);
}

pub const Storage = struct {
    allocator: t.Allocator,
    reservation: ?*CoordinatorReservation,
    config: t.Config,
    owned_planets: u8 = 0,
    setup_index: u8 = 0,
    metadata_index: u8 = 0,
    metadata_ready: bool = false,

    pub fn init(allocator: t.Allocator, config: t.Config) t.Failure!Storage {
        try t.validateConfig(config);
        var result = try allocate(allocator, config);
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        _ = try result.advanceMetadata(&budget);
        return result;
    }
    pub fn allocate(allocator: t.Allocator, config: t.Config) t.Failure!Storage {
        // Only the allocation count is inspected here. All semantics are paid.
        if (config.planet_count > 4) return error.InvalidConfig;
        const reservation = allocator.create(CoordinatorReservation) catch return error.OutOfMemory;
        var self: Storage = .{ .allocator = allocator, .reservation = reservation, .config = config };
        // Only allocation ledger fields are live before paid initialization.
        const m = self.metadata();
        m.owner = null;
        m.owner_generation = 0;
        m.builders = .{ null, null, null, null };
        m.planets = .{ null, null, null, null };
        errdefer self.deinit();
        for (config.planets[0..config.planet_count], 0..) |pc, index| {
            self.metadata().builders[index] = try planet.allocatePlanet(allocator, pc, @fromBackingInt(@as(u32, @intCast(index)) + 1), config.seed, config.civilization_count);
            self.owned_planets += 1;
        }
        return self;
    }
    pub fn advanceMetadata(self: *Storage, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const m = self.metadata();
        while (budget.remaining > 0 and !self.metadata_ready) {
            budget.remaining -= 1;
            if (self.metadata_index == 0) {
                m.descriptor_generations = .{ 1, 1 };
                m.committed_descriptor = null;
                m.obsolete_descriptor = null;
                m.preparing_descriptor = null;
                m.descriptor_header = .{};
                m.descriptor_cursor = 0;
                m.descriptor_ready = false;
                m.descriptor_work = 0;
                m.free_lease_count = 64;
                m.descriptor_leases = .{ 0, 0 };
                m.lane_live = false;
                m.acquisition_closed = false;
                m.faulted = false;
                m.published_sequence = @fromBackingInt(0);
            } else if (self.metadata_index <= 64) {
                const index = self.metadata_index - 1;
                m.leases[index] = .{};
                m.free_leases[index] = index;
            } else {
                m.prepared_roots = .{ null, null, null, null };
                self.metadata_ready = true;
            }
            self.metadata_index += 1;
        }
        return if (self.metadata_ready) .done else .pending;
    }
    pub fn metadata(self: *const Storage) *Metadata {
        return @ptrCast(@alignCast(&self.reservation.?.metadata));
    }
    pub fn requestedBytes(self: *const Storage) usize {
        return (if (self.reservation != null) @as(usize, requested_bytes) else 0) + @as(usize, self.owned_planets) * planet_requested_bytes;
    }
    pub fn advanceSetup(self: *Storage, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        while (self.setup_index < self.owned_planets) {
            if (budget.remaining == 0) return .pending;
            const m = self.metadata();
            const builder = &m.builders[self.setup_index].?;
            if (try planet.advancePlanetCreate(builder, budget) == .pending) return .pending;
            if (budget.remaining == 0) return .pending;
            budget.remaining -= 1;
            m.planets[self.setup_index] = try planet.takePlanet(builder);
            m.builders[self.setup_index] = null;
            self.setup_index += 1;
        }
        return .done;
    }
    // Frees only ledger-owned allocations, without page/refcount traversal.
    // Later fault shutdown calls this only after closing/draining public leases.
    pub fn deinit(self: *Storage) void {
        const reservation = self.reservation orelse return;
        const m = self.metadata();
        while (self.owned_planets > 0) {
            self.owned_planets -= 1;
            const index = self.owned_planets;
            if (m.planets[index]) |*authority| {
                authority.deinit();
                m.planets[index] = null;
            } else if (m.builders[index]) |*builder| {
                builder.deinit();
                m.builders[index] = null;
            }
        }
        self.reservation = null;
        self.allocator.destroy(reservation);
    }
};
