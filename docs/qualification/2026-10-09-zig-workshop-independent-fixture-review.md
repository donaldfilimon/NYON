# Task 4A–B independent fixture derivation review

2026-10-09. **Verdict: APPROVED for the reviewed literal subset only.** Critical: 0. Important: 0. Minor: 0. No byte difference or incorrect derived value was found. This approves the proposal's six explicit block literals, four complete SHA-256 inputs/results, scalar/order witnesses and six distance witnesses as source-contract-derived candidates. It does not freeze the full corpus, complete Workshop4A/4B, approve authority installation of raw stress data, or supply any Rust/Zig runtime result.

Reviewed proposal: .superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-independent-fixture-proposal.md, SHA-256 f3a2d982171840443c334cc44c7a79493622c752627874cd82e58b86e4654027. Earlier foundation report read in full: .superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-foundation-vector-seams.md, SHA-256 5ee6c1cadf997c3e730e53a2e08837718b6e5de8e8e9a46b643a89be3d05c954. Both fingerprints were checked again during review. The exact verification commands and their exit status appear below.

Only this review file was written. Other agents' changes were preserved. No fixture, production source, dependency, build/profile, gate, commit or remote action was changed or executed. Python standard-library arithmetic, hashing, JSON decoding for inspection, and literal byte comparison supplied the mechanical checks. No production serializer, generator, Rust or Zig executable supplied expected values.

## Authority and derivation independence

The accepted Workshop design supplies typed integer identities, checked geometry, ordered collections, capacity semantics and canonical sorted/minified packs. It names the exact pack hash domain, but does not enumerate every state field or state hash domain. The Library addendum explicitly preserves canonical V1 formats and simulation authority. The exact state/record field ordering and state-domain bytes are therefore current source compatibility supplements, not quotations from a complete independent wire specification. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:7,176-235,363-390; docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md:3,8,14-18; crates/nyon-workshop-core/src/model.rs:38-163,187-238]

I independently reconstructed the raw state from one explicitly typed byte fragment per collection, in declaration order, applying the chosen case parameters. I independently transcribed the canonical core pack from the committed semantic input, sorting the definition IDs and quantity IDs according to the declarations. The command below compares those reconstructed bytes against the proposal, checks declaration field lists directly, and only then hashes them. JSON decoding is used to compare semantic input values and inspect keys; no JSON encoder emits expected bytes. This is stronger than merely hashing the author's supplied input, but remains source-contract-derived rather than independent of the Rust source contract. [assets/workshop/core-pack-v1.json:1-59; crates/nyon-workshop-core/src/model.rs:38-144; crates/nyon-workshop-core/src/pack.rs:24-57,128-161,190-203,316-342]

**Provenance limit:** agreement with the two existing Rust literals is corroboration. It does not establish how those older goldens were originally derived. The migration plan expressly keeps independent fixtures and Rust differential witnesses distinct. [crates/nyon-workshop-core/tests/creator.rs:183-195; crates/nyon-workshop-core/tests/pack.rs:8-22; docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:154-157]

## Reviewed derivations

| Surface | Independent check and conclusion | Primary source |
| --- | --- | --- |
| Fixed IDs | High nibble then low nibble yields the 00..0f / 00..1f lowercase strings. EntityId and BranchId remain nominal 16-byte types; the other three wrappers remain nominal 32-byte types. Zero and all-FF are representable. Byte-array A=000100… sorts before B=010000…, whereas their little-endian integer values reverse that order. | crates/nyon-workshop-core/src/ids.rs:11-75 |
| Numeric wrappers | Tick is numeric u64, local is numeric u16. Decimal maxima independently equal 2^64−1=18446744073709551615 and 2^16−1=65535. 9007199254740993 is 2^53+1 and must survive as an integer. World radius/phase are u32, maximum 4294967295. | crates/nyon-workshop-core/src/ids.rs:5-9,85-87; crates/nyon-workshop-core/src/model.rs:60-71 |
| Names and escaping | Q, quote, backslash are semantic hex 51 22 5c. Their complete quoted JSON value is hex 22 51 5c 22 5c 5c 22, seven bytes. Both escaped characters are printable valid name bytes. The proposal's faction literal decodes to exactly those three semantic bytes. No newline or Unicode normalization is involved. | crates/nyon-workshop-core/src/ids.rs:140-151,285-291; crates/nyon-workshop-core/src/model.rs:38-43,187-188 |
| Coordinates and references | Coordinate bound is 131072×1024=134217728 inclusive. Point order is x,y with signed decimal values. ObjectRef uses kind then id, snake_case variant spellings, numeric local IDs and lowercase existing IDs. | crates/nyon-workshop-core/src/ids.rs:3,191-255 |
| Inventory | a precedes its extensions; ASCII '-'=45 < '0'=48 < '_'=95 yields a,a-,a0,a_. Present zero is emitted, not omitted. The source subtraction inserts the resulting zero value. | crates/nyon-workshop-core/src/ids.rs:89-91,277-283; crates/nyon-workshop-core/src/model.rs:70; crates/nyon-workshop-core/src/simulation.rs:522-535 |
| State shape | Both ordinary state serialization and the separately declared digest view use tick,factions,systems,stars,worlds,lanes,deposits,industries,routes,shipments,hazards,owners. Empty defaults give tick0 plus eleven empty maps. | crates/nyon-workshop-core/src/model.rs:129-163,187-238 |
| Faction, system, star | Ordered fields are name/color_rgb; name/position; system/name/archetype_id. RGB is exactly three u8 values, so [0,127,255] is representable. | crates/nyon-workshop-core/src/model.rs:38-58 |
| World, lane, deposit | Ordered fields are system/primary/name/archetype_id/orbit_radius_milli_au/orbit_period_ticks/phase_millidegrees/inventory; a/b/distance_units; world/resource_id/reserve_units. Literal numeric widths agree. | crates/nyon-workshop-core/src/model.rs:60-87 |
| Industry, route | Ordered fields are world/definition_id/linked_deposit/enabled; source/destination/resource_id/batch_units. Present ID and null, and true/false industry values are all represented by the reviewed blocks. | crates/nyon-workshop-core/src/model.rs:89-105 |
| Shipment, hazard | Ordered fields are route/source/destination/resource_id/units/dispatched_tick/arrival_tick; lane/hazard_id/start_tick/duration_ticks/cancelled. Tick/u64 fields and true/false hazard values agree. | crates/nyon-workshop-core/src/model.rs:107-127 |
| Pack root and definitions | Root order is kind,format_version,pack_id,pack_version,title,star_archetypes,world_archetypes,resources,industry_definitions,hazard_definitions. Simple definitions retain id,name,description; quantities retain resource_id,quantity; all three tagged industry variants and hazard fields match declaration order. | crates/nyon-workshop-core/src/pack.rs:24-57,144-161,190-203 |
| Pack ordering and values | Core resources become alloy,energy,ore; industries become extractor,foundry,solar-array; foundry inputs are energy,ore. All six quantities, names, descriptions, duration fields and hazard 1/2 ratio match the input. Empty pack validation success remains a source inference: no minimum collection cardinality exists, but no runtime admission ran. | assets/workshop/core-pack-v1.json:1-59; crates/nyon-workshop-core/src/pack.rs:128-139,322-445 |

All ten entity maps and the owners map in the raw full-state case have only one entry each. Their order is consequently not a multi-key sorting witness. Inventory does have four distinct ordered keys. The proposed second entity key, reverse/permuted insertions and owners-map ordering are still missing as executable concrete fixtures. [task-4a-independent-fixture-proposal.md:28,58,73]

The six independent squared-distance inequalities give 0,1,2,5,7 and 379625062. For the extreme corners, 144115187698503844 ≤ 144115188075855872 < 144115188457753969. Legal coordinates cannot overflow the cited i128 calculation because the largest absolute difference is 268435456. This arithmetic does not execute the production distance routine. [crates/nyon-workshop-core/src/ids.rs:3,195-201,234-247,297-312]

## Byte and hash results

All four complete preimages equal the independently reconstructed payload plus the specified ASCII domain and one terminal NUL. State domain length is 23 bytes; pack domain length is 22 bytes. All payloads are ASCII, hence exact UTF-8, with no BOM, newline, length prefix or additional bytes. These domain/input rules follow the hash update calls and the accepted pack contract. [crates/nyon-workshop-core/src/model.rs:218-238; crates/nyon-workshop-core/src/pack.rs:335-342; docs/superpowers/specs/2026-09-02-nyon-v2-design.md:388]

| Case | Payload / preimage bytes | Recalculated SHA-256 |
| --- | --- | --- |
| STATE_EMPTY | 153 / 176 | ce4b1e272e8d13891c4f8ef175c79bb46ae8fb5ebcd23241701afab7ec654f0d |
| STATE_ALL_RECORDS_RAW | 1970 / 1993 | f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5 |
| PACK_EMPTY | 199 / 221 | e5275e76792449c78553244bf536b6fa579140f19a7b7b9c2a5c24bf49239437 |
| PACK_CORE_MANUAL | 1490 / 1512 | 11e68b4a4b290f6bf14126f8ccf3378e1ec332962c697d6f958e76fed3ef4842 |

The four payloads, full input-hex blocks, byte counts and hashes all agree with the proposal. INDUSTRY_NULL_FALSE is 112 bytes and HAZARD_NOT_CANCELLED is 120 bytes; those two record-only literals also match the independent transcription. They have no separately asserted hash in the proposal. [task-4a-independent-fixture-proposal.md:51-88,155-160,166-275; verification command A below]

## Admission and coverage boundaries

The raw full-state literal is **not admissible authority**. Its self-lane, self-route, excessive phase, excessive reserve/batch, catalog-missing inventory resources and overlong hazard are deliberate schema stress. All ten IDs in this particular literal are distinct and all its entity references are present; cross-kind duplicate and dangling-reference examples are additional proposed cases, not coverage supplied by this literal. Hashing these bytes cannot establish admission or replayability, and the model encoder does not call a whole-state validator. [task-4a-independent-fixture-proposal.md:55-61,102; crates/nyon-workshop-core/src/model.rs:129-144,165-238; crates/nyon-workshop-core/src/command.rs:420-424,442-476,540-581,616-650,703-717]

Hazards128 correctly means **scheduled or active**, not total historical storage. At tick10 the four proposed tuples count 0,1,1,0. Future uncancelled hazards count; cancelled and end≤tick hazards do not. The stored-record end is checked before cancellation, so overflow in an already present raw hazard causes Arithmetic during the count; by contrast, a newly requested ScheduleHazard with overflowing start+duration is rejected as InvalidValue. Preserve that consumer distinction when authoring exact error fixtures. No discrepancy is assigned because proposal line105 explicitly describes the count predicate. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:226,235; crates/nyon-workshop-core/src/command.rs:623-637,755-768; task-4a-independent-fixture-proposal.md:105]

The strict parser's source order supports the proposed BOM, invalid UTF-8, over-size, decoded duplicate-key, float, trailing-value and depth boundaries. These are source-derived expectations, not runtime evidence. Raw-state serde behavior is a separate consumer boundary and cannot inherit strict pack guarantees. [crates/nyon-workshop-core/src/pack.rs:316-321,503-542,610-665; crates/nyon-workshop-core/src/model.rs:129-144; task-4a-independent-fixture-proposal.md:93-105]

**Pending before full corpus freeze:**

- Fully specified valid nonempty graph covering all ten kinds and owners, with a clear admission/replay construction; cross-kind duplicate and dangling references as separately named raw invalid cases.
- Concrete multi-key entity/owners order permutations, inventory absent versus zero, per-field equality changes and canonical dynamic-view equivalence.
- Full five-type fixed-ID scalar corpus, all bounded string/coordinate lengths and character classes, named consumer errors, escaped-input and negative-zero characterization, and nominal-type rejection witnesses.
- Concrete strict-pack complete inputs for size/depth/cardinality/quantity/duration boundaries, duplicates including escape-equivalent keys, unknown fields, trailing values, BOM/UTF-8, recipe and catalog reference failures, and canonical input permutations.
- Distinct-ID 129-stored/128-scheduled-or-active historical-hazard fixture and later atomic capacity/overflow witnesses.
- Later-owner revision/root/fork/entity/shipment namespace preimages and expected outputs, including parent/cursor none/some and first16 truncation. Existing source goldens alone do not close their provenance.
- Immutable fixture IDs/files/checksums and source-compatibility supplement acceptance; allocator-failure, deep clone/input-lifetime, Debug/ReleaseSafe and Rust differential/runtime evidence.

These are the foundation proposal's minimum groups and the fixture proposal's declared omissions. They are not newly discovered errors in the reviewed literals. [task-4a-foundation-vector-seams.md:125-137,159-170; task-4a-independent-fixture-proposal.md:30,73,91,105,129-131; docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:134-158]

**Freeze decision:** after the remaining case coverage and its review, the exact six literals and four hash records reviewed here can be included unchanged in a fixture manifest, bound to the current proposal/source fingerprints. This report does not itself create or authorize an immutable corpus freeze. Original Rust goldens retain their separate historical provenance classification. All implementation, admission, archive replay and runtime claims remain Pending.

## Reproducible verification

All commands below ran from /Users/donaldfilimon/dev/active/NYON. Command A exited **0** and reported six matching block literals, eleven matching model field-order lists, four matching full preimages/digests, exact escape/width/order values, core-pack semantic equality and six geometry witnesses. Command B exited **0** and matched the proposal, foundation and all ten directly named source fingerprints. A preliminary literal-only hash/hex check also exited 0 with the same four lengths and digests; Command A supersedes it by adding reconstruction.

Source reads used bounded cat/nl/sed output. The nested-guide search "rg --files --hidden .superpowers -g AGENTS.md -g CLAUDE.md" returned exit1 with no matches; that was a file-search result, not a gate failure. An initial JavaScript tool-wrapper quoting error occurred before its shell command started; the corrected Command A is the executed evidence. No test gate was run in this review-only scope.

### Command A: independent reconstruction, byte comparison and arithmetic

```sh
python3 - <<'PY'
from pathlib import Path
import hashlib, json, re
base=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
proposal=(base/'task-4a-independent-fixture-proposal.md').read_bytes()
fence=bytes([96])*3
def block(n):
    return proposal.split(fence+n.encode()+b'\n')[1].split(b'\n'+fence)[0]
# Explicit record fragments transcribed against model.rs declaration order.
# Values are the chosen raw-case parameters; no JSON serializer is used.
records=[
 b'"factions":{"00000000000000000000000000000001":{"name":"Q\\"\\\\","color_rgb":[0,127,255]}}',
 b'"systems":{"00000000000000000000000000000002":{"name":"S","position":{"x":-134217728,"y":134217728}}}',
 b'"stars":{"00000000000000000000000000000003":{"system":"00000000000000000000000000000002","name":"T","archetype_id":"yellow-dwarf"}}',
 b'"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":4294967295,"orbit_period_ticks":18446744073709551615,"phase_millidegrees":4294967295,"inventory":{"a":0,"a-":1,"a0":9007199254740993,"a_":18446744073709551615}}}',
 b'"lanes":{"00000000000000000000000000000005":{"a":"00000000000000000000000000000002","b":"00000000000000000000000000000002","distance_units":18446744073709551615}}',
 b'"deposits":{"00000000000000000000000000000006":{"world":"00000000000000000000000000000004","resource_id":"ore","reserve_units":18446744073709551615}}',
 b'"industries":{"00000000000000000000000000000007":{"world":"00000000000000000000000000000004","definition_id":"extractor","linked_deposit":"00000000000000000000000000000006","enabled":true}}',
 b'"routes":{"00000000000000000000000000000008":{"source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","batch_units":18446744073709551615}}',
 b'"shipments":{"00000000000000000000000000000009":{"route":"00000000000000000000000000000008","source":"00000000000000000000000000000004","destination":"00000000000000000000000000000004","resource_id":"ore","units":18446744073709551615,"dispatched_tick":0,"arrival_tick":18446744073709551615}}',
 b'"hazards":{"0000000000000000000000000000000a":{"lane":"00000000000000000000000000000005","hazard_id":"ion-storm","start_tick":0,"duration_ticks":18446744073709551615,"cancelled":true}}',
 b'"owners":{"00000000000000000000000000000004":"00000000000000000000000000000001"}'
]
expected={
 'STATE_EMPTY': b'{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}',
 'STATE_ALL_RECORDS_RAW': b'{"tick":18446744073709551615,'+b','.join(records)+b'}',
 'INDUSTRY_NULL_FALSE': b'{"world":"00000000000000000000000000000004","definition_id":"solar-array","linked_deposit":null,"enabled":false}',
 'HAZARD_NOT_CANCELLED': b'{"lane":"00000000000000000000000000000005","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}',
 'PACK_EMPTY': b'{"kind":"NYON_WORKSHOP_DATA","format_version":1,"pack_id":"a","pack_version":1,"title":"A","star_archetypes":[],"world_archetypes":[],"resources":[],"industry_definitions":[],"hazard_definitions":[]}',
 'PACK_CORE_MANUAL': (
 b'{"kind":"NYON_WORKSHOP_DATA","format_version":1,"pack_id":"nyon.core","pack_version":1,"title":"NYON Core",'
 b'"star_archetypes":[{"id":"yellow-dwarf","name":"Yellow Dwarf","description":"A stable main-sequence star."}],'
 b'"world_archetypes":[{"id":"rocky-world","name":"Rocky World","description":"A solid world suitable for industry."}],'
 b'"resources":[{"id":"alloy","name":"Alloy","description":"Refined structural material."},{"id":"energy","name":"Energy","description":"Electrical energy for industry."},{"id":"ore","name":"Ore","description":"Unrefined mineral feedstock."}],'
 b'"industry_definitions":[{"kind":"extractor","id":"extractor","name":"Extractor","description":"Uses energy to extract linked ore reserves.","duration_ticks":1,"energy_input":{"resource_id":"energy","quantity":1},"output":{"resource_id":"ore","quantity":2}},'
 b'{"kind":"processor","id":"foundry","name":"Foundry","description":"Converts energy and ore into alloy.","duration_ticks":1,"inputs":[{"resource_id":"energy","quantity":2},{"resource_id":"ore","quantity":3}],"outputs":[{"resource_id":"alloy","quantity":1}]},'
 b'{"kind":"solar","id":"solar-array","name":"Solar Array","description":"Produces four energy units per tick.","duration_ticks":1,"outputs":[{"resource_id":"energy","quantity":4}]}],'
 b'"hazard_definitions":[{"id":"ion-storm","name":"Ion Storm","description":"Halves route capacity on its lane using integer floor division.","duration_ticks":36000,"route_capacity_numerator":1,"route_capacity_denominator":2}]}')
}
model=Path('crates/nyon-workshop-core/src/model.rs').read_text()
raw=json.loads(expected['STATE_ALL_RECORDS_RAW'])
for name,collection in [('FactionV1','factions'),('SystemV1','systems'),('StarV1','stars'),('WorldV1','worlds'),('LaneV1','lanes'),('DepositV1','deposits'),('IndustryV1','industries'),('RouteV1','routes'),('ShipmentV1','shipments'),('HazardV1','hazards'),('WorkshopStateV1',None)]:
    body=re.search(r'pub struct '+name+r' \{(.*?)\n\}',model,re.S).group(1)
    fields=re.findall(r'pub (\w+):',body)
    value=next(iter(raw[collection].values())) if collection else raw
    assert fields==list(value),(name,fields,list(value))
for n,b in expected.items():
    assert b==block(n),n
    assert b.isascii() and not b.endswith(b'\n')
    if n.startswith(('STATE_','PACK_')):
        domain=b'NYON-WORKSHOP-'+(b'STATE' if n.startswith('STATE_') else b'PACK')+b'-V1'+bytes([0])
        section=proposal.split(b'### Exact '+n.encode()+b' SHA-256 input hex\n')[1]
        preimage=bytes.fromhex(section.split(fence+b'text\n')[1].split(b'\n'+fence)[0].decode())
        assert preimage==domain+b
        print(n,len(b),len(domain),len(preimage),hashlib.sha256(domain+b).hexdigest())
    else:
        print(n,len(b),'MATCH')
assert raw['factions']['0'*31+'1']['name'].encode().hex()=='51225c'
assert b'"Q\\"\\\\"'.hex()=='22515c225c5c22'
assert list(raw['worlds']['0'*31+'4']['inventory'])==sorted(['a_','a0','a-','a'])
assert (2**64-1,2**32-1,2**16-1,2**53+1)==(18446744073709551615,4294967295,65535,9007199254740993)
asset=json.loads(Path('assets/workshop/core-pack-v1.json').read_bytes())
# Decode for semantic comparison only, never encode to obtain expected bytes.
for key in ['star_archetypes','world_archetypes','resources','industry_definitions','hazard_definitions']:
    asset[key].sort(key=lambda x:x['id'])
for industry in asset['industry_definitions']:
    for key in ['inputs','outputs']:
        if key in industry:
            industry[key].sort(key=lambda x:x['resource_id'])
assert asset==json.loads(expected['PACK_CORE_MANUAL'])
assert bytes.fromhex('00010000000000000000000000000000') < bytes.fromhex('01000000000000000000000000000000')
assert int.from_bytes(bytes.fromhex('00010000000000000000000000000000'),'little') > int.from_bytes(bytes.fromhex('01000000000000000000000000000000'),'little')
for dx,dy,r in [(0,0,0),(1,1,1),(2,2,2),(3,4,5),(5,5,7),(268435456,268435456,379625062)]:
    square=dx*dx+dy*dy
    assert r*r<=square<(r+1)*(r+1)
    print('DISTANCE',dx,dy,r,r*r,square,(r+1)*(r+1))
print('PASS: 6 literal blocks, 11 model field-order lists, name bytes, widths, inventory ordering, ID endian witness, core-pack semantic values, 6 geometry witnesses')
PY
```

### Command B: source bindings

```sh
python3 - <<'PY'
from pathlib import Path
import hashlib,re
base=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
for name,want in [('task-4a-independent-fixture-proposal.md','f3a2d982171840443c334cc44c7a79493622c752627874cd82e58b86e4654027'),('task-4a-foundation-vector-seams.md','5ee6c1cadf997c3e730e53a2e08837718b6e5de8e8e9a46b643a89be3d05c954')]:
    actual=hashlib.sha256((base/name).read_bytes()).hexdigest()
    assert actual==want,(name,actual)
    print(name,actual)
text=(base/'task-4a-independent-fixture-proposal.md').read_text()
rows=re.findall(r'^\| [^|\n]*?'+chr(96)+r'([^'+chr(96)+r']+)'+chr(96)+r' \| '+chr(96)+r'([0-9a-f]{64})'+chr(96)+r' \|$',text,re.M)
assert len(rows)==10,len(rows)
for name,want in rows:
    actual=hashlib.sha256(Path(name).read_bytes()).hexdigest()
    assert actual==want,(name,actual)
    print(name,actual)
print('PASS: proposal, foundation, all 10 directly named source fingerprints')
PY
```

Gaps: full corpus coverage, immutable fixture freeze, implementation tests and every runtime claim remain Pending. Review content is frozen by the file SHA-256 returned with the completion message; that checksum is external to this file to avoid a self-referential hash.
