//! Paired deterministic endurance evidence for zero, one and several societies.
//! This does not qualify renderer performance or the declared capacity fixture.
use nyon::living_content::LivingContent;
use nyon_workshop_core::living::{builtin_first_expansion_v2, living_core_pack_v2};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ticks = std::env::args()
        .nth(1)
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(36_000);
    let catalog = living_core_pack_v2()?;
    for (name, first, second) in [
        (
            "zero",
            LivingContent::Blank.open(),
            LivingContent::Blank.open(),
        ),
        (
            "one",
            builtin_first_expansion_v2(),
            builtin_first_expansion_v2(),
        ),
        (
            "many",
            LivingContent::ValeConfluence.open(),
            LivingContent::ValeConfluence.open(),
        ),
    ] {
        let mut left = first.map_err(std::io::Error::other)?;
        let mut right = second.map_err(std::io::Error::other)?;
        let civilizations = left.state().civilizations.as_slice().len();
        let started = Instant::now();
        for boundary in 1..=ticks {
            let left_receipt = left.step()?;
            let right_receipt = right.step()?;
            if left_receipt != right_receipt || left.last_decision() != right.last_decision() {
                return Err(format!("{name} diverged at boundary {boundary}").into());
            }
            if boundary % 1_000 == 0 || boundary == ticks {
                if left.state() != right.state() {
                    return Err(format!("{name} states diverged at boundary {boundary}").into());
                }
                left.state().validate(&catalog)?;
            }
        }
        println!(
            "{}",
            serde_json::json!({
                "fixture": name,
                "initial_civilizations": civilizations,
                "ticks": left.state().tick.0,
                "elapsed_ms": started.elapsed().as_millis(),
                "digest": left.state().digest()?,
            "receipt": left.last_receipt().map(|receipt| receipt.digest),
                "worlds": left.state().worlds.as_slice().len(),
                "fleets": left.state().fleets.as_slice().len(),
                "shipments": left.state().shipments.as_slice().len(),
                "boundary": "paired native authority only; no renderer, wasm or capacity evidence"
            })
        );
    }
    Ok(())
}
