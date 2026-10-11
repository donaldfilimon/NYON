//! Classic raw persistence. Stores own paths/slots/hooks; outcomes own payloads
//! and diagnostic messages through the allocator supplied to the operation.
//! OutOfMemory is a resource error, never a recoverable stored-data failure.
const std = @import("std");
const builtin = @import("builtin");
const preferences = @import("preferences.zig");
const scenario = @import("scenario.zig");
const Allocator = std.mem.Allocator;

pub const SCENARIO_FILE_NAME = "scenario-v1.json";
pub const PREFERENCES_FILE_NAME = "preferences-v1.json";
pub const Record = enum {
    scenario,
    preferences,
    pub fn filename(self: Record) []const u8 {
        return switch (self) {
            .scenario => SCENARIO_FILE_NAME,
            .preferences => PREFERENCES_FILE_NAME,
        };
    }
    pub fn maxBytes(self: Record) usize {
        return switch (self) {
            .scenario => scenario.MAX_JSON_BYTES,
            .preferences => preferences.MAX_JSON_BYTES,
        };
    }
    fn label(self: Record) []const u8 {
        return switch (self) {
            .scenario => "scenario",
            .preferences => "preference",
        };
    }
};

pub const FailureKind = enum { io, oversized_slot, before_persist, browser, unavailable_path };
pub const StoreFailure = struct {
    record: Record,
    kind: FailureKind,
    cause: ?anyerror = null,
    message: ?[]u8 = null,
    max_bytes: usize = 0,
    /// Cleanup failure is secondary; it never replaces the original cause.
    cleanup_error: ?anyerror = null,

    pub fn deinit(self: *StoreFailure, allocator: Allocator) void {
        if (self.message) |message| allocator.free(message);
        self.* = undefined;
    }
    pub fn clone(self: StoreFailure, allocator: Allocator) Allocator.Error!StoreFailure {
        var result = self;
        if (self.message) |message| result.message = try allocator.dupe(u8, message);
        return result;
    }
    pub fn messageAlloc(self: StoreFailure, allocator: Allocator) Allocator.Error![]u8 {
        const detail = self.message orelse if (self.cause) |cause| @errorName(cause) else "unknown storage failure";
        return switch (self.kind) {
            .io => std.fmt.allocPrint(allocator, "{s} storage failed: {s}", .{ self.record.label(), detail }),
            .oversized_slot => std.fmt.allocPrint(allocator, "{s} storage slot exceeds the {d}-byte limit", .{ self.record.label(), self.max_bytes }),
            .before_persist => std.fmt.allocPrint(allocator, "{s} storage failed before atomic persist: {s}", .{ self.record.label(), detail }),
            .browser => std.fmt.allocPrint(allocator, "browser {s} storage failed: {s}", .{ self.record.label(), detail }),
            .unavailable_path => std.fmt.allocPrint(allocator, "preference storage path is unavailable: {s}", .{detail}),
        };
    }
    fn ioFailure(record: Record, cause: anyerror) StoreFailure {
        return .{ .record = record, .kind = .io, .cause = cause };
    }
    fn oversized(record: Record) StoreFailure {
        return .{ .record = record, .kind = .oversized_slot, .max_bytes = record.maxBytes() };
    }
};

pub const LoadOutcome = union(enum) {
    absent,
    payload: []u8,
    failure: StoreFailure,
    pub fn deinit(self: *LoadOutcome, allocator: Allocator) void {
        switch (self.*) {
            .payload => |bytes| allocator.free(bytes),
            .failure => |*failure| failure.deinit(allocator),
            .absent => {},
        }
        self.* = undefined;
    }
};
pub const SaveOutcome = union(enum) {
    saved,
    failure: StoreFailure,
    pub fn deinit(self: *SaveOutcome, allocator: Allocator) void {
        switch (self.*) {
            .failure => |*failure| failure.deinit(allocator),
            .saved => {},
        }
        self.* = undefined;
    }
};

/// Borrowed interface; its concrete store must outlive calls. Payloads/messages
/// are allocated with the caller's allocator, independent of store ownership.
pub const RawStore = struct {
    record: Record,
    context: *anyopaque,
    record_fn: *const fn (*anyopaque) Record,
    load_fn: *const fn (*anyopaque, Allocator) Allocator.Error!LoadOutcome,
    save_fn: *const fn (*anyopaque, Allocator, []const u8) Allocator.Error!SaveOutcome,
    pub fn matchesRecord(self: RawStore, expected: Record) bool {
        return self.record == expected and self.record_fn(self.context) == expected;
    }
    pub fn load(self: RawStore, allocator: Allocator) Allocator.Error!LoadOutcome {
        if (!self.matchesRecord(self.record)) return .{ .failure = recordMismatch(self.record) };
        return self.load_fn(self.context, allocator);
    }
    pub fn save(self: RawStore, allocator: Allocator, payload: []const u8) Allocator.Error!SaveOutcome {
        if (!self.matchesRecord(self.record)) return .{ .failure = recordMismatch(self.record) };
        return self.save_fn(self.context, allocator, payload);
    }
};

fn recordMismatch(record: Record) StoreFailure {
    return .{ .record = record, .kind = .io, .cause = error.RecordMismatch };
}

/// Both calls occur at most once; only absence selects the legacy adapter.
pub fn loadPrimaryOrLegacy(allocator: Allocator, primary: RawStore, legacy: RawStore) Allocator.Error!LoadOutcome {
    if (!primary.matchesRecord(primary.record) or !legacy.matchesRecord(primary.record)) return .{ .failure = recordMismatch(primary.record) };
    const result = try primary.load(allocator);
    return switch (result) {
        .absent => legacy.load(allocator),
        else => result,
    };
}
pub const FallbackStore = struct {
    primary: RawStore,
    legacy: RawStore,
    pub fn raw(self: *FallbackStore) RawStore {
        return .{ .record = self.primary.record, .context = self, .record_fn = recordOf, .load_fn = load, .save_fn = save };
    }
    fn recordOf(context: *anyopaque) Record {
        const self: *FallbackStore = @ptrCast(@alignCast(context));
        return self.primary.record_fn(self.primary.context);
    }
    fn load(context: *anyopaque, allocator: Allocator) Allocator.Error!LoadOutcome {
        const self: *FallbackStore = @ptrCast(@alignCast(context));
        return loadPrimaryOrLegacy(allocator, self.primary, self.legacy);
    }
    fn save(context: *anyopaque, allocator: Allocator, payload: []const u8) Allocator.Error!SaveOutcome {
        const self: *FallbackStore = @ptrCast(@alignCast(context));
        if (!self.primary.matchesRecord(self.primary.record) or !self.legacy.matchesRecord(self.primary.record)) return .{ .failure = recordMismatch(self.primary.record) };
        return self.primary.save(allocator, payload);
    }
};

/// Retains a construction failure as an owned raw adapter. This lets runtime
/// policy keep preferences usable with defaults when path resolution failed.
pub const UnavailableStore = struct {
    allocator: Allocator,
    failure: StoreFailure,
    pub fn init(allocator: Allocator, failure: StoreFailure) Allocator.Error!UnavailableStore {
        return .{ .allocator = allocator, .failure = try failure.clone(allocator) };
    }
    pub fn deinit(self: *UnavailableStore) void {
        self.failure.deinit(self.allocator);
        self.* = undefined;
    }
    pub fn raw(self: *UnavailableStore) RawStore {
        return .{ .record = self.failure.record, .context = self, .record_fn = recordOf, .load_fn = load, .save_fn = save };
    }
    fn recordOf(context: *anyopaque) Record {
        const self: *UnavailableStore = @ptrCast(@alignCast(context));
        return self.failure.record;
    }
    fn load(context: *anyopaque, allocator: Allocator) Allocator.Error!LoadOutcome {
        const self: *UnavailableStore = @ptrCast(@alignCast(context));
        return .{ .failure = try self.failure.clone(allocator) };
    }
    fn save(context: *anyopaque, allocator: Allocator, _: []const u8) Allocator.Error!SaveOutcome {
        const self: *UnavailableStore = @ptrCast(@alignCast(context));
        return .{ .failure = try self.failure.clone(allocator) };
    }
};

pub const MemoryStore = struct {
    allocator: Allocator,
    record: Record,
    slot: ?[]u8 = null,
    load_failure: ?[]u8 = null,
    next_save_failure: ?[]u8 = null,
    load_calls: usize = 0,
    save_calls: usize = 0,
    pub fn init(allocator: Allocator, record: Record) MemoryStore {
        return .{ .allocator = allocator, .record = record };
    }
    pub fn withSlot(allocator: Allocator, record: Record, payload: []const u8) Allocator.Error!MemoryStore {
        return .{ .allocator = allocator, .record = record, .slot = try allocator.dupe(u8, payload) };
    }
    pub fn deinit(self: *MemoryStore) void {
        if (self.slot) |bytes| self.allocator.free(bytes);
        if (self.load_failure) |bytes| self.allocator.free(bytes);
        if (self.next_save_failure) |bytes| self.allocator.free(bytes);
        self.* = undefined;
    }
    /// Persistent denial, matching the Rust preference memory adapter.
    pub fn denyLoad(self: *MemoryStore, message: []const u8) Allocator.Error!void {
        const owned = try self.allocator.dupe(u8, message);
        if (self.load_failure) |old| self.allocator.free(old);
        self.load_failure = owned;
    }
    pub fn failNextSave(self: *MemoryStore, message: []const u8) Allocator.Error!void {
        const owned = try self.allocator.dupe(u8, message);
        if (self.next_save_failure) |old| self.allocator.free(old);
        self.next_save_failure = owned;
    }
    pub fn raw(self: *MemoryStore) RawStore {
        return .{ .record = self.record, .context = self, .record_fn = recordOf, .load_fn = load, .save_fn = save };
    }
    fn recordOf(context: *anyopaque) Record {
        const self: *MemoryStore = @ptrCast(@alignCast(context));
        return self.record;
    }
    fn load(context: *anyopaque, allocator: Allocator) Allocator.Error!LoadOutcome {
        const self: *MemoryStore = @ptrCast(@alignCast(context));
        self.load_calls += 1;
        if (self.load_failure) |message| return .{ .failure = .{ .record = self.record, .kind = .browser, .message = try allocator.dupe(u8, message) } };
        if (self.slot) |slot| {
            if (self.record == .preferences and slot.len > self.record.maxBytes()) return .{ .failure = .oversized(self.record) };
            return .{ .payload = try allocator.dupe(u8, slot) };
        }
        return .absent;
    }
    fn save(context: *anyopaque, allocator: Allocator, payload: []const u8) Allocator.Error!SaveOutcome {
        const self: *MemoryStore = @ptrCast(@alignCast(context));
        self.save_calls += 1;
        if (self.record == .preferences and payload.len > self.record.maxBytes()) return .{ .failure = .oversized(self.record) };
        if (self.next_save_failure) |message| {
            const result: StoreFailure = .{ .record = self.record, .kind = .before_persist, .message = try allocator.dupe(u8, message) };
            self.allocator.free(message);
            self.next_save_failure = null;
            return .{ .failure = result };
        }
        const replacement = try self.allocator.dupe(u8, payload);
        if (self.slot) |old| self.allocator.free(old);
        self.slot = replacement;
        return .saved;
    }
};

pub const NativePlatform = enum { macos, windows, linux };
/// Borrowed raw OS path bytes; no environment is implicitly consulted.
pub const NativePathEnvironment = struct { home: ?[]const u8 = null, appdata: ?[]const u8 = null, xdg_data_home: ?[]const u8 = null };
pub const PathOutcome = union(enum) {
    path: []u8,
    failure: StoreFailure,
    pub fn deinit(self: *PathOutcome, allocator: Allocator) void {
        switch (self.*) {
            .path => |bytes| allocator.free(bytes),
            .failure => |*failure| failure.deinit(allocator),
        }
        self.* = undefined;
    }
};

// Rust std::sys::path::windows_prefix::parse_prefix and Prefix::{len,
// has_implicit_root,is_drive}. Zig Win32 path classification is intentionally
// broader (root-only paths, incomplete UNC, and non-ASCII drive letters).
const WindowsPrefixKind = enum { disk, unc, device, verbatim, verbatim_disk, verbatim_unc };
const WindowsPrefix = struct {
    kind: WindowsPrefixKind,
    len: usize,
    fn isVerbatim(self: WindowsPrefix) bool {
        return switch (self.kind) {
            .verbatim, .verbatim_disk, .verbatim_unc => true,
            else => false,
        };
    }
};
fn windowsSeparator(byte: u8) bool {
    return byte == '\\' or byte == '/';
}
fn windowsComponentLength(bytes: []const u8, verbatim: bool) usize {
    for (bytes, 0..) |byte, index| {
        if (byte == '\\' or (!verbatim and byte == '/')) return index;
    }
    return bytes.len;
}
fn windowsDrive(bytes: []const u8) bool {
    return bytes.len >= 2 and std.ascii.isAlphabetic(bytes[0]) and bytes[1] == ':';
}
fn windowsPrefix(bytes: []const u8) ?WindowsPrefix {
    if (bytes.len >= 2 and windowsSeparator(bytes[0]) and windowsSeparator(bytes[1])) {
        // Rust normalizes only the prefix lookahead's slashes. Verbatim's
        // initial four bytes must still contain no forward slash.
        if (std.mem.startsWith(u8, bytes, "\\\\?\\")) {
            const body = bytes[4..];
            if (body.len >= 4 and std.mem.eql(u8, body[0..3], "UNC") and windowsSeparator(body[3])) {
                const server_and_share = body[4..];
                const server_len = windowsComponentLength(server_and_share, true);
                const share_start = server_len + @intFromBool(server_len < server_and_share.len);
                const share_len = windowsComponentLength(server_and_share[share_start..], true);
                return .{ .kind = .verbatim_unc, .len = 8 + server_len + (if (share_len > 0) 1 + share_len else @as(usize, 0)) };
            }
            if (windowsDrive(body) and (body.len == 2 or windowsSeparator(body[2]))) return .{ .kind = .verbatim_disk, .len = 6 };
            return .{ .kind = .verbatim, .len = 4 + windowsComponentLength(body, true) };
        }
        if (bytes.len >= 4 and bytes[2] == '.' and windowsSeparator(bytes[3])) return .{ .kind = .device, .len = 4 + windowsComponentLength(bytes[4..], false) };
        const body = bytes[2..];
        const server_len = windowsComponentLength(body, false);
        const share_start = server_len + @intFromBool(server_len < body.len);
        const share_len = windowsComponentLength(body[share_start..], false);
        if (server_len == 0 or share_len == 0) return null;
        return .{ .kind = .unc, .len = 2 + server_len + 1 + share_len };
    }
    return if (windowsDrive(bytes)) .{ .kind = .disk, .len = 2 } else null;
}
fn windowsAbsolute(bytes: []const u8) bool {
    const prefix = windowsPrefix(bytes) orelse return false;
    // Every prefix except ordinary Disk has an implicit root in Rust.
    return prefix.kind != .disk or (bytes.len > prefix.len and windowsSeparator(bytes[prefix.len]));
}

/// PathBuf::_push for this module's fixed, nonempty relative child components.
/// For verbatim roots Rust reconstructs existing Components before appending:
/// root separators become backslashes, empty components collapse, and existing
/// verbatim . and .. components remain. New child names contain neither.
fn pushWindowsChild(allocator: Allocator, root: []const u8, child: []const u8) Allocator.Error![]u8 {
    var result: std.Io.Writer.Allocating = .init(allocator);
    defer result.deinit();
    const prefix = windowsPrefix(root);
    var need_separator = root.len != 0 and !windowsSeparator(root[root.len - 1]);
    if (prefix) |parsed| {
        if (parsed.isVerbatim()) {
            result.writer.writeAll(root[0..parsed.len]) catch return error.OutOfMemory;
            var body = root[parsed.len..];
            // has_physical_root uses both separator bytes even for verbatim.
            need_separator = true; // VerbatimDisk is not Prefix::Disk.
            if (body.len != 0 and windowsSeparator(body[0])) {
                result.writer.writeByte('\\') catch return error.OutOfMemory;
                body = body[1..];
                need_separator = false;
            }
            var components = std.mem.splitScalar(u8, body, '\\');
            while (components.next()) |component| {
                if (component.len == 0) continue;
                if (need_separator) result.writer.writeByte('\\') catch return error.OutOfMemory;
                result.writer.writeAll(component) catch return error.OutOfMemory;
                need_separator = true;
            }
        } else {
            result.writer.writeAll(root) catch return error.OutOfMemory;
            if (parsed.kind == .disk and parsed.len == root.len) need_separator = false;
        }
    } else result.writer.writeAll(root) catch return error.OutOfMemory;
    if (need_separator) result.writer.writeByte('\\') catch return error.OutOfMemory;
    result.writer.writeAll(child) catch return error.OutOfMemory;
    return result.toOwnedSlice();
}
fn joinPlatform(allocator: Allocator, platform: NativePlatform, parts: []const []const u8) Allocator.Error![]u8 {
    if (platform == .windows) {
        var joined = try allocator.dupe(u8, parts[0]);
        errdefer allocator.free(joined);
        for (parts[1..]) |part| {
            const replacement = try pushWindowsChild(allocator, joined, part);
            allocator.free(joined);
            joined = replacement;
        }
        return joined;
    }
    var result: std.Io.Writer.Allocating = .init(allocator);
    defer result.deinit();
    for (parts) |part| {
        if (part.len == 0) continue;
        if (result.written().len != 0 and result.written()[result.written().len - 1] != '/') result.writer.writeByte('/') catch return error.OutOfMemory;
        result.writer.writeAll(part) catch return error.OutOfMemory;
    }
    return result.toOwnedSlice();
}
pub fn nativePath(allocator: Allocator, record: Record, platform: NativePlatform, environment: NativePathEnvironment, legacy: bool) Allocator.Error!PathOutcome {
    const native_name = if (legacy) "Intergalactic Warfare" else "NYON";
    const linux_name = if (legacy) "intergalactic-warfare" else "nyon";
    const path = switch (platform) {
        .macos => if (environment.home) |home| try joinPlatform(allocator, platform, &.{ home, "Library/Application Support", native_name, record.filename() }) else null,
        .windows => if (environment.appdata) |appdata| try joinPlatform(allocator, platform, &.{ appdata, native_name, record.filename() }) else null,
        .linux => blk: {
            if (environment.xdg_data_home) |xdg| {
                if (std.fs.path.isAbsolutePosix(xdg)) break :blk try joinPlatform(allocator, platform, &.{ xdg, linux_name, record.filename() });
            }
            if (environment.home) |home| {
                if (std.fs.path.isAbsolutePosix(home)) break :blk try joinPlatform(allocator, platform, &.{ home, ".local/share", linux_name, record.filename() });
            }
            break :blk null;
        },
    };
    if (path) |owned| return .{ .path = owned };
    return .{ .failure = .{
        .record = record,
        .kind = if (record == .preferences) .unavailable_path else .io,
        .cause = error.FileNotFound,
        .message = try allocator.dupe(u8, if (record == .preferences) "scenario storage failed: required application-data environment path is unavailable" else "required application-data environment path is unavailable"),
    } };
}

/// Captures an optional injected profile value once. Caller frees successful
/// nonnull bytes. Unix paths remain opaque bytes, including non-UTF8 names.
pub fn resolveProfileRoot(allocator: Allocator, platform: NativePlatform, value: ?[]const u8) (Allocator.Error || error{InvalidProfileDirectory})!?[]u8 {
    const root = value orelse return null;
    const absolute = if (platform == .windows) windowsAbsolute(root) else std.fs.path.isAbsolutePosix(root);
    if (root.len == 0 or !absolute) return error.InvalidProfileDirectory;
    return try allocator.dupe(u8, root);
}

pub const FaultStage = enum { create, write, sync, persist };
const Fault = struct { stage: FaultStage, message: []u8 };
/// Optional per-save callback, called once after content sync and before rename.
/// A returned failure must own its message with the passed allocator.
pub const BeforePersistHook = struct {
    context: *anyopaque,
    call: *const fn (*anyopaque, Allocator) Allocator.Error!?StoreFailure,
};
pub const NativeStore = struct {
    allocator: Allocator,
    io: std.Io,
    /// Borrowed directory for relative paths; must outlive this store. Absolute
    /// paths retain normal OS semantics. deinit never closes this directory.
    base_dir: std.Io.Dir = .cwd(),
    record: Record,
    primary_path: []u8,
    legacy_path: ?[]u8 = null,
    next_fault: ?Fault = null,
    /// Last cleanup error, including failures during OutOfMemory handling.
    /// Reset on each admitted save; size rejection does not consume state.
    last_cleanup_error: ?anyerror = null,

    pub fn atPath(allocator: Allocator, io: std.Io, record: Record, path_bytes: []const u8) Allocator.Error!NativeStore {
        return .{ .allocator = allocator, .io = io, .record = record, .primary_path = try allocator.dupe(u8, path_bytes) };
    }
    /// Relative paths resolve under this borrowed directory, enabling isolated
    /// qualification without changing the process working directory.
    pub fn atDirPath(allocator: Allocator, io: std.Io, record: Record, base_dir: std.Io.Dir, path_bytes: []const u8) Allocator.Error!NativeStore {
        var result = try atPath(allocator, io, record, path_bytes);
        result.base_dir = base_dir;
        return result;
    }
    pub fn atPaths(allocator: Allocator, io: std.Io, record: Record, primary: []const u8, legacy: []const u8) Allocator.Error!NativeStore {
        var store = try atPath(allocator, io, record, primary);
        errdefer store.deinit();
        store.legacy_path = try allocator.dupe(u8, legacy);
        return store;
    }
    pub fn fromProfile(allocator: Allocator, io: std.Io, record: Record, platform: NativePlatform, profile: []const u8) (Allocator.Error || error{InvalidProfileDirectory})!NativeStore {
        const root = (try resolveProfileRoot(allocator, platform, profile)).?;
        defer allocator.free(root);
        const joined_path = try joinPlatform(allocator, platform, &.{ root, record.filename() });
        return .{ .allocator = allocator, .io = io, .record = record, .primary_path = joined_path };
    }
    pub fn fromEnvironment(allocator: Allocator, io: std.Io, record: Record, platform: NativePlatform, environment: NativePathEnvironment) Allocator.Error!NativeStoreOutcome {
        const primary = try nativePath(allocator, record, platform, environment, false);
        switch (primary) {
            .failure => |failure| return .{ .failure = failure },
            .path => |primary_path| {
                errdefer allocator.free(primary_path);
                const legacy = try nativePath(allocator, record, platform, environment, true);
                return switch (legacy) {
                    .failure => |failure| blk: {
                        allocator.free(primary_path);
                        break :blk .{ .failure = failure };
                    },
                    .path => |legacy_path| .{ .store = .{ .allocator = allocator, .io = io, .record = record, .primary_path = primary_path, .legacy_path = legacy_path } },
                };
            },
        }
    }
    pub fn deinit(self: *NativeStore) void {
        self.allocator.free(self.primary_path);
        if (self.legacy_path) |bytes| self.allocator.free(bytes);
        if (self.next_fault) |fault| self.allocator.free(fault.message);
        self.* = undefined;
    }
    pub fn path(self: *const NativeStore) []const u8 {
        return self.primary_path;
    }
    pub fn failNext(self: *NativeStore, stage: FaultStage, message: []const u8) Allocator.Error!void {
        const owned = try self.allocator.dupe(u8, message);
        if (self.next_fault) |old| self.allocator.free(old.message);
        self.next_fault = .{ .stage = stage, .message = owned };
    }
    fn consumeFault(self: *NativeStore, allocator: Allocator, stage: FaultStage) Allocator.Error!?StoreFailure {
        const fault = self.next_fault orelse return null;
        if (fault.stage != stage) return null;
        const result: StoreFailure = .{ .record = self.record, .kind = .io, .cause = error.InjectedFailure, .message = try allocator.dupe(u8, fault.message) };
        self.allocator.free(fault.message);
        self.next_fault = null;
        return result;
    }
    pub fn raw(self: *NativeStore) RawStore {
        return .{ .record = self.record, .context = self, .record_fn = recordOf, .load_fn = load, .save_fn = save };
    }
    fn recordOf(context: *anyopaque) Record {
        const self: *NativeStore = @ptrCast(@alignCast(context));
        return self.record;
    }
    fn loadPath(self: *NativeStore, allocator: Allocator, path_bytes: []const u8) Allocator.Error!LoadOutcome {
        const file = self.base_dir.openFile(self.io, path_bytes, .{}) catch |cause| {
            if (cause == error.FileNotFound) return .absent;
            return .{ .failure = .ioFailure(self.record, cause) };
        };
        defer file.close(self.io);
        const buffer = try allocator.alloc(u8, self.record.maxBytes() + 1);
        defer allocator.free(buffer);
        const len = file.readPositionalAll(self.io, buffer, 0) catch |cause| return .{ .failure = .ioFailure(self.record, cause) };
        if (len > self.record.maxBytes()) return .{ .failure = .oversized(self.record) };
        if (!std.unicode.utf8ValidateSlice(buffer[0..len])) return .{ .failure = .ioFailure(self.record, error.InvalidUtf8) };
        return .{ .payload = try allocator.dupe(u8, buffer[0..len]) };
    }
    fn load(context: *anyopaque, allocator: Allocator) Allocator.Error!LoadOutcome {
        const self: *NativeStore = @ptrCast(@alignCast(context));
        const primary = try self.loadPath(allocator, self.primary_path);
        return switch (primary) {
            .absent => if (self.legacy_path) |legacy| self.loadPath(allocator, legacy) else .absent,
            else => primary,
        };
    }
    fn save(context: *anyopaque, allocator: Allocator, payload: []const u8) Allocator.Error!SaveOutcome {
        const self: *NativeStore = @ptrCast(@alignCast(context));
        return self.saveWithBeforePersist(allocator, payload, null);
    }
    pub fn saveWithBeforePersist(self: *NativeStore, allocator: Allocator, payload: []const u8, hook: ?BeforePersistHook) Allocator.Error!SaveOutcome {
        if (self.record == .preferences and payload.len > self.record.maxBytes()) return .{ .failure = .oversized(self.record) };
        self.last_cleanup_error = null;
        // Rust Path::parent returns an empty parent for a single relative
        // component; Zig dirname returns null for that same input.
        const parent_path: ?[]const u8 = std.fs.path.dirname(self.primary_path) orelse
            if (self.primary_path.len != 0 and !std.fs.path.isAbsolute(self.primary_path) and std.fs.path.basename(self.primary_path).len != 0) @as([]const u8, "") else null;
        const parent = parent_path orelse return .{ .failure = .{
            .record = self.record,
            .kind = .io,
            .cause = error.InvalidInput,
            .message = try allocator.dupe(u8, if (self.record == .scenario) "scenario slot has no parent directory" else "preference slot has no parent directory"),
        } };
        const directory = self.base_dir;
        if (parent.len != 0) directory.createDirPath(self.io, parent) catch |cause| return .{ .failure = .ioFailure(self.record, cause) };
        if (try self.consumeFault(allocator, .create)) |failure| return .{ .failure = failure };
        // Exclusive creation prevents concurrent writers from sharing a temp.
        var temporary_path: ?[]u8 = null;
        defer if (temporary_path) |bytes| allocator.free(bytes);
        var temporary: std.Io.File = undefined;
        var attempts: usize = 0;
        while (attempts < 32) : (attempts += 1) {
            var random: [16]u8 = undefined;
            self.io.random(&random);
            var name_buffer: [64]u8 = undefined;
            const name = std.fmt.bufPrint(&name_buffer, ".nyon-classic-{s}.tmp", .{std.fmt.bytesToHex(random, .lower)}) catch unreachable;
            const candidate = try std.fs.path.join(allocator, &.{ parent, name });
            temporary = directory.createFile(self.io, candidate, .{
                .exclusive = true,
                .permissions = if (builtin.os.tag == .windows) .default_file else .fromMode(0o600),
            }) catch |cause| {
                allocator.free(candidate);
                if (cause == error.PathAlreadyExists) continue;
                return .{ .failure = .ioFailure(self.record, cause) };
            };
            temporary_path = candidate;
            break;
        }
        const owned_path = temporary_path orelse return .{ .failure = .ioFailure(self.record, error.PathAlreadyExists) };
        var result = self.writeAndSync(allocator, temporary, payload, hook) catch |cause| {
            temporary.close(self.io);
            self.cleanupTemporary(owned_path);
            return cause;
        };
        // Close even on failure; on Windows a closed temp can be replaced too.
        temporary.close(self.io);
        if (result == .saved) {
            if (try self.consumePersistFaultOrCleanup(allocator, owned_path)) |failure| return .{ .failure = failure };
            directory.rename(owned_path, directory, self.primary_path, self.io) catch |cause| {
                var failure = StoreFailure.ioFailure(self.record, cause);
                self.cleanupTemporary(owned_path);
                failure.cleanup_error = self.last_cleanup_error;
                return .{ .failure = failure };
            };
        } else {
            self.cleanupTemporary(owned_path);
            result.failure.cleanup_error = self.last_cleanup_error;
        }
        return result;
    }
    fn cleanupTemporary(self: *NativeStore, temporary_path: []const u8) void {
        self.base_dir.deleteFile(self.io, temporary_path) catch |cause| {
            self.last_cleanup_error = cause;
        };
    }
    fn consumePersistFaultOrCleanup(self: *NativeStore, allocator: Allocator, temporary_path: []const u8) Allocator.Error!?StoreFailure {
        const fault = self.consumeFault(allocator, .persist) catch |cause| {
            self.cleanupTemporary(temporary_path);
            return cause;
        };
        if (fault) |value| {
            var failure = value;
            self.cleanupTemporary(temporary_path);
            failure.cleanup_error = self.last_cleanup_error;
            return failure;
        }
        return null;
    }
    fn writeAndSync(self: *NativeStore, allocator: Allocator, file: std.Io.File, payload: []const u8, hook: ?BeforePersistHook) Allocator.Error!SaveOutcome {
        if (try self.consumeFault(allocator, .write)) |failure| return .{ .failure = failure };
        // PositionalAll writes directly (no user-space buffer to flush).
        file.writePositionalAll(self.io, payload, 0) catch |cause| return .{ .failure = .ioFailure(self.record, cause) };
        if (try self.consumeFault(allocator, .sync)) |failure| return .{ .failure = failure };
        file.sync(self.io) catch |cause| return .{ .failure = .ioFailure(self.record, cause) };
        if (hook) |callback| {
            if (try callback.call(callback.context, allocator)) |failure| return .{ .failure = failure };
        }
        return .saved;
    }
};
pub const NativeStoreOutcome = union(enum) {
    store: NativeStore,
    failure: StoreFailure,
    pub fn deinit(self: *NativeStoreOutcome, allocator: Allocator) void {
        switch (self.*) {
            .store => |*store| store.deinit(),
            .failure => |*failure| failure.deinit(allocator),
        }
        self.* = undefined;
    }
};

pub const PreferencesFailure = union(enum) {
    store: StoreFailure,
    codec: preferences.CodecFailure,
    pub fn deinit(self: *PreferencesFailure, allocator: Allocator) void {
        switch (self.*) {
            .store => |*failure| failure.deinit(allocator),
            .codec => {},
        }
        self.* = undefined;
    }
    pub fn messageAlloc(self: PreferencesFailure, allocator: Allocator) Allocator.Error![]u8 {
        return switch (self) {
            .store => |failure| failure.messageAlloc(allocator),
            .codec => |failure| switch (failure) {
                .too_large => allocator.dupe(u8, "preferences JSON exceeds the 16384-byte limit"),
                .json => |cause| std.fmt.allocPrint(allocator, "invalid preferences JSON: {s}", .{@errorName(cause)}),
                .version => allocator.dupe(u8, "unsupported preferences format version"),
                .ui_scale => |scale| std.fmt.allocPrint(allocator, "unsupported UI scale percentage {d}", .{scale}),
            },
        };
    }
};
pub const PreferencesOutcome = struct {
    preferences: preferences.UserPreferencesV1 = .{},
    failure: ?PreferencesFailure = null,
    pub fn deinit(self: *PreferencesOutcome, allocator: Allocator) void {
        if (self.failure) |*failure| failure.deinit(allocator);
        self.* = undefined;
    }
    /// Caller owns the optional message; this adds no second error prefix.
    pub fn recoverableMessage(self: PreferencesOutcome, allocator: Allocator) Allocator.Error!?[]u8 {
        return if (self.failure) |failure| try failure.messageAlloc(allocator) else null;
    }
};
pub fn loadOrDefault(allocator: Allocator, store: RawStore) Allocator.Error!PreferencesOutcome {
    if (!store.matchesRecord(.preferences)) return .{ .failure = .{ .store = recordMismatch(.preferences) } };
    const loaded = try store.load(allocator);
    switch (loaded) {
        .absent => return .{},
        .failure => |failure| return .{ .failure = .{ .store = failure } },
        .payload => |payload| {
            defer allocator.free(payload);
            return switch (try preferences.decodeDetailed(allocator, payload)) {
                .preferences => |value| .{ .preferences = value },
                .failure => |failure| .{ .failure = .{ .codec = failure } },
            };
        },
    }
}
pub fn saveOrDefault(allocator: Allocator, store: RawStore, requested: preferences.UserPreferencesV1) Allocator.Error!PreferencesOutcome {
    if (!store.matchesRecord(.preferences)) return .{ .failure = .{ .store = recordMismatch(.preferences) } };
    const payload = preferences.encode(allocator, requested) catch |cause| switch (cause) {
        error.OutOfMemory => return error.OutOfMemory,
        error.TooLarge => return .{ .failure = .{ .codec = .too_large } },
        error.Json => return .{ .failure = .{ .codec = .{ .json = cause } } },
        error.Version => return .{ .failure = .{ .codec = .version } },
        error.UiScale => return .{ .failure = .{ .codec = .{ .ui_scale = requested.ui_scale.percent() } } },
    };
    defer allocator.free(payload);
    return switch (try store.save(allocator, payload)) {
        .saved => .{ .preferences = requested },
        .failure => |failure| .{ .failure = .{ .store = failure } },
    };
}
