#!/usr/bin/env python3
"""Independent zero-record Proposed V3 fixtures. No engine/generator imports."""
import copy
import hashlib
import json
import re
import struct
import sys
from pathlib import Path
HERE=Path(__file__).resolve().parent
SPECS=HERE.parent.parent
CATALOG_FILE=HERE.parent/'living-v3-catalog-proposed/generatedvectors.json'
CATALOG_FREEZE='ab6b0619b6b88bb316246e711bacecd654a3617ecae6bab03f6224cf195fe20a'
COLLECTIONS='systems stars worlds lanes civilizations deposits colonies facilities construction_jobs hull_jobs fleets routes shipments relations agreements wars observations hazards settlement_claims occupations offers demand_requests commitments defense_requests shared_intelligence stellar_events'.split()
STATE_FIELDS=['tick','branch_sequence','next_sequence']+COLLECTIONS+['counters']
GENESIS_FIELDS=['kind','format_version','rules_version','catalog_hash','seed','generator','allocations','state']
ROOT_FIELDS=['id','parent','fork_revision','fork_tick','fork_state_digest','ordinal','head','tip_tick']
KINDS='system star world lane civilization deposit facility construction_job hull_job fleet hull route shipment hazard offer commitment defense_request stellar_event'.split()
U64=(1<<64)-1
CAP=8*1024*1024

def H(b):return hashlib.sha256(b).hexdigest()
def C(v):return json.dumps(v,ensure_ascii=False,separators=(',',':'),allow_nan=False).encode('utf-8')
def D(name):return ('NYON_LIVING_V3_'+name).encode('ascii')+b'\x00'
def L(b):return struct.pack('<I',len(b))+b
def fixedhex(v,n):return isinstance(v,str) and re.fullmatch('[0-9a-f]{'+str(n*2)+'}',v) is not None
def u64(v):return isinstance(v,str) and re.fullmatch('0|[1-9][0-9]*',v) is not None and len(v)<=20 and int(v)<=U64
def u32(v):return type(v) is int and 0<=v<(1<<32)
def slug(v):return isinstance(v,str) and re.fullmatch('[a-z][a-z0-9_]{0,63}',v) is not None

def dependency():
    b=CATALOG_FILE.read_bytes();assert H(b)==CATALOG_FREEZE,'catalog_dependency_changed'
    catalog=json.loads(b)
    for f,h in catalog['source_sha256'].items():assert H((SPECS/f).read_bytes())==h,'source_freeze_changed: '+f
    raw=bytes.fromhex(catalog['canonical_hex'])
    assert C(catalog['catalog'])==raw and len(raw)==4264
    assert H(D('PACK')+raw)==catalog['pack_sha256']=='9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'
    return catalog,raw

def decode(raw):
    # Only State/Genesis have this top-level raw cap in this family.
    if len(raw)>CAP:return None,[{'path':'$','predicate':'byte_capacity','normative_error':'byte_capacity'}]
    failures=[]
    depth=0;quoted=False;escape=False
    for b in raw:
        if quoted:
            if escape:escape=False
            elif b==92:escape=True
            elif b==34:quoted=False
        elif b==34:quoted=True
        elif b in (91,123):
            depth+=1
            if depth>64:return None,[{'path':'$','predicate':'depth_capacity'}]
        elif b in (93,125):depth-=1
    def pairs(ps):
        d={}
        for k,v in ps:
            if k in d:failures.append({'path':'$','predicate':'duplicate_key'})
            d[k]=v
        return d
    try:v=json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_constant=lambda _:(_ for _ in ()).throw(ValueError()))
    except (UnicodeError,ValueError,RecursionError):return None,[{'path':'$','predicate':'invalid_json_or_utf8'}]
    try:
        if C(v)!=raw:failures.append({'path':'$','predicate':'noncanonical_bytes'})
    except (UnicodeError,ValueError):failures.append({'path':'$','predicate':'noncanonical_bytes'})
    return v,failures

def validate(raw,family,cat_hash):
    x,fail=decode(raw);unsupported=[]
    def bad(p,c):fail.append({'path':p,'predicate':c})
    def fields(v,ks,p):
        if not isinstance(v,dict):bad(p,'record_shape');return False
        if list(v)!=ks:bad(p,'closed_fields_or_order')
        return all(k in v for k in ks)
    def state(v,p):
        if not fields(v,STATE_FIELDS,p):return
        for k in STATE_FIELDS[:3]:
            if not u64(v[k]):bad(p+'.'+k,'canonical_u64')
        for k in COLLECTIONS:
            if not isinstance(v[k],list):bad(p+'.'+k,'array')
            elif v[k]:unsupported.append(p+'.'+k)
        if type(v['counters']) is not dict or v['counters']!={}:bad(p+'.counters','closed_empty_counters')
    if x is not None:
        if family=='state':state(x,'$')
        elif family=='genesis':
            if fields(x,GENESIS_FIELDS,'$'):
                if x['kind']!='NYON_LIVING_GENESIS_V3':bad('$.kind','genesis_kind')
                for k in ['format_version','rules_version']:
                    if not u32(x[k]) or x[k]!=3:bad('$.'+k,'exact_version')
                if not fixedhex(x['catalog_hash'],32) or x['catalog_hash']!=cat_hash:bad('$.catalog_hash','catalog_binding')
                if not fixedhex(x['seed'],32):bad('$.seed','seed_width')
                if fields(x['generator'],['generator_id','generator_version'],'$.generator'):
                    if not slug(x['generator']['generator_id']):bad('$.generator.generator_id','slug')
                    if not u32(x['generator']['generator_version']):bad('$.generator.generator_version','u32')
                if not isinstance(x['allocations'],list):bad('$.allocations','array')
                else:
                    previous=None;ids=set()
                    can_frame=fixedhex(x['catalog_hash'],32) and fixedhex(x['seed'],32) and isinstance(x['generator'],dict) and slug(x['generator'].get('generator_id')) and u32(x['generator'].get('generator_version'))
                    g=bytes.fromhex(x['catalog_hash'])+bytes.fromhex(x['seed'])+L(x['generator']['generator_id'].encode())+struct.pack('<I',x['generator']['generator_version']) if can_frame else None
                    for j,a in enumerate(x['allocations']):
                        p='$.allocations['+str(j)+']'
                        if not fields(a,['kind','ordinal','id'],p):continue
                        kind=type(a['kind']) is int and 1<=a['kind']<=18
                        ordinal=u64(a['ordinal']) and int(a['ordinal'])>0
                        if not kind:bad(p+'.kind','registered_kind_u16')
                        if not ordinal:bad(p+'.ordinal','positive_kind_local_u64')
                        if not fixedhex(a['id'],16):bad(p+'.id','id_width')
                        if kind and ordinal:
                            key=(a['kind'],int(a['ordinal']))
                            if previous is not None and previous>=key:bad(p,'strict_numeric_kind_ordinal_order')
                            previous=key
                            if g is not None:
                                expected=H(D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',a['kind'],int(a['ordinal'])))[:32]
                                if a['id']!=expected:bad(p+'.id','genesis_issuance_frame')
                        if isinstance(a['id'],str):
                            if a['id'] in ids:bad(p+'.id','duplicate_live_id')
                            ids.add(a['id'])
                    # Correspondence is decidable when every live collection is empty.
                    if isinstance(x['state'],dict) and all(x['state'].get(k)==[] for k in COLLECTIONS) and x['allocations']:
                        bad('$.allocations','zero_manifest_has_no_entities_to_allocate')
                state(x['state'],'$.state')
                if isinstance(x['state'],dict) and u64(x['state'].get('tick')) and x['state']['tick']!='0':bad('$.state.tick','explicit_tick_zero_manifest')
                # Historical allocator headers not silently prescribed by a wider State decoder.
                if isinstance(x['state'],dict):
                    for k in ['branch_sequence','next_sequence']:
                        if u64(x['state'].get(k)) and x['state'][k]!='0':unsupported.append('$.state.'+k)
        else:raise AssertionError(family)
    return {'status':'reject' if fail else 'outside_family' if unsupported else 'admit_zero_record_state' if family=='state' else 'admit_zero_record_genesis','diagnostics':fail,'unsupported_paths':unsupported}

def frame(name,b):return {'name':name,'byte_length':len(b),'literal_hex':b.hex(),'sha256':H(b)}
def root_errors(v,rid,sd):
    # Root Branch/HistoryView are diagnostic records, not standalone portable kinds.
    errors=[]
    if not isinstance(v,dict) or list(v)!=ROOT_FIELDS:return ['closed_root_branch_fields']
    if not fixedhex(v['id'],16) or v['id']!=rid:errors.append('root_identity')
    if v['parent'] is not None:errors.append('root_parent_null')
    if v['fork_revision'] is not None:errors.append('root_fork_revision_null')
    if not u64(v['fork_tick']) or v['fork_tick']!='0':errors.append('root_fork_tick_zero')
    if not fixedhex(v['fork_state_digest'],32) or v['fork_state_digest']!=sd:errors.append('genesis_state_binding')
    if not u64(v['ordinal']) or v['ordinal']!='0':errors.append('root_ordinal_zero')
    if v['head'] is not None:errors.append('missing_reachable_revision_in_empty_history')
    if not u64(v['tip_tick']):errors.append('tip_tick_u64')
    elif v['tip_tick']!='0':errors.append('outside_zero_tip_family')
    return errors

def corpus():
    dep,cat_bytes=dependency();ch=dep['pack_sha256'];seed='00'*32;generator={'generator_id':'fixture_minimal','generator_version':1}
    s={**{k:'0' for k in STATE_FIELDS[:3]},**{k:[] for k in COLLECTIONS},'counters':{}}
    gen={'kind':'NYON_LIVING_GENESIS_V3','format_version':3,'rules_version':3,'catalog_hash':ch,'seed':seed,'generator':generator,'allocations':[],'state':s}
    wrap={'format_version':3,'rules_version':3,'catalog_hash':ch,'state':s}
    state_pre=D('STATE')+C(wrap);sd=H(state_pre)
    gen_pre=D('GENESIS')+C(gen);gd=H(gen_pre)
    g=bytes.fromhex(ch)+bytes.fromhex(seed)+L(generator['generator_id'].encode())+struct.pack('<I',1)
    root_pre=D('ROOT_BRANCH')+bytes.fromhex(ch)+bytes.fromhex(seed)+bytes.fromhex(gd)+L(generator['generator_id'].encode())+struct.pack('<I',1)
    rid=H(root_pre)[:32]
    branch={'id':rid,'parent':None,'fork_revision':None,'fork_tick':'0','fork_state_digest':sd,'ordinal':'0','head':None,'tip_tick':'0'}
    view={'branch':rid,'revision':None,'tick':'0'}
    marks={'accepted_sequence_high_water':'0','branch_sequence_high_water':'0'}
    assert not root_errors(branch,rid,sd)
    assert all(u64(v) and v=='0' for v in marks.values())
    assert list(view)==['branch','revision','tick'] and view['branch']==rid and view['revision'] is None and u64(view['tick'])
    cases=[]
    def add(label,family,b,status):
        result=validate(b,family,ch);assert result['status']==status,(label,result)
        assert label not in {c['id'] for c in cases}
        cases.append({'id':label,'family':family,'expected':status,'expected_error':'byte_capacity' if len(b)>CAP else None,'raw_hex':b.hex(),'byte_length':len(b),'diagnostics':result['diagnostics'],'unsupported_paths':result['unsupported_paths']})
    add('STATE-P01-zero','state',C(s),'admit_zero_record_state')
    add('GENESIS-P01-zero','genesis',C(gen),'admit_zero_record_genesis')
    def mut(label,fn,family='genesis'):
        v=copy.deepcopy(gen if family=='genesis' else s);fn(v);add(label,family,C(v),'reject')
    for k in STATE_FIELDS:
        mut('STATE-N-missing-'+k,lambda v,k=k:v.pop(k),'state')
    mut('STATE-N-extra',lambda v:v.__setitem__('hulls',[]),'state')
    mut('STATE-N-counters-nonempty',lambda v:v.__setitem__('counters',{'a':0}),'state')
    for k in COLLECTIONS:mut('STATE-N-array-type-'+k,lambda v,k=k:v.__setitem__(k,None),'state')
    for j,value in enumerate(['-1','-0','00','01','18446744073709551616',0,True,None]):
        for k in STATE_FIELDS[:3]:mut('STATE-N-u64-'+k+'-'+str(j),lambda v,k=k,value=value:v.__setitem__(k,value),'state')
    for k in GENESIS_FIELDS:mut('GENESIS-N-missing-'+k,lambda v,k=k:v.pop(k))
    mut('GENESIS-N-extra',lambda v:v.__setitem__('integrity_sha256','00'*32))
    for k,value in [('kind','NYON_LIVING_GENESIS_V2'),('format_version',2),('rules_version',2),('catalog_hash','11'*32),('seed','0'*63)]:mut('GENESIS-N-'+k,lambda v,k=k,value=value:v.__setitem__(k,value))
    mut('GENESIS-N-tick-one',lambda v:v['state'].__setitem__('tick','1'))
    for k in ['generator_id','generator_version']:mut('GENESIS-N-generator-missing-'+k,lambda v,k=k:v['generator'].pop(k))
    mut('GENESIS-N-generator-extra',lambda v:v['generator'].__setitem__('seed',seed))
    mut('GENESIS-N-generator-slug',lambda v:v['generator'].__setitem__('generator_id','Invalid'))
    for i,value in enumerate([True,-1,4294967296,'1',None]):mut('GENESIS-N-generator-version-'+str(i),lambda v,value=value:v['generator'].__setitem__('generator_version',value))
    valid_id=H(D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',1,1))[:32]
    allocation={'kind':1,'ordinal':'1','id':valid_id}
    mut('GENESIS-N-roster-unmatched-live-manifest',lambda v:v['allocations'].append(allocation))
    for field,values in [('kind',[0,19,65536,True,'1']),('ordinal',['0','-1','01','18446744073709551616',1]),('id',['00'*16,'0'*31,'A'*32,None])]:
        for i,value in enumerate(values):
            def edit(v,field=field,value=value):a=copy.deepcopy(allocation);a[field]=value;v['allocations']=[a]
            mut('GENESIS-N-roster-'+field+'-'+str(i),edit)
    for field in allocation:
        def edit(v,field=field):a=copy.deepcopy(allocation);a.pop(field);v['allocations']=[a]
        mut('GENESIS-N-allocation-missing-'+field,edit)
    mut('GENESIS-N-roster-duplicate',lambda v:v.__setitem__('allocations',[allocation,allocation]))
    mut('GENESIS-N-roster-reordered',lambda v:v.__setitem__('allocations',[{'kind':2,'ordinal':'1','id':H(D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',2,1))[:32]},allocation]))
    ordered={'state':gen['state'],**gen}
    add('GENESIS-N-record-field-order','genesis',C(ordered),'reject')
    state_order={'counters':{},**s}
    add('STATE-N-record-field-order','state',C(state_order),'reject')
    raw=C(gen)
    for label,b in [('duplicate-key',raw.replace(b'"rules_version":3',b'"rules_version":3,"rules_version":3',1)),('whitespace',b' '+raw),('newline',raw+b'\n'),('invalid-utf8',b'\xff'),('invalid-json',b'{'),('alternate-escape',raw.replace(b'fixture_minimal',b'\\u0066ixture_minimal',1)),('float',raw.replace(b'"generator_version":1',b'"generator_version":1.0',1)),('leading-zero',raw.replace(b'"generator_version":1',b'"generator_version":01',1)),('depth65',b'['*65+b'0'+b']'*65),('byte-cap-malformed',b'\xff'+b' '*(CAP))]:add('GENESIS-N-raw-'+label,'genesis',b,'reject')
    # No claim that syntactically well-formed inhabited records are invalid wire.
    nonempty=copy.deepcopy(s);nonempty['systems']=[{'id':valid_id,'name':'System'}]
    add('STATE-U01-nonempty-not-validated','state',C(nonempty),'outside_family')
    # Valid State header values are not invalid merely because this fixture starts0.
    width=copy.deepcopy(s)
    for k in STATE_FIELDS[:3]:width[k]=str(U64)
    add('STATE-P02-header-width-leaf','state',C(width),'admit_zero_record_state')
    # Shape/scalar admission only at nonzero counters/tick; no history replay assertion.
    cases[-1]['qualification']='zero-record structural State; full-range headers do not assert reached/replayed chronology'
    issuance=[]
    for kind,name in enumerate(KINDS,1):
        pre=D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',kind,1)
        issuance.append({'kind':kind,'kind_name':name,'ordinal':'1','namespace_flag':3,'scope':'issuance_frame_leaf_only','frame':frame(name,pre),'id':H(pre)[:32]})
    leaves=[]
    for kind,ord_ in [(1,1),(1,3),(2,1)]:
        pre=D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',kind,ord_)
        leaves.append({'kind':kind,'ordinal':str(ord_),'id':H(pre)[:32]})
    assert [(r['kind'],int(r['ordinal'])) for r in leaves]==[(1,1),(1,3),(2,1)]
    assert len({r['id'] for r in issuance})==18 and issuance[4]['id']!='00'*16
    root_negatives=[]
    for k,value in [('id','00'*16),('parent',rid),('fork_revision','00'*32),('fork_tick','1'),('fork_state_digest','00'*32),('ordinal','1'),('head','00'*32),('tip_tick','-1')]:
        b=copy.deepcopy(branch);b[k]=value;errors=root_errors(b,rid,sd);assert errors
        root_negatives.append({'id':'ROOT-L-negative-'+k,'scope':'root_branch_diagnostic_leaf','record':b,'canonical_hex':C(b).hex(),'expected':'reject_root_contract','expected_error':None,'diagnostics':errors})
    return {'status':'PROPOSED_ZERO_RECORD_FAMILY','source_sha256':dep['source_sha256'],'catalog_dependency':{'file':str(CATALOG_FILE.relative_to(SPECS)),'sha256':CATALOG_FREEZE,'canonical_hex':cat_bytes.hex(),'pack_hash':ch},'fixture_choices':{'seed':seed,'generator':generator},'state':s,'state_wrapper':wrap,'genesis':gen,'root_branch':branch,'history_view':view,'allocator_marks':marks,'queue_harness':{'pending_commands':0,'pending_tail':'0','claimed_portable_queue_schema':False},'frames':[frame('G',g),frame('STATE-canonical',C(s)),frame('STATE-wrapper',C(wrap)),frame('STATE',state_pre),frame('GENESIS-canonical',C(gen)),frame('GENESIS',gen_pre),frame('ROOT_BRANCH',root_pre),frame('root-Branch-canonical',C(branch)),frame('HistoryView-canonical',C(view)),frame('allocator-marks-diagnostic-canonical',C(marks))],'state_digest':sd,'genesis_digest':gd,'root_full_digest':H(root_pre),'root_id':rid,'cases':cases,'issuance_leaf_frames':issuance,'noncontiguous_roster_leaf':{'scope':'positive_sorted_kind_local_ordinal_grammar_and_frames_only_not_admitted_genesis','rows':leaves,'canonical_hex':C(leaves).hex(),'frames':[frame('kind-'+str(r['kind'])+'-ordinal-'+r['ordinal'],D('GENESIS_ENTITY')+g+b'\x03'+struct.pack('<HQ',r['kind'],int(r['ordinal']))) for r in leaves]},'root_diagnostic_negatives':root_negatives}

def coverage(c):return {'status':'Partial','family':'zero_record_state_genesis_root','counts':{'document_cases':len(c['cases']),'negative_documents':sum(x['expected']=='reject' for x in c['cases']),'zero_state_genesis_positives':2,'header_width_structural_positive':1,'unsupported_family_examples':1,'issuance_leaf_frames':18,'noncontiguous_roster_leaf':1,'root_diagnostic_negatives':8,'literal_frames':len(c['frames'])},'covered':[{'sections':'complete-schema §§Scalar grammar; Capacities and exact keys; Identity, kind tags and sequence consumption; History schemas; wire §§Canonical state layout, Identity framing','strength':'all applicable zero-record field/order/scalar/reference/empty-roster/genesis/root predicates; literal framing; full-range header scalar shape separate from replay admission'}],'missing':['Nonempty State families return outside_family, never fabricated invalid-wire errors; row schemas/history/initial observations/derived facts not validated.','All18 issuance frames and noncontiguous positive roster are leaf diagnostics, not full allocation-to-live/reserved Genesis correspondence.','No admitted Archive/Notebook/history/collision scanner or allocator abandonment/queue/replay proof.','Root Branch/HistoryView/allocator bytes are diagnostic records, not standalone portable envelopes.','Inhabited State and initial paid SnapshotReference provenance cycle unresolved.','No actor/forecast/scoring/mechanics/full six-gap closure or runtime/platform/budget/cancellation proof.','No exhaustive malformed-input matrix or universal failure precedence; diagnostics are local predicates except explicit raw byte-cap-first error.'],'source_sha256':c['source_sha256'],'catalog_dependency_sha256':CATALOG_FREEZE}
def encoded(x):return (json.dumps(x,ensure_ascii=False,indent=2)+'\n').encode()
def main():
    c=corpus();outputs={'generatedvectors.json':encoded(c),'coverage.json':encoded(coverage(c))}
    mode=sys.argv[1] if len(sys.argv)==2 else ''
    if mode=='generate':
        for f,b in outputs.items():(HERE/f).write_bytes(b)
    elif mode=='verify':
        for f,b in outputs.items():assert (HERE/f).read_bytes()==b,'generated_drift: '+f
    else:raise SystemExit('usage: derive.py generate|verify')
    print(mode+': '+str(len(c['cases']))+' documents,18 issuance leaves,8 root negatives; root '+c['root_id'])
if __name__=='__main__':main()
