//! Pure bounded source-parity gestures. No host, camera, Session or allocator effects.
//! Derived orbit overflow is preserved; a future consumer must reject it before camera use.
const std = @import("std");
const math = @import("classic_math.zig");
pub const POINTER_GESTURE_THRESHOLD: f32 = 6.0;
pub const PointerSource = enum { mouse, touch };
pub const PointerButton = enum { primary, secondary, middle };
fn valueOnly(comptime T: type) bool {
    return switch (@typeInfo(T)) {
        .bool, .int, .float, .@"enum", .void => true,
        .optional => |info| valueOnly(info.child),
        .array => |info| valueOnly(info.child),
        .@"struct" => |info| blk: {
            for (info.field_types) |field_type| {
                if (!valueOnly(field_type)) break :blk false;
            }
            break :blk true;
        },
        .@"union" => |info| blk: {
            if (info.tag_type == null) break :blk false;
            for (info.field_types) |field_type| {
                if (!valueOnly(field_type)) break :blk false;
            }
            break :blk true;
        },
        else => false,
    };
}
fn distance(a: math.Vec2, b: math.Vec2) f32 {
    @setFloatMode(.strict);
    const x = a.x - b.x;
    const y = a.y - b.y;
    return @sqrt(x * x + y * y);
}
pub fn InteractionController(comptime HudAction: type, comptime max_contacts: usize) type {
    if (!valueOnly(HudAction)) @compileError("HudAction must be a self-contained tagged value");
    if (@sizeOf(HudAction) > 64) @compileError("HudAction exceeds accepted storage cap");
    return struct {
        const Self = @This();
        pub const Hit = union(enum) { hud: HudAction, world, empty_scene };
        pub const Gesture = union(enum) { hud: HudAction, scene_tap: struct { position: math.Vec2, source: PointerSource }, immediate_launch: math.Vec2, orbit: math.Vec2, zoom: f32 };
        pub const BeginError = error{ NonFinitePosition, DuplicateContact, ContactLimit };
        const Owner = union(enum) { hud: HudAction, tap_only, orbit_or_tap, immediate_launch, pinch };
        const Contact = struct { id: u64, source: PointerSource, owner: Owner, start: math.Vec2, last: math.Vec2, moved: bool };
        pub const entry_bytes = @sizeOf(?Contact);
        contacts: [max_contacts]?Contact = @splat(null),
        pinch_distance: ?f32 = null,
        comptime {
            if (entry_bytes > 128) @compileError("contact exceeds accepted storage cap");
            if (@sizeOf(Self) > max_contacts * 128 + 64) @compileError("controller exceeds accepted storage cap");
        }
        pub fn init() Self {
            return .{};
        }
        fn index(self: *const Self, id: u64) ?usize {
            for (self.contacts, 0..) |entry, i| if (entry) |contact| {
                if (contact.id == id) return i;
            };
            return null;
        }
        pub fn hasContact(self: *const Self, id: u64) bool {
            return self.index(id) != null;
        }
        pub fn contactCount(self: *const Self) usize {
            var count: usize = 0;
            for (self.contacts) |entry| {
                if (entry != null) count += 1;
            }
            return count;
        }
        pub fn begin(self: *Self, id: u64, position: math.Vec2, source: PointerSource, button: PointerButton, hit: Hit) BeginError!void {
            if (!position.isFinite()) return error.NonFinitePosition;
            if (self.hasContact(id)) return error.DuplicateContact;
            if (max_contacts == 0) return error.ContactLimit;
            var free: ?usize = null;
            for (self.contacts, 0..) |entry, i| {
                if (entry == null) {
                    free = i;
                    break;
                }
            }
            const slot = free orelse return error.ContactLimit;
            const owner: Owner = switch (hit) {
                .hud => |action| .{ .hud = action },
                .world => if (source == .mouse and button == .primary) .tap_only else if (button == .secondary) .immediate_launch else .orbit_or_tap,
                .empty_scene => if (button == .primary) .orbit_or_tap else if (button == .secondary) .immediate_launch else .orbit_or_tap,
            };
            self.contacts[slot] = .{ .id = id, .source = source, .owner = owner, .start = position, .last = position, .moved = false };
            if (source == .touch) self.beginPinch();
        }
        // ID sorting preserves the source BTreeMap traversal without allocating.
        fn pair(self: *const Self, scene_touch: bool) ?[2]usize {
            var result: [2]usize = undefined;
            var count: usize = 0;
            for (self.contacts, 0..) |entry, i| if (entry) |contact| {
                const eligible = if (scene_touch) contact.source == .touch and contact.owner != .hud else contact.owner == .pinch;
                if (!eligible) continue;
                if (count == 2) return null;
                result[count] = i;
                count += 1;
            };
            if (count != 2) return null;
            if (self.contacts[result[0]].?.id > self.contacts[result[1]].?.id) std.mem.swap(usize, &result[0], &result[1]);
            return result;
        }
        fn touchDistance(self: *const Self) ?f32 {
            const indices = self.pair(false) orelse return null;
            return distance(self.contacts[indices[0]].?.last, self.contacts[indices[1]].?.last);
        }
        fn beginPinch(self: *Self) void {
            const indices = self.pair(true) orelse return;
            for (indices) |i| {
                self.contacts[i].?.owner = .pinch;
                self.contacts[i].?.moved = true;
            }
            self.pinch_distance = self.touchDistance();
        }
        pub fn update(self: *Self, id: u64, position: math.Vec2) ?Gesture {
            @setFloatMode(.strict);
            if (!position.isFinite()) return null;
            const i = self.index(id) orelse return null;
            const contact = &self.contacts[i].?;
            const delta: math.Vec2 = .{ .x = position.x - contact.last.x, .y = position.y - contact.last.y };
            contact.last = position;
            if (distance(position, contact.start) >= POINTER_GESTURE_THRESHOLD) contact.moved = true;
            if (contact.owner == .pinch) {
                const current = self.touchDistance() orelse return null;
                const previous = self.pinch_distance;
                self.pinch_distance = current;
                const old = previous orelse return null;
                if (!std.math.isFinite(current) or !std.math.isFinite(old) or current <= std.math.floatEps(f32) or old <= std.math.floatEps(f32)) return null;
                return .{ .zoom = (@log(current) - @log(old)) * 600.0 };
            }
            if (contact.owner == .orbit_or_tap and contact.moved) return .{ .orbit = delta };
            return null;
        }
        pub fn end(self: *Self, id: u64, position: math.Vec2, hud_hit: ?HudAction) ?Gesture {
            const i = self.index(id) orelse return null;
            var contact = self.contacts[i].?;
            self.contacts[i] = null;
            if (position.isFinite()) {
                contact.last = position;
                contact.moved = contact.moved or distance(position, contact.start) >= POINTER_GESTURE_THRESHOLD;
            }
            if (contact.owner == .pinch) {
                self.pinch_distance = null;
                for (&self.contacts) |*entry| if (entry.*) |*remaining| {
                    if (remaining.owner == .pinch) {
                        remaining.owner = .tap_only;
                        remaining.moved = true;
                    }
                };
                return null;
            }
            if (contact.moved) return null;
            return switch (contact.owner) {
                .hud => |action| if (hud_hit) |current| (if (std.meta.eql(action, current)) Gesture{ .hud = action } else null) else null,
                .tap_only, .orbit_or_tap => .{ .scene_tap = .{ .position = contact.last, .source = contact.source } },
                .immediate_launch => .{ .immediate_launch = contact.last },
                .pinch => unreachable,
            };
        }
        pub fn cancel(self: *Self) void {
            self.* = .{};
        }
    };
}
