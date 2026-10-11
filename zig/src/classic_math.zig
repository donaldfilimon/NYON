//! Portable scalar presentation math. Column storage, strict staged arithmetic,
//! unchecked inverse semantics; no SIMD/GPU ABI or cross-libm bit guarantee.
const std = @import("std");
pub const Vec2 = struct {
    x: f32,
    y: f32,
    pub fn isFinite(v: Vec2) bool {
        return std.math.isFinite(v.x) and std.math.isFinite(v.y);
    }
};
pub const Vec3 = struct {
    x: f32,
    y: f32,
    z: f32,
    pub const zero: Vec3 = .{ .x = 0, .y = 0, .z = 0 };
    pub fn isFinite(v: Vec3) bool {
        return std.math.isFinite(v.x) and std.math.isFinite(v.y) and std.math.isFinite(v.z);
    }
};
pub const Vec4 = struct {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    pub fn isFinite(v: Vec4) bool {
        return std.math.isFinite(v.x) and std.math.isFinite(v.y) and std.math.isFinite(v.z) and std.math.isFinite(v.w);
    }
};
pub fn add(a: Vec3, b: Vec3) Vec3 {
    @setFloatMode(.strict);
    return .{ .x = a.x + b.x, .y = a.y + b.y, .z = a.z + b.z };
}
pub fn sub(a: Vec3, b: Vec3) Vec3 {
    @setFloatMode(.strict);
    return .{ .x = a.x - b.x, .y = a.y - b.y, .z = a.z - b.z };
}
pub fn scale(a: Vec3, s: f32) Vec3 {
    @setFloatMode(.strict);
    return .{ .x = a.x * s, .y = a.y * s, .z = a.z * s };
}
pub fn dot3(a: Vec3, b: Vec3) f32 {
    @setFloatMode(.strict);
    return (a.x * b.x + a.y * b.y) + a.z * b.z;
}
pub fn cross(a: Vec3, b: Vec3) Vec3 {
    @setFloatMode(.strict);
    return .{ .x = a.y * b.z - a.z * b.y, .y = a.z * b.x - a.x * b.z, .z = a.x * b.y - a.y * b.x };
}
pub fn lengthSquared(a: Vec3) f32 {
    @setFloatMode(.strict);
    return dot3(a, a);
}
pub fn distance(a: Vec3, b: Vec3) f32 {
    @setFloatMode(.strict);
    return @sqrt(lengthSquared(sub(a, b)));
}
pub fn tryNormalize(a: Vec3) ?Vec3 {
    @setFloatMode(.strict);
    const reciprocal = 1 / @sqrt(lengthSquared(a));
    return if (std.math.isFinite(reciprocal) and reciprocal > 0) scale(a, reciprocal) else null;
}
pub fn clamp(value: f32, minimum: f32, maximum: f32) f32 {
    @setFloatMode(.strict);
    // Preserve NaN from public camera field mutation, like Rust f32::clamp.
    return if (value < minimum) minimum else if (value > maximum) maximum else value;
}
pub fn finiteFloor(value: f32, minimum: f32) f32 {
    @setFloatMode(.strict);
    return if (std.math.isFinite(value)) @max(value, minimum) else minimum;
}
pub fn radians(degrees: f32) f32 {
    @setFloatMode(.strict);
    return degrees * @as(f32, std.math.pi / 180.0);
}
pub fn totalOrder(a: f32, b: f32) std.math.Order {
    const ab: u32 = @bitCast(a);
    const bb: u32 = @bitCast(b);
    const ak = if (ab & 0x80000000 != 0) ~ab else ab ^ 0x80000000;
    const bk = if (bb & 0x80000000 != 0) ~bb else bb ^ 0x80000000;
    return std.math.order(ak, bk);
}
fn v4(x: f32, y: f32, z: f32, w: f32) Vec4 {
    return .{ .x = x, .y = y, .z = z, .w = w };
}
fn mul4(a: Vec4, b: Vec4) Vec4 {
    @setFloatMode(.strict);
    return v4(a.x * b.x, a.y * b.y, a.z * b.z, a.w * b.w);
}
fn sub4(a: Vec4, b: Vec4) Vec4 {
    @setFloatMode(.strict);
    return v4(a.x - b.x, a.y - b.y, a.z - b.z, a.w - b.w);
}
fn add4(a: Vec4, b: Vec4) Vec4 {
    @setFloatMode(.strict);
    return v4(a.x + b.x, a.y + b.y, a.z + b.z, a.w + b.w);
}
fn scale4(a: Vec4, s: f32) Vec4 {
    @setFloatMode(.strict);
    return mul4(a, v4(s, s, s, s));
}
pub const Mat4 = struct {
    cols: [4]Vec4,
    pub fn identity() Mat4 {
        return fromColumns(.{ v4(1, 0, 0, 0), v4(0, 1, 0, 0), v4(0, 0, 1, 0), v4(0, 0, 0, 1) });
    }
    pub fn fromColumns(cols: [4]Vec4) Mat4 {
        return .{ .cols = cols };
    }
    pub fn toColumns(a: Mat4) [4]Vec4 {
        return a.cols;
    }
    pub fn isFinite(a: Mat4) bool {
        for (a.cols) |c| {
            if (!c.isFinite()) return false;
        }
        return true;
    }
    pub fn mulVec4(a: Mat4, b: Vec4) Vec4 {
        @setFloatMode(.strict);
        return add4(add4(add4(scale4(a.cols[0], b.x), scale4(a.cols[1], b.y)), scale4(a.cols[2], b.z)), scale4(a.cols[3], b.w));
    }
    pub fn mulMat4(a: Mat4, b: Mat4) Mat4 {
        @setFloatMode(.strict);
        return fromColumns(.{ a.mulVec4(b.cols[0]), a.mulVec4(b.cols[1]), a.mulVec4(b.cols[2]), a.mulVec4(b.cols[3]) });
    }
    pub fn inverseUnchecked(a: Mat4) Mat4 {
        @setFloatMode(.strict);
        // Reviewed scalar adjugate/cofactor sequence. No determinant admission,
        // singular fallback, scaling repair, or optional glam assertion emulation.
        const m00 = a.cols[0].x;
        const m01 = a.cols[0].y;
        const m02 = a.cols[0].z;
        const m03 = a.cols[0].w;
        const m10 = a.cols[1].x;
        const m11 = a.cols[1].y;
        const m12 = a.cols[1].z;
        const m13 = a.cols[1].w;
        const m20 = a.cols[2].x;
        const m21 = a.cols[2].y;
        const m22 = a.cols[2].z;
        const m23 = a.cols[2].w;
        const m30 = a.cols[3].x;
        const m31 = a.cols[3].y;
        const m32 = a.cols[3].z;
        const m33 = a.cols[3].w;
        const c00 = m22 * m33 - m32 * m23;
        const c02 = m12 * m33 - m32 * m13;
        const c03 = m12 * m23 - m22 * m13;
        const c04 = m21 * m33 - m31 * m23;
        const c06 = m11 * m33 - m31 * m13;
        const c07 = m11 * m23 - m21 * m13;
        const c08 = m21 * m32 - m31 * m22;
        const c10 = m11 * m32 - m31 * m12;
        const c11 = m11 * m22 - m21 * m12;
        const c12 = m20 * m33 - m30 * m23;
        const c14 = m10 * m33 - m30 * m13;
        const c15 = m10 * m23 - m20 * m13;
        const c16 = m20 * m32 - m30 * m22;
        const c18 = m10 * m32 - m30 * m12;
        const c19 = m10 * m22 - m20 * m12;
        const c20 = m20 * m31 - m30 * m21;
        const c22 = m10 * m31 - m30 * m11;
        const c23 = m10 * m21 - m20 * m11;
        const f0 = v4(c00, c00, c02, c03);
        const f1 = v4(c04, c04, c06, c07);
        const f2 = v4(c08, c08, c10, c11);
        const f3 = v4(c12, c12, c14, c15);
        const f4 = v4(c16, c16, c18, c19);
        const f5 = v4(c20, c20, c22, c23);
        const v0 = v4(m10, m00, m00, m00);
        const v1 = v4(m11, m01, m01, m01);
        const v2 = v4(m12, m02, m02, m02);
        const v3 = v4(m13, m03, m03, m03);
        const inv0 = add4(sub4(mul4(v1, f0), mul4(v2, f1)), mul4(v3, f2));
        const inv1 = add4(sub4(mul4(v0, f0), mul4(v2, f3)), mul4(v3, f4));
        const inv2 = add4(sub4(mul4(v0, f1), mul4(v1, f3)), mul4(v3, f5));
        const inv3 = add4(sub4(mul4(v0, f2), mul4(v1, f4)), mul4(v2, f5));
        const c0 = mul4(inv0, v4(1, -1, 1, -1));
        const c1 = mul4(inv1, v4(-1, 1, -1, 1));
        const c2 = mul4(inv2, v4(1, -1, 1, -1));
        const c3 = mul4(inv3, v4(-1, 1, -1, 1));
        const d = mul4(a.cols[0], v4(c0.x, c1.x, c2.x, c3.x));
        const determinant = ((d.x + d.y) + d.z) + d.w;
        const reciprocal = 1 / determinant;
        return fromColumns(.{ scale4(c0, reciprocal), scale4(c1, reciprocal), scale4(c2, reciprocal), scale4(c3, reciprocal) });
    }
};
pub fn lookAtRH(eye: Vec3, center: Vec3, up: Vec3) Mat4 {
    @setFloatMode(.strict);
    // Unchecked basis construction preserves invalid generic inputs as invalid.
    const f = scale(sub(center, eye), 1 / @sqrt(lengthSquared(sub(center, eye))));
    const c = cross(f, up);
    const s = scale(c, 1 / @sqrt(lengthSquared(c)));
    const u = cross(s, f);
    return Mat4.fromColumns(.{ v4(s.x, u.x, -f.x, 0), v4(s.y, u.y, -f.y, 0), v4(s.z, u.z, -f.z, 0), v4(-dot3(eye, s), -dot3(eye, u), dot3(eye, f), 1) });
}
pub fn perspectiveRHZO(fov: f32, aspect: f32, near: f32, far: f32) Mat4 {
    @setFloatMode(.strict);
    const half = fov * 0.5;
    const h = @cos(half) / @sin(half);
    const inverse_range = 1 / (far - near);
    return Mat4.fromColumns(.{ v4(h / aspect, 0, 0, 0), v4(0, h, 0, 0), v4(0, 0, -far * inverse_range, -1), v4(0, 0, (-near * far) * inverse_range, 0) });
}
