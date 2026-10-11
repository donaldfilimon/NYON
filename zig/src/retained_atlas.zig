//! Owned frozen atlas metadata. No pixels, font fallback, renderer or text consumer.
const std = @import("std");
pub const Rect = @import("retained_geometry.zig").Rect;
pub const MAX_JSON_BYTES: usize = 65536;
pub const SCRATCH_BYTES: usize = 1048576;
pub const ENTRY_COUNT: usize = 205;
pub const FROZEN_SHA256: [32]u8 = .{ 0xe1, 0x8f, 0x5b, 0x93, 0xa7, 0xe4, 0xe9, 0x68, 0xaf, 0xb0, 0x28, 0xa3, 0xf7, 0xb2, 0x7c, 0x92, 0xe1, 0xfe, 0x86, 0x67, 0xd2, 0xae, 0x44, 0x0c, 0x5f, 0xda, 0x41, 0x79, 0x9e, 0x94, 0x16, 0xbb };
pub fn Outcome(comptime T: type) type {
    return union(enum) { ok: T, failure: Diagnostic };
}
pub const JsonCause = enum { InvalidSyntax, UnexpectedToken, MissingField, DuplicateField, UnknownField, InvalidEnum, NumericRange, TokenTooLong, OtherParseFailure };
pub const DescriptorField = enum { width, height, format, cell_size, font_px, font_origin_x, font_baseline_y, icon_size, edge_threshold, sdf_spread, ordering, payload_length };
pub const Resource = enum { input_bytes, boundaries, placements, staged_bytes };
pub const Diagnostic = union(enum) {
    InputTooLarge: struct { actual: usize, max: usize },
    ScratchTooSmall: struct { actual: usize, required: usize },
    ScratchExhausted,
    InvalidJson: struct { cause: JsonCause },
    ChecksumMismatch: struct { expected: [32]u8, actual: [32]u8 },
    UnsupportedVersion: struct { actual: u32 },
    DescriptorMismatch: struct { field: DescriptorField },
    EntryCount: struct { actual: usize },
    DuplicateKey: struct { entry: usize },
    InvalidBounds: struct { entry: usize },
    Overlap: struct { first: usize, second: usize },
    InvalidCell: struct { entry: usize },
    InvalidAdvance: struct { entry: usize },
    InvalidMapping: struct { entry: usize },
    InvalidUtf8: struct { byte_offset: usize },
    UnsupportedCharacter: struct { codepoint: u21, byte_offset: usize },
    MissingEntry: struct { index: usize },
    NonFiniteGeometry,
    LimitExceeded: struct { resource: Resource, actual: usize, max: usize },
    DestinationCapacity: struct { required: usize, available: usize },
    StagingCapacity: struct { required: usize, available: usize },
    InvalidCount: struct { count: usize, capacity: usize },
    ByteSizeOverflow,
    Aliasing,
};
pub const Weight = enum { Regular, SemiBold };
pub const Icon = enum { Play, Pause, Speed, Reset, Settings, Help, Crosshair, Save, Load, Check, Close, Zoom, FieldAtmosphere, FieldHydrosphere, FieldTopology };
pub const AtlasFormat = enum { r8_unorm };
pub const Bounds = struct { x: u32, y: u32, width: u32, height: u32 };
pub const AtlasEntry = struct { bounds: Bounds, advance: f32, identity: union(enum) { glyph: struct { codepoint: u21, weight: Weight }, icon: Icon } };
pub const Generator = struct { cell_size: u32, font_px: f32, font_origin_x: i32, font_baseline_y: i32, icon_size: u32, edge_threshold: u8, sdf_spread: u32 };
pub const AtlasMetrics = struct { version: u32, width: u32, height: u32, format: AtlasFormat, generator: Generator, entries: [ENTRY_COUNT]AtlasEntry };
pub const AtlasView = *const AtlasMetrics;
pub const PayloadDescriptor = struct { byte_length: usize };
pub const Measurement = struct { advance: f32 };
pub const Boundary = struct { byte_offset: usize, advance_before: f32 };
pub const GlyphPlacement = struct { rect: [4]f32, uv: [4]f32, color: [4]f32 };
pub const PlacementRequest = struct { baseline: [2]f32, font_size: f32, weight: Weight, color: [4]f32, text: []const u8, clip: ?Rect };
pub const TextLimits = struct { max_input_bytes: usize, max_boundaries: usize, max_placements: usize, max_staged_bytes: usize };
/// Fixture-only digest bypass; schema, budgets and ownership remain identical.
pub fn decodeFixtureMetadata(bytes: []const u8, payload: PayloadDescriptor, scratch: []u8) Outcome(AtlasMetrics) {
    if (bytes.len > MAX_JSON_BYTES) return .{ .failure = .{ .InputTooLarge = .{ .actual = bytes.len, .max = MAX_JSON_BYTES } } };
    if (scratch.len < SCRATCH_BYTES) return .{ .failure = .{ .ScratchTooSmall = .{ .actual = scratch.len, .required = SCRATCH_BYTES } } };
    if (disjoint(bytes, scratch)) |diagnostic| return .{ .failure = diagnostic };
    var fba = std.heap.FixedBufferAllocator.init(scratch[0..SCRATCH_BYTES]);
    const parsed = std.json.parseFromSlice(RawMetrics, fba.allocator(), bytes, .{
        .allocate = .alloc_if_needed,
        .max_value_len = 256,
        .ignore_unknown_fields = false,
        .duplicate_field_behavior = .@"error",
    }) catch |err| return .{ .failure = parseDiagnostic(err) };
    defer parsed.deinit();
    return validate(parsed.value, payload);
}

pub fn loadFrozenMetadata(bytes: []const u8, payload: PayloadDescriptor, scratch: []u8) Outcome(AtlasMetrics) {
    if (bytes.len > MAX_JSON_BYTES) return .{ .failure = .{ .InputTooLarge = .{ .actual = bytes.len, .max = MAX_JSON_BYTES } } };
    var actual: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash(bytes, &actual, .{});
    if (!std.mem.eql(u8, &actual, &FROZEN_SHA256)) return .{ .failure = .{ .ChecksumMismatch = .{ .expected = FROZEN_SHA256, .actual = actual } } };
    return decodeFixtureMetadata(bytes, payload, scratch);
}

/// One candidate and one assignment. Failure preserves every resident byte/view.
pub fn replaceFrozenMetadata(resident: *AtlasMetrics, bytes: []const u8, payload: PayloadDescriptor, scratch: []u8) Outcome(void) {
    if (disjoint(std.mem.asBytes(resident), bytes)) |diagnostic| return .{ .failure = diagnostic };
    if (disjoint(std.mem.asBytes(resident), scratch)) |diagnostic| return .{ .failure = diagnostic };
    return switch (loadFrozenMetadata(bytes, payload, scratch)) {
        .failure => |diagnostic| .{ .failure = diagnostic },
        .ok => |candidate| blk: {
            resident.* = candidate;
            break :blk .{ .ok = {} };
        },
    };
}

/// Checked whole backing ranges; an empty slice has no addressable bytes.
fn disjoint(first: []const u8, second: []const u8) ?Diagnostic {
    const first_start = @intFromPtr(first.ptr);
    const second_start = @intFromPtr(second.ptr);
    const first_end = std.math.add(usize, first_start, first.len) catch return .ByteSizeOverflow;
    const second_end = std.math.add(usize, second_start, second.len) catch return .ByteSizeOverflow;
    if (first.len != 0 and second.len != 0 and first_start < second_end and second_start < first_end) return .Aliasing;
    return null;
}

fn parseDiagnostic(err: anyerror) Diagnostic {
    if (err == error.OutOfMemory) return .ScratchExhausted;
    const cause: JsonCause = switch (err) {
        error.SyntaxError, error.UnexpectedEndOfInput => .InvalidSyntax,
        error.UnexpectedToken => .UnexpectedToken,
        error.MissingField => .MissingField,
        error.DuplicateField => .DuplicateField,
        error.UnknownField => .UnknownField,
        error.InvalidEnumTag => .InvalidEnum,
        error.InvalidNumber, error.Overflow => .NumericRange,
        error.ValueTooLong => .TokenTooLong,
        else => .OtherParseFailure,
    };
    return .{ .InvalidJson = .{ .cause = cause } };
}

// Default std JSON accepts numeric strings and integer-valued floats. These
// bounded wrappers require source number tokens and exact source widths.
fn Number(comptime T: type) type {
    return struct {
        value: T,
        pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!@This() {
            const token = try source.nextAllocMax(allocator, .alloc_if_needed, options.max_value_len.?);
            defer freeToken(allocator, token);
            const bytes = switch (token) {
                .number, .allocated_number => |value| value,
                else => return error.UnexpectedToken,
            };
            if (bytes.len > 256) return error.ValueTooLong;
            if (@typeInfo(T) == .float) return .{ .value = try std.fmt.parseFloat(T, bytes) };
            if (!std.json.isNumberFormattedLikeAnInteger(bytes)) return error.InvalidNumber;
            if (@typeInfo(T).int.signedness == .unsigned and bytes[0] == '-') return error.InvalidNumber;
            return .{ .value = std.fmt.parseInt(T, bytes, 10) catch return error.InvalidNumber };
        }
    };
}

const String = struct {
    bytes: []const u8,
    pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!String {
        const token = try source.nextAllocMax(allocator, .alloc_if_needed, options.max_value_len.?);
        // Arena retains allocated decoded strings until parsed.deinit(). Borrowed
        // strings also require this explicit bound; max_value_len ignores them.
        const bytes = switch (token) {
            .string, .allocated_string => |value| value,
            else => {
                freeToken(allocator, token);
                return error.UnexpectedToken;
            },
        };
        if (bytes.len > 256) {
            freeToken(allocator, token);
            return error.ValueTooLong;
        }
        return .{ .bytes = bytes };
    }
};
fn Enum(comptime T: type) type {
    return struct {
        value: T,
        pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!@This() {
            const string = try String.jsonParse(allocator, source, options);
            return .{ .value = std.meta.stringToEnum(T, string.bytes) orelse return error.InvalidEnumTag };
        }
    };
}
fn freeToken(allocator: std.mem.Allocator, token: std.json.Token) void {
    switch (token) {
        .allocated_number, .allocated_string => |bytes| allocator.free(bytes),
        else => {},
    }
}
const RawBounds = struct { x: Number(u32), y: Number(u32), width: Number(u32), height: Number(u32) };
const RawEntry = struct {
    key: String,
    kind: Enum(enum { glyph, icon }),
    codepoint: ?Number(u32) = null,
    font_weight: ?String = null,
    icon: ?String = null,
    atlas_bounds: RawBounds,
    advance: Number(f32),
};
const RawMetrics = struct {
    format_version: Number(u32),
    atlas: struct { width: Number(u32), height: Number(u32), format: Enum(AtlasFormat) },
    generator: struct { cell_size: Number(u32), font_px: Number(f32), font_origin_x: Number(i32), font_baseline_y: Number(i32), icon_size: Number(u32), edge_threshold: Number(u8), sdf_spread: Number(u32), ordering: String },
    entries: []RawEntry,
};
const ordering = "regular ASCII U+0020..U+007E; semibold ASCII U+0020..U+007E; frozen icons in source order";
const icon_names = [_][]const u8{ "play", "pause", "speed", "reset", "settings", "help", "crosshair", "save", "load", "check", "close", "zoom", "field-atmosphere", "field-hydrosphere", "field-topology" };
const icon_keys = [_][]const u8{ "icon:play", "icon:pause", "icon:speed", "icon:reset", "icon:settings", "icon:help", "icon:crosshair", "icon:save", "icon:load", "icon:check", "icon:close", "icon:zoom", "icon:field-atmosphere", "icon:field-hydrosphere", "icon:field-topology" };
fn descriptor(field: DescriptorField) Outcome(AtlasMetrics) {
    return .{ .failure = .{ .DescriptorMismatch = .{ .field = field } } };
}
fn optionalString(value: ?String, expected: []const u8) bool {
    return if (value) |s| std.mem.eql(u8, s.bytes, expected) else false;
}
fn validate(raw: RawMetrics, payload: PayloadDescriptor) Outcome(AtlasMetrics) {
    if (raw.format_version.value != 1) return .{ .failure = .{ .UnsupportedVersion = .{ .actual = raw.format_version.value } } };
    if (raw.atlas.width.value != 1024) return descriptor(.width);
    if (raw.atlas.height.value != 1024) return descriptor(.height);
    const g = raw.generator;
    if (g.cell_size.value != 64) return descriptor(.cell_size);
    if (g.font_px.value != 40) return descriptor(.font_px);
    if (g.font_origin_x.value != 8) return descriptor(.font_origin_x);
    if (g.font_baseline_y.value != 48) return descriptor(.font_baseline_y);
    if (g.icon_size.value != 40) return descriptor(.icon_size);
    if (g.edge_threshold.value != 96) return descriptor(.edge_threshold);
    if (g.sdf_spread.value != 8) return descriptor(.sdf_spread);
    if (!std.mem.eql(u8, g.ordering.bytes, ordering)) return descriptor(.ordering);
    if (payload.byte_length != 1048576) return descriptor(.payload_length);
    if (raw.entries.len != ENTRY_COUNT) return .{ .failure = .{ .EntryCount = .{ .actual = raw.entries.len } } };
    // Check all duplicate keys before ordered mapping so duplicate diagnostics
    // remain distinct even when a duplicate substitutes an expected key.
    for (raw.entries, 0..) |entry, i| for (raw.entries[0..i]) |previous| {
        if (std.mem.eql(u8, entry.key.bytes, previous.key.bytes)) return .{ .failure = .{ .DuplicateKey = .{ .entry = i } } };
    };
    var candidate: AtlasMetrics = .{ .version = 1, .width = 1024, .height = 1024, .format = .r8_unorm, .generator = .{ .cell_size = 64, .font_px = 40, .font_origin_x = 8, .font_baseline_y = 48, .icon_size = 40, .edge_threshold = 96, .sdf_spread = 8 }, .entries = undefined };
    for (raw.entries, 0..) |entry, i| {
        var identity: @FieldType(AtlasEntry, "identity") = undefined;
        if (i < 190) {
            const codepoint: u21 = @intCast(32 + i % 95);
            const weight: Weight = if (i < 95) .Regular else .SemiBold;
            const name = if (i < 95) "regular" else "semibold";
            var key_buf: [32]u8 = undefined;
            const key = std.fmt.bufPrint(&key_buf, "{s}:U+{X:0>4}", .{ name, @as(u32, codepoint) }) catch unreachable;
            if (entry.kind.value != .glyph or entry.codepoint == null or entry.codepoint.?.value != codepoint or !optionalString(entry.font_weight, name) or entry.icon != null or !std.mem.eql(u8, entry.key.bytes, key)) return .{ .failure = .{ .InvalidMapping = .{ .entry = i } } };
            identity = .{ .glyph = .{ .codepoint = codepoint, .weight = weight } };
        } else {
            if (entry.kind.value != .icon or entry.codepoint != null or entry.font_weight != null or !optionalString(entry.icon, icon_names[i - 190]) or !std.mem.eql(u8, entry.key.bytes, icon_keys[i - 190])) return .{ .failure = .{ .InvalidMapping = .{ .entry = i } } };
            identity = .{ .icon = @fromBackingInt(@intCast(i - 190)) };
        }
        const b = entry.atlas_bounds;
        const x_end = std.math.add(u32, b.x.value, b.width.value) catch return .{ .failure = .{ .InvalidBounds = .{ .entry = i } } };
        const y_end = std.math.add(u32, b.y.value, b.height.value) catch return .{ .failure = .{ .InvalidBounds = .{ .entry = i } } };
        if (x_end > 1024 or y_end > 1024) return .{ .failure = .{ .InvalidBounds = .{ .entry = i } } };
        if (b.width.value != 64 or b.height.value != 64 or b.x.value % 64 != 0 or b.y.value % 64 != 0) return .{ .failure = .{ .InvalidCell = .{ .entry = i } } };
        if (!std.math.isFinite(entry.advance.value) or entry.advance.value < 0) return .{ .failure = .{ .InvalidAdvance = .{ .entry = i } } };
        const bounds: Bounds = .{ .x = b.x.value, .y = b.y.value, .width = b.width.value, .height = b.height.value };
        for (candidate.entries[0..i], 0..) |previous, j| {
            const p = previous.bounds;
            if (bounds.x < p.x + p.width and p.x < x_end and bounds.y < p.y + p.height and p.y < y_end) return .{ .failure = .{ .Overlap = .{ .first = j, .second = i } } };
        }
        candidate.entries[i] = .{ .bounds = bounds, .advance = entry.advance.value, .identity = identity };
    }
    return .{ .ok = candidate };
}
