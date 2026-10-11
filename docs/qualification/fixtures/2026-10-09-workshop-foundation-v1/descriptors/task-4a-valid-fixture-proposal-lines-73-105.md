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
