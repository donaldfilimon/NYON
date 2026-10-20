//! Command admission ported from `zig/src/play_role.zig`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilizationId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Role {
    Creator = 0,
    Ruler = 1,
    Combined = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Provenance {
    Creator = 0,
    Ruler = 1,
    Autonomous = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AdmissionError {
    #[error("a civilization is required")]
    CivilizationRequired,
    #[error("creator commands are forbidden")]
    CreatorForbidden,
    #[error("ruler commands are forbidden")]
    RulerForbidden,
    #[error("creator commands cannot specify an actor")]
    InvalidActor,
    #[error("actor differs from the selected civilization")]
    WrongCivilization,
    #[error("autonomous commands are internal only")]
    InternalCommandOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionPolicy {
    role: Role,
    civilization: Option<CivilizationId>,
}

impl SessionPolicy {
    pub fn new(role: Role, civilization: Option<CivilizationId>) -> Result<Self, AdmissionError> {
        if role != Role::Creator && civilization.is_none() {
            return Err(AdmissionError::CivilizationRequired);
        }
        Ok(Self { role, civilization })
    }

    pub const fn role(self) -> Role { self.role }

    pub const fn civilization(self) -> Option<CivilizationId> { self.civilization }

    pub fn admit(self, provenance: Provenance, actor: Option<CivilizationId>) -> Result<(), AdmissionError> {
        match provenance {
            Provenance::Creator => {
                if self.role == Role::Ruler { return Err(AdmissionError::CreatorForbidden); }
                if actor.is_some() { return Err(AdmissionError::InvalidActor); }
            }
            Provenance::Ruler => {
                if self.role == Role::Creator { return Err(AdmissionError::RulerForbidden); }
                let selected = self.civilization.ok_or(AdmissionError::CivilizationRequired)?;
                if actor != Some(selected) { return Err(AdmissionError::WrongCivilization); }
            }
            Provenance::Autonomous => return Err(AdmissionError::InternalCommandOnly),
        }
        Ok(())
    }
}
