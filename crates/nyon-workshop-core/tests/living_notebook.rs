//! Notebook evidence must survive exact replay and reject incompatible attachments.
use nyon_workshop_core::living::*;

fn archive(authority: &LivingGalaxyAuthorityV2) -> ValidatedLivingArchiveV2 {
    decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap()
}
fn named(authority: &LivingGalaxyAuthorityV2, ordinal: u64) -> LivingNamedBookmarkV2 {
    let bookmark = authority.bookmark().unwrap();
    LivingNamedBookmarkV2 {
        ordinal,
        label: LivingNameV2::new("Before intervention").unwrap(),
        view: bookmark.view,
        digest: bookmark.digest,
    }
}
fn validate(
    bytes: &[u8],
    archive: ValidatedLivingArchiveV2,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let mut job = begin_decode_living_notebook_v2(bytes, archive)?;
    while job.poll(128)? == LivingArchiveReplayStatusV2::Pending {}
    job.finish()
}
#[test]
fn named_exact_bookmarks_reload_without_serializing_checkpoints() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    let first = named(&authority, 1);
    for _ in 0..61 {
        authority.step().unwrap();
    }
    let second = named(&authority, 2);
    let notebook =
        LivingNotebookV2::seal(&archive(&authority), vec![first, second], vec![]).unwrap();
    let bytes = notebook.canonical_bytes().unwrap();
    let decoded = validate(&bytes, archive(&authority)).unwrap();
    assert_eq!(decoded.notebook(), &notebook);
    assert!(!std::str::from_utf8(&bytes).unwrap().contains("checkpoint"));
}
#[test]
fn incorrect_digest_and_wrong_archive_are_rejected_without_installation() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    let before = authority.bookmark().unwrap();
    let mut bookmark = named(&authority, 1);
    bookmark.digest = LivingStateDigestV2([0; 32]);
    let notebook = LivingNotebookV2::seal(&archive(&authority), vec![bookmark], vec![]).unwrap();
    assert!(matches!(
        validate(&notebook.canonical_bytes().unwrap(), archive(&authority)),
        Err(LivingNotebookErrorV2::Evidence)
    ));
    assert_eq!(authority.bookmark().unwrap(), before);
    authority.step().unwrap();
    assert!(matches!(
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive(&authority)),
        Err(LivingNotebookErrorV2::Attachment)
    ));
}
#[test]
fn matching_tick_observations_are_recomputed_and_type_checked() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..5 {
        authority.step().unwrap();
    }
    let metric = LivingNotebookMetricV2::Tick;
    let value = measure_living_notebook_v2(authority.state(), metric, None).unwrap();
    let experiment = LivingNotebookExperimentV2 {
        ordinal: 1,
        label: LivingNameV2::new("Matching tick comparison").unwrap(),
        left: 1,
        right: 1,
        comparison_tick: LivingTickV2(5),
        observations: vec![LivingNotebookObservationV2 {
            metric,
            entity: None,
            left: value.clone(),
            right: value,
        }],
    };
    let notebook = LivingNotebookV2::seal(
        &archive(&authority),
        vec![named(&authority, 1)],
        vec![experiment.clone()],
    )
    .unwrap();
    validate(&notebook.canonical_bytes().unwrap(), archive(&authority)).unwrap();
    let mut forged = experiment;
    forged.observations[0].left = LivingNotebookValueV2::Entity(None);
    let notebook = LivingNotebookV2::seal(
        &archive(&authority),
        vec![named(&authority, 1)],
        vec![forged],
    )
    .unwrap();
    assert!(matches!(
        validate(&notebook.canonical_bytes().unwrap(), archive(&authority)),
        Err(LivingNotebookErrorV2::Evidence)
    ));
    let world = authority.state().worlds.as_slice()[0].id;
    assert!(matches!(
        measure_living_notebook_v2(authority.state(), metric, Some(world)),
        Err(LivingNotebookErrorV2::Shape)
    ));
}
#[test]
fn cancellation_and_invalid_budget_preserve_private_validation_boundary() {
    let authority = builtin_first_expansion_v2().unwrap();
    let notebook =
        LivingNotebookV2::seal(&archive(&authority), vec![named(&authority, 1)], vec![]).unwrap();
    let mut job =
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive(&authority))
            .unwrap();
    assert!(matches!(
        job.poll(0),
        Err(LivingNotebookErrorV2::Replay(LivingArchiveErrorV2::Budget))
    ));
    assert!(job.poll(128).is_ok());
    job.cancel();
    assert!(matches!(
        job.poll(128),
        Err(LivingNotebookErrorV2::Cancelled)
    ));
    assert!(matches!(
        job.finish(),
        Err(LivingNotebookErrorV2::Cancelled)
    ));
}
