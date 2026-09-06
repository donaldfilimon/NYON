mod common;

use nyon_workshop_core::{
    ArchiveDecodeJob, ArchiveDecodeStatus, CreatorBatchV1, CreatorOpV1, StateDigest,
    WorkshopHistory, WorkshopTick, decode_catalog_pack, encode_archive, encode_catalog_pack,
};

const MAX_ALLOY_WAIT_TICKS: u64 = 16;

#[test]
fn actual_eighteen_step_two_system_forge_journey_is_replayable() {
    let pack = common::pack();
    let mut completed_steps = 0_u8;

    // 1. Start a deterministic blank Workshop.
    let mut history = WorkshopHistory::from_seed_u64(pack.clone(), 0x4652_4745);
    assert!(history.state().systems.is_empty());
    completed_steps += 1;

    // 2-8. Shape the two systems, stars/worlds, factions/ownership, deposit,
    // industries, direct lane, and energy/ore routes as one recorded batch.
    let forge = common::create_forge(&mut history);
    assert_eq!(history.state().systems.len(), 2);
    completed_steps += 1;
    assert_eq!(history.state().stars.len(), 2);
    assert_eq!(history.state().worlds.len(), 2);
    completed_steps += 1;
    assert_eq!(history.state().factions.len(), 2);
    assert_eq!(history.state().owners.len(), 2);
    completed_steps += 1;
    assert_eq!(history.state().deposits[&forge.deposit].reserve_units, 100);
    completed_steps += 1;
    assert_eq!(history.state().industries.len(), 3);
    completed_steps += 1;
    assert_eq!(history.state().lanes.len(), 1);
    completed_steps += 1;
    assert_eq!(history.state().routes.len(), 2);
    completed_steps += 1;

    // 9. Run until alloy is produced.
    let production_tick = (0..MAX_ALLOY_WAIT_TICKS).find_map(|_| {
        let receipt = history.step().unwrap();
        (history.state().worlds[&forge.world_b]
            .inventory
            .get(&common::catalog("alloy"))
            .copied()
            .unwrap_or(0)
            > 0)
        .then_some(receipt.tick)
    });
    assert_eq!(production_tick, Some(WorkshopTick(3)));
    assert_eq!(history.state().tick, WorkshopTick(4));
    completed_steps += 1;

    // 10. Add a world while the session is running; creator authority remains immediate.
    history.set_paused(false);
    let running_edit = history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::CreateWorld {
                local: common::local(50),
                system: common::existing(forge.system_b),
                primary: common::existing(forge.star_b),
                name: common::name("Relay Minor"),
                archetype_id: common::catalog("rocky-world"),
                orbit_radius_milli_au: 1_600,
                orbit_period_ticks: 1_600,
                phase_millidegrees: 90_000,
            }],
        })
        .unwrap();
    let added_world = running_edit.entities[&common::local(50)];
    assert!(history.state().worlds.contains_key(&added_world));
    completed_steps += 1;

    // 11. Schedule and observe a half-open ion storm.
    let hazard_receipt = history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::ScheduleHazard {
                local: common::local(51),
                lane: common::existing(forge.lane),
                hazard_id: common::catalog("ion-storm"),
                start_tick: history.state().tick,
                duration_ticks: 2,
            }],
        })
        .unwrap();
    let main_branch = history.active_view().selected_branch;
    history.step().unwrap();
    let storm_digest = history.active_state_digest();
    completed_steps += 1;

    // 12. Pause and undo the storm batch at the same authoritative tick.
    history.set_paused(true);
    assert_eq!(history.undo().unwrap(), Some(running_edit.revision));
    assert_ne!(history.active_state_digest(), storm_digest);
    completed_steps += 1;

    // 13. Redo recovers the exact former digest.
    assert_eq!(history.redo().unwrap(), hazard_receipt.revision);
    assert_eq!(history.active_state_digest(), storm_digest);
    completed_steps += 1;

    // 14. Undo again and append an alternate edit, preserving the sibling.
    history.undo().unwrap();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::RenameObject {
                target: common::existing(added_world),
                name: common::name("Relay Alternate"),
            }],
        })
        .unwrap();
    let alternate_branch = history.active_view().selected_branch;
    let alternate_digest = history.active_state_digest();
    assert_ne!(alternate_branch, main_branch);
    assert_eq!(history.branches().len(), 2);
    completed_steps += 1;

    // 15. Switch between siblings and recover both digests.
    history.switch_branch(main_branch).unwrap();
    assert_eq!(history.active_state_digest(), storm_digest);
    history.switch_branch(alternate_branch).unwrap();
    assert_eq!(history.active_state_digest(), alternate_digest);
    completed_steps += 1;

    // 16. Save and continue from the explicitly selected branch.
    let archive = encode_archive(&history).unwrap();
    let continued = decode_incrementally(&pack, &archive.bytes);
    assert_eq!(continued.active_view().selected_branch, alternate_branch);
    completed_steps += 1;

    // 17. Export and re-import both the catalog and Workshop archive.
    let pack_bytes = encode_catalog_pack(&pack).unwrap();
    let imported_pack = decode_catalog_pack(&pack_bytes).unwrap();
    let imported = decode_incrementally(&imported_pack, &archive.bytes);
    completed_steps += 1;

    // 18. Recover catalog, graph, branch, tick, and final state digest exactly.
    assert_eq!(imported_pack.catalog_hash(), pack.catalog_hash());
    assert_eq!(imported.revisions(), history.revisions());
    assert_eq!(imported.branches(), history.branches());
    assert_eq!(imported.active_view(), history.active_view());
    assert_eq!(imported.active_state_digest(), alternate_digest);
    assert_eq!(
        alternate_digest,
        StateDigest([
            253, 241, 43, 228, 219, 29, 233, 251, 127, 49, 243, 239, 153, 115, 10, 251, 161, 124,
            207, 217, 172, 4, 64, 161, 43, 151, 187, 145, 208, 233, 127, 55,
        ])
    );
    completed_steps += 1;

    assert_eq!(completed_steps, 18);
}

fn decode_incrementally(
    catalog: &nyon_workshop_core::ValidatedCatalogPackV1,
    bytes: &[u8],
) -> WorkshopHistory {
    let mut job = ArchiveDecodeJob::new(catalog, bytes).unwrap();
    loop {
        match job.poll(64).unwrap() {
            ArchiveDecodeStatus::Pending => {}
            ArchiveDecodeStatus::Complete => return job.finish().unwrap(),
        }
    }
}
