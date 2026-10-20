const std = @import("std");
const types = @import("types.zig");
const limits = @import("limits.zig");
const catalog_codec = @import("catalog.zig");

pub const MetadataMutation = union(enum) {
    select: types.Selection,
    rename_slot: struct { slot_id: types.SlotId, label: []const u8 },
    rename_branch: struct { slot_id: types.SlotId, branch_id: types.BranchId, label: []const u8 },
    archive_slot: struct { slot_id: types.SlotId, archived: bool },
    remove_slot: types.SlotId,
};

fn validLabel(label: []const u8) bool {
    if (label.len == 0 or label.len > limits.max_label_bytes) return false;
    if (!std.unicode.utf8ValidateSlice(label)) return false;
    for (label) |byte| if (byte < 0x20) return false;
    return true;
}

/// Returns a detached proposal. Reserved numbers alone are not publication:
/// the store must durably bind its exact generation record with
/// validateAgainstLedger and perform the actual generation CAS. Native flush
/// and browser transaction atomicity are adapter responsibilities.
pub fn prepare(
    allocator: types.Allocator,
    current: *const types.Catalog,
    expected_generation: u64,
    reserved_generation: u64,
    reserved_watermark: u64,
    mutation: MetadataMutation,
) types.Error!types.Catalog {
    try catalog_codec.validate(current);
    if (current.generation != expected_generation) return error.Conflict;
    if (reserved_generation <= current.generation or
        reserved_watermark <= current.allocator_watermark) return error.Invalid;
    switch (mutation) {
        .rename_slot => |rename| if (!validLabel(rename.label)) return error.Invalid,
        .rename_branch => |rename| if (!validLabel(rename.label)) return error.Invalid,
        else => {},
    }

    var target_found = false;
    for (current.slots) |slot| {
        switch (mutation) {
            .rename_slot => |rename| if (slot.id == rename.slot_id) {
                target_found = true;
            },
            .rename_branch => |rename| {
                if (slot.id == rename.slot_id) {
                    for (slot.branches) |branch| {
                        if (branch.id == rename.branch_id) target_found = true;
                    }
                }
            },
            .archive_slot => |archive| if (slot.id == archive.slot_id) {
                target_found = true;
            },
            .remove_slot => |id| if (slot.id == id) {
                target_found = true;
            },
            .select => target_found = true,
        }
    }
    if (!target_found) return error.Invalid;

    const removed_id: ?types.SlotId = switch (mutation) {
        .remove_slot => |id| id,
        else => null,
    };
    const count = current.slots.len - @intFromBool(removed_id != null);
    const slots = try allocator.alloc(types.Slot, count);
    var initialized: usize = 0;
    errdefer {
        for (slots[0..initialized]) |*slot| slot.deinit(allocator);
        allocator.free(slots);
    }
    for (current.slots) |source| {
        if (removed_id == source.id) continue;
        const slot_label: []const u8 = switch (mutation) {
            .rename_slot => |rename| if (source.id == rename.slot_id) rename.label else source.label.bytes,
            else => source.label.bytes,
        };
        const label = try allocator.dupe(u8, slot_label);
        errdefer allocator.free(label);
        const branches = try allocator.alloc(types.Branch, source.branches.len);
        var initialized_branches: usize = 0;
        errdefer {
            for (branches[0..initialized_branches]) |*branch| branch.deinit(allocator);
            allocator.free(branches);
        }
        for (source.branches) |branch| {
            const branch_label: []const u8 = switch (mutation) {
                .rename_branch => |rename| if (source.id == rename.slot_id and branch.id == rename.branch_id) rename.label else branch.label.bytes,
                else => branch.label.bytes,
            };
            branches[initialized_branches] = .{
                .id = branch.id,
                .label = .{ .bytes = try allocator.dupe(u8, branch_label) },
                .tip_digest = branch.tip_digest,
                .cursor_digest = branch.cursor_digest,
            };
            initialized_branches += 1;
        }
        const archived = switch (mutation) {
            .archive_slot => |archive| if (source.id == archive.slot_id) archive.archived else source.archived,
            else => source.archived,
        };
        slots[initialized] = .{
            .id = source.id,
            .archived = archived,
            .label = .{ .bytes = label },
            .manifest_digest = source.manifest_digest,
            .branches = branches,
        };
        initialized += 1;
    }
    const selection: ?types.Selection = switch (mutation) {
        .select => |selected| selected,
        .remove_slot => |id| if (current.selection) |selected|
            (if (selected.slot_id == id) null else selected)
        else
            null,
        else => current.selection,
    };
    const proposal = types.Catalog{
        .generation = reserved_generation,
        .allocator_watermark = reserved_watermark,
        .slots = slots,
        .selection = selection,
    };
    try catalog_codec.validate(&proposal);
    return proposal;
}
