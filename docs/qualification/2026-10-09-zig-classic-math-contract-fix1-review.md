# Task 3G0 math contract FIX1 independent scoped review

2026-10-09. **Accept the FIX1 document corrections.** I1, M1 and M2 are ADDRESSED. New Critical: none. New Important: none. New Minor: none. Scope is only the FIX1 diff and breakage introduced by it, not a wider review or implementation authorization. The active parent F1 gate remains separate.

## Frozen inputs

- Proposal SHA-256: `acc092883213c50d18ab062d913916fec62149a49a2ccad6e3d4069b94a40a73`.
- FIX1 report SHA-256: `87deab63ac72f6b5fbee04da3a217dbc07099f2c80640c284a9e44d94b58c4c6`.
- Patch SHA-256: `26a5dc4e41d9c4b6e45a6982f579f06b07526403b05e2e619604738bb232b4a5`.
- Derivation SHA-256: `62b38d0614e7d054a6f56165e514f739e4641509b958220cddeca00976039424`.

## Findings disposition

**I1 ADDRESSED.** The revised text explicitly distinguishes nonfinite output from finite unusable output, supplies the diagonal determinant-overflow witness, prohibits adding a finite-determinant admission gate while claiming parity, and separates camera residual/behavior qualification from output finiteness. Independent staged f32 document arithmetic reproduced d=10000000000, finite cofactor=1.0000000150474662e30, determinant=positive infinity, reciprocal=0 and output=0. This follows scalar reciprocal multiplication and optional output-finiteness assertion; homogeneous w=0 fails the downstream unprojection guard. Signed zero bits are intentionally not claimed. [task-3g0-math-contract-proposal.md:47,67; glam-0.33.12/src/f32/scalar/mat4.rs:849-869; src/presentation/picking.rs:97-104]

**M1 ADDRESSED.** The revised generic lookAt contract requires normalized up and nonparallel direction with unequal eye/target, identifies source direction normalization and conditional normalized-direction/up assertions, and leaves actual feature-dependent assertion parity unknown. It adds no unconditional portable admission gate or panic. Source look_to_axes4 and RH look_at_mat4 documentation support those statements. [task-3g0-math-contract-proposal.md:19; glam-0.33.12/src/camera/camera_impl.rs:21-24; glam-0.33.12/src/camera/rh/view.rs:24-30]

**M2 ADDRESSED.** The added dense matrix and literal rational inverse are independently correct. Independent recursive determinant/minor expansion, separate from the author's Gauss-Jordan derivation and proposed scalar algorithm, gives determinant 5 and cofactor rows `(0,5,5,-5)`, `(-1,-6,-5,7)`, `(2,-3,0,1)`, `(1,1,-5,3)`. Transposing these cofactors and dividing by 5 reproduces every literal inverse entry. Independent Fraction multiplication verifies both A*B and B*A equal identity exactly and agrees with the evidence JSON. Proposed f32 entry/residual tolerances remain future qualification criteria, not observed implementation results. [task-3g0-math-contract-proposal.md:66,117-119; task-3g0-math-contract-fix1-derivation.json, matrix_rows/inverse_rows/AB/BA]

## Actual checks and limits

Read home and repository AGENTS, original review, FIX1 report, complete patch, revised proposal context, derivation and the narrowly cited scalar/view/picking source ranges. Read-only source commands returned exit 0. A standalone Python document check returned exit 0: all four supplied hashes matched; difflib regenerated the exact sibling patch from before/proposal; rational determinant/cofactors and both products passed explicit assertions; staged f32 overflow classification passed explicit assertions. This is independent document mathematical evidence, not production Rust or Zig execution.

Only this review file was written. No product/source, build/test, asset, dependency, configuration, commit or subagent actions were taken. No math implementation, runtime, host, GPU, browser or parent-gate claim follows. Concurrent owners and frozen work remain outside this review.
