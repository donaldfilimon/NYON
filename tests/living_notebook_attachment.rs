//! A validated notebook is still scoped to the exact retained history document.
use nyon::sandbox_session::{SandboxSession, SandboxUiAction};
use nyon_workshop_core::living::*;

fn head_notebook(authority: &LivingGalaxyAuthorityV2) -> ValidatedLivingNotebookV2 {
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let head = authority.bookmark().unwrap();
    let notebook = LivingNotebookV2::seal(
        &archive,
        vec![LivingNamedBookmarkV2 {
            ordinal: 1,
            label: LivingNameV2::new("Recorded full head").unwrap(),
            view: head.view,
            digest: head.digest,
        }],
        vec![],
    )
    .unwrap();
    let mut job =
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive).unwrap();
    while job.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    job.finish().unwrap()
}

#[test]
fn unrelated_future_notebook_cannot_install_into_same_genesis_origin() {
    let origin = builtin_first_expansion_v2().unwrap();
    let mut future = origin.clone();
    future.step().unwrap();
    future.step().unwrap();
    let validated = head_notebook(&future);
    let mut resident = SandboxSession::from_authority(origin);
    resident
        .apply_action(SandboxUiAction::RecordBookmark)
        .unwrap();
    let state = resident.bookmark().unwrap();
    let metadata = resident.notebook_snapshot();
    let generation = resident.notebook_generation();
    let dirty = resident.dirty();
    let result = resident.install_notebook(validated);
    assert!(
        result.is_err(),
        "notebook for tick2 was installed into document ending at tick0"
    );
    assert_eq!(resident.bookmark().unwrap(), state);
    assert_eq!(resident.notebook_snapshot(), metadata);
    assert_eq!(resident.notebook_generation(), generation);
    assert_eq!(resident.dirty(), dirty);
}

#[test]
fn older_continue_view_can_install_notebook_validated_against_retained_head() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    authority.step().unwrap();
    authority.step().unwrap();
    let head = authority.bookmark().unwrap();
    let validated = head_notebook(&authority);
    let mut job = begin_restore_living_view_v2(
        &authority,
        LivingHistoryViewV2 {
            tick: LivingTickV2(0),
            ..head.view
        },
    )
    .unwrap();
    while job.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    let past = job.finish().unwrap().into_authority();
    assert_eq!(past.state().tick, LivingTickV2(0));
    assert_eq!(
        past.retained_history_branch(head.view.branch)
            .unwrap()
            .state()
            .tick,
        LivingTickV2(2)
    );
    let before = past.bookmark().unwrap();
    let mut resident = SandboxSession::from_authority(past);
    resident.install_notebook(validated).unwrap();
    assert_eq!(resident.bookmarks(), &[head]);
    assert_eq!(resident.bookmark().unwrap(), before);
}
