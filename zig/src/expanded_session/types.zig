const std = @import("std");

pub const Allocator = std.mem.Allocator;
pub const Digest = [32]u8;
pub const SlotId = u64;
pub const BranchId = u64;
pub const Error = Allocator.Error || error{ Invalid, TooLarge, IdExhausted, Conflict };

pub const Label = struct {
    bytes: []u8,

    pub fn deinit(self: *Label, allocator: Allocator) void {
        allocator.free(self.bytes);
        self.* = undefined;
    }
};

pub const Branch = struct {
    id: BranchId,
    label: Label,
    tip_digest: Digest,
    cursor_digest: Digest,

    pub fn deinit(self: *Branch, allocator: Allocator) void {
        self.label.deinit(allocator);
        self.* = undefined;
    }
};

pub const Slot = struct {
    id: SlotId,
    archived: bool,
    label: Label,
    manifest_digest: Digest,
    branches: []Branch,

    pub fn deinit(self: *Slot, allocator: Allocator) void {
        for (self.branches) |*branch| branch.deinit(allocator);
        allocator.free(self.branches);
        self.label.deinit(allocator);
        self.* = undefined;
    }
};

pub const Selection = struct {
    slot_id: SlotId,
    branch_id: BranchId,
    cursor_digest: Digest,
    manifest_digest: Digest,
};

pub const Catalog = struct {
    generation: u64,
    allocator_watermark: u64,
    slots: []Slot,
    selection: ?Selection,

    pub fn deinit(self: *Catalog, allocator: Allocator) void {
        for (self.slots) |*slot| slot.deinit(allocator);
        allocator.free(self.slots);
        self.* = undefined;
    }
};
