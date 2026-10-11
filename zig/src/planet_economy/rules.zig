const surface = @import("../surface.zig");
const ids = @import("ids.zig");
const types = @import("types.zig");

pub const Footprint = types.Footprint;

// Explicit pure algorithm identity. The default pins the original island.
pub const AlgorithmContext = struct {
    planet: ids.PlanetId = ids.planet_id,
    system: u32 = 1,
    civilization_count: ?u8 = null,

    pub fn validOwner(self: AlgorithmContext, owner: types.CivilizationId) bool {
        const count = self.civilization_count orelse return owner == ids.civilization_id;
        return @backingInt(owner) >= 1 and @backingInt(owner) <= count;
    }
};

pub fn validateConfig(config: types.Config) types.Failure!void {
    if (config.side < 4 or config.side > 32) return error.CapacityUnsupported;
    const caps = config.capacities;
    if (caps.settlements == 0 or caps.settlements > 64 or
        caps.buildings == 0 or caps.buildings > 256 or
        caps.shipments == 0 or caps.shipments > 64 or
        caps.history_receipts == 0 or caps.history_receipts > 4096)
        return error.CapacityUnsupported;
}

// This first terrain formula intentionally uses only the low two seed bits.
pub fn terrain(seed: u64, cell: ids.CellId) types.Terrain {
    return if ((seed % 4 + cell.local % 4) % 4 == 0) .rocky else .fertile;
}

pub fn footprint(topology: surface.Topology, kind: types.BuildingKind, anchor: ids.CellId, facing: ?surface.Direction) types.Failure!Footprint {
    return footprintContext(.{}, topology, kind, anchor, facing);
}

pub fn footprintContext(context: AlgorithmContext, topology: surface.Topology, kind: types.BuildingKind, anchor: ids.CellId, facing: ?surface.Direction) types.Failure!Footprint {
    if (anchor.planet != context.planet) return error.InvalidPlanet;
    if (topology.side == 0) return error.CapacityUnsupported;
    const local = topology.fromId(anchor.local) catch return error.InvalidCell;
    switch (kind) {
        .farm => {
            const direction = facing orelse return error.InvalidEnvelope;
            const neighbor = topology.neighbor(local, direction) catch return error.InvalidCell;
            const id = topology.cellId(neighbor) catch return error.InvalidCell;
            return .{ .len = 2, .cells = .{ anchor, .{ .planet = anchor.planet, .local = id } } };
        },
        .quarry, .house => {
            if (facing != null) return error.InvalidEnvelope;
            return .{ .len = 1, .cells = .{ anchor, anchor } };
        },
    }
}
