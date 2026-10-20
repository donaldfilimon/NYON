const _contracts = @import("coordinated_galaxy/contracts_tests.zig");
const _wire = @import("coordinated_galaxy/wire_vectors_tests.zig");

test {
    _ = _contracts;
    _ = _wire;
}

comptime {
    _ = @import("coordinated_galaxy/planet_tests.zig");
    _ = @import("coordinated_galaxy/state_tests.zig");
    _ = @import("coordinated_galaxy/command_tests.zig");
}

comptime {
    _ = @import("coordinated_galaxy/view_tests.zig");
    _ = @import("coordinated_galaxy/history_tests.zig");
}

comptime {
    _ = @import("coordinated_galaxy/job_tests.zig");
}

comptime {
    _ = @import("coordinated_galaxy/fault_tests.zig");
}

comptime {
    _ = @import("coordinated_galaxy/replay_tests.zig");
}
