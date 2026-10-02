//! Platform-neutral mouse and touch gesture ownership.

use std::collections::BTreeMap;

use glam::Vec2;

use super::ui::UiAction;

pub const POINTER_GESTURE_THRESHOLD: f32 = 6.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerSource {
    Mouse,
    Touch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InteractionHit {
    Hud(UiAction),
    World,
    EmptyScene,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GestureAction {
    Hud(UiAction),
    SceneTap {
        position: Vec2,
        source: PointerSource,
    },
    ImmediateLaunch {
        position: Vec2,
    },
    Orbit {
        delta: Vec2,
    },
    Zoom {
        logical_delta: f32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GestureOwner {
    Hud(UiAction),
    TapOnly,
    OrbitOrTap,
    ImmediateLaunch,
    Pinch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Contact {
    source: PointerSource,
    owner: GestureOwner,
    start: Vec2,
    last: Vec2,
    moved: bool,
}

#[derive(Clone, Debug, Default)]
pub struct InteractionController {
    contacts: BTreeMap<u64, Contact>,
    pinch_distance: Option<f32>,
}

impl InteractionController {
    pub fn begin(
        &mut self,
        id: u64,
        position: Vec2,
        source: PointerSource,
        button: PointerButton,
        hit: InteractionHit,
    ) -> bool {
        if !position.is_finite() || self.contacts.contains_key(&id) {
            return false;
        }
        let owner = match hit {
            InteractionHit::Hud(action) => GestureOwner::Hud(action),
            InteractionHit::World
                if source == PointerSource::Mouse && button == PointerButton::Primary =>
            {
                GestureOwner::TapOnly
            }
            InteractionHit::EmptyScene if button == PointerButton::Primary => {
                GestureOwner::OrbitOrTap
            }
            InteractionHit::World | InteractionHit::EmptyScene
                if button == PointerButton::Secondary =>
            {
                GestureOwner::ImmediateLaunch
            }
            InteractionHit::World | InteractionHit::EmptyScene => GestureOwner::OrbitOrTap,
        };
        self.contacts.insert(
            id,
            Contact {
                source,
                owner,
                start: position,
                last: position,
                moved: false,
            },
        );

        if source == PointerSource::Touch {
            self.begin_pinch_if_ready();
        }
        true
    }

    pub fn update(&mut self, id: u64, position: Vec2) -> Option<GestureAction> {
        if !position.is_finite() {
            return None;
        }
        let contact = self.contacts.get_mut(&id)?;
        let delta = position - contact.last;
        contact.last = position;
        if position.distance(contact.start) >= POINTER_GESTURE_THRESHOLD {
            contact.moved = true;
        }
        let owner = contact.owner;

        if owner == GestureOwner::Pinch {
            let distance = self.touch_distance()?;
            let previous = self.pinch_distance.replace(distance)?;
            // Coincident fingers have no usable scale. Keep the new baseline
            // so separating them again resumes without a jump.
            if !distance.is_finite()
                || !previous.is_finite()
                || distance <= f32::EPSILON
                || previous <= f32::EPSILON
            {
                return None;
            }
            return Some(GestureAction::Zoom {
                // Camera zoom uses positive deltas to move closer.
                logical_delta: (distance.ln() - previous.ln()) * 600.0,
            });
        }
        if owner == GestureOwner::OrbitOrTap && contact.moved {
            return Some(GestureAction::Orbit { delta });
        }
        None
    }

    pub fn end(
        &mut self,
        id: u64,
        position: Vec2,
        hud_hit: Option<UiAction>,
    ) -> Option<GestureAction> {
        let mut contact = self.contacts.remove(&id)?;
        if position.is_finite() {
            contact.last = position;
            contact.moved |= position.distance(contact.start) >= POINTER_GESTURE_THRESHOLD;
        }
        if contact.owner == GestureOwner::Pinch {
            self.pinch_distance = None;
            for remaining in self.contacts.values_mut() {
                if remaining.owner == GestureOwner::Pinch {
                    remaining.owner = GestureOwner::TapOnly;
                    remaining.moved = true;
                }
            }
            return None;
        }
        match contact.owner {
            GestureOwner::Hud(action) if !contact.moved && hud_hit == Some(action) => {
                Some(GestureAction::Hud(action))
            }
            GestureOwner::TapOnly if !contact.moved => Some(GestureAction::SceneTap {
                position: contact.last,
                source: contact.source,
            }),
            GestureOwner::OrbitOrTap if !contact.moved => Some(GestureAction::SceneTap {
                position: contact.last,
                source: contact.source,
            }),
            GestureOwner::ImmediateLaunch if !contact.moved => {
                Some(GestureAction::ImmediateLaunch {
                    position: contact.last,
                })
            }
            GestureOwner::Hud(_)
            | GestureOwner::TapOnly
            | GestureOwner::OrbitOrTap
            | GestureOwner::ImmediateLaunch
            | GestureOwner::Pinch => None,
        }
    }

    pub fn cancel(&mut self) {
        self.contacts.clear();
        self.pinch_distance = None;
    }

    pub fn has_contact(&self, id: u64) -> bool {
        self.contacts.contains_key(&id)
    }

    fn begin_pinch_if_ready(&mut self) {
        let touch_ids: Vec<_> = self
            .contacts
            .iter()
            .filter_map(|(id, contact)| {
                (contact.source == PointerSource::Touch
                    && !matches!(contact.owner, GestureOwner::Hud(_)))
                .then_some(*id)
            })
            .collect();
        if touch_ids.len() != 2 {
            return;
        }
        for id in &touch_ids {
            if let Some(contact) = self.contacts.get_mut(id) {
                contact.owner = GestureOwner::Pinch;
                contact.moved = true;
            }
        }
        self.pinch_distance = self.touch_distance();
    }

    fn touch_distance(&self) -> Option<f32> {
        let mut points = self
            .contacts
            .values()
            .filter(|contact| contact.owner == GestureOwner::Pinch)
            .map(|contact| contact.last);
        let first = points.next()?;
        let second = points.next()?;
        (points.next().is_none()).then(|| first.distance(second))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spreading_fingers_zooms_in_and_pinching_zooms_out() {
        let mut controller = InteractionController::default();
        let mut camera = crate::presentation::camera::CameraState::new(Vec2::new(800.0, 600.0));
        for (id, position) in [(1, Vec2::ZERO), (2, Vec2::new(100.0, 0.0))] {
            controller.begin(
                id,
                position,
                PointerSource::Touch,
                PointerButton::Primary,
                InteractionHit::EmptyScene,
            );
        }
        let initial_distance = camera.target_distance;
        let Some(GestureAction::Zoom { logical_delta }) =
            controller.update(2, Vec2::new(200.0, 0.0))
        else {
            panic!("spreading fingers must zoom");
        };
        camera.zoom(logical_delta);
        assert!(camera.target_distance < initial_distance);
        let Some(GestureAction::Zoom { logical_delta }) =
            controller.update(2, Vec2::new(100.0, 0.0))
        else {
            panic!("pinching fingers must zoom");
        };
        camera.zoom(logical_delta);
        assert!((camera.target_distance - initial_distance).abs() < 0.0001);
        assert!(camera.target_matrices.is_finite());
    }

    #[test]
    fn coincident_fingers_resume_from_a_finite_baseline() {
        let mut controller = InteractionController::default();
        for (id, position) in [(1, Vec2::ZERO), (2, Vec2::new(100.0, 0.0))] {
            controller.begin(
                id,
                position,
                PointerSource::Touch,
                PointerButton::Primary,
                InteractionHit::EmptyScene,
            );
        }
        assert_eq!(controller.update(2, Vec2::ZERO), None);
        assert_eq!(controller.update(2, Vec2::new(100.0, 0.0)), None);
        let Some(GestureAction::Zoom { logical_delta }) =
            controller.update(2, Vec2::new(120.0, 0.0))
        else {
            panic!("zoom must resume after fingers separate");
        };
        assert!(logical_delta.is_finite() && logical_delta > 0.0);
        assert_eq!(controller.end(2, Vec2::new(120.0, 0.0), None), None);
        assert_eq!(controller.end(1, Vec2::ZERO, None), None);
    }

    #[test]
    fn hud_owns_the_gesture_and_drag_never_activates_it() {
        let mut controller = InteractionController::default();
        let action = UiAction::Launch;
        assert!(controller.begin(
            1,
            Vec2::new(10.0, 10.0),
            PointerSource::Touch,
            PointerButton::Primary,
            InteractionHit::Hud(action),
        ));
        controller.update(1, Vec2::new(30.0, 10.0));
        assert_eq!(controller.end(1, Vec2::new(30.0, 10.0), Some(action)), None);
    }

    #[test]
    fn touch_world_tap_is_not_an_immediate_launch() {
        let mut controller = InteractionController::default();
        controller.begin(
            7,
            Vec2::new(100.0, 100.0),
            PointerSource::Touch,
            PointerButton::Primary,
            InteractionHit::World,
        );
        assert!(matches!(
            controller.end(7, Vec2::new(101.0, 100.0), None),
            Some(GestureAction::SceneTap {
                source: PointerSource::Touch,
                ..
            })
        ));
    }

    #[test]
    fn touch_drag_from_a_world_orbits_after_the_shared_threshold() {
        let mut controller = InteractionController::default();
        assert!(controller.begin(
            8,
            Vec2::new(100.0, 100.0),
            PointerSource::Touch,
            PointerButton::Primary,
            InteractionHit::World,
        ));
        assert_eq!(
            controller.update(8, Vec2::new(100.0 + POINTER_GESTURE_THRESHOLD - 0.1, 100.0),),
            None
        );
        assert!(matches!(
            controller.update(8, Vec2::new(100.0 + POINTER_GESTURE_THRESHOLD + 1.0, 100.0),),
            Some(GestureAction::Orbit { .. })
        ));
        assert_eq!(
            controller.end(
                8,
                Vec2::new(100.0 + POINTER_GESTURE_THRESHOLD + 1.0, 100.0),
                None,
            ),
            None
        );
    }
}
