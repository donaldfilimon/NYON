# Task 3G0 proposed math contract and independent witness design

2026-10-09. Proposed, awaiting independent review. Intake remains frozen at SHA-256 `a80b075743ac2831bbc5d6e82486404128f4c87d413d223c2b5ccfcbe494b2e6`. No 3G product implementation is authorized by this proposal; dependency remains reviewed3F and reviewed math contract (`docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:95-106`). Only this document is owned; preserve active3F1/4A. Read-only `zig version` and `zig env` confirmed compiler0.18.0-dev.120+9fe22a29b, std `/Users/donaldfilimon/.zvm/master/lib/std`, host target aarch64-macos.27.2...27.2-none. No builds/tests executed.

Paths: G means `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12`; Z means `/Users/donaldfilimon/.zvm/master`; references below expand those exact prefixes. Resolved glam is0.33.12 (`Cargo.lock:1137-1142`).

## Immutable values and APIs

Proposed value structs `Vec2{x,y:f32}`, `Vec3{x,y,z:f32}`, `Vec4{x,y,z,w:f32}`, `Mat4{cols:[4]Vec4}`. All operations accept/copy values and return values; no allocator, pointer retention, platform type, SIMD ABI or alignment promise. Matrix index is `cols[column][row]`; column flattening order x/y/z/w for each column. GPU uniform packing belongs to Task6. API: vector add/sub/scale/dot3/cross/lengthSquared/distance/tryNormalize/isFinite; matrix identity/fromColumns/toColumns/mulVec4/mulMat4/inverseUnchecked/isFinite; RH lookAt/perspectiveRHZO; finite-clamp helpers; f32 totalOrder. Public Camera/Ray remain separate owners (`src/presentation/camera.rs:20-65`; `src/presentation/picking.rs:7-11`). `inverseUnchecked` naming distinguishes existing source behavior from optional new checked APIs. A checked inverse may be separately proposed later; camera parity uses the unchecked path for valid camera matrices.

Set float mode explicitly `.strict` in math functions. Pinned compiler documents strict IEEE default and optimized-mode permission for contraction/reassociation/reciprocals/ignoring signed zero, NaN and Inf (`Z/doc/langref.html:11533-11559`). Do not use @mulAdd, fast math or a different inverse specialized to camera without separate review. Pinned builtin APIs @sqrt/@sin/@cos/@exp accept float and return same type (`Z/doc/langref.html:11867-11924`). `std.math.degreesToRadians` multiplies angle by rad_per_deg (`Z/lib/std/math.zig:303-311`); propose f32-typed angle and f32 factor, with Rust bit witness for degree conversion before claiming exact match. Runtime sin/cos/exp inputs and intermediates remain f32. Rust glam std math calls f32::sin/cos/sin_cos/exp (`G/src/f32/math.rs:183-196,239-241`); Zig separate sin/cos may differ from Rust paired sin_cos. Strict mode does not prove matching transcendental rounding or target SIMD behavior.

## Exact algorithm contract

`dot3(a,b)=((a.x*b.x)+(a.y*b.y))+(a.z*b.z)`; cross uses component multiply/subtract; lengthSquared=dot3(v,v); length=sqrt(lengthSquared); reciprocal=1/length. `tryNormalize` returns v*reciprocal exactly when reciprocal finite and>0, otherwise null. Do not introduce epsilon or post-normalization finite checks that narrow source acceptance (`G/src/f32/vec3.rs:249-251,658-665`). Raw sphereDistance still assumes unit direction, does not normalize/check length, and preserves current input admission (`src/presentation/picking.rs:37-63`).

For every output row r, mulVec4 evaluates `((cols[0][r]*v.x + cols[1][r]*v.y) + cols[2][r]*v.z) + cols[3][r]*v.w`, with explicit staged f32 multiply/add. mulMat4(A,B) has columns mulVec4(A,B.cols[k]), k0..3. Scalar, SSE2, NEON and wasm paths expose the same four sequential column products/adds; their matrix operator uses per-column multiplication (`G/src/f32/scalar/mat4.rs:1550-1556,1815-1826`; `G/src/f32/neon/mat4.rs:1657-1663,1922-1932`; `G/src/f32/wasm/mat4.rs:1655-1661,1920-1930`; `G/src/f32/sse2/mat4.rs:1659-1665`). This semantic sequence is the proposed portable implementation, not a claim emitted instructions or all bits coincide.

RH lookAt: f=normalize(center-eye), s=normalize(cross(f,up)), u=cross(s,f). Columns `(s.x,u.x,-f.x,0)`, `(s.y,u.y,-f.y,0)`, `(s.z,u.z,-f.z,0)`, `(-dot(eye,s),-dot(eye,u),dot(eye,f),1)`. Generic valid lookAt preconditions are eye!=target, normalized up and direction not parallel to up. Source lookAt normalizes center-eye; look_to_axes4 conditionally asserts normalized direction/up under glam assertion configuration (`G/src/camera/camera_impl.rs:21-24`; `G/src/camera/rh/view.rs:24-30`). Production Y-up satisfies normalized-up. Proposed portable unchecked math does not add an unconditional admission gate or panic for these inputs; normalization/basis arithmetic on invalid generic input may produce invalid matrices. Optional Rust assertion parity is unknown until compiled features are established. Do not hide this by defaulting to identity (`G/src/camera/rh/view.rs:24-30`; `G/src/camera/camera_impl.rs:21-33,56-64`). Camera uses Y-up and default/limited pitch away from degeneracy (`src/presentation/camera.rs:10-18,215-235`).

RH perspective with NDC Z0..1/Y-up: halfFov=fov*0.5; h=cos(halfFov)/sin(halfFov); xx=h/aspect; yy=h; invRange=1/(far-near); zz=-far*invRange; tz=(-near*far)*invRange; zw=-1. Columns `(xx,0,0,0)`, `(0,yy,0,0)`, `(0,0,zz,zw)`, `(0,0,tz,0)`. Source wrapper selects true,true,false and valid camera uses45degrees/aspect clamp0.1..10/near0.1/far100 (`G/src/camera/rh/proj.rs:203`; `G/src/camera/camera_impl.rs:134-169`; `src/presentation/camera.rs:232-242`). Matrix composition is projection*view. No clip Y flip or -1 near depth.

Proposed inverse is the existing scalar adjugate/cofactor algorithm, not pivoted Gaussian elimination. Name `mXY=cols[X][Y]`. Compute exact pair-product differences below, each multiply/subtract staged f32:

```text
c00=m22*m33-m32*m23; c02=m12*m33-m32*m13; c03=m12*m23-m22*m13
c04=m21*m33-m31*m23; c06=m11*m33-m31*m13; c07=m11*m23-m21*m13
c08=m21*m32-m31*m22; c10=m11*m32-m31*m12; c11=m11*m22-m21*m12
c12=m20*m33-m30*m23; c14=m10*m33-m30*m13; c15=m10*m23-m20*m13
c16=m20*m32-m30*m22; c18=m10*m32-m30*m12; c19=m10*m22-m20*m12
c20=m20*m31-m30*m21; c22=m10*m31-m30*m11; c23=m10*m21-m20*m11
F0=(c00,c00,c02,c03); F1=(c04,c04,c06,c07); F2=(c08,c08,c10,c11)
F3=(c12,c12,c14,c15); F4=(c16,c16,c18,c19); F5=(c20,c20,c22,c23)
V0=(m10,m00,m00,m00); V1=(m11,m01,m01,m01)
V2=(m12,m02,m02,m02); V3=(m13,m03,m03,m03)
I0=(V1*F0 - V2*F1) + V3*F2
I1=(V0*F0 - V2*F3) + V3*F4
I2=(V0*F1 - V1*F3) + V3*F5
I3=(V0*F2 - V1*F4) + V2*F5
C0=I0*(1,-1,1,-1); C1=I1*(-1,1,-1,1)
C2=I2*(1,-1,1,-1); C3=I3*(-1,1,-1,1)
D=cols[0]*(C0.x,C1.x,C2.x,C3.x)
det=((D.x+D.y)+D.z)+D.w; invDet=1/det
return columns C0*invDet,C1*invDet,C2*invDet,C3*invDet
```

The scalar source is `G/src/f32/scalar/mat4.rs:796-869`. No det epsilon, singular-to-identity, pseudoinverse, scaling fix or finite-input gate is added. Exceptional zero/singular/nonfinite/overflow inputs can produce nonfinite entries OR finite numerically unusable results through unchecked IEEE arithmetic. Specifically diag(f32(1e10),f32(1e10),f32(1e10),f32(1e10)) has finite approximately1e30 cofactors but positive-infinite determinant; reciprocal becomes0 and inverse entries become finite zeros. Scalar source multiplies by reciprocal and its optional assertion checks output finiteness, not determinant finiteness (`G/src/f32/scalar/mat4.rs:849-869`). Do not add a finite determinant gate, epsilon or different admission rule while claiming parity. Output finiteness alone is not inverse correctness. Restrict correctness claims to specified valid-camera witnesses and explicitly check residuals/behavior there; downstream unprojection rejects this particular finite-zero result because homogeneous w=0 (`src/presentation/picking.rs:97-104`). Glam inverse describes invalid results and conditional glam_assert panic, whereas checked mode returns zero/false (`G/src/f32/scalar/mat4.rs:859-884`; `G/src/f32/neon/mat4.rs:968-983`). Proposed portable unchecked function has no unconditional Zig safety panic for these float operations; this difference from optional glam assertion configuration must stay explicit. Invalid matrices must never silently become valid camera transforms. Picking still rejects homogeneous nonfinite or abs(w)<=EPSILON and normalizes ray direction (`src/presentation/picking.rs:19-34,97-104`).

NEON inverse computes equivalent cofactors via shuffles and separate vmul/vsub/vadd, determinant through NEON dot4 horizontal reduction and reciprocal; wasm uses corresponding lane shuffles/float operations (`G/src/f32/neon/mat4.rs:794-962`; `G/src/f32/wasm/mat4.rs:820-961`). NEON dot4 is multiply then vaddvq, not scalar explicit left-association (`G/src/neon.rs:62-66`). Thus scalar cofactor-port bits are not automatically native NEON inverse bits. glam routes aarch64 to NEON absent core-simd/scalar-math features (`G/src/f32.rs:24-28,53-57`); this is cfg source, not a compiled feature-set audit. wasm simd feature/target must be recorded in future witness capture. Compare residues/tolerances rather than promise SIMD identity.

`totalOrder` proposed maps raw u32 float bits to unsigned ordered key: negative bits -> bitwise complement, nonnegative bits -> xor0x80000000; compare keys. This orders negative NaNs before -Inf, negative finite, -0, +0, positive finite, +Inf, positive NaNs and preserves payload order in the respective sign direction. It implements Rust total_cmp's signed transformed ordering, inspected in `/Users/donaldfilimon/.rustup/toolchains/nightly-2026-09-01-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/num/f32.rs:1600-1630`. Current repo-selected nightly2026-10-08 has no installed rust-src; September source is algorithm provenance, not proof of October compiler output. Use WorldId only on exact total-order equality (`src/presentation/picking.rs:82-86`). Ordinary `<` loses signed-zero/NaN total order; no new NaN hit injection or rejection is added.

## Independent literal witnesses

These are Proposed tests with mathematical derivations, not executed product tests. Exact below means exact value equality for representable finite rational literals, with signed-zero bits checked only in explicit order witnesses. No arithmetic-generated -0 bit guarantee is inferred.

| Witness | Independent expected result | Comparison |
| --- | --- | --- |
| Identity*Vec4 | (2,-3,4,1) unchanged | exact value |
| Translation T=(last column3,-2,5,1) | point(2,-3,4,1)->(5,-5,9,1); direction(2,-3,4,0) unchanged | exact value |
| inverse(T) | last column(-3,2,-5,1), other identity columns | exact value |
| Orthogonal quarter-turn R columns(0,1,0,0),(-1,0,0,0),(0,0,1,0),(0,0,0,1) | inverse=transpose; R*(2,-3,4,1)=(3,2,4,1) | exact value |
| Composition T*R vs R*T | point(2,-3,4,1): (6,0,9,1) vs (5,5,9,1) | exact value; catches multiply order |
| Scale diag(2,4,8,1) inverse | diag(0.5,0.25,0.125,1) | exact value |
| Dense inverse A rows(1,2,3,4),(0,1,2,3),(2,0,1,1),(1,1,0,1) | inverse rows (0,-1/5,2/5,1/5); (1,-6/5,-3/5,1/5); (1,-1,0,-1); (-1,7/5,1/5,3/5) | abs<=1e-6 entries; AB and BA residual<=1e-6 |
| Determinant overflow diag(f32(1e10),f32(1e10),f32(1e10),f32(1e10)) | finite cofactors, determinant+Inf, reciprocal0, finite-zero inverse; unprojection rejects w0 | classification/value zero; no output-finiteness admission |
| Inverse zero and duplicate columns | inverse nonfinite, never identity; unprojection/ray rejects | classification, no NaN payload golden |
| normalize(3,4,0) | approximately(0.6,0.8,0); zero/nonfinite vector null | abs<=1e-6 finite values; classification |
| totalOrder raw bits | ff800000(-Inf)<bf800000(-1)<80000000(-0)<00000000(+0)<3f800000(1)<7f800000(+Inf); positive NaN7fc00001<7fc00002, negative NaNffc00002<ffc00001 | exact ordering |
| lookAt eye(0,0,5),center0,upY | identity basis with translation Z=-5 | exact value |
| Analytic projection coefficient fixture h=1,aspect2,near1,far2 | columns(0.5,0,0,0),(0,1,0,0),(0,0,-2,-1),(0,0,-2,0); pointZ=-1 maps depth0, pointZ=-2 depth1 | exact rational coefficient helper arithmetic |
| Public perspective fov=f32(pi/2),aspect2,near1,far2 | above analytic matrix, near/far depths0/1 | abs<=1e-6; trig rounding acknowledged |
| Source production near0.1/far100 | negative viewZ at respective planes maps depth0/1 | abs<=1e-5; formula independent of computed inverse |

Analytic projection is derived from real half-angle45degrees with h=1, NOT claimed exact for f32 pi/2 passed to @sin/@cos. Expose no public h override solely to make the test tautological; literal Mat4 validates unprojection independently and public constructor compares separately. Independent near/far proof: clipZ=(-f/(f-n))*z - nf/(f-n), clipW=-z; z=-n gives0 and z=-f gives1. Document derivation evidence below uses rational arithmetic, not Rust/Zig production.

Repeated camera clamp fixtures: reduced motion + orbit(100,100), then repeat100 times -> target/current maxYaw/maxPitch; orbit(-100,-100) repeated -> minYaw/minPitch. zoom100000 repeatedly -> distance8; zoom-100000 repeatedly ->24. reset under Reduced -> yaw0/pitch55degree/distance15, target position preserved. Under Full reset changes targets but current remains until update; update0 leaves current values; update durations0.25 and1 must give same camera current results from same prior state (dt cap), but visual-time progression differs. These derive clamp idempotence and source reset/update rules; finite-angle endpoints use f32-derived constants, cross-libm intermediate matrices use tolerance (`src/presentation/camera.rs:108-197`). Expected blend at0.25 is1-exp(-3), analytically approximately0.9502129316; compare1e-6, not exact bits. Existing source inverse-product tolerance1e-4 is retained at clamped source viewports; default1440x900 projected centers/HUD/tie fixtures retain existing IDs and distance4 witness (`tests/presentation.rs:55-150`). Do not broaden tolerance because an implementation fails; investigate error first.

## Provenance and qualification policy

Exact contracts: identities/enums/counts/order/truncation/seed/ID ties, rejection classifications and rational literal arithmetic above. Camera trig/exp, projection/inverse/residuals use explicitly named tolerance witnesses. Proposed local source differential snapshots must record Rust toolchain, Cargo.lock SHA, target, features incl scalar/NEON/wasm, optimization/fast math configuration, ordered input f32 bits, output f32 bits and capture command. Capture Rust directly without running future Zig implementation to define its expected values. Validate high-signal values against analytic fixtures first. For each tested target publish absolute residuals (matrix inverse product<=1e-4 source threshold, public analytic projection<=1e-6, production nearfar<=1e-5, normalization/blend<=1e-6); unspecified arbitrary matrices are outside those particular tolerance guarantees. If default-camera projected picks differ, fail behavior even when residual tolerance passes. No cross-target bit-equivalence statement follows from a source witness.

Actual native renderer matrix upload, uniform flattening/ABI, browser WebGPU/WebGL convention reconciliation, backing/CSS/DPR event normalization, pixel framing/occlusion and manual accessibility remain unverified Task6/3J. Math-only witnesses cannot establish these. No parent review verdict or 3F readiness is assumed.

## Scoped derivation evidence

Executed a Python fractions-only document derivation during authoring (no product module import, no tests/build). Exact rational near/far proof for n=1/f=2 follows:

- z=-1: clipZ=0, clipW=1, NDC depth=0.
- z=-2: clipZ=2, clipW=2, NDC depth=1.

Translation/rotation composition derived by substituting T(x,y,z)=(x+3,y-2,z+5), R(x,y,z)=(-y,x,z): T(R(2,-3,4))=(6,0,9); R(T(2,-3,4))=(5,5,9). No oracle output was harvested from future Zig.

## Source SHA-256 identities

| Source | SHA-256 |
| --- | --- |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/scalar/mat4.rs` | `f1f73ff20a7ca2732476c5acb95e353289f07b8a45a34f5ca6b21c89672d3e31` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/neon/mat4.rs` | `737df862349c88f4b95eb560064fac3c40b264ee52e58a407d87ca6c7be7f387` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/wasm/mat4.rs` | `8d3c203266205d0b96c1ced9a775ee06c30f35cdd8bd9b0979a0736ad49d74ba` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/sse2/mat4.rs` | `5676b1da7a3a4f09c4f5d08c6e1ca9daeeee0f7157d97a5d5bb6fc737dc92abf` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/neon.rs` | `63c9d02c5288df84db6f77e6fb42be7dd62464a345beece3f7eae5174952e3a4` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/vec3.rs` | `d304233ca9be34ebdd2646eb1ce99a7b3c0e8002aec83db0861137304715e292` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32/math.rs` | `26a1eb02f38051fab1d805823bcb1fb8f469269a9d9f9cd52bbf68766ecaf34e` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/f32.rs` | `79eb47c7639abd833a2dcb8119c37432e2cae6f20d0bb8ee87a8035f8dbb1729` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/camera/camera_impl.rs` | `4c8810fac9472129e8c087f7da87a47fcec61b5b96872cc0c2bc62ccc3d8fbb5` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/camera/rh/proj.rs` | `f99a608ec74ea549f01207cf1faae201d806d4495be34b1bebc7f676d2e0320c` |
| `/Users/donaldfilimon/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/glam-0.33.12/src/camera/rh/view.rs` | `64f8ff3516be689b5da58480c2ce9f3a0dcdece47ce02a9d80d5fe64d0d343cf` |
| `/Users/donaldfilimon/.zvm/master/doc/langref.html` | `c325b616428b810665720a8c60208574bbd97b716ecb47d3a9ca35a59f3c7535` |
| `/Users/donaldfilimon/.zvm/master/lib/std/math.zig` | `5fd3f04fab913039c095a8c95a53cafc1e57f0bd13f15d25e86a8453e311e139` |
| `src/presentation/camera.rs` | `d6da1011ebbf27f4ffc213bd3745156ef534c3ae3a35c101d68dfbf94b8087b7` |
| `src/presentation/picking.rs` | `82047fb85ae8e34662dc13d56bafebb77b063d27a37f098910fac26d16acd297` |
| `tests/presentation.rs` | `a1376f5984743f7333e0bf231f06d5803d9e0ebff099e225613ec47dcd92e444` |
| `zig/.zigversion` | `8f79b2a57773c8f7593af87c07b4bf40777b6bc6ed51d049695568b327d411c6` |
| `/Users/donaldfilimon/.rustup/toolchains/nightly-2026-09-01-aarch64-apple-darwin/lib/rustlib/src/rust/library/core/src/num/f32.rs` | `9ecede93f9ba83c55ca7d15abcfd1ed891f11485a2140d7758652708b2559200` |

## Fix1 independent derivation extension

Dense inverse above was solved by exact rational Gauss-Jordan elimination in a document-only Python script, independent of the proposed cofactor implementation. Both A*B and B*A reduce exactly to identity. Expected entries are rational literals, not generated from production Rust/Zig. The diagonal overflow counterexample used staged Python struct f32 rounding and IEEE overflow-to-infinity emulation; it is not execution of either production implementation. Exact inputs/intermediates and products are recorded in sibling `task-3g0-math-contract-fix1-derivation.json`.
