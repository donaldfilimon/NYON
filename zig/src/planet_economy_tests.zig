const _contracts = @import("planet_economy/contracts_tests.zig");
const _state = @import("planet_economy/state_tests.zig");
const _command = @import("planet_economy/command_tests.zig");
const _phases = @import("planet_economy/phases_tests.zig");
const _history = @import("planet_economy/history_tests.zig");

test {
    _ = _contracts;
    _ = _state;
    _ = _command;
    _ = _phases;
    _ = _history;
}

comptime {
    _ = @import("planet_economy/jobs_tests.zig");
}
comptime {
    _ = @import("planet_economy/replay_tests.zig");
}
comptime {
    _ = @import("planet_economy/robustness_tests.zig");
}
comptime {
    _ = @import("planet_economy_cli_tests.zig");
}

comptime {
    _ = @import("planet_economy/invariants_tests.zig");
}
