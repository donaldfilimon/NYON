//! Classic scenario validation and canonical JSON, ported from src/scenario/.
//! Draft names belong to a draft arena; validated scenario state is all values.
const std = @import("std");
const model = @import("model.zig");
const generator = @import("generator.zig");

pub const FORMAT_VERSION: u32 = 1;
pub const RULES_VERSION: u32 = 1;
pub const GENERATOR_VERSION: u32 = 1;
pub const MAX_JSON_BYTES: usize = 65_536;
pub const ScenarioFingerprint = u64;
pub const ReplayIdentity = struct { scenario: ScenarioFingerprint, state: u64 };
pub const CodecError = error{ TooLarge, Json, Version, CanonicalNumber, Validation, OutOfMemory };
pub const CanonicalNumberFailure = struct { field: []const u8, kind: []const u8 };
pub const CodecFailure = union(enum) {
    too_large,
    json: anyerror,
    version,
    canonical_number: CanonicalNumberFailure,
    validation: ValidationReport,

    pub fn compactError(self: CodecFailure) CodecError {
        return switch (self) {
            .too_large => error.TooLarge,
            .json => error.Json,
            .version => error.Version,
            .canonical_number => error.CanonicalNumber,
            .validation => error.Validation,
        };
    }
};
pub const DecodeOutcome = union(enum) { scenario: ScenarioV1, failure: CodecFailure };
/// Successful draft outcomes own their arena and require draft.deinit().
pub const DecodeDraftOutcome = union(enum) { draft: ScenarioDraft, failure: CodecFailure };
pub const AdmissionError = error{ Validation, InvalidInitialState, StaleFingerprint };

pub const WorldDraft = struct {
    name: []const u8,
    x: i32,
    y: i32,
    owner: ?model.Faction,
    energy: u64,
    defense: u64,
    base_output: u64,
    base_regeneration: u64,
    atmosphere: u8,
    hydrosphere: u8,
    topology: u8,

    fn fromWorld(allocator: std.mem.Allocator, world: *const model.World) !WorldDraft {
        var draft = borrowWorld(world);
        draft.name = try allocator.dupe(u8, world.name.asSlice());
        return draft;
    }

    fn borrowWorld(world: *const model.World) WorldDraft {
        return .{
            .name = world.name.asSlice(),
            .x = world.position.x,
            .y = world.position.y,
            .owner = world.owner,
            .energy = world.energy,
            .defense = world.defense,
            .base_output = world.base_output_per_second,
            .base_regeneration = world.base_regeneration_per_second,
            .atmosphere = world.fields.atmosphere,
            .hydrosphere = world.fields.hydrosphere,
            .topology = world.fields.topology,
        };
    }
};

pub const ScenarioDraft = struct {
    seed: u64,
    rules: model.RulesV1,
    worlds: [model.WORLD_COUNT]WorldDraft,
    arena: std.heap.ArenaAllocator,

    pub fn factoryDefault(allocator: std.mem.Allocator) !ScenarioDraft {
        return generated(allocator, model.DEFAULT_SEED, .{});
    }

    pub fn generated(allocator: std.mem.Allocator, seed: u64, rules: model.RulesV1) !ScenarioDraft {
        const campaign = generator.generate(seed, rules);
        var result = ScenarioDraft{ .seed = seed, .rules = rules, .worlds = undefined, .arena = .init(allocator) };
        errdefer result.deinit();
        for (&result.worlds, &campaign.worlds) |*draft, *world| draft.* = try WorldDraft.fromWorld(result.arena.allocator(), world);
        return result;
    }

    pub fn clone(self: *const ScenarioDraft, allocator: std.mem.Allocator) !ScenarioDraft {
        var result = ScenarioDraft{ .seed = self.seed, .rules = self.rules, .worlds = self.worlds, .arena = .init(allocator) };
        errdefer result.deinit();
        for (&result.worlds) |*world| world.name = try result.arena.allocator().dupe(u8, world.name);
        return result;
    }

    pub fn deinit(self: *ScenarioDraft) void {
        self.arena.deinit();
        self.* = undefined;
    }

    /// Successful regeneration replaces detached draft worlds; an allocation
    /// failure preserves every byte of the existing draft.
    pub fn regenerateWorldsFromSeed(self: *ScenarioDraft) !void {
        const replacement = try generated(self.arena.child_allocator, self.seed, self.rules);
        self.deinit();
        self.* = replacement;
    }

    pub fn validationReport(self: *const ScenarioDraft) ValidationReport {
        return validate(self);
    }

    pub fn validated(self: *const ScenarioDraft) error{Validation}!ScenarioV1 {
        return fromDraft(self);
    }
};

pub const IssueSeverity = enum { err, warning };
pub const ScenarioIssueCode = enum {
    tick_rate,
    win_world_count,
    ai_period,
    hazard_period,
    hazard_duration,
    hazard_boundary_arithmetic,
    energy_cap,
    defense_cap,
    minimum_launch,
    field_cost_relation,
    world_name,
    world_coordinate,
    world_field_level,
    world_energy,
    world_defense,
    duplicate_position,
    missing_union,
    initial_victory,
    production_arithmetic,
    regeneration_arithmetic,
    fleet_speed_arithmetic,
    duplicate_name,
    close_position,
    missing_helix,
    missing_choir,
    no_launchable_union,
};

fn Text(comptime capacity: usize) type {
    return struct {
        bytes: [capacity]u8 = @splat(0),
        len: usize = 0,
        fn init(value: []const u8) @This() {
            std.debug.assert(value.len <= capacity);
            var result: @This() = .{ .len = value.len };
            @memcpy(result.bytes[0..value.len], value);
            return result;
        }
        pub fn asSlice(self: *const @This()) []const u8 {
            return self.bytes[0..self.len];
        }
    };
}

pub const ScenarioIssue = struct {
    severity: IssueSeverity,
    code: ScenarioIssueCode,
    field: Text(48),
    message: Text(96),
};

/// RulesV1 has at most 10 rule issues, 9 issues per world, 21 close pairs,
/// and 6 final issues: 100 total. This bound includes invalid drafts.
pub const ValidationReport = struct {
    entries: [100]ScenarioIssue = @splat(.{ .severity = .err, .code = .tick_rate, .field = .{}, .message = .{} }),
    len: usize = 0,

    pub fn issues(self: *const ValidationReport) []const ScenarioIssue {
        return self.entries[0..self.len];
    }
    pub fn isValid(self: *const ValidationReport) bool {
        return self.errorCount() == 0;
    }
    pub fn errorCount(self: *const ValidationReport) usize {
        var count: usize = 0;
        for (self.issues()) |issue| if (issue.severity == .err) {
            count += 1;
        };
        return count;
    }
    pub fn warningCount(self: *const ValidationReport) usize {
        return self.len - self.errorCount();
    }
    fn push(self: *ValidationReport, severity: IssueSeverity, code: ScenarioIssueCode, field: []const u8, message: []const u8) void {
        std.debug.assert(self.len < self.entries.len);
        self.entries[self.len] = .{ .severity = severity, .code = code, .field = .init(field), .message = .init(message) };
        self.len += 1;
    }
    fn errorIf(self: *ValidationReport, condition: bool, code: ScenarioIssueCode, field: []const u8, message: []const u8) void {
        if (condition) self.push(.err, code, field, message);
    }
    fn warningIf(self: *ValidationReport, condition: bool, code: ScenarioIssueCode, field: []const u8, message: []const u8) void {
        if (condition) self.push(.warning, code, field, message);
    }
};

pub const ScenarioV1 = struct {
    seed: u64,
    rules: model.RulesV1,
    worlds: [model.WORLD_COUNT]model.World,
    warnings: ValidationReport,
    fingerprint: ScenarioFingerprint,

    pub fn toDraft(self: *const ScenarioV1, allocator: std.mem.Allocator) !ScenarioDraft {
        var result = ScenarioDraft{ .seed = self.seed, .rules = self.rules, .worlds = undefined, .arena = .init(allocator) };
        errdefer result.deinit();
        for (&result.worlds, &self.worlds) |*draft, *world| draft.* = try WorldDraft.fromWorld(result.arena.allocator(), world);
        return result;
    }
};

/// Infallible genesis generation does not need an allocator. This path applies
/// the shared scenario validator to generated worlds without allocating names,
/// matching Simulation::generated's optional scenario identity in the source.
pub fn generatedValidated(seed: u64, rules: model.RulesV1) error{Validation}!ScenarioV1 {
    const campaign = generator.generate(seed, rules);
    var empty_storage: [0]u8 = .{};
    var fixed = std.heap.FixedBufferAllocator.init(&empty_storage);
    var draft = ScenarioDraft{ .seed = seed, .rules = rules, .worlds = undefined, .arena = .init(fixed.allocator()) };
    defer draft.deinit();
    for (&campaign.worlds, &draft.worlds) |*world, *target| target.* = WorldDraft.borrowWorld(world);
    return fromDraft(&draft);
}

pub fn fromDraft(draft: *const ScenarioDraft) error{Validation}!ScenarioV1 {
    const report = validate(draft);
    if (!report.isValid()) return error.Validation;
    return fromValidDraft(draft, &report);
}

fn fromValidDraft(draft: *const ScenarioDraft, report: *const ValidationReport) ScenarioV1 {
    var result = ScenarioV1{ .seed = draft.seed, .rules = draft.rules, .worlds = undefined, .warnings = .{}, .fingerprint = 0 };
    for (draft.worlds, &result.worlds, 0..) |world, *target, index| {
        target.* = .{
            .id = @intCast(index),
            .name = model.WorldName.init(world.name) catch unreachable,
            .position = .{ .x = world.x, .y = world.y },
            .owner = world.owner,
            .defense = world.defense,
            .energy = world.energy,
            .base_output_per_second = world.base_output,
            .base_regeneration_per_second = world.base_regeneration,
            .fields = model.FieldLevels.init(world.atmosphere, world.hydrosphere, world.topology),
            .production_remainder = 0,
            .regeneration_remainder = 0,
        };
    }
    for (report.issues()) |issue| {
        if (issue.severity == .warning) {
            result.warnings.entries[result.warnings.len] = issue;
            result.warnings.len += 1;
        }
    }
    result.fingerprint = computeFingerprint(&result);
    return result;
}

/// Zig struct fields remain mutable values. Cross the authority boundary only
/// through fresh shared validation: identities, initial runtime state and the
/// supplied fingerprint must agree. Warning payloads are always rederived.
pub fn admit(candidate: *const ScenarioV1) AdmissionError!ScenarioV1 {
    for (&candidate.worlds, 0..) |*world, index| {
        if (world.id != index or world.production_remainder != 0 or world.regeneration_remainder != 0)
            return error.InvalidInitialState;
        if (world.name.len > model.WorldName.MAX_BYTES) return error.Validation;
    }
    var empty_storage: [0]u8 = .{};
    var fixed = std.heap.FixedBufferAllocator.init(&empty_storage);
    var draft = ScenarioDraft{ .seed = candidate.seed, .rules = candidate.rules, .worlds = undefined, .arena = .init(fixed.allocator()) };
    defer draft.deinit();
    for (&candidate.worlds, &draft.worlds) |*world, *target| target.* = WorldDraft.borrowWorld(world);
    const fresh = try fromDraft(&draft);
    if (fresh.fingerprint != candidate.fingerprint) return error.StaleFingerprint;
    return fresh;
}

pub fn validate(draft: *const ScenarioDraft) ValidationReport {
    var report: ValidationReport = .{};
    const rules = draft.rules;
    report.errorIf(rules.tick_hz != 60, .tick_rate, "rules.tick_hz", "tick rate must remain 60");
    report.errorIf(rules.win_world_count < 2 or rules.win_world_count > 7, .win_world_count, "rules.win_world_count", "win world count must be between 2 and 7");
    report.errorIf(rules.ai_period_ticks == 0, .ai_period, "rules.ai_period_ticks", "AI period must be nonzero");
    report.errorIf(rules.hazard_period_ticks == 0, .hazard_period, "rules.hazard_period_ticks", "hazard period must be nonzero");
    report.errorIf(rules.hazard_duration_ticks == 0 or rules.hazard_duration_ticks > rules.hazard_period_ticks, .hazard_duration, "rules.hazard_duration_ticks", "hazard duration must be nonzero and no greater than its period");
    const hazard_bounds_fit = @as(u128, rules.hazard_first_tick) + rules.hazard_duration_ticks <= std.math.maxInt(u64) and
        @as(u128, rules.hazard_first_tick) + rules.hazard_period_ticks + rules.hazard_duration_ticks <= std.math.maxInt(u64);
    report.errorIf(!hazard_bounds_fit, .hazard_boundary_arithmetic, "rules.hazard_first_tick", "the first two hazard boundaries must fit u64");
    report.errorIf(rules.maximum_energy == 0, .energy_cap, "rules.maximum_energy", "maximum energy must be nonzero");
    report.errorIf(rules.maximum_defense == 0, .defense_cap, "rules.maximum_defense", "maximum defense must be nonzero");
    report.errorIf(rules.minimum_launch == 0 or rules.minimum_launch > rules.maximum_energy / 2, .minimum_launch, "rules.minimum_launch", "minimum launch must be between 1 and half maximum energy");
    report.errorIf(rules.field_lower_refund > rules.field_raise_cost or rules.field_raise_cost > rules.maximum_energy, .field_cost_relation, "rules.field_raise_cost", "refund must not exceed raise cost, and raise cost must fit the energy cap");
    const divisor: u128 = 60 * 10_000;
    var counts = [_]u8{ 0, 0, 0 };
    var launchable_union = false;
    for (draft.worlds, 0..) |world, index| {
        var field_buffer: [48]u8 = undefined;
        inline for (.{
            .{ "name", .world_name, "world name is not canonical uppercase ASCII" },
            .{ "position", .world_coordinate, "world coordinates must be between 0 and 10000" },
            .{ "fields", .world_field_level, "field levels must be between 0 and 10" },
            .{ "energy", .world_energy, "starting energy exceeds its cap" },
            .{ "defense", .world_defense, "starting defense exceeds its cap" },
            .{ "base_output", .production_arithmetic, "worst-case production arithmetic does not fit" },
            .{ "base_regeneration", .regeneration_arithmetic, "worst-case regeneration arithmetic does not fit" },
        }, 0..) |check, check_index| {
            const invalid = switch (check_index) {
                0 => if (model.WorldName.init(world.name)) |_| false else |_| true,
                1 => world.x < 0 or world.x > 10_000 or world.y < 0 or world.y > 10_000,
                2 => world.atmosphere > 10 or world.hydrosphere > 10 or world.topology > 10,
                3 => world.energy > rules.maximum_energy,
                4 => world.defense > rules.maximum_defense,
                5 => (@as(u128, world.base_output) * 15_000 + divisor - 1) / divisor > std.math.maxInt(u64),
                6 => @as(u128, world.base_regeneration) + 1000 > std.math.maxInt(u64) or
                    ((@as(u128, world.base_regeneration) + 1000) * 10_000 + divisor - 1) / divisor > std.math.maxInt(u64),
                else => unreachable,
            };
            report.errorIf(invalid, check[1], std.fmt.bufPrint(&field_buffer, "worlds[{d}].{s}", .{ index, check[0] }) catch unreachable, check[2]);
        }
        // BTreeMap::insert in the reference reports the most recent duplicate.
        var previous_position: ?usize = null;
        var previous_name: ?usize = null;
        for (draft.worlds[0..index], 0..) |previous, previous_index| {
            if (world.x == previous.x and world.y == previous.y) previous_position = previous_index;
            if (std.mem.eql(u8, world.name, previous.name)) previous_name = previous_index;
        }
        var message_buffer: [96]u8 = undefined;
        if (previous_position) |previous| report.push(.err, .duplicate_position, std.fmt.bufPrint(&field_buffer, "worlds[{d}].position", .{index}) catch unreachable, std.fmt.bufPrint(&message_buffer, "world position duplicates worlds[{d}]", .{previous}) catch unreachable);
        if (previous_name) |previous| report.push(.warning, .duplicate_name, std.fmt.bufPrint(&field_buffer, "worlds[{d}].name", .{index}) catch unreachable, std.fmt.bufPrint(&message_buffer, "world name duplicates worlds[{d}]", .{previous}) catch unreachable);
        if (world.owner) |owner| {
            counts[@backingInt(owner)] += 1;
            if (owner == .union_faction and world.energy / 2 >= rules.minimum_launch) launchable_union = true;
        }
    }
    for (0..model.WORLD_COUNT) |left| {
        for (left + 1..model.WORLD_COUNT) |right| {
            const a = draft.worlds[left];
            const b = draft.worlds[right];
            const dx = @as(i128, a.x) - b.x;
            const dy = @as(i128, a.y) - b.y;
            const squared = dx * dx + dy * dy;
            var field_buffer: [48]u8 = undefined;
            var message_buffer: [96]u8 = undefined;
            if (squared > 0 and squared < 250 * 250) report.push(.warning, .close_position, std.fmt.bufPrint(&field_buffer, "worlds[{d}].position", .{right}) catch unreachable, std.fmt.bufPrint(&message_buffer, "world is within 250 sectors of worlds[{d}]", .{left}) catch unreachable);
        }
    }
    report.errorIf(counts[0] == 0, .missing_union, "worlds", "at least one world must belong to the Union");
    report.errorIf(counts[0] >= rules.win_world_count or counts[1] >= rules.win_world_count or counts[2] >= rules.win_world_count, .initial_victory, "worlds", "no faction may begin at the win threshold");
    report.warningIf(counts[1] == 0, .missing_helix, "worlds", "no world belongs to the Helix");
    report.warningIf(counts[2] == 0, .missing_choir, "worlds", "no world belongs to the Choir");
    report.warningIf(!launchable_union, .no_launchable_union, "worlds", "the Union has no source able to launch");
    const effective_speed = @as(u128, rules.base_fleet_speed) * 14_000 / 10_000;
    report.errorIf(effective_speed > std.math.maxInt(u64) or (effective_speed * 10_000 + divisor - 1) / divisor > std.math.maxInt(u64), .fleet_speed_arithmetic, "rules.base_fleet_speed", "worst-case hydrosphere arithmetic must fit");
    return report;
}

const Fnv1a64 = struct {
    value: u64 = 0xCBF2_9CE4_8422_2325,
    fn bytes(self: *Fnv1a64, values: []const u8) void {
        for (values) |byte| self.value = (self.value ^ byte) *% 0x0000_0100_0000_01B3;
    }
    fn number(self: *Fnv1a64, comptime T: type, value: T) void {
        var bytes_buffer: [@sizeOf(T)]u8 = undefined;
        std.mem.writeInt(T, &bytes_buffer, value, .little);
        self.bytes(&bytes_buffer);
    }
};

pub fn computeFingerprint(scenario: *const ScenarioV1) ScenarioFingerprint {
    var hash: Fnv1a64 = .{};
    hash.bytes("IWSC\x00");
    hash.number(u32, FORMAT_VERSION);
    hash.number(u32, RULES_VERSION);
    hash.number(u32, GENERATOR_VERSION);
    hash.number(u64, scenario.seed);
    const rules = scenario.rules;
    hash.number(u32, rules.tick_hz);
    hash.number(u8, rules.win_world_count);
    for ([_]u64{ rules.ai_period_ticks, rules.hazard_first_tick, rules.hazard_period_ticks, rules.hazard_duration_ticks, rules.maximum_energy, rules.maximum_defense, rules.minimum_launch, rules.field_raise_cost, rules.field_lower_refund, rules.base_fleet_speed }) |value| hash.number(u64, value);
    for (scenario.worlds) |world| {
        hash.number(u8, world.id);
        hash.number(u8, @intCast(world.name.asSlice().len));
        hash.bytes(world.name.asSlice());
        hash.number(i32, world.position.x);
        hash.number(i32, world.position.y);
        hash.number(u8, if (world.owner) |owner| @as(u8, @backingInt(owner)) + 1 else 0);
        hash.number(u64, world.energy);
        hash.number(u64, world.defense);
        hash.number(u64, world.base_output_per_second);
        hash.number(u64, world.base_regeneration_per_second);
        hash.number(u8, world.fields.atmosphere);
        hash.number(u8, world.fields.hydrosphere);
        hash.number(u8, world.fields.topology);
    }
    return hash.value;
}

const WireRules = struct {
    tick_hz: u32,
    win_world_count: u8,
    ai_period_ticks: []const u8,
    hazard_first_tick: []const u8,
    hazard_period_ticks: []const u8,
    hazard_duration_ticks: []const u8,
    maximum_energy: []const u8,
    maximum_defense: []const u8,
    minimum_launch: []const u8,
    field_raise_cost: []const u8,
    field_lower_refund: []const u8,
    base_fleet_speed: []const u8,
};
const WireWorld = struct {
    name: []const u8,
    x: i32,
    y: i32,
    owner: []const u8,
    energy: []const u8,
    defense: []const u8,
    base_output: []const u8,
    base_regeneration: []const u8,
    atmosphere: u8,
    hydrosphere: u8,
    topology: u8,
};
const WireScenario = struct {
    format_version: u32,
    rules_version: u32,
    generator_version: u32,
    seed: []const u8,
    rules: WireRules,
    worlds: [model.WORLD_COUNT]WireWorld,
};

pub fn encode(allocator: std.mem.Allocator, candidate: *const ScenarioV1) CodecError![]u8 {
    const accepted = admit(candidate) catch return error.Validation;
    const scenario = &accepted;
    var arena: std.heap.ArenaAllocator = .init(allocator);
    defer arena.deinit();
    const temporary = arena.allocator();
    var seed_buffer: [16]u8 = undefined;
    const digits = "0123456789ABCDEF";
    for (&seed_buffer, 0..) |*byte, index| byte.* = digits[@as(u4, @truncate(scenario.seed >> @as(u6, @intCast((15 - index) * 4))))];
    var rules: WireRules = undefined;
    rules.tick_hz = scenario.rules.tick_hz;
    rules.win_world_count = scenario.rules.win_world_count;
    inline for (@typeInfo(WireRules).@"struct".field_names[2..]) |field| @field(rules, field) = try std.fmt.allocPrint(temporary, "{d}", .{@field(scenario.rules, field)});
    var worlds: [model.WORLD_COUNT]WireWorld = undefined;
    for (&worlds, &scenario.worlds) |*wire, *world| {
        wire.* = .{
            .name = world.name.asSlice(),
            .x = world.position.x,
            .y = world.position.y,
            .owner = ownerText(world.owner),
            .energy = try std.fmt.allocPrint(temporary, "{d}", .{world.energy}),
            .defense = try std.fmt.allocPrint(temporary, "{d}", .{world.defense}),
            .base_output = try std.fmt.allocPrint(temporary, "{d}", .{world.base_output_per_second}),
            .base_regeneration = try std.fmt.allocPrint(temporary, "{d}", .{world.base_regeneration_per_second}),
            .atmosphere = world.fields.atmosphere,
            .hydrosphere = world.fields.hydrosphere,
            .topology = world.fields.topology,
        };
    }
    return std.json.Stringify.valueAlloc(allocator, WireScenario{
        .format_version = FORMAT_VERSION,
        .rules_version = RULES_VERSION,
        .generator_version = GENERATOR_VERSION,
        .seed = &seed_buffer,
        .rules = rules,
        .worlds = worlds,
    }, .{});
}

pub fn encodeDraft(allocator: std.mem.Allocator, draft: *const ScenarioDraft) CodecError![]u8 {
    const scenario = try fromDraft(draft);
    return encode(allocator, &scenario);
}

pub fn decode(allocator: std.mem.Allocator, json: []const u8) CodecError!ScenarioV1 {
    return switch (try decodeDetailed(allocator, json)) {
        .scenario => |decoded| decoded,
        .failure => |failure| failure.compactError(),
    };
}

/// Failure payloads own their report bytes or reference static field/kind names;
/// allocation failure remains a distinct error, as in preferences.decodeDetailed.
pub fn decodeDetailed(allocator: std.mem.Allocator, json: []const u8) std.mem.Allocator.Error!DecodeOutcome {
    const outcome = try decodeDraftDetailed(allocator, json);
    switch (outcome) {
        .failure => |failure| return .{ .failure = failure },
        .draft => |decoded| {
            var draft = decoded;
            defer draft.deinit();
            const report = validate(&draft);
            if (!report.isValid()) return .{ .failure = .{ .validation = report } };
            return .{ .scenario = fromValidDraft(&draft, &report) };
        },
    }
}

const Value = std.json.Value;
fn Record(comptime field_names: []const []const u8) type {
    return struct {
        value: Value,
        fn get(self: @This(), name: []const u8) ?Value {
            if (self.value == .object) return self.value.object.get(name);
            inline for (field_names, 0..) |field, index| {
                if (std.mem.eql(u8, field, name)) return self.value.array.items[index];
            }
            return null;
        }
    };
}
fn record(value: Value, comptime expected_fields: []const []const u8) CodecError!Record(expected_fields) {
    // Serde's derive-generated struct visitor also accepts exact positional
    // sequences. Encoding still emits the canonical named-field object.
    switch (value) {
        .object => |fields| {
            if (fields.count() != expected_fields.len) return error.Json;
            for (expected_fields) |name| if (!fields.contains(name)) return error.Json;
        },
        .array => |fields| if (fields.items.len != expected_fields.len) return error.Json,
        else => return error.Json,
    }
    return .{ .value = value };
}
fn string(value: Value) CodecError![]const u8 {
    if (value != .string) return error.Json;
    return value.string;
}
fn integer(comptime T: type, value: Value) CodecError!T {
    if (value != .number_string) return error.Json;
    const text = value.number_string;
    // serde_json represents negative zero as a floating-point value, including
    // when the destination is signed, so it rejects this integer coordinate.
    if (std.mem.eql(u8, text, "-0")) return error.Json;
    const offset: usize = if (@typeInfo(T).int.signedness == .signed and text.len > 0 and text[0] == '-') 1 else 0;
    if (text.len == offset) return error.Json;
    for (text[offset..]) |byte| if (!std.ascii.isDigit(byte)) return error.Json;
    return std.fmt.parseInt(T, text, 10) catch error.Json;
}
pub fn parseSeed(value: []const u8) error{CanonicalNumber}!u64 {
    if (value.len != 16) return error.CanonicalNumber;
    for (value) |byte| if (!std.ascii.isDigit(byte) and (byte < 'A' or byte > 'F')) return error.CanonicalNumber;
    return std.fmt.parseInt(u64, value, 16) catch error.CanonicalNumber;
}
pub fn parseU64(value: []const u8) error{CanonicalNumber}!u64 {
    if (value.len == 0 or (value.len > 1 and value[0] == '0')) return error.CanonicalNumber;
    for (value) |byte| if (!std.ascii.isDigit(byte)) return error.CanonicalNumber;
    return std.fmt.parseInt(u64, value, 10) catch error.CanonicalNumber;
}
fn parseOwner(value: Value) CodecError!?model.Faction {
    const text = switch (value) {
        .string => |text| text,
        .object => |fields| blk: {
            if (fields.count() != 1) return error.Json;
            var iterator = fields.iterator();
            const entry = iterator.next().?;
            if (entry.value_ptr.* != .null) return error.Json;
            break :blk entry.key_ptr.*;
        },
        else => return error.Json,
    };
    inline for (.{ .{ "NEUTRAL", @as(?model.Faction, null) }, .{ "UNION", @as(?model.Faction, .union_faction) }, .{ "HELIX", @as(?model.Faction, .helix) }, .{ "CHOIR", @as(?model.Faction, .choir) } }) |pair| {
        if (std.mem.eql(u8, text, pair[0])) return pair[1];
    }
    return error.Json;
}

fn ownerText(faction: ?model.Faction) []const u8 {
    return if (faction) |value| switch (value) {
        .union_faction => "UNION",
        .helix => "HELIX",
        .choir => "CHOIR",
    } else "NEUTRAL";
}

fn parseWire(value: Value) CodecError!WireScenario {
    const top = try record(value, &.{ "format_version", "rules_version", "generator_version", "seed", "rules", "worlds" });
    var wire: WireScenario = undefined;
    wire.format_version = try integer(u32, top.get("format_version").?);
    wire.rules_version = try integer(u32, top.get("rules_version").?);
    wire.generator_version = try integer(u32, top.get("generator_version").?);
    wire.seed = try string(top.get("seed").?);
    const rules = try record(top.get("rules").?, &.{ "tick_hz", "win_world_count", "ai_period_ticks", "hazard_first_tick", "hazard_period_ticks", "hazard_duration_ticks", "maximum_energy", "maximum_defense", "minimum_launch", "field_raise_cost", "field_lower_refund", "base_fleet_speed" });
    wire.rules.tick_hz = try integer(u32, rules.get("tick_hz").?);
    wire.rules.win_world_count = try integer(u8, rules.get("win_world_count").?);
    inline for (@typeInfo(WireRules).@"struct".field_names[2..]) |field| @field(wire.rules, field) = try string(rules.get(field).?);
    const worlds = top.get("worlds").?;
    if (worlds != .array or worlds.array.items.len != model.WORLD_COUNT) return error.Json;
    for (worlds.array.items, &wire.worlds) |world_value, *world| {
        const fields = try record(world_value, &.{ "name", "x", "y", "owner", "energy", "defense", "base_output", "base_regeneration", "atmosphere", "hydrosphere", "topology" });
        world.* = .{
            .name = try string(fields.get("name").?),
            .x = try integer(i32, fields.get("x").?),
            .y = try integer(i32, fields.get("y").?),
            .owner = ownerText(try parseOwner(fields.get("owner").?)),
            .energy = try string(fields.get("energy").?),
            .defense = try string(fields.get("defense").?),
            .base_output = try string(fields.get("base_output").?),
            .base_regeneration = try string(fields.get("base_regeneration").?),
            .atmosphere = try integer(u8, fields.get("atmosphere").?),
            .hydrosphere = try integer(u8, fields.get("hydrosphere").?),
            .topology = try integer(u8, fields.get("topology").?),
        };
    }
    return wire;
}

pub fn decodeDraft(allocator: std.mem.Allocator, json: []const u8) CodecError!ScenarioDraft {
    return switch (try decodeDraftDetailed(allocator, json)) {
        .draft => |draft| draft,
        .failure => |failure| failure.compactError(),
    };
}

pub fn decodeDraftDetailed(allocator: std.mem.Allocator, json: []const u8) std.mem.Allocator.Error!DecodeDraftOutcome {
    if (json.len > MAX_JSON_BYTES) return .{ .failure = .too_large };
    // Rust receives &str; the Zig byte-slice boundary must enforce its UTF-8
    // invariant explicitly before handing strings to the JSON parser.
    if (!std.unicode.utf8ValidateSlice(json)) return .{ .failure = .{ .json = error.InvalidUtf8 } };
    var draft = ScenarioDraft{ .seed = undefined, .rules = undefined, .worlds = undefined, .arena = .init(allocator) };
    var owns_draft = true;
    defer if (owns_draft) draft.deinit();
    const value = std.json.parseFromSliceLeaky(Value, draft.arena.allocator(), json, .{
        .allocate = .alloc_always,
        .parse_numbers = false,
        .duplicate_field_behavior = .@"error",
    }) catch |err| return if (err == error.OutOfMemory) error.OutOfMemory else .{ .failure = .{ .json = err } };
    // Match serde's two phases: first the complete typed wire representation,
    // then versions/canonical strings, finally detached scenario validation.
    const wire = parseWire(value) catch |err| return .{ .failure = .{ .json = err } };
    if (wire.format_version != FORMAT_VERSION or wire.rules_version != RULES_VERSION or wire.generator_version != GENERATOR_VERSION) return .{ .failure = .version };
    draft.seed = parseSeed(wire.seed) catch return .{ .failure = .{ .canonical_number = .{ .field = "seed", .kind = "16-digit uppercase hexadecimal" } } };
    draft.rules.tick_hz = wire.rules.tick_hz;
    draft.rules.win_world_count = wire.rules.win_world_count;
    inline for (@typeInfo(model.RulesV1).@"struct".field_names[2..]) |field| {
        @field(draft.rules, field) = parseU64(@field(wire.rules, field)) catch return .{ .failure = .{ .canonical_number = .{ .field = "rules." ++ field, .kind = "unsigned decimal" } } };
    }
    for (wire.worlds, &draft.worlds) |wire_world, *world| {
        world.* = .{
            .name = wire_world.name,
            .x = wire_world.x,
            .y = wire_world.y,
            .owner = parseOwner(.{ .string = wire_world.owner }) catch unreachable,
            .energy = undefined,
            .defense = undefined,
            .base_output = undefined,
            .base_regeneration = undefined,
            .atmosphere = wire_world.atmosphere,
            .hydrosphere = wire_world.hydrosphere,
            .topology = wire_world.topology,
        };
        inline for (.{ "energy", "defense", "base_output", "base_regeneration" }) |field| {
            @field(world, field) = parseU64(@field(wire_world, field)) catch return .{ .failure = .{ .canonical_number = .{ .field = "world." ++ field, .kind = "unsigned decimal" } } };
        }
    }
    owns_draft = false;
    return .{ .draft = draft };
}
