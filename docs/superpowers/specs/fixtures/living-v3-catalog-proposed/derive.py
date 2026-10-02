#!/usr/bin/env python3
"""Independent Proposed exact-core Catalog derivation. No runtime imports."""
import copy, hashlib, json, re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
SPECS = ROOT / 'docs/superpowers/specs'
FILES = ['2026-10-01-living-'+s+'.md' for s in ['expansion-architecture','societies-rules','ecology-rules','diplomacy-expansion','expansion-wire','expansion-experience','v3-complete-schema']] + ['2026-10-02-living-v3-scoring.md']
FROZEN = ['539009046173d4e6ffced1c7c3b79f1eaf977860d3bdfcd2e171fad76d630615','aa221fd432f193db4eedade9b690f4156bee264525ac58b5595a679ce9a02e71','003659ee5eacd9b952344066eb6024964b630273505f598a86a6b3dfd3287fc1','045ebab7aed37ab81848dfc4e9225cfb43434f4518926b305397b468e7395747','1a8ff87d275e4f5e4a7e620179237f41339e64541c9942af3100374dfac70904','66cf1a76ae3111a7d7007892b180f9267f94d4fc5adcffd69b77e78ed9ab7cc4','9a05423608640515a6797fc3e73167a7213266d2e790f4d47ee73e0589996d73','c8bd14e773cc59fc8c9f7eb78b1eea6b5a32f648c6955b00ffe757f34c34976d']
def sha(b): return hashlib.sha256(b).hexdigest()
def canonical(x): return json.dumps(x, ensure_ascii=False, separators=(',', ':'), allow_nan=False).encode('utf-8')
def row(**kw): return kw
def amount(r,q): return row(resource=r,quantity=q)
def recipe(p,d,i,o): return row(type='recipe',period_ticks=p,deposit_units=d,inputs=[amount(*a) for a in i],outputs=[amount(*a) for a in o])
def industry(i,n,c,t,m,e,w,s): return row(id=i,name=n,alloy_cost=c,build_ticks=t,slot_cost=1,max_per_colony=m,effect=e,workforce_units=w,stress_per_operation=s)
def additions(c):
    c['kind']='NYON_LIVING_PACK_V3'; c['format_version']=3; c['rules_version']=3
    c['resources'].append(row(id='food',name='Food',storage_limit=10000)); c['resources'].sort(key=lambda r:r['id'])
    for i in c['industry_definitions']:
        i['workforce_units']=0 if i['id']=='defense_battery' else 1
        i['stress_per_operation']={'extractor':1,'foundry':2}.get(i['id'],0)
    c['industry_definitions'] += [industry('farm','Farm',30,150,8,recipe(10,0,[('energy',2)],[('food',20)]),1,0),industry('habitation','Habitation',30,150,8,row(type='habitation',base_capacity=1000),0,0),industry('research_lab','Research Lab',60,300,8,row(type='research',period_ticks=20,inputs=[amount('alloy',1),amount('energy',2)],points=1),1,0)]
    c['industry_definitions'].sort(key=lambda r:r['id'])
    for h in c['hull_definitions']: h['role']={'colony_ark':'ark','escort':'escort','scout':'scout'}[h['id']]
    c['technology_definitions']=[row(id='agronomy',name='Agronomy',prerequisites=[],point_cost=100,effect=row(type='farm_base_output',units=25)),row(id='efficient_logistics',name='Efficient Logistics',prerequisites=[],point_cost=100,effect=row(type='departure_energy_scale',numerator=3,denominator=4)),row(id='resilient_habitation',name='Resilient Habitation',prerequisites=['agronomy'],point_cost=150,effect=row(type='habitability_penalty_scale',numerator=1,denominator=2))]
    return c

def snapshot_core():
    text=(SPECS/FILES[6]).read_text()
    snapshot=json.loads(text.rsplit('```json\n',1)[1].split('\n```',1)[0])
    return additions(snapshot)

def table_core():
    # Independent literal transcription of schema documentary initial snapshot.
    return row(kind='NYON_LIVING_PACK_V3',format_version=3,rules_version=3,pack_id='nyon_living_core',pack_version=1,
      star_archetypes=[row(id='main_sequence',name='Main Sequence'),row(id='red_dwarf',name='Red Dwarf')],
      world_archetypes=[row(id='barren',name='Barren'),row(id='temperate',name='Temperate')],
      resources=[row(id=r,name=r.title(),storage_limit=10000) for r in ['alloy','energy','food','ore']],
      industry_definitions=[industry('defense_battery','Defense Battery',40,200,1,row(type='defense',hit_points=40,damage_per_round=4),0,0),industry('extractor','Extractor',30,150,6,recipe(10,4,[('energy',2)],[('ore',4)]),1,1),industry('farm','Farm',30,150,8,recipe(10,0,[('energy',2)],[('food',20)]),1,0),industry('foundry','Foundry',40,200,6,recipe(20,0,[('energy',4),('ore',4)],[('alloy',2)]),1,2),industry('habitation','Habitation',30,150,8,row(type='habitation',base_capacity=1000),0,0),industry('research_lab','Research Lab',60,300,8,row(type='research',period_ticks=20,inputs=[amount('alloy',1),amount('energy',2)],points=1),1,0),industry('shipyard','Shipyard',60,300,1,row(type='hull_assembly',concurrent_jobs=1),1,0),industry('solar_array','Solar Array',20,100,6,recipe(10,0,[],[('energy',8)]),1,0)],
      hull_definitions=[row(id=i,name=n,alloy_cost=c,build_ticks=t,hit_points=h,damage=d,role=r) for i,n,c,t,h,d,r in [('colony_ark','Colony Ark',60,200,20,0,'ark'),('escort','Escort',20,100,10,2,'escort'),('scout','Scout',10,50,10,0,'scout')]],
      hazard_definitions=[row(id='ion_storm',name='Ion Storm',effect=row(type='freight_batch_scale',numerator=1,denominator=2))],
      policy_definitions=[row(id=i,name=i.title(),bonuses=[row(intent=t,bonus=20) for t in ts]) for i,ts in [('expansionist',['build_ark','settle']),('guardian',['build_battery','build_escort','defend']),('industrialist',['build_extractor','build_foundry']),('neutral',[]),('trader',['establish_trade_route','supply'])]],
      technology_definitions=[row(id='agronomy',name='Agronomy',prerequisites=[],point_cost=100,effect=row(type='farm_base_output',units=25)),row(id='efficient_logistics',name='Efficient Logistics',prerequisites=[],point_cost=100,effect=row(type='departure_energy_scale',numerator=3,denominator=4)),row(id='resilient_habitation',name='Resilient Habitation',prerequisites=['agronomy'],point_cost=150,effect=row(type='habitability_penalty_scale',numerator=1,denominator=2))])

INTENTS='build_ark build_battery build_escort build_extractor build_foundry build_scout build_shipyard build_solar_array defend establish_trade_route settle supply build_farm build_habitation build_research_lab select_research request_supply negotiate_access negotiate_supply negotiate_defense blockade adaptation'.split()
IND={'recipe':['period_ticks','deposit_units','inputs','outputs'],'hull_assembly':['concurrent_jobs'],'defense':['hit_points','damage_per_round'],'habitation':['base_capacity'],'research':['period_ticks','inputs','points']}
HAZ={'freight_batch_scale':['numerator','denominator']}
TECH={'farm_base_output':['units'],'habitability_penalty_scale':['numerator','denominator'],'departure_energy_scale':['numerator','denominator']}
SHAPES={'star_archetypes':['id','name'],'world_archetypes':['id','name'],'resources':['id','name','storage_limit'],'industry_definitions':['id','name','alloy_cost','build_ticks','slot_cost','max_per_colony','effect','workforce_units','stress_per_operation'],'hull_definitions':['id','name','alloy_cost','build_ticks','hit_points','damage','role'],'hazard_definitions':['id','name','effect'],'policy_definitions':['id','name','bonuses'],'technology_definitions':['id','name','prerequisites','point_cost','effect']}
CAPS=dict(zip(SHAPES,[64,64,4,64,32,32,32,64]))
TOP=['kind','format_version','rules_version','pack_id','pack_version']+list(SHAPES)
def name_ok(v): return isinstance(v,str) and 1<=len(v)<=64 and len(v.encode('utf-8'))<=256 and not any(ord(c)<32 or 127<=ord(c)<=159 or 0xD800<=ord(c)<=0xDFFF for c in v) and not v.startswith(' ') and not v.endswith(' ') and '  ' not in v
def slug_ok(v): return isinstance(v,str) and re.fullmatch('[a-z][a-z0-9_]{0,63}',v) is not None

def validate(raw):
    """Exact-core admission plus independently collected diagnostics; no total error ordering."""
    if len(raw)>1048576: return [{'path':'$', 'predicate':'byte_capacity', 'normative_error':'byte_capacity'}]
    defects=[]
    def bad(p,c): defects.append({'path':p,'predicate':c})
    def pairs(items):
        d={}
        for k,v in items:
            if k in d: bad('$','duplicate_key')
            d[k]=v
        return d
    # Scanner bounds nesting BEFORE the general JSON parser, respecting strings/escapes.
    depth=0; quoted=False; escaped=False
    for b in raw:
        if quoted:
            if escaped: escaped=False
            elif b==92: escaped=True
            elif b==34: quoted=False
        elif b==34: quoted=True
        elif b in (91,123):
            depth+=1
            if depth>64: bad('$','depth_capacity'); break
        elif b in (93,125): depth-=1
    if any(d['predicate']=='depth_capacity' for d in defects): return defects
    try: x=json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,parse_constant=lambda s: (_ for _ in ()).throw(ValueError(s)))
    except (UnicodeError,ValueError,RecursionError): bad('$','invalid_json_or_utf8'); return defects
    try:
        if canonical(x)!=raw: bad('$','noncanonical_bytes')
    except (ValueError,UnicodeError): bad('$','noncanonical_bytes'); return defects
    def shape(v,keys,p):
        if not isinstance(v,dict): bad(p,'record_shape'); return False
        if list(v)!=keys: bad(p,'closed_fields_or_order')
        return all(k in v for k in keys)
    def uint(v,p,positive=False):
        if type(v) is not int or not 0<=v<=4294967295 or (positive and v==0): bad(p,'u32_positive' if positive else 'u32')
    def sorted_rows(v,key,p,cap=None):
        if not isinstance(v,list): bad(p,'array'); return []
        if cap is not None and len(v)>cap: bad(p,'record_capacity')
        keys=[r.get(key) if isinstance(r,dict) else None for r in v]
        if not all(isinstance(k,str) for k in keys) or any(a>=b for a,b in zip(keys,keys[1:]) if isinstance(a,str) and isinstance(b,str)): bad(p,'strict_sorted_unique')
        return v
    if not shape(x,TOP,'$'): return defects
    for k,v in [('kind','NYON_LIVING_PACK_V3'),('format_version',3),('rules_version',3)]:
        if x[k]!=v or (k!='kind' and type(x[k]) is not int): bad('$.'+k,'fixed_envelope')
    if not slug_ok(x['pack_id']): bad('$.pack_id','slug')
    if type(x['pack_version']) is not int or x['pack_version']<=0: bad('$.pack_version','positive_pack_version_width_unbound')
    refs=set(r.get('id') for r in x['resources'] if isinstance(r,dict) and isinstance(r.get('id'),str)) if isinstance(x['resources'],list) else set()
    def amounts(v,p):
        for j,a in enumerate(sorted_rows(v,'resource',p,4)):
            q=f'{p}[{j}]'
            if shape(a,['resource','quantity'],q):
                if not slug_ok(a['resource']): bad(q+'.resource','slug')
                if not isinstance(a['resource'],str) or a['resource'] not in refs: bad(q+'.resource','missing_reference')
                uint(a['quantity'],q+'.quantity')
    def effect(v,registry,p):
        if not isinstance(v,dict) or not isinstance(v.get('type'),str) or v.get('type') not in registry: bad(p,'invalid_enum'); return
        fields=registry[v['type']]
        if not shape(v,['type']+fields,p): return
        for k in fields:
            if k in ('inputs','outputs'): amounts(v[k],p+'.'+k)
            else: uint(v[k],p+'.'+k,k in ('period_ticks','concurrent_jobs','base_capacity','points'))
    for group,keys in SHAPES.items():
        for j,r in enumerate(sorted_rows(x[group],'id','$.'+group,CAPS[group])):
            p=f'$.{group}[{j}]'
            if not shape(r,keys,p): continue
            if not slug_ok(r['id']): bad(p+'.id','slug')
            if not name_ok(r['name']): bad(p+'.name','name')
            for k in keys:
                if k not in ('id','name','effect','role','bonuses','prerequisites'): uint(r[k],p+'.'+k,k in ('alloy_cost','build_ticks','slot_cost','max_per_colony','point_cost'))
            if group=='industry_definitions': effect(r['effect'],IND,p+'.effect')
            elif group=='hazard_definitions': effect(r['effect'],HAZ,p+'.effect')
            elif group=='technology_definitions':
                effect(r['effect'],TECH,p+'.effect')
                v=r['prerequisites']
                if not isinstance(v,list) or not all(slug_ok(a) for a in v): bad(p+'.prerequisites','slug_array')
                elif any(a>=b for a,b in zip(v,v[1:])): bad(p+'.prerequisites','strict_sorted_unique')
            elif group=='hull_definitions' and r['role'] not in ('ark','escort','scout'): bad(p+'.role','invalid_enum')
            elif group=='policy_definitions':
                for q,b in enumerate(sorted_rows(r['bonuses'],'intent',p+'.bonuses')):
                    bp=f'{p}.bonuses[{q}]'
                    if shape(b,['intent','bonus'],bp):
                        if b['intent'] not in INTENTS: bad(bp+'.intent','invalid_enum')
                        uint(b['bonus'],bp+'.bonus')
    if refs!={'alloy','energy','food','ore'}: bad('$.resources','fixed_resource_roster')
    ts={r['id']:r for r in x['technology_definitions'] if isinstance(r,dict) and isinstance(r.get('id'),str)} if isinstance(x['technology_definitions'],list) else {}
    visiting=set(); done=set()
    def visit(k):
        if k in visiting: bad('$.technology_definitions','invalid_graph'); return
        if k in done: return
        visiting.add(k)
        for dep in ts[k].get('prerequisites',[]) if isinstance(ts[k].get('prerequisites'),list) else []:
            if isinstance(dep,str):
                if dep not in ts: bad('$.technology_definitions.'+k+'.prerequisites','missing_reference')
                else: visit(dep)
        visiting.remove(k); done.add(k)
    if len(ts)<=64:
        for k in ts: visit(k)
    # Exact core is the ONLY admitted catalog in this family. This comparison is
    # after the recursive predicates, never a replacement for nested validation.
    expected=table_core()
    def compare(a,b,p):
        if type(a) is not type(b): bad(p,'exact_core_value'); return
        if isinstance(b,dict):
            for k in b:
                if k in a: compare(a[k],b[k],p+'.'+k)
        elif isinstance(b,list):
            if len(a)!=len(b): bad(p,'exact_core_roster')
            for j,(av,bv) in enumerate(zip(a,b)): compare(av,bv,f'{p}[{j}]')
        elif a!=b: bad(p,'exact_core_value')
    compare(x,expected,'$')
    return defects

def corpus():
    freeze={f:sha((SPECS/f).read_bytes()) for f in FILES}
    assert list(freeze.values())==FROZEN,'source_freeze_changed'
    core=table_core(); raw=canonical(core)
    assert raw==canonical(snapshot_core()),'independent_table_snapshot_disagreement'
    assert not validate(raw),validate(raw)
    cases=[]
    def add(label,b,accept=False):
        assert all(case['id']!=label for case in cases),'duplicate_case_id: '+label
        ds=validate(b); assert (not ds)==accept,(label,ds)
        cases.append(row(id=label,scope='exact_core_catalog',expected='admit' if accept else 'reject',expected_error='byte_capacity' if len(b)>1048576 else None,raw_hex=b.hex(),byte_length=len(b),diagnostics=ds))
    add('CAT-P01-exact-core',raw,True)
    def mutate(label,fn):
        c=copy.deepcopy(core); fn(c); add(label,canonical(c))
    for k,v in [('kind','other'),('format_version',2),('rules_version',2),('pack_version',0)]: mutate('CAT-N-'+k,lambda c,k=k,v=v:c.__setitem__(k,v))
    mutate('CAT-N-top-extra',lambda c:c.__setitem__('extra',0))
    mutate('CAT-N-top-missing',lambda c:c.pop('pack_id'))
    add('CAT-N-record-field-order',canonical({**{'pack_id':core['pack_id']},**core}))
    for label,b in [('duplicate-key',raw.replace(b'"pack_version":1',b'"pack_version":1,"pack_version":1',1)),('whitespace',b' '+raw),('newline',raw+b'\n'),('unicode-escape',raw.replace(b'Alloy',b'\\u0041lloy',1)),('invalid-utf8',b'\xff'),('invalid-json',b'{'),('leading-zero',raw.replace(b'"storage_limit":10000',b'"storage_limit":010000',1)),('exponent',raw.replace(b'"storage_limit":10000',b'"storage_limit":1e4',1)),('signed-positive',raw.replace(b'"storage_limit":10000',b'"storage_limit":+10000',1))]: add('CAT-N-'+label,b)
    for v in [1.5,True,'10000',None,-1,4294967296]: mutate('CAT-N-u32-'+str(v),lambda c,v=v:c['resources'][0].__setitem__('storage_limit',v))
    for i,v in enumerate(['','Upper','é','a'*65]): mutate('CAT-N-slug-'+str(i),lambda c,v=v:c.__setitem__('pack_id',v))
    for i,v in enumerate(['','a'*65,'\x00','\x7f','\x9f',' x','x ','x  y']): mutate('CAT-N-name-'+str(i),lambda c,v=v:c['resources'][0].__setitem__('name',v))
    for group in SHAPES:
        mutate('CAT-N-'+group+'-missing-field',lambda c,g=group:c[g][0].pop('name'))
        mutate('CAT-N-'+group+'-extra-field',lambda c,g=group:c[g][0].__setitem__('extra',0))
        mutate('CAT-N-'+group+'-duplicate',lambda c,g=group:c[g].insert(0,copy.deepcopy(c[g][0])))
        if len(core[group])>1: mutate('CAT-N-'+group+'-reorder',lambda c,g=group:c[g].reverse())
        def over(c,g=group):
            template=c[g][0]; c[g]=[dict(template,id='z'+str(j).zfill(3)) for j in range(CAPS[g]+1)]
        mutate('CAT-N-'+group+'-cap-plus-one',over)
    for group in ['industry_definitions','hazard_definitions','technology_definitions']:
        for j,r in enumerate(core[group]):
            tag=r['effect']['type']
            # One witness per effect variant (recipe appears several times).
            if any(v['effect']['type']==tag for v in core[group][:j]): continue
            for op in ('unknown-tag','missing-payload','extra-payload','wrong-payload'):
                def edit(c,g=group,j=j,op=op):
                    e=c[g][j]['effect']; field=list(e)[1]
                    if op=='unknown-tag': e['type']='unknown'
                    elif op=='missing-payload': e.pop(field)
                    elif op=='extra-payload': e['extra']=0
                    else: e[field]={}
                mutate('CAT-N-'+tag+'-'+op,edit)
    for j in range(3): mutate('CAT-N-role-'+str(j),lambda c,j=j:c['hull_definitions'][j].__setitem__('role','unknown'))
    mutate('CAT-N-intent-unknown',lambda c:c['policy_definitions'][0]['bonuses'][0].__setitem__('intent','unknown'))
    mutate('CAT-N-bonus-order',lambda c:c['policy_definitions'][0]['bonuses'].reverse())
    mutate('CAT-N-bonus-duplicate',lambda c:c['policy_definitions'][0]['bonuses'].append(copy.deepcopy(c['policy_definitions'][0]['bonuses'][0])))
    mutate('CAT-N-bonus-extra',lambda c:c['policy_definitions'][0]['bonuses'][0].__setitem__('extra',0))
    mutate('CAT-N-bonus-missing',lambda c:c['policy_definitions'][0]['bonuses'][0].pop('bonus'))
    mutate('CAT-N-amount-order',lambda c:c['industry_definitions'][3]['effect']['inputs'].reverse())
    mutate('CAT-N-amount-duplicate',lambda c:c['industry_definitions'][3]['effect']['inputs'].append(copy.deepcopy(c['industry_definitions'][3]['effect']['inputs'][0])))
    mutate('CAT-N-amount-extra',lambda c:c['industry_definitions'][1]['effect']['inputs'][0].__setitem__('extra',0))
    mutate('CAT-N-amount-missing',lambda c:c['industry_definitions'][1]['effect']['inputs'][0].pop('quantity'))
    mutate('CAT-N-resource-reference',lambda c:c['industry_definitions'][1]['effect']['inputs'][0].__setitem__('resource','missing'))
    mutate('CAT-N-resource-food-omitted',lambda c:c['resources'].pop(2))
    mutate('CAT-N-prerequisite-reference',lambda c:c['technology_definitions'][0].__setitem__('prerequisites',['missing']))
    mutate('CAT-N-prerequisite-self',lambda c:c['technology_definitions'][0].__setitem__('prerequisites',['agronomy']))
    mutate('CAT-N-prerequisite-cycle',lambda c:c['technology_definitions'][0].__setitem__('prerequisites',['resilient_habitation']))
    mutate('CAT-N-prerequisite-order',lambda c:c['technology_definitions'][2].__setitem__('prerequisites',['efficient_logistics','agronomy']))
    mutate('CAT-N-prerequisite-duplicate',lambda c:c['technology_definitions'][2].__setitem__('prerequisites',['agronomy','agronomy']))
    for g,j,k in [('industry_definitions',1,'alloy_cost'),('industry_definitions',1,'build_ticks'),('industry_definitions',1,'slot_cost'),('industry_definitions',1,'max_per_colony'),('industry_definitions',1,'workforce_units'),('industry_definitions',1,'stress_per_operation'),('hull_definitions',0,'hit_points'),('hull_definitions',1,'damage'),('technology_definitions',0,'point_cost')]: mutate('CAT-N-fixed-'+g+'-'+k,lambda c,g=g,j=j,k=k:c[g][j].__setitem__(k,c[g][j][k]+1))
    for g,j,k in [('industry_definitions',1,'period_ticks'),('industry_definitions',5,'period_ticks'),('technology_definitions',0,'units'),('technology_definitions',1,'denominator'),('hazard_definitions',0,'denominator')]: mutate('CAT-N-zero-'+g+'-'+str(j)+'-'+k,lambda c,g=g,j=j,k=k:c[g][j]['effect'].__setitem__(k,0))
    mutate('CAT-N-amount-cap-plus-one',lambda c:c['industry_definitions'][1]['effect'].__setitem__('inputs',[amount('z'+str(j),1) for j in range(5)]))
    mutate('CAT-N-resource-id-nonscalar',lambda c:c['resources'][0].__setitem__('id',[]))
    mutate('CAT-N-effect-tag-nonscalar',lambda c:c['industry_definitions'][0]['effect'].__setitem__('type',[]))
    mutate('CAT-N-prerequisite-nonscalar',lambda c:c['technology_definitions'][0].__setitem__('prerequisites',[[]]))
    for label,value in [('array',[]),('record',{}),('boolean',True),('null',None)]:
        mutate('CAT-N-amount-resource-'+label,lambda c,value=value:c['industry_definitions'][1]['effect']['inputs'][0].__setitem__('resource',value))
    for label,value in [('record',{}),('null',None),('integer',1)]:
        mutate('CAT-N-input-container-'+label,lambda c,value=value:c['industry_definitions'][1]['effect'].__setitem__('inputs',value))
        mutate('CAT-N-amount-record-'+label,lambda c,value=value:c['industry_definitions'][1]['effect'].__setitem__('inputs',[value]))
    add('CAT-N-depth-65',b'['*65+b'0'+b']'*65)
    add('CAT-N-byte-cap-malformed-utf8',b'\xff'+b' '*(1048576))
    leaves=[]
    for label,v in [('quote-backslash','a"b\\c'),('non-ascii','Étoile'),('scalar-64','a'*64),('supplementary-256-bytes','😀'*64),('composed','é'),('decomposed','e\u0301')]:
        assert name_ok(v); leaves.append(row(id='CAT-L-name-'+label,scope='canonical_name_leaf_only',value=v,canonical_hex=canonical(v).hex()))
    for v in ['a','a'*64]: assert slug_ok(v); leaves.append(row(id='CAT-L-slug-'+str(len(v)),scope='slug_leaf_only',value=v))
    for tag in INTENTS: leaves.append(row(id='CAT-L-intent-'+tag,scope='policy_intent_leaf_only',value=tag))
    changed=copy.deepcopy(core); changed['industry_definitions'][1]['alloy_cost']=31
    changed_pre=b'NYON_LIVING_V3_PACK\x00'+canonical(changed)
    pre=b'NYON_LIVING_V3_PACK\x00'+raw
    return row(status='Proposed',source_sha256=freeze,catalog=core,canonical_hex=raw.hex(),byte_length=len(raw),pack_preimage_hex=pre.hex(),pack_sha256=sha(pre),independent_snapshot_reconstruction_equal=True,cases=cases,leaf_cases=leaves,hash_sensitivity=[row(id='PACK-wellformed-changed-cost',preimage_hex=changed_pre.hex(),sha256=sha(changed_pre),note='Different digest, not standalone integrity_mismatch; changed balance rejects separately.'),row(id='PACK-domain-missing-nul',preimage_hex=(b'NYON_LIVING_V3_PACK'+raw).hex(),sha256=sha(b'NYON_LIVING_V3_PACK'+raw)),row(id='PACK-cross-domain',preimage_hex=(b'NYON_LIVING_V3_STATE\x00'+raw).hex(),sha256=sha(b'NYON_LIVING_V3_STATE\x00'+raw)),row(id='PACK-changed-byte',preimage_hex=(pre+b' ').hex(),sha256=sha(pre+b' '))])

def coverage(c):
    return row(status='Partial',family='exact_frozen_core_catalog',admitted_catalogs=1,reconstruction_checks=1,rejected_documents=len(c['cases'])-1,leaf_positives=len(c['leaf_cases']),hash_sensitivity_cases=4,covered=[row(section='complete-schema §§Scalar grammar; Catalog schema and definition registry; Documentary initial catalog snapshot; Hash framing',scope='recursive closed shapes and all declared Catalog effect/role variants, fixed values, raw canonical grammar, graph/references/caps, exact PACK'),row(section='societies-rules §§Added catalog definitions; Research',scope='fixed added industry/technology values')],missing=['No admitted custom metadata/balance/definition roster variants: permissible variation unspecified.','pack_version width is not specified; no width-edge acceptance.','General fractional effects beyond initial fixed values are unspecified.','No total failure precedence or exhaustive leaf-to-error mapping except raw byte cap first.','Malformed nested shapes/scalars are sampled, not an exhaustive general Catalog decoder proof.','No full State/Genesis/history/actor/admission/scoring/mechanics fixtures; full gap6 Partial.','Two reconstructions share canonical encoder and additions literals; external independent formula review remains required.'],source_sha256=c['source_sha256'])
def encoded(x): return (json.dumps(x,ensure_ascii=False,indent=2)+'\n').encode('utf-8')
def main():
    c=corpus(); outputs={'generatedvectors.json':encoded(c),'coverage.json':encoded(coverage(c))}
    mode=sys.argv[1] if len(sys.argv)>1 else ''
    if mode=='generate':
        for f,b in outputs.items(): (HERE/f).write_bytes(b)
    elif mode=='verify':
        for f,b in outputs.items(): assert (HERE/f).read_bytes()==b,f+' mismatch'
    else: raise SystemExit('usage: derive.py generate|verify')
    print(mode+': 1 admitted Catalog; '+str(len(c['cases'])-1)+' rejected documents; '+str(len(c['leaf_cases']))+' leaf positives; PACK '+c['pack_sha256'])
if __name__=='__main__': main()
