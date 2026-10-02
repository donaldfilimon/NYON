//! Reproducible authority-only timing evidence; not renderer or cap qualification.
use nyon_workshop_core::living::{builtin_first_expansion_v2, builtin_living_sandbox_v2};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for (name, authority) in [
        ("demonstration", builtin_living_sandbox_v2()),
        ("first_expansion", builtin_first_expansion_v2()),
    ] {
        let mut authority = authority.map_err(std::io::Error::other)?;
        let mut samples = Vec::with_capacity(10_000);
        for _ in 0..10_000 {
            let start = Instant::now();
            authority.step()?;
            samples.push(start.elapsed().as_nanos());
        }
        samples.sort_unstable();
        println!(
            "{}",
            serde_json::json!({
                "fixture":name, "ticks":authority.state().tick.0,
                "p50_us":samples[5_000] as f64 / 1_000.0,
                "p95_us":samples[9_500] as f64 / 1_000.0,
                "max_us":samples[9_999] as f64 / 1_000.0,
            "digest":authority.state().digest()?,
                "boundary":"authority-only existing fixture; not declared capacity or frame time"
            })
        );
    }
    Ok(())
}
