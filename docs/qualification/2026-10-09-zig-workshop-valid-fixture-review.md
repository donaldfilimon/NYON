# Task 4A valid-graph and consumer-boundary fixture derivation review

2026-10-09. **Verdict: APPROVED for the reviewed subset only.** Critical: 0. Important: 0. Minor: 0. The literal graph construction, seven state payloads/preimages/hashes, namespace calculations, order comparisons and finite scalar cases have no identified discrepancy. This is independent review of source-derived candidates. Actual command admission, receipts, replay, atomic rejection, Zig implementation, immutable corpus freeze and the complete Workshop corpus remain Pending. [task-4a-valid-fixture-proposal.md:3,29,293,337,339]

Reviewed input: .superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-valid-fixture-proposal.md, 46672 bytes, SHA-256 9bdb9290fe84f046a13cc485e9b50435ffc462292023511ab2bcb3a0c7c7b920. Review-input manifest SHA-256 e4e18a90285db9d4c43551f8510d0cf4281b1265d107c8ed727940768ba2267b. Its nine bound payload files match both byte lengths and hashes. All twelve source bindings, including the earlier proposal and approved subset review, were rehashed without drift. [task-4a-valid-fixture-evidence/review-input-manifest.json:4,25; task-4a-valid-fixture-evidence/source-bindings-before.txt:1; identity verification command C below]

Only this new review file was written. No production code, existing document, fixture, author evidence, profile, dependency, build, test, gate, branch, commit or remote action was changed or executed. Standard-library SHA-256, integer arithmetic, regular expressions, JSON decoding for inspection and explicit literal-fragment comparison were used. The author's three derivation scripts were read but were not executed or imported by this reviewer. Commands A and B below independently repeat the derivations; none calls a NYON serializer, generator, authority or executable.

## Authority and independence

The accepted design establishes integer authority, checked geometry, ordered typed collections, local-reference rules, atomic rejection, capacities, identity composition and simulation phase order. It names the pack hash domain but does not enumerate all canonical state field/domain details or every namespace byte width. The exact record declarations, state digest view and hash-update calls remain explicitly identified source compatibility supplements. Review agreement is therefore independent of either runtime implementation's output, while still derived from the current Rust source contract where the accepted document is less specific. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:176,203,213,316,320,326,337,388; crates/nyon-workshop-core/src/model.rs:38,129,195,218; crates/nyon-workshop-core/src/command.rs:850,877]

The Library addendum leaves Workshop V1 formats and authority intact and requires exact-catalog validation/replay before client session replacement. A state payload's representability or hash cannot stand in for that installation path. [docs/superpowers/specs/2026-09-04-nyon-workshop-library-addendum.md:8,14; crates/nyon-workshop-core/src/model.rs:165,187]

The previous six-literal subset and its review were read and their bindings remain unchanged. Their declared omissions motivated this finite addition; they did not authorize a full corpus freeze. Matching their canonical core-pack hash does not establish the older Rust golden's original provenance. [task-4a-independent-fixture-proposal.md:9,11,85,129; task-4a-independent-fixture-review.md:15,59,71; task-4a-foundation-vector-seams.md:125,162]

## Construction and transition review

| Surface | Independently checked result | Evidence |
| --- | --- | --- |
| Creator batch | Exact 2030 bytes; seventeen operations; thirteen unique created local IDs; all reference fields point to compatible earlier creates. Tagged variant fields follow command declaration order. Four SetOwner targets are systems/worlds, which are in the eight ownable kinds. | task-4a-valid-fixture-proposal.md:20; crates/nyon-workshop-core/src/command.rs:25,33,489,664,720; Commands A/B |
| Admission preconditions | Distinct system positions; matching star/system primaries; radius and period positive; phases below360000; lane distance5; reserve10 and batch3 within1..1e9; source/destination distinct with a direct lane; extractor linked to the source world's ore deposit; solar linked deposit null; hazard start10/duration2 valid against maximum36000. These are source-derived expected conditions, not executed admission. | crates/nyon-workshop-core/src/command.rs:361,385,411,442,464,498,540,616; assets/workshop/core-pack-v1.json:8,11,19,27,49; task-4a-valid-fixture-proposal.md:23 |
| Revision namespace | Source-order preimage is2149 bytes. Rules u32LE1, catalog raw32, zero seed32, parent-none00, tick0/ordinal0 u64LE, batch-length u64LE ee07000000000000, exact batch. Full SHA matches 4bd91874ea1ae84d0dae94674a5ccfbb365699722b6efe73a39b5427abdba375. | crates/nyon-workshop-core/src/command.rs:850; task-4a-valid-fixture-proposal.md:27,31; Command A |
| Entity namespaces | All thirteen complete preimages and full32 hashes match the identity log. Kind discriminators agree with enum order; local IDs use u16LE; IDs are first16 bytes. They are distinct. | crates/nyon-workshop-core/src/model.rs:22; crates/nyon-workshop-core/src/command.rs:877; task-4a-valid-fixture-proposal.md:35; Command A |
| Shipment namespaces | Both route/raw16 plus dispatch-tick/u64LE preimages and full32 hashes match; first16 values are 328c08df01608b3877b8429f1034aec0 and f38be69ca71d2f805143fb6f503edfa3. Neither collides with the thirteen created IDs. | crates/nyon-workshop-core/src/simulation.rs:537; task-4a-valid-fixture-proposal.md:49; Command A |
| First step, current tick0 | No hazard boundary or active ratio. Solar precedes extractor by typed ID: energy0+4−1=3, ore0+2=2, reserve10−2=8. Route dispatches3 over distance5, travel1, retaining energy0; new shipment dispatch0/arrival1; final state tick1. | crates/nyon-workshop-core/src/simulation.rs:86,240,278,308,356,464,522; assets/workshop/core-pack-v1.json:19,27; task-4a-valid-fixture-proposal.md:53 |
| Second step, current tick1 | Delivery first adds3 energy to World B and removes the earlier shipment. Solar/extractor leave World A energy3/ore4 and reserve6. Route dispatches3, retaining energy0, with dispatch1/arrival2; final state tick2. Static maps and owners remain unchanged. | crates/nyon-workshop-core/src/simulation.rs:86,175,240,308,356,128; task-4a-valid-fixture-proposal.md:55 |
| All-kind graph | The final literal has fourteen distinct resident entities, all ten kinds and four owners. Thirteen creator entities plus one in-flight shipment explain the count. Replay construction is coherent: recorded revision at tick0/ordinal0 with post-batch digest, then materialize to tick2. Actual receipt/replay comparison is Pending. | task-4a-valid-fixture-proposal.md:29,62,66,70; crates/nyon-workshop-core/src/history.rs:434,495,504,510 |

The independent expected state bytes below were rebuilt using separately typed fixed record fragments in declaration order and the recalculated identities. JSON decoding inspected keys/values but did not produce expected bytes. The two tick calculations were traced against the source phases and committed recipe values; no authority or step function ran. [crates/nyon-workshop-core/src/model.rs:38,129; crates/nyon-workshop-core/src/simulation.rs:73,240,308,356; Command A]

## Canonical payloads and comparisons

All seven payloads, complete domain preimage hex strings, payload/preimage lengths and SHA-256 values match the frozen author log and proposal table. The prefix is exactly ASCII NYON-WORKSHOP-STATE-V1 plus terminal NUL, 23 bytes. There is no newline, BOM, separate length or additional terminator. Eleven declaration field lists were checked across the seven states, with bytewise entity/owner order and ASCII inventory order. [crates/nyon-workshop-core/src/model.rs:38,129,195,218; task-4a-valid-fixture-proposal.md:13,355; Command A]

| Case | Payload / preimage bytes | Recalculated SHA-256 |
| --- | --- | --- |
| STATE_VALID_POST_BATCH |2329 /2352 |be16ba67a02e59b5d235863175066a7ffc1dc9d2a3348536467c7b9cf1e3f9b3 |
| STATE_VALID_AFTER_TICK_0 |2588 /2611 |e8e1f2ac8b0e08f81a3183ce5e86fdcf8c2e995f22dfa978d5a0aaa6f9260c6f |
| STATE_VALID_AFTER_TICK_1 |2598 /2621 |d10142cc43ede9c2e94d26ba489f888e57ce878a5e46fc9f4959fd3fe4ef6465 |
| STATE_RAW_DANGLING |474 /497 |4bd5d8f19ea678f5107ea0b013bbcac90f11cda485e11cf6c6e97b1172242746 |
| STATE_RAW_CROSS_KIND_DUPLICATE |292 /315 |380f940f29986675ca18d5602607b00263b5131d7fdf73c99605706be6e836a3 |
| STATE_RAW_ENERGY_ABSENT |2587 /2610 |420ffc864553d8d6a8b9c0c50e9e938f82618b36ebe8a173b728bbed0983d885 |
| STATE_RAW_129_STORED_128_COUNTED |22298 /22321 |92bf6f358ea671639c5c5a726fbb2a03bc183cf5373fe3aeba21dc941c64bc82 |

Dynamic-view comparison is concrete: substituting tick/worlds/deposits/shipments from the final literal into the post-batch literal produces exactly the final expected bytes; all four overrides differ. The eight remaining maps stay from the static receiver, matching CanonicalStateRef. This is byte equivalence under the proposed consumer contract, not an executed implementation call or lifetime witness. [task-4a-valid-fixture-proposal.md:75; crates/nyon-workshop-core/src/model.rs:195; Command A]

All fifteen insertion lists for five multi-key collections are proper permutations of the final literal's keys. The reverse/mixed/sorted cases and inventory permutations consequently have the stated common bytewise order under the BTreeMap contract. The separate two-key faction/owners literals expose the reversed little-endian integer interpretation. This finite subset does not claim multiple resident keys in every entity collection. [task-4a-valid-fixture-proposal.md:77,81,85,88,92; crates/nyon-workshop-core/src/ids.rs:14,17; crates/nyon-workshop-core/src/model.rs:129; Command B]

The exact World A inventory fragments energy0/ore4 and ore4 are structurally and bytewise distinct. Their two complete state hashes differ as stated. Lookup's numeric default0 does not permit an encoder or equality implementation to erase a resident zero; subtract_inventory inserts the checked zero result. [task-4a-valid-fixture-proposal.md:95,98,102,105; crates/nyon-workshop-core/src/simulation.rs:402,522; Commands A/B]

## Invalid receivers and hazard precedence

STATE_RAW_DANGLING and STATE_RAW_CROSS_KIND_DUPLICATE were independently transcribed and compared byte for byte. The model can represent/hash both, with no standalone whole-state validation in canonical_bytes/digest. The dangling world is resident while its system/primary/owner faction are absent. Existing system2 resolution returns UnknownEntity. The duplicate cross-kind id1 resolves first as Faction, so an existing-system reference returns WrongEntityKind despite its systems membership. Neither is an IdentityCollision witness: allocation checks the newly derived candidate ID. [task-4a-valid-fixture-proposal.md:109,112,115,118,122,126; crates/nyon-workshop-core/src/model.rs:165,187; crates/nyon-workshop-core/src/command.rs:703,720]

The two named raw batches correctly target the internal apply_recorded_batch seam. Public WorkshopAuthority::new creates empty state; its state accessor is immutable. from_parts is crate-private, so this is not a new public raw installation route. CursorNone/tick0 and small byte/operation counts allow the source resolution error to be reached in that proposed seam. Atomic rejection and actual consumer invocation remain Pending. [task-4a-valid-fixture-proposal.md:129; crates/nyon-workshop-core/src/command.rs:170,199,241,263,275,385]

The displayed hazard list contains exactly128 distinct16-byte keys from raw value0x1000 through0x107f in byte order, plus historical key0x1080. They are separate from the valid graph's thirteen creator IDs. Exact replacement of the post-batch hazard fragment and tick produces the independently checked22298-byte raw state. Its128 current records have checked end12>tick10; the historical record has end10==tick10. Thus scheduled_or_active_hazard_count is128 while raw storage length, and entity_count_for_kind(Hazard), are129. Historical entries remain stored. [task-4a-valid-fixture-proposal.md:133,136,274; crates/nyon-workshop-core/src/command.rs:755; crates/nyon-workshop-core/src/simulation.rs:548; Command A]

Cancelling the first current record reduces the count to127. An uncancelled future hazard counts; a cancelled one does not. Existing-record end overflow returns CreatorRejectionV1::Arithmetic before cancellation is tested. ScheduleHazard calls this count and capacity guard before lane/catalog/new-value checks; with fewer than128 qualifying records, a new overflowing start+duration reaches InvalidValue, while128 qualifying records cause Capacity first. hazard_boundaries skips a cancelled record early, but active_hazard_ratios checks the end before cancellation, so that shortcut cannot establish successful full-step behavior. [task-4a-valid-fixture-proposal.md:278,279,280,281,282,283,284,285,287,290; crates/nyon-workshop-core/src/command.rs:623,632,743,755; crates/nyon-workshop-core/src/simulation.rs:151,464]

The final overflow precedence row explicitly requires128 qualifying records at the receiver's own tick. Merely retiming the displayed tick10 population to2^64−2 would make its short hazards historical and would not produce Capacity. For the stated predicate row,128 records starting2^64−2 with duration1 have representable end2^64−1 and qualify. This is source arithmetic clarification for future exact consumer fixtures; no historical authority population or runtime result is inferred. [task-4a-valid-fixture-proposal.md:284,285,293; crates/nyon-workshop-core/src/command.rs:632,755]

## Scalars and remaining boundaries

The finite scalar choices agree with the exact byte validators:39 string cases, nine coordinate values, three point-constructor cases, and45 decoded fixed-ID cases across five nominal consumers. Boundary lengths, allowed printable bytes, names' spacing rules, catalog-versus-pack dot distinction, decoded non-ASCII rejection, zero/FF/nibble valid controls and exact decoded ID length/hex classes are correct. No trimming or normalization is introduced. The255/256/257 distance choices give travel ticks1/1/2 by the accepted ceiling formula. [task-4a-valid-fixture-proposal.md:297,301,303,305,307,309,311,316,320,331; crates/nyon-workshop-core/src/ids.rs:43,94,145,195,227,234,269,277,285,293; crates/nyon-workshop-core/src/simulation.rs:375; Command B]

Identical valid wire strings for same-width nominal wrappers are intentionally accepted by each decoded validator. Cross-type rejection remains a future Zig compile-time/API property. Constructor errors and decoded custom invalid canonical identifier errors are correctly separated from unknown aggregate-deserializer prefixes/locations. These calculations do not characterize escaped JSON lexemes, negative zero, quoted numbers, numeric-overflow mapping, duplicate raw keys or aggregate unknown fields. [task-4a-valid-fixture-proposal.md:316,329; crates/nyon-workshop-core/src/ids.rs:11,26,71,209; task-4a-foundation-vector-seams.md:51,54,168]

**Residual coverage is not a finding against this finite subset.** Before full corpus freeze: complete per-field equality mutations; nominal compile failures; clone/allocator/caller-input lifetime witnesses; full raw ingress characterization; strict complete pack size/depth/cardinality/recipe/reference cases; parent-some/root/fork/cursor/all-kind/local-bound namespace families; immutable fixture IDs/files/provenance checksums; and actual command/tick/receipt/replay comparison with atomic preservation remain Pending. The129-resident population has raw count-consumer provenance only; a command-admitted historical population requires later bounded batches/ticks and authenticated receipts/replay. [task-4a-valid-fixture-proposal.md:293,337,339; task-4a-foundation-vector-seams.md:125; task-4a-independent-fixture-review.md:59]

## Executed review evidence

All commands ran in /Users/donaldfilimon/dev/active/NYON. Command A exited0 and printed all seven matching state rows, matching revision, and its reconstruction PASS. Command B exited0 and printed matching finite scalar/order/reference arithmetic. A preliminary B run exited0 before its point checks were expanded; the final B below includes those three explicit checks and labels the thirteen created locals precisely. Command C's identity portion exited0 with proposal, manifest, nine payload and twelve source bindings matching. The separate nested-guide search returned1 with no matching .superpowers AGENTS.md/CLAUDE.md; this is a file-search result, not a product gate.

The full commands below are review arithmetic/literal checks. They do not run the author's scripts, serialize a decoded JSON object, invoke a production generator, execute command admission, replay an archive, run implementation tests or approve a corpus freeze.

### Command A: independent namespace/state reconstruction

~~~python
from pathlib import Path
import hashlib,json,re
base=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
proposal=(base/'task-4a-valid-fixture-proposal.md').read_bytes()
fence=bytes([96])*3
def literal(name):
    token=fence+name.encode()+b'\n'
    assert proposal.count(token)==1
    return proposal.split(token)[1].split(b'\n'+fence)[0]
batch=literal('BATCH_VALID_GRAPH')
ops=json.loads(batch)['operations']
assert len(batch)==2030 and len(ops)==17
# Source declaration order, including serde's leading variant tag.
source=Path('crates/nyon-workshop-core/src/command.rs').read_text()
variants={'create_faction':'CreateFaction','create_system':'CreateSystem','create_star':'CreateStar','create_world':'CreateWorld','connect_lane':'ConnectLane','create_deposit':'CreateDeposit','place_industry':'PlaceIndustry','connect_route':'ConnectRoute','schedule_hazard':'ScheduleHazard','set_owner':'SetOwner'}
for op in ops:
    body=re.search(r'    '+variants[op['kind']]+r' \{(.*?)\n    \}',source,re.S).group(1)
    assert list(op)==['kind']+re.findall(r'^        (\w+):',body,re.M)
assert re.search(r'WORKSHOP_RULES_VERSION: u32 = 1',Path('crates/nyon-workshop-core/src/lib.rs').read_text())
old=(base/'task-4a-independent-fixture-proposal.md').read_bytes()
pack=old.split(fence+b'PACK_CORE_MANUAL\n')[1].split(b'\n'+fence)[0]
catalog=hashlib.sha256(b'NYON-WORKSHOP-PACK-V1\0'+pack).digest()
assert catalog.hex()=='11e68b4a4b290f6bf14126f8ccf3378e1ec332962c697d6f958e76fed3ef4842'
revision_input=(b'NYON-WORKSHOP-REVISION-V1\0'+bytes.fromhex('01000000')+catalog+bytes(32)+bytes([0])+bytes(16)+len(batch).to_bytes(8,'little')+batch)
revision=hashlib.sha256(revision_input).digest()
log=(base/'task-4a-valid-fixture-evidence/derive-identities.log').read_text().splitlines()
assert 'REVISION_PREIMAGE_HEX '+revision_input.hex() in log
assert 'REVISION '+revision.hex() in log
assert len(revision_input)==2149 and len(batch).to_bytes(8,'little').hex()=='ee07000000000000'
# The fixed chosen local/kind table is independently transcribed from the batch
# and EntityKind declaration. This is identity arithmetic, no authority execution.
chosen=[('FACTION',1,0),('SYSTEM_A',2,1),('SYSTEM_B',3,1),('STAR_A',4,2),('STAR_B',5,2),('WORLD_A',6,3),('WORLD_B',7,3),('LANE',8,4),('DEPOSIT',9,5),('SOLAR',10,6),('ROUTE',11,7),('HAZARD',12,9),('EXTRACTOR',13,6)]
ids={}
for label,local,kind in chosen:
    pre=b'NYON-WORKSHOP-ENTITY-V1\0'+revision+bytes([kind])+local.to_bytes(2,'little')
    full=hashlib.sha256(pre).digest(); ids[label]=full[:16].hex()
    assert ' '.join([label,str(local),str(kind),pre.hex(),full.hex(),ids[label]]) in log
assert len(set(ids.values()))==13
shipment={}
for tick in (0,1):
    pre=b'NYON-WORKSHOP-SHIPMENT-V1\0'+bytes.fromhex(ids['ROUTE'])+tick.to_bytes(8,'little')
    full=hashlib.sha256(pre).digest();shipment[tick]=full[:16].hex()
    assert ' '.join(['SHIPMENT',str(tick),pre.hex(),full.hex(),shipment[tick]]) in log
    assert shipment[tick] not in ids.values()
assert ids['SOLAR']<ids['EXTRACTOR']
assert ids['WORLD_B']<ids['WORLD_A']
# Hand-transcribed fixed record fragments in model.rs order. No JSON encoder.
F,SA,SB,TA,TB,WA,WB,L,D,I,R,H,X=[ids[k] for k in ('FACTION','SYSTEM_A','SYSTEM_B','STAR_A','STAR_B','WORLD_A','WORLD_B','LANE','DEPOSIT','SOLAR','ROUTE','HAZARD','EXTRACTOR')]
def entry(key,fragment): return '"'+key+'":'+fragment
factions=entry(F,'{"name":"Union","color_rgb":[12,34,56]}')
systems=entry(SA,'{"name":"A","position":{"x":0,"y":0}}')+','+entry(SB,'{"name":"B","position":{"x":3,"y":4}}')
stars=entry(TA,'{"system":"'+SA+'","name":"A Star","archetype_id":"yellow-dwarf"}')+','+entry(TB,'{"system":"'+SB+'","name":"B Star","archetype_id":"yellow-dwarf"}')
wb_prefix='{"system":"'+SB+'","primary":"'+TB+'","name":"B World","archetype_id":"rocky-world","orbit_radius_milli_au":2000,"orbit_period_ticks":20,"phase_millidegrees":180000,"inventory":'
wa_prefix='{"system":"'+SA+'","primary":"'+TA+'","name":"A World","archetype_id":"rocky-world","orbit_radius_milli_au":1000,"orbit_period_ticks":10,"phase_millidegrees":0,"inventory":'
lanes=entry(L,'{"a":"'+SA+'","b":"'+SB+'","distance_units":5}')
deposit_prefix='{"world":"'+WA+'","resource_id":"ore","reserve_units":'
industries=entry(I,'{"world":"'+WA+'","definition_id":"solar-array","linked_deposit":null,"enabled":true}')+','+entry(X,'{"world":"'+WA+'","definition_id":"extractor","linked_deposit":"'+D+'","enabled":true}')
routes=entry(R,'{"source":"'+WA+'","destination":"'+WB+'","resource_id":"energy","batch_units":3}')
hazard_fragment='{"lane":"'+L+'","hazard_id":"ion-storm","start_tick":10,"duration_ticks":2,"cancelled":false}'
hazards=entry(H,hazard_fragment)
owners=','.join(entry(k,'"'+F+'"') for k in (WB,SA,SB,WA))
# Values follow the manual two-tick calculation: solar +4, extractor -1/+2,
# reserve -2, route -3; tick1 first delivers the older shipment to B.
expected={}
for tick,ainv,binv,reserve in [(0,'{}','{}',10),(1,'{"energy":0,"ore":2}','{}',8),(2,'{"energy":0,"ore":4}','{"energy":3}',6)]:
    worlds=entry(WB,wb_prefix+binv+'}')+','+entry(WA,wa_prefix+ainv+'}')
    deposits=entry(D,deposit_prefix+str(reserve)+'}')
    shipments='' if tick==0 else entry(shipment[tick-1],'{"route":"'+R+'","source":"'+WA+'","destination":"'+WB+'","resource_id":"energy","units":3,"dispatched_tick":'+str(tick-1)+',"arrival_tick":'+str(tick)+'}')
    text=('{"tick":'+str(tick)+',"factions":{'+factions+'},"systems":{'+systems+'},"stars":{'+stars+'},"worlds":{'+worlds+'},"lanes":{'+lanes+'},"deposits":{'+deposits+'},"industries":{'+industries+'},"routes":{'+routes+'},"shipments":{'+shipments+'},"hazards":{'+hazards+'},"owners":{'+owners+'}}')
    n=['STATE_VALID_POST_BATCH','STATE_VALID_AFTER_TICK_0','STATE_VALID_AFTER_TICK_1'][tick]
    expected[n]=text.encode()
    assert expected[n]==literal(n),n
# Independently transcribed raw records, distinct from admitted-state candidates.
expected['STATE_RAW_DANGLING']=b'{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{"00000000000000000000000000000004":{"system":"00000000000000000000000000000002","primary":"00000000000000000000000000000003","name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":1,"orbit_period_ticks":1,"phase_millidegrees":0,"inventory":{}}},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{"00000000000000000000000000000004":"00000000000000000000000000000001"}}'
expected['STATE_RAW_CROSS_KIND_DUPLICATE']=b'{"tick":0,"factions":{"00000000000000000000000000000001":{"name":"F","color_rgb":[1,2,3]}},"systems":{"00000000000000000000000000000001":{"name":"S","position":{"x":0,"y":0}}},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}'
for n in ('STATE_RAW_DANGLING','STATE_RAW_CROSS_KIND_DUPLICATE'): assert expected[n]==literal(n)
zero=b'"inventory":{"energy":0,"ore":4}'
assert expected['STATE_VALID_AFTER_TICK_1'].count(zero)==1
expected['STATE_RAW_ENERGY_ABSENT']=expected['STATE_VALID_AFTER_TICK_1'].replace(zero,b'"inventory":{"ore":4}')
keys=literal('HAZARD_IDS_128_CURRENT').decode().splitlines()
# Range checks inspect the fully supplied literal key list; it emits no fixture.
assert len(keys)==len(set(keys))==128
assert [int(k,16) for k in keys]==list(range(4096,4224))
assert all(len(k)==32 and k==k.lower() for k in keys)
assert literal('HAZARD_CURRENT_RECORD').decode()==hazard_fragment
historical_fragment='{"lane":"'+L+'","hazard_id":"ion-storm","start_tick":0,"duration_ticks":10,"cancelled":false}'
assert literal('HAZARD_HISTORICAL_RECORD').decode()==historical_fragment
hazard_contents=','.join(entry(k,hazard_fragment) for k in keys)+','+entry('00000000000000000000000000001080',historical_fragment)
needle=('"hazards":{'+hazards+'}').encode()
assert expected['STATE_VALID_POST_BATCH'].count(needle)==1
expected['STATE_RAW_129_STORED_128_COUNTED']=expected['STATE_VALID_POST_BATCH'].replace(b'{"tick":0,',b'{"tick":10,',1).replace(needle,('"hazards":{'+hazard_contents+'}').encode(),1)
state_log=(base/'task-4a-valid-fixture-evidence/derive-state-boundaries.log').read_text().splitlines()
for n,payload in expected.items():
    pre=b'NYON-WORKSHOP-STATE-V1\0'+payload
    sha=hashlib.sha256(pre).hexdigest()
    assert 'STATE_PAYLOAD '+n+' '+payload.decode() in state_log,n
    assert 'STATE_PREIMAGE_HEX '+n+' '+pre.hex() in state_log,n
    assert 'STATE_HASH '+n+' '+str(len(payload))+' '+str(len(pre))+' '+sha in state_log,n
    row=re.search(rb'^\| '+n.encode()+rb' \|(\d+) /(\d+) \|'+bytes([96])+rb'([0-9a-f]{64})'+bytes([96])+rb' \|$',proposal,re.M)
    assert row and (int(row[1]),int(row[2]),row[3].decode())==(len(payload),len(pre),sha)
    print(n,len(payload),len(pre),sha)
model=Path('crates/nyon-workshop-core/src/model.rs').read_text()
collections=[('FactionV1','factions'),('SystemV1','systems'),('StarV1','stars'),('WorldV1','worlds'),('LaneV1','lanes'),('DepositV1','deposits'),('IndustryV1','industries'),('RouteV1','routes'),('ShipmentV1','shipments'),('HazardV1','hazards'),('WorkshopStateV1',None)]
for n,payload in expected.items():
    obj=json.loads(payload)
    for struct,collection in collections:
        fields=re.findall(r'pub (\w+):',re.search(r'pub struct '+struct+r' \{(.*?)\n\}',model,re.S).group(1))
        values=[obj] if collection is None else obj[collection].values()
        for record in values: assert list(record)==fields,(n,struct)
    for collection in list(obj)[1:]:
        assert list(obj[collection])==sorted(obj[collection]),(n,collection)
        if collection=='worlds':
            for w in obj['worlds'].values():assert list(w['inventory'])==sorted(w['inventory'])
def section(payload,start,following):
    a=payload.index(b'"'+start+b'":');z=payload.index(b',"'+following+b'":',a)
    return payload[a:z]
dynamic=expected['STATE_VALID_POST_BATCH'].replace(b'{"tick":0,',b'{"tick":2,',1)
for a,z in [(b'worlds',b'lanes'),(b'deposits',b'industries'),(b'shipments',b'hazards')]:
    before=section(dynamic,a,z);after=section(expected['STATE_VALID_AFTER_TICK_1'],a,z)
    assert before!=after;dynamic=dynamic.replace(before,after,1)
assert dynamic==expected['STATE_VALID_AFTER_TICK_1']
# Manual arithmetic has no production step implementation.
assert 5*5==3*3+4*4 and max(1,(5+255)//256)==1
assert (0+4-1-3,0+2,10-2)==(0,2,8)
assert (0+4-1-3,2+2,8-2,0+3)==(0,4,6,3)
current=json.loads(expected['STATE_RAW_129_STORED_128_COUNTED'])['hazards']
assert len(current)==129 and sum(not h['cancelled'] and h['start_tick']+h['duration_ticks']>10 for h in current.values())==128
assert 128-1==127 and 0+10==10 and 0+11>10 and 20+1>10
overflow=json.loads(literal('HAZARD_RAW_CANCELLED_OVERFLOW'))
assert overflow['start_tick']+overflow['duration_ticks']==2**64
assert overflow['cancelled'] is True
print('REVISION',len(batch),len(revision_input),revision.hex())
print('PASS: canonical batch order,13 entity and2 shipment preimages/full hashes/first16;7 independently rebuilt state bytes/preimages/hashes;11 declaration lists across7 states;manual two ticks;dynamic overrides;129-resident/128-count arithmetic')

~~~

### Command B: independent finite byte classes, ordering and static references

~~~python
from pathlib import Path
import json,re
base=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
proposal=(base/'task-4a-valid-fixture-proposal.md').read_bytes()
fence=bytes([96])*3
def literal(n):return proposal.split(fence+n.encode()+b'\n')[1].split(b'\n'+fence)[0]
# A separate regular-expression formulation of byte classes checks finite choices.
classes=[
('catalog',rb'[a-z][a-z0-9_-]{0,31}',[b'a',b'a0_-',b'abcdefghijklmnopqrstuvwxyz012345'],[b'',b'abcdefghijklmnopqrstuvwxyz0123456',b'0a',b'A',b'a.b',b'a b',bytes.fromhex('c3a9')]),
('pack',rb'[a-z][a-z0-9._-]{0,47}',[b'a.b',b'a'*48],[b'',b'a'*49,b'0a',b'A',b'a/b']),
]
count=0
for label,pattern,yes,no in classes:
 for values,want in [(yes,True),(no,False)]:
  for value in values:
   assert bool(re.fullmatch(pattern,value))==want,(label,value)
   count+=1
name_yes=[b'A',b'A B',b'!~',bytes.fromhex('51225c'),b'A'*64]
name_no=[b'',b'A'*65,b' A',b'A ',b'A  B',bytes.fromhex('410042'),bytes.fromhex('411f42'),bytes.fromhex('417f42'),bytes.fromhex('41c3a9')]
for values,want in [(name_yes,True),(name_no,False)]:
 for value in values:
  good=bool(re.fullmatch(rb'[\x20-\x7e]{1,64}',value)) and not value.startswith(b' ') and not value.endswith(b' ') and b'  ' not in value
  assert good==want;count+=1
for values,want in [([b'',b' A  B ',b'A'*512],True),([b'A'*513,bytes.fromhex('00'),bytes.fromhex('1f'),bytes.fromhex('7f'),bytes.fromhex('c3a9')],False)]:
 for value in values:
  assert bool(re.fullmatch(rb'[\x20-\x7e]{0,512}',value))==want;count+=1
assert count==39
limit=131072*1024
assert limit==134217728
assert all(-limit<=v<=limit for v in [-134217728,-1,0,1,134217728])
assert all(not -limit<=v<=limit for v in [-134217729,134217729,-9223372036854775808,9223372036854775807])
for x,y,want in [(-134217728,134217728,True),(134217729,0,False),(0,-134217729,False)]:
 assert (-limit<=x<=limit and -limit<=y<=limit)==want
assert 255**2<=255**2<256**2 and 256**2<=256**2<257**2 and 257**2<=257**2<258**2
assert [max(1,(n+255)//256) for n in (255,256,257)]==[1,1,2]
decoded=0
for label,width in [('EntityId',16),('BranchId',16),('RevisionId',32),('CatalogHash',32),('StateDigest',32)]:
 n=2*width;pattern=rb'[0-9a-f]{'+str(n).encode()+b'}'
 yes=[b'0'*n,b'f'*n,bytes(range(width)).hex().encode()]
 no=[b'0'*(n-1),b'0'*(n+1),b'A'+b'0'*(n-1),b'g'+b'0'*(n-1),b'/'+b'0'*(n-1),bytes.fromhex('c3a9')+b'0'*(n-2)]
 for values,want in [(yes,True),(no,False)]:
  for value in values:
   assert bool(re.fullmatch(pattern,value))==want
   if want:assert len(bytes.fromhex(value.decode()))==width
   decoded+=1
assert decoded==45
final=json.loads(literal('STATE_VALID_AFTER_TICK_1'))
SA='276ba6d854e10c707e8a2c7d7b563136';SB='72705000a50df53fdde38b7d8ae0fbcc'
TA='66f23b435e12d5071e0de540fd32a1a2';TB='e3b2357f43e3841e9fe8e359871fd6a2'
WA='a1ab79b780b86ae057c5edafa1106ed8';WB='19db1cfd5d4fe686505d9c9d0d016cbc'
I='4d4946530457913e248d68410d692932';X='f2481d1c434c9a316d9fc01a0c1b1016'
permutations=[
{'systems':[SB,SA],'stars':[TB,TA],'worlds':[WA,WB],'industries':[X,I],'owners':[WA,SB,SA,WB]},
{'systems':[SB,SA],'stars':[TA,TB],'worlds':[WA,WB],'industries':[I,X],'owners':[SB,WB,WA,SA]},
{'systems':[SA,SB],'stars':[TA,TB],'worlds':[WB,WA],'industries':[I,X],'owners':[WB,SA,SB,WA]}]
for case in permutations:
 for collection,order in case.items():
  assert len(order)==len(set(order))
  assert sorted(order,key=bytes.fromhex)==list(final[collection])
assert sorted([b'ore',b'energy'])==[b'energy',b'ore']
for n in ['FACTION_ENDIAN_ORDER','OWNERS_ENDIAN_ORDER']:
 keys=list(json.loads(literal(n)))
 assert keys==['00010000000000000000000000000000','01000000000000000000000000000000']
 assert bytes.fromhex(keys[0])<bytes.fromhex(keys[1])
 assert int.from_bytes(bytes.fromhex(keys[0]),'little')>int.from_bytes(bytes.fromhex(keys[1]),'little')
assert literal('INVENTORY_ENERGY_ABSENT')==b'{"ore":4}'
assert literal('INVENTORY_ENERGY_ZERO')==b'{"energy":0,"ore":4}'
batch=json.loads(literal('BATCH_VALID_GRAPH'))
# Static inspection of backwards compatible reference fields, not command application.
creation_kind={'create_faction':'faction','create_system':'system','create_star':'star','create_world':'world','connect_lane':'lane','create_deposit':'deposit','place_industry':'industry','connect_route':'route','schedule_hazard':'hazard'}
reference_fields={'create_star':{'system':'system'},'create_world':{'system':'system','primary':'star'},'connect_lane':{'a':'system','b':'system'},'create_deposit':{'world':'world'},'place_industry':{'world':'world','linked_deposit':'deposit'},'connect_route':{'source':'world','destination':'world'},'schedule_hazard':{'lane':'lane'},'set_owner':{'target':None,'faction':'faction'}}
seen={}
for op in batch['operations']:
 for field,kind in reference_fields.get(op['kind'],{}).items():
  ref=op[field]
  if ref is None:continue
  assert list(ref)==['kind','id'] and ref['kind']=='local' and ref['id'] in seen
  if kind:assert seen[ref['id']]==kind
  else:assert seen[ref['id']] in {'system','star','world','lane','deposit','industry','route','hazard'}
 if 'local' in op:
  assert op['local'] not in seen;seen[op['local']]=creation_kind[op['kind']]
assert len(seen)==13 and set(seen)==set(range(1,14))
# Catalog semantic choices directly inspected from committed input.
pack=json.loads(Path('assets/workshop/core-pack-v1.json').read_bytes())
solar=next(v for v in pack['industry_definitions'] if v['id']=='solar-array')
extractor=next(v for v in pack['industry_definitions'] if v['id']=='extractor')
hazard=pack['hazard_definitions'][0]
assert solar['kind']=='solar' and solar['duration_ticks']==1 and solar['outputs']==[{'resource_id':'energy','quantity':4}]
assert extractor['kind']=='extractor' and extractor['duration_ticks']==1 and extractor['energy_input']=={'resource_id':'energy','quantity':1} and extractor['output']=={'resource_id':'ore','quantity':2}
assert hazard['id']=='ion-storm' and hazard['duration_ticks']==36000
print('PASS:39 string byte cases,9 coordinate scalars,3 point constructor cases,45 decoded ID cases,15 map insertion lists,2 endian-opposed maps,absent/zero,3 distance/travel cases,13 distinct created locals and backward-compatible reference fields,core recipe values')

~~~

### Command C: identity binding check

~~~python
from pathlib import Path
import hashlib,json
b=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
p=b/'task-4a-valid-fixture-proposal.md'
e=b/'task-4a-valid-fixture-evidence'
assert hashlib.sha256(p.read_bytes()).hexdigest()=='9bdb9290fe84f046a13cc485e9b50435ffc462292023511ab2bcb3a0c7c7b920'
assert hashlib.sha256((e/'review-input-manifest.json').read_bytes()).hexdigest()=='e4e18a90285db9d4c43551f8510d0cf4281b1265d107c8ed727940768ba2267b'
m=json.loads((e/'review-input-manifest.json').read_bytes())
for row in m['files']:
    data=Path(row['path']).read_bytes()
    assert len(data)==row['bytes']
    assert hashlib.sha256(data).hexdigest()==row['sha256']
sources=(e/'source-bindings-before.txt').read_text().splitlines()
assert len(sources)==12
for line in sources:
    want,name=line.split('  ',1)
    q=Path(name)
    if not q.exists():
        q=b/name
    assert hashlib.sha256(q.read_bytes()).hexdigest()==want,(name,'DRIFT')
print('PASS: proposal, manifest, nine bound payloads, twelve source bindings unchanged')
~~~

Gaps: immutable full-corpus freeze; residual cases listed above; every implementation, admission, atomic-preservation, replay and runtime result remains Pending. This review's externally returned file SHA-256 binds the report without a self-referential checksum.

