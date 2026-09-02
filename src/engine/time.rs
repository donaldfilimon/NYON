use std::time::Duration;

const NANOS_PER_SECOND: u128 = 1_000_000_000;
const MAX_FRAME_DELTA: Duration = Duration::from_millis(250);

pub struct FixedClock {
    tick_hz: u32,
    scaled_accumulator: u128,
    pending_steps: usize,
    max_steps: usize,
}

impl FixedClock {
    pub fn new(tick_hz: u32, max_steps: usize) -> Self {
        assert!(tick_hz > 0, "tick_hz must be positive");
        Self {
            tick_hz,
            scaled_accumulator: 0,
            pending_steps: 0,
            max_steps,
        }
    }

    pub fn push_frame(&mut self, delta: Duration) {
        let accepted_delta = delta.min(MAX_FRAME_DELTA);
        self.scaled_accumulator = self.scaled_accumulator.saturating_add(
            accepted_delta
                .as_nanos()
                .saturating_mul(self.tick_hz as u128),
        );

        let whole_steps = self.scaled_accumulator / NANOS_PER_SECOND;
        self.scaled_accumulator %= NANOS_PER_SECOND;
        let available = self.max_steps.saturating_sub(self.pending_steps);
        self.pending_steps = self
            .pending_steps
            .saturating_add((whole_steps as usize).min(available));
    }

    pub fn take_step(&mut self) -> bool {
        if self.pending_steps == 0 {
            return false;
        }
        self.pending_steps -= 1;
        true
    }

    pub fn alpha(&self) -> f32 {
        self.scaled_accumulator as f32 / NANOS_PER_SECOND as f32
    }

    pub fn clear(&mut self) {
        self.scaled_accumulator = 0;
        self.pending_steps = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::FixedClock;
    use std::time::Duration;

    #[test]
    fn fixed_clock_clamps_long_frames_and_limits_work() {
        let mut clock = FixedClock::new(60, 15);
        clock.push_frame(Duration::from_secs(2));
        let mut steps = 0;
        while clock.take_step() {
            steps += 1;
        }
        assert_eq!(steps, 15);
        assert!((0.0..1.0).contains(&clock.alpha()));
    }

    #[test]
    fn exact_sixtieth_carries_rational_remainder() {
        let mut clock = FixedClock::new(60, 15);
        clock.push_frame(Duration::from_nanos(16_666_666));
        assert!(!clock.take_step());
        assert_eq!(clock.alpha(), 999_999_960.0 / 1_000_000_000.0);
        clock.push_frame(Duration::from_nanos(1));
        assert!(clock.take_step());
        assert_eq!(clock.alpha(), 20.0 / 1_000_000_000.0);
    }

    #[test]
    fn clear_discards_pending_work_and_remainder() {
        let mut clock = FixedClock::new(60, 15);
        clock.push_frame(Duration::from_millis(100));
        clock.clear();
        assert!(!clock.take_step());
        assert_eq!(clock.alpha(), 0.0);
    }
}
