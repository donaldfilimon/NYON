//! Durable-exit state machine: decides whether a quit request must wait for the
//! workshop to save, and lets a pending exit be cancelled. Pure; no I/O.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct DurableExitState {
    pending: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DurableExitAction {
    ExitNow,
    PrepareWorkshop,
    Wait,
}

impl DurableExitState {
    pub(super) const fn is_pending(self) -> bool {
        self.pending
    }

    pub(super) fn request(&mut self, workshop_requires_save: bool) -> DurableExitAction {
        if !workshop_requires_save {
            return DurableExitAction::ExitNow;
        }
        if self.pending {
            return DurableExitAction::Wait;
        }
        self.pending = true;
        DurableExitAction::PrepareWorkshop
    }

    pub(super) fn observe_workshop(&mut self, continue_ready: bool) -> DurableExitAction {
        if !self.pending {
            return DurableExitAction::Wait;
        }
        if continue_ready {
            self.pending = false;
            DurableExitAction::ExitNow
        } else {
            DurableExitAction::Wait
        }
    }

    pub(super) fn cancel(&mut self) -> bool {
        std::mem::take(&mut self.pending)
    }
}
