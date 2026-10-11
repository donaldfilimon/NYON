## Every-field equality matrix

Baseline S is exactly `STATE_ALL_RECORDS_RAW` from the reviewed six-literal proposal, source declaration order preserved. It is intentionally **invalid authority** and is never installed. Each row below defines a typed detached-copy comparison; mutate only the specified path with the displayed before→after values. Compare original S to the changed copy: expect structural inequality and different canonical bytes. Hash inequality is not used as an equality oracle. All unchanged clone controls must compare equal despite different allocation addresses/capacities/insertion histories. [task-4a-independent-fixture-proposal.md:55,57,61; crates/nyon-workshop-core/src/model.rs:38,129,187; task-4a-foundation-vector-seams.md:80]

Paths use the full typed entity key alias defined above; these aliases specify exact IDs, not list indices. A grouped row with multiple suffixes means separate detached copies, never combined mutation. This is48 leaf/option/inventory cases covering all41 top-level record fields, including each RGB channel, both position axes and inventory key/presence/value distinctions. [crates/nyon-workshop-core/src/model.rs:38-127]

| Path in S | Exact before→after, each independent case |
| --- | --- |
| factions[I1].name | decoded `HEX(51225c)`→`R` |
| factions[I1].color_rgb[0,1,2] | index0:0→1; index1:127→128; index2:255→254 |
| systems[I2].name | `S`→`R` |
| systems[I2].position.x | -134217728→-134217727 |
| systems[I2].position.y | 134217728→134217727 |
| stars[I3].system | I2→I10 |
| stars[I3].name | `T`→`R` |
| stars[I3].archetype_id | `yellow-dwarf`→`x` |
| worlds[I4].system | I2→I10 |
| worlds[I4].primary | I3→I10 |
| worlds[I4].name | `W`→`R` |
| worlds[I4].archetype_id | `rocky-world`→`x` |
| worlds[I4].orbit_radius_milli_au | 4294967295→4294967294 |
| worlds[I4].orbit_period_ticks | 18446744073709551615→18446744073709551614 |
| worlds[I4].phase_millidegrees | 4294967295→4294967294 |
| worlds[I4].inventory["a"] value | resident0→resident1 |
| worlds[I4].inventory["a"] presence | resident0→absent, preserve all other entries |
| worlds[I4].inventory["a_"] value | 18446744073709551615→18446744073709551614 |
| worlds[I4].inventory key | rename a0→a1, preserving9007199254740993 |
| lanes[I5].a | I2→I10 |
| lanes[I5].b | I2→I10 |
| lanes[I5].distance_units | 18446744073709551615→18446744073709551614 |
| deposits[I6].world | I4→I10 |
| deposits[I6].resource_id | `ore`→`x` |
| deposits[I6].reserve_units | 18446744073709551615→18446744073709551614 |
| industries[I7].world | I4→I10 |
| industries[I7].definition_id | `extractor`→`x` |
| industries[I7].linked_deposit | Some(I6)→None; separately Some(I6)→Some(I10) |
| industries[I7].enabled | true→false |
| routes[I8].source | I4→I10 |
| routes[I8].destination | I4→I10 |
| routes[I8].resource_id | `ore`→`x` |
| routes[I8].batch_units | 18446744073709551615→18446744073709551614 |
| shipments[I9].route | I8→I10 |
| shipments[I9].source | I4→I10 |
| shipments[I9].destination | I4→I10 |
| shipments[I9].resource_id | `ore`→`x` |
| shipments[I9].units | 18446744073709551615→18446744073709551614 |
| shipments[I9].dispatched_tick | 0→1 |
| shipments[I9].arrival_tick | 18446744073709551615→18446744073709551614 |
| hazards[Ia].lane | I5→I10 |
| hazards[Ia].hazard_id | `ion-storm`→`x` |
| hazards[Ia].start_tick | 0→1 |
| hazards[Ia].duration_ticks | 18446744073709551615→18446744073709551614 |
| hazards[Ia].cancelled | true→false |

The remaining24 independent state-container comparisons are: tick max→max-1 (one); owners[I4] I1→I10, rename owner key I4→I10 preserving I1, remove owners[I4] (three); for each of the ten single-entry entity maps in S, rename its key to I10 preserving the record, and separately remove its only entry (twenty). All expect inequality. S has12 top-level state fields (tick plus11 maps); owners keys and values both contribute. No key rename silently rewrites references; these are raw equality witnesses, not valid command mutations. [crates/nyon-workshop-core/src/model.rs:129-144]

Together there are72 independent changed-copy cases. Also run equality reflexivity, symmetry (both argument orders for all72), equal detached deep clone, and equality under each already proposed order permutation. Deep copy independence uses the faction name and World inventory mutations above, then frees the clone and verifies original exact S bytes; free original first in a separate case and verify clone bytes. Allocation-failure enumeration for name/key/map insertion, deep clone and encode/digest remains a future implementation witness, not supplied by this matrix. [crates/nyon-workshop-core/src/model.rs:38,129; task-4a-valid-fixture-proposal.md:77; task-4a-foundation-vector-seams.md:137]
