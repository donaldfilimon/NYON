use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::game::model::WorldId;

use super::SceneWorld;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneRay {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl SceneRay {
    pub fn from_logical_pointer(
        pointer: Vec2,
        logical_viewport: Vec2,
        inverse_view_projection: Mat4,
    ) -> Option<Self> {
        if !pointer.is_finite()
            || !logical_viewport.is_finite()
            || logical_viewport.x <= 0.0
            || logical_viewport.y <= 0.0
        {
            return None;
        }
        let x = pointer.x * 2.0 / logical_viewport.x - 1.0;
        let y = 1.0 - pointer.y * 2.0 / logical_viewport.y;
        let near = unproject(inverse_view_projection, Vec3::new(x, y, 0.0))?;
        let far = unproject(inverse_view_projection, Vec3::new(x, y, 1.0))?;
        let direction = (far - near).try_normalize()?;
        Some(Self {
            origin: near,
            direction,
        })
    }

    pub fn sphere_distance(self, center: Vec3, radius: f32) -> Option<f32> {
        if !self.origin.is_finite()
            || !self.direction.is_finite()
            || !center.is_finite()
            || !radius.is_finite()
            || radius <= 0.0
        {
            return None;
        }
        let offset = self.origin - center;
        let half_b = offset.dot(self.direction);
        let c = offset.length_squared() - radius * radius;
        let discriminant = half_b * half_b - c;
        if discriminant < 0.0 {
            return None;
        }
        let root = discriminant.sqrt();
        let near = -half_b - root;
        let far = -half_b + root;
        if near > 0.0 {
            Some(near)
        } else if far > 0.0 {
            Some(far)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldHit {
    pub world: WorldId,
    pub distance: f32,
}

pub fn pick_world(ray: SceneRay, worlds: &[SceneWorld]) -> Option<WorldHit> {
    worlds
        .iter()
        .filter_map(|world| {
            ray.sphere_distance(world.position, world.radius)
                .map(|distance| WorldHit {
                    world: world.id,
                    distance,
                })
        })
        .min_by(|left, right| {
            left.distance
                .total_cmp(&right.distance)
                .then_with(|| left.world.cmp(&right.world))
        })
}

pub fn pick_world_after_hud(
    hud_consumed: bool,
    ray: SceneRay,
    worlds: &[SceneWorld],
) -> Option<WorldHit> {
    (!hud_consumed).then(|| pick_world(ray, worlds)).flatten()
}

fn unproject(inverse_view_projection: Mat4, point: Vec3) -> Option<Vec3> {
    let homogeneous = inverse_view_projection * Vec4::new(point.x, point.y, point.z, 1.0);
    if !homogeneous.is_finite() || homogeneous.w.abs() <= f32::EPSILON {
        return None;
    }
    let world = homogeneous.truncate() / homogeneous.w;
    world.is_finite().then_some(world)
}
