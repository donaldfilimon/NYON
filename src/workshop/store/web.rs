//! IndexedDB persistence for Galaxy Workshop.
//!
//! Browser writes stage archive bytes and their slot reference in the same
//! read-write transaction. A completed job is published only after IndexedDB
//! reports that transaction committed. Request, transaction, quota, and schema
//! failures therefore leave the previously committed browser state intact.

use super::*;

pub const INDEXED_DB_NAME: &str = "nyon.workshop.v1";
pub const INDEXED_DB_VERSION: u32 = 1;
pub const SLOTS_OBJECT_STORE: &str = "slots";
pub const ARCHIVES_OBJECT_STORE: &str = "archives";
pub const PACKS_OBJECT_STORE: &str = "packs";

/// Failure modes used by the deterministic transaction conformance model.
///
/// The model stages a request against a cloned committed state, then either
/// publishes the candidate or injects one of these terminal failures. It lets
/// native tests prove the state-preservation contract that the wasm adapter
/// implements with real IndexedDB transactions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexedDbModelFailure {
    Abort,
    QuotaDenied,
    Unavailable,
    SchemaMismatch,
    Evicted,
}

impl IndexedDbModelFailure {
    fn error(self) -> WorkshopStoreError {
        match self {
            Self::Abort => WorkshopStoreError::IndexedDbTransactionAborted,
            Self::QuotaDenied => WorkshopStoreError::IndexedDbQuotaDenied,
            Self::Unavailable => WorkshopStoreError::IndexedDbUnavailable,
            Self::SchemaMismatch => WorkshopStoreError::IndexedDbSchemaMismatch,
            Self::Evicted => WorkshopStoreError::IndexedDbEvicted,
        }
    }
}

/// Pure deterministic conformance model for IndexedDB transaction publication.
///
/// This is not a second persistence implementation. It is a host-testable
/// oracle for the important transaction boundary: mutations become visible
/// only after the transaction succeeds.
#[derive(Clone, Default)]
pub struct IndexedDbTransactionModel {
    jobs: JobTable,
    committed: MemoryWorkshopStore,
    next_failure: Option<IndexedDbModelFailure>,
}

impl IndexedDbTransactionModel {
    pub fn inject_next_failure(&mut self, failure: IndexedDbModelFailure) {
        self.next_failure = Some(failure);
    }
}

impl WorkshopStore for IndexedDbTransactionModel {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        let job = self.jobs.reserve(request.class())?;
        let mut candidate = self.committed.clone();
        let result = candidate.execute(request).and_then(|result| {
            if let Some(failure) = self.next_failure.take() {
                return Err(failure.error());
            }
            self.committed = candidate;
            Ok(result)
        });
        self.jobs.finish(job, result);
        Ok(job)
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        self.jobs.abandon(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.jobs.poll(job)
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm;

#[cfg(target_arch = "wasm32")]
pub use wasm::IndexedDbWorkshopStore;
