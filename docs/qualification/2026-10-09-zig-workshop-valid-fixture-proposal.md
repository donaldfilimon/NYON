# Task 4A remaining valid-graph and consumer-boundary fixture proposal

2026-10-09. **Current:** handwritten literal inputs, independently calculated standard SHA-256 preimages and source-contract arithmetic recorded below. **Proposed:** later fixture consumers and expected outcomes. **Pending:** independent review, immutable corpus freeze, command admission, authenticated replay, Zig implementation and runtime parity. No Rust or Zig serializer, authority, generator, test or build ran in this task.

This report and its scoped derivation evidence are the only files written. The previous six-literal proposal and approved subset review remain unchanged. Those artifacts identify the remaining coverage groups; this proposal supplies a finite additional subset, not the complete corpus. [task-4a-independent-fixture-review.md, § Admission and coverage boundaries]

## Authority and method

The accepted Workshop V1 design requires typed IDs, checked integer authority, sorted collections, atomic command admission, the ten capacity classes, deterministic phase order and content-addressed revisions. The Library addendum preserves the V1 format and authority and requires exact-catalog validation and replay before session installation. Exact state field order, domain bytes and namespace widths below are current source compatibility supplements where the design is less specific. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:176,213,316,320,337; docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md:8,14; crates/nyon-workshop-core/src/model.rs:129,195; crates/nyon-workshop-core/src/command.rs:850]

Method: choose semantic values, type canonical JSON literally in declaration order, manually sort keys by raw ID/ASCII order, and use only standard `hashlib.sha256` on those exact bytes. Identity calculations use the cited fixed-width domains and integer byte arithmetic. JSON decoding in derivation checks inspects handwritten bytes, never generates expected JSON. No production code is imported or invoked. Stored state and command provenance are separate: raw encoding/digest has no standalone whole-state validator; source predicates give proposed command/replay expectations, not executed admission. [crates/nyon-workshop-core/src/model.rs:165,187,195; crates/nyon-workshop-core/src/command.rs:275,703,850; crates/nyon-workshop-core/src/history.rs:434]

Every named fenced JSON payload is exact ASCII/UTF-8 bytes, excluding its fence and the following newline. Hash domains include one terminal NUL and no newline, length prefix or BOM unless the namespace formula explicitly includes a length. [crates/nyon-workshop-core/src/model.rs:218; crates/nyon-workshop-core/src/command.rs:858; crates/nyon-workshop-core/src/simulation.rs:537]

## Valid graph construction proposal

Start with the validated built-in core pack whose independently transcribed canonical payload and source-contract-derived catalog hash were reviewed previously. Use seed32 all zero, initial empty state at tick0, expected cursor null and ordinal0. The batch below contains 17 operations, 13 distinct local IDs, compatible backward local references and four owners. Faction1 owns systems2/3 and worlds6/7. Owner targets exclude factions and shipments, matching the current eight ownable kinds. [task-4a-independent-fixture-proposal.md:85,88; task-4a-independent-fixture-review.md, § Byte and hash results; crates/nyon-workshop-core/src/command.rs:170,275,489,664]

```BATCH_VALID_GRAPH
{"expected_cursor":null,"expected_tick":0,"operations":[{"kind":"create_faction","local":1,"name":"Union","color_rgb":[12,34,56]},{"kind":"create_system","local":2,"name":"A","position":{"x":0,"y":0}},{"kind":"create_system","local":3,"name":"B","position":{"x":3,"y":4}},{"kind":"create_star","local":4,"system":{"kind":"local","id":2},"name":"A Star","archetype_id":"yellow-dwarf"},{"kind":"create_star","local":5,"system":{"kind":"local","id":3},"name":"B Star","archetype_id":"yellow-dwarf"},{"kind":"create_world","local":6,"system":{"kind":"local","id":2},"primary":{"kind":"local","id":4},"name":"A World","archetype_id":"rocky-world","orbit_radius_milli_au":1000,"orbit_period_ticks":10,"phase_millidegrees":0},{"kind":"create_world","local":7,"system":{"kind":"local","id":3},"primary":{"kind":"local","id":5},"name":"B World","archetype_id":"rocky-world","orbit_radius_milli_au":2000,"orbit_period_ticks":20,"phase_millidegrees":180000},{"kind":"connect_lane","local":8,"a":{"kind":"local","id":2},"b":{"kind":"local","id":3}},{"kind":"create_deposit","local":9,"world":{"kind":"local","id":6},"resource_id":"ore","reserve_units":10},{"kind":"place_industry","local":10,"world":{"kind":"local","id":6},"definition_id":"solar-array","linked_deposit":null},{"kind":"connect_route","local":11,"source":{"kind":"local","id":6},"destination":{"kind":"local","id":7},"resource_id":"energy","batch_units":3},{"kind":"schedule_hazard","local":12,"lane":{"kind":"local","id":8},"hazard_id":"ion-storm","start_tick":10,"duration_ticks":2},{"kind":"place_industry","local":13,"world":{"kind":"local","id":6},"definition_id":"extractor","linked_deposit":{"kind":"local","id":9}},{"kind":"set_owner","target":{"kind":"local","id":2},"faction":{"kind":"local","id":1}},{"kind":"set_owner","target":{"kind":"local","id":3},"faction":{"kind":"local","id":1}},{"kind":"set_owner","target":{"kind":"local","id":6},"faction":{"kind":"local","id":1}},{"kind":"set_owner","target":{"kind":"local","id":7},"faction":{"kind":"local","id":1}}]}
```

The chosen catalog IDs exist; system positions differ; each world's primary belongs to its system; positive radii/periods and phases below360000 satisfy creation predicates. The direct lane has floor distance5. Deposit reserve10, route batch3, distinct source/destination and direct lane satisfy their predicates. The extractor links the source world's ore deposit; the solar has null linked deposit; future hazard start10/duration2 is within ion-storm maximum36000 with no overflow. These are source-grounded expected admission conditions, not admission results. [assets/workshop/core-pack-v1.json:7,10,13,19,27,52; crates/nyon-workshop-core/src/command.rs:361,385,411,442,464,498,540,616]

### Identity and replay provenance

Revision preimage is `ASCII NYON-WORKSHOP-REVISION-V1 + NUL || rules1:u32LE || catalog32 || zero-seed32 || parent-none marker00 || tick0:u64LE || ordinal0:u64LE || exact batch-byte-length:u64LE || BATCH_VALID_GRAPH`. Each created entity preimage is `ASCII NYON-WORKSHOP-ENTITY-V1 + NUL || revision32 || kind:u8 || local:u16LE`; ID is the first16 SHA-256 bytes. Shipment preimages use `ASCII NYON-WORKSHOP-SHIPMENT-V1 + NUL || route16 || dispatch_tick:u64LE`, also first16. [crates/nyon-workshop-core/src/command.rs:850; crates/nyon-workshop-core/src/simulation.rs:537]

Exact namespace and state inputs/results follow after independent calculation. A future runtime witness must submit this exact batch to an empty authority and compare receipt revision/local IDs/post-batch digest, then advance two steps and compare all three state literals. A later replay witness must record the batch at tick0/ordinal0 with post-batch digest, materialize its revision at tick2 using the same catalog and seed, and compare final state/digest. History replays the recorded batch, checks revision and post-batch digest, then advances requested remaining ticks. This recipe is proposed and was not executed here; archive framing, branch IDs and full archive authentication remain later owners. [crates/nyon-workshop-core/src/command.rs:203,263,304; crates/nyon-workshop-core/src/history.rs:434,495,504,511]

The identity calculator completed with exit0. Batch length is2030 bytes (length field `ee07000000000000`); revision preimage is2149 bytes; revision is `4bd91874ea1ae84d0dae94674a5ccfbb365699722b6efe73a39b5427abdba375`. Exact complete preimage hex and each full32-byte hash are in `task-4a-valid-fixture-evidence/derive-identities.log`. That log is derivation evidence, not an authority receipt. [task-4a-valid-fixture-evidence/derive-identities.log:1]

| Label | Local / kind byte | Derived first16-byte EntityId |
| --- | --- | --- |
| Faction | 1 / 0 | `63f518df5eb3fcec49988a5268a87186` |
| System A | 2 / 1 | `276ba6d854e10c707e8a2c7d7b563136` |
| System B | 3 / 1 | `72705000a50df53fdde38b7d8ae0fbcc` |
| Star A | 4 / 2 | `66f23b435e12d5071e0de540fd32a1a2` |
| Star B | 5 / 2 | `e3b2357f43e3841e9fe8e359871fd6a2` |
| World A | 6 / 3 | `a1ab79b780b86ae057c5edafa1106ed8` |
| World B | 7 / 3 | `19db1cfd5d4fe686505d9c9d0d016cbc` |
| Lane | 8 / 4 | `58b0355b77ed5f2324f682c46e478f1a` |
| Deposit | 9 / 5 | `6b750a76af1f6186033c0df234e688b2` |
| Solar | 10 / 6 | `4d4946530457913e248d68410d692932` |
| Route | 11 / 7 | `dfca1f06f3f75acc407aa5f2fb2197ab` |
| Hazard | 12 / 9 | `078c9a39b907cae8d99541c761a04bfe` |
| Extractor | 13 / 6 | `f2481d1c434c9a316d9fc01a0c1b1016` |

These values and their distinctness are standard-hash derivations; nominal kind assignment follows the source enum and allocation formula. At dispatch tick0 the shipment ID is `328c08df01608b3877b8429f1034aec0`; at dispatch tick1 it is `f38be69ca71d2f805143fb6f503edfa3`. [task-4a-valid-fixture-evidence/derive-identities.log:6; crates/nyon-workshop-core/src/model.rs:22; crates/nyon-workshop-core/src/command.rs:877; crates/nyon-workshop-core/src/simulation.rs:537]

### Hand-traced ticks

At tick0 the future hazard is inactive and produces no boundary event. Shipments are initially absent. Primary-industry traversal visits solar ID4d… before extractor IDf2…, both duration1: solar adds4 energy, extractor subtracts1 energy/adds2 ore and reserve falls10→8. There is no processor. Lane distance5 gives `max(1,(5+255)/256)=1`; route dispatches min(batch3,energy3)=3, retaining the zero energy entry, and creates shipment tick0→arrival1. Stored state tick becomes1. [crates/nyon-workshop-core/src/simulation.rs:73,151,240,278,308,356,522; assets/workshop/core-pack-v1.json:19,27; task-4a-valid-fixture-evidence/derive-identities.log:21]

At tick1 delivery first adds3 energy to World B and removes the tick0 shipment. Solar adds4 energy to World A; extractor subtracts1, adds2 ore (2→4) and reserve8→6. Route dispatches3 energy, retaining energy0, and creates the tick1→arrival2 shipment. Stored state tick becomes2. Hazard record and all static maps/owners remain identical. These are manual source phase calculations. [crates/nyon-workshop-core/src/simulation.rs:86,175,240,308,356,128]

### Exact canonical state literals

The three blocks use state/record declaration order and sorted typed-ID maps; World B precedes A, while System A and Star A precede B. Industry order is solar then extractor. Owners order is World B, System A, System B, World A. [crates/nyon-workshop-core/src/model.rs:38,129,195; task-4a-valid-fixture-evidence/derive-identities.log:21]

```STATE_VALID_POST_BATCH
{"tick":0,"factions":{"63f518df5eb3fcec49988a5268a87186":{"name":"Union","color_rgb":[12,34,56]}},"systems":{"276ba6d854e10c707e8a2c7d7b563136":{"name":"A","position":{"x":0,"y":0}},"72705000a50df53fdde38b7d8ae0fbcc":{"name":"B","position":{"x":3,"y":4}}},"stars":{"66f23b435e12d5071e0de540fd32a1a2":{"system":"276ba6d854e10c707e8a2c7d7b563136","name":"A Star","archetype_id":"yellow-dwarf"},"e3b2357f43e3841e9fe8e359871fd6a2":{"system":"72705000a50df53fdde38b7d8ae0fbcc","name":"B Star","archetype_id":"yellow-dwarf"}},"worlds":{"19db1cfd5d4fe686505d9c9d0d016cbc":{"system":"72705000a50df53fdde38b7d8ae0fbcc","primary":"e3b2357f43e3841e9fe8e359871fd6a2","name":"B World","archetype_id":"rocky-world","orbit_radius_milli_au":2000,"orbit_period_ticks":20,"phase_millidegrees":180000,"inventory":{}},"a1ab79b780b86ae057c5edafa1106ed8":{"system":"276ba6d854e10c707e8a2c7d7b563136","primary":"66f23b435e12d5071e0de540fd32a1a2","name":"A World","archetype_id":"rocky-world","orbit_radius_milli_au":1000,"orbit_period_ticks":10,"phase_millidegrees":0,"inventory":{}}},"lanes":{"58b0355b77ed5f2324f682c46e478f1a":{"a":"276ba6d854e10c707e8a2c7d7b563136","b":"72705000a50df53fdde38b7d8ae0fbcc","distance_units":5}},"deposits":{"6b750a76af1f6186033c0df234e688b2":{"world":"a1ab79b780b86ae057c5edafa1106ed8","resource_id":"ore","reserve_units":10}},"industries":{"4d4946530457913e248d68410d692932":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"solar-array","linked_deposit":null,"enabled":true},"f2481d1c434c9a316d9fc01a0c1b1016":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"extractor","linked_deposit":"6b750a76af1f6186033c0df234e688b2","enabled":true}},"routes":{"dfca1f06f3f75acc407aa5f2fb2197ab":{"source":"a1ab79b780b86ae057c5edafa1106ed8","destination":"19db1cfd5d4fe686505d9c9d0d016cbc","resource_id":"energy","batch_units":3}},"shipments":{},"hazards":{"078c9a39b907cae8d99541c761a04bfe":{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}},"owners":{"19db1cfd5d4fe686505d9c9d0d016cbc":"63f518df5eb3fcec49988a5268a87186","276ba6d854e10c707e8a2c7d7b563136":"63f518df5eb3fcec49988a5268a87186","72705000a50df53fdde38b7d8ae0fbcc":"63f518df5eb3fcec49988a5268a87186","a1ab79b780b86ae057c5edafa1106ed8":"63f518df5eb3fcec49988a5268a87186"}}
```

```STATE_VALID_AFTER_TICK_0
{"tick":1,"factions":{"63f518df5eb3fcec49988a5268a87186":{"name":"Union","color_rgb":[12,34,56]}},"systems":{"276ba6d854e10c707e8a2c7d7b563136":{"name":"A","position":{"x":0,"y":0}},"72705000a50df53fdde38b7d8ae0fbcc":{"name":"B","position":{"x":3,"y":4}}},"stars":{"66f23b435e12d5071e0de540fd32a1a2":{"system":"276ba6d854e10c707e8a2c7d7b563136","name":"A Star","archetype_id":"yellow-dwarf"},"e3b2357f43e3841e9fe8e359871fd6a2":{"system":"72705000a50df53fdde38b7d8ae0fbcc","name":"B Star","archetype_id":"yellow-dwarf"}},"worlds":{"19db1cfd5d4fe686505d9c9d0d016cbc":{"system":"72705000a50df53fdde38b7d8ae0fbcc","primary":"e3b2357f43e3841e9fe8e359871fd6a2","name":"B World","archetype_id":"rocky-world","orbit_radius_milli_au":2000,"orbit_period_ticks":20,"phase_millidegrees":180000,"inventory":{}},"a1ab79b780b86ae057c5edafa1106ed8":{"system":"276ba6d854e10c707e8a2c7d7b563136","primary":"66f23b435e12d5071e0de540fd32a1a2","name":"A World","archetype_id":"rocky-world","orbit_radius_milli_au":1000,"orbit_period_ticks":10,"phase_millidegrees":0,"inventory":{"energy":0,"ore":2}}},"lanes":{"58b0355b77ed5f2324f682c46e478f1a":{"a":"276ba6d854e10c707e8a2c7d7b563136","b":"72705000a50df53fdde38b7d8ae0fbcc","distance_units":5}},"deposits":{"6b750a76af1f6186033c0df234e688b2":{"world":"a1ab79b780b86ae057c5edafa1106ed8","resource_id":"ore","reserve_units":8}},"industries":{"4d4946530457913e248d68410d692932":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"solar-array","linked_deposit":null,"enabled":true},"f2481d1c434c9a316d9fc01a0c1b1016":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"extractor","linked_deposit":"6b750a76af1f6186033c0df234e688b2","enabled":true}},"routes":{"dfca1f06f3f75acc407aa5f2fb2197ab":{"source":"a1ab79b780b86ae057c5edafa1106ed8","destination":"19db1cfd5d4fe686505d9c9d0d016cbc","resource_id":"energy","batch_units":3}},"shipments":{"328c08df01608b3877b8429f1034aec0":{"route":"dfca1f06f3f75acc407aa5f2fb2197ab","source":"a1ab79b780b86ae057c5edafa1106ed8","destination":"19db1cfd5d4fe686505d9c9d0d016cbc","resource_id":"energy","units":3,"dispatched_tick":0,"arrival_tick":1}},"hazards":{"078c9a39b907cae8d99541c761a04bfe":{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}},"owners":{"19db1cfd5d4fe686505d9c9d0d016cbc":"63f518df5eb3fcec49988a5268a87186","276ba6d854e10c707e8a2c7d7b563136":"63f518df5eb3fcec49988a5268a87186","72705000a50df53fdde38b7d8ae0fbcc":"63f518df5eb3fcec49988a5268a87186","a1ab79b780b86ae057c5edafa1106ed8":"63f518df5eb3fcec49988a5268a87186"}}
```

```STATE_VALID_AFTER_TICK_1
{"tick":2,"factions":{"63f518df5eb3fcec49988a5268a87186":{"name":"Union","color_rgb":[12,34,56]}},"systems":{"276ba6d854e10c707e8a2c7d7b563136":{"name":"A","position":{"x":0,"y":0}},"72705000a50df53fdde38b7d8ae0fbcc":{"name":"B","position":{"x":3,"y":4}}},"stars":{"66f23b435e12d5071e0de540fd32a1a2":{"system":"276ba6d854e10c707e8a2c7d7b563136","name":"A Star","archetype_id":"yellow-dwarf"},"e3b2357f43e3841e9fe8e359871fd6a2":{"system":"72705000a50df53fdde38b7d8ae0fbcc","name":"B Star","archetype_id":"yellow-dwarf"}},"worlds":{"19db1cfd5d4fe686505d9c9d0d016cbc":{"system":"72705000a50df53fdde38b7d8ae0fbcc","primary":"e3b2357f43e3841e9fe8e359871fd6a2","name":"B World","archetype_id":"rocky-world","orbit_radius_milli_au":2000,"orbit_period_ticks":20,"phase_millidegrees":180000,"inventory":{"energy":3}},"a1ab79b780b86ae057c5edafa1106ed8":{"system":"276ba6d854e10c707e8a2c7d7b563136","primary":"66f23b435e12d5071e0de540fd32a1a2","name":"A World","archetype_id":"rocky-world","orbit_radius_milli_au":1000,"orbit_period_ticks":10,"phase_millidegrees":0,"inventory":{"energy":0,"ore":4}}},"lanes":{"58b0355b77ed5f2324f682c46e478f1a":{"a":"276ba6d854e10c707e8a2c7d7b563136","b":"72705000a50df53fdde38b7d8ae0fbcc","distance_units":5}},"deposits":{"6b750a76af1f6186033c0df234e688b2":{"world":"a1ab79b780b86ae057c5edafa1106ed8","resource_id":"ore","reserve_units":6}},"industries":{"4d4946530457913e248d68410d692932":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"solar-array","linked_deposit":null,"enabled":true},"f2481d1c434c9a316d9fc01a0c1b1016":{"world":"a1ab79b780b86ae057c5edafa1106ed8","definition_id":"extractor","linked_deposit":"6b750a76af1f6186033c0df234e688b2","enabled":true}},"routes":{"dfca1f06f3f75acc407aa5f2fb2197ab":{"source":"a1ab79b780b86ae057c5edafa1106ed8","destination":"19db1cfd5d4fe686505d9c9d0d016cbc","resource_id":"energy","batch_units":3}},"shipments":{"f38be69ca71d2f805143fb6f503edfa3":{"route":"dfca1f06f3f75acc407aa5f2fb2197ab","source":"a1ab79b780b86ae057c5edafa1106ed8","destination":"19db1cfd5d4fe686505d9c9d0d016cbc","resource_id":"energy","units":3,"dispatched_tick":1,"arrival_tick":2}},"hazards":{"078c9a39b907cae8d99541c761a04bfe":{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}},"owners":{"19db1cfd5d4fe686505d9c9d0d016cbc":"63f518df5eb3fcec49988a5268a87186","276ba6d854e10c707e8a2c7d7b563136":"63f518df5eb3fcec49988a5268a87186","72705000a50df53fdde38b7d8ae0fbcc":"63f518df5eb3fcec49988a5268a87186","a1ab79b780b86ae057c5edafa1106ed8":"63f518df5eb3fcec49988a5268a87186"}}
```

## Exact consumer comparisons

**Dynamic view:** take `STATE_VALID_POST_BATCH` as the static receiver. Pass tick2 and the complete `worlds`, `deposits`, `shipments` maps from `STATE_VALID_AFTER_TICK_1` to the dynamic digest view. Exact expected canonical bytes and digest are those of `STATE_VALID_AFTER_TICK_1`. Static factions/systems/stars/lanes/industries/routes/hazards/owners come from the receiver; receiver tick/worlds/deposits/shipments must be ignored. All four overrides differ in this case. The separate ordinary full-state consumer constructs a detached state with those four substitutions and expects the same bytes/digest. This compares concrete literal bytes, not merely two implementation-produced hashes. Borrowed replacements must remain valid for the complete synchronous encoding call; allocation and lifetime witnesses are Pending. [crates/nyon-workshop-core/src/model.rs:191,195,220; crates/nyon-workshop-core/src/simulation.rs:133]

**Multi-key insertion cases:** construct typed maps directly using the corresponding exact records above, then canonical-encode the whole state. Input insertion order is the explicit list below; values and IDs are unchanged. Every permutation expects exactly `STATE_VALID_AFTER_TICK_1`, including its hash. This is a constructor/ordered-container proposal, not a claim that arbitrary raw-state parser quirks were executed. Map order follows derived typed-ID ordering; equality follows values, not resident insertion order. [crates/nyon-workshop-core/src/ids.rs:11,14; crates/nyon-workshop-core/src/model.rs:129,187]

| Case | Concrete insertion order |
| --- | --- |
| MAPS_REVERSE | systems B,A; stars B,A; worlds A,B; industries extractor,solar; owners World A,System B,System A,World B; inventory World A ore4,energy0 |
| MAPS_MIXED | systems B,A; stars A,B; worlds A,B; industries solar,extractor; owners System B,World B,World A,System A; inventory World A energy0,ore4 |
| MAPS_SORTED_CONTROL | systems A,B; stars A,B; worlds B,A; industries solar,extractor; owners World B,System A,System B,World A; inventory energy0,ore4 |

A separate raw two-faction map makes incorrect host-endian sorting observable. IDs A=`00010000000000000000000000000000`, B=`01000000000000000000000000000000`; insertion B,A must emit the following map. A sorts first by its first differing byte, despite A's little-endian integer being larger than B's. The matching owners map uses the same ordered keys. These literal raw-map cases assert container order only and have no command-derived identity/admission claim. [crates/nyon-workshop-core/src/ids.rs:14,17; crates/nyon-workshop-core/src/model.rs:133,143]

```FACTION_ENDIAN_ORDER
{"00010000000000000000000000000000":{"name":"A","color_rgb":[0,0,0]},"01000000000000000000000000000000":{"name":"B","color_rgb":[255,255,255]}}
```

```OWNERS_ENDIAN_ORDER
{"00010000000000000000000000000000":"01000000000000000000000000000000","01000000000000000000000000000000":"00010000000000000000000000000000"}
```

**Absent versus zero:** the two literal inventories below are distinct structural values and distinct canonical bytes. Replacing World A's inventory in the final valid-graph literal with the first creates a raw comparison variant whose `energy` entry is absent; the second is the actual manually traced inventory with retained zero. Their complete state inputs and domain hashes are recorded in the scoped derivation log. A canonical encoder/equality consumer must preserve the difference. A simulation arithmetic lookup may return numeric0 for either, which does not authorize erasing the resident zero entry. [crates/nyon-workshop-core/src/model.rs:62,70; crates/nyon-workshop-core/src/simulation.rs:402,522]

```INVENTORY_ENERGY_ABSENT
{"ore":4}
```

```INVENTORY_ENERGY_ZERO
{"energy":0,"ore":4}
```

The raw comparison variant is defined by one exact byte replacement in `STATE_VALID_AFTER_TICK_1`, `"inventory":{"energy":0,"ore":4}`→`"inventory":{"ore":4}`, occurring once. It has no additional command/replay provenance. These exact two spelling fragments fully specify its bytes without a serializer.

## Separate invalid raw states

`STATE_RAW_DANGLING` contains one world with absent system/primary and an owner whose faction is absent. The model can represent and hash it, but it is not a valid authority state. `contains_entity(world4)` is true and `entity_kind(world4)` is World; system2/primary3/faction1 are absent. A proposed `CreateStar` existing-system2 reference on this raw state returns `UnknownEntity` at resolution; a local missing reference is a different `InvalidLocalReference` case. A future whole-state admission error is unknown because model has no such validator. [crates/nyon-workshop-core/src/model.rs:129,165,187; crates/nyon-workshop-core/src/command.rs:385,720]

```STATE_RAW_DANGLING
{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":1,"orbit_period_ticks":1,"phase_millidegrees":0,"inventory":{}}},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{"00000000000000000000000000000004":"00000000000000000000000000000001"}}
```

`STATE_RAW_CROSS_KIND_DUPLICATE` contains the same exact resident key in factions and systems. It is not a duplicate JSON key within one object: both objects have unique keys. `entity_kind(id1)` returns Faction, the first matching collection, while both maps contain it. `contains_entity(id1)` is true. A proposed `CreateStar` with existing-system id1 returns `WrongEntityKind` even though the systems map also contains id1. This differs from creator allocation's `IdentityCollision`, which requires collision with the newly derived candidate ID. Do not invent that collision witness from this raw example. [crates/nyon-workshop-core/src/model.rs:165; crates/nyon-workshop-core/src/command.rs:703,720]

```STATE_RAW_CROSS_KIND_DUPLICATE
{"tick":0,"factions":{"00000000000000000000000000000001":{"name":"F","color_rgb":[1,2,3]}},"systems":{"00000000000000000000000000000001":{"name":"S","position":{"x":0,"y":0}}},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}
```

```BATCH_RAW_MISSING_SYSTEM
{"expected_cursor":null,"expected_tick":0,"operations":[{"kind":"create_star","local":1,"system":{"kind":"existing","id":"00000000000000000000000000000002"},"name":"T","archetype_id":"yellow-dwarf"}]}
```

```BATCH_RAW_CROSS_KIND_SYSTEM
{"expected_cursor":null,"expected_tick":0,"operations":[{"kind":"create_star","local":1,"system":{"kind":"existing","id":"00000000000000000000000000000001"},"name":"T","archetype_id":"yellow-dwarf"}]}
```

Those two batches target the source's internal `apply_recorded_batch`/operation resolution consumer with the respective raw receiver, tick0/parentNone. Public `WorkshopAuthority::new` starts from empty state and exposes no public raw-state setter, so this is an internal test-seam proposal, not a public install path. Expected operation rejection precedes insertion; actual atomic/runtime rejection remains Pending. [crates/nyon-workshop-core/src/command.rs:170,199,263,304]

## Distinct-ID historical hazard predicate fixture

`HAZARD_IDS_128_CURRENT` is a fully enumerated list of128 distinct raw16-byte IDs. Bind each ID to the exact `HAZARD_CURRENT_RECORD` below. Add historical ID `00000000000000000000000000001080` bound to `HAZARD_HISTORICAL_RECORD`. Receiver tick is10, and the lane reference is the existing lane from the valid graph. These are **raw count-predicate inputs**, not command-derived hazard identities or an admitted history. Their sole fixture consumer is the scheduled-or-active count/capacity guard; a model encoder may separately exercise their exact bytes. [crates/nyon-workshop-core/src/command.rs:623,755; crates/nyon-workshop-core/src/model.rs:119,129]

```HAZARD_IDS_128_CURRENT
00000000000000000000000000001000
00000000000000000000000000001001
00000000000000000000000000001002
00000000000000000000000000001003
00000000000000000000000000001004
00000000000000000000000000001005
00000000000000000000000000001006
00000000000000000000000000001007
00000000000000000000000000001008
00000000000000000000000000001009
0000000000000000000000000000100a
0000000000000000000000000000100b
0000000000000000000000000000100c
0000000000000000000000000000100d
0000000000000000000000000000100e
0000000000000000000000000000100f
00000000000000000000000000001010
00000000000000000000000000001011
00000000000000000000000000001012
00000000000000000000000000001013
00000000000000000000000000001014
00000000000000000000000000001015
00000000000000000000000000001016
00000000000000000000000000001017
00000000000000000000000000001018
00000000000000000000000000001019
0000000000000000000000000000101a
0000000000000000000000000000101b
0000000000000000000000000000101c
0000000000000000000000000000101d
0000000000000000000000000000101e
0000000000000000000000000000101f
00000000000000000000000000001020
00000000000000000000000000001021
00000000000000000000000000001022
00000000000000000000000000001023
00000000000000000000000000001024
00000000000000000000000000001025
00000000000000000000000000001026
00000000000000000000000000001027
00000000000000000000000000001028
00000000000000000000000000001029
0000000000000000000000000000102a
0000000000000000000000000000102b
0000000000000000000000000000102c
0000000000000000000000000000102d
0000000000000000000000000000102e
0000000000000000000000000000102f
00000000000000000000000000001030
00000000000000000000000000001031
00000000000000000000000000001032
00000000000000000000000000001033
00000000000000000000000000001034
00000000000000000000000000001035
00000000000000000000000000001036
00000000000000000000000000001037
00000000000000000000000000001038
00000000000000000000000000001039
0000000000000000000000000000103a
0000000000000000000000000000103b
0000000000000000000000000000103c
0000000000000000000000000000103d
0000000000000000000000000000103e
0000000000000000000000000000103f
00000000000000000000000000001040
00000000000000000000000000001041
00000000000000000000000000001042
00000000000000000000000000001043
00000000000000000000000000001044
00000000000000000000000000001045
00000000000000000000000000001046
00000000000000000000000000001047
00000000000000000000000000001048
00000000000000000000000000001049
0000000000000000000000000000104a
0000000000000000000000000000104b
0000000000000000000000000000104c
0000000000000000000000000000104d
0000000000000000000000000000104e
0000000000000000000000000000104f
00000000000000000000000000001050
00000000000000000000000000001051
00000000000000000000000000001052
00000000000000000000000000001053
00000000000000000000000000001054
00000000000000000000000000001055
00000000000000000000000000001056
00000000000000000000000000001057
00000000000000000000000000001058
00000000000000000000000000001059
0000000000000000000000000000105a
0000000000000000000000000000105b
0000000000000000000000000000105c
0000000000000000000000000000105d
0000000000000000000000000000105e
0000000000000000000000000000105f
00000000000000000000000000001060
00000000000000000000000000001061
00000000000000000000000000001062
00000000000000000000000000001063
00000000000000000000000000001064
00000000000000000000000000001065
00000000000000000000000000001066
00000000000000000000000000001067
00000000000000000000000000001068
00000000000000000000000000001069
0000000000000000000000000000106a
0000000000000000000000000000106b
0000000000000000000000000000106c
0000000000000000000000000000106d
0000000000000000000000000000106e
0000000000000000000000000000106f
00000000000000000000000000001070
00000000000000000000000000001071
00000000000000000000000000001072
00000000000000000000000000001073
00000000000000000000000000001074
00000000000000000000000000001075
00000000000000000000000000001076
00000000000000000000000000001077
00000000000000000000000000001078
00000000000000000000000000001079
0000000000000000000000000000107a
0000000000000000000000000000107b
0000000000000000000000000000107c
0000000000000000000000000000107d
0000000000000000000000000000107e
0000000000000000000000000000107f
```

```HAZARD_CURRENT_RECORD
{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}
```

```HAZARD_HISTORICAL_RECORD
{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":0,"duration_ticks":10,"cancelled":false}
```

Exact state-byte composition: use `STATE_VALID_POST_BATCH`, replace its single `"tick":0` prefix by `"tick":10`, and replace its single hazards-object fragment with the object entries `"<each literal ID>":<HAZARD_CURRENT_RECORD>` in the displayed ID order followed by historical-ID/record. Concatenation uses ASCII `{`, `}`, `:`, `,`, and quotes only. It has129 distinct resident keys, 128 computed ends12>tick10 and one end10==tick10; source count is128. The complete minified payload and full domain preimage are printed in `derive-state-boundaries.log`, so no consumer must implement this composition to obtain an expected hash. This is hand-authored literal-fragment composition, not a production serializer or a runtime-generated fixture. [crates/nyon-workshop-core/src/model.rs:129,218; crates/nyon-workshop-core/src/command.rs:755]

| Concrete predicate variant | Expected source predicate / command consequence |
| --- | --- |
| 129 resident records as specified | count128; next ScheduleHazard capacity guard returns `Capacity { kind: Hazard }` before reference/value checks. Preserve every historical record; no total-storage cap128. |
| Same129 records, ID1000 record `cancelled:true` | count127; next schedule's capacity guard passes. Later lane/catalog/value/identity checks still apply, so guard success is not whole-command admission. |
| At tick10, uncancelled start0/duration11 | contributes1, checked end11>10 |
| At tick10, uncancelled start20/duration1 | contributes1 although future; this is scheduled capacity |
| At tick10, cancelled start20/duration1 | contributes0 after checked end21 |
| Replace historical ID1080 by overflow record below, even cancelled | count consumer returns `Arithmetic` before examining cancelled. There is no successful numeric count. |
| New ScheduleHazard request, receiver tick18446744073709551614, zero existing hazards, valid lane/catalog, start18446744073709551615/duration1 | capacity guard passes, then new value predicate returns `InvalidValue` for overflowing start+duration. |
| Same overflowing new request but receiver with128 qualifying records | `Capacity { kind: Hazard }` takes precedence; do not claim it reaches `InvalidValue`. |

All table outcomes are source-derived and unexecuted. Checked end precedes cancellation in the existing-record count; capacity precedes lane/catalog/value validation in a new request. This distinction is specific to the named consumers. A cancelled hazard may be skipped earlier by `hazard_boundaries`, but `active_hazard_ratios` also checks end before cancellation; do not generalize one phase's shortcut into a successful full tick. [crates/nyon-workshop-core/src/command.rs:623,632,743,755; crates/nyon-workshop-core/src/simulation.rs:151,464]

```HAZARD_RAW_CANCELLED_OVERFLOW
{"lane":"58b0355b77ed5f2324f682c46e478f1a","hazard_id":"ion-storm","start_tick":18446744073709551615,"duration_ticks":1,"cancelled":true}
```

The count fixture is complete for its finite raw consumer. A separate **command-admitted** historical population requires constructing actual derived identities across bounded batches/ticks, recording receipts and replaying them; that is still Pending. This report never promotes raw count inputs to an installable save or claims successful runtime capacity rejection.

## Finite scalar constructor and decoded-ID corpus

These inputs name their actual scalar consumer. String notation is decoded bytes; `ASCII A repeated N` means exactly N bytes0x41 and no implicit newline, while hex inputs state complete exceptional bytes. Constructor errors and pure helper booleans below follow source predicates; no runtime constructor or deserializer ran. Unknown raw JSON lexeme behavior is explicitly excluded. [crates/nyon-workshop-core/src/ids.rs:43,94,145,195,269]

| Consumer | Exact finite inputs | Proposed outcomes |
| --- | --- | --- |
| CatalogId::new | `a`; `a0_-`; `abcdefghijklmnopqrstuvwxyz012345` (32 bytes) | success with unchanged content |
| CatalogId::new | empty; `abcdefghijklmnopqrstuvwxyz0123456` (33); `0a`; `A`; `a.b`; `a b`; UTF-8 hex `c3a9` | `IdentifierError::CatalogId` |
| ObjectName::new | `A`; `A B`; `!~`; semantic bytes `51225c` (Q, quote, backslash); ASCII A repeated64 | success with unchanged content; quoted JSON spelling already independently reviewed |
| ObjectName::new | empty; ASCII A repeated65; ` A`; `A `; `A  B`; decoded bytes `410042`, `411f42`, `417f42`, UTF-8 `41c3a9` | `IdentifierError::ObjectName` |
| is_pack_id helper | `a.b`; ASCII a repeated48 | true; dot allowed here |
| is_pack_id helper | empty; ASCII a repeated49; `0a`; `A`; `a/b` | false |
| is_description helper | empty; ` A  B `; ASCII A repeated512 | true; whitespace restrictions for names do not apply |
| is_description helper | ASCII A repeated513; hex `00`, `1f`, `7f`, `c3a9` | false |
| GalaxyCoord::new | -134217728,-1,0,1,134217728 | success, exact signed values retained |
| GalaxyCoord::new | -134217729,134217729,-9223372036854775808,9223372036854775807 | `IdentifierError::CoordinateOutOfRange`; i64-bound inputs still fail coordinate bounds |
| GalaxyPointV1::new | (-134217728,134217728) | success; x then y in canonical bytes |
| GalaxyPointV1::new | (134217729,0), (0,-134217729) | CoordinateOutOfRange; first failing axis wins at construction |

Source checks are byte-length based with no trimming, case folding or Unicode normalization. Constructor accepted strings are owned; content/lifetime/allocation witnesses belong the Zig implementation. Pack-description helpers do not imply complete pack admission. [crates/nyon-workshop-core/src/ids.rs:94,145,227,269,277,285,293]

For **each** of EntityId and BranchId, test the following decoded string rows as its own nominal consumer. For **each** of RevisionId, CatalogHash and StateDigest, use the second group. The raw JSON wire shape alone does not distinguish EntityId from BranchId or the three32-byte types. Nominal cross-type rejection is a future Zig compile-time/API property; do not fabricate a runtime parser rejection for identical valid wire strings. [crates/nyon-workshop-core/src/ids.rs:11,26,71]

| Decoded string group | Exact bytes / strings | Proposed decoded-validator outcome |
| --- | --- | --- |
| 16-byte valid | `00000000000000000000000000000000`; `ffffffffffffffffffffffffffffffff`; `000102030405060708090a0b0c0d0e0f` | success, raw16 bytes retained |
| 16-byte length failures | `0000000000000000000000000000000` (31); `000000000000000000000000000000000` (33) | custom `invalid canonical identifier` |
| 16-byte character failures,32-byte-length strings | `A0000000000000000000000000000000`; `g0000000000000000000000000000000`; `/0000000000000000000000000000000` | same custom error |
| 16-byte non-ASCII case | hex decoded bytes `c3a9` followed by30 bytes0x30 (32-byte string) | same custom error despite correct byte length |
| 32-byte valid | `0000000000000000000000000000000000000000000000000000000000000000`; `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`; `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f` | success, raw32 bytes retained |
| 32-byte length failures |63 bytes0x30;65 bytes0x30 | same custom error |
| 32-byte character failures | ASCII A/g/slash respectively followed by63 bytes0x30 | same custom error |
| 32-byte non-ASCII case | hex `c3a9` followed by62 bytes0x30 (64-byte string) | same custom error |

Each group contains nine decoded inputs including three valid controls. The source first checks exact byte length and then ASCII digit/a–f classes. Error display prefix/JSON location depends on the consuming deserializer and was not characterized. Escaped decoded IDs, negative-zero signed/unsigned lexemes, quoted numeric forms, JSON numeric overflow mapping, duplicate raw JSON keys and unknown-field handling through aggregate consumers remain separate Pending ingress cases. [crates/nyon-workshop-core/src/ids.rs:43,71,209; task-4a-foundation-vector-seams.md, § Raw schema, validation and capacity boundary]

Additional checked-distance finite pair cases: (0,0)→(255,0) gives255; (0,0)→(256,0) gives256; (0,0)→(257,0) gives257. The later route consumer derives1,1,2 travel ticks respectively using `(distance+255)/256` and minimum1. These supplement the reviewed diagonal/floor-distance cases; no new legal-coordinate overflow case is claimed. [crates/nyon-workshop-core/src/ids.rs:234; crates/nyon-workshop-core/src/simulation.rs:375]

## Residual groups and freeze boundary

This finite addition covers source-grounded command/tick construction of a valid all-ten-kind graph, full literal state bytes across three stages, concrete map order permutations, absent-versus-zero and dynamic override comparison, separate raw dangling/cross-kind receivers, a fully specified129-resident/128-count hazard predicate case, and a finite scalar boundary subset. It does not close the entire4A or Workshop corpus.

Still Pending: independent review of this proposal and compatibility supplement; immutable fixture files/manifests/checksums; actual command admission, receipt/state/replay parity and invalid atomic preservation; command-admitted129-record historical history; complete per-field equality mutation matrix; clone/allocation/input lifetime tests; nominal Zig compile-time rejection; remaining aggregate raw ingress characterization; strict complete pack/depth/cardinality/quantity/duration/recipe/reference cases; revision parent-some, root/fork/cursor/entity/shipment namespace families beyond the construction derivations here; every Workshop implementation, gate, archive/session admission and host/runtime result. The current SHA calculations cannot replace any of those witnesses. [task-4a-foundation-vector-seams.md, § Namespace fixtures; task-4a-independent-fixture-review.md, § Admission and coverage boundaries; docs/superpowers/specs/2026-09-02-nyon-v2-design.md:316,359,370]

The proposal must be independently reviewed before any corpus freeze. A proposal report hash binds review input only and is not a frozen fixture manifest or task-completion claim.

## Actual derivation evidence

All commands ran from `/Users/donaldfilimon/dev/active/NYON`. The following three standard-library scripts completed with actual command exit0. They read handwritten proposal literals/current source, performed explicit byte/arithmetic comparisons and ordinary SHA-256 calculations, and printed scoped evidence. JSON decoding was inspection only; none used a JSON encoder, imported NYON, invoked a serializer/generator/authority or ran an implementation gate. The scripts are preserved verbatim with their complete logs in `task-4a-valid-fixture-evidence/`. [task-4a-valid-fixture-evidence/derive-identities.py:1; task-4a-valid-fixture-evidence/derive-state-boundaries.py:1; task-4a-valid-fixture-evidence/derive-scalars-order.py:1]

| Executed command | Recorded derivation result |
| --- | --- |
| `python3 .../task-4a-valid-fixture-evidence/derive-identities.py` | 2030 literal batch bytes,2149 complete revision preimage bytes; 13 distinct created first16 IDs; two dispatch-tick shipment preimages; full preimages/full32 hashes and first16 outputs preserved. |
| `python3 .../task-4a-valid-fixture-evidence/derive-state-boundaries.py` | 11 source field-order lists across five handwritten whole states;14 distinct entities/ten kinds/four owners in final graph; manual tick values; exact dynamic four-override byte comparison; endian-map order; absent-versus-zero;129 distinct hazard residents/count128/cancel-count127/overflow Arithmetic; seven complete state payloads/preimages. |
| `python3 .../task-4a-valid-fixture-evidence/derive-scalars-order.py` | 39 finite string byte-class cases; nine coordinate bounds;45 decoded fixed-ID cases across five nominal consumers;15 concrete insertion-list memberships;255/256/257 distance and1/1/2 travel arithmetic. |

Those result labels describe source-contract calculations, not Rust/Zig tests, command admission, replay or application execution. Full exact minified payload bytes and every complete state domain preimage hex are in the state-boundary log. The following hashes are calculated from those literal inputs, never implementation-produced state output. [task-4a-valid-fixture-evidence/derive-state-boundaries.log:6]

| Case | Payload / complete domain-preimage bytes | Independently calculated state SHA-256 |
| --- | --- | --- |
| STATE_VALID_POST_BATCH |2329 /2352 |`be16ba67a02e59b5d235863175066a7ffc1dc9d2a3348536467c7b9cf1e3f9b3` |
| STATE_VALID_AFTER_TICK_0 |2588 /2611 |`e8e1f2ac8b0e08f81a3183ce5e86fdcf8c2e995f22dfa978d5a0aaa6f9260c6f` |
| STATE_VALID_AFTER_TICK_1 |2598 /2621 |`d10142cc43ede9c2e94d26ba489f888e57ce878a5e46fc9f4959fd3fe4ef6465` |
| STATE_RAW_DANGLING |474 /497 |`4bd5d8f19ea678f5107ea0b013bbcac90f11cda485e11cf6c6e97b1172242746` |
| STATE_RAW_CROSS_KIND_DUPLICATE |292 /315 |`380f940f29986675ca18d5602607b00263b5131d7fdf73c99605706be6e836a3` |
| STATE_RAW_ENERGY_ABSENT |2587 /2610 |`420ffc864553d8d6a8b9c0c50e9e938f82618b36ebe8a173b728bbed0983d885` |
| STATE_RAW_129_STORED_128_COUNTED |22298 /22321 |`92bf6f358ea671639c5c5a726fbb2a03bc183cf5373fe3aeba21dc941c64bc82` |

### Source fingerprints

The before binding file records these12 inputs. The final binding check re-reads and compares all12; its outcome and command exit are in the scoped evidence. Existing accepted documents/source/earlier proposal/review were not changed by this author. Fingerprints below are ordinary file SHA-256, separate from domain-separated Workshop identities. [task-4a-valid-fixture-evidence/source-bindings-before.txt:1]

| Input | File SHA-256 |
| --- | --- |
| `docs/superpowers/specs/2026-09-02-nyon-v2-design.md` |`f386a8c074ad87c431a1066f7a55a8a96270c84692554b9c4a919c2d543e195d` |
| `docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md` |`784781b8f7c23eabbe3dbed0c2fd90fbf1a7a9bd7701802bc5ced759f3c898ae` |
| `crates/nyon-workshop-core/src/ids.rs` |`489895dc85c97f524a3727677fb63be68e3678897e811a48c7013789af82637d` |
| `crates/nyon-workshop-core/src/model.rs` |`e69a03a72cd278e3159858a281ef71526b3683218a3fa5d7955ce4814f77a6c9` |
| `crates/nyon-workshop-core/src/pack.rs` |`acc1c3d19b80a051461e0628bde62f66dfafca4cd846a874a36b829b7fe5bb3c` |
| `crates/nyon-workshop-core/src/command.rs` |`576716226d01e5f23855456e27f1d88aa1d39c2192612a26114a59725024f782` |
| `crates/nyon-workshop-core/src/simulation.rs` |`c4076de2542d2688546da88d33eedde1c85e35b9407684b7651e040c5bbc819b` |
| `crates/nyon-workshop-core/src/history.rs` |`c5cd94e8c196a4bd0bcda637184fd597d04d69fbc565a6dc4c2dda85c5903080` |
| `assets/workshop/core-pack-v1.json` |`5565bd115dcc53858d47e9e712c270d5ad460a4bd441625ff9ffaf59d65a6db5` |
| `task-4a-foundation-vector-seams.md` |`5ee6c1cadf997c3e730e53a2e08837718b6e5de8e8e9a46b643a89be3d05c954` |
| `task-4a-independent-fixture-proposal.md` |`f3a2d982171840443c334cc44c7a79493622c752627874cd82e58b86e4654027` |
| `task-4a-independent-fixture-review.md` |`2b154ab046e150c75339f52847f50bf0ed691490f301a6d2691d511cd7ff3b2d` |

Author self-check: every complete literal retains declaration order and sorted keys; raw invalid/count receivers are separate from the source-grounded valid construction; zero entries are retained; source predicate errors are labeled by consumer; namespace preimages contain actual NUL bytes and fixed-width LE integers; no proposal result is labeled admission/replay/gate success. Independent review remains required.

Gaps: independent review and corpus freeze; residual groups listed above; every implementation, admission, replay and runtime result remains Pending.
