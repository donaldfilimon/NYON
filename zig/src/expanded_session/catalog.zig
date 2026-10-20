const std = @import("std");
const types = @import("types.zig");
const limits = @import("limits.zig");

const header_len: usize = 48;

fn validLabel(label: []const u8) bool {
    if (label.len == 0 or label.len > limits.max_label_bytes) return false;
    if (!std.unicode.utf8ValidateSlice(label)) return false;
    for (label) |byte| if (byte < 0x20) return false;
    return true;
}

pub fn validate(value: *const types.Catalog) types.Error!void {
    if (value.slots.len > limits.max_slots) return error.TooLarge;
    if (value.generation == 0) {
        if (value.allocator_watermark != 0 or value.slots.len != 0 or value.selection != null)
            return error.Invalid;
    } else if (value.allocator_watermark == 0) return error.Invalid;

    var previous_slot: u64 = 0;
    var branch_ids: [limits.max_slots * limits.max_branches_per_slot]u64 = undefined;
    var branch_count: usize = 0;
    var selection_found = false;
    for (value.slots) |slot| {
        if (slot.id == 0 or slot.id <= previous_slot or !validLabel(slot.label.bytes)) return error.Invalid;
        if (slot.branches.len > limits.max_branches_per_slot) return error.TooLarge;
        previous_slot = slot.id;
        var previous_branch: u64 = 0;
        for (slot.branches) |branch| {
            if (branch.id == 0 or branch.id <= previous_branch or !validLabel(branch.label.bytes)) return error.Invalid;
            for (branch_ids[0..branch_count]) |prior| {
                if (prior == branch.id) return error.Invalid;
            }
            branch_ids[branch_count] = branch.id;
            branch_count += 1;
            previous_branch = branch.id;
            if (value.selection) |selected| {
                if (selected.slot_id == slot.id and selected.branch_id == branch.id) {
                    if (!std.mem.eql(u8, &selected.cursor_digest, &branch.cursor_digest) or
                        !std.mem.eql(u8, &selected.manifest_digest, &slot.manifest_digest)) return error.Invalid;
                    selection_found = true;
                }
            }
        }
    }
    if (value.selection != null and !selection_found) return error.Invalid;
}

const Writer = struct {
    bytes: []u8,
    pos: usize = 0,

    fn put(self: *Writer, source: []const u8) void {
        @memcpy(self.bytes[self.pos..][0..source.len], source);
        self.pos += source.len;
    }
    fn int(self: *Writer, comptime T: type, number: T) void {
        std.mem.writeInt(T, self.bytes[self.pos..][0..@sizeOf(T)], number, .little);
        self.pos += @sizeOf(T);
    }
};

pub fn encode(allocator: types.Allocator, value: *const types.Catalog) types.Error![]u8 {
    try validate(value);
    var body_len: usize = 19 + if (value.selection == null) @as(usize, 0) else 80;
    for (value.slots) |slot| {
        body_len += 8 + 1 + 2 + slot.label.bytes.len + 32 + 2;
        for (slot.branches) |branch| body_len += 8 + 2 + branch.label.bytes.len + 32 + 32;
    }
    const total = header_len + body_len;
    if (total > limits.max_catalog_bytes) return error.TooLarge;
    const bytes = try allocator.alloc(u8, total);
    var writer = Writer{ .bytes = bytes };
    writer.put("NYONCAT1");
    writer.int(u16, 1);
    writer.int(u16, 0);
    writer.int(u32, @intCast(body_len));
    writer.pos += 32;
    writer.int(u64, value.generation);
    writer.int(u64, value.allocator_watermark);
    writer.int(u16, @intCast(value.slots.len));
    for (value.slots) |slot| {
        writer.int(u64, slot.id);
        writer.int(u8, @intFromBool(slot.archived));
        writer.int(u16, @intCast(slot.label.bytes.len));
        writer.put(slot.label.bytes);
        writer.put(&slot.manifest_digest);
        writer.int(u16, @intCast(slot.branches.len));
        for (slot.branches) |branch| {
            writer.int(u64, branch.id);
            writer.int(u16, @intCast(branch.label.bytes.len));
            writer.put(branch.label.bytes);
            writer.put(&branch.tip_digest);
            writer.put(&branch.cursor_digest);
        }
    }
    if (value.selection) |selected| {
        writer.int(u8, 1);
        writer.int(u64, selected.slot_id);
        writer.int(u64, selected.branch_id);
        writer.put(&selected.cursor_digest);
        writer.put(&selected.manifest_digest);
    } else writer.int(u8, 0);
    std.debug.assert(writer.pos == total);
    std.crypto.hash.sha2.Sha256.hash(bytes[header_len..], bytes[16..48], .{});
    return bytes;
}

const Reader = struct {
    bytes: []const u8,
    pos: usize = 0,

    fn take(self: *Reader, count: usize) types.Error![]const u8 {
        if (count > self.bytes.len - self.pos) return error.Invalid;
        const result = self.bytes[self.pos..][0..count];
        self.pos += count;
        return result;
    }
    fn int(self: *Reader, comptime T: type) types.Error!T {
        return std.mem.readInt(T, (try self.take(@sizeOf(T)))[0..@sizeOf(T)], .little);
    }
    fn digest(self: *Reader) types.Error!types.Digest {
        return (try self.take(32))[0..32].*;
    }
};

pub fn decode(allocator: types.Allocator, bytes: []const u8) types.Error!types.Catalog {
    if (bytes.len > limits.max_catalog_bytes) return error.TooLarge;
    if (bytes.len < header_len + 19 or !std.mem.eql(u8, bytes[0..8], "NYONCAT1")) return error.Invalid;
    if (std.mem.readInt(u16, bytes[8..10], .little) != 1 or
        std.mem.readInt(u16, bytes[10..12], .little) != 0 or
        std.mem.readInt(u32, bytes[12..16], .little) != bytes.len - header_len) return error.Invalid;
    var digest: types.Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(bytes[header_len..], &digest, .{});
    if (!std.mem.eql(u8, &digest, bytes[16..48])) return error.Invalid;

    var reader = Reader{ .bytes = bytes[header_len..] };
    const generation = try reader.int(u64);
    const watermark = try reader.int(u64);
    const count = try reader.int(u16);
    if (count > limits.max_slots) return error.TooLarge;
    const slots = try allocator.alloc(types.Slot, count);
    var initialized_slots: usize = 0;
    errdefer {
        for (slots[0..initialized_slots]) |*slot| slot.deinit(allocator);
        allocator.free(slots);
    }
    while (initialized_slots < count) {
        const id = try reader.int(u64);
        const archived = try reader.int(u8);
        if (archived > 1) return error.Invalid;
        const label_len = try reader.int(u16);
        if (label_len == 0 or label_len > limits.max_label_bytes) return error.Invalid;
        const label_source = try reader.take(label_len);
        if (!validLabel(label_source)) return error.Invalid;
        const label = try allocator.dupe(u8, label_source);
        errdefer allocator.free(label);
        const manifest_digest = try reader.digest();
        const branch_count = try reader.int(u16);
        if (branch_count > limits.max_branches_per_slot) return error.TooLarge;
        const branches = try allocator.alloc(types.Branch, branch_count);
        var initialized_branches: usize = 0;
        errdefer {
            for (branches[0..initialized_branches]) |*branch| branch.deinit(allocator);
            allocator.free(branches);
        }
        while (initialized_branches < branch_count) {
            const branch_id = try reader.int(u64);
            const branch_label_len = try reader.int(u16);
            if (branch_label_len == 0 or branch_label_len > limits.max_label_bytes) return error.Invalid;
            const branch_label_source = try reader.take(branch_label_len);
            if (!validLabel(branch_label_source)) return error.Invalid;
            const branch_label = try allocator.dupe(u8, branch_label_source);
            errdefer allocator.free(branch_label);
            const tip_digest = try reader.digest();
            const cursor_digest = try reader.digest();
            branches[initialized_branches] = .{
                .id = branch_id,
                .label = .{ .bytes = branch_label },
                .tip_digest = tip_digest,
                .cursor_digest = cursor_digest,
            };
            initialized_branches += 1;
        }
        slots[initialized_slots] = .{
            .id = id,
            .archived = archived == 1,
            .label = .{ .bytes = label },
            .manifest_digest = manifest_digest,
            .branches = branches,
        };
        initialized_slots += 1;
    }
    const presence = try reader.int(u8);
    if (presence > 1) return error.Invalid;
    const selection: ?types.Selection = if (presence == 1) .{
        .slot_id = try reader.int(u64),
        .branch_id = try reader.int(u64),
        .cursor_digest = try reader.digest(),
        .manifest_digest = try reader.digest(),
    } else null;
    if (reader.pos != reader.bytes.len) return error.Invalid;
    const result = types.Catalog{
        .generation = generation,
        .allocator_watermark = watermark,
        .slots = slots,
        .selection = selection,
    };
    try validate(&result);
    return result;
}
