const std = @import("std");
const atlas = @import("retained_atlas.zig");
const t = std.testing;
const a = t.allocator;
const payload: atlas.PayloadDescriptor = .{ .byte_length = 1048576 };
fn frozen() ![]u8 {
    return std.Io.Dir.cwd().readFileAlloc(t.io, "assets/ui/atlas-metrics.json", a, .limited(65536)) catch |err| switch (err) {
        error.FileNotFound => std.Io.Dir.cwd().readFileAlloc(t.io, "../assets/ui/atlas-metrics.json", a, .limited(65536)),
        else => return err,
    };
}
fn failure(value: anytype) !atlas.Diagnostic {
    return switch (value) {
        .failure => |f| f,
        .ok => error.ExpectedFailure,
    };
}
test "atlas admission length checksum scratch diagnostics" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048577);
    defer a.free(scratch);
    const large = try a.alloc(u8, 65537);
    defer a.free(large);
    @memset(large, ' ');
    try t.expectEqualDeep(atlas.Diagnostic{ .InputTooLarge = .{ .actual = 65537, .max = 65536 } }, try failure(atlas.loadFrozenMetadata(large, payload, scratch)));
    var digest: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash("{", &digest, .{});
    try t.expectEqualDeep(atlas.Diagnostic{ .ChecksumMismatch = .{ .expected = atlas.FROZEN_SHA256, .actual = digest } }, try failure(atlas.loadFrozenMetadata("{", payload, scratch[0..0])));
    try t.expectEqualDeep(atlas.Diagnostic{ .ScratchTooSmall = .{ .actual = 1048575, .required = 1048576 } }, try failure(atlas.loadFrozenMetadata(bytes, payload, scratch[0..1048575])));
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidJson = .{ .cause = .InvalidSyntax } }, try failure(atlas.decodeFixtureMetadata("{", payload, scratch)));
}
fn success(value: anytype) !atlas.AtlasMetrics {
    return switch (value) {
        .ok => |v| v,
        .failure => |f| {
            std.debug.print("unexpected {any}\n", .{f});
            return error.ExpectedSuccess;
        },
    };
}
test "atlas full frozen mapping owned table prefix and replacement atomicity" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048577);
    defer a.free(scratch);
    @memset(scratch, 0x55);
    var resident = try success(atlas.loadFrozenMetadata(bytes, payload, scratch[0..1048576]));
    const view: atlas.AtlasView = &resident;
    const old = resident;
    const old_bytes = try a.dupe(u8, std.mem.asBytes(&resident));
    defer a.free(old_bytes);
    try t.expectEqual(@as(u32, 1), resident.version);
    try t.expectEqual(@as(u32, 1024), resident.width);
    try t.expectEqual(@as(u32, 1024), resident.height);
    try t.expectEqual(atlas.AtlasFormat.r8_unorm, resident.format);
    try t.expectEqualDeep(atlas.Generator{ .cell_size = 64, .font_px = 40, .font_origin_x = 8, .font_baseline_y = 48, .icon_size = 40, .edge_threshold = 96, .sdf_spread = 8 }, resident.generator);
    const icons = [_]atlas.Icon{ .Play, .Pause, .Speed, .Reset, .Settings, .Help, .Crosshair, .Save, .Load, .Check, .Close, .Zoom, .FieldAtmosphere, .FieldHydrosphere, .FieldTopology };
    for (resident.entries, 0..) |entry, i| {
        try t.expectEqualDeep(atlas.Bounds{ .x = @intCast(i % 16 * 64), .y = @intCast(i / 16 * 64), .width = 64, .height = 64 }, entry.bounds);
        if (i < 190) try t.expectEqualDeep(@TypeOf(entry.identity){ .glyph = .{ .codepoint = @intCast(32 + i % 95), .weight = if (i < 95) .Regular else .SemiBold } }, entry.identity) else try t.expectEqualDeep(@TypeOf(entry.identity){ .icon = icons[i - 190] }, entry.identity);
        try t.expect(std.math.isFinite(entry.advance) and entry.advance >= 0);
    }
    try t.expectEqual(@as(f32, 11.25), resident.entries[0].advance);
    try t.expectEqual(@as(f32, 27.597656), resident.entries[33].advance);
    try t.expectEqual(@as(f32, 29.101562), resident.entries[128].advance);
    _ = try success(atlas.loadFrozenMetadata(bytes, payload, scratch));
    try t.expectEqual(@as(u8, 0x55), scratch[1048576]);
    try t.expectEqualDeep(atlas.Diagnostic{ .ChecksumMismatch = .{ .expected = atlas.FROZEN_SHA256, .actual = blk: {
        var h: [32]u8 = undefined;
        std.crypto.hash.sha2.Sha256.hash("bad", &h, .{});
        break :blk h;
    } } }, try failure(atlas.replaceFrozenMetadata(&resident, "bad", payload, scratch)));
    try t.expectEqualDeep(old, view.*);
    try t.expectEqualSlices(u8, old_bytes, std.mem.asBytes(&resident));
    @memset(bytes, 0xee);
    @memset(scratch, 0xff);
    try t.expectEqualDeep(old, view.*);
    try t.expectEqualSlices(u8, old_bytes, std.mem.asBytes(&resident));
    std.debug.print("AtlasEntry={d} AtlasMetrics={d} Outcome={d} Generator={d} Boundary={d} GlyphPlacement={d} caller scratch={d}; no embedded scratch\n", .{ @sizeOf(atlas.AtlasEntry), @sizeOf(atlas.AtlasMetrics), @sizeOf(atlas.Outcome(atlas.AtlasMetrics)), @sizeOf(atlas.Generator), @sizeOf(atlas.Boundary), @sizeOf(atlas.GlyphPlacement), atlas.SCRATCH_BYTES });
}
test "atlas full scratch and resident alias rejection before writes" {
    const bytes = try frozen();
    defer a.free(bytes);
    const storage = try a.alignedAlloc(u8, .of(atlas.AtlasMetrics), 1048576 + 65536);
    defer a.free(storage);
    @memset(storage, 0x33);
    @memcpy(storage[1048576..][0..bytes.len], bytes);
    const before = try a.dupe(u8, storage);
    defer a.free(before);
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(atlas.decodeFixtureMetadata(storage[1048576..][0..bytes.len], payload, storage)));
    try t.expectEqualSlices(u8, before, storage);
    const resident: *atlas.AtlasMetrics = @ptrCast(storage.ptr);
    const resident_before = try a.dupe(u8, std.mem.asBytes(resident));
    defer a.free(resident_before);
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(atlas.replaceFrozenMetadata(resident, bytes, payload, storage)));
    try t.expectEqualSlices(u8, resident_before, std.mem.asBytes(resident));
}
test "atlas independent schema mutations exact diagnostic categories" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048576);
    defer a.free(scratch);
    const Case = struct { needle: []const u8, replacement: []const u8, expected: atlas.Diagnostic };
    const cases = [_]Case{
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": 2", .expected = .{ .UnsupportedVersion = .{ .actual = 2 } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": \"1\"", .expected = .{ .InvalidJson = .{ .cause = .UnexpectedToken } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": 1.0", .expected = .{ .InvalidJson = .{ .cause = .NumericRange } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": 4294967296", .expected = .{ .InvalidJson = .{ .cause = .NumericRange } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": -1", .expected = .{ .InvalidJson = .{ .cause = .NumericRange } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"format_version\": 1, \"format_version\": 1", .expected = .{ .InvalidJson = .{ .cause = .DuplicateField } } },
        .{ .needle = "\"format_version\": 1", .replacement = "\"unexpected\": 1, \"format_version\": 1", .expected = .{ .InvalidJson = .{ .cause = .UnknownField } } },
        .{ .needle = "\"format_version\": 1,", .replacement = "", .expected = .{ .InvalidJson = .{ .cause = .MissingField } } },
        .{ .needle = "\"font_px\": 40.0", .replacement = "\"font_px\": \"40.0\"", .expected = .{ .InvalidJson = .{ .cause = .UnexpectedToken } } },
        .{ .needle = "\"font_px\": 40.0", .replacement = "\"font_px\": 1e400", .expected = .{ .DescriptorMismatch = .{ .field = .font_px } } },
        .{ .needle = "\"format\": \"r8_unorm\"", .replacement = "\"format\": 0", .expected = .{ .InvalidJson = .{ .cause = .UnexpectedToken } } },
        .{ .needle = "\"format\": \"r8_unorm\"", .replacement = "\"format\": \"r8unorm\"", .expected = .{ .InvalidJson = .{ .cause = .InvalidEnum } } },
        .{ .needle = "\"width\": 1024", .replacement = "\"width\": 512", .expected = .{ .DescriptorMismatch = .{ .field = .width } } },
        .{ .needle = "\"height\": 1024", .replacement = "\"height\": 512", .expected = .{ .DescriptorMismatch = .{ .field = .height } } },
        .{ .needle = "\"cell_size\": 64", .replacement = "\"cell_size\": 32", .expected = .{ .DescriptorMismatch = .{ .field = .cell_size } } },
        .{ .needle = "\"font_origin_x\": 8", .replacement = "\"font_origin_x\": -2147483649", .expected = .{ .InvalidJson = .{ .cause = .NumericRange } } },
        .{ .needle = "\"edge_threshold\": 96", .replacement = "\"edge_threshold\": 256", .expected = .{ .InvalidJson = .{ .cause = .NumericRange } } },
        .{ .needle = "\"key\": \"regular:U+0020\"", .replacement = "\"key\": \"wrong\"", .expected = .{ .InvalidMapping = .{ .entry = 0 } } },
        .{ .needle = "\"key\": \"regular:U+0021\"", .replacement = "\"key\": \"regular:U+0020\"", .expected = .{ .DuplicateKey = .{ .entry = 1 } } },
        .{ .needle = "\"codepoint\": 32", .replacement = "\"codepoint\": 31", .expected = .{ .InvalidMapping = .{ .entry = 0 } } },
        .{ .needle = "\"codepoint\": 32,", .replacement = "", .expected = .{ .InvalidMapping = .{ .entry = 0 } } },
        .{ .needle = "\"font_weight\": \"regular\"", .replacement = "\"font_weight\": \"semiBold\"", .expected = .{ .InvalidMapping = .{ .entry = 0 } } },
        .{ .needle = "\"kind\": \"glyph\"", .replacement = "\"kind\": \"icon\"", .expected = .{ .InvalidMapping = .{ .entry = 0 } } },
        .{ .needle = "\"x\": 64", .replacement = "\"x\": 0", .expected = .{ .Overlap = .{ .first = 0, .second = 1 } } },
        .{ .needle = "\"x\": 0", .replacement = "\"x\": 1", .expected = .{ .InvalidCell = .{ .entry = 0 } } },
        .{ .needle = "\"width\": 64", .replacement = "\"width\": 0", .expected = .{ .InvalidCell = .{ .entry = 0 } } },
        .{ .needle = "\"x\": 0", .replacement = "\"x\": 4294967295", .expected = .{ .InvalidBounds = .{ .entry = 0 } } },
        .{ .needle = "\"advance\": 11.25", .replacement = "\"advance\": -1", .expected = .{ .InvalidAdvance = .{ .entry = 0 } } },
        .{ .needle = "\"advance\": 11.25", .replacement = "\"advance\": 1e400", .expected = .{ .InvalidAdvance = .{ .entry = 0 } } },
        .{ .needle = "\"icon\": \"play\"", .replacement = "\"icon\": \"pause\"", .expected = .{ .InvalidMapping = .{ .entry = 190 } } },
        .{ .needle = "\"icon\": \"play\"", .replacement = "\"icon\": \"play\", \"codepoint\": 32", .expected = .{ .InvalidMapping = .{ .entry = 190 } } },
    };
    for (cases) |c| {
        const mutated = try replaceFirst(bytes, c.needle, c.replacement);
        defer a.free(mutated);
        try t.expect(!std.mem.eql(u8, bytes, mutated));
        try t.expectEqualDeep(c.expected, try failure(atlas.decodeFixtureMetadata(mutated, payload, scratch)));
    }
    const trailing = try std.mem.concat(a, u8, &.{ bytes, " true" });
    defer a.free(trailing);
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidJson = .{ .cause = .InvalidSyntax } }, try failure(atlas.decodeFixtureMetadata(trailing, payload, scratch)));
    try t.expectEqualDeep(atlas.Diagnostic{ .DescriptorMismatch = .{ .field = .payload_length } }, try failure(atlas.decodeFixtureMetadata(bytes, .{ .byte_length = 1048575 }, scratch)));
    try t.expectEqualDeep(atlas.Diagnostic{ .EntryCount = .{ .actual = 0 } }, try failure(atlas.decodeFixtureMetadata("{\"format_version\":1,\"atlas\":{\"width\":1024,\"height\":1024,\"format\":\"r8_unorm\"},\"generator\":{\"cell_size\":64,\"font_px\":40,\"font_origin_x\":8,\"font_baseline_y\":48,\"icon_size\":40,\"edge_threshold\":96,\"sdf_spread\":8,\"ordering\":\"regular ASCII U+0020..U+007E; semibold ASCII U+0020..U+007E; frozen icons in source order\"},\"entries\":[]}", payload, scratch)));
}

fn replaceFirst(bytes: []const u8, needle: []const u8, replacement: []const u8) ![]u8 {
    const at = std.mem.indexOf(u8, bytes, needle) orelse return error.MissingMutationNeedle;
    return std.mem.concat(a, u8, &.{ bytes[0..at], replacement, bytes[at + needle.len ..] });
}
test "atlas counts token ceilings schema descriptor types and replacement preservation" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048576);
    defer a.free(scratch);
    var resident = try success(atlas.loadFrozenMetadata(bytes, payload, scratch));
    const before = resident;
    const before_bytes = try a.dupe(u8, std.mem.asBytes(&resident));
    defer a.free(before_bytes);
    const view: atlas.AtlasView = &resident;
    try t.expectEqualDeep(atlas.Diagnostic{ .DescriptorMismatch = .{ .field = .payload_length } }, try failure(atlas.replaceFrozenMetadata(&resident, bytes, .{ .byte_length = 0 }, scratch)));
    try t.expectEqualDeep(before, view.*);
    try t.expectEqualSlices(u8, before_bytes, std.mem.asBytes(&resident));
    try t.expectEqualDeep(atlas.Diagnostic{ .ScratchTooSmall = .{ .actual = 0, .required = 1048576 } }, try failure(atlas.replaceFrozenMetadata(&resident, bytes, payload, scratch[0..0])));
    try t.expectEqualDeep(before, view.*);
    try t.expectEqualSlices(u8, before_bytes, std.mem.asBytes(&resident));
    switch (atlas.replaceFrozenMetadata(&resident, bytes, payload, scratch)) {
        .ok => {},
        .failure => return error.ExpectedSuccess,
    }
    try t.expectEqualDeep(before, resident);
    const entries_start = std.mem.indexOf(u8, bytes, "    {\n      \"key\"").?;
    const first_end = std.mem.indexOfPos(u8, bytes, entries_start, "\n    },\n").? + 8;
    const missing = try std.mem.concat(a, u8, &.{ bytes[0..entries_start], bytes[first_end..] });
    defer a.free(missing);
    try t.expectEqualDeep(atlas.Diagnostic{ .EntryCount = .{ .actual = 204 } }, try failure(atlas.decodeFixtureMetadata(missing, payload, scratch)));
    const extra = try std.mem.concat(a, u8, &.{ bytes[0..entries_start], bytes[entries_start..first_end], bytes[entries_start..] });
    defer a.free(extra);
    try t.expectEqualDeep(atlas.Diagnostic{ .EntryCount = .{ .actual = 206 } }, try failure(atlas.decodeFixtureMetadata(extra, payload, scratch)));
    const long = try a.alloc(u8, 257);
    defer a.free(long);
    @memset(long, 'a');
    const value = try std.mem.concat(a, u8, &.{ "\"key\": \"", long, "\"" });
    defer a.free(value);
    const bad = try replaceFirst(bytes, "\"key\": \"regular:U+0020\"", value);
    defer a.free(bad);
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidJson = .{ .cause = .TokenTooLong } }, try failure(atlas.decodeFixtureMetadata(bad, payload, scratch)));
    const descriptors = [_]struct { needle: []const u8, replacement: []const u8, field: atlas.DescriptorField }{
        .{ .needle = "\"font_px\": 40.0", .replacement = "\"font_px\": 41", .field = .font_px },
        .{ .needle = "\"font_origin_x\": 8", .replacement = "\"font_origin_x\": 9", .field = .font_origin_x },
        .{ .needle = "\"font_baseline_y\": 48", .replacement = "\"font_baseline_y\": 47", .field = .font_baseline_y },
        .{ .needle = "\"icon_size\": 40", .replacement = "\"icon_size\": 41", .field = .icon_size },
        .{ .needle = "\"edge_threshold\": 96", .replacement = "\"edge_threshold\": 95", .field = .edge_threshold },
        .{ .needle = "\"sdf_spread\": 8", .replacement = "\"sdf_spread\": 9", .field = .sdf_spread },
        .{ .needle = "regular ASCII U+0020..U+007E", .replacement = "regular ASCII U+0020..U+007F", .field = .ordering },
    };
    for (descriptors) |d| {
        const b = try replaceFirst(bytes, d.needle, d.replacement);
        defer a.free(b);
        try t.expectEqualDeep(atlas.Diagnostic{ .DescriptorMismatch = .{ .field = d.field } }, try failure(atlas.decodeFixtureMetadata(b, payload, scratch)));
    }
    for ([_][]const u8{ "true", "null", "[]", "{}" }) |value_json| {
        const replacement = try std.mem.concat(a, u8, &.{ "\"format_version\": ", value_json });
        defer a.free(replacement);
        const b = try replaceFirst(bytes, "\"format_version\": 1", replacement);
        defer a.free(b);
        try t.expectEqualDeep(atlas.Diagnostic{ .InvalidJson = .{ .cause = .UnexpectedToken } }, try failure(atlas.decodeFixtureMetadata(b, payload, scratch)));
    }
    const impossible: [*]u8 = @ptrFromInt(std.math.maxInt(usize) - 16);
    try t.expectEqualDeep(atlas.Diagnostic.ByteSizeOverflow, try failure(atlas.decodeFixtureMetadata("", payload, impossible[0..1048576])));
    // Resident aliased by readable input must fail without inspecting its bytes.
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(atlas.replaceFrozenMetadata(&resident, std.mem.asBytes(&resident), payload, scratch)));
    try t.expectEqualDeep(before, view.*);
}
test "atlas bounded adversarial overcount public decoder" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048576);
    defer a.free(scratch);
    const begin = std.mem.indexOf(u8, bytes, "    {\n      \"key\"").?;
    const patterns = [_][]const u8{
        "{\"key\":\"\",\"kind\":\"glyph\",\"atlas_bounds\":{\"x\":0,\"y\":0,\"width\":0,\"height\":0},\"advance\":0}",
        "{\"key\":\"\\u0061\",\"kind\":\"glyph\",\"font_weight\":\"\\u0061\",\"icon\":\"\\u0061\",\"atlas_bounds\":{\"x\":0,\"y\":0,\"width\":0,\"height\":0},\"advance\":0}",
    };
    for (patterns) |pattern| {
        var input: std.ArrayList(u8) = .empty;
        defer input.deinit(a);
        try input.appendSlice(a, bytes[0..begin]);
        var n: usize = 0;
        while (input.items.len + pattern.len + 4 <= 65536) {
            if (n != 0) try input.append(a, ',');
            try input.appendSlice(a, pattern);
            n += 1;
        }
        try input.appendSlice(a, "]}");
        const used = input.items.len;
        try input.resize(a, 65536);
        @memset(input.items[used..], ' ');
        const result = try failure(atlas.decodeFixtureMetadata(input.items, payload, scratch));
        std.debug.print("public adversarial bytes={d} raw entries={d} diagnostic={any}\n", .{ input.items.len, n, result });
        try t.expectEqualDeep(atlas.Diagnostic{ .EntryCount = .{ .actual = n } }, result);
    }
}
test "atlas all frozen numeric field transfers and maximum input ceiling" {
    const bytes = try frozen();
    defer a.free(bytes);
    const scratch = try a.alloc(u8, 1048576);
    defer a.free(scratch);
    const decoded = try success(atlas.loadFrozenMetadata(bytes, payload, scratch));
    // Separate generic source reader witnesses numeric transfer. Mapping/bounds
    // above use source/spec formulas; this is not generator provenance or a golden.
    const source = try std.json.parseFromSlice(std.json.Value, a, bytes, .{});
    defer source.deinit();
    const entries = source.value.object.get("entries").?.array.items;
    try t.expectEqual(@as(usize, 205), entries.len);
    for (entries, decoded.entries) |raw, entry| {
        const v = raw.object.get("advance").?;
        const expected: f32 = switch (v) {
            .float => |f| @floatCast(f),
            .integer => |n| @floatFromInt(n),
            else => return error.InvalidSourceNumeric,
        };
        try t.expectEqual(expected, entry.advance);
    }
    const padded = try a.alloc(u8, 65536);
    defer a.free(padded);
    @memcpy(padded[0..bytes.len], bytes);
    @memset(padded[bytes.len..], ' ');
    const max = try success(atlas.decodeFixtureMetadata(padded, payload, scratch));
    try t.expectEqualDeep(decoded, max);
    const escaped = try a.alloc(u8, 257 * 6);
    defer a.free(escaped);
    for (0..257) |i| @memcpy(escaped[i * 6 ..][0..6], "\\u0061");
    const replacement = try std.mem.concat(a, u8, &.{ "\"key\": \"", escaped, "\"" });
    defer a.free(replacement);
    const bad = try replaceFirst(bytes, "\"key\": \"regular:U+0020\"", replacement);
    defer a.free(bad);
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidJson = .{ .cause = .TokenTooLong } }, try failure(atlas.decodeFixtureMetadata(bad, payload, scratch)));
    const short_replacement = try std.mem.concat(a, u8, &.{ "\"key\": \"", escaped[0 .. 256 * 6], "\"" });
    defer a.free(short_replacement);
    const at_limit = try replaceFirst(bytes, "\"key\": \"regular:U+0020\"", short_replacement);
    defer a.free(at_limit);
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidMapping = .{ .entry = 0 } }, try failure(atlas.decodeFixtureMetadata(at_limit, payload, scratch)));
}

// H2b additions: expectations are the independently reviewed source literals.
const text_api = @import("retained_text.zig");
const text_limits: atlas.TextLimits = .{ .max_input_bytes = 65536, .max_boundaries = 65537, .max_placements = 8192, .max_staged_bytes = 1048592 };
fn textSuccess(value: anytype) !atlas.Measurement {
    return switch (value) {
        .ok => |v| v,
        .failure => |f| {
            std.debug.print("unexpected text diagnostic {any}\n", .{f});
            return error.ExpectedSuccess;
        },
    };
}
fn textAtlas() !atlas.AtlasMetrics {
    const bytes = try frozen();
    defer t.allocator.free(bytes);
    const scratch = try t.allocator.alloc(u8, atlas.SCRATCH_BYTES);
    defer t.allocator.free(scratch);
    return success(atlas.loadFrozenMetadata(bytes, payload, scratch));
}
fn request(value: []const u8) atlas.PlacementRequest {
    return .{ .baseline = .{ 100, 100 }, .font_size = 40, .weight = .Regular, .color = .{ -2, 3, 0.5, 1 }, .text = value, .clip = null };
}
fn expectPreserved(comptime T: type, before: []const u8, destination: []T, previous: usize, count: usize) !void {
    try t.expectEqualSlices(u8, before, std.mem.sliceAsBytes(destination));
    try t.expectEqual(previous, count);
}
test "text independent measurement literals and scalar terminal boundaries" {
    const metrics = try textAtlas();
    for ([_]struct { s: []const u8, size: f32, weight: atlas.Weight, advance: f32 }{
        .{ .s = "Aa", .size = 40, .weight = .Regular, .advance = 50.05859375 },
        .{ .s = "Aa", .size = 40, .weight = .SemiBold, .advance = 52.0703125 },
        .{ .s = "WWW", .size = 40, .weight = .Regular, .advance = 118.2421875 },
        .{ .s = "iii", .size = 40, .weight = .Regular, .advance = 29.0625 },
        .{ .s = "Aa", .size = 20, .weight = .Regular, .advance = 25.029296875 },
        .{ .s = " ", .size = 40, .weight = .Regular, .advance = 11.25 },
        .{ .s = "", .size = 40, .weight = .Regular, .advance = 0 },
    }) |v| try t.expectEqual(v.advance, (try textSuccess(text_api.measure(&metrics, v.size, v.weight, v.s, text_limits))).advance);
    var d: [4]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&d), 0x55);
    var stage: [3]atlas.Boundary = undefined;
    var count: usize = 4;
    const tail = std.mem.asBytes(&d[3]).*;
    const regular = [_]atlas.Boundary{ .{ .byte_offset = 0, .advance_before = 0 }, .{ .byte_offset = 1, .advance_before = 27.59765625 }, .{ .byte_offset = 2, .advance_before = 50.05859375 } };
    const semibold = [_]atlas.Boundary{ .{ .byte_offset = 0, .advance_before = 0 }, .{ .byte_offset = 1, .advance_before = 29.1015625 }, .{ .byte_offset = 2, .advance_before = 52.0703125 } };
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .Regular, "Aa", text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 3), count);
    try t.expectEqualDeep(regular, d[0..3].*);
    try t.expectEqualSlices(u8, &tail, std.mem.asBytes(&d[3]));
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .SemiBold, "Aa", text_limits, &d, &count, &stage));
    try t.expectEqualDeep(semibold, d[0..3].*);
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .Regular, "", text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 1), count);
    try t.expectEqualDeep(atlas.Boundary{ .byte_offset = 0, .advance_before = 0 }, d[0]);
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .Regular, " ", text_limits, &d, &count, &stage));
    try t.expectEqual(@as(f32, 11.25), d[1].advance_before);
}
test "text independent placement clipping four edges zero size and HDR lanes" {
    const metrics = try textAtlas();
    var d: [3]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&d), 0x44);
    var stage: [2]atlas.GlyphPlacement = undefined;
    var count: usize = 1;
    const prefix = std.mem.asBytes(&d[0]).*;
    const tail = std.mem.asBytes(&d[2]).*;
    var r = request("A");
    try t.expectEqual(@as(f32, 27.59765625), (try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage))).advance);
    try t.expectEqual(@as(usize, 2), count);
    try t.expectEqualDeep(atlas.GlyphPlacement{ .rect = .{ 92, 52, 64, 64 }, .uv = .{ 0.0625, 0.125, 0.0625, 0.0625 }, .color = r.color }, d[1]);
    try t.expectEqualSlices(u8, &prefix, std.mem.asBytes(&d[0]));
    try t.expectEqualSlices(u8, &tail, std.mem.asBytes(&d[2]));
    for ([_]struct { clip: atlas.Rect, rect: [4]f32, uv: [4]f32 }{
        .{ .clip = .{ .min = .{ 100, 60 }, .max = .{ 116, 76 } }, .rect = .{ 100, 60, 16, 16 }, .uv = .{ 0.0703125, 0.1328125, 0.015625, 0.015625 } },
        .{ .clip = .{ .min = .{ 124, 52 }, .max = .{ 200, 116 } }, .rect = .{ 124, 52, 32, 64 }, .uv = .{ 0.09375, 0.125, 0.03125, 0.0625 } },
        .{ .clip = .{ .min = .{ 0, 52 }, .max = .{ 124, 116 } }, .rect = .{ 92, 52, 32, 64 }, .uv = .{ 0.0625, 0.125, 0.03125, 0.0625 } },
        .{ .clip = .{ .min = .{ 92, 84 }, .max = .{ 156, 200 } }, .rect = .{ 92, 84, 64, 32 }, .uv = .{ 0.0625, 0.15625, 0.0625, 0.03125 } },
        .{ .clip = .{ .min = .{ 92, 0 }, .max = .{ 156, 84 } }, .rect = .{ 92, 52, 64, 32 }, .uv = .{ 0.0625, 0.125, 0.0625, 0.03125 } },
    }) |v| {
        r.clip = v.clip;
        count = 0;
        _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
        try t.expectEqual(@as(usize, 1), count);
        try t.expectEqualDeep(v.rect, d[0].rect);
        try t.expectEqualDeep(v.uv, d[0].uv);
    }
    for ([_]atlas.Rect{ .{ .min = .{ 156, 52 }, .max = .{ 200, 116 } }, .{ .min = .{ 0, 0 }, .max = .{ 0, 0 } }, .{ .min = .{ 0, 0 }, .max = .{ 1, 1 } } }) |clip| {
        r.clip = clip;
        count = 0;
        _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
        try t.expectEqual(@as(usize, 0), count);
    }
    r = request("A");
    r.font_size = 0;
    count = 0;
    _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 1), count);
    try t.expectEqualDeep([4]f32{ 100, 100, 0, 0 }, d[0].rect);
    r.clip = .{ .min = .{ 0, 0 }, .max = .{ 200, 200 } };
    count = 0;
    _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 0), count);
    r = request(" ");
    count = 0;
    _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 0), count);
}
test "text complete rollback late unsupported and malformed UTF8 before capacity" {
    const metrics = try textAtlas();
    var d: [2]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&d), 0x33);
    const before = std.mem.asBytes(&d).*;
    var stage: [1]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&stage), 0x22);
    const stage_before = std.mem.asBytes(&stage).*;
    var count: usize = 1;
    for ([_]struct { s: []const u8, diagnostic: atlas.Diagnostic }{
        .{ .s = "A🪐", .diagnostic = .{ .UnsupportedCharacter = .{ .codepoint = 0x1fa90, .byte_offset = 1 } } },
        .{ .s = "A\n", .diagnostic = .{ .UnsupportedCharacter = .{ .codepoint = 10, .byte_offset = 1 } } },
        .{ .s = "\t", .diagnostic = .{ .UnsupportedCharacter = .{ .codepoint = 9, .byte_offset = 0 } } },
        .{ .s = "é", .diagnostic = .{ .UnsupportedCharacter = .{ .codepoint = 233, .byte_offset = 0 } } },
        .{ .s = "A\xff", .diagnostic = .{ .InvalidUtf8 = .{ .byte_offset = 1 } } },
        .{ .s = "\xc0\xaf", .diagnostic = .{ .InvalidUtf8 = .{ .byte_offset = 0 } } },
        .{ .s = "\xed\xa0\x80", .diagnostic = .{ .InvalidUtf8 = .{ .byte_offset = 0 } } },
        .{ .s = "\xf4\x90\x80\x80", .diagnostic = .{ .InvalidUtf8 = .{ .byte_offset = 0 } } },
        .{ .s = "A\xe2\x82", .diagnostic = .{ .InvalidUtf8 = .{ .byte_offset = 1 } } },
    }) |v| {
        var r = request(v.s);
        r.clip = .{ .min = .{ 0, 0 }, .max = .{ 1, 1 } };
        var limits = text_limits;
        limits.max_placements = 1;
        limits.max_staged_bytes = 0;
        try t.expectEqualDeep(v.diagnostic, try failure(text_api.appendPlacements(&metrics, r, limits, &d, &count, &stage)));
        try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
        try t.expectEqualSlices(u8, &stage_before, std.mem.asBytes(&stage));
        try t.expectEqualDeep(v.diagnostic, try failure(text_api.measure(&metrics, 40, .Regular, v.s, text_limits)));
    }
    var b: [3]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&b), 0x77);
    const bb = std.mem.asBytes(&b).*;
    var bs: [1]atlas.Boundary = undefined;
    var bc: usize = 2;
    try t.expectEqualDeep(atlas.Diagnostic{ .UnsupportedCharacter = .{ .codepoint = 0x1fa90, .byte_offset = 1 } }, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "A🪐", text_limits, &b, &bc, &bs)));
    try expectPreserved(atlas.Boundary, &bb, &b, 2, bc);
}
test "text exact budgets capacities old counts and whole rollback" {
    const metrics = try textAtlas();
    var d: [4]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&d), 0x66);
    const before = std.mem.asBytes(&d).*;
    var stage: [3]atlas.Boundary = undefined;
    var count: usize = 2;
    for ([_]struct { limits: atlas.TextLimits, dl: usize, sl: usize, diag: atlas.Diagnostic }{
        .{ .limits = .{ .max_input_bytes = 65536, .max_boundaries = 2, .max_placements = 8192, .max_staged_bytes = 48 }, .dl = 2, .sl = 1, .diag = .{ .LimitExceeded = .{ .resource = .boundaries, .actual = 3, .max = 2 } } },
        .{ .limits = .{ .max_input_bytes = 65536, .max_boundaries = 65537, .max_placements = 8192, .max_staged_bytes = 47 }, .dl = 2, .sl = 1, .diag = .{ .LimitExceeded = .{ .resource = .staged_bytes, .actual = 48, .max = 47 } } },
        .{ .limits = text_limits, .dl = 2, .sl = 1, .diag = .{ .DestinationCapacity = .{ .required = 3, .available = 2 } } },
        .{ .limits = text_limits, .dl = 4, .sl = 2, .diag = .{ .StagingCapacity = .{ .required = 3, .available = 2 } } },
    }) |v| {
        try t.expectEqualDeep(v.diag, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "Aa", v.limits, d[0..v.dl], &count, stage[0..v.sl])));
        try expectPreserved(atlas.Boundary, &before, &d, 2, count);
    }
    var exact = text_limits;
    exact.max_staged_bytes = 48;
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .Regular, "Aa", exact, &d, &count, &stage));
    count = 5;
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidCount = .{ .count = 5, .capacity = 4 } }, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "", text_limits, &d, &count, &stage)));
    var zero = text_limits;
    zero.max_boundaries = 0;
    count = 2;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .boundaries, .actual = 1, .max = 0 } }, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "", zero, &d, &count, &stage)));
    var pd: [2]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&pd), 0x11);
    const pb = std.mem.asBytes(&pd).*;
    var ps: [1]atlas.GlyphPlacement = undefined;
    count = 0;
    zero = .{ .max_input_bytes = 0, .max_boundaries = 0, .max_placements = 0, .max_staged_bytes = 0 };
    _ = try textSuccess(text_api.appendPlacements(&metrics, request(""), zero, &pd, &count, &ps));
    try expectPreserved(atlas.GlyphPlacement, &pb, &pd, 0, count);
    try t.expectEqualDeep(atlas.Diagnostic{ .InputTooLarge = .{ .actual = 1, .max = 0 } }, try failure(text_api.measure(&metrics, 40, .Regular, "A", zero)));
    zero.max_input_bytes = 65536;
    _ = try textSuccess(text_api.appendPlacements(&metrics, request(" "), zero, &pd, &count, &ps));
    var clipped = request("A");
    clipped.clip = .{ .min = .{ 0, 0 }, .max = .{ 0, 0 } };
    _ = try textSuccess(text_api.appendPlacements(&metrics, clipped, zero, &pd, &count, &ps));
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .placements, .actual = 1, .max = 0 } }, try failure(text_api.appendPlacements(&metrics, request("A"), zero, &pd, &count, &ps)));
    try expectPreserved(atlas.GlyphPlacement, &pb, &pd, 0, count);
    var lim = text_limits;
    lim.max_staged_bytes = 47;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .staged_bytes, .actual = 48, .max = 47 } }, try failure(text_api.appendPlacements(&metrics, request("A"), lim, pd[0..0], &count, ps[0..0])));
    try t.expectEqualDeep(atlas.Diagnostic{ .DestinationCapacity = .{ .required = 1, .available = 0 } }, try failure(text_api.appendPlacements(&metrics, request("A"), text_limits, pd[0..0], &count, ps[0..0])));
    try t.expectEqualDeep(atlas.Diagnostic{ .StagingCapacity = .{ .required = 1, .available = 0 } }, try failure(text_api.appendPlacements(&metrics, request("A"), text_limits, &pd, &count, ps[0..0])));
    lim.max_staged_bytes = 48;
    _ = try textSuccess(text_api.appendPlacements(&metrics, request("A"), lim, &pd, &count, &ps));
    count = 3;
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidCount = .{ .count = 3, .capacity = 2 } }, try failure(text_api.appendPlacements(&metrics, request(""), text_limits, &pd, &count, &ps)));
}
test "text global ceilings full byte boundary count and 8191 append" {
    const metrics = try textAtlas();
    const input = try t.allocator.alloc(u8, 65537);
    defer t.allocator.free(input);
    @memset(input, ' ');
    try t.expectEqual(@as(f32, 737280), (try textSuccess(text_api.measure(&metrics, 40, .Regular, input[0..65536], text_limits))).advance);
    try t.expectEqualDeep(atlas.Diagnostic{ .InputTooLarge = .{ .actual = 65537, .max = 65536 } }, try failure(text_api.measure(&metrics, 40, .Regular, input, text_limits)));
    const d = try t.allocator.alloc(atlas.Boundary, 65537);
    defer t.allocator.free(d);
    const stage = try t.allocator.alloc(atlas.Boundary, 65537);
    defer t.allocator.free(stage);
    var count: usize = 0;
    _ = try textSuccess(text_api.replaceBoundaries(&metrics, 40, .Regular, input[0..65536], text_limits, d, &count, stage));
    try t.expectEqual(@as(usize, 65537), count);
    try t.expectEqualDeep(atlas.Boundary{ .byte_offset = 65536, .advance_before = 737280 }, d[65536]);
    for ([_]atlas.Resource{ .input_bytes, .boundaries, .placements }) |resource| {
        var lim = text_limits;
        switch (resource) {
            .input_bytes => lim.max_input_bytes = 65537,
            .boundaries => lim.max_boundaries = 65538,
            .placements => lim.max_placements = 8193,
            else => unreachable,
        }
        const max: usize = switch (resource) {
            .input_bytes => 65536,
            .boundaries => 65537,
            .placements => 8192,
            else => unreachable,
        };
        try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = resource, .actual = max + 1, .max = max } }, try failure(text_api.measure(&metrics, 40, .Regular, "", lim)));
    }
    const pd = try t.allocator.alloc(atlas.GlyphPlacement, 8192);
    defer t.allocator.free(pd);
    @memset(std.mem.sliceAsBytes(pd), 0x54);
    const pb = try t.allocator.dupe(u8, std.mem.sliceAsBytes(pd));
    defer t.allocator.free(pb);
    var ps: [2]atlas.GlyphPlacement = undefined;
    count = 8191;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .placements, .actual = 8193, .max = 8192 } }, try failure(text_api.appendPlacements(&metrics, request("AA"), text_limits, pd, &count, &ps)));
    try expectPreserved(atlas.GlyphPlacement, pb, pd, 8191, count);
    _ = try textSuccess(text_api.appendPlacements(&metrics, request("A"), text_limits, pd, &count, &ps));
    try t.expectEqual(@as(usize, 8192), count);
    try t.expectEqualSlices(u8, pb[0 .. 8191 * 48], std.mem.sliceAsBytes(pd)[0 .. 8191 * 48]);
    var lim = text_limits;
    lim.max_placements = 8191;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .placements, .actual = 8192, .max = 8191 } }, try failure(text_api.appendPlacements(&metrics, request(""), lim, pd, &count, &ps)));
}
test "text mutated mappings advances and finite arithmetic failures" {
    var metrics = try textAtlas();
    var d: [2]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&d), 0x21);
    const before = std.mem.asBytes(&d).*;
    var stage: [1]atlas.GlyphPlacement = undefined;
    var count: usize = 1;
    const original = metrics.entries[33];
    metrics.entries[33].identity = .{ .icon = .Play };
    try t.expectEqualDeep(atlas.Diagnostic{ .MissingEntry = .{ .index = 33 } }, try failure(text_api.measure(&metrics, 40, .Regular, "A", text_limits)));
    metrics.entries[33] = original;
    metrics.entries[33].identity = .{ .glyph = .{ .codepoint = 66, .weight = .Regular } };
    try t.expectEqualDeep(atlas.Diagnostic{ .MissingEntry = .{ .index = 33 } }, try failure(text_api.measure(&metrics, 40, .Regular, "A", text_limits)));
    metrics.entries[33] = original;
    for ([_]f32{ -1, std.math.inf(f32), std.math.nan(f32) }) |v| {
        metrics.entries[33].advance = v;
        try t.expectEqualDeep(atlas.Diagnostic{ .InvalidAdvance = .{ .entry = 33 } }, try failure(text_api.appendPlacements(&metrics, request("A"), text_limits, &d, &count, &stage)));
        try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
    }
    metrics.entries[33] = original;
    for ([_]f32{ -1, std.math.inf(f32), std.math.nan(f32), std.math.floatMax(f32) }) |v| {
        var r = request("AA");
        r.font_size = v;
        try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
        try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
    }
    const original_px = metrics.generator.font_px;
    for ([_]f32{ 0, -1, std.math.inf(f32), std.math.nan(f32) }) |v| {
        metrics.generator.font_px = v;
        try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.measure(&metrics, 40, .Regular, "", text_limits)));
    }
    metrics.generator.font_px = original_px;
    for (0..2) |lane| {
        var r = request("A");
        r.baseline[lane] = std.math.inf(f32);
        try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    }
    for (0..4) |lane| {
        var r = request("A");
        r.color[lane] = std.math.nan(f32);
        try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    }
    var r = request("A");
    r.clip = .{ .min = .{ 1, 0 }, .max = .{ 0, 1 } };
    try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    r.clip = .{ .min = .{ -std.math.floatMax(f32), 0 }, .max = .{ std.math.floatMax(f32), 1 } };
    try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    metrics.entries[33].bounds.width = 0;
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidBounds = .{ .entry = 33 } }, try failure(text_api.appendPlacements(&metrics, request("A"), text_limits, &d, &count, &stage)));
    metrics.entries[33] = original;
    metrics.entries[33].bounds.x = std.math.maxInt(u32);
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidBounds = .{ .entry = 33 } }, try failure(text_api.appendPlacements(&metrics, request("A"), text_limits, &d, &count, &stage)));
    metrics.entries[33] = original;
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
}
test "text whole writable backing aliases including unused tails preserve staging" {
    const metrics = try textAtlas();
    var d: [4]atlas.GlyphPlacement align(@alignOf(usize)) = undefined;
    @memset(std.mem.asBytes(&d), 0x45);
    const before = std.mem.asBytes(&d).*;
    var stage: [3]atlas.GlyphPlacement align(@alignOf(usize)) = undefined;
    @memset(std.mem.asBytes(&stage), 0x23);
    const sb = std.mem.asBytes(&stage).*;
    var count: usize = 0;
    var r = request(std.mem.asBytes(&stage)[0..1]);
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    r = request(std.mem.asBytes(&d)[3 * 48 .. 3 * 48 + 1]);
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(&metrics, request(""), text_limits, d[0..3], &count, d[2..4])));
    const dc: *usize = @ptrCast(@alignCast(&d[3]));
    dc.* = 0;
    const changed = std.mem.asBytes(&d).*;
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(&metrics, request(""), text_limits, &d, dc, &stage)));
    try t.expectEqualSlices(u8, &changed, std.mem.asBytes(&d));
    @memcpy(std.mem.asBytes(&d), &before);
    const sc: *usize = @ptrCast(@alignCast(&stage[2]));
    sc.* = 0;
    const changed_stage = std.mem.asBytes(&stage).*;
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(&metrics, request(""), text_limits, &d, sc, &stage)));
    try t.expectEqualSlices(u8, &changed_stage, std.mem.asBytes(&stage));
    @memcpy(std.mem.asBytes(&stage), &sb);
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 0, count);
    try t.expectEqualSlices(u8, &sb, std.mem.asBytes(&stage));
    // Readable text can legally borrow the readable atlas itself, no writes alias it.
    var readable = try textAtlas();
    const raw = std.mem.asBytes(&readable);
    raw[0] = ' ';
    _ = try textSuccess(text_api.appendPlacements(&readable, request(raw[0..1]), text_limits, &d, &count, &stage));
    try t.expectEqual(@as(usize, 0), count);
    // Atlas overlaps destination on metrics valid aligned allocation, without fake pointers.
    const storage = try t.allocator.alignedAlloc(u8, .fromByteUnits(@alignOf(atlas.AtlasMetrics)), 6600 + 48);
    defer t.allocator.free(storage);
    const ap: *atlas.AtlasMetrics = @ptrCast(@alignCast(storage.ptr));
    ap.* = metrics;
    const dest: []atlas.GlyphPlacement = @as([*]atlas.GlyphPlacement, @ptrCast(@alignCast(storage.ptr)))[0..1];
    const old = try t.allocator.dupe(u8, storage);
    defer t.allocator.free(old);
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.appendPlacements(ap, request(""), text_limits, dest, &count, &stage)));
    try t.expectEqualSlices(u8, old, storage);
    var bd: [4]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&bd), 0x42);
    var bs: [3]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&bs), 0x41);
    const bbefore = std.mem.asBytes(&bd).*;
    const bs_before = std.mem.asBytes(&bs).*;
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, std.mem.asBytes(&bs)[0..1], text_limits, &bd, &count, &bs)));
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, std.mem.asBytes(&bd)[48..49], text_limits, &bd, &count, &bs)));
    try t.expectEqualDeep(atlas.Diagnostic.Aliasing, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "", text_limits, bd[0..3], &count, bd[2..4])));
    try expectPreserved(atlas.Boundary, &bbefore, &bd, 0, count);
    try t.expectEqualSlices(u8, &bs_before, std.mem.asBytes(&bs));
}

test "text exact zero staging and overglobal old count preserve complete output" {
    const metrics = try textAtlas();
    var d: [2]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&d), 0x73);
    const before = std.mem.asBytes(&d).*;
    var stage: [1]atlas.Boundary = undefined;
    @memset(std.mem.asBytes(&stage), 0x62);
    const sb = std.mem.asBytes(&stage).*;
    var count: usize = 1;
    var lim = text_limits;
    lim.max_staged_bytes = 0;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .staged_bytes, .actual = 16, .max = 0 } }, try failure(text_api.replaceBoundaries(&metrics, 40, .Regular, "", lim, &d, &count, &stage)));
    try expectPreserved(atlas.Boundary, &before, &d, 1, count);
    try t.expectEqualSlices(u8, &sb, std.mem.asBytes(&stage));
    const pd = try t.allocator.alloc(atlas.GlyphPlacement, 8194);
    defer t.allocator.free(pd);
    @memset(std.mem.sliceAsBytes(pd), 0x63);
    const pb = try t.allocator.dupe(u8, std.mem.sliceAsBytes(pd));
    defer t.allocator.free(pb);
    var ps: [1]atlas.GlyphPlacement = undefined;
    count = 8193;
    try t.expectEqualDeep(atlas.Diagnostic{ .LimitExceeded = .{ .resource = .placements, .actual = 8193, .max = 8192 } }, try failure(text_api.appendPlacements(&metrics, request(""), text_limits, pd, &count, &ps)));
    try expectPreserved(atlas.GlyphPlacement, pb, pd, 8193, count);
}
test "text both weight placements half scale and mutations outside clipping" {
    var metrics = try textAtlas();
    var d: [2]atlas.GlyphPlacement = undefined;
    @memset(std.mem.asBytes(&d), 0x51);
    var stage: [1]atlas.GlyphPlacement = undefined;
    var count: usize = 0;
    var r = request("A");
    r.font_size = 20;
    _ = try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage));
    try t.expectEqualDeep([4]f32{ 96, 76, 32, 32 }, d[0].rect);
    try t.expectEqualDeep([4]f32{ 0.0625, 0.125, 0.0625, 0.0625 }, d[0].uv);
    r = request("A");
    r.weight = .SemiBold;
    count = 0;
    try t.expectEqual(@as(f32, 29.1015625), (try textSuccess(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage))).advance);
    try t.expectEqualDeep([4]f32{ 0, 0.5, 0.0625, 0.0625 }, d[0].uv);
    const before = std.mem.asBytes(&d).*;
    r = request("A");
    r.clip = .{ .min = .{ 0, 0 }, .max = .{ 1, 1 } };
    count = 1;
    const original = metrics.entries[33];
    metrics.entries[33].identity = .{ .glyph = .{ .codepoint = 65, .weight = .SemiBold } };
    try t.expectEqualDeep(atlas.Diagnostic{ .MissingEntry = .{ .index = 33 } }, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
    metrics.entries[33] = original;
    metrics.generator.cell_size = 0;
    try t.expectEqualDeep(atlas.Diagnostic{ .InvalidCell = .{ .entry = 33 } }, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
    metrics.generator.cell_size = 64;
    metrics.width = 0;
    try t.expectEqualDeep(atlas.Diagnostic{ .DescriptorMismatch = .{ .field = .width } }, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
    metrics.width = 1024;
    r = request("A");
    r.font_size = 40;
    r.baseline = .{ -std.math.floatMax(f32), 0 };
    metrics.generator.font_origin_x = std.math.maxInt(i32);
    r.font_size = 1e32;
    try t.expectEqualDeep(atlas.Diagnostic.NonFiniteGeometry, try failure(text_api.appendPlacements(&metrics, r, text_limits, &d, &count, &stage)));
    try expectPreserved(atlas.GlyphPlacement, &before, &d, 1, count);
}
