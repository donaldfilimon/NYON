#!/usr/bin/env python3
"""Current-source independent package A. Explicit shape/hash and admission boundaries."""
import copy
import hashlib
import json
from pathlib import Path
import runpy
import sys

BASE = Path(__file__).resolve().parent
SPECS = BASE.parent.parent
FREEZE = {'2026-10-01-living-expansion-architecture.md': '539009046173d4e6ffced1c7c3b79f1eaf977860d3bdfcd2e171fad76d630615', '2026-10-01-living-societies-rules.md': 'aa221fd432f193db4eedade9b690f4156bee264525ac58b5595a679ce9a02e71', '2026-10-01-living-ecology-rules.md': '003659ee5eacd9b952344066eb6024964b630273505f598a86a6b3dfd3287fc1', '2026-10-01-living-diplomacy-expansion.md': '045ebab7aed37ab81848dfc4e9225cfb43434f4518926b305397b468e7395747', '2026-10-01-living-expansion-wire.md': '1a8ff87d275e4f5e4a7e620179237f41339e64541c9942af3100374dfac70904', '2026-10-01-living-expansion-experience.md': '66cf1a76ae3111a7d7007892b180f9267f94d4fc5adcffd69b77e78ed9ab7cc4', '2026-10-01-living-v3-complete-schema.md': '9a05423608640515a6797fc3e73167a7213266d2e790f4d47ee73e0589996d73', '2026-10-02-living-v3-scoring.md': 'c8bd14e773cc59fc8c9f7eb78b1eea6b5a32f648c6955b00ffe757f34c34976d'}
WIRE_HELPER = SPECS / 'fixtures/living-v3-wire-proposed/derive.py'
HELPER_HASH = 'b328ea621eb552cc948c5d1de35ebe0b96bb41a19f9f05e75ccb791af76a6938'
assert hashlib.sha256(WIRE_HELPER.read_bytes()).hexdigest() == HELPER_HASH
w = runpy.run_path(str(WIRE_HELPER))
C, H, D = w['canonical'], w['H'], w['D']
U64 = 2**64 - 1


def frame(name, raw):
    return {'domain': name, 'canonical_utf8': raw.decode(), 'canonical_hex': raw.hex(),
            'byte_length': len(raw), 'hash_preimage_hex': (D(name) + raw).hex(),
            'sha256': H(D(name) + raw)}


def finalize_coordinator(body):
    return dict(body, integrity_sha256=H(D('COORDINATOR') + C(body)))


def coordinator(revision, maximum=False):
    linked = populated_hash_shape()
    return finalize_coordinator({'kind': 'NYON_SANDBOX_SELECTION_V3', 'format_version': 3,
        'authority': 'living_v3', 'rules_version': 3, 'slot': '11'*16,
        'catalog_hash': linked['catalog_frame']['sha256'], 'view': {'branch': linked['root_branch'], 'revision': revision,
        'tick': str(U64 if maximum else 0)}, 'generation': str(U64 if maximum else 0)})


def check_coordinator(raw):
    # Only explicitly normative exact boundaries; unspecified decoding errors stay rejection-only.
    if len(raw) > 504:
        return 'byte_capacity'
    try:
        v = w['parse'](raw, 504)
    except Exception:
        return 'rejection_only_decode_or_grammar'
    keys = 'kind format_version authority rules_version slot catalog_hash view generation integrity_sha256'.split()
    if set(v) != set(keys):
        return 'unknown_field' if set(v)-set(keys) else 'rejection_only_missing_field'
    if v['kind'] != 'NYON_SANDBOX_SELECTION_V3' or v['authority'] != 'living_v3':
        return 'unsupported_kind'
    if v['format_version'] != 3:
        return 'unsupported_format_version'
    if v['rules_version'] != 3:
        return 'unsupported_rules_version'
    if C(v) != raw:
        return 'noncanonical_bytes'
    if H(D('COORDINATOR') + C({k:v[k] for k in keys[:-1]})) != v['integrity_sha256']:
        return 'integrity_mismatch'
    # View/catalog/slot history admission explicitly NOT established here.
    return 'canonical_integrity_envelope_valid'


def coordinator_cases():
    values = [('maximum_nonnull', coordinator('44'*32, True), 504),
              ('maximum_null', coordinator(None, True), 442),
              ('ordinary_null', coordinator(None), None)]
    result = []
    for name, v, size in values:
        raw = C(v)
        if size is not None:
            assert len(raw) == size
        assert check_coordinator(raw) == 'canonical_integrity_envelope_valid'
        result.append({'id':name, 'document':v, 'frame':frame('COORDINATOR', C(dict(list(v.items())[:-1]))),
            'raw_hex':raw.hex(), 'raw_byte_length':len(raw),
            'expected':'canonical_integrity_envelope_valid', 'admission':'external_slot_catalog_history_unproven'})
    raw = C(values[0][1])
    negatives = [('505_whitespace', raw+b' ', 'byte_capacity'),
                 ('505_malformed_utf8_first', b'\xff'+b'x'*504, 'byte_capacity'),
                 ('505_malformed_json', b'{'+b'x'*504, 'byte_capacity')]
    bad = copy.deepcopy(values[0][1]); bad['kind']='X'*len(bad['kind'])
    negatives.append(('505_unsupported_kind', C(bad)+b' ', 'byte_capacity'))
    bad = copy.deepcopy(values[0][1]); bad['format_version']=9
    negatives.append(('505_unsupported_format', C(bad)+b' ', 'byte_capacity'))
    bad = copy.deepcopy(values[0][1]); bad['rules_version']=9
    negatives.append(('505_unsupported_rules', C(bad)+b' ', 'byte_capacity'))
    bad = copy.deepcopy(values[2][1]); bad['extra']=0
    malformed = C(bad)
    negatives.append(('505_unknown_field',malformed+b' '*(505-len(malformed)),'byte_capacity'))
    for name, k, value, expected in [('unknown_field','x',0,'unknown_field'),
            ('wrong_kind','kind','NO','unsupported_kind'),
            ('wrong_format','format_version',9,'unsupported_format_version'),
            ('wrong_rules','rules_version',9,'unsupported_rules_version'),
            ('forged_integrity','integrity_sha256','00'*32,'integrity_mismatch')]:
        bad=copy.deepcopy(values[2][1]);bad[k]=value
        if k != 'integrity_sha256':
            body={field:value for field,value in bad.items() if field != 'integrity_sha256'}
            bad=finalize_coordinator(body)
        negatives.append((name,C(bad),expected))
    negatives += [('504_malformed_utf8_first',b'\xff'+b'x'*503,'rejection_only_decode_or_grammar'),
                  ('504_malformed_json',b'{'+b'x'*503,'rejection_only_decode_or_grammar')]
    for name, raw, expected in negatives:
        assert check_coordinator(raw)==expected
        result.append({'id':name,'raw_hex':raw.hex(),'raw_byte_length':len(raw),
            'expected_wire_error':None if expected.startswith('rejection_only') else expected,
            'expected':expected,'tag_strength':'rejection_only' if expected.startswith('rejection_only') else 'exact_wire_tag'})
    return result


def populated_hash_shape():
    cat=w['frozen_catalog']();w['catalog'](cat);ch=H(D('PACK')+C(cat))
    seed='00'*32;gen={'generator_id':'package_a','generator_version':1};alloc=[];issuance_frames=[]
    def issue(kind,ordinal):
        frame_bytes=(D('GENESIS_ENTITY')+bytes.fromhex(ch)+bytes.fromhex(seed)+
            w['length'](gen['generator_id'].encode())+w['le'](1,4)+b'\x03'+
            w['le'](kind,2)+w['le'](ordinal,8))
        id=H(frame_bytes)[:32];alloc.append({'kind':kind,'ordinal':str(ordinal),'id':id})
        issuance_frames.append({'kind':kind,'ordinal':str(ordinal),'frame_hex':frame_bytes.hex(),'sha256':H(frame_bytes),'id':id})
        return id
    system=issue(1,1);hidden_system=issue(1,2);star=issue(2,1);world=issue(3,1);hidden=issue(3,2);actor=issue(5,1);eid=issue(18,1)
    state=w['state_empty']()
    state['systems']=sorted([{'id':system,'name':'Observed System'},{'id':hidden_system,'name':'Hidden System'}],key=lambda x:x['id'])
    state['stars']=[{'id':star,'system':system,'archetype':'main_sequence','name':'Observed Star','next_activity_tick':'1000'}]
    def wr(id,sys,owner):
        return {'id':id,'system':sys,'archetype':'temperate','name':'World', 'owner':owner,
            'inventory':{'energy':0,'ore':0,'alloy':0,'food':0},
            'environment':{'base_habitability':10000,'stress':0,'fertility':10000,'maximum_fertility':10000,'next_environment_tick':'100'}}
    own=wr(world,system,actor);state['worlds']=sorted([own,wr(hidden,hidden_system,None)],key=lambda x:x['id'])
    research={'target':None,'progress':[],'completed_technologies':[]}
    state['civilizations']=[{'id':actor,'name':'Actor','policy':'neutral','research':research,'status':'active'}]
    colony={'world':world,'owner':actor,'hub':{'energy_next_due':'10','ore_next_due':'10','fallback_next_due':'10','blocked_reasons':[]},
        'society':{'population':0,'specialization':'balanced','next_needs_tick':'100','last_needs':None,'workforce_assignments':[]}}
    state['colonies']=[colony]
    event={'id':eid,'star':star,'system':system,'warning_tick':'0','start_tick':'100','end_tick':'200','habitability_impact':2000,
        'freight_factor':{'numerator':1,'denominator':2},'originating_observer':actor,'observed_tick':'0'}
    scope={'system':system,'originating_observer':actor,'observed_tick':'0','events':[event]}
    environment={'base_habitability':10000,'stress':0,'fertility':10000,'maximum_fertility':10000,'stellar_scope':scope}
    observation={'civilization':actor,'world':world,'observed_tick':'0','owner':actor,'remaining_deposit':'0',
        'facilities':[],'stationed_hulls':[],'operational_defense_batteries':0,'environment':environment,'threats':[],'hazard_scopes':[]}
    state['observations']=[observation]
    root_event={k:v for k,v in event.items() if k not in ('system','originating_observer','observed_tick')}
    root_event['provenance']={'type':'genesis','generator':gen,'kind':18,'ordinal':'1'}
    state['stellar_events']=[root_event]
    genesis={'kind':'NYON_LIVING_GENESIS_V3','format_version':3,'rules_version':3,'catalog_hash':ch,'seed':seed,
        'generator':gen,'allocations':sorted(alloc,key=lambda x:(x['kind'],int(x['ordinal']))),'state':state}
    gd=H(D('GENESIS')+C(genesis));branch_frame=D('ROOT_BRANCH')+bytes.fromhex(ch)+bytes.fromhex(seed)+bytes.fromhex(gd)+w['length'](gen['generator_id'].encode())+w['le'](1,4)
    branch=H(branch_frame)[:32]
    payload={'actor':actor,'own_worlds':[own],'own_colonies':[colony],'own_facilities':[],'own_fleets':[],
        'actor_policy':'neutral','actor_status':'active','research':research,'own_jobs':[],'own_hull_jobs':[],
        'own_routes':[],'own_shipments':[],'incoming_shipments':[],'agreements':[],'wars':[],'offers':[],
        'known_hazards':[],'known_stellar_events':[event],'known_hazard_scopes':[],'known_stellar_scopes':[scope],
        'known_systems':state['systems'],'known_lanes':[],'relations':[],'commitments':[],
        'observations':[observation],'demands':[],'defense_requests':[],'shared_intelligence':[]}
    obsframe=frame('OBSERVATION',C(payload));sd=H(D('STATE')+C(w['state_payload'](ch,state)))
    context={'domain':3,'catalog_hash':ch,'branch':branch,'tick':'0','phase':7,'permitted_snapshot_digest':obsframe['sha256']}
    snap={'domain':3,'catalog_hash':ch,'branch':branch,'revision':None,'tick':'0','state_digest':sd,'phase':7,'permitted_snapshot_digest':obsframe['sha256']}
    trace_bytes=D('CANDIDATE_AUDIT')+bytes.fromhex(actor)+w['le'](0,4)+w['length'](C(context))
    audit={'actor':actor,'lane':0,'snapshot':snap,'enumerated_count':'0','trace_digest':H(trace_bytes),'family_bests':[],'attempts':[]}
    modified=copy.deepcopy(state)
    next(v for v in modified['worlds'] if v['id']==hidden)['inventory']['energy']=1
    modified_sd=H(D('STATE')+C(w['state_payload'](ch,modified)))
    assert modified_sd!=sd
    modified_snap=dict(snap,state_digest=modified_sd,revision='11'*32)
    assert C(context)==C({k:modified_snap[k] for k in context})
    return {'admission':'complete_hash_shape_only_not_authenticated_history_or_actor_admission',
        'catalog':cat,'catalog_frame':frame('PACK',C(cat)),'referenced_state':state,
        'state_frame':frame('STATE',C(w['state_payload'](ch,state))),'genesis':genesis,
        'genesis_frame':frame('GENESIS',C(genesis)),'genesis_issuance_frames':issuance_frames,'root_branch_frame_hex':branch_frame.hex(),'root_branch_full_digest':H(branch_frame),'root_branch':branch,
        'full_actor_payload':payload,'permitted_snapshot_frame':obsframe,'ActorInputContext':context,
        'ActorInputContext_canonical_hex':C(context).hex(),'outer_snapshot':snap,
        'empty_trace_frame_hex':trace_bytes.hex(),'empty_trace_digest':H(trace_bytes),'CandidateAudit_hash_shape':audit,
        'hidden_change':{'mutated_referenced_state':modified,'state_digest':modified_sd,'outer_snapshot':modified_snap,
        'ActorInputContext':context,'same_permitted_digest':obsframe['sha256'],'same_trace_digest':H(trace_bytes),
        'admission':'arbitrary_revision_marker_for_provenance_shape_not_admitted_creator_revision'},
        'omissions':['No authenticated phase7 source selection/replay','No claim empty eligible trace is actual actor output',
        'No Receipt authentication of audit or hidden creator revision','No full State/Genesis validator']}


def corpus():
    for n,h in FREEZE.items():
        assert H((SPECS/n).read_bytes())==h,'source_freeze_changed'
    return {'status':'PROPOSED_PARTIAL_PACKAGE_A_NOT_FULL_GAP6','source_freeze':FREEZE,
        'helper_source_hash':HELPER_HASH,'coordinator_cases':coordinator_cases(),
        'populated_actor_hash_shape':populated_hash_shape()}


def coverage():
    return {'status':'PARTIAL_GAP6_PACKAGE_A', 'source_freeze':FREEZE,
        'independent_review':'pending',
        'rows':[
          {'section':'complete-schema Integrated correction I6 / Coordinator',
           'fixture_ids':['maximum_nonnull','maximum_null','ordinary_null','505_whitespace','505_malformed_utf8_first','505_malformed_json','505_unsupported_kind','505_unsupported_format','505_unsupported_rules','505_unknown_field','unknown_field','wrong_kind','wrong_format','wrong_rules','forged_integrity','504_malformed_utf8_first','504_malformed_json'],
           'derivation':'derive.py coordinator_cases/check_coordinator',
           'covered':['504/442 canonical envelope widths','all Coordinator field order','COORDINATOR domain/integrity framing','505 preparse byte_capacity before malformed UTF8/JSON/kind/version','single-defect unknown_field/kind/format/rules/integrity','504 malformed rejection-only'],
           'missing':['full external slot/catalog/archive/view admission','duplicate keys/unknown nested view fields','u64/hex/canonical/depth negatives','<=504 multi-failure precedence not specified by frozen source'],
           'validation_strength':'partial envelope validator, not full Coordinator admission'},
          {'section':'complete-schema SnapshotReference actor payload / Integrated correction I2',
           'fixture_ids':['populated_actor_hash_shape'], 'derivation':'derive.py populated_hash_shape',
           'covered':['all 28 actor payload fields in exact order','populated own World/Colony/Research/Observation/StellarScope/positive fact with referenced topology','full OBSERVATION canonical bytes and domain digest','full ActorInputContext','full SnapshotReference binding exact wrapped State digest','empty trace exact context/actor/lane framing','hidden-root stock mutation changes outer State digest without changing permitted/context/trace bytes'],
           'missing':['remote world geometry absent from frozen actor payload','owned per-ID deposits absent','claim/occupation source rows and seed-dependent future arbitration unbound','foreign recipient storage/disposition and standing-route capacity outcomes unbound','full authenticated capture/history/revision validation','nonempty eligible CandidateTrace','Receipt/Event authentication of audit','exact per-array source selection for relations and defense_requests','complete State/Genesis validator','full admitted hidden creator revision'],
           'validation_strength':'complete-field hash shape only; no admitted actor trace or authority history'},
          {'section':'complete-schema Catalog and identity/state/genesis/root framing',
           'fixture_ids':['populated_actor_hash_shape'], 'derivation':'derive.py populated_hash_shape plus frozen historical written-formula helper',
           'covered':['current full catalog canonical bytes and hash','subset catalog validation','genesis entity IDs/kind/ordinal frame','complete populated State wrapper bytes/hash','Genesis and root-branch framing'],
           'missing':['every catalog predicate and rejection','all18 issuance kinds','full allocation/collision-history validation','all nonempty State families','valid chronology and complete Genesis/Archive admission'],
           'validation_strength':'explicit framing inputs, no complete document acceptance'},
          {'section':'complete-schema Integrated correction I1 environmental roster/capacity',
           'fixture_ids':[], 'derivation':'not implemented in this package',
           'covered':[], 'missing':['admitted moved/shortened expiry','same-scope cross-world empty coverage','ties/conflicting provenance','128/129 positive roster boundary','full actor hash/replay for each environmental case'],
           'validation_strength':'blocked full admission family pending exact source predicates; separate12fragment corpus remains scoped historical/current-source subset'}],
        'original_gap_status':{str(n):'Partial or open; no closure from this package' for n in range(1,7)},
        'all_other_missing':['complete command/Event/Facts/Reason/Record/TypedKey/metric variant registries','200-boundary scoring and all utility policy tables','complete diplomatic/conservation trajectories','nonempty archives/notebooks/transfers/anchors/checkpoints','bounded collision/cancellation/work/CAS behavior','all platform/runtime/resource qualification'],
        'unresolved_exact_error_boundaries':['<=504 malformed UTF8/JSON tags absent from closed enum; rejection-only','<=504 multiple schema/version/kind/integrity defects order unbound; only cap-first normative']}


def main():
    assert len(sys.argv)==2 and sys.argv[1] in ('generate','verify')
    raw=(json.dumps(corpus(),ensure_ascii=False,indent=2)+'\n').encode();target=BASE/'generatedvectors.json'
    ledger=(json.dumps(coverage(),ensure_ascii=False,indent=2)+'\n').encode();ledger_path=BASE/'coverage.json'
    if sys.argv[1]=='generate':
        target.write_bytes(raw);ledger_path.write_bytes(ledger)
    else:
        assert target.read_bytes()==raw,'generated_corpus_drift'
        assert ledger_path.read_bytes()==ledger,'coverage_ledger_drift'
    print(sys.argv[1]+': 17 coordinator cases and1 full-field actor hash-shape family; admission unproven')


if __name__=='__main__':main()
