const std = @import("std");
const t = @import("types.zig");
pub const Output = struct {
    config: t.Config,
    systems: [64]t.System,
    planets: [512]t.Planet,
    civilizations: [16]t.Civilization,
    wire: [16384]u8,
    digest: [32]u8,
    wire_len: u16,
};
pub const Scratch = struct {
    input: [16384]u8,
    sha: std.crypto.hash.sha2.Sha256,
    block: [64]u8,
    input_len: u16,
    parsed_cursor: u16,
    record_cursor: u16,
    block_cursor: u16,
    phase: u8,
    mode: u8,
    total_units: u64,
};
/// Budget for the later inline Setup controls and owning handles together.
pub const control_envelope_bytes: usize = 1024;
pub const output_alignment_pad: usize = @alignOf(Output) - 1;
pub const scratch_alignment_pad: usize = @alignOf(Scratch) - 1;

pub fn assertControlsFit(comptime Controls: type, comptime Handles: type) void {
    if (@sizeOf(Controls) + @sizeOf(Handles) > control_envelope_bytes)
        @compileError("strategic controls exceed reserved envelope");
}

pub fn candidateReservation() usize {
    return @sizeOf(Output) + @sizeOf(Scratch) + control_envelope_bytes +
        output_alignment_pad + scratch_alignment_pad;
}

/// Detached ownership includes the output allocation, owning handle and pad.
pub fn detachedReservation() usize {
    return @sizeOf(Output) + control_envelope_bytes + output_alignment_pad;
}

pub fn replacementPeakReservation() usize {
    return candidateReservation() + detachedReservation();
}
comptime {
    if (@sizeOf(Output) > 65_536 or @sizeOf(Scratch) > 65_536) @compileError("strategic allocation ceiling exceeded");
    if (@sizeOf(t.System) > 256 or @sizeOf(t.Planet) > 256 or @sizeOf(t.Civilization) > 256)
        @compileError("strategic record work bound exceeded");
    if (candidateReservation() > 131_072) @compileError("strategic candidate reservation exceeded");
    if (detachedReservation() > 65_536) @compileError("strategic detached reservation exceeded");
    if (replacementPeakReservation() > 196_608) @compileError("strategic replacement peak exceeded");
}
