//! The platform-neutral portable transfer protocol (route-design task 11,
//! addendum §7).
//!
//! Canonical pack and archive bytes are produced and validated elsewhere: in
//! the Workshop core, the session and the exact-catalog Library client. This
//! module only describes how those bytes cross into, and out of, a platform
//! file surface, through one small object-safe trait with bounded jobs.
//!
//! # What is here, and what is deliberately not
//!
//! - [`TransferAdapter`], the protocol: Choose Import, Hand Off Export, and a
//!   poll-driven job that ends in Import Chosen, Export Handed Off, Cancelled
//!   or a bounded opaque failure.
//! - [`SuggestedName`], §7's two predictable, sanitized file names.
//! - [`HandoffOutcome`], the only words the product may use for a finished
//!   handoff. §1 reserves "saved" for a native write that flushed,
//!   synchronized and placed its file.
//! - [`ScriptedTransfer`], the test adapter. It touches no platform API, and
//!   no product entry point installs it.
//!
//! **No real adapter exists yet.** §8 gates the native picker on a
//! compatibility spike, and the route design gates the browser adapter on the
//! same spike, so a product build installs nothing and the handoff renders
//! disabled with a visible reason. The trait deliberately has no `Send` or
//! `Sync` bound: a browser adapter will hold `Rc<RefCell<..>>` mailboxes, as
//! the wasm GPU initialization does.

use std::{
    cell::RefCell,
    collections::VecDeque,
    fmt::{self, Write as _},
    rc::Rc,
};

use crate::workshop::{
    CatalogHash,
    store::{MAX_WORKSHOP_ARCHIVE_BYTES, MAX_WORKSHOP_PACK_BYTES, SlotName},
};

/// What a transfer carries. The decoder stays authoritative; the extension
/// and media type are convenience hints only (§7).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferKind {
    /// A `.nyonworkshop.json` Workshop archive.
    WorkshopArchive,
    /// A `.nyonpack.json` content pack.
    ContentPack,
}

impl TransferKind {
    /// The largest payload of this kind any transfer may carry: the store's
    /// own limit, so nothing the store would refuse can be imported or
    /// handed off.
    pub const fn max_bytes(self) -> usize {
        match self {
            Self::WorkshopArchive => MAX_WORKSHOP_ARCHIVE_BYTES,
            Self::ContentPack => MAX_WORKSHOP_PACK_BYTES,
        }
    }

    /// The file-name suffix hint, including its leading dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::WorkshopArchive => ".nyonworkshop.json",
            Self::ContentPack => ".nyonpack.json",
        }
    }

    /// The media-type hint.
    pub const fn media_type(self) -> &'static str {
        "application/json"
    }
}

/// A predictable, sanitized suggested file name (§7).
///
/// Built only by the two constructors, so every name ends in its kind's
/// extension and contains nothing but ASCII letters, digits, `-`, `_` and the
/// extension's dots. A slot name is printable ASCII already, but may still
/// hold path separators, reserved characters or a leading dot, none of which
/// may reach a platform picker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SuggestedName(String);

impl SuggestedName {
    /// `<slot-name>.nyonworkshop.json`, with the slot name sanitized.
    pub fn workshop_archive(name: &SlotName) -> Self {
        let mut output = String::with_capacity(
            name.as_str().len() + TransferKind::WorkshopArchive.extension().len(),
        );
        output.extend(name.as_str().chars().map(sanitize_char));
        output.push_str(TransferKind::WorkshopArchive.extension());
        Self(output)
    }

    /// `<catalog-hash-prefix>.nyonpack.json`: the first eight bytes of the
    /// hash as sixteen lowercase hex digits.
    pub fn content_pack(hash: CatalogHash) -> Self {
        let mut output = String::with_capacity(16 + TransferKind::ContentPack.extension().len());
        for byte in &hash.0[..8] {
            // Writing to a `String` cannot fail.
            let _ = write!(output, "{byte:02x}");
        }
        output.push_str(TransferKind::ContentPack.extension());
        Self(output)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SuggestedName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Letters, digits, `-` and `_` survive; a space becomes `-`; everything else,
/// including `.`, `/`, `\` and every reserved picker character, becomes `_`.
const fn sanitize_char(character: char) -> char {
    match character {
        'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
        ' ' => '-',
        _ => '_',
    }
}

/// One transfer the product asks a platform to perform.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferRequest {
    /// Ask the user for one file of `kind`, reading at most `max_bytes`.
    ChooseImport {
        kind: TransferKind,
        max_bytes: usize,
    },
    /// Hand validated canonical bytes to the platform. Always the second
    /// stage of an export, started by its own direct user activation (§7).
    HandOffExport {
        kind: TransferKind,
        suggested_name: SuggestedName,
        bytes: Box<[u8]>,
    },
}

impl TransferRequest {
    /// The bounds every adapter enforces before it starts anything: an import
    /// may not ask for more than its kind allows, and an export must carry
    /// between one byte and its kind's limit.
    pub fn check(&self) -> Result<(), TransferError> {
        match self {
            Self::ChooseImport { kind, max_bytes } => {
                if *max_bytes == 0 || *max_bytes > kind.max_bytes() {
                    return Err(TransferError::new(TransferFailureCode::TooLarge));
                }
            }
            Self::HandOffExport { kind, bytes, .. } => {
                if bytes.is_empty() || bytes.len() > kind.max_bytes() {
                    return Err(TransferError::new(TransferFailureCode::TooLarge));
                }
            }
        }
        Ok(())
    }
}

/// How far a finished export actually got. The product's wording is keyed to
/// this, never to which adapter ran (§1, §8, §9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandoffOutcome {
    /// A browser accepted the download. Consumption is not observable.
    DownloadStarted,
    /// The operating system accepted the bytes; no write was proven.
    HandedToSystem,
    /// A native write finished without the full durability sequence.
    Written,
    /// A native write finished with same-directory temporary output, flush,
    /// file synchronization, atomic placement and parent synchronization
    /// where the platform supports it. The only outcome that may say "saved".
    DurablySaved,
}

impl HandoffOutcome {
    /// The user-facing sentence for this outcome.
    pub const fn label(self) -> &'static str {
        match self {
            Self::DownloadStarted => "Download started.",
            Self::HandedToSystem => "Copy handed to the operating system.",
            Self::Written => "Copy written.",
            Self::DurablySaved => "Copy saved.",
        }
    }
}

/// The terminal answer of a transfer job that did not fail.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferOutcome {
    /// The user chose a file, and these are its bytes, at most the request's
    /// `max_bytes`.
    ImportChosen {
        kind: TransferKind,
        bytes: Box<[u8]>,
    },
    /// The export left the product.
    ExportHandedOff(HandoffOutcome),
    /// The user dismissed the picker or the save. Ordinary, not a failure,
    /// and never a diagnostic (§8).
    Cancelled,
}

/// A bounded opaque failure. It carries no platform text, no path and no
/// content, so nothing a platform says can reach a log through it (§8, §10).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferFailureCode {
    /// No adapter is installed.
    Unavailable,
    /// The adapter already has a job.
    Busy,
    /// The request or the chosen file exceeds its kind's limit, or is empty.
    TooLarge,
    /// The adapter ran out of job identifiers.
    Exhausted,
    /// The platform refused or failed the transfer.
    Platform,
    /// The adapter answered with something the protocol does not allow: an
    /// unknown job, or an outcome of the wrong shape.
    Protocol,
}

impl TransferFailureCode {
    /// A fixed, safe sentence for this failure.
    pub const fn message(self) -> &'static str {
        match self {
            Self::Unavailable => "Portable file transfer is not available.",
            Self::Busy => "Another portable file transfer is still running.",
            Self::TooLarge => "The portable file is empty or too large.",
            Self::Exhausted => "Portable file transfer ran out of job identifiers.",
            Self::Platform => "The system could not complete the portable file transfer.",
            Self::Protocol => "Portable file transfer returned an unexpected result.",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{}", .code.message())]
pub struct TransferError {
    pub code: TransferFailureCode,
}

impl TransferError {
    pub const fn new(code: TransferFailureCode) -> Self {
        Self { code }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransferJobId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferJobState {
    Pending,
    Complete(Result<TransferOutcome, TransferError>),
    /// The job was never started, was already polled to completion, or was
    /// abandoned.
    Unknown,
}

/// The portable transfer protocol (§7).
///
/// Mirrors [`crate::workshop::store::WorkshopStore`]'s job shape on purpose,
/// so a client that owns a transfer across frames follows the same rules:
///
/// - **Bounded.** An adapter holds at most one job; a second `start` is
///   refused with [`TransferFailureCode::Busy`]. Every request is checked with
///   [`TransferRequest::check`] before anything starts.
/// - **Terminal once.** [`Self::poll`] returns a completed state at most once
///   and frees the job when it does; afterwards the ID is
///   [`TransferJobState::Unknown`].
/// - **Abandon drops the outcome, not the work.** A download a browser has
///   already accepted cannot be recalled. A caller that abandons a handoff
///   must not claim either result.
/// - **Imports are capped.** An adapter checks the reported size before
///   allocating and reads at most `max_bytes + 1`, answering
///   [`TransferFailureCode::TooLarge`] rather than delivering more.
///
/// Object safe, and with no `Send`/`Sync` bound, so a browser adapter over
/// `Rc` mailboxes can implement it.
pub trait TransferAdapter {
    fn start(&mut self, request: TransferRequest) -> Result<TransferJobId, TransferError>;

    /// Releases `job` without delivering its outcome, and reports whether a
    /// live job was released.
    fn abandon(&mut self, job: TransferJobId) -> bool;

    fn poll(&mut self, job: TransferJobId) -> TransferJobState;
}

/// How [`ScriptedTransfer`] answers its next job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScriptedReply {
    /// A handoff completes with this outcome. An import completes with the
    /// scripted import bytes, or as cancelled when none are scripted.
    Complete(HandoffOutcome),
    /// The user cancels.
    Cancel,
    /// The job completes with this failure.
    Fail(TransferFailureCode),
    /// `start` itself refuses with this failure.
    Refuse(TransferFailureCode),
    /// The job completes with an outcome of the wrong shape: an import answer
    /// to a handoff, or a handoff answer to an import.
    WrongShape,
    /// The adapter forgets the job, so polling it answers `Unknown`.
    Forget,
}

/// One export the scripted adapter accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptedHandoff {
    pub kind: TransferKind,
    pub suggested_name: SuggestedName,
    pub bytes: Box<[u8]>,
}

#[derive(Debug)]
struct ScriptState {
    replies: VecDeque<ScriptedReply>,
    default_reply: ScriptedReply,
    import_bytes: Option<Box<[u8]>>,
    pending_polls: u32,
    next_job: u64,
    active: Option<ScriptedJob>,
    started: Vec<TransferRequest>,
    handed_off: Vec<ScriptedHandoff>,
    abandoned: usize,
}

#[derive(Debug)]
struct ScriptedJob {
    id: TransferJobId,
    request: TransferRequest,
    reply: ScriptedReply,
    polls_left: u32,
}

/// The test adapter: a deterministic in-memory [`TransferAdapter`] that
/// touches no platform API.
///
/// Clones share one script, so a test can keep a handle after boxing a clone
/// into the runtime, then steer replies and read what was handed off. It is
/// never installed by a product entry point.
#[derive(Clone, Debug)]
pub struct ScriptedTransfer {
    state: Rc<RefCell<ScriptState>>,
}

impl Default for ScriptedTransfer {
    fn default() -> Self {
        Self {
            state: Rc::new(RefCell::new(ScriptState {
                replies: VecDeque::new(),
                default_reply: ScriptedReply::Complete(HandoffOutcome::DownloadStarted),
                import_bytes: None,
                pending_polls: 0,
                next_job: 0,
                active: None,
                started: Vec::new(),
                handed_off: Vec::new(),
                abandoned: 0,
            })),
        }
    }
}

impl ScriptedTransfer {
    /// Queues the reply for the next job; later jobs use the default.
    pub fn push_reply(&self, reply: ScriptedReply) {
        self.state.borrow_mut().replies.push_back(reply);
    }

    /// The reply for every job with nothing queued.
    pub fn set_default_reply(&self, reply: ScriptedReply) {
        self.state.borrow_mut().default_reply = reply;
    }

    /// The file an import "chooses". `None` makes a completing import cancel.
    pub fn set_import_bytes(&self, bytes: Option<Box<[u8]>>) {
        self.state.borrow_mut().import_bytes = bytes;
    }

    /// How many polls answer `Pending` before a job completes.
    pub fn set_pending_polls(&self, polls: u32) {
        self.state.borrow_mut().pending_polls = polls;
    }

    /// Every request `start` accepted, in order.
    pub fn started(&self) -> Vec<TransferRequest> {
        self.state.borrow().started.clone()
    }

    /// Every export that completed as handed off, in order.
    pub fn handed_off(&self) -> Vec<ScriptedHandoff> {
        self.state.borrow().handed_off.clone()
    }

    /// How many live jobs were abandoned.
    pub fn abandoned(&self) -> usize {
        self.state.borrow().abandoned
    }

    /// Whether a job is outstanding.
    pub fn busy(&self) -> bool {
        self.state.borrow().active.is_some()
    }
}

impl TransferAdapter for ScriptedTransfer {
    fn start(&mut self, request: TransferRequest) -> Result<TransferJobId, TransferError> {
        let mut state = self.state.borrow_mut();
        request.check()?;
        if state.active.is_some() {
            return Err(TransferError::new(TransferFailureCode::Busy));
        }
        let reply = state
            .replies
            .pop_front()
            .unwrap_or_else(|| state.default_reply.clone());
        if let ScriptedReply::Refuse(code) = reply {
            return Err(TransferError::new(code));
        }
        let id = TransferJobId(state.next_job);
        state.next_job = state
            .next_job
            .checked_add(1)
            .ok_or(TransferError::new(TransferFailureCode::Exhausted))?;
        state.started.push(request.clone());
        let polls_left = state.pending_polls;
        state.active = Some(ScriptedJob {
            id,
            request,
            reply,
            polls_left,
        });
        Ok(id)
    }

    fn abandon(&mut self, job: TransferJobId) -> bool {
        let mut state = self.state.borrow_mut();
        if state.active.as_ref().is_some_and(|active| active.id == job) {
            state.active = None;
            state.abandoned += 1;
            true
        } else {
            false
        }
    }

    fn poll(&mut self, job: TransferJobId) -> TransferJobState {
        let mut state = self.state.borrow_mut();
        let Some(active) = state.active.as_mut().filter(|active| active.id == job) else {
            return TransferJobState::Unknown;
        };
        if active.polls_left > 0 {
            active.polls_left -= 1;
            return TransferJobState::Pending;
        }
        let Some(ScriptedJob { request, reply, .. }) = state.active.take() else {
            return TransferJobState::Unknown;
        };
        let result = match (reply, request) {
            (ScriptedReply::Forget, _) => return TransferJobState::Unknown,
            (ScriptedReply::Cancel, _) => Ok(TransferOutcome::Cancelled),
            (ScriptedReply::Fail(code) | ScriptedReply::Refuse(code), _) => {
                Err(TransferError::new(code))
            }
            (
                ScriptedReply::Complete(outcome),
                TransferRequest::HandOffExport {
                    kind,
                    suggested_name,
                    bytes,
                },
            ) => {
                state.handed_off.push(ScriptedHandoff {
                    kind,
                    suggested_name,
                    bytes,
                });
                Ok(TransferOutcome::ExportHandedOff(outcome))
            }
            (ScriptedReply::Complete(_), TransferRequest::ChooseImport { kind, max_bytes }) => {
                match state.import_bytes.clone() {
                    None => Ok(TransferOutcome::Cancelled),
                    // Read at most one byte past the limit, then refuse.
                    Some(bytes) if bytes.len() > max_bytes => {
                        Err(TransferError::new(TransferFailureCode::TooLarge))
                    }
                    Some(bytes) => Ok(TransferOutcome::ImportChosen { kind, bytes }),
                }
            }
            (ScriptedReply::WrongShape, TransferRequest::HandOffExport { kind, .. }) => {
                Ok(TransferOutcome::ImportChosen {
                    kind,
                    bytes: Box::from(&b"{}"[..]),
                })
            }
            (ScriptedReply::WrongShape, TransferRequest::ChooseImport { .. }) => Ok(
                TransferOutcome::ExportHandedOff(HandoffOutcome::DownloadStarted),
            ),
        };
        TransferJobState::Complete(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handoff(bytes: &[u8]) -> TransferRequest {
        TransferRequest::HandOffExport {
            kind: TransferKind::WorkshopArchive,
            suggested_name: SuggestedName::workshop_archive(&SlotName::new("Forge").unwrap()),
            bytes: Box::from(bytes),
        }
    }

    #[test]
    fn suggested_archive_names_keep_only_safe_characters() {
        for (name, expected) in [
            ("Two-System Forge", "Two-System-Forge.nyonworkshop.json"),
            (
                r#"a/b\c:d*e?f"g<h>i|j"#,
                "a_b_c_d_e_f_g_h_i_j.nyonworkshop.json",
            ),
            (".hidden", "_hidden.nyonworkshop.json"),
            ("..", "__.nyonworkshop.json"),
            ("snake_case 42", "snake_case-42.nyonworkshop.json"),
            (
                "~`!@#$%^&()+={}[];',",
                "____________________.nyonworkshop.json",
            ),
        ] {
            let suggested = SuggestedName::workshop_archive(&SlotName::new(name).unwrap());
            assert_eq!(suggested.as_str(), expected, "{name}");
            assert_eq!(suggested.to_string(), expected);
        }
        // The widest name a slot can have stays one path component.
        let widest = SlotName::new("/".repeat(64)).unwrap();
        let suggested = SuggestedName::workshop_archive(&widest);
        assert_eq!(suggested.as_str().len(), 64 + ".nyonworkshop.json".len());
        assert!(!suggested.as_str().contains('/'));
    }

    #[test]
    fn suggested_pack_names_use_a_sixteen_digit_hash_prefix() {
        // Only the first eight bytes may reach the name.
        let mut hash = [0xff_u8; 32];
        for (index, byte) in hash.iter_mut().take(8).enumerate() {
            *byte = u8::try_from(index * 17).unwrap();
        }
        assert_eq!(
            SuggestedName::content_pack(CatalogHash(hash)).as_str(),
            "0011223344556677.nyonpack.json"
        );
        assert_eq!(
            SuggestedName::content_pack(CatalogHash([0xab; 32])).as_str(),
            "abababababababab.nyonpack.json"
        );
    }

    #[test]
    fn kinds_carry_the_store_limits_and_hints() {
        assert_eq!(
            TransferKind::WorkshopArchive.max_bytes(),
            MAX_WORKSHOP_ARCHIVE_BYTES
        );
        assert_eq!(
            TransferKind::ContentPack.max_bytes(),
            MAX_WORKSHOP_PACK_BYTES
        );
        assert_eq!(TransferKind::ContentPack.extension(), ".nyonpack.json");
        assert_eq!(
            TransferKind::WorkshopArchive.media_type(),
            "application/json"
        );
    }

    #[test]
    fn requests_are_bounded_before_anything_starts() {
        let too_large = TransferError::new(TransferFailureCode::TooLarge);
        assert_eq!(handoff(b"{}").check(), Ok(()));
        assert_eq!(handoff(b"").check(), Err(too_large));
        let over = TransferRequest::HandOffExport {
            kind: TransferKind::ContentPack,
            suggested_name: SuggestedName::content_pack(CatalogHash([0; 32])),
            bytes: vec![b' '; MAX_WORKSHOP_PACK_BYTES + 1].into_boxed_slice(),
        };
        assert_eq!(over.check(), Err(too_large));
        let at = TransferRequest::HandOffExport {
            kind: TransferKind::ContentPack,
            suggested_name: SuggestedName::content_pack(CatalogHash([0; 32])),
            bytes: vec![b' '; MAX_WORKSHOP_PACK_BYTES].into_boxed_slice(),
        };
        assert_eq!(at.check(), Ok(()));
        for (max_bytes, expected) in [
            (0, Err(too_large)),
            (1, Ok(())),
            (MAX_WORKSHOP_PACK_BYTES, Ok(())),
            (MAX_WORKSHOP_PACK_BYTES + 1, Err(too_large)),
        ] {
            let request = TransferRequest::ChooseImport {
                kind: TransferKind::ContentPack,
                max_bytes,
            };
            assert_eq!(request.check(), expected, "{max_bytes}");
        }

        let mut adapter = ScriptedTransfer::default();
        assert_eq!(adapter.start(handoff(b"")), Err(too_large));
        assert!(adapter.started().is_empty());
        assert!(!adapter.busy());
    }

    #[test]
    fn only_durable_saves_say_saved() {
        for outcome in [
            HandoffOutcome::DownloadStarted,
            HandoffOutcome::HandedToSystem,
            HandoffOutcome::Written,
        ] {
            assert!(
                !outcome.label().to_ascii_lowercase().contains("saved"),
                "{outcome:?}"
            );
        }
        assert_eq!(HandoffOutcome::DownloadStarted.label(), "Download started.");
        assert_eq!(
            HandoffOutcome::HandedToSystem.label(),
            "Copy handed to the operating system."
        );
        assert_eq!(HandoffOutcome::Written.label(), "Copy written.");
        assert_eq!(HandoffOutcome::DurablySaved.label(), "Copy saved.");
    }

    #[test]
    fn the_adapter_holds_one_job_and_answers_it_once() {
        let mut adapter = ScriptedTransfer::default();
        let handle = adapter.clone();
        handle.set_pending_polls(1);
        let job = adapter.start(handoff(b"{\"a\":1}")).unwrap();
        assert!(handle.busy());
        assert_eq!(
            adapter.start(handoff(b"{}")),
            Err(TransferError::new(TransferFailureCode::Busy))
        );
        assert_eq!(
            adapter.poll(TransferJobId(job.0 + 1)),
            TransferJobState::Unknown
        );
        assert_eq!(adapter.poll(job), TransferJobState::Pending);
        assert!(
            handle.handed_off().is_empty(),
            "nothing is handed off early"
        );
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::ExportHandedOff(
                HandoffOutcome::DownloadStarted
            )))
        );
        assert_eq!(adapter.poll(job), TransferJobState::Unknown);
        assert!(!handle.busy());
        assert_eq!(
            handle.handed_off(),
            vec![ScriptedHandoff {
                kind: TransferKind::WorkshopArchive,
                suggested_name: SuggestedName::workshop_archive(&SlotName::new("Forge").unwrap()),
                bytes: Box::from(&b"{\"a\":1}"[..]),
            }]
        );
        assert_eq!(handle.started().len(), 1);

        let second = adapter.start(handoff(b"{}")).unwrap();
        assert_ne!(second, job);
    }

    #[test]
    fn abandoning_frees_the_job_and_forgets_its_outcome() {
        let mut adapter = ScriptedTransfer::default();
        let handle = adapter.clone();
        handle.set_pending_polls(3);
        let job = adapter.start(handoff(b"{}")).unwrap();
        assert!(!adapter.abandon(TransferJobId(job.0 + 1)));
        assert!(adapter.abandon(job));
        assert!(!adapter.abandon(job));
        assert_eq!(handle.abandoned(), 1);
        assert_eq!(adapter.poll(job), TransferJobState::Unknown);
        assert!(handle.handed_off().is_empty());
        adapter.start(handoff(b"{}")).unwrap();
    }

    #[test]
    fn scripted_replies_cover_every_terminal_shape() {
        let mut adapter = ScriptedTransfer::default();
        let handle = adapter.clone();
        handle.push_reply(ScriptedReply::Cancel);
        handle.push_reply(ScriptedReply::Fail(TransferFailureCode::Platform));
        handle.push_reply(ScriptedReply::Refuse(TransferFailureCode::Busy));
        handle.push_reply(ScriptedReply::WrongShape);
        handle.push_reply(ScriptedReply::Forget);
        handle.set_default_reply(ScriptedReply::Complete(HandoffOutcome::DurablySaved));

        let job = adapter.start(handoff(b"{}")).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::Cancelled))
        );
        let job = adapter.start(handoff(b"{}")).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Err(TransferError::new(TransferFailureCode::Platform)))
        );
        assert_eq!(
            adapter.start(handoff(b"{}")),
            Err(TransferError::new(TransferFailureCode::Busy))
        );
        assert!(!handle.busy(), "a refused start holds nothing");
        let job = adapter.start(handoff(b"{}")).unwrap();
        assert!(matches!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::ImportChosen { .. }))
        ));
        let job = adapter.start(handoff(b"{}")).unwrap();
        assert_eq!(adapter.poll(job), TransferJobState::Unknown);
        assert!(!handle.busy());
        let job = adapter.start(handoff(b"{}")).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::ExportHandedOff(
                HandoffOutcome::DurablySaved
            )))
        );
        assert_eq!(handle.handed_off().len(), 1, "only the completed handoff");
        assert_eq!(
            handle.started().len(),
            5,
            "the refused start is not recorded"
        );
    }

    #[test]
    fn imports_deliver_at_most_the_requested_bytes() {
        let mut adapter = ScriptedTransfer::default();
        let handle = adapter.clone();
        let choose = |max_bytes| TransferRequest::ChooseImport {
            kind: TransferKind::ContentPack,
            max_bytes,
        };

        let job = adapter.start(choose(4)).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::Cancelled)),
            "no file chosen"
        );

        handle.set_import_bytes(Some(Box::from(&b"abcd"[..])));
        let job = adapter.start(choose(4)).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::ImportChosen {
                kind: TransferKind::ContentPack,
                bytes: Box::from(&b"abcd"[..]),
            }))
        );
        let job = adapter.start(choose(3)).unwrap();
        assert_eq!(
            adapter.poll(job),
            TransferJobState::Complete(Err(TransferError::new(TransferFailureCode::TooLarge)))
        );

        handle.push_reply(ScriptedReply::WrongShape);
        let job = adapter.start(choose(4)).unwrap();
        assert!(matches!(
            adapter.poll(job),
            TransferJobState::Complete(Ok(TransferOutcome::ExportHandedOff(_)))
        ));
        assert!(handle.handed_off().is_empty());
    }

    #[test]
    fn the_protocol_is_object_safe() {
        let mut boxed: Box<dyn TransferAdapter> = Box::new(ScriptedTransfer::default());
        let job = boxed.start(handoff(b"{}")).unwrap();
        assert!(matches!(boxed.poll(job), TransferJobState::Complete(Ok(_))));
    }

    #[test]
    fn failure_messages_are_fixed_text() {
        for code in [
            TransferFailureCode::Unavailable,
            TransferFailureCode::Busy,
            TransferFailureCode::TooLarge,
            TransferFailureCode::Exhausted,
            TransferFailureCode::Platform,
            TransferFailureCode::Protocol,
        ] {
            assert_eq!(TransferError::new(code).to_string(), code.message());
            assert!(code.message().ends_with('.'));
        }
    }
}
