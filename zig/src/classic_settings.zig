//! Pure Classic settings actions and bounded keyboard focus. No retained preferences.
const prefs = @import("preferences.zig");
pub const Action = union(enum) { set_ui_scale: prefs.UiScale, set_motion: prefs.MotionPreference, set_high_contrast: bool, set_graphics_quality: prefs.GraphicsQuality, reset_onboarding, close };
pub const CONTROL_COUNT: usize = 11;
pub fn actions(value: prefs.UserPreferencesV1) [CONTROL_COUNT]Action {
    return .{
        .{ .set_ui_scale = .percent85 },                                   .{ .set_ui_scale = .percent100 },               .{ .set_ui_scale = .percent115 },   .{ .set_ui_scale = .percent130 },
        .{ .set_motion = if (value.motion == .full) .reduced else .full }, .{ .set_high_contrast = !value.high_contrast }, .{ .set_graphics_quality = .auto }, .{ .set_graphics_quality = .low },
        .{ .set_graphics_quality = .high },                                .reset_onboarding,                              .close,
    };
}
pub const State = struct {
    _index: usize = 0,
    pub fn focusIndex(self: State) usize {
        return self._index;
    }
    pub fn focus(self: *State, index: usize) void {
        self._index = @min(index, CONTROL_COUNT - 1);
    }
    pub fn next(self: *State) void {
        self._index = (self._index + 1) % CONTROL_COUNT;
    }
    pub fn previous(self: *State) void {
        self._index = (self._index + CONTROL_COUNT - 1) % CONTROL_COUNT;
    }
};
