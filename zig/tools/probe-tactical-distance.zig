//! Isolated exact-distance feasibility probe; no game or authority integration.
const std = @import("std");
const surface = @import("surface");
const Allocator = std.mem.Allocator;

const minimum_side = 4;
const maximum_side = 32;
const maximum_cells = 6 * maximum_side * maximum_side;
const side_count = maximum_side - minimum_side + 1;
const field_count = 1627;
const distance_bytes = 6_061_056;
const mapping_count = 68_556;
const partition_bytes = 6 * 1024 * 1024;
const raw_end = distance_bytes + mapping_count * @sizeOf(Mapping) + side_count * @sizeOf(u64) + field_count * @sizeOf(u32) + 48 * 9;

const Mapping = extern struct { transform: u8, orbit: u8 };
const Matrix = [9]i8;
const Partition = extern struct {
    distances: [distance_bytes]u8,
    mappings: [mapping_count]Mapping,
    side_offsets: [side_count]u64,
    field_offsets: [field_count]u32,
    matrices: [48]Matrix,
    padding: [partition_bytes - raw_end]u8,
};
comptime {
    if (@sizeOf(Mapping) != 2 or raw_end != 6_205_340 or @sizeOf(Partition) != partition_bytes)
        @compileError("distance partition layout differs from approved accounting");
    if (@offsetOf(Partition, "mappings") != distance_bytes or
        @offsetOf(Partition, "side_offsets") != distance_bytes + mapping_count * 2 or
        @offsetOf(Partition, "field_offsets") != distance_bytes + mapping_count * 2 + side_count * 8 or
        @offsetOf(Partition, "matrices") != raw_end - 48 * 9 or
        @offsetOf(Partition, "padding") != raw_end)
        @compileError("distance partition has unexpected internal padding");
}

const Vec = [3]i32;
const Basis = struct { normal: Vec, right: Vec, up: Vec };
const bases = [_]Basis{
    .{ .normal = .{ 1, 0, 0 }, .right = .{ 0, 0, -1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ -1, 0, 0 }, .right = .{ 0, 0, 1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, -1 } },
    .{ .normal = .{ 0, -1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, 1 } },
    .{ .normal = .{ 0, 0, 1 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 0, -1 }, .right = .{ -1, 0, 0 }, .up = .{ 0, 1, 0 } },
};
const permutations = [_][3]u8{
    .{ 0, 1, 2 }, .{ 0, 2, 1 }, .{ 1, 0, 2 },
    .{ 1, 2, 0 }, .{ 2, 0, 1 }, .{ 2, 1, 0 },
};
const directions = [_]surface.Direction{ .east, .west, .north, .south };

fn dot(a: Vec, b: Vec) i32 {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

fn matrixVector(matrix: Matrix, vector: Vec) Vec {
    var result: Vec = undefined;
    for (0..3) |row| {
        result[row] = 0;
        for (0..3) |column| result[row] += @as(i32, matrix[row * 3 + column]) * vector[column];
    }
    return result;
}

fn matrices() [48]Matrix {
    var result: [48]Matrix = undefined;
    var index: usize = 0;
    for (permutations) |permutation| {
        for (0..8) |signs| {
            var matrix: Matrix = @splat(0);
            for (0..3) |row| matrix[row * 3 + permutation[row]] = if ((signs & (@as(usize, 1) << @intCast(row))) == 0) 1 else -1;
            result[index] = matrix;
            index += 1;
        }
    }
    return result;
}

fn transform(topology: surface.Topology, matrix: Matrix, id: u16) !u16 {
    const cell = try topology.fromId(id);
    const basis = bases[@backingInt(cell.face)];
    const extent = @as(i32, topology.side) - 1;
    const x = 2 * @as(i32, cell.x) - extent;
    const y = 2 * @as(i32, cell.y) - extent;
    var position: Vec = undefined;
    for (0..3) |axis| position[axis] = basis.normal[axis] * extent + basis.right[axis] * x + basis.up[axis] * y;
    const normal = matrixVector(matrix, basis.normal);
    position = matrixVector(matrix, position);
    for (bases, 0..) |target, face| {
        if (!std.mem.eql(i32, &target.normal, &normal)) continue;
        const mapped: surface.Cell = .{
            .face = @fromBackingInt(@intCast(face)),
            .x = @intCast(@divExact(dot(position, target.right) + extent, 2)),
            .y = @intCast(@divExact(dot(position, target.up) + extent, 2)),
        };
        return @intCast(try topology.cellId(mapped));
    }
    return error.InvalidTransform;
}

fn totalCells(side: u16) usize {
    return 6 * @as(usize, side) * side;
}

fn orbitCount(side: u16) usize {
    const half = (@as(usize, side) + 1) / 2;
    return half * (half + 1) / 2;
}

fn fieldBase(side: u16) usize {
    var result: usize = 0;
    for (minimum_side..side) |s| result += orbitCount(@intCast(s));
    return result;
}

fn validate(side: u16, goal: u16, source: u16) !void {
    if (side < minimum_side or side > maximum_side) return error.InvalidSide;
    if (goal >= totalCells(side) or source >= totalCells(side)) return error.InvalidCell;
}

fn bfs(topology: surface.Topology, goal: u16, distances: []u8, queue: *[maximum_cells]u16) !u8 {
    const count = totalCells(topology.side);
    if (distances.len != count or goal >= count) return error.InvalidCell;
    @memset(distances, 255);
    distances[goal] = 0;
    queue[0] = goal;
    var head: usize = 0;
    var tail: usize = 1;
    var maximum: u8 = 0;
    while (head < tail) : (head += 1) {
        const current = queue[head];
        const cell = try topology.fromId(current);
        for (directions) |direction| {
            const next: u16 = @intCast(try topology.cellId(try topology.neighbor(cell, direction)));
            if (distances[next] != 255) continue;
            if (distances[current] == 254) return error.DistanceOverflow;
            if (tail == count or tail == queue.len) return error.QueueOverflow;
            const next_distance = distances[current] + 1;
            distances[next] = next_distance;
            maximum = @max(maximum, next_distance);
            queue[tail] = next;
            tail += 1;
        }
    }
    if (tail != count) return error.UnreachableCell;
    return maximum;
}

const DistanceCache = struct {
    source: *Partition,
    replay: *Partition,
    maximum_distance: u8,

    fn init(allocator: Allocator) !DistanceCache {
        const source = try allocator.create(Partition);
        errdefer allocator.destroy(source);
        const replay = try allocator.create(Partition);
        errdefer allocator.destroy(replay);
        if (@intFromPtr(source) == @intFromPtr(replay)) return error.OverlappingPartitions;
        @memset(std.mem.asBytes(source), 0);
        source.matrices = matrices();
        var queue: [maximum_cells]u16 = undefined;
        var mapping_cursor: usize = 0;
        var field_cursor: usize = 0;
        var distance_cursor: usize = 0;
        var maximum: u8 = 0;
        for (minimum_side..maximum_side + 1) |raw_side| {
            const side: u16 = @intCast(raw_side);
            const topology = try surface.Topology.init(side);
            const count = totalCells(side);
            const half = (raw_side + 1) / 2;
            source.side_offsets[raw_side - minimum_side] = mapping_cursor;
            for (0..count) |goal_id| {
                const goal: u16 = @intCast(goal_id);
                var found = false;
                for (source.matrices, 0..) |matrix, transform_index| {
                    const mapped = try transform(topology, matrix, goal);
                    const cell = try topology.fromId(mapped);
                    if (cell.face != .positive_z or cell.x > cell.y or cell.y >= half) continue;
                    source.mappings[mapping_cursor + goal_id] = .{
                        .transform = @intCast(transform_index),
                        .orbit = @intCast(@as(usize, cell.y) * (cell.y + 1) / 2 + cell.x),
                    };
                    found = true;
                    break;
                }
                if (!found) return error.UnmappedGoal;
            }
            mapping_cursor += count;
            for (0..half) |y| for (0..y + 1) |x| {
                const goal: u16 = @intCast(try topology.cellId(.{ .face = .positive_z, .x = @intCast(x), .y = @intCast(y) }));
                source.field_offsets[field_cursor] = @intCast(distance_cursor);
                maximum = @max(maximum, try bfs(topology, goal, source.distances[distance_cursor .. distance_cursor + count], &queue));
                field_cursor += 1;
                distance_cursor += count;
            };
        }
        if (mapping_cursor != mapping_count or field_cursor != field_count or distance_cursor != distance_bytes)
            return error.LayoutMismatch;
        @memcpy(std.mem.asBytes(replay), std.mem.asBytes(source));
        return .{ .source = source, .replay = replay, .maximum_distance = maximum };
    }

    fn deinit(self: *DistanceCache, allocator: Allocator) void {
        allocator.destroy(self.replay);
        allocator.destroy(self.source);
        self.* = undefined;
    }

    fn distance(self: *const DistanceCache, side: u16, goal: u16, source: u16) !u8 {
        try validate(side, goal, source);
        const topology = try surface.Topology.init(side);
        const part = self.source;
        const mapping = part.mappings[part.side_offsets[side - minimum_side] + goal];
        const mapped_source = try transform(topology, part.matrices[mapping.transform], source);
        const field = fieldBase(side) + mapping.orbit;
        const value = part.distances[part.field_offsets[field] + mapped_source];
        if (value == 255) return error.UnreachableCell;
        return value;
    }

    fn nextCell(self: *const DistanceCache, side: u16, goal: u16, source: u16) !?u16 {
        try validate(side, goal, source);
        if (source == goal) return null;
        const current_distance = try self.distance(side, goal, source);
        const topology = try surface.Topology.init(side);
        const cell = try topology.fromId(source);
        var best: ?u16 = null;
        var best_distance: u8 = 255;
        for (directions) |direction| {
            const next: u16 = @intCast(try topology.cellId(try topology.neighbor(cell, direction)));
            const distance_to_goal = try self.distance(side, goal, next);
            if (distance_to_goal >= current_distance) continue;
            if (best == null or distance_to_goal < best_distance or (distance_to_goal == best_distance and next < best.?)) {
                best = next;
                best_distance = distance_to_goal;
            }
        }
        return best orelse error.UnreachableCell;
    }
};

const Counts = struct { transform_cells: u64 = 0, neighbor_edges: u64 = 0, pair_checks: u64 = 0, maximum_oracle_distance: u8 = 0 };

fn verify(cache: *const DistanceCache) !Counts {
    var counts: Counts = .{};
    const partition = cache.source;
    var queue: [maximum_cells]u16 = undefined;
    var distances: [maximum_cells]u8 = undefined;
    var transformed: [maximum_cells]u16 = undefined;
    var seen: [maximum_cells]bool = undefined;
    for (minimum_side..maximum_side + 1) |raw_side| {
        const side: u16 = @intCast(raw_side);
        const topology = try surface.Topology.init(side);
        const count = totalCells(side);
        const half = (raw_side + 1) / 2;
        var used_orbits: [136]bool = @splat(false);
        for (0..count) |goal_id| {
            const mapping = partition.mappings[partition.side_offsets[raw_side - minimum_side] + goal_id];
            if (mapping.transform >= 48 or mapping.orbit >= orbitCount(side)) return error.InvalidMapping;
            const mapped = try transform(topology, partition.matrices[mapping.transform], @intCast(goal_id));
            const cell = try topology.fromId(mapped);
            if (cell.face != .positive_z or cell.x > cell.y or cell.y >= half or
                mapping.orbit != @as(usize, cell.y) * (cell.y + 1) / 2 + cell.x) return error.InvalidMapping;
            used_orbits[mapping.orbit] = true;
        }
        for (used_orbits[0..orbitCount(side)]) |used| if (!used) return error.UnpopulatedOrbit;
        for (partition.matrices) |matrix| {
            @memset(seen[0..count], false);
            for (0..count) |id| {
                const mapped = try transform(topology, matrix, @intCast(id));
                if (mapped >= count or seen[mapped]) return error.NotBijection;
                seen[mapped] = true;
                transformed[id] = mapped;
                counts.transform_cells += 1;
            }
            for (seen[0..count]) |present| if (!present) return error.NotBijection;
            var inverse: ?Matrix = null;
            for (partition.matrices) |candidate| {
                var equal = true;
                for (0..3) |row| for (0..3) |column| {
                    if (candidate[row * 3 + column] != matrix[column * 3 + row]) equal = false;
                };
                if (equal) {
                    inverse = candidate;
                    break;
                }
            }
            if (inverse == null) return error.MissingInverse;
            for (0..count) |id| {
                const source = try topology.fromId(id);
                const mapped = try topology.fromId(transformed[id]);
                if (try transform(topology, inverse.?, transformed[id]) != id) return error.InvalidInverse;
                for (directions) |direction| {
                    const neighbor: usize = @intCast(try topology.cellId(try topology.neighbor(source, direction)));
                    const target_neighbor = transformed[neighbor];
                    var matched = false;
                    for (directions) |target_direction| {
                        const adjacent = try topology.cellId(try topology.neighbor(mapped, target_direction));
                        if (adjacent == target_neighbor) matched = true;
                    }
                    if (!matched) return error.BrokenEdge;
                    counts.neighbor_edges += 1;
                }
            }
        }
        if (side == 4 or side == 5 or side == 8 or side == 32) {
            for (0..count) |goal_id| {
                counts.maximum_oracle_distance = @max(counts.maximum_oracle_distance, try bfs(topology, @intCast(goal_id), distances[0..count], &queue));
                for (0..count) |source_id| {
                    const source: u16 = @intCast(source_id);
                    if (try cache.distance(side, @intCast(goal_id), source) != distances[source_id]) return error.DistanceMismatch;
                    var expected: ?u16 = null;
                    if (goal_id != source_id) {
                        const cell = try topology.fromId(source);
                        for (directions) |direction| {
                            const next: u16 = @intCast(try topology.cellId(try topology.neighbor(cell, direction)));
                            if (distances[next] >= distances[source_id]) continue;
                            if (expected == null or distances[next] < distances[expected.?] or
                                (distances[next] == distances[expected.?] and next < expected.?)) expected = next;
                        }
                    }
                    if (try cache.nextCell(side, @intCast(goal_id), source) != expected) return error.NextCellMismatch;
                    counts.pair_checks += 1;
                }
            }
        }
    }
    if (counts.maximum_oracle_distance != cache.maximum_distance) return error.MaximumMismatch;
    if (!std.mem.eql(u8, std.mem.asBytes(cache.source), std.mem.asBytes(cache.replay))) return error.ReplayMismatch;
    return counts;
}

const Query = extern struct { side: u16, goal: u16, source: u16 };
comptime {
    if (@sizeOf(Query) != 6) @compileError("query corpus must occupy six bytes per intent");
}

fn now(io: std.Io) std.Io.Timestamp {
    return std.Io.Clock.awake.now(io);
}
fn elapsed(start: std.Io.Timestamp, end: std.Io.Timestamp) u64 {
    return @intCast(start.durationTo(end).toNanoseconds());
}

fn measure(cache: *const DistanceCache, allocator: Allocator, io: std.Io) !void {
    const queries = try allocator.alloc(Query, 16_384);
    defer allocator.free(queries);
    var goals_seen: [6144]bool = @splat(false);
    var sources_seen: [6144]bool = @splat(false);
    var hash = std.crypto.hash.sha2.Sha256.init(.{});
    hash.update("NYONDC01");
    var bytes4: [4]u8 = undefined;
    std.mem.writeInt(u32, &bytes4, 16_384, .little);
    hash.update(&bytes4);
    for (queries, 0..) |*query, j| {
        query.* = .{ .side = 32, .goal = @intCast(j % 6144), .source = @intCast((37 * j + 19) % 6144) };
        goals_seen[query.goal] = true;
        sources_seen[query.source] = true;
        var bytes2: [2]u8 = undefined;
        for ([_]u16{ query.side, query.goal, query.source }) |value| {
            std.mem.writeInt(u16, &bytes2, value, .little);
            hash.update(&bytes2);
        }
    }
    for (goals_seen, sources_seen) |goal_seen, source_seen| {
        if (!goal_seen or !source_seen) return error.IncompleteCorpusCoverage;
    }
    var digest: [32]u8 = undefined;
    hash.final(&digest);
    var samples: [100]u64 = undefined;
    var checksum: u64 = 0;
    for (&samples) |*sample| {
        const begin = now(io);
        for (queries) |query| {
            const next = try cache.nextCell(query.side, query.goal, query.source);
            checksum = checksum *% 1_315_423_911 +% @as(u64, next orelse 65535);
        }
        sample.* = elapsed(begin, now(io));
    }
    std.mem.sort(u64, &samples, {}, std.sort.asc(u64));
    std.debug.print("corpus_sha256={x} corpus_goal_coverage=6144 corpus_source_coverage=6144 checksum={d} cached_ns_min={d} cached_ns_p95={d} cached_ns_max={d}\n", .{ digest, checksum, samples[0], samples[94], samples[99] });
    const topology = try surface.Topology.init(32);
    var queue: [maximum_cells]u16 = undefined;
    var distances: [maximum_cells]u8 = undefined;
    var baseline_checksum: u64 = 0;
    const baseline_start = now(io);
    for (queries) |query| {
        _ = try bfs(topology, query.goal, &distances, &queue);
        baseline_checksum = baseline_checksum *% 1_315_423_911 +% distances[query.source];
    }
    const verify_named_bulk_arrays_declared_bytes = @sizeOf([maximum_cells]u16) * 2 +
        @sizeOf([maximum_cells]u8) + @sizeOf([maximum_cells]bool);
    const measure_named_bulk_arrays_declared_bytes = @sizeOf(@TypeOf(queue)) +
        @sizeOf(@TypeOf(distances)) + @sizeOf(@TypeOf(goals_seen)) +
        @sizeOf(@TypeOf(sources_seen)) + @sizeOf(@TypeOf(samples));
    std.debug.print("baseline_bfs_one_batch_ns={d} baseline_distance_checksum={d} explicit_heap_request_peak={d} retained_explicit_requests={d} corpus_explicit_request={d} construction_queue_declared_bytes={d} verify_named_bulk_arrays_declared_bytes={d} measure_named_bulk_arrays_declared_bytes={d}\n", .{
        elapsed(baseline_start, now(io)),                      baseline_checksum,
        2 * @sizeOf(Partition) + @sizeOf(Query) * queries.len, 2 * @sizeOf(Partition),
        @sizeOf(Query) * queries.len,                          @sizeOf(@TypeOf(queue)),
        verify_named_bulk_arrays_declared_bytes,               measure_named_bulk_arrays_declared_bytes,
    });
    std.debug.print("compiled_peak_stack=unmeasured process_peak=unmeasured runtime_allocator_overhead=excluded\n", .{});
}

pub fn main(init: std.process.Init) !void {
    const allocator = init.gpa;
    const io = init.io;
    const start = now(io);
    var cache = try DistanceCache.init(allocator);
    defer cache.deinit(allocator);
    const partition_alignment = @alignOf(Partition);
    const source_mod_alignment = @intFromPtr(cache.source) % partition_alignment;
    const replay_mod_alignment = @intFromPtr(cache.replay) % partition_alignment;
    if (source_mod_alignment != 0 or replay_mod_alignment != 0) return error.PartitionMisaligned;
    const initialized = now(io);
    const counts = try verify(&cache);
    const verified = now(io);
    std.debug.print("build=ReleaseSafe retained_source={d} retained_replay={d} partition_align={d} source_mod_align={d} replay_mod_align={d} raw_end={d} partition_slack={d} maximum_distance={d} transform_cells={d} neighbor_edges={d} pair_checks={d} init_ns={d} verify_ns={d}\n", .{
        @sizeOf(Partition),             @sizeOf(Partition),     partition_alignment,
        source_mod_alignment,           replay_mod_alignment,   raw_end,
        partition_bytes - raw_end,      cache.maximum_distance, counts.transform_cells,
        counts.neighbor_edges,          counts.pair_checks,     elapsed(start, initialized),
        elapsed(initialized, verified),
    });
    try measure(&cache, allocator, io);
}

test "invalid inputs and same-goal proposal" {
    var cache = try DistanceCache.init(std.testing.allocator);
    defer cache.deinit(std.testing.allocator);
    for ([_]u16{ 0, 3, 33 }) |side| {
        try std.testing.expectError(error.InvalidSide, cache.distance(side, 0, 0));
        try std.testing.expectError(error.InvalidSide, cache.nextCell(side, 0, 0));
    }
    try std.testing.expectError(error.InvalidCell, cache.distance(4, 96, 0));
    try std.testing.expectError(error.InvalidCell, cache.distance(4, 0, 96));
    try std.testing.expectEqual(@as(?u16, null), try cache.nextCell(4, 71, 71));
    try std.testing.expectEqual(@as(u8, 0), try cache.distance(4, 71, 71));
}

test "orbit counts, seams and original-ID tie" {
    try std.testing.expectEqual(@as(usize, 3), orbitCount(4));
    try std.testing.expectEqual(@as(usize, 6), orbitCount(5));
    try std.testing.expectEqual(@as(usize, 136), orbitCount(32));
    var cache = try DistanceCache.init(std.testing.allocator);
    defer cache.deinit(std.testing.allocator);
    try std.testing.expectEqual(@as(u8, 1), try cache.distance(4, 71, 4));
    try std.testing.expectEqual(@as(?u16, 71), try cache.nextCell(4, 71, 4));
    const topology = try surface.Topology.init(4);
    const a = try topology.fromId(71);
    try std.testing.expectEqual(@as(u64, 4), try topology.cellId(try topology.neighbor(a, .east)));
    const b = try topology.fromId(4);
    var reciprocal = false;
    for (directions) |direction| if (try topology.cellId(try topology.neighbor(b, direction)) == 71) {
        reciprocal = true;
    };
    try std.testing.expect(reciprocal);
    var queue: [maximum_cells]u16 = undefined;
    var distances: [maximum_cells]u8 = undefined;
    var found_tie = false;
    for (0..96) |goal_id| {
        _ = try bfs(topology, @intCast(goal_id), distances[0..96], &queue);
        for (0..96) |source_id| {
            if (source_id == goal_id) continue;
            const cell = try topology.fromId(source_id);
            var best: ?u16 = null;
            var choices: usize = 0;
            for (directions) |direction| {
                const next: u16 = @intCast(try topology.cellId(try topology.neighbor(cell, direction)));
                if (distances[next] + 1 != distances[source_id]) continue;
                choices += 1;
                if (best == null or next < best.?) best = next;
            }
            if (choices < 2) continue;
            try std.testing.expectEqual(best, try cache.nextCell(4, @intCast(goal_id), @intCast(source_id)));
            found_tie = true;
            break;
        }
        if (found_tie) break;
    }
    try std.testing.expect(found_tie);
}
