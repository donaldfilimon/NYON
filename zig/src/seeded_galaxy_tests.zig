test {
    _ = @import("seeded_galaxy/qualification_tests.zig");
    _ = @import("seeded_galaxy/wire_tests.zig");
    _ = @import("seeded_galaxy/lifecycle_tests.zig");
    _ = @import("seeded_galaxy/contracts_tests.zig");
    _ = @import("seeded_galaxy/generation_tests.zig");
    _ = @import("seeded_galaxy/policy_bridge_tests.zig");
}

/// Explicit qualification executable; not part of runtime authority.
pub fn main(init: @import("std").process.Init) !void {
    try @import("seeded_galaxy/qualification_tests.zig").benchmark(init);
}
