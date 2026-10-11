const std = @import("std");
const store = @import("classic_store.zig");
const preferences = @import("preferences.zig");
const scenario = @import("scenario.zig");
const model = @import("model.zig");
const simulation = @import("simulation.zig");
const t = std.testing;
const a = t.allocator;
const io = t.io;

// Every filesystem qualification owns a unique /private/tmp directory. No
// actual HOME/XDG/APPDATA/profile environment is read, even for path tests.
const Fixture = struct {
    root: []u8,
    dir: std.Io.Dir,
    fn init() !Fixture {
        var random: [16]u8 = undefined;
        io.random(&random);
        const root = try std.fmt.allocPrint(a, "/private/tmp/nyon-task2a-{s}", .{std.fmt.bytesToHex(random, .lower)});
        errdefer a.free(root);
        const dir = try std.Io.Dir.cwd().createDirPathOpen(io, root, .{ .open_options = .{ .iterate = true } });
        return .{ .root = root, .dir = dir };
    }
    fn deinit(self: *Fixture) void {
        self.dir.close(io);
        std.Io.Dir.cwd().deleteTree(io, self.root) catch @panic("owned fixture cleanup failed");
        a.free(self.root);
        self.* = undefined;
    }
    fn path(self: Fixture, name: []const u8) ![]u8 {
        return std.fs.path.join(a, &.{ self.root, name });
    }
    fn write(self: Fixture, name: []const u8, bytes: []const u8) !void {
        try self.dir.writeFile(io, .{ .sub_path = name, .data = bytes });
    }
    fn read(self: Fixture, name: []const u8) ![]u8 {
        return self.dir.readFileAlloc(io, name, a, .limited(70_000));
    }
    fn noTemps(self: Fixture) !void {
        var iter = self.dir.iterate();
        while (try iter.next(io)) |entry| try t.expect(!std.mem.startsWith(u8, entry.name, ".nyon-classic-"));
    }
    fn missing(self: Fixture, name: []const u8) !void {
        try t.expectError(error.FileNotFound, self.dir.openFile(io, name, .{}));
    }
};

fn expectLoaded(raw: store.RawStore, expected: []const u8) !void {
    var result = try raw.load(a);
    defer result.deinit(a);
    try t.expect(result == .payload);
    try t.expectEqualStrings(expected, result.payload);
}
fn expectAbsent(raw: store.RawStore) !void {
    var result = try raw.load(a);
    defer result.deinit(a);
    try t.expect(result == .absent);
}
fn expectSaved(raw: store.RawStore, payload: []const u8) !void {
    var result = try raw.save(a, payload);
    defer result.deinit(a);
    try t.expect(result == .saved);
}
fn expectFailure(raw: store.RawStore, kind: store.FailureKind, cause: ?anyerror) !void {
    var result = try raw.load(a);
    defer result.deinit(a);
    try t.expect(result == .failure);
    try t.expectEqual(kind, result.failure.kind);
    if (cause) |value| try t.expectEqual(value, result.failure.cause.?);
}
fn pathEquals(record: store.Record, platform: store.NativePlatform, env: store.NativePathEnvironment, legacy: bool, expected: []const u8) !void {
    var result = try store.nativePath(a, record, platform, env, legacy);
    defer result.deinit(a);
    try t.expect(result == .path);
    try t.expectEqualStrings(expected, result.path);
}

test "exact platform primary and legacy vectors retain namespaces" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        const file = record.filename();
        const mac_primary = try std.fmt.allocPrint(a, "/Users/example/Library/Application Support/NYON/{s}", .{file});
        defer a.free(mac_primary);
        const mac_legacy = try std.fmt.allocPrint(a, "/Users/example/Library/Application Support/Intergalactic Warfare/{s}", .{file});
        defer a.free(mac_legacy);
        try pathEquals(record, .macos, .{ .home = "/Users/example" }, false, mac_primary);
        try pathEquals(record, .macos, .{ .home = "/Users/example" }, true, mac_legacy);
        const win_primary = try std.fmt.allocPrint(a, "C:\\Users\\example\\AppData\\Roaming\\NYON\\{s}", .{file});
        defer a.free(win_primary);
        const win_legacy = try std.fmt.allocPrint(a, "C:\\Users\\example\\AppData\\Roaming\\Intergalactic Warfare\\{s}", .{file});
        defer a.free(win_legacy);
        try pathEquals(record, .windows, .{ .appdata = "C:\\Users\\example\\AppData\\Roaming" }, false, win_primary);
        try pathEquals(record, .windows, .{ .appdata = "C:\\Users\\example\\AppData\\Roaming" }, true, win_legacy);
        const linux_primary = try std.fmt.allocPrint(a, "/data/nyon/{s}", .{file});
        defer a.free(linux_primary);
        const linux_legacy = try std.fmt.allocPrint(a, "/data/intergalactic-warfare/{s}", .{file});
        defer a.free(linux_legacy);
        try pathEquals(record, .linux, .{ .xdg_data_home = "/data" }, false, linux_primary);
        try pathEquals(record, .linux, .{ .xdg_data_home = "/data" }, true, linux_legacy);
    }
}

test "Linux relative empty and absent XDG use absolute HOME and missing roots are failures" {
    const invalid_xdg = [_]?[]const u8{ null, "", "relative", "./data" };
    for (invalid_xdg) |xdg| {
        try pathEquals(.scenario, .linux, .{ .home = "/home/test", .xdg_data_home = xdg }, false, "/home/test/.local/share/nyon/scenario-v1.json");
        try pathEquals(.preferences, .linux, .{ .home = "/home/test", .xdg_data_home = xdg }, true, "/home/test/.local/share/intergalactic-warfare/preferences-v1.json");
        for ([_]?[]const u8{ null, "", "relative" }) |home| {
            var result = try store.nativePath(a, .scenario, .linux, .{ .home = home, .xdg_data_home = xdg }, false);
            defer result.deinit(a);
            try t.expect(result == .failure);
            try t.expectEqual(error.FileNotFound, result.failure.cause.?);
        }
    }
    for ([_]store.NativePlatform{ .macos, .windows, .linux }) |platform| {
        for ([_]store.Record{ .scenario, .preferences }) |record| {
            var result = try store.nativePath(a, record, platform, .{}, false);
            defer result.deinit(a);
            try t.expect(result == .failure);
            const message = try result.failure.messageAlloc(a);
            defer a.free(message);
            try t.expectEqualStrings(if (record == .scenario) "scenario storage failed: required application-data environment path is unavailable" else "preference storage path is unavailable: scenario storage failed: required application-data environment path is unavailable", message);
        }
    }
    // Rust does not reject non-absolute/empty macOS and Windows environment roots.
    try pathEquals(.scenario, .macos, .{ .home = "" }, false, "Library/Application Support/NYON/scenario-v1.json");
    try pathEquals(.preferences, .windows, .{ .appdata = "relative" }, false, "relative\\NYON\\preferences-v1.json");
}

test "profile resolver rejects invalid roots and preserves Unix opaque bytes" {
    try t.expectEqual(@as(?[]u8, null), try store.resolveProfileRoot(a, .macos, null));
    for ([_][]const u8{ "", "relative", "./profile" }) |value| try t.expectError(error.InvalidProfileDirectory, store.resolveProfileRoot(a, .macos, value));
    const bytes = "/private/tmp/opaque-\xff";
    const root = (try store.resolveProfileRoot(a, .macos, bytes)).?;
    defer a.free(root);
    try t.expectEqualSlices(u8, bytes, root);
    const windows = (try store.resolveProfileRoot(a, .windows, "C:\\profile")).?;
    defer a.free(windows);
    try t.expectEqualStrings("C:\\profile", windows);
    try t.expectError(error.InvalidProfileDirectory, store.resolveProfileRoot(a, .windows, "C:relative"));
}

test "raw memory absence present empty present invalid and result ownership" {
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    try expectAbsent(memory.raw());
    try expectSaved(memory.raw(), "");
    try expectLoaded(memory.raw(), "");
    try expectSaved(memory.raw(), "not JSON");
    var loaded = try memory.raw().load(a);
    defer loaded.deinit(a);
    try expectSaved(memory.raw(), "replacement");
    try t.expectEqualStrings("not JSON", loaded.payload);
    try expectLoaded(memory.raw(), "replacement");
}

test "memory save failure is one shot preserves previous or absent slot and owned message" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var memory = store.MemoryStore.init(a, record);
        defer memory.deinit();
        try memory.failNextSave("injected exact cause");
        var failed = try memory.raw().save(a, "new");
        defer failed.deinit(a);
        try t.expect(failed == .failure);
        try t.expectEqualStrings("injected exact cause", failed.failure.message.?);
        try expectAbsent(memory.raw());
        try expectSaved(memory.raw(), "old");
        try memory.failNextSave("replacement failure");
        var failed_again = try memory.raw().save(a, "new");
        defer failed_again.deinit(a);
        try expectLoaded(memory.raw(), "old");
        try expectSaved(memory.raw(), "new");
        try expectLoaded(memory.raw(), "new");
    }
}

test "preference memory caps precede save hook consumption and denial precedes load cap" {
    const bytes = try a.alloc(u8, preferences.MAX_JSON_BYTES + 1);
    defer a.free(bytes);
    @memset(bytes, 'x');
    var memory = try store.MemoryStore.withSlot(a, .preferences, bytes);
    defer memory.deinit();
    try expectFailure(memory.raw(), .oversized_slot, null);
    try memory.denyLoad("denied before cap");
    try expectFailure(memory.raw(), .browser, null);
    try expectFailure(memory.raw(), .browser, null);
    try memory.failNextSave("pending");
    var oversized = try memory.raw().save(a, bytes);
    defer oversized.deinit(a);
    try t.expectEqual(store.FailureKind.oversized_slot, oversized.failure.kind);
    try t.expectEqualStrings("pending", memory.next_save_failure.?);
    var failed = try memory.raw().save(a, "small");
    defer failed.deinit(a);
    try t.expectEqualStrings("pending", failed.failure.message.?);
    try expectSaved(memory.raw(), bytes[0..preferences.MAX_JSON_BYTES]);
    var uncapped = store.MemoryStore.init(a, .scenario);
    defer uncapped.deinit();
    const large_scenario = try a.alloc(u8, scenario.MAX_JSON_BYTES + 1);
    defer a.free(large_scenario);
    @memset(large_scenario, 's');
    try expectSaved(uncapped.raw(), large_scenario);
    try expectLoaded(uncapped.raw(), large_scenario);
}

test "fallback invokes each load once and preserves invalid or errored primary" {
    var primary = store.MemoryStore.init(a, .scenario);
    defer primary.deinit();
    var legacy = try store.MemoryStore.withSlot(a, .scenario, "legacy bytes");
    defer legacy.deinit();
    var fallback = store.FallbackStore{ .primary = primary.raw(), .legacy = legacy.raw() };
    try expectLoaded(fallback.raw(), "legacy bytes");
    try t.expectEqual(@as(usize, 1), primary.load_calls);
    try t.expectEqual(@as(usize, 1), legacy.load_calls);
    try t.expect(primary.slot == null);
    try expectSaved(fallback.raw(), "");
    try expectLoaded(fallback.raw(), "");
    try expectSaved(fallback.raw(), "invalid JSON");
    try expectLoaded(fallback.raw(), "invalid JSON");
    try primary.denyLoad("primary error");
    try expectFailure(fallback.raw(), .browser, null);
    try t.expectEqual(@as(usize, 1), legacy.load_calls);
    try t.expectEqualStrings("legacy bytes", legacy.slot.?);
}

test "fallback legacy failure follows primary absence and no primary is created" {
    var primary = store.MemoryStore.init(a, .preferences);
    defer primary.deinit();
    var legacy = store.MemoryStore.init(a, .preferences);
    defer legacy.deinit();
    try legacy.denyLoad("legacy failure");
    var fallback = store.FallbackStore{ .primary = primary.raw(), .legacy = legacy.raw() };
    var result = try fallback.raw().load(a);
    defer result.deinit(a);
    try t.expectEqualStrings("legacy failure", result.failure.message.?);
    try t.expectEqual(@as(usize, 1), primary.load_calls);
    try t.expectEqual(@as(usize, 1), legacy.load_calls);
    try t.expect(primary.slot == null);
}

test "typed preference defaults success codec failure and original store failure" {
    var memory = store.MemoryStore.init(a, .preferences);
    defer memory.deinit();
    var absent = try store.loadOrDefault(a, memory.raw());
    defer absent.deinit(a);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, absent.preferences);
    try t.expect(absent.failure == null and memory.slot == null);
    const requested: preferences.UserPreferencesV1 = .{ .ui_scale = .percent130, .motion = .reduced, .high_contrast = true, .graphics_quality = .high, .onboarding_completed = true };
    var saved = try store.saveOrDefault(a, memory.raw(), requested);
    defer saved.deinit(a);
    try t.expect(saved.failure == null);
    try t.expectEqualDeep(requested, saved.preferences);
    var loaded = try store.loadOrDefault(a, memory.raw());
    defer loaded.deinit(a);
    try t.expectEqualDeep(requested, loaded.preferences);
    try memory.failNextSave("exact injected message");
    var failed_save = try store.saveOrDefault(a, memory.raw(), .{});
    defer failed_save.deinit(a);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, failed_save.preferences);
    try t.expect(failed_save.failure.? == .store);
    const message = (try failed_save.recoverableMessage(a)).?;
    defer a.free(message);
    try t.expectEqualStrings("preference storage failed before atomic persist: exact injected message", message);
    var still_saved = try store.loadOrDefault(a, memory.raw());
    defer still_saved.deinit(a);
    try t.expectEqualDeep(requested, still_saved.preferences);
    try expectSaved(memory.raw(), "{\"format_version\":2,\"ui_scale_percent\":100,\"motion\":\"FULL\",\"high_contrast\":false,\"graphics_quality\":\"AUTO\",\"onboarding_completed\":false}");
    var codec_error = try store.loadOrDefault(a, memory.raw());
    defer codec_error.deinit(a);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, codec_error.preferences);
    try t.expect(codec_error.failure.? == .codec);
    try t.expect(codec_error.failure.?.codec == .version);
    try memory.denyLoad("load denied");
    var store_error = try store.loadOrDefault(a, memory.raw());
    defer store_error.deinit(a);
    try t.expect(store_error.failure.? == .store);
    try t.expectEqual(store.FailureKind.browser, store_error.failure.?.store.kind);
    try t.expectEqualStrings("load denied", store_error.failure.?.store.message.?);
}

test "typed preference malformed invalid scale and oversized codec failures preserve details" {
    const payloads = [_][]const u8{ "", "{}", "{\"format_version\":1,\"ui_scale_percent\":99,\"motion\":\"FULL\",\"high_contrast\":false,\"graphics_quality\":\"AUTO\",\"onboarding_completed\":false}" };
    for (payloads, 0..) |payload, index| {
        var memory = try store.MemoryStore.withSlot(a, .preferences, payload);
        defer memory.deinit();
        var outcome = try store.loadOrDefault(a, memory.raw());
        defer outcome.deinit(a);
        try t.expect(outcome.failure.? == .codec);
        if (index == 2) {
            try t.expectEqual(@as(u16, 99), outcome.failure.?.codec.ui_scale);
            const message = (try outcome.recoverableMessage(a)).?;
            defer a.free(message);
            try t.expectEqualStrings("unsupported UI scale percentage 99", message);
        } else try t.expect(outcome.failure.?.codec == .json);
    }
    // Preference memory catches its own cap as Store, not Codec.
    const large = try a.alloc(u8, preferences.MAX_JSON_BYTES + 1);
    defer a.free(large);
    @memset(large, 'x');
    var memory = try store.MemoryStore.withSlot(a, .preferences, large);
    defer memory.deinit();
    var oversized = try store.loadOrDefault(a, memory.raw());
    defer oversized.deinit(a);
    try t.expect(oversized.failure.? == .store);
    try t.expectEqual(preferences.MAX_JSON_BYTES, oversized.failure.?.store.max_bytes);
}

test "native injected profiles have separate filenames no legacy and fresh durable codec readback" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    const nested = try fixture.path("nested/profile");
    defer a.free(nested);
    var scenarios = try store.NativeStore.fromProfile(a, io, .scenario, .macos, nested);
    defer scenarios.deinit();
    var prefs = try store.NativeStore.fromProfile(a, io, .preferences, .macos, nested);
    defer prefs.deinit();
    try t.expect(scenarios.legacy_path == null and prefs.legacy_path == null);
    try t.expect(std.mem.endsWith(u8, scenarios.path(), store.SCENARIO_FILE_NAME));
    try t.expect(std.mem.endsWith(u8, prefs.path(), store.PREFERENCES_FILE_NAME));
    try expectAbsent(scenarios.raw());
    try expectAbsent(prefs.raw());
    const valid = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    const admitted = try scenario.admit(&valid);
    const encoded = try scenario.encode(a, &admitted);
    defer a.free(encoded);
    try expectSaved(scenarios.raw(), encoded);
    var saved = try store.saveOrDefault(a, prefs.raw(), .{ .ui_scale = .percent115, .graphics_quality = .low });
    defer saved.deinit(a);
    try t.expect(saved.failure == null);
    var fresh = try store.NativeStore.fromProfile(a, io, .scenario, .macos, nested);
    defer fresh.deinit();
    var loaded = try fresh.raw().load(a);
    defer loaded.deinit(a);
    const decoded = try scenario.decode(a, loaded.payload);
    try t.expectEqual(admitted.fingerprint, decoded.fingerprint);
    var active = try simulation.Simulation.fromScenario(a, &decoded);
    defer active.deinit();
    try t.expectEqual(admitted.fingerprint, active.scenario_fingerprint.?);
    var fresh_prefs = try store.NativeStore.fromProfile(a, io, .preferences, .macos, nested);
    defer fresh_prefs.deinit();
    var read_prefs = try store.loadOrDefault(a, fresh_prefs.raw());
    defer read_prefs.deinit(a);
    try t.expectEqualDeep(saved.preferences, read_prefs.preferences);
    try expectLoaded(scenarios.raw(), encoded);
    const persisted = try std.Io.Dir.cwd().openFile(io, scenarios.path(), .{});
    defer persisted.close(io);
    const stat = try persisted.stat(io);
    try t.expectEqual(@as(std.posix.mode_t, 0o600), stat.permissions.toMode() & 0o777);
}

test "native absent-only fallback is read-only for both record namespaces" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var fixture = try Fixture.init();
        defer fixture.deinit();
        try fixture.write("legacy.json", "legacy sentinel");
        const primary = try fixture.path(record.filename());
        defer a.free(primary);
        const legacy = try fixture.path("legacy.json");
        defer a.free(legacy);
        var native = try store.NativeStore.atPaths(a, io, record, primary, legacy);
        defer native.deinit();
        try expectLoaded(native.raw(), "legacy sentinel");
        try fixture.missing(record.filename());
        try expectSaved(native.raw(), "");
        try expectLoaded(native.raw(), "");
        try expectSaved(native.raw(), "invalid primary JSON");
        try expectLoaded(native.raw(), "invalid primary JSON");
        const bytes = try fixture.read("legacy.json");
        defer a.free(bytes);
        try t.expectEqualStrings("legacy sentinel", bytes);
        try fixture.noTemps();
    }
}

test "native raw cap boundaries invalid UTF8 and errored primary block legacy" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var fixture = try Fixture.init();
        defer fixture.deinit();
        const primary = try fixture.path(record.filename());
        defer a.free(primary);
        const legacy = try fixture.path("legacy.json");
        defer a.free(legacy);
        try fixture.write("legacy.json", "valid legacy");
        var native = try store.NativeStore.atPaths(a, io, record, primary, legacy);
        defer native.deinit();
        const bytes = try a.alloc(u8, record.maxBytes() + 2);
        defer a.free(bytes);
        @memset(bytes, 'x');
        try fixture.write(record.filename(), bytes[0..record.maxBytes()]);
        try expectLoaded(native.raw(), bytes[0..record.maxBytes()]);
        try fixture.write(record.filename(), bytes[0 .. record.maxBytes() + 1]);
        try expectFailure(native.raw(), .oversized_slot, null);
        try fixture.write(record.filename(), bytes);
        try expectFailure(native.raw(), .oversized_slot, null);
        for ([_][]const u8{ "\xff", "\xc0\xaf", "\xe2\x82" }) |invalid| {
            try fixture.write(record.filename(), invalid);
            try expectFailure(native.raw(), .io, error.InvalidUtf8);
        }
        try fixture.dir.deleteFile(io, record.filename());
        try fixture.dir.createDir(io, record.filename(), .default_dir);
        try expectFailure(native.raw(), .io, error.IsDir);
        const legacy_read = try fixture.read("legacy.json");
        defer a.free(legacy_read);
        try t.expectEqualStrings("valid legacy", legacy_read);
    }
}

test "native scenario raw save is uncapped but load is bounded" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    var native = try store.NativeStore.fromProfile(a, io, .scenario, .macos, fixture.root);
    defer native.deinit();
    const bytes = try a.alloc(u8, scenario.MAX_JSON_BYTES + 1);
    defer a.free(bytes);
    @memset(bytes, 's');
    try expectSaved(native.raw(), bytes);
    const readback = try fixture.read(store.SCENARIO_FILE_NAME);
    defer a.free(readback);
    try t.expectEqualSlices(u8, bytes, readback);
    try expectFailure(native.raw(), .oversized_slot, null);
}

const HookState = struct {
    calls: usize = 0,
    record: store.Record,
    fn reject(context: *anyopaque, allocator: std.mem.Allocator) std.mem.Allocator.Error!?store.StoreFailure {
        const self: *HookState = @ptrCast(@alignCast(context));
        self.calls += 1;
        return .{ .record = self.record, .kind = .before_persist, .message = try allocator.dupe(u8, "hook original failure") };
    }
    fn accept(context: *anyopaque, _: std.mem.Allocator) std.mem.Allocator.Error!?store.StoreFailure {
        const self: *HookState = @ptrCast(@alignCast(context));
        self.calls += 1;
        return null;
    }
};

test "native before-persist rejection preserves absent old primary and legacy with cleanup" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var fixture = try Fixture.init();
        defer fixture.deinit();
        const primary = try fixture.path(record.filename());
        defer a.free(primary);
        const legacy = try fixture.path("legacy.json");
        defer a.free(legacy);
        try fixture.write("legacy.json", "legacy sentinel");
        var native = try store.NativeStore.atPaths(a, io, record, primary, legacy);
        defer native.deinit();
        var hook = HookState{ .record = record };
        var failed_absent = try native.saveWithBeforePersist(a, "new", .{ .context = &hook, .call = HookState.reject });
        defer failed_absent.deinit(a);
        try t.expectEqual(@as(usize, 1), hook.calls);
        try t.expectEqual(store.FailureKind.before_persist, failed_absent.failure.kind);
        try t.expectEqualStrings("hook original failure", failed_absent.failure.message.?);
        try fixture.missing(record.filename());
        try fixture.noTemps();
        try expectLoaded(native.raw(), "legacy sentinel");
        try expectSaved(native.raw(), "old primary");
        var failed_existing = try native.saveWithBeforePersist(a, "new", .{ .context = &hook, .call = HookState.reject });
        defer failed_existing.deinit(a);
        try t.expectEqual(@as(usize, 2), hook.calls);
        try expectLoaded(native.raw(), "old primary");
        try fixture.noTemps();
        var accepted = try native.saveWithBeforePersist(a, "new", .{ .context = &hook, .call = HookState.accept });
        defer accepted.deinit(a);
        try t.expect(accepted == .saved);
        try t.expectEqual(@as(usize, 3), hook.calls);
        try expectLoaded(native.raw(), "new");
        const legacy_bytes = try fixture.read("legacy.json");
        defer a.free(legacy_bytes);
        try t.expectEqualStrings("legacy sentinel", legacy_bytes);
        try fixture.noTemps();
    }
}

test "native injected creation write sync persist failures preserve records and clean temps" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        for ([_]store.FaultStage{ .create, .write, .sync, .persist }) |stage| {
            var fixture = try Fixture.init();
            defer fixture.deinit();
            var native = try store.NativeStore.fromProfile(a, io, record, .macos, fixture.root);
            defer native.deinit();
            try native.failNext(stage, "stage original message");
            var absent_failure = try native.raw().save(a, "new");
            defer absent_failure.deinit(a);
            try t.expect(absent_failure == .failure);
            try t.expectEqual(error.InjectedFailure, absent_failure.failure.cause.?);
            try t.expectEqualStrings("stage original message", absent_failure.failure.message.?);
            try expectAbsent(native.raw());
            try fixture.noTemps();
            try expectSaved(native.raw(), "old");
            try native.failNext(stage, "stage original message");
            var failure = try native.raw().save(a, "new");
            defer failure.deinit(a);
            try t.expect(failure == .failure and failure.failure.cleanup_error == null);
            try expectLoaded(native.raw(), "old");
            try fixture.noTemps();
            try expectSaved(native.raw(), "new");
            try expectLoaded(native.raw(), "new");
        }
    }
}

test "native preference cap precedes parent creation and all fault or callback consumption" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    const nested = try fixture.path("missing/profile");
    defer a.free(nested);
    var native = try store.NativeStore.fromProfile(a, io, .preferences, .macos, nested);
    defer native.deinit();
    const bytes = try a.alloc(u8, preferences.MAX_JSON_BYTES + 1);
    defer a.free(bytes);
    @memset(bytes, 'x');
    var hook = HookState{ .record = .preferences };
    try native.failNext(.persist, "pending fault");
    var oversized = try native.saveWithBeforePersist(a, bytes, .{ .context = &hook, .call = HookState.reject });
    defer oversized.deinit(a);
    try t.expectEqual(store.FailureKind.oversized_slot, oversized.failure.kind);
    try t.expectEqual(@as(usize, 0), hook.calls);
    try t.expect(native.next_fault != null);
    try fixture.missing("missing");
    var failed = try native.raw().save(a, bytes[0..preferences.MAX_JSON_BYTES]);
    defer failed.deinit(a);
    try t.expectEqualStrings("pending fault", failed.failure.message.?);
    try expectAbsent(native.raw());
    try expectSaved(native.raw(), bytes[0..preferences.MAX_JSON_BYTES]);
    try expectLoaded(native.raw(), bytes[0..preferences.MAX_JSON_BYTES]);
}

test "real replacement failure retains destination directory and its sentinel and removes temp" {
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var fixture = try Fixture.init();
        defer fixture.deinit();
        try fixture.dir.createDir(io, record.filename(), .default_dir);
        const sentinel_name = try std.fs.path.join(a, &.{ record.filename(), "sentinel" });
        defer a.free(sentinel_name);
        try fixture.write(sentinel_name, "retained destination");
        try fixture.write("legacy.json", "retained legacy");
        const primary = try fixture.path(record.filename());
        defer a.free(primary);
        const legacy = try fixture.path("legacy.json");
        defer a.free(legacy);
        var native = try store.NativeStore.atPaths(a, io, record, primary, legacy);
        defer native.deinit();
        var failed = try native.raw().save(a, "replacement");
        defer failed.deinit(a);
        try t.expect(failed == .failure);
        try t.expectEqual(store.FailureKind.io, failed.failure.kind);
        try t.expect(failed.failure.cause != null and failed.failure.cleanup_error == null);
        const sentinel = try fixture.read(sentinel_name);
        defer a.free(sentinel);
        try t.expectEqualStrings("retained destination", sentinel);
        const legacy_bytes = try fixture.read("legacy.json");
        defer a.free(legacy_bytes);
        try t.expectEqualStrings("retained legacy", legacy_bytes);
        try fixture.noTemps();
    }
}

test "real parent creation failure and parentless save retain exact typed cause" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    try fixture.write("blocked", "parent sentinel");
    const path = try fixture.path("blocked/slot.json");
    defer a.free(path);
    var native = try store.NativeStore.atPath(a, io, .scenario, path);
    defer native.deinit();
    var failed = try native.raw().save(a, "replacement");
    defer failed.deinit(a);
    try t.expectEqual(store.FailureKind.io, failed.failure.kind);
    try t.expectEqual(error.NotDir, failed.failure.cause.?);
    const sentinel = try fixture.read("blocked");
    defer a.free(sentinel);
    try t.expectEqualStrings("parent sentinel", sentinel);
    try fixture.noTemps();
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var parentless = try store.NativeStore.atPath(a, io, record, "");
        defer parentless.deinit();
        var failure = try parentless.raw().save(a, "x");
        defer failure.deinit(a);
        try t.expectEqual(error.InvalidInput, failure.failure.cause.?);
        try t.expectEqualStrings(if (record == .scenario) "scenario slot has no parent directory" else "preference slot has no parent directory", failure.failure.message.?);
    }
}

test "native preference failure leaves scenario sentinel and vice versa" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    var scenarios = try store.NativeStore.fromProfile(a, io, .scenario, .macos, fixture.root);
    defer scenarios.deinit();
    var prefs = try store.NativeStore.fromProfile(a, io, .preferences, .macos, fixture.root);
    defer prefs.deinit();
    try expectSaved(scenarios.raw(), "scenario sentinel");
    try expectSaved(prefs.raw(), "preference sentinel");
    try prefs.failNext(.persist, "preference failure");
    var pf = try store.saveOrDefault(a, prefs.raw(), .{});
    defer pf.deinit(a);
    try t.expect(pf.failure.? == .store);
    try expectLoaded(scenarios.raw(), "scenario sentinel");
    try expectLoaded(prefs.raw(), "preference sentinel");
    try scenarios.failNext(.sync, "scenario failure");
    var sf = try scenarios.raw().save(a, "new scenario");
    defer sf.deinit(a);
    try expectLoaded(scenarios.raw(), "scenario sentinel");
    try expectLoaded(prefs.raw(), "preference sentinel");
    try fixture.noTemps();
}

test "native explicit profile preserves nonUTF8 bytes and exposes host path rejection" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    const root = try fixture.path("opaque-\xff");
    defer a.free(root);
    var native = try store.NativeStore.fromProfile(a, io, .scenario, .macos, root);
    defer native.deinit();
    try t.expectEqualSlices(u8, root, std.fs.path.dirname(native.path()).?);
    var saved = try native.raw().save(a, "valid UTF8 record");
    defer saved.deinit(a);
    try t.expect(saved == .failure);
    try t.expectEqual(store.FailureKind.io, saved.failure.kind);
    try t.expectEqual(error.BadPathName, saved.failure.cause.?);
}

fn allocationOwnership(allocator: std.mem.Allocator) !void {
    var memory = try store.MemoryStore.withSlot(allocator, .preferences, "old");
    defer memory.deinit();
    try memory.denyLoad("denied");
    var denied = try memory.raw().load(allocator);
    defer denied.deinit(allocator);
    const message = try denied.failure.messageAlloc(allocator);
    defer allocator.free(message);
    try memory.failNextSave("pending failure");
    var failed = try memory.raw().save(allocator, "new");
    defer failed.deinit(allocator);
    var saved = try store.saveOrDefault(allocator, memory.raw(), .{});
    defer saved.deinit(allocator);
    var paths = try store.NativeStore.fromEnvironment(allocator, io, .preferences, .linux, .{ .home = "/injected/unopened/home" });
    defer paths.deinit(allocator);
    var missing = try store.NativeStore.fromEnvironment(allocator, io, .preferences, .macos, .{});
    defer missing.deinit(allocator);
    var unavailable = try store.UnavailableStore.init(allocator, missing.failure);
    defer unavailable.deinit();
    var unavailable_outcome = try store.loadOrDefault(allocator, unavailable.raw());
    defer unavailable_outcome.deinit(allocator);
    const unavailable_message = (try unavailable_outcome.recoverableMessage(allocator)).?;
    defer allocator.free(unavailable_message);
    var profile = try store.NativeStore.fromProfile(allocator, io, .scenario, .macos, "/injected/unopened/profile");
    defer profile.deinit();
}

test "all memory path construction and failure-message allocations have explicit cleanup" {
    try t.checkAllAllocationFailures(a, allocationOwnership, .{});
}

fn nativeAllocationOwnership(allocator: std.mem.Allocator) !void {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    var native = try store.NativeStore.fromProfile(allocator, io, .preferences, .macos, fixture.root);
    defer native.deinit();
    // Test allocator failures before replacement preserve durable old bytes.
    try fixture.write(store.PREFERENCES_FILE_NAME, "old");
    var saved = native.raw().save(allocator, "new") catch |failure| {
        try expectLoaded(native.raw(), "old");
        try fixture.noTemps();
        return failure;
    };
    defer saved.deinit(allocator);
    var loaded = try native.raw().load(allocator);
    defer loaded.deinit(allocator);
    try t.expectEqualStrings("new", loaded.payload);
    try native.failNext(.persist, "pending persist failure");
    var failed = native.raw().save(allocator, "replacement") catch |failure| {
        try expectLoaded(native.raw(), "new");
        try fixture.noTemps();
        return failure;
    };
    defer failed.deinit(allocator);
    try expectLoaded(native.raw(), "new");
    try fixture.noTemps();
}

test "all native save load and fault-message allocations clean up and preserve slot on OOM" {
    try t.checkAllAllocationFailures(a, nativeAllocationOwnership, .{});
}

fn typedAllocationOwnership(allocator: std.mem.Allocator, payload: []const u8) !void {
    var memory = try store.MemoryStore.withSlot(allocator, .preferences, payload);
    defer memory.deinit();
    var outcome = try store.loadOrDefault(allocator, memory.raw());
    defer outcome.deinit(allocator);
    try t.expect(outcome.failure == null);
    var saved = try store.saveOrDefault(allocator, memory.raw(), outcome.preferences);
    defer saved.deinit(allocator);
    try t.expect(saved.failure == null);
}

test "typed codec and store allocations keep OutOfMemory distinct" {
    const payload = try preferences.encode(a, .{ .ui_scale = .percent85 });
    defer a.free(payload);
    try t.checkAllAllocationFailures(a, typedAllocationOwnership, .{payload});
    var memory = try store.MemoryStore.withSlot(a, .preferences, "persisted sentinel");
    defer memory.deinit();
    const old = memory.slot.?;
    var failing = t.FailingAllocator.init(a, .{ .fail_index = 0 });
    try t.expectError(error.OutOfMemory, store.saveOrDefault(failing.allocator(), memory.raw(), .{}));
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    try t.expectEqualStrings("persisted sentinel", old);
    try memory.failNextSave("pending");
    try t.expectError(error.OutOfMemory, memory.raw().save(failing.allocator(), "new"));
    try t.expectEqualStrings("pending", memory.next_save_failure.?);
    try t.expectEqualStrings("persisted sentinel", memory.slot.?);
}

test "wrong-record preference facade never calls or changes scenario storage" {
    var memory = try store.MemoryStore.withSlot(a, .scenario, "scenario sentinel");
    defer memory.deinit();
    var loaded = try store.loadOrDefault(a, memory.raw());
    defer loaded.deinit(a);
    var saved = try store.saveOrDefault(a, memory.raw(), .{ .onboarding_completed = true });
    defer saved.deinit(a);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, loaded.preferences);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, saved.preferences);
    try t.expectEqual(error.RecordMismatch, loaded.failure.?.store.cause.?);
    try t.expectEqual(error.RecordMismatch, saved.failure.?.store.cause.?);
    try t.expectEqual(@as(usize, 0), memory.load_calls);
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    try t.expectEqualStrings("scenario sentinel", memory.slot.?);
}

test "mismatched fallback rejects both load and save before either namespace is called" {
    var primary = try store.MemoryStore.withSlot(a, .scenario, "scenario sentinel");
    defer primary.deinit();
    var legacy = try store.MemoryStore.withSlot(a, .preferences, "preference sentinel");
    defer legacy.deinit();
    var fallback = store.FallbackStore{ .primary = primary.raw(), .legacy = legacy.raw() };
    var load_direct = try store.loadPrimaryOrLegacy(a, primary.raw(), legacy.raw());
    defer load_direct.deinit(a);
    var loaded = try fallback.raw().load(a);
    defer loaded.deinit(a);
    var saved = try fallback.raw().save(a, "new");
    defer saved.deinit(a);
    try t.expectEqual(error.RecordMismatch, load_direct.failure.cause.?);
    try t.expectEqual(error.RecordMismatch, loaded.failure.cause.?);
    try t.expectEqual(error.RecordMismatch, saved.failure.cause.?);
    try t.expectEqual(@as(usize, 0), primary.load_calls + primary.save_calls + legacy.load_calls + legacy.save_calls);
    try t.expectEqualStrings("scenario sentinel", primary.slot.?);
    try t.expectEqualStrings("preference sentinel", legacy.slot.?);
}

test "mutated raw metadata and stale adapter metadata fail before dispatch" {
    var scenario_memory = try store.MemoryStore.withSlot(a, .scenario, "scenario sentinel");
    defer scenario_memory.deinit();
    var raw = scenario_memory.raw();
    raw.record = .preferences;
    var loaded = try store.loadOrDefault(a, raw);
    defer loaded.deinit(a);
    var saved = try store.saveOrDefault(a, raw, .{});
    defer saved.deinit(a);
    try t.expectEqual(error.RecordMismatch, loaded.failure.?.store.cause.?);
    try t.expectEqual(error.RecordMismatch, saved.failure.?.store.cause.?);
    var raw_failed = try raw.save(a, "new");
    defer raw_failed.deinit(a);
    try t.expectEqual(error.RecordMismatch, raw_failed.failure.cause.?);
    var pref_memory = try store.MemoryStore.withSlot(a, .preferences, "retained sentinel");
    defer pref_memory.deinit();
    const stale = pref_memory.raw();
    pref_memory.record = .scenario;
    var stale_load = try stale.load(a);
    defer stale_load.deinit(a);
    var stale_save = try store.saveOrDefault(a, stale, .{});
    defer stale_save.deinit(a);
    try t.expectEqual(error.RecordMismatch, stale_load.failure.cause.?);
    try t.expectEqual(error.RecordMismatch, stale_save.failure.?.store.cause.?);
    try t.expectEqual(@as(usize, 0), scenario_memory.load_calls + scenario_memory.save_calls + pref_memory.load_calls + pref_memory.save_calls);
    try t.expectEqualStrings("scenario sentinel", scenario_memory.slot.?);
    try t.expectEqualStrings("retained sentinel", pref_memory.slot.?);
}

test "unavailable construction failure remains typed owned and recoverable without default saving" {
    var resolution = try store.NativeStore.fromEnvironment(a, io, .preferences, .macos, .{});
    defer resolution.deinit(a);
    try t.expect(resolution == .failure);
    var unavailable = try store.UnavailableStore.init(a, resolution.failure);
    defer unavailable.deinit();
    var loaded = try store.loadOrDefault(a, unavailable.raw());
    defer loaded.deinit(a);
    var saved = try store.saveOrDefault(a, unavailable.raw(), .{ .onboarding_completed = true });
    defer saved.deinit(a);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, loaded.preferences);
    try t.expectEqualDeep(preferences.UserPreferencesV1{}, saved.preferences);
    try t.expectEqual(store.FailureKind.unavailable_path, loaded.failure.?.store.kind);
    try t.expectEqual(store.FailureKind.unavailable_path, saved.failure.?.store.kind);
    try t.expectEqual(error.FileNotFound, loaded.failure.?.store.cause.?);
    const message = (try saved.recoverableMessage(a)).?;
    defer a.free(message);
    try t.expectEqualStrings("preference storage path is unavailable: scenario storage failed: required application-data environment path is unavailable", message);
    try t.expect(resolution.failure.message.?.ptr != unavailable.failure.message.?.ptr);
    try t.expect(unavailable.failure.message.?.ptr != loaded.failure.?.store.message.?.ptr);
}

test "load buffers and failure messages outlive concrete memory store" {
    var memory = try store.MemoryStore.withSlot(a, .preferences, "retained record");
    var loaded = try memory.raw().load(a);
    defer loaded.deinit(a);
    try memory.failNextSave("retained message");
    var failure = try memory.raw().save(a, "new");
    defer failure.deinit(a);
    memory.deinit();
    try t.expectEqualStrings("retained record", loaded.payload);
    try t.expectEqualStrings("retained message", failure.failure.message.?);
}

const RemoveOwnedTemporaryHook = struct {
    dir: std.Io.Dir,
    record: store.Record,
    removed: usize = 0,
    fn call(context: *anyopaque, _: std.mem.Allocator) std.mem.Allocator.Error!?store.StoreFailure {
        const self: *RemoveOwnedTemporaryHook = @ptrCast(@alignCast(context));
        var iter = self.dir.iterate();
        while (iter.next(io) catch |cause| return .{ .record = self.record, .kind = .io, .cause = cause }) |entry| {
            if (!std.mem.startsWith(u8, entry.name, ".nyon-classic-")) continue;
            self.dir.deleteFile(io, entry.name) catch |cause| return .{ .record = self.record, .kind = .io, .cause = cause };
            self.removed += 1;
        }
        return null;
    }
};

test "real missing-temporary persist failure preserves old slot and exposes secondary cleanup failure" {
    // Callback removes only the file owned by this disposable save. Rename and
    // subsequent cleanup produce real FileNotFound results, without OS-wide
    // permission changes, process disruption, or touching another store.
    var fixture = try Fixture.init();
    defer fixture.deinit();
    var native = try store.NativeStore.fromProfile(a, io, .scenario, .macos, fixture.root);
    defer native.deinit();
    try expectSaved(native.raw(), "old primary");
    var hook = RemoveOwnedTemporaryHook{ .dir = fixture.dir, .record = .scenario };
    var failed = try native.saveWithBeforePersist(a, "new primary", .{ .context = &hook, .call = RemoveOwnedTemporaryHook.call });
    defer failed.deinit(a);
    try t.expectEqual(@as(usize, 1), hook.removed);
    try t.expectEqual(store.FailureKind.io, failed.failure.kind);
    try t.expectEqual(error.FileNotFound, failed.failure.cause.?);
    try t.expectEqual(error.FileNotFound, failed.failure.cleanup_error.?);
    try t.expectEqual(error.FileNotFound, native.last_cleanup_error.?);
    try expectLoaded(native.raw(), "old primary");
    try fixture.noTemps();
    try expectSaved(native.raw(), "later primary");
    try t.expect(native.last_cleanup_error == null);
    try expectLoaded(native.raw(), "later primary");
}

test "single-component relative native path uses borrowed owned directory like Rust parent" {
    var fixture = try Fixture.init();
    defer fixture.deinit();
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var native = try store.NativeStore.atDirPath(a, io, record, fixture.dir, record.filename());
        defer native.deinit();
        try expectAbsent(native.raw());
        try expectSaved(native.raw(), "relative path record");
        try expectLoaded(native.raw(), "relative path record");
        const saved = try fixture.read(record.filename());
        defer a.free(saved);
        try t.expectEqualStrings("relative path record", saved);
        var parentless_root = try store.NativeStore.atDirPath(a, io, record, fixture.dir, "/");
        defer parentless_root.deinit();
        var rejected = try parentless_root.raw().save(a, "must not reach filesystem");
        defer rejected.deinit(a);
        try t.expectEqual(error.InvalidInput, rejected.failure.cause.?);
    }
    try fixture.noTemps();
}

test "Windows explicit profiles require Rust prefix plus root and retain raw bytes" {
    const invalid = [_][]const u8{
        "",
        "profile",
        "C:profile",
        "/profile",
        "\\profile",
        "/",
        "\\",
        "C:",
        ".",
        "..",
        "1:\\profile",
        "€:\\profile",
        "\\\\",
        "\\\\server",
        "\\\\server\\",
        "//server",
        "//server/",
        "\\\\server\\\\share",
        "\\\\?",
        "\\\\.",
    };
    for (invalid) |value| {
        try t.expectError(error.InvalidProfileDirectory, store.resolveProfileRoot(a, .windows, value));
        try t.expectError(error.InvalidProfileDirectory, store.NativeStore.fromProfile(a, io, .scenario, .windows, value));
        try t.expectError(error.InvalidProfileDirectory, store.NativeStore.fromProfile(a, io, .preferences, .windows, value));
    }
    const valid = [_][]const u8{
        "C:\\profile",
        "C:/profile",
        "c:\\",
        "Z:/",
        "\\\\server\\share\\profile",
        "//server/share/profile",
        "\\/server/share",
        "\\\\.\\COM42",
        "//./COM42",
        "\\\\.\\",
        "\\\\?\\C:\\profile",
        "\\\\?\\C:",
        "\\\\?\\C:relative",
        "\\\\?\\Volume{guid}\\",
        "\\\\?\\UNC\\server\\share",
        "\\\\?\\UNC\\",
        "\\\\?\\",
        "//?/C:/profile",
        "\\\\?\\UNC/server\\share",
    };
    for (valid) |value| {
        const resolved = (try store.resolveProfileRoot(a, .windows, value)).?;
        defer a.free(resolved);
        try t.expectEqualSlices(u8, value, resolved);
        // Pure constructors must not inspect or create these platform vectors.
        var native = try store.NativeStore.fromProfile(a, io, .scenario, .windows, value);
        defer native.deinit();
        try t.expect(native.legacy_path == null);
    }
    try t.expectEqual(@as(?[]u8, null), try store.resolveProfileRoot(a, .windows, null));
}

test "Windows APPDATA joining preserves Rust drive-relative and verbatim push semantics" {
    const cases = [_]struct { root: []const u8, namespace_prefix: []const u8 }{
        .{ .root = "C:", .namespace_prefix = "C:" },
        .{ .root = "c:", .namespace_prefix = "c:" },
        .{ .root = "C:relative", .namespace_prefix = "C:relative\\" },
        .{ .root = "", .namespace_prefix = "" },
        .{ .root = "relative", .namespace_prefix = "relative\\" },
        .{ .root = "\\profile", .namespace_prefix = "\\profile\\" },
        .{ .root = "/profile", .namespace_prefix = "/profile\\" },
        .{ .root = "C:\\data", .namespace_prefix = "C:\\data\\" },
        .{ .root = "C:/", .namespace_prefix = "C:/" },
        .{ .root = "1:", .namespace_prefix = "1:\\" },
        .{ .root = "€:", .namespace_prefix = "€:\\" },
        .{ .root = "\\\\server\\share", .namespace_prefix = "\\\\server\\share\\" },
        .{ .root = "//server/share", .namespace_prefix = "//server/share\\" },
        .{ .root = "\\\\.\\COM42", .namespace_prefix = "\\\\.\\COM42\\" },
        .{ .root = "//./COM42", .namespace_prefix = "//./COM42\\" },
        .{ .root = "\\\\?\\C:", .namespace_prefix = "\\\\?\\C:\\" },
        .{ .root = "\\\\?\\C:/a//b", .namespace_prefix = "\\\\?\\C:\\a//b\\" },
        .{ .root = "\\\\?\\C:\\one\\\\.\\..\\two\\", .namespace_prefix = "\\\\?\\C:\\one\\.\\..\\two\\" },
        .{ .root = "\\\\?\\prefix/", .namespace_prefix = "\\\\?\\prefix/\\" },
        .{ .root = "\\\\?\\UNC/server\\share", .namespace_prefix = "\\\\?\\UNC/server\\share\\" },
        .{ .root = "\\\\?\\", .namespace_prefix = "\\\\?\\\\" },
        .{ .root = "\\\\?\\UNC\\", .namespace_prefix = "\\\\?\\UNC\\\\" },
    };
    for (cases) |case| {
        for ([_]store.Record{ .scenario, .preferences }) |record| {
            for ([_]bool{ false, true }) |legacy| {
                const name = if (legacy) "Intergalactic Warfare" else "NYON";
                const expected = try std.fmt.allocPrint(a, "{s}{s}\\{s}", .{ case.namespace_prefix, name, record.filename() });
                defer a.free(expected);
                try pathEquals(record, .windows, .{ .appdata = case.root }, legacy, expected);
            }
        }
    }
}

test "Windows explicit profile joins normalize slot path without changing resolver bytes" {
    const profile = "\\\\?\\C:/one\\\\.\\..\\two\\";
    const resolved = (try store.resolveProfileRoot(a, .windows, profile)).?;
    defer a.free(resolved);
    try t.expectEqualStrings(profile, resolved);
    for ([_]store.Record{ .scenario, .preferences }) |record| {
        var native = try store.NativeStore.fromProfile(a, io, record, .windows, resolved);
        defer native.deinit();
        const expected = try std.fmt.allocPrint(a, "\\\\?\\C:\\one\\.\\..\\two\\{s}", .{record.filename()});
        defer a.free(expected);
        try t.expectEqualStrings(expected, native.path());
        try t.expect(native.legacy_path == null);
    }
}

fn windowsAllocationOwnership(allocator: std.mem.Allocator) !void {
    for ([_][]const u8{ "C:", "relative", "", "\\\\server\\share", "\\\\?\\C:/one\\\\.\\..\\two\\" }) |root| {
        var primary = try store.nativePath(allocator, .scenario, .windows, .{ .appdata = root }, false);
        defer primary.deinit(allocator);
        var legacy = try store.nativePath(allocator, .preferences, .windows, .{ .appdata = root }, true);
        defer legacy.deinit(allocator);
    }
    var profile = try store.NativeStore.fromProfile(allocator, io, .preferences, .windows, "\\\\?\\UNC\\server\\share\\");
    defer profile.deinit();
}

test "Windows prefix and joined path allocations clean up on every OOM" {
    try t.checkAllAllocationFailures(a, windowsAllocationOwnership, .{});
}
