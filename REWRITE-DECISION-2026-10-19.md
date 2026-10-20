# Rewrite decision — 2026-10-19

Status: Canceled — further Zig rewrite, migration, completion and cutover work.

Authority: Donald explicitly canceled the Zig rewrite plans and requested resumption of retained Rust implementations and consideration of the newest Rust release. This supersedes prior rewrite approvals and open continuation instructions.

Current: retained Rust workspace in Cargo.toml, src/ and crates/. Resume the Workshop Rust implementation using the native and platform gates documented in AGENTS.md.

Preservation: existing Zig code, commits, untracked files, dirty work and past qualification records remain intact. Historical checkboxes and evidence describe work already done or formerly planned; unfinished Zig tasks are canceled. No deployment, data migration or service cutover is performed.

Current compiler: Rust stable **1.99.1**, edition 2024. The retained root/core/atlas manifests and browser build scripts already select this compiler. Preserve the in-progress dependency and lockfile changes; further compatibility adjustments require a reproduced compiler failure.

Qualification policy: Donald paused tests and CI gates for this upgrade. Compilation-only Cargo checks are permitted, including compiling test targets without executing them. Do not run cargo test, Clippy, formatting gates, check-workshop.sh, fuzz execution or CI. Runtime, linking, platform and feature-parity acceptance remain separate and unverified. CI still pins nightly-2026-10-08; CI changes are deferred.

Recovery direction: port implemented Zig-only capabilities into versioned pure Rust authority interfaces, reusing retained Rust engine/UI/storage/tooling. Then improve the remaining full-game designs as Rust plans. Existing save identities and profiles remain unchanged; new gameplay uses separate versioned identities. No Zig continuation, Rust retirement, source deletion, commit, push or deployment is authorized.

See docs/qualification/2026-10-19-rust-stable-compilation.md and docs/qualification/2026-10-19-rust-feature-parity.md for evidence and remaining work.
