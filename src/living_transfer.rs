//! File handoff is presentation, independent of Living storage transactions.
//! Import bytes are always validated by the authority before installation.

use nyon_workshop_core::living::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LIVING_MAX_NOTEBOOK_BYTES_V2, LIVING_MAX_PACK_BYTES_V2,
};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingTransferKind {
    Archive,
    Catalog,
    Notebook,
}

impl LivingTransferKind {
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::Archive => LIVING_MAX_ARCHIVE_BYTES_V2,
            Self::Catalog => LIVING_MAX_PACK_BYTES_V2,
            Self::Notebook => LIVING_MAX_NOTEBOOK_BYTES_V2,
        }
    }
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Archive => ".nyonliving.json",
            Self::Catalog => ".nyonlivingpack.json",
            Self::Notebook => ".nyonlivingnotebook.json",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LivingTransferRequest {
    Import(LivingTransferKind),
    Export {
        kind: LivingTransferKind,
        suggested_name: String,
        bytes: Vec<u8>,
    },
}

impl LivingTransferRequest {
    pub fn validate(&self) -> Result<(), LivingTransferError> {
        if let Self::Export {
            kind,
            suggested_name,
            bytes,
        } = self
        {
            if bytes.is_empty() || bytes.len() > kind.max_bytes() {
                return Err(LivingTransferError::TooLarge);
            }
            if suggested_name.is_empty()
                || suggested_name.len() > 96
                || !suggested_name.ends_with(kind.suffix())
                || suggested_name.bytes().any(|byte| {
                    !byte.is_ascii_alphanumeric() && !matches!(byte, b'-' | b'_' | b'.')
                })
            {
                return Err(LivingTransferError::InvalidName);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingExportOutcome {
    DurablySaved,
    DownloadStarted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LivingTransferOutcome {
    Imported {
        kind: LivingTransferKind,
        bytes: Vec<u8>,
    },
    Exported(LivingExportOutcome),
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingTransferError {
    #[error("file transfer already active")]
    Busy,
    #[error("file exceeds Living transfer limit")]
    TooLarge,
    #[error("invalid export file name")]
    InvalidName,
    #[error("platform file handoff unavailable")]
    Unavailable,
    #[error("platform file handoff failed")]
    Platform,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LivingTransferStatus {
    Idle,
    Pending,
    Ready(Result<LivingTransferOutcome, LivingTransferError>),
}

/// Begin must be called directly from a user activation for browser gestures.
pub trait LivingTransfer {
    fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError>;
    fn poll(&mut self) -> LivingTransferStatus;
    fn cancel(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transfer_rejects_paths_and_empty_or_excessive_payloads() {
        for name in [
            "../galaxy.nyonliving.json",
            "galaxy/other.nyonliving.json",
            "galaxy.json",
        ] {
            assert_eq!(
                LivingTransferRequest::Export {
                    kind: LivingTransferKind::Archive,
                    suggested_name: name.into(),
                    bytes: vec![1]
                }
                .validate(),
                Err(LivingTransferError::InvalidName)
            );
        }
        assert_eq!(
            LivingTransferRequest::Export {
                kind: LivingTransferKind::Archive,
                suggested_name: "Galaxy.nyonliving.json".into(),
                bytes: vec![]
            }
            .validate(),
            Err(LivingTransferError::TooLarge)
        );
    }
}
