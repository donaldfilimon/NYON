pub const model = @import("model.zig");
pub const play_role = @import("play_role.zig");
pub const planet_economy = @import("planet_economy/root.zig");
pub const coordinated_galaxy = @import("coordinated_galaxy/root.zig");
pub const planet_economy_cli = @import("planet_economy_cli.zig");
pub const generator = @import("generator.zig");
pub const economy = @import("economy.zig");
pub const navigation = @import("navigation.zig");
pub const simulation = @import("simulation.zig");
pub const scenario = @import("scenario.zig");
pub const preferences = @import("preferences.zig");
pub const classic_store = @import("classic_store.zig");

pub const classic_store_load = @import("classic_store_load.zig");
pub const classic_clock = @import("classic_clock.zig");
pub const classic_app = @import("classic_app.zig");
pub const classic_commands = @import("classic_commands.zig");
pub const classic_editor = @import("classic_editor.zig");
pub const classic_editor_layout = @import("classic_editor_layout.zig");
pub const classic_editor_ui = @import("classic_editor_ui.zig");
pub const classic_editor_transactions = @import("classic_editor_transactions.zig");
pub const classic_onboarding = @import("classic_onboarding.zig");
pub const classic_settings = @import("classic_settings.zig");
pub const classic_session = @import("classic_session.zig");
pub const classic_interaction = @import("classic_interaction.zig");
pub const classic_input = @import("classic_input.zig");
pub const classic_guide = @import("classic_guide.zig");
pub const classic_ui = @import("classic_ui.zig");

pub const classic_advisory_types = @import("classic_advisory_types.zig");
pub const classic_advisory_features = @import("classic_advisory_features.zig");
pub const classic_advisory_model = @import("classic_advisory_model.zig");

pub const classic_advisory_controller = @import("classic_advisory_controller.zig");

pub const classic_advisory_projection = @import("classic_advisory_projection.zig");

pub const classic_math = @import("classic_math.zig");
pub const classic_camera = @import("classic_camera.zig");
pub const classic_picking = @import("classic_picking.zig");

pub const classic_start_marker = @import("classic_start_marker.zig");
pub const classic_start_marker_adapter = @import("classic_start_marker_adapter.zig");

pub const classic_scene = @import("classic_scene.zig");

pub const retained_semantics = @import("retained_semantics.zig");

pub const retained_geometry = @import("retained_geometry.zig");

pub const retained_text = @import("retained_text.zig");
pub const retained_atlas = @import("retained_atlas.zig");

test {
    _ = play_role;
    _ = surface;
    _ = @import("classic_interaction_tests.zig");
    _ = @import("classic_input_tests.zig");
    _ = @import("classic_guide_tests.zig");
    _ = @import("classic_start_marker_adapter_tests.zig");
    _ = @import("classic_start_marker_tests.zig");
    _ = @import("classic_editor_ui_tests.zig");
    _ = @import("classic_editor_layout_tests.zig");
    _ = @import("classic_ui_tests.zig");
    _ = @import("retained_text_tests.zig");
    _ = @import("retained_semantics_tests.zig");
    _ = @import("retained_geometry_tests.zig");
    _ = @import("classic_composition_tests.zig");
    _ = @import("classic_scene_tests.zig");
    _ = @import("classic_math_tests.zig");
    _ = @import("classic_camera_picking_tests.zig");
    _ = @import("classic_advisory_integration_tests.zig");
    _ = @import("classic_advisory_controller_tests.zig");
    _ = @import("classic_advisory_reference_tests.zig");
    _ = model;
    _ = generator;
    _ = economy;
    _ = navigation;
    _ = simulation;
    _ = @import("simulation_tests.zig");
    _ = @import("scenario_tests.zig");
    _ = @import("preferences_tests.zig");
    _ = @import("integration_tests.zig");
    _ = @import("classic_store_tests.zig");
    _ = @import("store_integration_tests.zig");
    _ = @import("classic_clock_tests.zig");
    _ = @import("classic_app_tests.zig");
    _ = @import("classic_commands_tests.zig");
    _ = @import("classic_editor_tests.zig");
    _ = @import("classic_editor_transactions_tests.zig");
    _ = @import("classic_onboarding_tests.zig");
    _ = @import("classic_settings_tests.zig");
    _ = @import("classic_session_tests.zig");
}
pub const surface = @import("surface.zig");

test {
    _ = @import("planet_economy_tests.zig");
    _ = @import("coordinated_galaxy_tests.zig");
}
