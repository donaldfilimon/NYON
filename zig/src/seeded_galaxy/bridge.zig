const t = @import("types.zig");
const Manifest = @import("manifest.zig");
const cg = @import("../coordinated_galaxy/types.zig");

pub const BridgeOptions = struct {
    planet_capacities: []const cg.PlanetCapacities,
    global_history_capacity: u16,
};

pub fn toCoordinatedConfig(manifest: *const Manifest, options: BridgeOptions) t.Failure!cg.Config {
    const source = try manifest.config();
    if (source.systems > 4 or source.planets > 4 or source.civilizations > 4)
        return error.CapacityUnsupported;
    if (options.planet_capacities.len != source.planets) return error.InvalidConfig;

    var config: cg.Config = .{
        .seed = source.seed,
        .system_count = @intCast(source.systems),
        .planet_count = @intCast(source.planets),
        .civilization_count = @intCast(source.civilizations),
        .global_history_capacity = options.global_history_capacity,
    };
    var planets: [4]t.Planet = undefined;
    const page = try manifest.planets(0, planets[0..source.planets]);
    if (page.written != source.planets or page.next != null) return error.InvariantViolation;
    for (planets[0..source.planets], options.planet_capacities, 0..) |planet, capacities, index| {
        config.planets[index] = .{
            .system = @fromBackingInt(@backingInt(planet.system)),
            .side = planet.side,
            .capacities = capacities,
        };
    }
    cg.validateConfig(config) catch return error.InvalidConfig;
    return config;
}
