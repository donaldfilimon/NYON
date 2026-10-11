# Task 3G0 independent math contract review

2026-10-09. Verdict: **Request changes, document only**. One Important finding must be corrected before accepting this math contract. No Critical findings. This does not authorize 3G implementation or qualify GUI, host, implementation math, or the full game.

Reviewed candidate SHA-256 `06c1ac2b26b504518b3baf2c68e0391cff90c79e6b15055fa2ffb7dd23da4d19`; owning intake SHA-256 `a80b075743ac2831bbc5d6e82486404128f4c87d413d223c2b5ccfcbe494b2e6`. Canonical checkout `/Users/donaldfilimon/dev/active/NYON`. Read home/repository AGENTS, the intake, migration global constraints and 3G dependency row. Only this review file was written. Other product work remained frozen; concurrent owners were preserved.

## Findings

**Important I1, candidate line 47: overflow does not imply a nonfinite inverse.** The assertion “Zero/singular/nonfinite/overflow matrices yield invalid inverse entries through IEEE arithmetic” overstates the unchecked scalar algorithm. For a diagonal matrix with all four diagonal entries `f32(1e10)`, triple-product cofactors remain finite (approximately `1.0000000150474662e30`), while the determinant overflows to positive infinity. Its reciprocal becomes zero and every output entry becomes finite zero. This follows the specified algorithm unchanged, including its lack of finite determinant checks; scalar glam computes `inverse.mul(1.0 / dot1)` and its optional assertion checks output finiteness, not determinant finiteness (glam scalar mat4.rs:849-869). A matrix-wide finiteness check alone therefore cannot establish a usable inverse. Picking rejects that particular zero matrix because homogeneous w is zero, but that is a distinct downstream guard (src/presentation/picking.rs:97-104).

Required correction: state that unchecked arithmetic may produce nonfinite entries **or finite numerically unusable results** on exceptional inputs. Preserve source admission and arithmetic; do not add a determinant gate while claiming literal parity. Restrict inverse correctness to the specified valid-camera witness domain and keep residual/picking guards explicit. Add the determinant-overflow classification witness so a future implementer does not silently invent rejection or rely on output finiteness as an inverse proof (candidate:47,79).

**Minor M1, candidate line 19: normalized-up precondition and optional assertions should be explicit.** Source `look_to_axes4` asserts normalized up and normalized direction under glam assertion configuration, and the RH look-at documentation identifies the normalized-up requirement (glam camera/camera_impl.rs:21-24; camera/rh/view.rs:24-30). The proposed valid-camera preconditions list only unequal eye/target and nonparallel up. Y-up already satisfies the requirement in the production camera, so this does not change valid camera behavior. State generic lookAt's normalized-up requirement and clarify its invalid-input/assertion policy, as already done for inverse. Do not claim assertion parity across unknown Rust feature configurations.

**Minor M2, candidate lines 59-65: sparse inverse witnesses provide limited indexing coverage.** Identity, translation, one-axis rotation and diagonal scale are useful independent fixtures, but most cofactor terms remain zero. Add an independently solved small dense or sheared invertible matrix with literal expected inverse or solution vectors, derived without the implementation under test, to exercise off-diagonal cofactor signs/indexing. The written formulas themselves match inspected scalar source (glam scalar mat4.rs:796-869); this is a witness-strength recommendation rather than an identified formula error.

## Confirmed document contracts

Current source inspection supports immutable column indexing and sequential matrix-vector/matrix-matrix composition, RH view basis signs and translation, projection Y-up/Z0..1/negative-Z forward, and the written scalar inverse coefficients, signs and left-associated determinant (candidate:9,17-44; glam scalar mat4.rs:796-869,1550-1556; camera/camera_impl.rs:21-33,56-64,134-169). No formula sign or transposition defect was found.

The reciprocal-finite-positive tryNormalize rule matches Rust source and does not add an epsilon or post-check. Raw sphereDistance retains source admission, including manually supplied nonunit and zero directions; unprojection retains finite homogeneous/divided-result and w-epsilon rejection (candidate:15,47; glam vec3.rs:658-665; src/presentation/picking.rs:14-63,97-104). The ordered-key transform is equivalent to the inspected Rust total_cmp signed transform, with signed-zero and NaN payload ordering explicit; WorldId applies only on equal distance ordering (candidate:51,67; cited September Rust f32.rs:1600-1630; src/presentation/picking.rs:82-86).

Strict-mode intent is supported by the pinned Zig language reference; it does not prove libm equivalence. Separate Rust sin_cos versus Zig sin/cos, native NEON dot reduction versus scalar association, unknown compiled features, and older installed rust-src provenance remain correctly bounded rather than promoted to cross-target bit claims (candidate:11,49,51,79; pinned langref.html:11533-11559; glam math.rs:183-196,239-241; neon.rs:62-66; f32.rs:24-28,53-57).

The analytic projection near/far depth and T/R composition values are independently correct. The f32 pi/2 public-constructor fixture properly uses a tolerance rather than asserting exact trig bits. Normalization/blend tolerance, clamped-camera inverse residual threshold, source behavioral picks, and future differential-capture provenance are separately named (candidate:55-79; tests/presentation.rs:55-82). These are Proposed qualification tests, not passing executed implementation tests.

## Checks and limits

Actual commands, all exit 0:

- `shasum -a 256` on candidate and owning intake confirmed the two frozen identities above.
- Python pathlib/re/hashlib comparison of every source identity row in candidate:96-113 confirmed all 18 hashes against disk. No source bytes were edited.
- Read-only `cat`, `nl`, `sed`, and Python exact-range reads inspected the candidate, intake, owning migration constraints and cited math/picking/camera/toolchain sources.
- Independent Python `fractions.Fraction` near/far proof produced depth 0 at z=-1 and depth 1 at z=-2, and substitution of T/R verified `(6,0,9)` versus `(5,5,9)`.
- Independent Python `struct.pack/unpack('f')` with overflow mapped to IEEE infinity demonstrated finite diagonal cofactors, infinite determinant and zero reciprocal/output. This is a staged arithmetic document counterexample, not execution of Rust/glam or Zig product code.

Unknown: actual October Rust compiler emitted arithmetic/features; native/browser transcendental differences; future implementation residuals and picks; generic arbitrary-matrix error bounds. No production build, test, dependency, profile, asset, commit, host or manual checks were run. Existing source test thresholds were inspected only. Acceptance remains blocked on I1's document correction and re-review of the resulting candidate identity; 3F readiness is not assumed.
