//! Replay a portable browser/native export with the native authority.
//! This establishes transfer/digest parity, not a native UI journey or work-budget proof.
use nyon_workshop_core::living::{
    LivingArchiveReplayStatusV2, begin_decode_living_notebook_v2, decode_living_archive_v2,
    decode_living_catalog_pack_v2,
};
use std::{fs::File, io::Read};

fn bounded_file(path: &str, cap: usize) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > cap {
        return Err("portable file exceeds its accepted size limit".into());
    }
    Ok(bytes)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !(3..=4).contains(&args.len()) {
        return Err(
            "usage: living_archive_verify ARCHIVE CATALOG NOTEBOOK [EXPECTED_DIGEST]".into(),
        );
    }
    let catalog = decode_living_catalog_pack_v2(&bounded_file(&args[1], 2 * 1024 * 1024)?)?;
    let archive = decode_living_archive_v2(&bounded_file(&args[0], 32 * 1024 * 1024)?, &catalog)?;
    let bookmark = archive.authority().bookmark()?;
    let digest = serde_json::to_value(bookmark.digest)?;
    if args
        .get(3)
        .is_some_and(|expected| digest.as_str() != Some(expected))
    {
        return Err("native replay differs from the expected exported digest".into());
    }
    let mut notes =
        begin_decode_living_notebook_v2(&bounded_file(&args[2], 1024 * 1024)?, archive)?;
    while notes.poll(1024)? == LivingArchiveReplayStatusV2::Pending {}
    let notes = notes.finish()?;
    println!(
        "{}",
        serde_json::json!({
            "bookmark": bookmark,
            "bookmarks": notes.notebook().bookmarks.len(),
            "experiments": notes.notebook().experiments.len(),
            "boundary": "native authority replay of portable export; no UI or strict-budget claim"
        })
    );
    Ok(())
}
