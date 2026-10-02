//! Independent native coordinator generations in the NYON application-data root.
use super::{LibrarySelection, SelectionError, SelectionTarget, recover_selection};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct NativeLibrarySelection {
    root: PathBuf,
}
fn storage(error: std::io::Error) -> SelectionError {
    SelectionError::Storage(error.to_string())
}
impl NativeLibrarySelection {
    pub fn new(application_data_root: impl Into<PathBuf>) -> Self {
        Self {
            root: application_data_root.into(),
        }
    }
    fn path(&self, suffix: &str) -> PathBuf {
        self.root.join(format!("library-selection-v1.{suffix}"))
    }
    fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, SelectionError> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(storage(error)),
        };
        let limit = if path
            .extension()
            .is_some_and(|extension| extension == "head")
        {
            32
        } else {
            super::MAX_RECORD_BYTES
        };
        if file.metadata().map_err(storage)?.len() > limit as u64 {
            return Ok(Some(Vec::new()));
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(storage)?;
        if bytes.len() > limit {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(bytes))
    }
    pub fn read(&self) -> Result<Option<LibrarySelection>, SelectionError> {
        let a = Self::read_optional(&self.path("a"))?;
        let b = Self::read_optional(&self.path("b"))?;
        let head_bytes = Self::read_optional(&self.path("head"))?;
        let head = head_bytes
            .as_ref()
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|value| value.parse::<u64>().ok());
        if a.is_none() && b.is_none() && head_bytes.is_some() {
            return Err(SelectionError::CorruptRecovery);
        }
        recover_selection(a.as_deref(), b.as_deref(), head)
    }
    /// Persist a selection only after the target authority transaction completed.
    /// Lock ownership prevents concurrent read/CAS/write races across handles.
    pub fn compare_and_swap(
        &self,
        expected: Option<u64>,
        target: Option<SelectionTarget>,
    ) -> Result<LibrarySelection, SelectionError> {
        self.compare_and_swap_guarded(expected, target, None)
    }
    fn compare_and_swap_guarded(
        &self,
        expected: Option<u64>,
        target: Option<SelectionTarget>,
        cancel: Option<&AtomicU8>,
    ) -> Result<LibrarySelection, SelectionError> {
        fs::create_dir_all(&self.root).map_err(storage)?;
        let lock_path = self.path("lock");
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(storage)?;
        lock.lock().map_err(storage)?;
        let current = self.read()?;
        if current.as_ref().map(|record| record.generation) != expected {
            return Err(SelectionError::StaleGeneration);
        }
        let generation = expected
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(SelectionError::Exhausted)?;
        let record = LibrarySelection::new(generation, target)?;
        let a = Self::read_optional(&self.path("a"))?
            .and_then(|bytes| LibrarySelection::decode(&bytes).ok());
        let suffix = if a.as_ref().map(|value| value.generation) == expected && expected.is_some() {
            "b"
        } else {
            "a"
        };
        if cancel.is_some_and(|state| {
            state
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        }) {
            return Err(SelectionError::Cancelled);
        }
        self.atomic_write(&self.path(suffix), &record.encode()?)?;
        self.atomic_write(&self.path("head"), generation.to_string().as_bytes())?;
        Ok(record)
    }
    fn atomic_write(&self, target: &Path, bytes: &[u8]) -> Result<(), SelectionError> {
        let temporary = target.with_extension(format!(
            "{}-pending",
            target
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("record")
        ));
        // The OS lock owns this fixed staging path; crash leftovers are disposable.
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(storage)?;
        }
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(storage)?;
            file.write_all(bytes).map_err(storage)?;
            file.sync_all().map_err(storage)?;
            drop(file);
            fs::rename(&temporary, target).map_err(storage)?;
            fs::File::open(&self.root)
                .and_then(|directory| directory.sync_all())
                .map_err(storage)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

use super::{
    LibrarySelectionCoordinator, SelectionCancel, SelectionJobId, SelectionPoll, SelectionRequest,
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
    thread,
};
struct Job {
    owner: u64,
    state: Arc<AtomicU8>,
    receiver: Option<mpsc::Receiver<SelectionPoll>>,
    request: Option<SelectionRequest>,
    result: Option<SelectionPoll>,
}
/// All filesystem reads/locks/fsync run on worker threads, never in poll.
pub struct NativeLibraryCoordinator {
    root: PathBuf,
    jobs: BTreeMap<SelectionJobId, Job>,
}
impl NativeLibraryCoordinator {
    pub fn new(application_data_root: impl Into<PathBuf>) -> Self {
        Self {
            root: application_data_root.into(),
            jobs: BTreeMap::new(),
        }
    }
}
impl NativeLibraryCoordinator {
    /// Release a terminal result after its owner consumed it. Unfinished jobs cannot be forgotten.
    pub fn release(&mut self, owner_epoch: u64, id: SelectionJobId) -> bool {
        if self
            .jobs
            .get(&id)
            .is_some_and(|job| job.owner == owner_epoch && job.result.is_some())
        {
            self.jobs.remove(&id);
            true
        } else {
            false
        }
    }
}
impl LibrarySelectionCoordinator for NativeLibraryCoordinator {
    fn release(&mut self, owner_epoch: u64, id: SelectionJobId) -> bool {
        NativeLibraryCoordinator::release(self, owner_epoch, id)
    }
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: SelectionRequest,
    ) -> Result<SelectionJobId, SelectionError> {
        if self.jobs.len() >= 16 {
            return Err(SelectionError::Storage("coordinator job capacity".into()));
        }
        let id = SelectionJobId(super::allocate_job_id()?);
        let state = Arc::new(AtomicU8::new(0));
        self.jobs.insert(
            id,
            Job {
                owner: owner_epoch,
                state,
                receiver: None,
                request: Some(request),
                result: None,
            },
        );
        Ok(id)
    }
    fn poll(&mut self, owner_epoch: u64, id: SelectionJobId, work_budget: usize) -> SelectionPoll {
        let Some(job) = self.jobs.get_mut(&id) else {
            return SelectionPoll::Failed(SelectionError::InvalidTarget);
        };
        if job.owner != owner_epoch {
            return SelectionPoll::Failed(SelectionError::InvalidTarget);
        }
        if work_budget == 0 {
            return SelectionPoll::Pending;
        }
        if let Some(request) = job.request.take() {
            let worker_state = Arc::clone(&job.state);
            let (sender, receiver) = mpsc::channel();
            let root = self.root.clone();
            thread::spawn(move || {
                if worker_state.load(Ordering::Acquire) == 2 {
                    let _ = sender.send(SelectionPoll::Cancelled);
                    return;
                }
                let store = NativeLibrarySelection::new(root);
                let result = match request {
                    SelectionRequest::Read => store.read(),
                    SelectionRequest::CompareAndSwap { expected, target } => store
                        .compare_and_swap_guarded(expected, target, Some(&worker_state))
                        .map(Some),
                };
                let outcome = match result {
                    Ok(record) => SelectionPoll::Ready(record),
                    Err(SelectionError::Cancelled) => SelectionPoll::Cancelled,
                    Err(error) => SelectionPoll::Failed(error),
                };
                worker_state.store(3, Ordering::Release);
                let _ = sender.send(outcome);
            });
            job.receiver = Some(receiver);
        }
        if job.result.is_none() {
            job.result = job
                .receiver
                .as_ref()
                .and_then(|receiver| receiver.try_recv().ok());
        }
        job.result.clone().unwrap_or(SelectionPoll::Pending)
    }
    fn cancel(
        &mut self,
        owner_epoch: u64,
        id: SelectionJobId,
    ) -> Result<SelectionCancel, SelectionError> {
        let Some(job) = self.jobs.get_mut(&id) else {
            return Err(SelectionError::InvalidTarget);
        };
        if job.owner != owner_epoch {
            return Err(SelectionError::InvalidTarget);
        }
        if job.result.is_none() {
            job.result = job
                .receiver
                .as_ref()
                .and_then(|receiver| receiver.try_recv().ok());
        }
        match &job.result {
            Some(SelectionPoll::Ready(Some(record))) => {
                Ok(SelectionCancel::CommitAlreadyWon(record.clone()))
            }
            Some(SelectionPoll::Failed(_)) => Ok(SelectionCancel::CancelPending),
            Some(_) => {
                self.jobs.remove(&id);
                Ok(SelectionCancel::Cancelled)
            }
            None if job
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                .is_ok() =>
            {
                self.jobs.remove(&id);
                Ok(SelectionCancel::Cancelled)
            }
            None => Ok(SelectionCancel::CancelPending),
        }
    }
}

impl Drop for NativeLibraryCoordinator {
    fn drop(&mut self) {
        for job in self.jobs.values() {
            let _ = job
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}
