//! Finite presentation camera state. Scope identity is separate from authority.
use crate::living_navigation::LivingEntitySelection;
use glam::Vec2;
use nyon_workshop_core::living::LivingEntityIdV2;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum LivingCameraLevel {
    Galaxy,
    System,
    World,
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LivingCameraScope {
    pub level: LivingCameraLevel,
    pub entity: Option<LivingEntityIdV2>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingCameraState {
    pub pan: Vec2,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub zoom: f32,
    pub focus_target: Option<LivingEntitySelection>,
}
impl Default for LivingCameraState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            yaw_degrees: 0.0,
            pitch_degrees: 55.0,
            zoom: 1.0,
            focus_target: None,
        }
    }
}
impl LivingCameraState {
    pub fn pan(&mut self, delta: Vec2) -> bool {
        if !delta.is_finite() {
            return false;
        }
        self.pan = (self.pan + delta).clamp(Vec2::splat(-4096.0), Vec2::splat(4096.0));
        true
    }
    pub fn orbit(&mut self, degrees: Vec2) -> bool {
        if !degrees.is_finite() {
            return false;
        }
        self.yaw_degrees = (self.yaw_degrees + degrees.x).clamp(-45.0, 45.0);
        self.pitch_degrees = (self.pitch_degrees + degrees.y).clamp(35.0, 70.0);
        true
    }
    pub fn zoom_at(&mut self, factor: f32, point: Vec2) -> bool {
        if !factor.is_finite() || factor <= 0.0 || !point.is_finite() {
            return false;
        }
        let zoom = (self.zoom * factor).clamp(0.25, 8.0);
        let ratio = zoom / self.zoom;
        self.pan =
            (point - (point - self.pan) * ratio).clamp(Vec2::splat(-4096.0), Vec2::splat(4096.0));
        self.zoom = zoom;
        true
    }
}
