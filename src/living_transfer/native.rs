//! Native OS picker followed by bounded off-thread read or atomic durable write.
use super::*;
use std::{
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
};

struct Job {
    state: Arc<AtomicU8>,
    receiver: mpsc::Receiver<Result<LivingTransferOutcome, LivingTransferError>>,
    task: tokio::task::JoinHandle<()>,
}
#[derive(Default)]
pub struct NativeLivingTransfer {
    job: Option<Job>,
}

fn read_file(
    path: &std::path::Path,
    kind: LivingTransferKind,
) -> Result<LivingTransferOutcome, LivingTransferError> {
    let file = std::fs::File::open(path).map_err(|_| LivingTransferError::Platform)?;
    if file
        .metadata()
        .map_err(|_| LivingTransferError::Platform)?
        .len()
        > kind.max_bytes() as u64
    {
        return Err(LivingTransferError::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(kind.max_bytes() as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| LivingTransferError::Platform)?;
    if bytes.is_empty() || bytes.len() > kind.max_bytes() {
        return Err(LivingTransferError::TooLarge);
    }
    Ok(LivingTransferOutcome::Imported { kind, bytes })
}
fn write_file(
    path: &std::path::Path,
    bytes: &[u8],
    state: &AtomicU8,
) -> Result<LivingTransferOutcome, LivingTransferError> {
    let parent = path.parent().ok_or(LivingTransferError::Platform)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| LivingTransferError::Platform)?;
    temporary
        .write_all(bytes)
        .map_err(|_| LivingTransferError::Platform)?;
    temporary
        .flush()
        .map_err(|_| LivingTransferError::Platform)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| LivingTransferError::Platform)?;
    // Cancellation wins until placement. Once placement wins, poll reports its result.
    if state
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Ok(LivingTransferOutcome::Cancelled);
    }
    temporary
        .persist(path)
        .map_err(|_| LivingTransferError::Platform)?;
    #[cfg(unix)]
    std::fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| LivingTransferError::Platform)?;
    Ok(LivingTransferOutcome::Exported(
        LivingExportOutcome::DurablySaved,
    ))
}
impl LivingTransfer for NativeLivingTransfer {
    fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
        if self.job.is_some() {
            return Err(LivingTransferError::Busy);
        }
        request.validate()?;
        let runtime =
            tokio::runtime::Handle::try_current().map_err(|_| LivingTransferError::Unavailable)?;
        let dialog = rfd::AsyncFileDialog::new().add_filter("NYON JSON", &["json"]);
        let state = Arc::new(AtomicU8::new(0));
        let worker = Arc::clone(&state);
        let (sender, receiver) = mpsc::channel();
        let task = match request {
            LivingTransferRequest::Import(kind) => {
                let choice = dialog.pick_file();
                runtime.spawn(async move {
                    let result = if let Some(file) = choice.await {
                        if worker.load(Ordering::Acquire) == 2 {
                            Ok(LivingTransferOutcome::Cancelled)
                        } else {
                            let path = file.path().to_path_buf();
                            tokio::task::spawn_blocking(move || read_file(&path, kind))
                                .await
                                .unwrap_or(Err(LivingTransferError::Platform))
                        }
                    } else {
                        Ok(LivingTransferOutcome::Cancelled)
                    };
                    let _ = sender.send(result);
                })
            }
            LivingTransferRequest::Export {
                suggested_name,
                bytes,
                ..
            } => {
                let choice = dialog.set_file_name(suggested_name).save_file();
                runtime.spawn(async move {
                    let result = if let Some(file) = choice.await {
                        let path = file.path().to_path_buf();
                        tokio::task::spawn_blocking(move || write_file(&path, &bytes, &worker))
                            .await
                            .unwrap_or(Err(LivingTransferError::Platform))
                    } else {
                        Ok(LivingTransferOutcome::Cancelled)
                    };
                    let _ = sender.send(result);
                })
            }
        };
        self.job = Some(Job {
            state,
            receiver,
            task,
        });
        Ok(())
    }
    fn poll(&mut self) -> LivingTransferStatus {
        let Some(job) = self.job.as_ref() else {
            return LivingTransferStatus::Idle;
        };
        match job.receiver.try_recv() {
            Ok(result) => {
                self.job = None;
                LivingTransferStatus::Ready(result)
            }
            Err(mpsc::TryRecvError::Empty) => LivingTransferStatus::Pending,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.job = None;
                LivingTransferStatus::Ready(Err(LivingTransferError::Platform))
            }
        }
    }
    fn cancel(&mut self) {
        if let Some(job) = self.job.as_ref()
            && job
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            job.task.abort();
            self.job = None;
        }
    }
}
impl Drop for NativeLivingTransfer {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_import_and_cancelled_atomic_export_preserve_destination() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("galaxy.json");
        std::fs::write(&path, b"original").unwrap();
        assert!(
            matches!(read_file(&path,LivingTransferKind::Archive),Ok(LivingTransferOutcome::Imported {bytes,..}) if bytes==b"original")
        );
        let cancelled = AtomicU8::new(2);
        assert_eq!(
            write_file(&path, b"new", &cancelled).unwrap(),
            LivingTransferOutcome::Cancelled
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"original");
        let active = AtomicU8::new(0);
        assert_eq!(
            write_file(&path, b"new", &active).unwrap(),
            LivingTransferOutcome::Exported(LivingExportOutcome::DurablySaved)
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }
}
