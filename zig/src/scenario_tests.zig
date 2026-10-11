const std = @import("std");
const scenario = @import("scenario.zig");
const model = @import("model.zig");
const allocator = std.testing.allocator;

// Independently transcribed from Rust WireScenario's field order and the frozen
// src/game/model.rs generator vector. The fingerprint was derived with Python
// struct.pack's little-endian fields and the source's FNV1a64 formula (478 bytes).
const DEFAULT_FINGERPRINT: u64 = 0xD728_015A_9A3D_8835;
const DEFAULT_JSON =
    "{\"format_version\":1,\"rules_version\":1,\"generator_version\":1,\"seed\":\"4947573120260902\",\"rules\":{\"tick_hz\":60,\"win_world_count\":5,\"ai_period_ticks\":\"60\",\"hazard_first_tick\":\"2700\",\"hazard_period_ticks\":\"2700\",\"hazard_duration_ticks\":\"720\",\"maximum_energy\":\"250000\",\"maximum_defense\":\"100000\",\"minimum_launch\":\"10000\",\"field_raise_cost\":\"20000\",\"field_lower_refund\":\"10000\",\"base_fleet_speed\":\"23\"},\"worlds\":[" ++
    "{\"name\":\"ASTER VALE\",\"x\":1037,\"y\":5027,\"owner\":\"UNION\",\"energy\":\"80000\",\"defense\":\"43735\",\"base_output\":\"2904\",\"base_regeneration\":\"488\",\"atmosphere\":6,\"hydrosphere\":7,\"topology\":3}," ++
    "{\"name\":\"KHEPRI\",\"x\":8793,\"y\":2818,\"owner\":\"HELIX\",\"energy\":\"80000\",\"defense\":\"44244\",\"base_output\":\"2678\",\"base_regeneration\":\"463\",\"atmosphere\":5,\"hydrosphere\":6,\"topology\":7}," ++
    "{\"name\":\"MERIDIAN\",\"x\":8517,\"y\":7827,\"owner\":\"CHOIR\",\"energy\":\"80000\",\"defense\":\"43986\",\"base_output\":\"2246\",\"base_regeneration\":\"405\",\"atmosphere\":6,\"hydrosphere\":3,\"topology\":7}," ++
    "{\"name\":\"VESPER\",\"x\":4811,\"y\":1320,\"owner\":\"NEUTRAL\",\"energy\":\"0\",\"defense\":\"58596\",\"base_output\":\"2591\",\"base_regeneration\":\"454\",\"atmosphere\":7,\"hydrosphere\":7,\"topology\":6}," ++
    "{\"name\":\"ORISON\",\"x\":4905,\"y\":8955,\"owner\":\"NEUTRAL\",\"energy\":\"0\",\"defense\":\"46169\",\"base_output\":\"2848\",\"base_regeneration\":\"275\",\"atmosphere\":7,\"hydrosphere\":4,\"topology\":7}," ++
    "{\"name\":\"NACRE\",\"x\":3746,\"y\":3709,\"owner\":\"NEUTRAL\",\"energy\":\"0\",\"defense\":\"43254\",\"base_output\":\"1944\",\"base_regeneration\":\"405\",\"atmosphere\":5,\"hydrosphere\":4,\"topology\":3}," ++
    "{\"name\":\"UMBRA\",\"x\":5925,\"y\":6136,\"owner\":\"NEUTRAL\",\"energy\":\"0\",\"defense\":\"45102\",\"base_output\":\"2998\",\"base_regeneration\":\"474\",\"atmosphere\":6,\"hydrosphere\":7,\"topology\":7}]}";

fn hasCode(report: *const scenario.ValidationReport, code: scenario.ScenarioIssueCode) bool {
    for (report.issues()) |issue| if (issue.code == code) return true;
    return false;
}

test "factory scenario has independent canonical JSON and fingerprint oracle" {
    var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    const validated = try draft.validated();
    try std.testing.expectEqualDeep(validated, try scenario.generatedValidated(model.DEFAULT_SEED, .{}));
    try std.testing.expectError(error.Validation, scenario.generatedValidated(model.DEFAULT_SEED, .{ .tick_hz = 59 }));
    try std.testing.expectEqual(DEFAULT_FINGERPRINT, validated.fingerprint);
    try std.testing.expectEqual(@as(usize, 0), validated.warnings.len);
    const encoded = try scenario.encode(allocator, &validated);
    defer allocator.free(encoded);
    try std.testing.expectEqualStrings(DEFAULT_JSON, encoded);
    const decoded = try scenario.decode(allocator, DEFAULT_JSON);
    try std.testing.expectEqualDeep(validated, decoded);
    for (decoded.worlds, 0..) |world, index| {
        try std.testing.expectEqual(@as(model.WorldId, @intCast(index)), world.id);
        try std.testing.expectEqual(@as(u64, 0), world.production_remainder);
        try std.testing.expectEqual(@as(u64, 0), world.regeneration_remainder);
    }
}

test "validator preserves every reachable Rust error boundary" {
    const codes = [_]scenario.ScenarioIssueCode{
        .tick_rate,                  .win_world_count,  .ai_period,         .hazard_period,           .hazard_duration,
        .hazard_boundary_arithmetic, .energy_cap,       .defense_cap,       .minimum_launch,          .field_cost_relation,
        .world_name,                 .world_coordinate, .world_field_level, .world_energy,            .world_defense,
        .duplicate_position,         .missing_union,    .initial_victory,   .regeneration_arithmetic, .fleet_speed_arithmetic,
    };
    for (codes) |code| {
        var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
        defer draft.deinit();
        switch (code) {
            .tick_rate => draft.rules.tick_hz = 59,
            .win_world_count => draft.rules.win_world_count = 1,
            .ai_period => draft.rules.ai_period_ticks = 0,
            .hazard_period => draft.rules.hazard_period_ticks = 0,
            .hazard_duration => draft.rules.hazard_duration_ticks = 0,
            .hazard_boundary_arithmetic => draft.rules.hazard_first_tick = std.math.maxInt(u64),
            .energy_cap => draft.rules.maximum_energy = 0,
            .defense_cap => draft.rules.maximum_defense = 0,
            .minimum_launch => draft.rules.minimum_launch = 0,
            .field_cost_relation => draft.rules.field_lower_refund = draft.rules.field_raise_cost + 1,
            .world_name => draft.worlds[0].name = "lowercase",
            .world_coordinate => draft.worlds[0].x = -1,
            .world_field_level => draft.worlds[0].atmosphere = 11,
            .world_energy => draft.worlds[0].energy = draft.rules.maximum_energy + 1,
            .world_defense => draft.worlds[0].defense = draft.rules.maximum_defense + 1,
            .duplicate_position => {
                draft.worlds[1].x = draft.worlds[0].x;
                draft.worlds[1].y = draft.worlds[0].y;
            },
            .missing_union => draft.worlds[0].owner = null,
            .initial_victory => for (draft.worlds[0..5]) |*world| {
                world.owner = .union_faction;
            },
            .regeneration_arithmetic => draft.worlds[0].base_regeneration = std.math.maxInt(u64),
            .fleet_speed_arithmetic => draft.rules.base_fleet_speed = std.math.maxInt(u64),
            else => unreachable,
        }
        const report = draft.validationReport();
        try std.testing.expect(hasCode(&report, code));
        try std.testing.expect(!report.isValid());
        try std.testing.expectError(error.Validation, draft.validated());
        try std.testing.expectError(error.Validation, scenario.encodeDraft(allocator, &draft));
    }
}

test "validator accepts maximal valid arithmetic and field boundaries" {
    var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    draft.rules.win_world_count = 2;
    draft.rules.hazard_duration_ticks = draft.rules.hazard_period_ticks;
    draft.rules.minimum_launch = 1;
    draft.rules.field_lower_refund = draft.rules.field_raise_cost;
    draft.worlds[0].name = "ABCDEFGHIJKLMNOPQRSTUVWX";
    draft.worlds[0].x = 0;
    draft.worlds[0].y = 10_000;
    draft.worlds[0].atmosphere = 0;
    draft.worlds[0].hydrosphere = 10;
    draft.worlds[0].topology = 0;
    draft.worlds[0].base_output = std.math.maxInt(u64);
    draft.worlds[0].base_regeneration = std.math.maxInt(u64) - 1000;
    draft.rules.base_fleet_speed = @intCast(@as(u128, std.math.maxInt(u64)) * 10_000 / 14_000);
    try std.testing.expect(draft.validationReport().isValid());
    draft.rules.win_world_count = 7;
    try std.testing.expect(draft.validationReport().isValid());
    draft.rules.base_fleet_speed = 0;
    try std.testing.expect(draft.validationReport().isValid());
    const validated = try draft.validated();
    const encoded = try scenario.encode(allocator, &validated);
    defer allocator.free(encoded);
    try std.testing.expectEqualDeep(validated, try scenario.decode(allocator, encoded));
}

test "warning order and duplicate reference indices match Rust" {
    var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    draft.worlds[1].name = draft.worlds[0].name;
    draft.worlds[1].x = draft.worlds[0].x + 1;
    draft.worlds[1].y = draft.worlds[0].y;
    draft.worlds[1].owner = null;
    draft.worlds[2].owner = null;
    draft.worlds[0].energy = draft.rules.minimum_launch * 2 - 1;
    const report = draft.validationReport();
    try std.testing.expect(report.isValid());
    const expected = [_]scenario.ScenarioIssueCode{ .duplicate_name, .close_position, .missing_helix, .missing_choir, .no_launchable_union };
    try std.testing.expectEqual(expected.len, report.len);
    for (report.issues(), expected) |issue, code| try std.testing.expectEqual(code, issue.code);
    const validated = try draft.validated();
    try std.testing.expectEqualDeep(report.issues(), validated.warnings.issues());
    draft.worlds[2].name = draft.worlds[0].name;
    draft.worlds[2].x = draft.worlds[1].x;
    draft.worlds[2].y = draft.worlds[1].y;
    const duplicate_report = draft.validationReport();
    var found = false;
    for (duplicate_report.issues()) |issue| {
        if (issue.code == .duplicate_position and std.mem.eql(u8, issue.field.asSlice(), "worlds[2].position")) {
            try std.testing.expectEqualStrings("world position duplicates worlds[1]", issue.message.asSlice());
            found = true;
        }
    }
    try std.testing.expect(found);
}

fn replaceFirst(original: []const u8, needle: []const u8, replacement: []const u8) ![]u8 {
    const offset = std.mem.indexOf(u8, original, needle) orelse return error.NotFound;
    return std.mem.concat(allocator, u8, &.{ original[0..offset], replacement, original[offset + needle.len ..] });
}

test "JSON rejects unknown missing duplicate fields and every wire type mismatch" {
    const changes = .{
        .{ "{", "{\"unknown\":true," },
        .{ "\"tick_hz\":60", "\"extra\":0,\"tick_hz\":60" },
        .{ "\"name\":\"ASTER VALE\"", "\"name\":\"ASTER VALE\",\"id\":0" },
        .{ "\"tick_hz\":60", "\"tick_hz\":60,\"tick_hz\":60" },
        .{ "\"tick_hz\":60,", "" },
        .{ "\"tick_hz\":60", "\"tick_hz\":\"60\"" },
        .{ "\"tick_hz\":60", "\"tick_hz\":60.0" },
        .{ "\"tick_hz\":60", "\"tick_hz\":6e1" },
        .{ "\"tick_hz\":60", "\"tick_hz\":-0" },
        .{ "\"tick_hz\":60", "\"tick_hz\":4294967296" },
        .{ "\"ai_period_ticks\":\"60\"", "\"ai_period_ticks\":60" },
        .{ "\"owner\":\"UNION\"", "\"owner\":null" },
        .{ "\"owner\":\"UNION\"", "\"owner\":1" },
        .{ "\"owner\":\"UNION\"", "\"owner\":\"union\"" },
        .{ "\"atmosphere\":6", "\"atmosphere\":256" },
        .{ "\"x\":1037", "\"x\":\"1037\"" },
        .{ "\"x\":1037", "\"x\":2147483648" },
    };
    inline for (changes) |change| {
        const json = try replaceFirst(DEFAULT_JSON, change[0], change[1]);
        defer allocator.free(json);
        try std.testing.expectError(error.Json, scenario.decode(allocator, json));
    }
    const wrong_version = try replaceFirst(DEFAULT_JSON, "\"format_version\":1", "\"format_version\":2");
    defer allocator.free(wrong_version);
    try std.testing.expectError(error.Version, scenario.decode(allocator, wrong_version));
    const last_world_offset = std.mem.indexOf(u8, DEFAULT_JSON, ",{\"name\":\"UMBRA\"").?;
    const six_worlds = try std.mem.concat(allocator, u8, &.{ DEFAULT_JSON[0..last_world_offset], "]}" });
    defer allocator.free(six_worlds);
    try std.testing.expectError(error.Json, scenario.decode(allocator, six_worlds));
    const seven_worlds_end = DEFAULT_JSON.len - 2;
    const eight_worlds = try std.mem.concat(allocator, u8, &.{ DEFAULT_JSON[0..seven_worlds_end], DEFAULT_JSON[last_world_offset..seven_worlds_end], "]}" });
    defer allocator.free(eight_worlds);
    try std.testing.expectError(error.Json, scenario.decode(allocator, eight_worlds));
}

test "coordinate negative zero is a serde JSON type rejection" {
    // Current Rust oracle returned ERR\tJson for each x/y token of -0.
    inline for (.{ .{ "\"x\":1037", "\"x\":-0" }, .{ "\"y\":5027", "\"y\":-0" } }) |change| {
        const json = try replaceFirst(DEFAULT_JSON, change[0], change[1]);
        defer allocator.free(json);
        try std.testing.expectError(error.Json, scenario.decode(allocator, json));
        try std.testing.expectError(error.Json, scenario.decodeDraft(allocator, json));
    }
}

fn expectCanonicalDetails(json: []const u8, field: []const u8, kind: []const u8) !void {
    const decoded = try scenario.decodeDetailed(allocator, json);
    try std.testing.expect(decoded == .failure and decoded.failure == .canonical_number);
    try std.testing.expectEqualStrings(field, decoded.failure.canonical_number.field);
    try std.testing.expectEqualStrings(kind, decoded.failure.canonical_number.kind);
    const draft = try scenario.decodeDraftDetailed(allocator, json);
    try std.testing.expect(draft == .failure and draft.failure == .canonical_number);
    try std.testing.expectEqualDeep(decoded.failure.canonical_number, draft.failure.canonical_number);
    try std.testing.expectError(error.CanonicalNumber, scenario.decode(allocator, json));
    try std.testing.expectError(error.CanonicalNumber, scenario.decodeDraft(allocator, json));
}

test "detailed canonical errors preserve every source field and kind" {
    // An executable oracle compiled the current Rust model/scenario/codec source
    // with pinned dependencies and reported exactly these 15 field/kind pairs.
    const invalid_seed = try replaceFirst(DEFAULT_JSON, "4947573120260902", "lowercase");
    defer allocator.free(invalid_seed);
    try expectCanonicalDetails(invalid_seed, "seed", "16-digit uppercase hexadecimal");
    const rules: model.RulesV1 = .{};
    inline for (@typeInfo(model.RulesV1).@"struct".field_names[2..]) |field| {
        const needle = try std.fmt.allocPrint(allocator, "\"{s}\":\"{d}\"", .{ field, @field(rules, field) });
        defer allocator.free(needle);
        const replacement = try std.fmt.allocPrint(allocator, "\"{s}\":\"01\"", .{field});
        defer allocator.free(replacement);
        const invalid = try replaceFirst(DEFAULT_JSON, needle, replacement);
        defer allocator.free(invalid);
        try expectCanonicalDetails(invalid, "rules." ++ field, "unsigned decimal");
    }
    inline for (.{ .{ "energy", "80000" }, .{ "defense", "43735" }, .{ "base_output", "2904" }, .{ "base_regeneration", "488" } }) |pair| {
        const invalid = try replaceFirst(DEFAULT_JSON, "\"" ++ pair[0] ++ "\":\"" ++ pair[1] ++ "\"", "\"" ++ pair[0] ++ "\":\"01\"");
        defer allocator.free(invalid);
        try expectCanonicalDetails(invalid, "world." ++ pair[0], "unsigned decimal");
    }
}

fn invalidDiagnosticJson() ![]u8 {
    var json = try allocator.dupe(u8, DEFAULT_JSON);
    errdefer allocator.free(json);
    inline for (.{
        .{ "\"tick_hz\":60", "\"tick_hz\":59" },
        .{ "ASTER VALE", "lowercase" },
        .{ "KHEPRI", "lowercase" },
        .{ "\"x\":8793", "\"x\":1038" },
        .{ "\"y\":2818", "\"y\":5027" },
        .{ "\"owner\":\"HELIX\"", "\"owner\":\"NEUTRAL\"" },
        .{ "\"owner\":\"CHOIR\"", "\"owner\":\"NEUTRAL\"" },
        .{ "\"energy\":\"80000\"", "\"energy\":\"19999\"" },
        .{ "\"base_regeneration\":\"454\"", "\"base_regeneration\":\"18446744073709551615\"" },
        .{ "\"base_fleet_speed\":\"23\"", "\"base_fleet_speed\":\"18446744073709551615\"" },
    }) |change| {
        const changed = try replaceFirst(json, change[0], change[1]);
        allocator.free(json);
        json = changed;
    }
    return json;
}

test "detailed validation errors own the full source report including warnings" {
    // Current Rust source oracle: Validation errors=5 warnings=5 followed by
    // these exact ten ordered severity/code/field/message rows.
    const expected = .{
        .{ scenario.IssueSeverity.err, scenario.ScenarioIssueCode.tick_rate, "rules.tick_hz", "tick rate must remain 60" },
        .{ scenario.IssueSeverity.err, scenario.ScenarioIssueCode.world_name, "worlds[0].name", "world name is not canonical uppercase ASCII" },
        .{ scenario.IssueSeverity.err, scenario.ScenarioIssueCode.world_name, "worlds[1].name", "world name is not canonical uppercase ASCII" },
        .{ scenario.IssueSeverity.warning, scenario.ScenarioIssueCode.duplicate_name, "worlds[1].name", "world name duplicates worlds[0]" },
        .{ scenario.IssueSeverity.err, scenario.ScenarioIssueCode.regeneration_arithmetic, "worlds[3].base_regeneration", "worst-case regeneration arithmetic does not fit" },
        .{ scenario.IssueSeverity.warning, scenario.ScenarioIssueCode.close_position, "worlds[1].position", "world is within 250 sectors of worlds[0]" },
        .{ scenario.IssueSeverity.warning, scenario.ScenarioIssueCode.missing_helix, "worlds", "no world belongs to the Helix" },
        .{ scenario.IssueSeverity.warning, scenario.ScenarioIssueCode.missing_choir, "worlds", "no world belongs to the Choir" },
        .{ scenario.IssueSeverity.warning, scenario.ScenarioIssueCode.no_launchable_union, "worlds", "the Union has no source able to launch" },
        .{ scenario.IssueSeverity.err, scenario.ScenarioIssueCode.fleet_speed_arithmetic, "rules.base_fleet_speed", "worst-case hydrosphere arithmetic must fit" },
    };
    const json = try invalidDiagnosticJson();
    defer allocator.free(json);
    const decoded = try scenario.decodeDetailed(allocator, json);
    try std.testing.expect(decoded == .failure and decoded.failure == .validation);
    const report = decoded.failure.validation;
    try std.testing.expectEqual(@as(usize, 5), report.errorCount());
    try std.testing.expectEqual(@as(usize, 5), report.warningCount());
    try std.testing.expectEqual(expected.len, report.len);
    @memset(json, 'X');
    inline for (expected, 0..) |row, index| {
        const issue = report.issues()[index];
        try std.testing.expectEqual(row[0], issue.severity);
        try std.testing.expectEqual(row[1], issue.code);
        try std.testing.expectEqualStrings(row[2], issue.field.asSlice());
        try std.testing.expectEqualStrings(row[3], issue.message.asSlice());
    }
}

test "admission and encoding reject stale or invalid mutable values" {
    const original = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    var candidate = original;
    candidate.worlds[0].energy += 1;
    try std.testing.expectError(error.StaleFingerprint, scenario.admit(&candidate));
    try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    candidate.fingerprint = scenario.computeFingerprint(&candidate);
    try std.testing.expectEqualDeep(candidate.worlds, (try scenario.admit(&candidate)).worlds);
    candidate = original;
    candidate.rules.tick_hz = 59;
    try std.testing.expectError(error.Validation, scenario.admit(&candidate));
    try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    candidate = original;
    candidate.worlds[0].id = 1;
    try std.testing.expectError(error.InvalidInitialState, scenario.admit(&candidate));
    try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    inline for (.{ "production_remainder", "regeneration_remainder" }) |field| {
        candidate = original;
        @field(candidate.worlds[0], field) = 1;
        try std.testing.expectError(error.InvalidInitialState, scenario.admit(&candidate));
        try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    }
    candidate = original;
    candidate.worlds[0].name.len = 255;
    try std.testing.expectError(error.Validation, scenario.admit(&candidate));
    try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    candidate = original;
    candidate.fingerprint ^= 1;
    try std.testing.expectError(error.StaleFingerprint, scenario.admit(&candidate));
    try std.testing.expectError(error.Validation, scenario.encode(allocator, &candidate));
    candidate = original;
    candidate.warnings.len = std.math.maxInt(usize);
    try std.testing.expectEqual(@as(usize, 0), (try scenario.admit(&candidate)).warnings.len);
    const canonical = try scenario.encode(allocator, &candidate);
    defer allocator.free(canonical);
    try std.testing.expectEqualStrings(DEFAULT_JSON, canonical);
}

test "admission rederives authentic warnings for valid changed values" {
    var candidate = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    candidate.worlds[1].name = candidate.worlds[0].name;
    candidate.fingerprint = scenario.computeFingerprint(&candidate);
    candidate.warnings.len = 0;
    const accepted = try scenario.admit(&candidate);
    try std.testing.expectEqual(@as(usize, 1), accepted.warnings.len);
    try std.testing.expectEqual(scenario.ScenarioIssueCode.duplicate_name, accepted.warnings.issues()[0].code);
    try std.testing.expectEqualStrings("world name duplicates worlds[0]", accepted.warnings.issues()[0].message.asSlice());
}

test "detailed decoding separates resource errors from invalid data" {
    var failing = std.testing.FailingAllocator.init(allocator, .{ .fail_index = 0 });
    try std.testing.expectError(error.OutOfMemory, scenario.decodeDetailed(failing.allocator(), DEFAULT_JSON));
    try std.testing.expectError(error.OutOfMemory, scenario.decodeDraftDetailed(failing.allocator(), DEFAULT_JSON));
    const oversized = try allocator.alloc(u8, scenario.MAX_JSON_BYTES + 1);
    defer allocator.free(oversized);
    @memset(oversized, ' ');
    const capped = try scenario.decodeDetailed(failing.allocator(), oversized);
    try std.testing.expect(capped == .failure and capped.failure == .too_large);
    const malformed = try scenario.decodeDetailed(allocator, "{");
    try std.testing.expect(malformed == .failure and malformed.failure == .json);
    try std.testing.expect(malformed.failure.json != error.Json);
    const bad_seed = try replaceFirst(DEFAULT_JSON, "4947573120260902", "lowercase");
    defer allocator.free(bad_seed);
    const number = try scenario.decodeDetailed(allocator, bad_seed);
    @memset(bad_seed, 'X');
    try std.testing.expectEqualStrings("seed", number.failure.canonical_number.field);
    try std.testing.expectEqualStrings("16-digit uppercase hexadecimal", number.failure.canonical_number.kind);
}

fn exerciseDetailedAllocations(gpa: std.mem.Allocator, canonical_json: []const u8, invalid_json: []const u8) !void {
    const accepted = try scenario.decodeDetailed(gpa, DEFAULT_JSON);
    try std.testing.expect(accepted == .scenario);
    const number = try scenario.decodeDetailed(gpa, canonical_json);
    try std.testing.expect(number == .failure and number.failure == .canonical_number);
    const invalid = try scenario.decodeDetailed(gpa, invalid_json);
    try std.testing.expect(invalid == .failure and invalid.failure == .validation);
    const draft_outcome = try scenario.decodeDraftDetailed(gpa, invalid_json);
    try std.testing.expect(draft_outcome == .draft);
    var draft = draft_outcome.draft;
    defer draft.deinit();
    const report = draft.validationReport();
    try std.testing.expectEqualDeep(report, invalid.failure.validation);
    const malformed = try scenario.decodeDetailed(gpa, "{");
    try std.testing.expect(malformed == .failure and malformed.failure == .json);
}

test "detailed decoder allocation failures preserve every outcome ownership path" {
    const bad_seed = try replaceFirst(DEFAULT_JSON, "4947573120260902", "lowercase");
    defer allocator.free(bad_seed);
    const invalid = try invalidDiagnosticJson();
    defer allocator.free(invalid);
    try std.testing.checkAllAllocationFailures(allocator, exerciseDetailedAllocations, .{ bad_seed, invalid });
}

test "serde positional records and unit enum objects canonicalize identically" {
    // Executed oracle extracted the exact current Rust WireScenario, WireRules,
    // WireWorld and WireOwner derives: all three record sequences and singleton
    // unit-enum objects were accepted by serde_json 1.0.151.
    const parsed = try std.json.parseFromSlice(std.json.Value, allocator, DEFAULT_JSON, .{});
    defer parsed.deinit();
    var rule_values = std.json.Array.init(allocator);
    defer rule_values.deinit();
    for ([_][]const u8{ "tick_hz", "win_world_count", "ai_period_ticks", "hazard_first_tick", "hazard_period_ticks", "hazard_duration_ticks", "maximum_energy", "maximum_defense", "minimum_launch", "field_raise_cost", "field_lower_refund", "base_fleet_speed" }) |name| {
        try rule_values.append(parsed.value.object.get("rules").?.object.get(name).?);
    }
    var world_values = std.json.Array.init(allocator);
    defer world_values.deinit();
    var world_rows: [model.WORLD_COUNT]std.json.Array = @splat(.init(allocator));
    defer for (&world_rows) |*row| row.deinit();
    for (parsed.value.object.get("worlds").?.array.items, &world_rows) |world, *row| {
        for ([_][]const u8{ "name", "x", "y", "owner", "energy", "defense", "base_output", "base_regeneration", "atmosphere", "hydrosphere", "topology" }) |name| try row.append(world.object.get(name).?);
        try world_values.append(.{ .array = row.* });
    }
    var top_values = std.json.Array.init(allocator);
    defer top_values.deinit();
    for ([_][]const u8{ "format_version", "rules_version", "generator_version", "seed" }) |name| try top_values.append(parsed.value.object.get(name).?);
    try top_values.append(.{ .array = rule_values });
    try top_values.append(.{ .array = world_values });
    const positional = try std.json.Stringify.valueAlloc(allocator, std.json.Value{ .array = top_values }, .{});
    defer allocator.free(positional);
    const decoded = try scenario.decode(allocator, positional);
    const canonical = try scenario.encode(allocator, &decoded);
    defer allocator.free(canonical);
    try std.testing.expectEqualStrings(DEFAULT_JSON, canonical);
    _ = top_values.pop();
    const short_record = try std.json.Stringify.valueAlloc(allocator, std.json.Value{ .array = top_values }, .{});
    defer allocator.free(short_record);
    try std.testing.expectError(error.Json, scenario.decode(allocator, short_record));
    for ([_][]const u8{ "UNION", "HELIX", "CHOIR", "NEUTRAL" }) |tag| {
        const needle = try std.fmt.allocPrint(allocator, "\"owner\":\"{s}\"", .{tag});
        defer allocator.free(needle);
        const replacement = try std.fmt.allocPrint(allocator, "\"owner\":{{\"{s}\":null}}", .{tag});
        defer allocator.free(replacement);
        const json = try replaceFirst(DEFAULT_JSON, needle, replacement);
        defer allocator.free(json);
        try std.testing.expectEqual(DEFAULT_FINGERPRINT, (try scenario.decode(allocator, json)).fingerprint);
    }
    inline for (.{ "0", "[]", "{}", "false", "\"UNION\"" }) |payload| {
        const json = try replaceFirst(DEFAULT_JSON, "\"owner\":\"UNION\"", "\"owner\":{\"UNION\":" ++ payload ++ "}");
        defer allocator.free(json);
        try std.testing.expectError(error.Json, scenario.decode(allocator, json));
    }
}

test "wire type validation precedes versions and canonical number conversion" {
    const wrong_version = try replaceFirst(DEFAULT_JSON, "\"format_version\":1", "\"format_version\":2");
    defer allocator.free(wrong_version);
    const malformed_wire = try replaceFirst(wrong_version, "\"atmosphere\":6", "\"atmosphere\":\"6\"");
    defer allocator.free(malformed_wire);
    try std.testing.expectError(error.Json, scenario.decode(allocator, malformed_wire));
    const bad_seed = try replaceFirst(DEFAULT_JSON, "4947573120260902", "lowercase");
    defer allocator.free(bad_seed);
    const malformed_with_bad_seed = try replaceFirst(bad_seed, "\"owner\":\"UNION\"", "\"owner\":null");
    defer allocator.free(malformed_with_bad_seed);
    try std.testing.expectError(error.Json, scenario.decode(allocator, malformed_with_bad_seed));
}

test "JSON canonical strings and preparse byte cap are exact" {
    for ([_][]const u8{ "", "00", "01", "-1", "+1", " 1", "1 ", "1.0", "1e0", "18446744073709551616" }) |invalid| {
        try std.testing.expectError(error.CanonicalNumber, scenario.parseU64(invalid));
    }
    try std.testing.expectEqual(@as(u64, 0), try scenario.parseU64("0"));
    try std.testing.expectEqual(std.math.maxInt(u64), try scenario.parseU64("18446744073709551615"));
    for ([_][]const u8{ "", "494757312026902", "49475731202609020", "abcdef0123456789", "0x47573120260902", "G947573120260902" }) |invalid| {
        try std.testing.expectError(error.CanonicalNumber, scenario.parseSeed(invalid));
    }
    try std.testing.expectEqual(std.math.maxInt(u64), try scenario.parseSeed("FFFFFFFFFFFFFFFF"));
    const leading_zero = try replaceFirst(DEFAULT_JSON, "\"250000\"", "\"0250000\"");
    defer allocator.free(leading_zero);
    try std.testing.expectError(error.CanonicalNumber, scenario.decode(allocator, leading_zero));
    const lower_seed = try replaceFirst(DEFAULT_JSON, "4947573120260902", "abcdef0123456789");
    defer allocator.free(lower_seed);
    try std.testing.expectError(error.CanonicalNumber, scenario.decode(allocator, lower_seed));
    const oversized = try allocator.alloc(u8, scenario.MAX_JSON_BYTES + 1);
    defer allocator.free(oversized);
    @memset(oversized, ' ');
    try std.testing.expectError(error.TooLarge, scenario.decode(allocator, oversized));
    try std.testing.expectError(error.Json, scenario.decode(allocator, oversized[0..scenario.MAX_JSON_BYTES]));
    @memcpy(oversized[0..DEFAULT_JSON.len], DEFAULT_JSON);
    try std.testing.expectEqual(DEFAULT_FINGERPRINT, (try scenario.decode(allocator, oversized[0..scenario.MAX_JSON_BYTES])).fingerprint);
    const invalid_utf8 = try allocator.dupe(u8, DEFAULT_JSON);
    defer allocator.free(invalid_utf8);
    invalid_utf8[std.mem.indexOf(u8, invalid_utf8, "ASTER VALE").?] = 0xff;
    try std.testing.expectError(error.Json, scenario.decode(allocator, invalid_utf8));
}

test "invalid decoded drafts remain detached and own input strings" {
    const invalid_json = try replaceFirst(DEFAULT_JSON, "ASTER VALE", "lowercase");
    defer allocator.free(invalid_json);
    var draft = try scenario.decodeDraft(allocator, invalid_json);
    defer draft.deinit();
    try std.testing.expectError(error.Validation, draft.validated());
    try std.testing.expectError(error.Validation, scenario.decode(allocator, invalid_json));
    @memset(invalid_json, 'X');
    try std.testing.expectEqualStrings("lowercase", draft.worlds[0].name);
    var copied = try draft.clone(allocator);
    defer copied.deinit();
    draft.worlds[0].name = "ASTER VALE";
    try std.testing.expectEqualStrings("lowercase", copied.worlds[0].name);
    const accepted = try draft.validated();
    draft.worlds[0].name = "OTHER WORLD";
    try std.testing.expectEqualStrings("ASTER VALE", accepted.worlds[0].name.asSlice());
    try draft.regenerateWorldsFromSeed();
    try std.testing.expectEqual(DEFAULT_FINGERPRINT, (try draft.validated()).fingerprint);
}

test "scenario identity includes names rules and all initial world fields" {
    var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    inline for (@typeInfo(model.RulesV1).@"struct".field_names[1..]) |field| {
        var changed = try draft.clone(allocator);
        defer changed.deinit();
        @field(changed.rules, field) += 1;
        try std.testing.expect((try changed.validated()).fingerprint != DEFAULT_FINGERPRINT);
    }
    inline for (@typeInfo(scenario.WorldDraft).@"struct".field_names) |field| {
        var changed = try draft.clone(allocator);
        defer changed.deinit();
        if (comptime std.mem.eql(u8, field, "name")) changed.worlds[3].name = "ASTER PRIME" else if (comptime std.mem.eql(u8, field, "owner")) changed.worlds[3].owner = .union_faction else @field(changed.worlds[3], field) += 1;
        try std.testing.expect((try changed.validated()).fingerprint != DEFAULT_FINGERPRINT);
    }
    draft.seed += 1;
    try std.testing.expect((try draft.validated()).fingerprint != DEFAULT_FINGERPRINT);
}

fn exerciseAllocations(gpa: std.mem.Allocator) !void {
    var draft = try scenario.ScenarioDraft.factoryDefault(gpa);
    defer draft.deinit();
    var cloned = try draft.clone(gpa);
    defer cloned.deinit();
    try cloned.regenerateWorldsFromSeed();
    const validated = try cloned.validated();
    var reverted = try validated.toDraft(gpa);
    defer reverted.deinit();
    const encoded = try scenario.encodeDraft(gpa, &reverted);
    defer gpa.free(encoded);
    _ = try scenario.decode(gpa, encoded);
}

test "failed regeneration preserves the previous draft" {
    var failing = std.testing.FailingAllocator.init(allocator, .{});
    var draft = try scenario.ScenarioDraft.factoryDefault(failing.allocator());
    defer draft.deinit();
    const previous = try draft.validated();
    draft.seed = 2;
    failing.fail_index = failing.alloc_index;
    try std.testing.expectError(error.OutOfMemory, draft.regenerateWorldsFromSeed());
    try std.testing.expectEqual(@as(u64, 2), draft.seed);
    try std.testing.expectEqualDeep(previous.worlds[0], (try draft.validated()).worlds[0]);
}

test "all draft codec allocation failures release ownership" {
    try std.testing.checkAllAllocationFailures(allocator, exerciseAllocations, .{});
}
