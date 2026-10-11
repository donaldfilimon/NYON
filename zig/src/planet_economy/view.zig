const ids = @import("ids.zig");
const types = @import("types.zig");
const state = @import("state.zig");

pub fn cell(storage: *const state.Storage, id: ids.CellId) types.Failure!types.CellView {
    return cellAt(storage, storage.active, id);
}

pub fn cellAt(storage: *const state.Storage, root: u1, id: ids.CellId) types.Failure!types.CellView {
    if (id.planet != storage.context.planet) return error.InvalidPlanet;
    const count: u64 = @as(u64, 6) * storage.config.side * storage.config.side;
    if (id.local >= count) return error.InvalidCell;
    const terrain = try storage.terrainAt(root, @intCast(id.local));
    const claim = (try storage.readRecord(root, .claim, @intCast(id.local))).claim;
    return .{
        .revision = storage.rootMeta(root).revision,
        .id = id,
        .terrain = terrain,
        .settlement = claim.settlement,
        .building = claim.building,
        .is_hall = claim.is_hall,
    };
}

pub fn settlement(storage: *const state.Storage, id: ids.SettlementId) types.Failure!types.SettlementView {
    return settlementAt(storage, storage.active, id);
}

pub fn settlementAt(storage: *const state.Storage, root: u1, id: ids.SettlementId) types.Failure!types.SettlementView {
    if (@backingInt(id) == 0) return error.NotFound;
    const revision = storage.rootMeta(root).revision;
    for (0..storage.config.capacities.settlements) |slot| {
        const record = (try storage.readRecord(root, .settlement, @intCast(slot))).settlement;
        if (record.id == id) return .{
            .revision = revision,
            .id = record.id,
            .owner = record.owner,
            .hall = record.hall,
            .population = record.population,
            .food = record.food,
            .material = record.material,
            .incoming_food = record.incoming_food,
            .incoming_material = record.incoming_material,
            .housing = record.housing,
            .hunger = record.hunger,
            .fed_streak = record.fed_streak,
        };
    }
    return error.NotFound;
}

pub fn receipts(storage: *const state.Storage, revision: ids.Revision, cursor: types.ReceiptCursor, out: []types.ReceiptView) types.Failure!types.ReceiptPage {
    return receiptsAt(storage, storage.active, revision, cursor, out);
}

pub fn receiptsAt(storage: *const state.Storage, root: u1, revision: ids.Revision, cursor: types.ReceiptCursor, out: []types.ReceiptView) types.Failure!types.ReceiptPage {
    const meta = storage.rootMeta(root);
    if (revision != meta.revision) return error.StaleView;
    if (out.len > 64) return error.CapacityUnsupported;
    if (cursor > meta.history_used) return error.NotFound;
    if (out.len == 0) return .{ .written = 0, .next = cursor };
    const count = @min(out.len, meta.history_used - cursor);
    for (out[0..count], 0..) |*row, index| {
        const slot = cursor + index;
        const len = storage.reservation.history_meta[slot];
        if (len > 1024) return error.InvariantViolation;
        row.len = @intCast(len);
        @memcpy(row.bytes[0..len], storage.reservation.history[slot][0..len]);
    }
    const next: u16 = cursor + @as(u16, @intCast(count));
    return .{ .written = @intCast(count), .next = if (next == meta.history_used) null else next };
}
