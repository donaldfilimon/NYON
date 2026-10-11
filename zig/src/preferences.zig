//! Classic preferences model and versioned JSON codec from src/preferences/mod.rs.
//! Pure codec coverage only: native/browser stores and load/save outcome wrappers
//! are not implemented in this module. Decoded preferences own no input slices.
const std = @import("std");

pub const FORMAT_VERSION: u32 = 1;
pub const MAX_JSON_BYTES: usize = 16 * 1024;

pub const UiScale = enum {
    percent85,
    percent100,
    percent115,
    percent130,

    pub fn percent(self: UiScale) u16 {
        return switch (self) {
            .percent85 => 85,
            .percent100 => 100,
            .percent115 => 115,
            .percent130 => 130,
        };
    }

    pub fn factor(self: UiScale) f32 {
        return @as(f32, @floatFromInt(self.percent())) / 100.0;
    }

    pub fn fromPercent(value: u16) error{UiScale}!UiScale {
        return switch (value) {
            85 => .percent85,
            100 => .percent100,
            115 => .percent115,
            130 => .percent130,
            else => error.UiScale,
        };
    }
};

pub const MotionPreference = enum { full, reduced };
pub const GraphicsQuality = enum { auto, low, high };

pub const PresentationPreferences = struct {
    graphics_quality: GraphicsQuality = .auto,
    motion: MotionPreference = .full,
    high_contrast: bool = false,
};

pub const UserPreferencesV1 = struct {
    ui_scale: UiScale = .percent100,
    motion: MotionPreference = .full,
    high_contrast: bool = false,
    graphics_quality: GraphicsQuality = .auto,
    onboarding_completed: bool = false,

    pub fn presentation(self: UserPreferencesV1) PresentationPreferences {
        return .{
            .graphics_quality = self.graphics_quality,
            .motion = self.motion,
            .high_contrast = self.high_contrast,
        };
    }
};

pub const CodecError = error{ TooLarge, Json, Version, UiScale };
pub const CodecFailure = union(enum) {
    too_large,
    json: anyerror,
    version,
    ui_scale: u16,
};
pub const DecodeOutcome = union(enum) {
    preferences: UserPreferencesV1,
    failure: CodecFailure,
};

/// Returns an owned canonical JSON record; the caller frees it with allocator.
pub fn encode(allocator: std.mem.Allocator, preferences: UserPreferencesV1) (std.mem.Allocator.Error || CodecError)![]u8 {
    const wire = WirePreferences.fromPreferences(preferences);
    const payload = try std.json.Stringify.valueAlloc(allocator, wire, .{});
    errdefer allocator.free(payload);
    if (payload.len > MAX_JSON_BYTES) return error.TooLarge;
    return payload;
}

pub fn decode(allocator: std.mem.Allocator, json: []const u8) (std.mem.Allocator.Error || CodecError)!UserPreferencesV1 {
    return switch (try decodeDetailed(allocator, json)) {
        .preferences => |preferences| preferences,
        .failure => |failure| switch (failure) {
            .too_large => error.TooLarge,
            .json => error.Json,
            .version => error.Version,
            .ui_scale => error.UiScale,
        },
    };
}

/// Preserves the unsupported scale value and underlying JSON error while
/// keeping allocation failures distinct from invalid stored data.
pub fn decodeDetailed(allocator: std.mem.Allocator, json: []const u8) std.mem.Allocator.Error!DecodeOutcome {
    if (json.len > MAX_JSON_BYTES) return .{ .failure = .too_large };
    const parsed = std.json.parseFromSlice(SerdeRecord(WirePreferences), allocator, json, .{
        .duplicate_field_behavior = .@"error",
        .ignore_unknown_fields = false,
    }) catch |failure| switch (failure) {
        error.OutOfMemory => return error.OutOfMemory,
        else => return .{ .failure = .{ .json = failure } },
    };
    defer parsed.deinit();
    const wire = parsed.value.value;
    if (wire.format_version.value != FORMAT_VERSION) return .{ .failure = .version };
    const scale = UiScale.fromPercent(wire.ui_scale_percent.value) catch
        return .{ .failure = .{ .ui_scale = wire.ui_scale_percent.value } };
    return .{ .preferences = .{
        .ui_scale = scale,
        .motion = switch (wire.motion) {
            .FULL => .full,
            .REDUCED => .reduced,
        },
        .high_contrast = wire.high_contrast,
        .graphics_quality = switch (wire.graphics_quality) {
            .AUTO => .auto,
            .LOW => .low,
            .HIGH => .high,
        },
        .onboarding_completed = wire.onboarding_completed,
    } };
}

const WirePreferences = struct {
    format_version: StrictUnsigned(u32),
    ui_scale_percent: StrictUnsigned(u16),
    motion: WireMotion,
    high_contrast: bool,
    graphics_quality: WireGraphicsQuality,
    onboarding_completed: bool,

    fn fromPreferences(preferences: UserPreferencesV1) WirePreferences {
        return .{
            .format_version = .{ .value = FORMAT_VERSION },
            .ui_scale_percent = .{ .value = preferences.ui_scale.percent() },
            .motion = switch (preferences.motion) {
                .full => .FULL,
                .reduced => .REDUCED,
            },
            .high_contrast = preferences.high_contrast,
            .graphics_quality = switch (preferences.graphics_quality) {
                .auto => .AUTO,
                .low => .LOW,
                .high => .HIGH,
            },
            .onboarding_completed = preferences.onboarding_completed,
        };
    }
};

// std.json's default integer parser accepts numeric strings and integral
// floats. serde's u16/u32 fields accept only nonnegative integer JSON tokens.
fn StrictUnsigned(comptime T: type) type {
    return struct {
        value: T,

        pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!@This() {
            const token = try source.nextAllocMax(allocator, .alloc_if_needed, options.max_value_len.?);
            defer freeToken(allocator, token);
            const bytes = switch (token) {
                .number, .allocated_number => |bytes| bytes,
                else => return error.UnexpectedToken,
            };
            if (bytes.len == 0 or bytes[0] == '-' or !std.json.isNumberFormattedLikeAnInteger(bytes)) return error.InvalidNumber;
            return .{ .value = std.fmt.parseInt(T, bytes, 10) catch return error.InvalidNumber };
        }

        pub fn jsonStringify(self: @This(), writer: anytype) !void {
            try writer.write(self.value);
        }
    };
}

const WireMotion = enum {
    FULL,
    REDUCED,

    pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!WireMotion {
        return parseUnitEnum(WireMotion, allocator, source, options);
    }
};
const WireGraphicsQuality = enum {
    AUTO,
    LOW,
    HIGH,

    pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!WireGraphicsQuality {
        return parseUnitEnum(WireGraphicsQuality, allocator, source, options);
    }
};

// Serde unit variants accept a named string or a single-key object whose
// value is null. Encoding always emits the named string.
fn parseUnitEnum(comptime T: type, allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!T {
    const object = try source.peekNextTokenType() == .object_begin;
    if (object) _ = try source.next();
    const token = try source.nextAllocMax(allocator, .alloc_if_needed, options.max_value_len.?);
    defer freeToken(allocator, token);
    const name = switch (token) {
        .string, .allocated_string => |name| name,
        else => return error.UnexpectedToken,
    };
    const value = std.meta.stringToEnum(T, name) orelse return error.InvalidEnumTag;
    if (object) {
        if (try source.next() != .null) return error.UnexpectedToken;
        if (try source.next() != .object_end) return error.UnexpectedToken;
    }
    return value;
}

// Serde-derived structs accept either a field object or an exact field-order
// sequence. Keep sequence acceptance without altering the canonical object.
fn SerdeRecord(comptime T: type) type {
    return struct {
        value: T,

        pub fn jsonParse(allocator: std.mem.Allocator, source: anytype, options: std.json.ParseOptions) std.json.ParseError(@TypeOf(source.*))!@This() {
            if (try source.peekNextTokenType() != .array_begin) return .{ .value = try std.json.innerParse(T, allocator, source, options) };
            _ = try source.next();
            var value: T = undefined;
            const info = @typeInfo(T).@"struct";
            inline for (info.field_names, info.field_types) |name, Field| {
                @field(value, name) = try std.json.innerParse(Field, allocator, source, options);
            }
            if (try source.next() != .array_end) return error.UnexpectedToken;
            return .{ .value = value };
        }
    };
}

fn freeToken(allocator: std.mem.Allocator, token: std.json.Token) void {
    switch (token) {
        .allocated_string, .allocated_number => |bytes| allocator.free(bytes),
        else => {},
    }
}
