const role = @import("../play_role.zig");

pub const PlanetId = enum(u32) { _ };
pub const SettlementId = enum(u64) { _ };
pub const BuildingId = enum(u64) { _ };
pub const ShipmentId = enum(u64) { _ };
pub const Tick = enum(u64) { _ };
pub const Revision = enum(u64) { _ };
pub const CommandSequence = enum(u64) { _ };
pub const JobId = enum(u64) { _ };
pub const CellId = struct { planet: PlanetId, local: u64 };

pub const planet_id: PlanetId = @fromBackingInt(@as(u32, 1));
pub const civilization_id: role.CivilizationId = @fromBackingInt(@as(u32, 7));
