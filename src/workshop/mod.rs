//! Client-facing boundary for NYON Galaxy Workshop.
//!
//! Authoritative state, commands, history, and archives live in the pure
//! `nyon-workshop-core` crate. Platform pacing, persistence, presentation, and
//! input adapters are added beneath this facade and may depend on the root
//! client; the core crate never depends in the opposite direction.

pub mod session;
pub mod store;

pub use nyon_workshop_core::*;
