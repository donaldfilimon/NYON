//! Allocation-free staged text preparation over the existing frozen atlas.
const std = @import("std");
const atlas_types = @import("retained_atlas.zig");
pub const Outcome = atlas_types.Outcome;
pub const Diagnostic = atlas_types.Diagnostic;
pub const Resource = atlas_types.Resource;
pub const AtlasView = atlas_types.AtlasView;
pub const Measurement = atlas_types.Measurement;
pub const Boundary = atlas_types.Boundary;
pub const GlyphPlacement = atlas_types.GlyphPlacement;
pub const PlacementRequest = atlas_types.PlacementRequest;
pub const TextLimits = atlas_types.TextLimits;
pub const Weight = atlas_types.Weight;
pub const Rect = atlas_types.Rect;

const MAX_INPUT: usize = 65536;
const MAX_BOUNDARIES: usize = 65537;
const MAX_PLACEMENTS: usize = 8192;
const Range = struct { start: usize, end: usize };
const RangeError = error{ByteSizeOverflow};
fn range(start: usize, count: usize, size: usize) RangeError!Range {
    const bytes = std.math.mul(usize, count, size) catch return error.ByteSizeOverflow;
    return .{ .start = start, .end = std.math.add(usize, start, bytes) catch return error.ByteSizeOverflow };
}
fn sliceRange(comptime T: type, value: []const T) RangeError!Range {
    return range(@intFromPtr(value.ptr), value.len, @sizeOf(T));
}
fn objectRange(value: anytype) RangeError!Range {
    return range(@intFromPtr(value), 1, @sizeOf(@TypeOf(value.*)));
}
fn overlaps(first: Range, second: Range) bool {
    return first.start != first.end and second.start != second.end and first.start < second.end and second.start < first.end;
}
fn checkRanges(writable: []const Range, readable: []const Range) ?Diagnostic {
    for (writable, 0..) |output, i| {
        for (writable[0..i]) |previous| if (overlaps(output, previous)) return .Aliasing;
        for (readable) |input| if (overlaps(output, input)) return .Aliasing;
    }
    return null;
}
fn limit(resource: Resource, actual: usize, max: usize) Diagnostic {
    return .{ .LimitExceeded = .{ .resource = resource, .actual = actual, .max = max } };
}
fn validateLimits(limits: TextLimits, text: []const u8) ?Diagnostic {
    if (limits.max_input_bytes > MAX_INPUT) return limit(.input_bytes, limits.max_input_bytes, MAX_INPUT);
    if (limits.max_boundaries > MAX_BOUNDARIES) return limit(.boundaries, limits.max_boundaries, MAX_BOUNDARIES);
    if (limits.max_placements > MAX_PLACEMENTS) return limit(.placements, limits.max_placements, MAX_PLACEMENTS);
    if (text.len > limits.max_input_bytes) return .{ .InputTooLarge = .{ .actual = text.len, .max = limits.max_input_bytes } };
    return null;
}
fn stagingBytes(count: usize, size: usize) RangeError!usize {
    return std.math.mul(usize, count, size) catch return error.ByteSizeOverflow;
}

pub fn measure(atlas: AtlasView, font_size: f32, weight: Weight, text: []const u8, limits: TextLimits) Outcome(Measurement) {
    if (validateLimits(limits, text)) |diagnostic| return .{ .failure = diagnostic };
    // Even read-only calls reject unrepresentable readable ranges.
    _ = objectRange(atlas) catch return .{ .failure = .ByteSizeOverflow };
    _ = sliceRange(u8, text) catch return .{ .failure = .ByteSizeOverflow };
    return switch (walk(atlas, font_size, weight, text, null, .none)) {
        .failure => |diagnostic| .{ .failure = diagnostic },
        .ok => |result| .{ .ok = .{ .advance = result.advance } },
    };
}

pub fn replaceBoundaries(atlas: AtlasView, font_size: f32, weight: Weight, text: []const u8, limits: TextLimits, destination: []Boundary, count: *usize, staging: []Boundary) Outcome(Measurement) {
    // The approved value parameters expose local copies, never caller record addresses.
    const local_limits = limits;
    const local_font = font_size;
    const local_weight = weight;
    const local_text = text;
    if (validateLimits(local_limits, local_text)) |diagnostic| return .{ .failure = diagnostic };
    const writes = [_]Range{
        sliceRange(Boundary, destination) catch return .{ .failure = .ByteSizeOverflow },
        sliceRange(Boundary, staging) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(count) catch return .{ .failure = .ByteSizeOverflow },
    };
    const reads = [_]Range{
        objectRange(atlas) catch return .{ .failure = .ByteSizeOverflow },
        sliceRange(u8, local_text) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_limits) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_font) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_weight) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_text) catch return .{ .failure = .ByteSizeOverflow },
    };
    if (checkRanges(&writes, &reads)) |diagnostic| return .{ .failure = diagnostic };
    if (count.* > destination.len) return .{ .failure = .{ .InvalidCount = .{ .count = count.*, .capacity = destination.len } } };
    const candidate = switch (walk(atlas, local_font, local_weight, local_text, null, .none)) {
        .failure => |diagnostic| return .{ .failure = diagnostic },
        .ok => |result| result,
    };
    if (candidate.boundaries > local_limits.max_boundaries) return .{ .failure = limit(.boundaries, candidate.boundaries, local_limits.max_boundaries) };
    const bytes = stagingBytes(candidate.boundaries, @sizeOf(Boundary)) catch return .{ .failure = .ByteSizeOverflow };
    if (bytes > local_limits.max_staged_bytes) return .{ .failure = limit(.staged_bytes, bytes, local_limits.max_staged_bytes) };
    if (candidate.boundaries > destination.len) return .{ .failure = .{ .DestinationCapacity = .{ .required = candidate.boundaries, .available = destination.len } } };
    if (candidate.boundaries > staging.len) return .{ .failure = .{ .StagingCapacity = .{ .required = candidate.boundaries, .available = staging.len } } };
    // This exact same strict-f32 walk fills only after whole-input admission.
    switch (walk(atlas, local_font, local_weight, local_text, null, .{ .boundaries = staging[0..candidate.boundaries] })) {
        .failure => |diagnostic| return .{ .failure = diagnostic },
        .ok => {},
    }
    @memcpy(destination[0..candidate.boundaries], staging[0..candidate.boundaries]);
    count.* = candidate.boundaries;
    return .{ .ok = .{ .advance = candidate.advance } };
}

pub fn appendPlacements(atlas: AtlasView, request: PlacementRequest, limits: TextLimits, destination: []GlyphPlacement, count: *usize, staging: []GlyphPlacement) Outcome(Measurement) {
    const local_request = request;
    const local_limits = limits;
    if (validateLimits(local_limits, local_request.text)) |diagnostic| return .{ .failure = diagnostic };
    const writes = [_]Range{
        sliceRange(GlyphPlacement, destination) catch return .{ .failure = .ByteSizeOverflow },
        sliceRange(GlyphPlacement, staging) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(count) catch return .{ .failure = .ByteSizeOverflow },
    };
    const reads = [_]Range{
        objectRange(atlas) catch return .{ .failure = .ByteSizeOverflow },
        sliceRange(u8, local_request.text) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_request) catch return .{ .failure = .ByteSizeOverflow },
        objectRange(&local_limits) catch return .{ .failure = .ByteSizeOverflow },
    };
    if (checkRanges(&writes, &reads)) |diagnostic| return .{ .failure = diagnostic };
    const old_count = count.*;
    if (old_count > destination.len) return .{ .failure = .{ .InvalidCount = .{ .count = old_count, .capacity = destination.len } } };
    if (old_count > local_limits.max_placements) return .{ .failure = limit(.placements, old_count, local_limits.max_placements) };
    const candidate = switch (walk(atlas, local_request.font_size, local_request.weight, local_request.text, &local_request, .none)) {
        .failure => |diagnostic| return .{ .failure = diagnostic },
        .ok => |result| result,
    };
    const total = std.math.add(usize, old_count, candidate.placements) catch return .{ .failure = .ByteSizeOverflow };
    if (total > local_limits.max_placements) return .{ .failure = limit(.placements, total, local_limits.max_placements) };
    const bytes = stagingBytes(candidate.placements, @sizeOf(GlyphPlacement)) catch return .{ .failure = .ByteSizeOverflow };
    if (bytes > local_limits.max_staged_bytes) return .{ .failure = limit(.staged_bytes, bytes, local_limits.max_staged_bytes) };
    if (total > destination.len) return .{ .failure = .{ .DestinationCapacity = .{ .required = total, .available = destination.len } } };
    if (candidate.placements > staging.len) return .{ .failure = .{ .StagingCapacity = .{ .required = candidate.placements, .available = staging.len } } };
    switch (walk(atlas, local_request.font_size, local_request.weight, local_request.text, &local_request, .{ .placements = staging[0..candidate.placements] })) {
        .failure => |diagnostic| return .{ .failure = diagnostic },
        .ok => {},
    }
    @memcpy(destination[old_count..total], staging[0..candidate.placements]);
    count.* = total;
    return .{ .ok = .{ .advance = candidate.advance } };
}

const Stats = struct { advance: f32, boundaries: usize, placements: usize };
const Sink = union(enum) { none, boundaries: []Boundary, placements: []GlyphPlacement };
const FloatError = error{NonFiniteGeometry};
fn finite(value: f32) FloatError!f32 {
    @setFloatMode(.strict);
    if (!std.math.isFinite(value)) return error.NonFiniteGeometry;
    return value;
}
fn add(x: f32, y: f32) FloatError!f32 {
    @setFloatMode(.strict);
    return finite(x + y);
}
fn sub(x: f32, y: f32) FloatError!f32 {
    @setFloatMode(.strict);
    return finite(x - y);
}
fn mul(x: f32, y: f32) FloatError!f32 {
    @setFloatMode(.strict);
    return finite(x * y);
}
fn div(x: f32, y: f32) FloatError!f32 {
    @setFloatMode(.strict);
    return finite(x / y);
}
fn scaleFor(atlas: AtlasView, font_size: f32) FloatError!f32 {
    @setFloatMode(.strict);
    _ = try finite(font_size);
    _ = try finite(atlas.generator.font_px);
    if (font_size < 0 or atlas.generator.font_px <= 0) return error.NonFiniteGeometry;
    return div(font_size, atlas.generator.font_px);
}
fn validateRequest(request: *const PlacementRequest) FloatError!void {
    @setFloatMode(.strict);
    for (request.baseline) |value| _ = try finite(value);
    for (request.color) |value| _ = try finite(value);
    if (request.clip) |clip| {
        _ = Rect.fromBounds(clip.min, clip.max) catch return error.NonFiniteGeometry;
        _ = try sub(clip.max[0], clip.min[0]);
        _ = try sub(clip.max[1], clip.min[1]);
    }
}

/// Shared preflight/fill traversal: strict scalar-order operations and original
/// byte offsets. The sink is inactive for admission; no output cap cuts this walk.
fn walk(atlas: AtlasView, font_size: f32, weight: Weight, text: []const u8, request: ?*const PlacementRequest, sink: Sink) Outcome(Stats) {
    @setFloatMode(.strict);
    const scale = scaleFor(atlas, font_size) catch return .{ .failure = .NonFiniteGeometry };
    if (request) |r| validateRequest(r) catch return .{ .failure = .NonFiniteGeometry };
    var result: Stats = .{ .advance = 0, .boundaries = 0, .placements = 0 };
    var byte_offset: usize = 0;
    while (byte_offset < text.len) {
        const length = std.unicode.utf8ByteSequenceLength(text[byte_offset]) catch return .{ .failure = .{ .InvalidUtf8 = .{ .byte_offset = byte_offset } } };
        if (length > text.len - byte_offset) return .{ .failure = .{ .InvalidUtf8 = .{ .byte_offset = byte_offset } } };
        const cp = std.unicode.utf8Decode(text[byte_offset..][0..length]) catch return .{ .failure = .{ .InvalidUtf8 = .{ .byte_offset = byte_offset } } };
        if (cp < 32 or cp > 126) return .{ .failure = .{ .UnsupportedCharacter = .{ .codepoint = cp, .byte_offset = byte_offset } } };
        const index: usize = @as(usize, cp - 32) + @as(usize, if (weight == .Regular) 0 else 95);
        const entry = &atlas.entries[index];
        switch (entry.identity) {
            .glyph => |identity| if (identity.codepoint != cp or identity.weight != weight) return .{ .failure = .{ .MissingEntry = .{ .index = index } } },
            .icon => return .{ .failure = .{ .MissingEntry = .{ .index = index } } },
        }
        if (!std.math.isFinite(entry.advance) or entry.advance < 0) return .{ .failure = .{ .InvalidAdvance = .{ .entry = index } } };
        if (request) |r| {
            // Validate full geometry before clipping, including completely hidden glyphs.
            if (cp != 32) {
                const glyph = switch (placement(atlas, entry, index, result.advance, scale, r)) {
                    .failure => |diagnostic| return .{ .failure = diagnostic },
                    .ok => |value| value,
                };
                if (glyph) |value| {
                    switch (sink) {
                        .placements => |output| output[result.placements] = value,
                        else => {},
                    }
                    result.placements = std.math.add(usize, result.placements, 1) catch return .{ .failure = .ByteSizeOverflow };
                }
            }
        }
        switch (sink) {
            .boundaries => |output| output[result.boundaries] = .{ .byte_offset = byte_offset, .advance_before = result.advance },
            else => {},
        }
        result.boundaries = std.math.add(usize, result.boundaries, 1) catch return .{ .failure = .ByteSizeOverflow };
        const step = mul(entry.advance, scale) catch return .{ .failure = .NonFiniteGeometry };
        result.advance = add(result.advance, step) catch return .{ .failure = .NonFiniteGeometry };
        byte_offset += length;
    }
    switch (sink) {
        .boundaries => |output| output[result.boundaries] = .{ .byte_offset = text.len, .advance_before = result.advance },
        else => {},
    }
    result.boundaries = std.math.add(usize, result.boundaries, 1) catch return .{ .failure = .ByteSizeOverflow };
    return .{ .ok = result };
}

fn placement(atlas: AtlasView, entry: *const atlas_types.AtlasEntry, index: usize, advance: f32, scale: f32, request: *const PlacementRequest) Outcome(?GlyphPlacement) {
    @setFloatMode(.strict);
    if (atlas.width != 1024) return .{ .failure = .{ .DescriptorMismatch = .{ .field = .width } } };
    if (atlas.height != 1024) return .{ .failure = .{ .DescriptorMismatch = .{ .field = .height } } };
    const b = entry.bounds;
    const right = std.math.add(u32, b.x, b.width) catch return .{ .failure = .{ .InvalidBounds = .{ .entry = index } } };
    const bottom = std.math.add(u32, b.y, b.height) catch return .{ .failure = .{ .InvalidBounds = .{ .entry = index } } };
    if (b.width == 0 or b.height == 0 or right > 1024 or bottom > 1024) return .{ .failure = .{ .InvalidBounds = .{ .entry = index } } };
    if (atlas.generator.cell_size == 0 or b.width != atlas.generator.cell_size or b.height != atlas.generator.cell_size) return .{ .failure = .{ .InvalidCell = .{ .entry = index } } };
    return .{ .ok = derivePlacement(atlas, entry, advance, scale, request) catch return .{ .failure = .NonFiniteGeometry } };
}
fn derivePlacement(atlas: AtlasView, entry: *const atlas_types.AtlasEntry, advance: f32, scale: f32, request: *const PlacementRequest) FloatError!?GlyphPlacement {
    @setFloatMode(.strict);
    const origin = try mul(@floatFromInt(atlas.generator.font_origin_x), scale);
    const baseline = try mul(@floatFromInt(atlas.generator.font_baseline_y), scale);
    const x = try sub(try add(request.baseline[0], advance), origin);
    const y = try sub(request.baseline[1], baseline);
    const size = try mul(@floatFromInt(atlas.generator.cell_size), scale);
    const max: [2]f32 = .{ try add(x, size), try add(y, size) };
    _ = try sub(max[0], x);
    _ = try sub(max[1], y);
    var glyph: GlyphPlacement = .{
        .rect = .{ x, y, size, size },
        .uv = .{ try div(@floatFromInt(entry.bounds.x), 1024), try div(@floatFromInt(entry.bounds.y), 1024), try div(@floatFromInt(entry.bounds.width), 1024), try div(@floatFromInt(entry.bounds.height), 1024) },
        .color = request.color,
    };
    // Validate the complete base UV extent before any culling.
    _ = try add(glyph.uv[0], glyph.uv[2]);
    _ = try add(glyph.uv[1], glyph.uv[3]);
    if (request.clip) |clip| {
        if (size == 0) return null;
        const rect: Rect = .{ .min = .{ x, y }, .max = max };
        const hit = rect.intersection(clip) orelse return null;
        const x0 = try div(try sub(hit.min[0], x), size);
        const y0 = try div(try sub(hit.min[1], y), size);
        const x1 = try div(try sub(hit.max[0], x), size);
        const y1 = try div(try sub(hit.max[1], y), size);
        glyph.rect = .{ hit.min[0], hit.min[1], try sub(hit.max[0], hit.min[0]), try sub(hit.max[1], hit.min[1]) };
        glyph.uv = .{ try add(glyph.uv[0], try mul(glyph.uv[2], x0)), try add(glyph.uv[1], try mul(glyph.uv[3], y0)), try mul(glyph.uv[2], try sub(x1, x0)), try mul(glyph.uv[3], try sub(y1, y0)) };
        _ = try add(glyph.uv[0], glyph.uv[2]);
        _ = try add(glyph.uv[1], glyph.uv[3]);
    }
    return glyph;
}

// Helper-only coverage: inaccessible original caller records and impossible
// address/length arithmetic are not claimed as reachable public API fixtures.
test "text checked range staged byte overflow and local readable record helper" {
    const t = std.testing;
    try t.expectError(error.ByteSizeOverflow, range(std.math.maxInt(usize), 1, 1));
    try t.expectError(error.ByteSizeOverflow, range(0, std.math.maxInt(usize), 48));
    try t.expectError(error.ByteSizeOverflow, stagingBytes(std.math.maxInt(usize), @sizeOf(GlyphPlacement)));
    try t.expectEqual(@as(usize, 48), try stagingBytes(1, @sizeOf(GlyphPlacement)));
    var local_request: PlacementRequest = .{ .baseline = .{ 0, 0 }, .font_size = 40, .weight = .Regular, .color = .{ 1, 1, 1, 1 }, .text = "", .clip = null };
    var local_limits: TextLimits = .{ .max_input_bytes = 0, .max_boundaries = 0, .max_placements = 0, .max_staged_bytes = 0 };
    const request_range = try objectRange(&local_request);
    const limits_range = try objectRange(&local_limits);
    try t.expectEqualDeep(Diagnostic.Aliasing, checkRanges(&.{request_range}, &.{request_range}).?);
    try t.expectEqualDeep(Diagnostic.Aliasing, checkRanges(&.{limits_range}, &.{limits_range}).?);
    try t.expect(checkRanges(&.{}, &.{ request_range, request_range, limits_range }) == null);
    try t.expect(!overlaps(try range(0, 0, 48), try range(0, 1, 48)));
    try t.expect(!overlaps(try range(0, 1, 48), try range(48, 1, 48)));
    std.debug.print("H2b sizes AtlasMetrics={d} Boundary={d} GlyphPlacement={d} PlacementRequest={d} TextLimits={d} Measurement={d} OutcomeMeasurement={d}; caller boundary per slice={d}, pair={d}; placement per slice={d}, pair={d}\n", .{ @sizeOf(atlas_types.AtlasMetrics), @sizeOf(Boundary), @sizeOf(GlyphPlacement), @sizeOf(PlacementRequest), @sizeOf(TextLimits), @sizeOf(Measurement), @sizeOf(Outcome(Measurement)), 65537 * @sizeOf(Boundary), 2 * 65537 * @sizeOf(Boundary), 8192 * @sizeOf(GlyphPlacement), 2 * 8192 * @sizeOf(GlyphPlacement) });
}
