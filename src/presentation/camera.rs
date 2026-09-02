use std::time::Duration;

use glam::{
    Mat4, Vec2, Vec3,
    camera::rh::{proj::directx::perspective, view::look_at_mat4},
};

use super::MotionPreference;

pub const DEFAULT_YAW_RADIANS: f32 = 0.0;
pub const DEFAULT_PITCH_RADIANS: f32 = 55.0_f32.to_radians();
pub const MIN_PITCH_RADIANS: f32 = 35.0_f32.to_radians();
pub const MAX_PITCH_RADIANS: f32 = 70.0_f32.to_radians();
pub const MIN_YAW_RADIANS: f32 = -45.0_f32.to_radians();
pub const MAX_YAW_RADIANS: f32 = 45.0_f32.to_radians();
pub const DEFAULT_DISTANCE: f32 = 15.0;
pub const MIN_DISTANCE: f32 = 8.0;
pub const MAX_DISTANCE: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraMatrices {
    pub view: Mat4,
    pub projection: Mat4,
    pub view_projection: Mat4,
    pub inverse_view_projection: Mat4,
}

impl CameraMatrices {
    pub fn is_finite(self) -> bool {
        self.view
            .to_cols_array()
            .iter()
            .all(|value| value.is_finite())
            && self
                .projection
                .to_cols_array()
                .iter()
                .all(|value| value.is_finite())
            && self
                .view_projection
                .to_cols_array()
                .iter()
                .all(|value| value.is_finite())
            && self
                .inverse_view_projection
                .to_cols_array()
                .iter()
                .all(|value| value.is_finite())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraState {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub target_yaw: f32,
    pub target_pitch: f32,
    pub target_distance: f32,
    pub current_matrices: CameraMatrices,
    pub target_matrices: CameraMatrices,
    pub logical_viewport: Vec2,
    pub motion: MotionPreference,
}

impl CameraState {
    pub fn new(logical_viewport: Vec2) -> Self {
        let logical_viewport = finite_viewport(logical_viewport);
        let matrices = camera_matrices(
            Vec3::ZERO,
            DEFAULT_YAW_RADIANS,
            DEFAULT_PITCH_RADIANS,
            DEFAULT_DISTANCE,
            logical_viewport,
        );
        Self {
            target: Vec3::ZERO,
            yaw: DEFAULT_YAW_RADIANS,
            pitch: DEFAULT_PITCH_RADIANS,
            distance: DEFAULT_DISTANCE,
            target_yaw: DEFAULT_YAW_RADIANS,
            target_pitch: DEFAULT_PITCH_RADIANS,
            target_distance: DEFAULT_DISTANCE,
            current_matrices: matrices,
            target_matrices: matrices,
            logical_viewport,
            motion: MotionPreference::Full,
        }
    }

    pub fn eye(&self) -> Vec3 {
        camera_eye(self.target, self.yaw, self.pitch, self.distance)
    }

    pub fn set_viewport(&mut self, logical_viewport: Vec2) {
        self.logical_viewport = finite_viewport(logical_viewport);
        self.rebuild_matrices();
    }

    pub fn set_motion(&mut self, motion: MotionPreference) {
        self.motion = motion;
        if motion == MotionPreference::Reduced {
            self.snap_to_target();
        }
    }

    pub fn orbit(&mut self, yaw_delta: f32, pitch_delta: f32) {
        if yaw_delta.is_finite() {
            self.target_yaw = (self.target_yaw + yaw_delta).clamp(MIN_YAW_RADIANS, MAX_YAW_RADIANS);
        }
        if pitch_delta.is_finite() {
            self.target_pitch =
                (self.target_pitch + pitch_delta).clamp(MIN_PITCH_RADIANS, MAX_PITCH_RADIANS);
        }
        self.target_matrices = camera_matrices(
            self.target,
            self.target_yaw,
            self.target_pitch,
            self.target_distance,
            self.logical_viewport,
        );
        if self.motion == MotionPreference::Reduced {
            self.snap_to_target();
        }
    }

    pub fn zoom(&mut self, logical_delta: f32) {
        if logical_delta.is_finite() {
            let factor = (-logical_delta * 0.0015).exp();
            self.target_distance =
                (self.target_distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
            self.target_matrices = camera_matrices(
                self.target,
                self.target_yaw,
                self.target_pitch,
                self.target_distance,
                self.logical_viewport,
            );
            if self.motion == MotionPreference::Reduced {
                self.snap_to_target();
            }
        }
    }

    pub fn reset(&mut self) {
        self.target_yaw = DEFAULT_YAW_RADIANS;
        self.target_pitch = DEFAULT_PITCH_RADIANS;
        self.target_distance = DEFAULT_DISTANCE;
        self.target_matrices = camera_matrices(
            self.target,
            self.target_yaw,
            self.target_pitch,
            self.target_distance,
            self.logical_viewport,
        );
        if self.motion == MotionPreference::Reduced {
            self.snap_to_target();
        }
    }

    pub fn update(&mut self, delta: Duration) {
        if self.motion == MotionPreference::Reduced {
            self.snap_to_target();
            return;
        }
        let seconds = delta.as_secs_f32().clamp(0.0, 0.25);
        let blend = 1.0 - (-12.0 * seconds).exp();
        self.yaw += (self.target_yaw - self.yaw) * blend;
        self.pitch += (self.target_pitch - self.pitch) * blend;
        self.distance += (self.target_distance - self.distance) * blend;
        self.rebuild_matrices();
    }

    fn snap_to_target(&mut self) {
        self.yaw = self.target_yaw;
        self.pitch = self.target_pitch;
        self.distance = self.target_distance;
        self.rebuild_matrices();
    }

    fn rebuild_matrices(&mut self) {
        self.current_matrices = camera_matrices(
            self.target,
            self.yaw,
            self.pitch,
            self.distance,
            self.logical_viewport,
        );
        self.target_matrices = camera_matrices(
            self.target,
            self.target_yaw,
            self.target_pitch,
            self.target_distance,
            self.logical_viewport,
        );
    }
}

fn finite_viewport(viewport: Vec2) -> Vec2 {
    Vec2::new(
        if viewport.x.is_finite() {
            viewport.x.max(1.0)
        } else {
            1.0
        },
        if viewport.y.is_finite() {
            viewport.y.max(1.0)
        } else {
            1.0
        },
    )
}

fn camera_eye(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Vec3 {
    let horizontal = pitch.cos() * distance;
    target
        + Vec3::new(
            yaw.sin() * horizontal,
            pitch.sin() * distance,
            yaw.cos() * horizontal,
        )
}

fn camera_matrices(
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    viewport: Vec2,
) -> CameraMatrices {
    let eye = camera_eye(target, yaw, pitch, distance);
    let view = look_at_mat4(eye, target, Vec3::Y);
    let aspect = (viewport.x / viewport.y).clamp(0.1, 10.0);
    let projection = perspective(45.0_f32.to_radians(), aspect, 0.1, 100.0);
    let view_projection = projection * view;
    CameraMatrices {
        view,
        projection,
        view_projection,
        inverse_view_projection: view_projection.inverse(),
    }
}
