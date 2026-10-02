use nyon::{
    living_creator::{LivingCreatorDraft, LivingCreatorTool},
    sandbox_session::{SandboxSession, SandboxUiAction},
};
use nyon_workshop_core::living::{LivingEventProvenanceV2, builtin_living_sandbox_v2};

#[test]
fn every_creator_tool_produces_a_typed_operation_form_and_search_is_draft_only() {
    let authority = builtin_living_sandbox_v2().unwrap();
    let digest = authority.state().digest().unwrap();
    for tool in LivingCreatorTool::ALL {
        let draft = LivingCreatorDraft::new(tool, authority.state(), None);
        let operations = draft
            .operations()
            .unwrap_or_else(|error| panic!("{}: {error}", tool.label()));
        assert!(!operations.is_empty());
    }
    let mut draft = LivingCreatorDraft::new(LivingCreatorTool::System, authority.state(), None);
    draft.edit("query", "world").unwrap();
    assert!(
        draft
            .matching_tools()
            .iter()
            .all(|tool| tool.label().to_ascii_lowercase().contains("world"))
    );
    assert_eq!(authority.state().digest().unwrap(), digest);
}

#[test]
fn preview_is_private_apply_is_recorded_and_editing_invalidates_preview() {
    let mut session = SandboxSession::open().unwrap();
    let digest = session.state().digest().unwrap();
    let original = session.state().systems.as_slice().len();
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    session
        .edit_creator_field("0.name", "Experimental system")
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    assert_eq!(session.state().digest().unwrap(), digest);
    assert_eq!(session.state().systems.as_slice().len(), original);
    session
        .edit_creator_field("0.name", "Reviewed system")
        .unwrap();
    assert!(session.apply_action(SandboxUiAction::CreatorApply).is_err());
    assert_eq!(session.state().digest().unwrap(), digest);
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    assert_eq!(session.state().systems.as_slice().len(), original + 1);
    assert!(!session.creator_modal_open());
    assert_eq!(session.tick(), 0);
    let receipt = session.receipts().back().unwrap();
    receipt.verify().unwrap();
    assert!(
        receipt
            .events
            .iter()
            .all(|event| matches!(event.provenance, LivingEventProvenanceV2::Creator { .. }))
    );
}

#[test]
fn invalid_draft_and_cancel_preserve_authority_and_resume_prior_transport_intent() {
    let mut session = SandboxSession::open().unwrap();
    session.toggle_pause();
    let digest = session.state().digest().unwrap();
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    assert!(session.paused());
    session.edit_creator_field("0.name", "").unwrap();
    assert!(
        session
            .apply_action(SandboxUiAction::CreatorPreview)
            .is_err()
    );
    assert_eq!(session.state().digest().unwrap(), digest);
    session
        .apply_action(SandboxUiAction::CreatorCancel)
        .unwrap();
    assert!(!session.paused());
    assert_eq!(session.state().digest().unwrap(), digest);
}

#[test]
fn audio_and_import_previews_hold_transport_without_mutating_authority() {
    use glam::Vec2;
    use nyon::living_audio::LivingAudioSettings;
    let mut session = SandboxSession::open().unwrap();
    session.toggle_pause();
    let bookmark = session.bookmark().unwrap();
    session.set_audio_projection(LivingAudioSettings {
        muted: true,
        master: 75,
        ..LivingAudioSettings::default()
    });
    session.apply_action(SandboxUiAction::AudioPicker).unwrap();
    assert!(session.paused());
    let frame = session.frame(Vec2::new(900.0, 480.0), false);
    let mute = frame
        .controls
        .iter()
        .find(|control| control.action_id.as_str() == "sandbox.audio.mute")
        .unwrap();
    assert!(mute.label.contains("on"));
    assert!(mute.bounds.height() >= 44.0);
    session
        .apply_action(SandboxUiAction::CreatorCancel)
        .unwrap();
    assert!(!session.paused());
    session.set_import_preview(Some("Imported galaxy\n8 worlds, 3 civilizations"));
    assert!(session.paused());
    assert!(
        session
            .frame(Vec2::new(900.0, 480.0), false)
            .controls
            .iter()
            .any(|control| control.action
                == nyon::ui::platform::PlatformUiAction::Sandbox(SandboxUiAction::ApplyImport))
    );
    session.set_import_preview(None);
    assert!(!session.paused());
    assert_eq!(session.bookmark().unwrap(), bookmark);
}

#[test]
fn bounded_history_reconstruction_installs_only_after_polling_and_preserves_bookmarks() {
    use std::time::Duration;
    let mut session = SandboxSession::open().unwrap();
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    let edited = session.bookmark().unwrap();
    session
        .apply_action(SandboxUiAction::RecordBookmark)
        .unwrap();
    session.apply_action(SandboxUiAction::Undo).unwrap();
    assert_eq!(session.bookmark().unwrap(), edited);
    for _ in 0..100 {
        session.update(Duration::ZERO).unwrap();
    }
    assert_ne!(session.bookmark().unwrap(), edited);
    assert_eq!(session.bookmarks(), &[edited]);
    session
        .apply_action(SandboxUiAction::RestoreBookmark(0))
        .unwrap();
    for _ in 0..100 {
        session.update(Duration::ZERO).unwrap();
    }
    assert_eq!(session.bookmark().unwrap(), edited);
}

#[test]
fn modal_controls_fit_supported_scales_and_short_viewports() {
    use glam::Vec2;
    for scale in [0.85, 1.0, 1.15, 1.3] {
        for action in [
            SandboxUiAction::AudioPicker,
            SandboxUiAction::TransferPicker,
            SandboxUiAction::HistoryPicker,
            SandboxUiAction::ContentPicker,
            SandboxUiAction::OpenCreator(LivingCreatorTool::System),
        ] {
            let mut session = SandboxSession::open().unwrap();
            session.apply_action(action).unwrap();
            if action == SandboxUiAction::TransferPicker {
                session.set_transfer_capabilities(true, true, true);
                session.set_export_projection(true, "Exact snapshot prepared", [true; 3]);
            }
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            let modal = frame.modal.unwrap();
            let controls = frame
                .controls
                .iter()
                .filter(|control| control.enabled)
                .collect::<Vec<_>>();
            for control in &controls {
                assert!(control.bounds.height() >= 44.0, "{}", control.label);
                assert!(
                    modal.contains(control.bounds.min) && modal.contains(control.bounds.max),
                    "{} {:?}",
                    control.label,
                    control.bounds
                );
            }
            for (index, a) in controls.iter().enumerate() {
                for b in controls.iter().skip(index + 1) {
                    let overlap = (a.bounds.max.min(b.bounds.max) - a.bounds.min.max(b.bounds.min))
                        .min_element();
                    assert!(overlap <= 0.0, "{} overlaps {}", a.label, b.label);
                }
            }
        }
    }
}

#[test]
fn removal_preview_names_the_full_cascade_and_installs_only_on_apply() {
    let mut session = SandboxSession::open().unwrap();
    let selected = session.selected_world().unwrap();
    let digest = session.state().digest().unwrap();
    let worlds = session.state().worlds.as_slice().len();
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::Remove))
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    assert_eq!(session.state().digest().unwrap(), digest);
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    assert_eq!(session.state().worlds.as_slice().len(), worlds - 1);
    assert!(
        !session
            .state()
            .worlds
            .as_slice()
            .iter()
            .any(|row| row.id == selected)
    );
    assert!(
        !session
            .state()
            .facilities
            .as_slice()
            .iter()
            .any(|row| row.world == selected)
    );
    assert!(
        !session
            .state()
            .colonies
            .as_slice()
            .iter()
            .any(|row| row.world == selected)
    );
}

#[test]
fn unrelated_boundary_cannot_certify_corridor_but_exact_bookmark_remains_available() {
    let mut session = SandboxSession::open().unwrap();
    let genesis = session.bookmark().unwrap();
    assert!(
        session
            .apply_action(SandboxUiAction::RecordExperiment(3))
            .is_err()
    );
    assert_eq!(session.bookmark().unwrap(), genesis);
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    let boundary = session.bookmark().unwrap();
    let generation = session.notebook_generation();
    assert!(
        session
            .apply_action(SandboxUiAction::RecordExperiment(3))
            .is_err()
    );
    assert!(session.bookmarks().is_empty());
    assert_eq!(session.notebook_generation(), generation);
    session
        .apply_action(SandboxUiAction::RecordBookmark)
        .unwrap();
    assert_eq!(session.bookmark().unwrap(), boundary);
    assert_eq!(session.bookmarks(), &[boundary]);
    let receipt = session.bookmark_receipt(0).unwrap();
    receipt.verify().unwrap();
    assert_eq!(receipt.tick, boundary.view.tick);
    assert_eq!(receipt.state_digest, boundary.digest);
    assert_eq!(session.bookmark_experiment(0), None);
}

#[test]
fn library_lifecycle_controls_preserve_authority_and_rename_is_conventional_draft() {
    use glam::Vec2;
    use nyon::living_store::{LivingGenerationV2, LivingSlotDescriptorV2, LivingSlotIdV2};
    use nyon::ui::platform::PlatformUiAction;
    use nyon_workshop_core::living::LivingCatalogHashV2;
    let rows = [
        LivingSlotDescriptorV2 {
            id: LivingSlotIdV2(10),
            name: "Active galaxy".to_owned(),
            archived: false,
            generation: LivingGenerationV2(5),
            catalog_hash: LivingCatalogHashV2([3; 32]),
            diagnostic: None,
        },
        LivingSlotDescriptorV2 {
            id: LivingSlotIdV2(11),
            name: "Archived galaxy".to_owned(),
            archived: true,
            generation: LivingGenerationV2(9),
            catalog_hash: LivingCatalogHashV2([3; 32]),
            diagnostic: None,
        },
    ];
    for scale in [0.85, 1.0, 1.15, 1.3] {
        let mut session = SandboxSession::open().unwrap();
        session.toggle_pause();
        let bookmark = session.bookmark().unwrap();
        session.set_storage_projection("2 saved galaxies", &rows, false);
        session.open_library().unwrap();
        assert!(session.paused());
        for (index, row) in rows.iter().enumerate() {
            if index > 0 {
                session
                    .apply_action(SandboxUiAction::InspectorPage(false))
                    .unwrap();
            }
            let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
            let modal = frame.modal.unwrap();
            let controls = frame
                .controls
                .iter()
                .filter(|control| control.enabled)
                .collect::<Vec<_>>();
            assert!(frame.controls.iter().any(|control| control.action
                == PlatformUiAction::Sandbox(if row.archived {
                    SandboxUiAction::RestoreSave(row.id.0)
                } else {
                    SandboxUiAction::ArchiveSave(row.id.0)
                })
                && control.enabled));
            let delete = frame
                .controls
                .iter()
                .find(|control| {
                    control.action
                        == PlatformUiAction::Sandbox(SandboxUiAction::DeleteSave(row.id.0))
                })
                .unwrap();
            assert_eq!(delete.enabled, row.archived);
            for control in &controls {
                assert!(control.bounds.height() >= 44.0);
                assert!(modal.contains(control.bounds.min) && modal.contains(control.bounds.max));
            }
            for (position, a) in controls.iter().enumerate() {
                for b in controls.iter().skip(position + 1) {
                    assert!(
                        (a.bounds.max.min(b.bounds.max) - a.bounds.min.max(b.bounds.min))
                            .min_element()
                            <= 0.0,
                        "{} overlaps {}",
                        a.label,
                        b.label
                    );
                }
            }
        }
        session
            .apply_action(SandboxUiAction::RenameSave(10))
            .unwrap();
        assert_eq!(
            session.creator_field_value("save_name"),
            Some("Active galaxy")
        );
        session
            .edit_creator_field("save_name", "Edited galaxy")
            .unwrap();
        let frame = session.frame_with_scale(Vec2::new(800.0, 480.0), scale, false);
        assert!(frame.controls.iter().any(|control| control.action
            == PlatformUiAction::Sandbox(SandboxUiAction::ConfirmRenameSave(10))
            && control.enabled));
        session
            .edit_creator_field("save_name", " invalid ")
            .unwrap();
        assert!(
            !session
                .frame(Vec2::new(800.0, 480.0), false)
                .controls
                .iter()
                .any(|control| control.action
                    == PlatformUiAction::Sandbox(SandboxUiAction::ConfirmRenameSave(10))
                    && control.enabled)
        );
        session
            .apply_action(SandboxUiAction::CancelRenameSave)
            .unwrap();
        assert!(session.paused());
        session
            .apply_action(SandboxUiAction::CreatorCancel)
            .unwrap();
        assert!(!session.paused());
        assert_eq!(session.bookmark().unwrap(), bookmark);
        assert!(
            session
                .apply_action(SandboxUiAction::RenameSave(999))
                .is_err()
        );
        assert_eq!(session.bookmark().unwrap(), bookmark);
    }
}

fn apply_tool(session: &mut SandboxSession, tool: LivingCreatorTool) {
    session
        .apply_action(SandboxUiAction::OpenCreator(tool))
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap_or_else(|error| panic!("{} preview: {error}", tool.label()));
    session
        .apply_action(SandboxUiAction::CreatorApply)
        .unwrap_or_else(|error| panic!("{} apply: {error}", tool.label()));
}

#[test]
fn new_edit_forms_apply_typed_revisions_without_reminting_selected_entities() {
    for tool in [
        LivingCreatorTool::EditSystem,
        LivingCreatorTool::EditStar,
        LivingCreatorTool::EditWorld,
        LivingCreatorTool::EditLane,
        LivingCreatorTool::EditDeposit,
        LivingCreatorTool::EditCivilization,
        LivingCreatorTool::FacilityEnabled,
        LivingCreatorTool::EditFleet,
        LivingCreatorTool::EditHull,
    ] {
        let mut session = SandboxSession::open().unwrap();
        let worlds = session
            .state()
            .worlds
            .as_slice()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let original = session.bookmark().unwrap();
        apply_tool(&mut session, tool);
        assert_ne!(session.bookmark().unwrap(), original, "{}", tool.label());
        assert_eq!(
            session
                .state()
                .worlds
                .as_slice()
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            worlds
        );
        assert_eq!(session.tick(), 0);
    }
}

#[test]
fn split_merge_and_cancellation_forms_preserve_hull_identity_and_refund_jobs() {
    let mut session = SandboxSession::open().unwrap();
    let source = session
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| fleet.hulls.as_slice().len() > 1)
        .unwrap()
        .id;
    session
        .select_entity(nyon::living_navigation::LivingEntitySelection::new(
            nyon::living_navigation::LivingEntityKind::Fleet,
            source,
        ))
        .unwrap();
    let hulls = |session: &SandboxSession| {
        let mut ids = session
            .state()
            .fleets
            .as_slice()
            .iter()
            .flat_map(|row| row.hulls.as_slice().iter().map(|hull| hull.id))
            .collect::<Vec<_>>();
        ids.sort();
        ids
    };
    let original = hulls(&session);
    let fleets = session.state().fleets.as_slice().len();
    apply_tool(&mut session, LivingCreatorTool::SplitFleet);
    assert_eq!(session.state().fleets.as_slice().len(), fleets + 1);
    assert_eq!(hulls(&session), original);
    apply_tool(&mut session, LivingCreatorTool::MergeFleets);
    assert_eq!(session.state().fleets.as_slice().len(), fleets);
    assert_eq!(hulls(&session), original);
    apply_tool(&mut session, LivingCreatorTool::Construction);
    assert_eq!(session.state().construction_jobs.as_slice().len(), 1);
    let before = session
        .state()
        .worlds
        .as_slice()
        .iter()
        .map(|row| row.inventory.alloy as u64)
        .sum::<u64>();
    apply_tool(&mut session, LivingCreatorTool::CancelConstruction);
    assert!(session.state().construction_jobs.as_slice().is_empty());
    let after = session
        .state()
        .worlds
        .as_slice()
        .iter()
        .map(|row| row.inventory.alloy as u64)
        .sum::<u64>();
    assert!(after > before);
    apply_tool(&mut session, LivingCreatorTool::HullJob);
    assert_eq!(session.state().hull_jobs.as_slice().len(), 1);
    apply_tool(&mut session, LivingCreatorTool::CancelHullJob);
    assert!(session.state().hull_jobs.as_slice().is_empty());
}

#[test]
fn route_hazard_and_scrapping_forms_preserve_existing_manifests() {
    use nyon::living_content::LivingContent;
    let mut session = SandboxSession::from_authority(LivingContent::Commerce.open().unwrap());
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    let routes = session
        .state()
        .routes
        .as_slice()
        .iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    let shipments = session.state().shipments.clone();
    apply_tool(&mut session, LivingCreatorTool::EditRoute);
    assert_eq!(
        session
            .state()
            .routes
            .as_slice()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        routes
    );
    assert_eq!(session.state().shipments, shipments);
    apply_tool(&mut session, LivingCreatorTool::RouteSuspended);
    assert!(
        session
            .state()
            .routes
            .as_slice()
            .iter()
            .any(|row| row.suspended)
    );
    apply_tool(&mut session, LivingCreatorTool::RouteSuspended);
    let mut session =
        SandboxSession::from_authority(LivingContent::InterruptedCorridor.open().unwrap());
    let hazard = session.state().hazards.as_slice()[0].id;
    apply_tool(&mut session, LivingCreatorTool::EditHazard);
    assert_eq!(session.state().hazards.as_slice()[0].id, hazard);
    apply_tool(&mut session, LivingCreatorTool::CancelHazard);
    assert!(session.state().hazards.as_slice().is_empty());
    let facilities = session.state().facilities.as_slice().len();
    apply_tool(&mut session, LivingCreatorTool::ScrapFacility);
    assert_eq!(session.state().facilities.as_slice().len(), facilities - 1);
}

#[test]
fn branch_here_previews_and_retains_an_unedited_genesis_future() {
    let mut session = SandboxSession::open().unwrap();
    let original = session.bookmark().unwrap();
    session.apply_action(SandboxUiAction::BeginBranch).unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    assert_eq!(session.bookmark().unwrap(), original);
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    let fork = session.bookmark().unwrap();
    assert_ne!(fork.view.branch, original.view.branch);
    assert_eq!(fork.view.tick, original.view.tick);
    assert!(fork.view.revision.is_some());
}

#[test]
fn notebook_names_and_save_acknowledgements_preserve_later_metadata() {
    let mut session = SandboxSession::open().unwrap();
    let boundary = session.bookmark().unwrap();
    session.mark_saved(boundary);
    assert!(!session.dirty());
    session
        .apply_action(SandboxUiAction::HistoryPicker)
        .unwrap();
    session
        .edit_creator_field("bookmark_label", "Before intervention")
        .unwrap();
    session
        .apply_action(SandboxUiAction::RecordBookmark)
        .unwrap();
    let captured = session.notebook_generation();
    assert!(session.dirty());
    session
        .edit_creator_field("bookmark_label", "Named observation")
        .unwrap();
    session
        .apply_action(SandboxUiAction::RenameBookmark(0))
        .unwrap();
    assert!(!session.mark_notebook_saved(captured));
    assert!(session.dirty());
    let (bookmarks, experiments) = session.notebook_snapshot();
    assert_eq!(bookmarks[0].ordinal, 1);
    assert_eq!(bookmarks[0].label.as_str(), "Named observation");
    assert_eq!(bookmarks[0].digest, boundary.digest);
    assert!(experiments.is_empty());
    let generation = session.notebook_generation();
    session.edit_creator_field("bookmark_label", "").unwrap();
    assert!(
        session
            .apply_action(SandboxUiAction::RenameBookmark(0))
            .is_err()
    );
    assert_eq!(session.notebook_generation(), generation);
    assert_eq!(session.notebook_snapshot().0, bookmarks);
    assert!(session.mark_notebook_saved(generation));
    assert!(!session.dirty());
    assert!(session.record_comparison_experiment().is_err());
}

#[test]
fn notebook_experiments_measure_completed_matching_tick_states() {
    use nyon_workshop_core::living::measure_living_notebook_v2;
    let mut session = SandboxSession::open().unwrap();
    session
        .apply_action(SandboxUiAction::RecordBookmark)
        .unwrap();
    session.apply_action(SandboxUiAction::BeginBranch).unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    session.apply_action(SandboxUiAction::CreatorApply).unwrap();
    session
        .apply_action(SandboxUiAction::CompareBookmark(0))
        .unwrap();
    assert!(session.record_comparison_experiment().is_err());
    for _ in 0..100 {
        session.update(std::time::Duration::ZERO).unwrap();
    }
    let boundary = session.bookmark().unwrap();
    session.record_comparison_experiment().unwrap();
    let (bookmarks, experiments) = session.notebook_snapshot();
    assert_eq!(bookmarks.len(), 3);
    assert_eq!(experiments.len(), 1);
    let comparison = session.comparison().unwrap();
    let experiment = &experiments[0];
    assert_eq!(experiment.comparison_tick, comparison.tick);
    for observation in &experiment.observations {
        assert_eq!(
            observation.left,
            measure_living_notebook_v2(
                &comparison.left_state,
                observation.metric,
                observation.entity
            )
            .unwrap()
        );
        assert_eq!(
            observation.right,
            measure_living_notebook_v2(
                &comparison.right_state,
                observation.metric,
                observation.entity
            )
            .unwrap()
        );
    }
    assert_eq!(session.bookmark().unwrap(), boundary);
}

#[test]
fn inspected_creator_targets_keep_the_exact_record_fields_and_unowned_context_needs_owner_choice() {
    use nyon::living_navigation::{LivingEntityKind as K, LivingEntitySelection as S};
    use nyon_workshop_core::living::LivingEntityIdV2;
    let authority = nyon::living_content::LivingContent::ValeConfluence
        .open()
        .unwrap();
    let state = authority.state();
    let star = &state.stars.as_slice()[1];
    let draft = LivingCreatorDraft::new_for_selection(
        LivingCreatorTool::EditStar,
        state,
        Some(S::new(K::Star, star.id)),
    );
    let value = serde_json::to_value(draft.operations().unwrap()).unwrap();
    let hex = |id: LivingEntityIdV2| {
        id.0.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    assert_eq!(value[0]["target"]["id"], hex(star.id));
    assert_eq!(value[0]["system"]["id"], hex(star.system));
    assert_eq!(value[0]["name"], star.name.as_str());
    assert_eq!(value[0]["archetype"], star.archetype.as_str());
    let deposit = &state.deposits.as_slice()[1];
    let draft = LivingCreatorDraft::new_for_selection(
        LivingCreatorTool::EditDeposit,
        state,
        Some(S::new(K::Deposit, deposit.id)),
    );
    let value = serde_json::to_value(draft.operations().unwrap()).unwrap();
    assert_eq!(value[0]["target"]["id"], hex(deposit.id));
    assert_eq!(value[0]["remaining_units"], deposit.remaining_units);
    let world = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.owner.is_none())
        .unwrap();
    let draft = LivingCreatorDraft::new_for_selection(
        LivingCreatorTool::Fleet,
        state,
        Some(S::new(K::World, world.id)),
    );
    let value = serde_json::to_value(draft.operations().unwrap()).unwrap();
    assert_eq!(value[0]["world"]["id"], hex(world.id));
    assert_eq!(value[0]["owner"]["id"], "00000000000000000000000000000000");
    let civilization = &state.civilizations.as_slice()[1];
    let draft = LivingCreatorDraft::new_for_selection(
        LivingCreatorTool::EditCivilization,
        state,
        Some(S::new(K::Civilization, civilization.id)),
    );
    let value = serde_json::to_value(draft.operations().unwrap()).unwrap();
    assert_eq!(value[0]["target"]["id"], hex(civilization.id));
    assert_eq!(value[0]["name"], civilization.name.as_str());
    assert_eq!(value[0]["policy"], civilization.policy.0.as_str());
}

#[test]
fn private_preview_fit_reveals_off_scope_changes_above_the_dock_and_rejects_invalidated_preview() {
    use glam::Vec2;
    use nyon::{living_camera::LivingCameraLevel, ui::platform::PlatformRect};
    let mut session = SandboxSession::open().unwrap();
    session
        .apply_action(SandboxUiAction::NavigateWorld)
        .unwrap();
    assert_eq!(session.camera_scope().level, LivingCameraLevel::World);
    let bookmark = session.bookmark();
    let selected = session.selected_entity();
    session
        .apply_action(SandboxUiAction::OpenCreator(LivingCreatorTool::System))
        .unwrap();
    session
        .edit_creator_field("0.name", "Off scope preview")
        .unwrap();
    session
        .apply_action(SandboxUiAction::CreatorPreview)
        .unwrap();
    for scale in [0.85, 1.5] {
        let viewport = Vec2::new(800.0, 480.0);
        let frame = session.frame_with_scale(viewport, scale, false);
        let bounds = PlatformRect::from_xywh(0.0, 0.0, viewport.x, viewport.y);
        session.scene_frame(bounds);
        session
            .apply_action(SandboxUiAction::CreatorFitPreview)
            .unwrap();
        assert_eq!(session.camera_scope().level, LivingCameraLevel::Galaxy);
        let scene = session.scene_frame(bounds);
        assert!(!scene.ghosts.is_empty());
        let live_ids = nyon::living_navigation::navigator_rows(session.state())
            .into_iter()
            .map(|row| row.selection)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(scene.ghosts.iter().all(|ghost| ghost.kind
            == nyon::presentation::living::LivingGhostKind::Added
            && !live_ids.contains(&ghost.selection)));
        for ghost in &scene.ghosts {
            assert!(bounds.contains(ghost.position));
            assert!(ghost.position.y < frame.modal.unwrap().min.y - 8.0);
        }
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            assert!(control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0);
            assert!(control.bounds.max.x <= viewport.x && control.bounds.max.y <= viewport.y);
            let node = frame.semantics.node(control.semantic_id.as_str()).unwrap();
            assert_eq!(node.bounds, Some(control.bounds.into()));
        }
    }
    assert_eq!(session.bookmark(), bookmark);
    assert_eq!(session.selected_entity(), selected);
    let camera = session.camera_state();
    session
        .edit_creator_field("0.name", "Invalidated preview")
        .unwrap();
    assert!(
        session
            .apply_action(SandboxUiAction::CreatorFitPreview)
            .is_err()
    );
    assert_eq!(session.camera_state(), camera);
    assert_eq!(session.bookmark(), bookmark);
}
