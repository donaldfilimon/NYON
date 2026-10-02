//! At most two scene contacts produce presentation gestures in logical pixels.
use glam::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LivingCameraGesture {
    pub pan: Vec2,
    pub zoom: Option<(f32, Vec2)>,
}

#[derive(Clone, Debug, Default)]
pub struct LivingCameraTouches {
    contacts: Vec<(u64, Vec2)>,
}

impl LivingCameraTouches {
    pub fn contains(&self, id: u64) -> bool {
        self.contacts.iter().any(|(contact, _)| *contact == id)
    }
    pub fn is_empty(&self) -> bool {
        self.contacts.is_empty()
    }
    pub fn clear(&mut self) {
        self.contacts.clear();
    }
    pub fn begin(&mut self, id: u64, point: Vec2) -> bool {
        if !point.is_finite() || self.contains(id) || self.contacts.len() == 2 {
            return false;
        }
        self.contacts.push((id, point));
        true
    }
    pub fn end(&mut self, id: u64) {
        self.contacts.retain(|(contact, _)| *contact != id);
    }
    pub fn update(&mut self, id: u64, point: Vec2) -> Option<LivingCameraGesture> {
        if !point.is_finite() {
            return None;
        }
        let index = self
            .contacts
            .iter()
            .position(|(contact, _)| *contact == id)?;
        let center = |contacts: &[(u64, Vec2)]| {
            contacts
                .iter()
                .fold(Vec2::ZERO, |sum, (_, point)| sum + *point)
                / contacts.len() as f32
        };
        let old_center = center(&self.contacts);
        let old_distance =
            (self.contacts.len() == 2).then(|| self.contacts[0].1.distance(self.contacts[1].1));
        let mut next = self.contacts.clone();
        next[index].1 = point;
        let new_center = center(&next);
        let pan = new_center - old_center;
        if !pan.is_finite() || !new_center.is_finite() {
            return None;
        }
        let zoom = old_distance.and_then(|distance| {
            let next_distance = next[0].1.distance(next[1].1);
            if distance > 1.0 && distance.is_finite() && next_distance.is_finite() {
                Some(((next_distance / distance).clamp(0.25, 4.0), new_center))
            } else {
                None
            }
        });
        self.contacts = next;
        Some(LivingCameraGesture { pan, zoom })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_contacts_pan_and_pinch_then_remaining_contact_continues_without_jump() {
        let mut touches = LivingCameraTouches::default();
        assert!(touches.begin(9, Vec2::new(10.0, 10.0)));
        assert!(touches.begin(4, Vec2::new(30.0, 10.0)));
        assert!(!touches.begin(3, Vec2::ZERO));
        let gesture = touches.update(4, Vec2::new(50.0, 10.0)).unwrap();
        assert_eq!(gesture.pan, Vec2::new(10.0, 0.0));
        assert_eq!(gesture.zoom, Some((2.0, Vec2::new(30.0, 10.0))));
        touches.end(9);
        let gesture = touches.update(4, Vec2::new(55.0, 15.0)).unwrap();
        assert_eq!(gesture.pan, Vec2::splat(5.0));
        assert_eq!(gesture.zoom, None);
        touches.end(4);
        assert!(touches.is_empty());
    }

    #[test]
    fn invalid_and_unregistered_contacts_preserve_the_gesture_baseline() {
        let mut touches = LivingCameraTouches::default();
        assert!(!touches.begin(1, Vec2::splat(f32::NAN)));
        touches.begin(1, Vec2::new(10.0, 20.0));
        assert!(!touches.begin(1, Vec2::ZERO));
        assert_eq!(touches.update(8, Vec2::ZERO), None);
        assert_eq!(touches.update(1, Vec2::splat(f32::INFINITY)), None);
        assert_eq!(
            touches.update(1, Vec2::new(15.0, 25.0)).unwrap().pan,
            Vec2::splat(5.0)
        );
        touches.clear();
        assert_eq!(touches.update(1, Vec2::ZERO), None);
    }

    #[test]
    fn coincident_contacts_do_not_publish_an_invalid_zoom() {
        let mut touches = LivingCameraTouches::default();
        touches.begin(1, Vec2::ZERO);
        touches.begin(2, Vec2::ZERO);
        assert_eq!(touches.update(2, Vec2::X).unwrap().zoom, None);
    }
}
