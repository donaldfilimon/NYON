//! Move-only detached Classic editor transaction owner. Borrow core and store per call.
const std = @import("std");
const app = @import("classic_app.zig");
const editor = @import("classic_editor.zig");
const scenario = @import("scenario.zig");
const store = @import("classic_store.zig");
pub const Outcome = enum { unchanged, opened, modal_opened, absent, loaded, saved, closed, applied, replaced, failed };
pub const Failure = union(enum) { invalid_text, confirmation_active, no_confirmation, stale_confirmation, codec: scenario.CodecFailure, validation: scenario.ValidationReport, admission: scenario.AdmissionError };
pub const StoreStatusFailure = union(enum) { codec: scenario.CodecFailure, storage: store.StoreFailure };
pub const Status = union(enum) { none, no_saved_scenario, loaded, saved, failure: StoreStatusFailure };
pub const Effects = struct { reset_camera: bool = false, clear_focus: bool = false, restart_advisory: bool = false };
pub const Transactions = struct {
    allocator: std.mem.Allocator,
    session: ?editor.EditorState = null,
    status: Status = .none,
    /// Latest action-only failure is independent of the prior store message.
    action_failure: ?Failure = null,
    effects: Effects = .{},
    pub fn init(allocator: std.mem.Allocator) Transactions {
        return .{ .allocator = allocator };
    }
    pub fn clearStatus(self: *Transactions) void {
        switch (self.status) {
            .failure => |*f| switch (f.*) {
                .storage => |*detail| detail.deinit(self.allocator),
                else => {},
            },
            else => {},
        }
        self.status = .none;
    }
    pub fn deinit(self: *Transactions) void {
        if (self.session) |*s| s.deinit();
        self.clearStatus();
        self.* = undefined;
    }
    fn checkMode(self: *const Transactions, core: *const app.AppCore) error{ModeSessionMismatch}!void {
        if ((self.session != null) != (core.mode() == .editing_scenario)) return error.ModeSessionMismatch;
    }
    fn close(self: *Transactions, core: *app.AppCore) Outcome {
        self.session.?.deinit();
        self.session = null;
        self.clearStatus();
        self.action_failure = null;
        core.setMode(.playing);
        _ = core.recover();
        self.effects = .{ .clear_focus = true };
        return .closed;
    }
    pub fn open(self: *Transactions, core: *app.AppCore) !Outcome {
        try self.checkMode(core);
        if (self.session != null) return .unchanged;
        var draft = try core.activeScenario().toDraft(self.allocator);
        defer draft.deinit();
        const staged = try editor.EditorState.init(self.allocator, &draft);
        self.session = staged;
        self.clearStatus();
        self.action_failure = null;
        self.effects = .{ .clear_focus = true };
        core.setMode(.editing_scenario);
        return .opened;
    }
    fn failed(self: *Transactions, failure: Failure) Outcome {
        self.action_failure = failure;
        return .failed;
    }
    fn storeFailed(self: *Transactions, failure: StoreStatusFailure) Outcome {
        self.clearStatus();
        self.status = .{ .failure = failure };
        self.action_failure = null;
        return .failed;
    }
    fn message(self: *Transactions, status: Status) void {
        self.clearStatus();
        self.status = status;
        self.action_failure = null;
    }
    fn checkStore(self: *Transactions, raw: store.RawStore) bool {
        if (raw.matchesRecord(.scenario)) return true;
        _ = self.storeFailed(.{ .storage = .{ .record = .scenario, .kind = .io, .cause = error.RecordMismatch } });
        return false;
    }
    fn load(self: *Transactions, raw: store.RawStore) !Outcome {
        if (!self.checkStore(raw)) return .failed;
        var loaded = try raw.load(self.allocator);
        defer loaded.deinit(self.allocator);
        switch (loaded) {
            .absent => {
                self.message(.no_saved_scenario);
                return .absent;
            },
            .failure => |detail| {
                loaded = .absent;
                return self.storeFailed(.{ .storage = detail });
            },
            .payload => |bytes| {
                var decoded = try scenario.decodeDraftDetailed(self.allocator, bytes);
                switch (decoded) {
                    .failure => |detail| return self.storeFailed(.{ .codec = detail }),
                    .draft => |*draft| {
                        defer draft.deinit();
                        const report = draft.validationReport();
                        if (!report.isValid()) return self.storeFailed(.{ .codec = .{ .validation = report } });
                        try self.session.?.replaceCurrent(draft);
                        self.message(.loaded);
                        return .loaded;
                    },
                }
            },
        }
    }
    fn saveBytes(self: *Transactions, raw: store.RawStore, bytes: []const u8) !Outcome {
        if (!self.checkStore(raw)) return .failed;
        var result = try raw.save(self.allocator, bytes);
        defer result.deinit(self.allocator);
        switch (result) {
            .saved => {
                self.message(.saved);
                return .saved;
            },
            .failure => |detail| {
                result = .saved;
                return self.storeFailed(.{ .storage = detail });
            },
        }
    }
    fn requestSave(self: *Transactions, raw: store.RawStore) !Outcome {
        const s = &self.session.?;
        if (s.hasParseErrors()) return self.failed(.invalid_text);
        const report = s.validationReport();
        if (!report.isValid()) return self.failed(.{ .codec = .{ .validation = report } });
        const candidate = s.candidate() catch unreachable;
        const bytes = scenario.encode(s.modal.allocator, &candidate) catch |cause| switch (cause) {
            error.OutOfMemory => return error.OutOfMemory,
            error.TooLarge => return self.failed(.{ .codec = .too_large }),
            else => unreachable,
        };
        var adopted = false;
        defer if (!adopted) s.modal.allocator.free(bytes);
        if (!self.checkStore(raw)) return .failed;
        var occupied = try raw.load(self.allocator);
        defer occupied.deinit(self.allocator);
        switch (occupied) {
            .absent => return self.saveBytes(raw, bytes),
            .failure => |detail| {
                occupied = .absent;
                return self.storeFailed(.{ .storage = detail });
            },
            .payload => {
                s.modal.begin(.overwrite_save, bytes, null);
                adopted = true;
                return .modal_opened;
            },
        }
    }
    fn confirm(self: *Transactions, core: *app.AppCore, raw: store.RawStore) !Outcome {
        var taken = self.session.?.modal;
        self.session.?.modal = .{ .allocator = taken.allocator };
        defer taken.deinit();
        const kind = taken.kind orelse return self.failed(.no_confirmation);
        return switch (kind) {
            .load_dirty => try self.load(raw),
            .overwrite_save => if (taken.save_bytes) |bytes| try self.saveBytes(raw, bytes) else self.failed(.no_confirmation),
            .apply_and_restart => {
                const captured = taken.apply_candidate orelse return self.failed(.stale_confirmation);
                const fresh = self.session.?.candidate() catch |cause| return self.failed(if (cause == error.InvalidText) .invalid_text else .{ .validation = self.session.?.validationReport() });
                if (!meaningfulEqual(&captured, &fresh)) return self.failed(.stale_confirmation);
                core.replaceScenario(&captured) catch |cause| return self.failed(.{ .admission = cause });
                self.session.?.deinit();
                self.session = null;
                self.clearStatus();
                self.action_failure = null;
                self.effects = .{ .reset_camera = true, .clear_focus = true, .restart_advisory = true };
                return .applied;
            },
            .cancel_dirty => self.close(core),
            .revert, .factory_defaults, .regenerate_from_seed => {
                var replacement = switch (kind) {
                    .revert => try self.session.?.copyOriginal(self.allocator),
                    .factory_defaults => try scenario.ScenarioDraft.factoryDefault(self.allocator),
                    .regenerate_from_seed => try scenario.ScenarioDraft.generated(self.allocator, self.session.?.current().seed, self.session.?.current().rules),
                    else => unreachable,
                };
                defer replacement.deinit();
                try self.session.?.replaceCurrent(&replacement);
                return .replaced;
            },
        };
    }
    pub fn dispatch(self: *Transactions, core: *app.AppCore, raw: store.RawStore, action: editor.Action) !Outcome {
        const outcome = try self.dispatchInner(core, raw, action);
        if (outcome != .failed) self.action_failure = null;
        return outcome;
    }
    fn dispatchInner(self: *Transactions, core: *app.AppCore, raw: store.RawStore, action: editor.Action) !Outcome {
        try self.checkMode(core);
        if (self.session == null) return .unchanged;
        const s = &self.session.?;
        if (action == .cancel) {
            if (!s.isDirty()) return self.close(core);
            s.modal.begin(.cancel_dirty, null, null);
            return .modal_opened;
        }
        if (action == .confirm) return self.confirm(core, raw);
        if (action == .dismiss_confirmation) {
            s.modal.dismiss();
            return .unchanged;
        }
        if (s.modal.kind != null) return switch (action) {
            .load, .save, .apply_and_restart => self.failed(.confirmation_active),
            else => .unchanged,
        };
        switch (action) {
            .select_section => |section| {
                s.selectSection(section);
                return .unchanged;
            },
            .select_world => |world| {
                s.selectWorld(world);
                return .unchanged;
            },
            .revert => s.modal.begin(.revert, null, null),
            .factory_defaults => s.modal.begin(.factory_defaults, null, null),
            .regenerate_from_seed => s.modal.begin(.regenerate_from_seed, null, null),
            else => {},
        }
        if (s.modal.kind != null) return .modal_opened;
        if (action == .load) {
            if (!s.isDirty()) return self.load(raw);
            s.modal.begin(.load_dirty, null, null);
            return .modal_opened;
        }
        if (action == .save) return self.requestSave(raw);
        if (action == .apply_and_restart) {
            const candidate = s.candidate() catch |cause| return self.failed(if (cause == error.InvalidText) .invalid_text else .{ .validation = s.validationReport() });
            s.modal.begin(.apply_and_restart, null, candidate);
            return .modal_opened;
        }
        return .unchanged;
    }
};

// ScenarioV1 is mutable inline storage. Compare live meaning, never padding or
// unused capacities. Check *both* values' lengths before taking any slice.
fn textEqual(x: anytype, y: @TypeOf(x)) bool {
    if (x.len > x.bytes.len or y.len > y.bytes.len or x.len != y.len) return false;
    return std.mem.eql(u8, x.bytes[0..x.len], y.bytes[0..y.len]);
}
fn meaningfulEqual(x: *const scenario.ScenarioV1, y: *const scenario.ScenarioV1) bool {
    if (x.seed != y.seed or x.fingerprint != y.fingerprint or !std.meta.eql(x.rules, y.rules)) return false;
    for (x.worlds, y.worlds) |first, second| {
        if (!textEqual(first.name, second.name)) return false;
        inline for (comptime std.meta.fieldNames(@TypeOf(first))) |field| {
            if (comptime !std.mem.eql(u8, field, "name")) {
                if (!std.meta.eql(@field(first, field), @field(second, field))) return false;
            }
        }
    }
    if (x.warnings.len > x.warnings.entries.len or y.warnings.len > y.warnings.entries.len or x.warnings.len != y.warnings.len) return false;
    for (x.warnings.entries[0..x.warnings.len], y.warnings.entries[0..y.warnings.len]) |first, second| {
        if (first.severity != second.severity or first.code != second.code or !textEqual(first.field, second.field) or !textEqual(first.message, second.message)) return false;
    }
    return true;
}
