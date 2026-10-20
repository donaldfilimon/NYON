# Expanded durable-session specification review

Date: 2026-10-11. Independent technical review of the exact revision below is
complete; storage implementation is not qualified. Subsequent full-scale stream
integration needs a new review. Root verified Donald's delegated design approval
in "Finish NYON rewrite in Zig"; no separate approval-only pause remains.

Specification: [expanded durable sessions](../superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md).
Reviewed SHA256: `6e113ae956f6c9cd189c136e0633f6eee08cce357d3357463615e6568b0ea751`.

Review corrections cover endurance bounds, physical wire formats, exact Continue
capture, repair generation, transactional pin capture, staging inventories,
resident versus operation pin lifetimes, and resident retention after trim.
The final correction used a fresh implementer and fresh reviewer. The final
scoped verdict was spec compliance PASS and design quality PASS, with no remaining
findings in that scope; the reviewer independently verified the source hash.

The proposed contract uses a new save namespace, streamed immutable history,
canonical portable indexes, generation CAS, captured Continue state, an allocation
and repair ledger, and distinct resident and operation pins. Required future
fixtures include save N / persist N+1 / commit N / GC / export, and trim / external
slot deletion / recovery advancement / GC / resident replay and export.

Validation here covers specification consistency, links, whitespace and source
identity. No new save, IndexedDB, crash-recovery, GC, replay, endurance or capacity
runtime tests have executed. A detailed implementation plan and reviewed authority
and streaming-history interfaces remain prerequisites. The current four-world
coordinator does not prove the expanded game's capacity.
