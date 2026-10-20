const t = @import("types.zig");
const Manifest = @import("manifest.zig");
const role = @import("../play_role.zig");

pub fn validate(manifest: *const Manifest, policy: role.SessionPolicy) t.Failure!void {
    _ = role.SessionPolicy.init(policy.role, policy.civilization) catch return error.InvalidPolicy;
    const config = try manifest.config();
    if (policy.civilization) |selected| {
        const id = @backingInt(selected);
        if (id < 1 or id > config.civilizations) return error.InvalidPolicy;
    }
}

pub fn admit(manifest: *const Manifest, policy: role.SessionPolicy, provenance: role.Provenance, actor: ?role.CivilizationId) t.Failure!void {
    policy.admit(provenance, actor) catch return error.InvalidPolicy;
    try validate(manifest, policy);
}
