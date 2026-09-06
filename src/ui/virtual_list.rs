//! Saturating row windows for bounded Workshop drawers.

use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VisibleWindow {
    pub start: usize,
    pub length: usize,
    pub total: usize,
}

impl VisibleWindow {
    pub fn new(total: usize, length: usize, start: usize) -> Self {
        let length = length.min(total);
        let maximum_start = total.saturating_sub(length);
        Self {
            start: start.min(maximum_start),
            length,
            total,
        }
    }

    pub fn range(self) -> Range<usize> {
        self.start..self.start.saturating_add(self.length).min(self.total)
    }

    pub fn reveal(&mut self, index: usize) {
        if self.length == 0 || self.total == 0 {
            self.start = 0;
            return;
        }
        let index = index.min(self.total - 1);
        if index < self.start {
            self.start = index;
        } else if index >= self.start.saturating_add(self.length) {
            self.start = index.saturating_add(1).saturating_sub(self.length);
        }
        self.start = self.start.min(self.total.saturating_sub(self.length));
    }

    pub fn scroll_rows(&mut self, delta: isize) {
        let maximum_start = self.total.saturating_sub(self.length);
        self.start = if delta >= 0 {
            self.start.saturating_add(delta as usize).min(maximum_start)
        } else {
            self.start.saturating_sub(delta.unsigned_abs())
        };
    }
}
