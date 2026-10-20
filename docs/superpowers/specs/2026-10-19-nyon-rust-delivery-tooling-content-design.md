# NYON Rust Platform Delivery, Tooling and Content Design

Status: Proposed Rust recovery design. The user approved the overall recovery sequence; this artifact does not claim subsystem/runtime acceptance.

**Goal:** Integrate recovered gameplay with retained Rust hosts and tooling, then qualify the complete game on required platforms.

**Architecture:** Keep winit/wgpu, existing Tokio native acquisition, wasm spawn_local generations, Rust fontdue/resvg atlas builder and existing storage adapters. Integrate missing semantics without introducing a second host or renderer. Content is versioned input to the same authority.

## Global constraints

- Rust stable 1.99.1, edition 2024; preserve `forbid(unsafe_code)` and pure-core dependencies.
- Existing source, dirty work, save identities, independent fixtures and historical evidence remain intact.
- Compilation only while Donald's test/CI pause applies. Test scenarios below are future acceptance, not executed or green.
- No commits, pushes, deployment, new production dependencies, automatic profile migration or Zig implementation work.
- Current evidence is in `docs/qualification/2026-10-19-rust-stable-compilation.md`; source revisions are pinned in `docs/qualification/2026-10-19-rust-recovery-inputs.md`.


## Platform and asset decisions

Rust stable1.99.1 owns active game/core/tool implementation. Keep the retained Rust platform architecture; canceled AppKit/Metal/Win32-only rewrite paths are historical. Native macOS is first; Windows/Linux and wasm WebGPU/WebGL2 remain required. Keep logical-pixel layout and the frozen36-byte Vertex contract. Native device acquisition returns to winit through generation mailbox; stale completions cannot install.

Reuse existing fontdue/resvg atlas generation and committed atlas assets. Owned font/SVG parser replacement from the canceled Zig atlas plan is not a prerequisite for Rust recovery. Bring over useful deterministic-source receipts and candidate publication requirements only through a separate Rust enhancement. Resolve the old atlas CLI source-authority gap with an explicit authenticated manifest in an invocation-owned repository-shaped root; never production fixture fallback. Qualification identity must include host,target,backend,viewport,scale and profile, with validated evidence closure. Candidate outcomes must never be promoted to release merely by missing hardware rows.

## Storage and journeys

Keep existing profiles untouched. Install new versioned sessions only after validated decode, authority, graph and selected-branch checks. Native saves and actual IndexedDB rollback/retry/abandoned jobs require live evidence. Native model tests do not qualify IndexedDB. Offline entry and WebGPU initialization fallback must use rebuilt separate artifacts.

## Content and delivery

Templates, tutorials, demos, difficulty, searchable help and campaigns must teach cube seams, economic costs, provenance and congestion through real rules. Qualify keyboard/pointer/touch/IME, screen readers, audio, resize/high-DPI, device loss, offline and fallback states. Retain named-hardware targets: authority p95<=5ms, native/WebGPU frame p95<=16.7ms, WebGL2 Low<=33.3ms, input p95<=100ms. Missing hardware stays Unverified.

## Current policy

Do not run release build gates, asset generation/check tools, browser build wrappers or runtime journeys during the current compilation-only upgrade. CI nightly pin changes and all later verification are explicit deferred tasks. No deployment or publication follows from a local build.

## Self-review

Checked scope, contract precedence, preserved identities, paused-verification policy and known source-review gaps. Referenced historical numerical contracts remain bound by the input hash manifest, not silently accepted through their old approval labels. Product implementation and runtime acceptance statuses must remain separate.
