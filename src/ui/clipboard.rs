//! macOS clipboard operations run outside the UI thread. A completion owns the
//! original resident, field, text and selection; it cannot edit a later owner.

use std::{rc::Rc, sync::mpsc};

use super::text_edit::TextEdit;

pub const MAX_CLIPBOARD_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardOperation {
    Copy,
    Cut,
    Paste,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardError {
    Busy,
    Unsupported,
    NoSelection,
    TooLarge,
    InvalidUtf8,
    RejectedText,
    ProcessFailed,
    IoFailed,
    Cancelled,
    TimedOut,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Busy => "A clipboard operation is still running.",
            Self::Unsupported => "System clipboard support is available on macOS.",
            Self::NoSelection => "Select text before copying or cutting.",
            Self::TooLarge => "Clipboard text exceeds 1 MiB.",
            Self::InvalidUtf8 => "Clipboard text is not valid UTF-8.",
            Self::RejectedText => {
                "Clipboard text does not fit this field or contains unsupported control characters."
            }
            Self::ProcessFailed => "The macOS clipboard command failed.",
            Self::IoFailed => "Clipboard transfer failed.",
            Self::Cancelled => "Clipboard operation was cancelled.",
            Self::TimedOut => "Clipboard operation timed out.",
        })
    }
}
impl std::error::Error for ClipboardError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipboardApply {
    Stale,
    Unchanged,
    Changed,
}

struct Owner {
    scope: Rc<()>,
    field: String,
    editor: TextEdit,
}

pub struct ClipboardCompletion {
    owner: Owner,
    operation: ClipboardOperation,
    result: Result<Option<String>, ClipboardError>,
}

impl ClipboardCompletion {
    /// A failed paste or stale completion preserves both text and selection.
    pub fn apply(
        self,
        scope: &Rc<()>,
        field: &str,
        editor: &mut TextEdit,
        max_bytes: usize,
    ) -> Result<ClipboardApply, ClipboardError> {
        if !Rc::ptr_eq(scope, &self.owner.scope)
            || field != self.owner.field
            || *editor != self.owner.editor
        {
            return Ok(ClipboardApply::Stale);
        }
        let text = self.result?;
        match self.operation {
            ClipboardOperation::Copy => Ok(ClipboardApply::Unchanged),
            ClipboardOperation::Cut => {
                if editor.selection().is_empty() {
                    return Ok(ClipboardApply::Unchanged);
                }
                editor.delete(true);
                Ok(ClipboardApply::Changed)
            }
            ClipboardOperation::Paste => {
                let text = text.ok_or(ClipboardError::IoFailed)?;
                if text.is_empty() {
                    return Ok(ClipboardApply::Unchanged);
                }
                if editor.insert(&text, max_bytes) {
                    Ok(ClipboardApply::Changed)
                } else {
                    Err(ClipboardError::RejectedText)
                }
            }
        }
    }
}

struct Pending {
    owner: Owner,
    operation: ClipboardOperation,
    receiver: mpsc::Receiver<Result<Option<String>, ClipboardError>>,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// One UI mailbox. The process lane remains occupied until its worker stops,
/// including when cancellation drops the mailbox. No UI call joins a worker.
#[derive(Default)]
pub struct NativeClipboard {
    pending: Option<Pending>,
}

impl NativeClipboard {
    pub fn request(
        &mut self,
        scope: Rc<()>,
        field: String,
        editor: TextEdit,
        operation: ClipboardOperation,
    ) -> Result<(), ClipboardError> {
        if self.pending.is_some() {
            return Err(ClipboardError::Busy);
        }
        let selection = editor.selection();
        if operation != ClipboardOperation::Paste && selection.is_empty() {
            return Err(ClipboardError::NoSelection);
        }
        if selection.len() > MAX_CLIPBOARD_BYTES {
            return Err(ClipboardError::TooLarge);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (scope, field);
            Err(ClipboardError::Unsupported)
        }
        #[cfg(target_os = "macos")]
        {
            let lane = worker::Lane::acquire()?;
            let copied = editor.value()[selection].to_owned();
            let (sender, receiver) = mpsc::channel();
            let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let worker_cancelled = std::sync::Arc::clone(&cancelled);
            std::thread::Builder::new()
                .name("nyon-clipboard".to_owned())
                .spawn(move || {
                    let _lane = lane;
                    let result = worker::run(operation, copied, &worker_cancelled);
                    let _ = sender.send(result);
                })
                .map_err(|_| ClipboardError::IoFailed)?;
            self.pending = Some(Pending {
                owner: Owner {
                    scope,
                    field,
                    editor,
                },
                operation,
                receiver,
                cancelled,
            });
            Ok(())
        }
    }

    pub fn poll(&mut self) -> Option<ClipboardCompletion> {
        let result = match self.pending.as_ref()?.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err(ClipboardError::IoFailed),
        };
        let pending = self.pending.take()?;
        Some(ClipboardCompletion {
            owner: pending.owner,
            operation: pending.operation,
            result,
        })
    }

    pub fn cancel(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending
                .cancelled
                .store(true, std::sync::atomic::Ordering::Release);
        }
    }
}

impl Drop for NativeClipboard {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(any(target_os = "macos", test))]
fn read_text(reader: impl std::io::Read) -> Result<String, ClipboardError> {
    use std::io::Read;
    let mut bytes = Vec::new();
    reader
        .take((MAX_CLIPBOARD_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ClipboardError::IoFailed)?;
    if bytes.len() > MAX_CLIPBOARD_BYTES {
        return Err(ClipboardError::TooLarge);
    }
    String::from_utf8(bytes).map_err(|_| ClipboardError::InvalidUtf8)
}

#[cfg(target_os = "macos")]
mod worker {
    use std::{
        io::Write,
        process::{Child, Command, Stdio},
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        time::{Duration, Instant},
    };

    use super::{ClipboardError, ClipboardOperation, read_text};

    static OCCUPIED: AtomicBool = AtomicBool::new(false);
    const TIMEOUT: Duration = Duration::from_secs(5);

    pub(super) struct Lane;
    impl Lane {
        pub(super) fn acquire() -> Result<Self, ClipboardError> {
            OCCUPIED
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .map(|_| Self)
                .map_err(|_| ClipboardError::Busy)
        }
    }
    impl Drop for Lane {
        fn drop(&mut self) {
            OCCUPIED.store(false, Ordering::Release);
        }
    }

    fn stop(child: &mut Child) {
        let _ = child.kill();
        let _ = child.wait();
    }

    pub(super) fn run(
        operation: ClipboardOperation,
        copied: String,
        cancelled: &AtomicBool,
    ) -> Result<Option<String>, ClipboardError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(ClipboardError::Cancelled);
        }
        let paste = operation == ClipboardOperation::Paste;
        let mut child = Command::new(if paste {
            "/usr/bin/pbpaste"
        } else {
            "/usr/bin/pbcopy"
        })
        .env("LC_CTYPE", "en_US.UTF-8")
        .stdin(if paste { Stdio::null() } else { Stdio::piped() })
        .stdout(if paste { Stdio::piped() } else { Stdio::null() })
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ClipboardError::ProcessFailed)?;
        let stdout = child.stdout.take();
        let stdin = child.stdin.take();
        let (sender, receiver) = mpsc::channel();
        if std::thread::Builder::new()
            .name("nyon-clipboard-io".to_owned())
            .spawn(move || {
                let result = if paste {
                    stdout
                        .ok_or(ClipboardError::IoFailed)
                        .and_then(read_text)
                        .map(Some)
                } else {
                    stdin
                        .ok_or(ClipboardError::IoFailed)
                        .and_then(|mut writer| {
                            writer
                                .write_all(copied.as_bytes())
                                .map_err(|_| ClipboardError::IoFailed)
                        })
                        .map(|()| None)
                };
                let _ = sender.send(result);
            })
            .is_err()
        {
            stop(&mut child);
            return Err(ClipboardError::IoFailed);
        }
        let started = Instant::now();
        let mut output = None;
        loop {
            let interruption = if cancelled.load(Ordering::Acquire) {
                Some(ClipboardError::Cancelled)
            } else if started.elapsed() >= TIMEOUT {
                Some(ClipboardError::TimedOut)
            } else {
                None
            };
            if let Some(error) = interruption {
                stop(&mut child);
                return Err(error);
            }
            if output.is_none() {
                match receiver.recv_timeout(Duration::from_millis(10)) {
                    Ok(Ok(value)) => output = Some(value),
                    Ok(Err(error)) => {
                        stop(&mut child);
                        return Err(error);
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        stop(&mut child);
                        return Err(ClipboardError::IoFailed);
                    }
                }
            }
            match child.try_wait() {
                Ok(Some(status)) if !status.success() => {
                    return Err(ClipboardError::ProcessFailed);
                }
                Ok(Some(_)) if output.is_some() => return output.ok_or(ClipboardError::IoFailed),
                Ok(_) => {}
                Err(_) => {
                    stop(&mut child);
                    return Err(ClipboardError::ProcessFailed);
                }
            }
            if output.is_some() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion(
        scope: &Rc<()>,
        field: &str,
        editor: &TextEdit,
        operation: ClipboardOperation,
        text: Option<&str>,
    ) -> ClipboardCompletion {
        ClipboardCompletion {
            owner: Owner {
                scope: Rc::clone(scope),
                field: field.to_owned(),
                editor: editor.clone(),
            },
            operation,
            result: Ok(text.map(str::to_owned)),
        }
    }

    #[test]
    fn clipboard_read_bounds_bytes_and_rejects_partial_utf8_and_read_errors() {
        assert_eq!(
            read_text(std::io::Cursor::new(vec![b'x'; MAX_CLIPBOARD_BYTES]))
                .unwrap()
                .len(),
            MAX_CLIPBOARD_BYTES
        );
        assert_eq!(
            read_text(std::io::Cursor::new(vec![b'x'; MAX_CLIPBOARD_BYTES + 1])),
            Err(ClipboardError::TooLarge)
        );
        assert_eq!(
            read_text(std::io::Cursor::new(vec![0xf0, 0x9f])),
            Err(ClipboardError::InvalidUtf8)
        );
        assert_eq!(
            read_text(std::io::Cursor::new("星🌌")),
            Ok("星🌌".to_owned())
        );
        struct Fails;
        impl std::io::Read for Fails {
            fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("read failed"))
            }
        }
        assert_eq!(read_text(Fails), Err(ClipboardError::IoFailed));
    }

    #[test]
    fn late_cut_cannot_edit_another_resident_field_or_selection() {
        let scope = Rc::new(());
        let mut original = TextEdit::new("A星B".to_owned());
        original.select_all();
        let mut editor = original.clone();
        assert_eq!(
            completion(&scope, "name", &original, ClipboardOperation::Cut, None).apply(
                &Rc::new(()),
                "name",
                &mut editor,
                32
            ),
            Ok(ClipboardApply::Stale)
        );
        assert_eq!(editor, original);
        assert_eq!(
            completion(&scope, "name", &original, ClipboardOperation::Cut, None).apply(
                &scope,
                "seed",
                &mut editor,
                32
            ),
            Ok(ClipboardApply::Stale)
        );
        assert_eq!(editor, original);
        editor.move_caret(-1, false);
        let changed_selection = editor.clone();
        assert_eq!(
            completion(&scope, "name", &original, ClipboardOperation::Cut, None).apply(
                &scope,
                "name",
                &mut editor,
                32
            ),
            Ok(ClipboardApply::Stale)
        );
        assert_eq!(editor, changed_selection);
        editor = original.clone();
        assert_eq!(
            completion(&scope, "name", &original, ClipboardOperation::Cut, None).apply(
                &scope,
                "name",
                &mut editor,
                32
            ),
            Ok(ClipboardApply::Changed)
        );
        assert_eq!(editor.value(), "");
    }

    #[test]
    fn failed_mailbox_completion_is_suppressed_after_owner_change() {
        let scope = Rc::new(());
        let original = TextEdit::new("draft".to_owned());
        let (sender, receiver) = mpsc::channel();
        let mut clipboard = NativeClipboard {
            pending: Some(Pending {
                owner: Owner {
                    scope: Rc::clone(&scope),
                    field: "name".to_owned(),
                    editor: original.clone(),
                },
                operation: ClipboardOperation::Paste,
                receiver,
                cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }),
        };
        assert!(clipboard.poll().is_none());
        drop(sender);
        let completion = clipboard.poll().unwrap();
        let mut current = original.clone();
        assert_eq!(
            completion.apply(&Rc::new(()), "name", &mut current, 32),
            Ok(ClipboardApply::Stale)
        );
        assert_eq!(current, original);
        assert!(clipboard.poll().is_none());
    }

    #[test]
    fn paste_rejection_preserves_selection_and_unicode_paste_is_atomic() {
        let scope = Rc::new(());
        let mut editor = TextEdit::new("seed".to_owned());
        editor.select_all();
        let original = editor.clone();
        for text in ["too long", "bad\n"] {
            assert_eq!(
                completion(
                    &scope,
                    "seed",
                    &original,
                    ClipboardOperation::Paste,
                    Some(text)
                )
                .apply(&scope, "seed", &mut editor, 4),
                Err(ClipboardError::RejectedText)
            );
            assert_eq!(editor, original);
        }
        assert_eq!(
            completion(
                &scope,
                "seed",
                &original,
                ClipboardOperation::Paste,
                Some("星")
            )
            .apply(&scope, "seed", &mut editor, 4),
            Ok(ClipboardApply::Changed)
        );
        assert_eq!(editor.value(), "星");
    }
}
