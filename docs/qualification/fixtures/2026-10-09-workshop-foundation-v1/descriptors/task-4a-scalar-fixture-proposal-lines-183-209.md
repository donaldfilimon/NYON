## Complete dynamic replacements, isolated from static maps

The source dynamic view replaces **whole** tick/worlds/deposits/shipments, borrowing every other map from the receiver. It does not merge old and candidate maps and cannot retain slices after the synchronous call. Its map/field order matches ordinary canonical state encoding. The valid-graph all-four example remains unchanged; these additional raw sentinel cases detect omitted individual overrides, accidental merging and accidentally replacing static content. [crates/nyon-workshop-core/src/model.rs:195-238; task-4a-valid-fixture-proposal.md:75]

Use receiver S and define exact field fragments below, all copied as literal reviewed bytes. Each named replacement is one top-level fragment occurrence, with no whitespace inserted. Full input/preimage hex and SHA-256 for each byte-defined dynamic expected state are in `derive-literals-01.log`; the calculator performs literal byte substitutions only, never a JSON serializer. These are **independent standard hash calculations**, not an implemented dynamic-view call.

| Replacement symbol | Exact old fragment | Exact replacement |
| --- | --- | --- |
| T | `"tick":18446744073709551615` | `"tick":0` |
| W | `"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":4294967295,"orbit_period_ticks":18446744073709551615,"phase_millidegrees":4294967295,"inventory":{"a":0,"a-":1,"a0":9007199254740993,"a_":18446744073709551615}}}` | `"worlds":{}` |
| D | `"deposits":{"00000000000000000000000000000006":{"world":"00000000000000000000000000000004","resource_id":"ore","reserve_units":18446744073709551615}}` | `"deposits":{}` |
| H | `"shipments":{"00000000000000000000000000000009":{"route":"00000000000000000000000000000008","source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","units":18446744073709551615,"dispatched_tick":0,"arrival_tick":18446744073709551615}}` | `"shipments":{}` |
| F | `"name":"Q\"\\"` | `"name":"R"` |

| Future dynamic consumer | Receiver, override inputs | Exact expected bytes |
| --- | --- | --- |
| DV_IDENTITY | S; S.tick/S.worlds/S.deposits/S.shipments | S |
| DV_TICK_ONLY | S; tick0, all three maps from S | apply T only to S |
| DV_WORLDS_EMPTY | S; S.tick, empty worlds, S.deposits/S.shipments | apply W only |
| DV_DEPOSITS_EMPTY | S; S.tick/S.worlds, empty deposits, S.shipments | apply D only |
| DV_SHIPMENTS_EMPTY | S; S.tick/S.worlds/S.deposits, empty shipments | apply H only |
| DV_ALL_EMPTY | S; tick0, all three maps empty | apply T,W,D,H |
| DV_POISONED_RECEIVER | receiver bytes S after T,W,D,H; replacements restore S.tick and complete three maps from detached S | S, because receiver's four dynamic fields are ignored |
| DV_STATIC_CHANGED | receiver S after F; overrides S.tick and S's three complete maps | apply F only, because receiver faction must remain |
| DV_DISTINCT_ALLOCATION | S; equal clones of three S maps, same tick | S, independent of addresses/capacities |

For DV_ALL_EMPTY the expected worlds/deposits/shipments maps have zero entries despite nonempty receiver maps. This catches a merge implementation. DV_POISONED_RECEIVER catches use of receiver tick/maps. DV_STATIC_CHANGED catches use of a whole replacement state rather than receiver static maps. Caller data must remain alive throughout encoding; snapshot before/after unchanged, then release temporary replacements and verify receiver bytes to establish no ownership transfer. Comparing dynamic digest with ordinary encoding of a detached state using the same substitutions is a **second** consumer witness; both must compare against the literal expected bytes/preimages, not just each other's output. Every actual lifetime/allocator/runtime observation remains Pending. [crates/nyon-workshop-core/src/model.rs:195,220; task-4a-foundation-vector-seams.md:28,137]
