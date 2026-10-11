# Task 4A scalar, nominal, equality and dynamic fixture derivation review

2026-10-09. **Verdict: APPROVED for the reviewed subset only.** Critical: 0. Important: 0. Minor: 0. No incorrect candidate byte value, arithmetic result, descriptor coverage or source-derived error boundary was identified. Actual Rust/Zig constructor, decoder, isolated compile rejection, structural equality, ownership and dynamic-view execution remain **Pending**. This review neither freezes a corpus nor completes Workshop. [task-4a-scalar-fixture-proposal.md:3,181,213,229]

Reviewed proposal: `.superpowers/sdd/2026-10-09-nyon-zig-migration/task-4a-scalar-fixture-proposal.md`, 35,488 bytes, SHA-256 `65132949e7639f5847d60798c775b0cf3a50b1325de2812f59aad8c44952929c`. Command A below rehashed the proposal and all 19 source bindings, including Cargo.lock and four pinned serde registry source files, without drift. Only this new review report was written. No production source, existing proposal/review, fixture, profile, asset or dependency was edited. No author generator, product serializer, Rust/Zig executable, build, test, gate, subagent, commit or remote action ran. [task-4a-scalar-fixture-evidence/source-before.json; command A below]

## Source contracts and review independence

The accepted design fixes integer authority, distinct nominal IDs, checked coordinates, deterministic integer distance, canonical state and atomic limits. The current IDs/model declarations supplement the design with exact scalar predicates, record ordering, tagged-reference representation and state hash domain. These source-derived compatibility details are identified as such rather than attributed to a fully specified independent wire document. The foundation report and the earlier independent-literal and valid-graph proposals/reviews were read for scope and provenance. The previously reviewed raw stress state remains invalid authority data and is used solely for representability/equality/encoding witnesses. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:176,203,211,235; crates/nyon-workshop-core/src/ids.rs:26,250,269; crates/nyon-workshop-core/src/model.rs:38,129,195,218; task-4a-foundation-vector-seams.md:22,24; task-4a-independent-fixture-review.md:3; task-4a-valid-fixture-review.md:3]

I independently transcribed each of the ten raw records and the owner map into exact byte fragments, assembled the baseline in source declaration order, and checked it against the earlier literal. I assembled each dynamic expectation by selecting complete top-level fragments rather than importing or running the author's derivation/substitution generator. Standard-library SHA-256 hashed the independently assembled bytes with the explicit 23-byte state domain. Python JSON decoding was used only to inspect values/field order and verify escape descriptors. No encoder produced expected payload bytes. [crates/nyon-workshop-core/src/model.rs:38,129,195,218; command A below]

## Reviewed finite coverage

The five fixed-ID wrappers have the specified 16/32 raw-byte and 32/64 decoded-hex widths. Their custom deserializers first acquire a string, then enforce decoded UTF-8 byte length and lowercase hex alphabet. The proposal's three successful plus 14 invalid decoded descriptors per type give 85 decoded cases; two escaped-string descriptors and four wrong-wire-type descriptors per type add 30, yielding 115. Invalid UTF-8 characters here are valid UTF-8 strings outside the alphabet, not invalid Rust string arguments. Big-endian lexicographic byte order is correctly distinguished from a little-endian integer comparison. [crates/nyon-workshop-core/src/ids.rs:11,26,43,71; task-4a-scalar-fixture-proposal.md:22,34,46; command A below]

The 20 directed cross-ID nominal pairs, six directed numeric-wrapper pairs and two CatalogId/ObjectName pairs total 28 negative compile descriptors; ten matching-type controls are separately named. Those are proposed isolated future invocations. Their enumeration is verified, but neither a Zig API nor successful/rejected compilation is claimed. The same-width ID pairs correctly test nominal identity rather than raw-array width alone. [docs/superpowers/specs/2026-09-02-nyon-v2-design.md:178; crates/nyon-workshop-core/src/ids.rs:9,11,85,89,140,191; task-4a-scalar-fixture-proposal.md:48,59; command A below]

Cargo.lock binds serde/serde_core 1.0.229 and serde_json 1.0.151; the manifests request ordinary derive/JSON dependencies. I inspected the pinned number parser and integer visitors directly. Negative-zero selects F64 in parse_number; decimal/exponent inputs likewise select F64. The i64/u16/u64 visitors do not implement float coercion and inherit the rejecting default. Unsigned overflow switches to F64; signed underflow follows the documented parser branch. GalaxyCoord adds its custom bound failure only after successful i64 deserialization. The proposal preserves these direct-consumer boundaries without inventing Workshop overflow errors or claiming resolved-feature ingress execution. [Cargo.lock:2506,2516,2536; crates/nyon-workshop-core/Cargo.toml:10; local serde_json-1.0.151/src/de.rs:462,509; local serde_core-1.0.229/src/de/impls.rs:399,420,436; local serde_core-1.0.229/src/de/mod.rs:1491; crates/nyon-workshop-core/src/ids.rs:209; task-4a-scalar-fixture-proposal.md:63,80]

The coordinate limit is 134,217,728. Opposite corners produce square 144,115,188,075,855,872 and integer floor root 379,625,062; the neighboring-square inequalities passed independently. The extra 3/4/5 and opposite-axis witnesses likewise passed. No legal-coordinate DistanceOverflow witness is invented. Point unknown fields are explicitly denied; ObjectRefV1 lacks that annotation. Aggregate duplicate-key/tag and unknown-field ingress remains separately Pending, as the proposal states. [crates/nyon-workshop-core/src/ids.rs:3,219,234,250,297; task-4a-scalar-fixture-proposal.md:82,84,88,90; command A below]

For all 128 ASCII byte values at first and subsequent positions, the ID helper alphabets match the source: 26 lowercase initial bytes, 38 catalog subsequent bytes and 39 pack subsequent bytes. Name interior printable bytes comprise 95 values; 33 controls/DEL fail, while isolated description space succeeds. Length bounds 32/48/64/512, edge/doubled spaces, quote/backslash, four valid Unicode sequences and escaped JSON strings are correctly separated by consumer. The pinned string escape parser decodes escapes before validation. Ownership/OOM clauses are explicitly proposed Zig resource contracts, not executed Rust failures. [crates/nyon-workshop-core/src/ids.rs:93,130,144,181,269,277,285,293; local serde_json-1.0.151/src/read.rs:874,900; task-4a-scalar-fixture-proposal.md:94,103,105,107,109,111; command A below]

The 199-byte minimal pack bridge and its field substitutions match the two-stage source boundary. Pack metadata name/title/description are ordinary strings subsequently validated; a definition's invalid CatalogId fails typed deserialization and maps to Json before validate_identity. The bridge does not stand in for the strict catalog corpus or executed pack acceptance. [crates/nyon-workshop-core/src/pack.rs:145,192,316,356,448; task-4a-scalar-fixture-proposal.md:119,121,123; command A below]

The equality matrix has 45 displayed rows that expand to 48 independent record mutations through three RGB-channel cases and two linked-deposit cases. It covers all 41 declared record fields. Tick, three owner cases and two cases for each of ten entity maps add 24 container mutations, totaling 72. I checked source field lists, original values, typed entity keys and the row coverage. Each described replacement is a different value or map key/presence; hash inequality is correctly not an equality oracle. Reflexivity/symmetry, detached clones, different insertion histories and free/allocator witnesses remain future consumers. [crates/nyon-workshop-core/src/model.rs:38,129; task-4a-scalar-fixture-proposal.md:127,129,179,181; command A below]

All nine dynamic payload lengths, full domain preimages and hashes reproduced. Individual empty-map cases replace complete collections; all-empty detects merging; poisoned receiver restores replacements; static faction change is retained; identity and distinct allocation share the same exact byte expectation. The other static maps stayed unchanged in the independent reconstruction. Runtime borrowing/lifetime/ownership and caller immutability remain Pending. [crates/nyon-workshop-core/src/model.rs:195,220; task-4a-scalar-fixture-proposal.md:185,187,207,209,231; command A below]

## Findings and residual ownership

No Critical, Important or Minor findings.

4A still owns compatibility acceptance for source supplements, an immutable reviewed fixture manifest, selected Zig constructor/map/clone/encode/digest APIs and actual scalar/nominal/equality/geometry/lifetime/OOM/differential witnesses. Direct scalar source inspection does not characterize raw aggregate duplicate keys or unknown-field/tag ingress. The report approves finite derivation inputs only. [task-4a-scalar-fixture-proposal.md:213; task-4a-foundation-vector-seams.md:125,162,168]

4B retains complete strict catalog JSON, size/depth/cardinality, quantity/duration, duplicate definitions, recipes/references, canonical export/hash and immutable catalog ownership. 4C retains creator wire, revision/namespace and receipt/order/size corpus. Admission/capacity, six-phase execution, authenticated history/replay/archive and Forge/platform acceptance belong to later owners. Those later operational results are not prerequisites for reviewing finite 4A byte candidates; the migration plan explicitly sequences 4A then 4B/4C/4D/4E. Full Workshop corpus, freeze and implementation remain Pending. [task-4a-scalar-fixture-proposal.md:215,217; docs/superpowers/plans/2026-10-09-nyon-zig-migration.md:144,152]

## Actual review commands and exits

Read-only source commands exited 0: `cat AGENTS.md`; `cat crates/nyon-workshop-core/src/ids.rs crates/nyon-workshop-core/src/model.rs`; targeted `sed` reads of pinned serde_json number/string parsing, serde_core integer visitors/default visit_f64, pack declarations/decode/validation, accepted design types/limits and migration decomposition; Python reads of the proposal, source manifest and earlier proposal/review scope sections. These reads supplied the source reasoning above and did not execute a product consumer.

One initial tool orchestration contained an unescaped JavaScript template delimiter and was rejected before any shell child ran. The first executed reconstruction command exited 1 because my review script incorrectly expected 44 displayed matrix rows. Inspection found 45 rows, exactly consistent with the proposal's 48 expanded cases. Correcting that reviewer-only assertion yielded the final command below, exit **0**. No candidate file was changed and this was not a product RED→GREEN or proposal finding.

### Command A: final independent descriptor/literal/source verification

Run from `/Users/donaldfilimon/dev/active/NYON` with `python3 - <<'PY'`, the following exact code, and closing `PY`. Child exit **0**.

~~~python
from pathlib import Path
import hashlib,json,re
b=Path('.superpowers/sdd/2026-10-09-nyon-zig-migration')
p=(b/'task-4a-scalar-fixture-proposal.md').read_text()
assert len(p.encode())==35488 and hashlib.sha256(p.encode()).hexdigest()=='65132949e7639f5847d60798c775b0cf3a50b1325de2812f59aad8c44952929c'
sources=json.loads((b/'task-4a-scalar-fixture-evidence/source-before.json').read_text())['sources']
assert len(sources)==19
for row in sources:
 d=Path(row['path']).read_bytes()
 assert len(d)==row['bytes'] and hashlib.sha256(d).hexdigest()==row['sha256'],row['path']
lock=Path('Cargo.lock').read_text()
for n,v in [('serde','1.0.229'),('serde_core','1.0.229'),('serde_json','1.0.151')]:
 assert re.search(r'\[\[package\]\]\nname = "'+n+r'"\nversion = "'+re.escape(v)+'"',lock)
print('PASS proposal and 19 source identities including pinned dependencies')
i=lambda n: ('0'*31+format(n,'x')).encode()
q=lambda n:b'"'+i(n)+b'"'
u=b'18446744073709551615'
records=[
('factions',1,b'{"name":"Q\\"\\\\","color_rgb":[0,127,255]}'),
('systems',2,b'{"name":"S","position":{"x":-134217728,"y":134217728}}'),
('stars',3,b'{"system":'+q(2)+b',"name":"T","archetype_id":"yellow-dwarf"}'),
('worlds',4,b'{"system":'+q(2)+b',"primary":'+q(3)+b',"name":"W","archetype_id":"rocky-world","orbit_radius_milli_au":4294967295,"orbit_period_ticks":'+u+b',"phase_millidegrees":4294967295,"inventory":{"a":0,"a-":1,"a0":9007199254740993,"a_":'+u+b'}}'),
('lanes',5,b'{"a":'+q(2)+b',"b":'+q(2)+b',"distance_units":'+u+b'}'),
('deposits',6,b'{"world":'+q(4)+b',"resource_id":"ore","reserve_units":'+u+b'}'),
('industries',7,b'{"world":'+q(4)+b',"definition_id":"extractor","linked_deposit":'+q(6)+b',"enabled":true}'),
('routes',8,b'{"source":'+q(4)+b',"destination":'+q(4)+b',"resource_id":"ore","batch_units":'+u+b'}'),
('shipments',9,b'{"route":'+q(8)+b',"source":'+q(4)+b',"destination":'+q(4)+b',"resource_id":"ore","units":'+u+b',"dispatched_tick":0,"arrival_tick":'+u+b'}'),
('hazards',10,b'{"lane":'+q(5)+b',"hazard_id":"ion-storm","start_tick":0,"duration_ticks":'+u+b',"cancelled":true}')]
frags={name:b'"'+name.encode()+b'":{'+q(n)+b':'+v+b'}' for name,n,v in records}
S=b'{"tick":'+u+b','+b','.join(frags.values())+b',"owners":{'+q(4)+b':'+q(1)+b'}}'
fence=chr(96)*3
prior=(b/'task-4a-independent-fixture-proposal.md').read_text()
want=prior.split(fence+'STATE_ALL_RECORDS_RAW\n')[1].split('\n'+fence)[0].encode()
assert S==want
model=Path('crates/nyon-workshop-core/src/model.rs').read_text()
data=json.loads(S)
classes=['FactionV1','SystemV1','StarV1','WorldV1','LaneV1','DepositV1','IndustryV1','RouteV1','ShipmentV1','HazardV1']
field_total=0
for cls,(name,n,v) in zip(classes,records):
 fields=re.findall(r'pub (\w+):',re.search(r'pub struct '+cls+r' \{(.*?)\n\}',model,re.S).group(1))
 assert fields==list(json.loads(v))
 field_total+=len(fields)
assert field_total==41
state_fields=re.findall(r'pub (\w+):',re.search(r'pub struct WorkshopStateV1 \{(.*?)\n\}',model,re.S).group(1))
assert state_fields==list(data)
table=p.split('## Every-field equality matrix')[1].split('The remaining24')[0]
rows=[x for x in table.splitlines() if x.startswith('| ')][2:]
assert len(rows)==45,len(rows)
covered=set();count=0
for row in rows:
 path,change=row.split('|')[1:3];path=path.strip()
 m=re.match(r'(\w+)\[I([1-9a])\]\.(\w+)',path);assert m,path
 name,n,field=m.groups();n=int(n,16)
 value=data[name][i(n).decode()][field];covered.add((name,field))
 if field=='color_rgb':assert value==[0,127,255];count+=3
 elif field=='position':assert value=={'x':-134217728,'y':134217728};count+=1
 elif field=='inventory':assert value=={'a':0,'a-':1,'a0':9007199254740993,'a_':2**64-1};count+=1
 elif field=='linked_deposit':assert value==i(6).decode();count+=2
 else:
  count+=1
  pairs=re.findall(r'([^;]*?)→([^;]*)',change);assert len(pairs)==1
  before=pairs[0][0].strip(' '+chr(96))
  if before.startswith('I'):assert value==i(int(before[1:],16)).decode(),path
  elif before=='true':assert value is True
  elif before.startswith('decoded'):assert value.encode().hex()=='51225c'
  elif before.isdecimal():assert value==int(before),path
  else:assert value==before,(path,value,before)
expected={(name,f) for name,n,v in records for f in json.loads(v)}
assert covered==expected and count==48,(count,covered^expected)
assert 1+3+2*len(records)==24 and count+24==72
print('PASS independently reconstructed S; 41 declared fields; 48 record plus 24 state mutations')
def assemble(tick=u,empty=(),faction=None):
 fs=[]
 for name,n,v in records:
  if name in empty:fs.append(b'"'+name.encode()+b'":{}')
  elif name=='factions' and faction is not None:fs.append(b'"factions":{'+q(1)+b':'+faction+b'}')
  else:fs.append(frags[name])
 return b'{"tick":'+tick+b','+b','.join(fs)+b',"owners":{'+q(4)+b':'+q(1)+b'}}'
cases={'DV_IDENTITY':S,'DV_TICK_ONLY':assemble(b'0'),'DV_WORLDS_EMPTY':assemble(empty=('worlds',)),'DV_DEPOSITS_EMPTY':assemble(empty=('deposits',)),'DV_SHIPMENTS_EMPTY':assemble(empty=('shipments',)),'DV_ALL_EMPTY':assemble(b'0',('worlds','deposits','shipments')),'DV_STATIC_CHANGED':assemble(faction=b'{"name":"R","color_rgb":[0,127,255]}'),'DV_POISONED_RECEIVER':S,'DV_DISTINCT_ALLOCATION':S}
log=(b/'task-4a-scalar-fixture-evidence/derive-literals-01.log').read_text()
domain=b'NYON-WORKSHOP-STATE-V1'+bytes([0])
for name,payload in cases.items():
 hp=re.search(r'^literal_dynamic_payload_hex '+name+r' ([0-9a-f]+)$',log,re.M).group(1)
 hi=re.search(r'^literal_dynamic_preimage_hex '+name+r' ([0-9a-f]+)$',log,re.M).group(1)
 assert bytes.fromhex(hp)==payload and bytes.fromhex(hi)==domain+payload
 digest=hashlib.sha256(domain+payload).hexdigest()
 row=re.search(r'^\| '+name+r' \| (\d+) \| (\d+) \| ([0-9a-f]{64}) \|',p,re.M).groups()
 assert row==(str(len(payload)),str(len(domain+payload)),digest)
 actual=json.loads(payload)
 for key in state_fields:
  if key not in {'tick','worlds','deposits','shipments','factions'}:assert actual[key]==data[key]
 if name=='DV_STATIC_CHANGED':assert actual['factions'][i(1).decode()]['name']=='R'
 else:assert actual['factions']==data['factions']
 print(name,len(payload),len(domain+payload),digest)
print('PASS nine independently assembled payloads/full preimages/SHA-256 and static/dynamic sentinels')
types={'EntityId':16,'BranchId':16,'RevisionId':32,'CatalogHash':32,'StateDigest':32}
total=0
for name,N in types.items():
 L=2*N
 bad=[b'0'*k for k in [0,1,L-1,L+1]]+[bytes([c])+b'0'*(L-1) for c in [0x41,0x46,0x47,0x67,0x2f,0x20,9,0,127]]+[bytes.fromhex('c3a9')+b'0'*(L-2)]
 assert len(bad)==14
 assert all(len(x)!=L or not all(c in b'0123456789abcdef' for c in x) for x in bad)
 assert json.loads('"\\u0030'+'0'*(L-1)+'"')=='0'*L
 assert json.loads('"\\u0041'+'0'*(L-1)+'"')=='A'+'0'*(L-1)
 A=bytes([0,1])+bytes(N-2);B=bytes([1])+bytes(N-1)
 assert A<B and int.from_bytes(A,'little')>int.from_bytes(B,'little')
 total+=17+2+4
assert total==115
names=list(types)
pairs={(a,z) for a in names for z in names if a!=z}
pairs|={(a,z) for a in ['WorkshopTick','BatchLocalId','GalaxyCoord'] for z in ['WorkshopTick','BatchLocalId','GalaxyCoord'] if a!=z}
pairs|={('CatalogId','ObjectName'),('ObjectName','CatalogId')}
assert len(pairs)==28 and len(names)+3+2==10
cat0={c for c in range(128) if 97<=c<=122}
cat1={c for c in range(128) if 97<=c<=122 or 48<=c<=57 or c in (45,95)}
assert (len(cat0),len(cat1),len(cat1|{46}))==(26,38,39)
assert len(set(range(32,127)))==95 and len(set(range(32))|{127})==33
for dx,dy,r in [(268435456,268435456,379625062),(3,4,5),(268435456,0,268435456)]:
 s=dx*dx+dy*dy;assert r*r<=s<(r+1)*(r+1);print('distance',dx,dy,s,r)
bridge=p.split(fence+'PACK_SCALAR_BRIDGE_BASE\n')[1].split('\n'+fence)[0].encode()
assert len(bridge)==199 and json.loads(bridge)['star_archetypes']==[]
print('PASS 115 ID descriptors; 28 negative/10 positive nominal descriptors; finite ASCII classes and distance arithmetic; 199-byte bridge')
print('No product runtime, encoder, author generator, Rust/Zig compiler, build or gate executed.')

~~~

Exact final stdout:

~~~text
PASS proposal and 19 source identities including pinned dependencies
PASS independently reconstructed S; 41 declared fields; 48 record plus 24 state mutations
DV_IDENTITY 1970 1993 f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5
DV_TICK_ONLY 1951 1974 9b13a2d2a7e7951ed448f466c4efa5878e00a5b2d09c4a551e0c3804425421a9
DV_WORLDS_EMPTY 1621 1644 9e80f17c4766fbe86ecbe67c5469ae23a31903b0ec6a011599f8ee87f9e9e0a3
DV_DEPOSITS_EMPTY 1834 1857 af2c2cbeecd92d05e6463532f109164c7c3f88c848d3db40641ca8fa262bb79d
DV_SHIPMENTS_EMPTY 1693 1716 a2ce1ccdd23373291ea67f6ae7b22666e2e12ff3fae90a4acfc4ee5c745b6e97
DV_ALL_EMPTY 1189 1212 ca4d56f73f7049e0551e1bbee6a7c8f7f75ab8a1362a187fe74a3bce29ec1e3e
DV_STATIC_CHANGED 1966 1989 d430b81c3ca9523df9954f3055acf90663d32fa3a2be933c37d33f83f076890c
DV_POISONED_RECEIVER 1970 1993 f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5
DV_DISTINCT_ALLOCATION 1970 1993 f0c1dc0c71db4253e4f3ba8b31b5f9323efe8b5d52565d46dcd0e9a643fd8da5
PASS nine independently assembled payloads/full preimages/SHA-256 and static/dynamic sentinels
distance 268435456 268435456 144115188075855872 379625062
distance 3 4 25 5
distance 268435456 0 72057594037927936 268435456
PASS 115 ID descriptors; 28 negative/10 positive nominal descriptors; finite ASCII classes and distance arithmetic; 199-byte bridge
No product runtime, encoder, author generator, Rust/Zig compiler, build or gate executed.
~~~

Gaps: full immutable corpus freeze and every actual constructor, decoder, compile, equality, ownership, admission, replay and runtime result remain Pending. The report's externally returned SHA-256 binds this file without a self-referential checksum.

