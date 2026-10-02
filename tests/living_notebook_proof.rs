//! Independently checks the immutable retained-history proof carried by validated notes.
use nyon_workshop_core::living::*;
fn proof(authority: &LivingGalaxyAuthorityV2) -> ValidatedLivingNotebookV2 {
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let notebook = LivingNotebookV2::seal(&archive, vec![], vec![]).unwrap();
    let mut job =
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive).unwrap();
    while job.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    job.finish().unwrap()
}
fn edit(authority: &LivingGalaxyAuthorityV2, mode: LivingCommandModeV2) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    let civs = authority.state().civilizations.as_slice();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Existing { id: civs[0].id },
                to: LivingCommandTargetV2::Existing { id: civs[1].id },
                base_disposition: -30,
            }],
        },
    }
}
#[test]
fn proof_distinguishes_paused_and_running_mode_even_same_revision_identity() {
    let original = builtin_living_sandbox_v2().unwrap();
    let mut running = original.clone();
    running
        .submit(edit(&running, LivingCommandModeV2::Running))
        .unwrap();
    running.step().unwrap();
    let mut paused = original;
    paused.step().unwrap();
    paused
        .apply_paused(edit(&paused, LivingCommandModeV2::Paused))
        .unwrap();
    assert_eq!(
        running.revisions()[0].id,
        paused.revisions()[0].id,
        "fixture isolates retained mode metadata"
    );
    let validated = proof(&running);
    assert!(validated.matches_retained_history(&running).unwrap());
    assert!(!validated.matches_retained_history(&paused).unwrap());
}
#[test]
fn retained_document_extension_fork_and_pending_input_require_fresh_proof() {
    let original = builtin_living_sandbox_v2().unwrap();
    let validated = proof(&original);
    let mut later = original.clone();
    later.step().unwrap();
    assert!(!validated.matches_retained_history(&later).unwrap());
    let mut forked = original.clone();
    forked.fork_view_paused(vec![]).unwrap();
    assert!(!validated.matches_retained_history(&forked).unwrap());
    let mut pending = original;
    pending
        .submit(edit(&pending, LivingCommandModeV2::Running))
        .unwrap();
    assert!(!validated.matches_retained_history(&pending).unwrap());
}

#[test]
fn session_genesis_check_rejects_other_genesis_without_metadata_mutation() {
    use nyon::sandbox_session::SandboxSession;
    let original = builtin_living_sandbox_v2().unwrap();
    let validated = proof(&original);
    let bytes = encode_living_archive_v2(&original).unwrap();
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut manifest: LivingGenesisManifestV2 =
        serde_json::from_value(document["genesis_manifest"].clone()).unwrap();
    let mut worlds = manifest.state.worlds.clone().into_vec();
    worlds[0].inventory.energy += 1;
    manifest.state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let genesis = validate_living_genesis_manifest_v2(manifest, original.catalog()).unwrap();
    let other = LivingGalaxyAuthorityV2::from_genesis(
        original.catalog().clone(),
        original.genesis_seed(),
        original.generator().clone(),
        genesis,
    )
    .unwrap();
    assert_ne!(
        original.genesis_digest().unwrap(),
        other.genesis_digest().unwrap()
    );
    let mut resident = SandboxSession::from_authority(other);
    let before = resident.notebook_snapshot();
    let generation = resident.notebook_generation();
    assert!(resident.install_notebook(validated).is_err());
    assert_eq!(resident.notebook_snapshot(), before);
    assert_eq!(resident.notebook_generation(), generation);
}
