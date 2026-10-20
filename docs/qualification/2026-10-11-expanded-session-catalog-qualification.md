# Expanded session catalog qualification

Date: 2026-10-11. **Current: pure metadata slice qualified at the exact source
hashes below. Partial: shared integration and durable sessions remain pending.**

The implementation adds owned canonical catalog encoding/decoding, allocation
ledger records and complete-chain binding, and detached selection/rename/archive/
remove proposals. It performs no profile or host-store I/O. Exact catalog and
ledger bytes use new identities; raw reservations and metadata proposals do not
prove a durable allocation, graph closure or successful storage CAS.

The reviewed plan is `2026-10-11-nyon-expanded-session-catalog.md`, SHA256
`b404476de83de41ba577d895a88153d1522145a51ac237f4b71b21c2cc3afc5a`.
Controller approval uses Donald's verified delegation of game design decisions.

## Observed focused gate

Exact compiler: `0.18.0-dev.146+35accc06e`, checked against `zig/.zigversion`
before every command. Source hashes were checked before each final run. Commands
ran from `zig/`; all exits below are observed worker evidence reviewed independently.

| Suite | Debug | ReleaseSafe |
| --- | --- | --- |
| `zig test src/expanded_session/catalog_tests.zig` | 12/12, exit0 | 12/12, exit0 |
| `zig test src/expanded_session/allocator_ledger_tests.zig` | 12/12, exit0 | 12/12, exit0 |
| `zig test src/expanded_session/metadata_tests.zig` | 9/9, exit0 | 9/9, exit0 |

ReleaseSafe uses `-O ReleaseSafe`. Total33/33 per mode,66 test executions.
`zig fmt --check src/expanded_session` exited0. Each implementation task recorded
a compiled behavioral RED; final ordering mutation proof removed only the order
guard in a disposable frozen harness and the unique descending-branch regression
failed as intended. That expected mutant failure is not a production failure.

Coverage includes rehashed malformed framing/semantics, independent branch order
and duplicate guards, nested allocation failures, caller preservation, owned
labels, 64×64 branch capacity, exact-watermark counter binding, exhaustion, old
recovery binding, selection consistency and invalid rename rejection before any
allocation. Public comments retain complete trusted EOF/binding and proposal/CAS
preconditions. All three task reviews and the final whole-slice scoped review
approved this exact revision; no open slice findings remain. Final review SHA256:
`d2ef2038ed38993b3d0287d3866bb71fbd8e0ceb26a96805edcd72fff21189e9`.

## Frozen source

```text
db6515d4799b8ff6ba2d620ab3c74dfdf99d13f5026eca71c161e147c4840179  zig/src/expanded_session/allocator_ledger.zig
5245f126aa42b858962893ba52b804567b7b07ae133701241782fa4b572ba970  zig/src/expanded_session/allocator_ledger_tests.zig
34557a067409a5d79c61e34d0c217695e248d4c7da03a41f40554e4bbbabce11  zig/src/expanded_session/catalog.zig
83e5f6645e6ab3384985f2d98a848ae64277f954ebb22f9912f7a061d9e1791d  zig/src/expanded_session/catalog_tests.zig
675114d707838bdbff981e4637e2aecd758f280f3ad0452de3103d317e04117d  zig/src/expanded_session/limits.zig
ae38a97fca9cc0a52cbd1d25914780abb4d22b2b871976b01c8dca914102e531  zig/src/expanded_session/metadata.zig
abc03472a2e07ef551c4e81fd6543866cfbc7428ccf6174ff76da3da4d3ebb5a  zig/src/expanded_session/metadata_tests.zig
1fd5ce39314ab4475b910e6eb3d8cee1f3b5f982b15ae360ebbd267776c34908  zig/src/expanded_session/types.zig
```

## Remaining qualification

These suites remain outside shared `root.zig`, `build.zig` and `tools/check.sh`
until owner coordination and a fresh integrated gate. This receipt is not a
frozen full-repository pass. Graph/manifest ancestry and closure, journal EOF
transport/recovery, real durable allocation/locking/CAS, native/IndexedDB stores,
streamed history, import/export, replay, and playable Save/Continue are unverified.
No commit, push, dependency, shared-root/build change or source retirement is
part of this slice. The larger game remains incomplete.
