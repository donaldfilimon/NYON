//! Non-color, presentation-only first-selection cue for Classic's tutorial.

use glam::Vec2;

use crate::{
    app::onboarding::OnboardingStep,
    engine::primitives::PrimitiveBatch,
    game::model::{Campaign, Faction, WorldId},
    presentation::{SceneFrame, ui::UiFrame},
};

#[derive(Clone, Debug, PartialEq)]
pub struct StartMarker {
    pub world: WorldId,
    pub world_name: String,
    pub point: Vec2,
    pub label_min: Vec2,
}

pub fn start_marker(
    step: Option<OnboardingStep>,
    campaign: &Campaign,
    scene: &SceneFrame,
    ui: &UiFrame,
) -> Option<StartMarker> {
    if step != Some(OnboardingStep::SelectUnionWorld) {
        return None;
    }
    scene
        .worlds
        .iter()
        .filter(|world| campaign.worlds[usize::from(world.id.0)].owner == Some(Faction::Union))
        .find_map(|world| {
            let clip = scene.camera.view_projection * world.position.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.0 {
                return None;
            }
            let ndc = clip.truncate() / clip.w;
            let point = Vec2::new(
                (ndc.x + 1.0) * ui.viewport.x * 0.5,
                (1.0 - ndc.y) * ui.viewport.y * 0.5,
            );
            if point.cmplt(Vec2::ZERO).any() || point.cmpgt(ui.viewport).any() {
                return None;
            }
            let label_min = Vec2::new(
                (point.x - 90.0).clamp(8.0, (ui.viewport.x - 188.0).max(8.0)),
                (point.y - 92.0)
                    .max(190.0)
                    .min((ui.viewport.y - 180.0).max(190.0)),
            );
            Some(StartMarker {
                world: world.id,
                world_name: campaign.worlds[usize::from(world.id.0)].name.to_string(),
                point,
                label_min,
            })
        })
}

impl StartMarker {
    pub fn draw(&self, batch: &mut PrimitiveBatch) {
        let accent = [1.0, 0.83, 0.22, 1.0];
        let edge = self.label_min + Vec2::new(90.0, 54.0);
        batch.line(edge, self.point, 3.0, accent);
        batch.ring(self.point, 22.0, 3.0, accent);
    }
}
