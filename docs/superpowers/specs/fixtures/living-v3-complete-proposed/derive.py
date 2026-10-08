#!/usr/bin/env python3
"""Generate or read-only verify the Proposed foundation. No Rust/runtime input."""
import argparse
import base64
import copy
import hashlib
import json
import tempfile
from pathlib import Path
import catalog as c
import genesis as g
import profiles
import provenance as p
import predicate_matrix
import visibility_cases
import fix1_cases

HERE = Path(__file__).resolve().parent
SCHEMA = '2026-10-01-living-v3-complete-schema.md'

def dump(value):
    return (json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode('utf8')

def add_case(cases, name, raw, expected, clause, catalog, strength='exact_stage_error_path'):
    actual = g.outcome(raw,catalog)
    for key,value in expected.items():
        if actual.get(key) != value: raise AssertionError((name,expected,actual))
    if actual.get('coverage'):
        raise AssertionError(('Unsupported is not positive or rejection evidence',name))
    cases.append(dict(id=name,input_base64=base64.b64encode(raw).decode(),input_identity=p.identity(raw),
                      expected=expected,strength=strength,source=SCHEMA+':'+clause,
                      derivation='genesis.validate / profiles.build',independent_review='pending'))

def negative(cases,doc,catalog,name,path,value=None,*,delete=False,expected=None,clause='4197-4216'):
    bad=g.mutation(doc,path,value,delete)
    add_case(cases,name,c.encoded(bad),expected or {'admitted':False},clause,catalog,
             'exact_stage_error_path' if expected and 'path' in expected else 'rejection_only')

def genesis_cases(catalog):
    cases=[]; positive={}; outputs={}
    configs=[('empty',None),('inhabited',{}),
       ('three_systems',dict(systems=3,stars=3,worlds=3,lanes=3)),
       ('no_stars',dict(stars=0)),('no_deposits',dict(deposits=0)),('no_fleets',dict(fleets=0)),
       ('dormant_colony_absent',dict(colonies=0)),
       ('many_hulls',dict(hulls_per_fleet=16)),('multiple_actors',dict(civilizations=3,worlds=6,colonies=3,fleets=3)),
       ('noncontiguous_ordinals',dict(ordinal_start=400)),('unicode_max',dict(name='𐐀'*64)),
       ('unicode_replacement',dict(name='�')),('unicode_no_normalization',dict(name='é')),
       ('generator_max',dict(generator_id='a'+'x'*63,generator_version=2**32-1,seed='ff'*32)),
       ('generator_zero_version',dict(generator_version=0)),
       ('systems_cap64',dict(systems=64,stars=64,worlds=64,lanes=63)),
       ('stars_cap128',dict(stars=128)),('worlds_cap512',dict(worlds=512)),
       ('civs_cap16',dict(civilizations=16,worlds=16,colonies=16,fleets=16)),
       ('observations_cap8192',dict(systems=1,stars=1,worlds=512,civilizations=16,colonies=16,lanes=0,fleets=0)),
       ('deposits_cap1024',dict(deposits=1024)),('colonies_cap512',dict(worlds=512,colonies=512)),
       ('lanes_cap256',dict(systems=24,stars=24,worlds=24,lanes=256)),
       ('fleets_cap256',dict(fleets=256)),('hulls_global_cap2048',dict(fleets=128,hulls_per_fleet=16)),
       ('deposit_u64_max',dict(deposits=1,amount=g.U64)),('deposit_zero',dict(amount=0))]
    configs.extend(('population_'+str(n),dict(population=n)) for n in (0,1,99,100,199,200,10000))
    configs.extend(('deposit_'+r,dict(resource=r)) for r in ('alloy','energy','food','ore'))
    for name,kw in configs:
        doc=g.envelope() if kw is None else profiles.build(catalog=catalog,**kw)
        raw=c.encoded(doc)
        add_case(cases,'positive.'+name,raw,{'admitted':True},'4088-4143',catalog,'complete_bootstrap_admission')
        positive[name]=doc
        outputs['documents/'+name+'.genesis.json']=raw
        outputs['documents/'+name+'.state-wrapper.json']=c.encoded(dict(format_version=3,rules_version=3,catalog_hash=doc['catalog_hash'],state=doc['state']))
    full=positive['inhabited']
    # General metadata is linked through every fresh ID/root, never patched post-hash.
    metadata=copy.deepcopy(catalog)
    metadata['pack_id']='a'+'z'*63; metadata['pack_version']=2**32-1
    for collection in c.COLLECTIONS:
        for row in metadata[collection]: row['name']='Metadata Ω'
    assert c.admit(c.encoded(metadata))['admitted']
    doc=profiles.build(catalog=metadata,seed='ab'*32)
    add_case(cases,'positive.metadata_catalog_link',c.encoded(doc),{'admitted':True},'1689-1727;3742-3822',metadata,'complete_bootstrap_admission_with_own_catalog')
    cases[-1]['catalog_base64']=base64.b64encode(c.encoded(metadata)).decode()
    outputs['documents/metadata.catalog.json']=c.encoded(metadata)
    outputs['documents/metadata.genesis.json']=c.encoded(doc)
    outputs['documents/metadata.state-wrapper.json']=c.encoded(dict(format_version=3,rules_version=3,catalog_hash=doc['catalog_hash'],state=doc['state']))
    positive['metadata_catalog_link']=doc
    # Admitted variations retain exact initial facts after explicit producer projection.
    variations=[]
    for specialization in ('balanced','agrarian','industrial','research'):
        variations.append(('specialization_'+specialization,g.mutation(full,'/state/colonies/0/society/specialization',specialization)))
    for policy in ('expansionist','guardian','industrialist','neutral','trader'):
        variations.append(('policy_'+policy,g.mutation(full,'/state/civilizations/0/policy',policy)))
    for archetype in ('barren','temperate'):
        variations.append(('world_archetype_'+archetype,g.mutation(full,'/state/worlds/0/archetype',archetype)))
    for archetype in ('main_sequence','red_dwarf'):
        variations.append(('star_archetype_'+archetype,g.mutation(full,'/state/stars/0/archetype',archetype)))
    for endpoint in (0,10000):
        alt=copy.deepcopy(full)
        for world in alt['state']['worlds']:
            world['inventory']={r:endpoint for r in world['inventory']}
            for key in ('base_habitability','stress','fertility','maximum_fertility'): world['environment'][key]=endpoint
        alt['state']['observations']=g.project(alt['state'],catalog)
        variations.append(('local_domains_'+str(endpoint),alt))
    reverse=copy.deepcopy(full)
    lane=reverse['state']['lanes'][0];lane['system_a'],lane['system_b']=lane['system_b'],lane['system_a']
    variations.append(('distinct_lane_opposite_direction',reverse))
    own=profiles.build(catalog=catalog,fleets=0,colonies=0)
    own['state']['worlds'][0]['owner']=own['state']['civilizations'][0]['id']
    own['state']['observations']=g.project(own['state'],catalog)
    variations.append(('owned_world_without_colony',own))
    variations.append(('ordinal_u64_max',profiles.build(catalog=catalog,systems=1,stars=1,worlds=1,lanes=0,ordinal_start=g.U64)))
    for name,doc in variations:
        add_case(cases,'positive.'+name,c.encoded(doc),{'admitted':True},'4088-4143',catalog,'complete_bootstrap_admission')
        positive[name]=doc
        outputs['documents/'+name+'.genesis.json']=c.encoded(doc)
        outputs['documents/'+name+'.state-wrapper.json']=c.encoded(dict(format_version=3,rules_version=3,catalog_hash=doc['catalog_hash'],state=doc['state']))
    # Every required member in admitted populated input, including null and empty fields.
    for container in c.containers(full):
        path=container['path']
        if container['kind']=='record':
            for field in container['required_fields_in_order']:
                target=c.pointer(path,field)
                stage=4 if target in ('/kind','/format_version','/rules_version') else 5
                negative(cases,full,catalog,'missing'+target,target,delete=True,
                         expected=dict(admitted=False,stage=stage,error='noncanonical_bytes',path=target))
            bad=g.mutation(full,c.pointer(path,'z_extra'),0)
            bad=g.mutation(bad,c.pointer(path,'a_extra'),0)
            add_case(cases,'unknown'+path,c.encoded(bad),dict(admitted=False,stage=5,error='unknown_field',path=c.pointer(path,'a_extra')),'4208',catalog)
    # Semantic baseline supplies a valid witness for each mutated initial constraint.
    home=next(i for i,w in enumerate(full['state']['worlds']) if w['owner'])
    wp='/state/worlds/'+str(home)
    semantics=[('/state/tick','1'),('/state/branch_sequence','1'),('/state/next_sequence','1'),
       ('/state/stars/0/next_activity_tick','999'),(wp+'/inventory/food',10001),
       (wp+'/environment/base_habitability',10001),(wp+'/environment/stress',10001),
       (wp+'/environment/fertility',10001),(wp+'/environment/maximum_fertility',10001),
       (wp+'/environment/maximum_fertility',9999),(wp+'/environment/next_environment_tick','101'),
       ('/state/civilizations/0/status','dormant'),('/state/civilizations/0/research/target','agronomy'),
       ('/state/civilizations/0/research/progress',[dict(technology='agronomy',points=1)]),
       ('/state/civilizations/0/research/completed_technologies',['agronomy']),
       ('/state/colonies/0/hub/energy_next_due','11'),('/state/colonies/0/hub/ore_next_due','99'),
       ('/state/colonies/0/hub/fallback_next_due','0'),('/state/colonies/0/hub/blocked_reasons',[dict(type='disabled')]),
       ('/state/colonies/0/society/population',10001),('/state/colonies/0/society/next_needs_tick','99'),
       ('/state/colonies/0/society/workforce_assignments',[full['state']['fleets'][0]['id']]),
       ('/state/fleets/0/hulls',[]),('/state/fleets/0/hulls/0/hit_points',9),
       ('/state/fleets/0/hulls/0/return_credit',True),('/state/fleets/0/hulls/0/next_repair','1'),
       ('/state/fleets/0/hulls/0/passengers/count',1),('/state/fleets/0/hulls/0/passengers/voyage_origin',full['state']['worlds'][home]['id']),
       ('/state/fleets/0/supply/readiness','supplied'),('/allocations/0/ordinal','0'),
       ('/allocations/0/kind',0),('/allocations/0/kind',19),('/allocations',full['allocations'][:-1]),
       ('/state/observations',[])]
    for i,(path,value) in enumerate(semantics): negative(cases,full,catalog,'semantic.'+str(i),path,value,clause='4088-4143')
    # Derived facts are checked against independently computed B5/B6, not themselves.
    for path,value in list(c.leaves(full['state']['observations'])):
        target='/state/observations'+path
        if type(value) is int: replacement=value+1
        elif isinstance(value,str) and value.isdigit() and len(value)!=32: replacement=str(int(value)+1)
        elif isinstance(value,str) and len(value)==32: replacement='ff'*16
        else: continue
        negative(cases,full,catalog,'projection'+path,target,replacement,clause='4105-4124')
    for name,cap in zip(g.ARRAYS,g.LIMITS):
        negative(cases,full,catalog,'cap.'+name,'/state/'+name,[None]*(cap+1),expected=dict(admitted=False,stage=5,error='record_capacity',path='/state/'+name))
    negative(cases,full,catalog,'cap.hulls_per_fleet','/state/fleets/0/hulls',[None]*17,
             expected=dict(admitted=False,stage=5,error='record_capacity',path='/state/fleets/0/hulls'))
    over=profiles.build(catalog=catalog,fleets=129,hulls_per_fleet=16)
    add_case(cases,'cap.global_hulls',c.encoded(over),dict(admitted=False,stage=7,error='record_capacity',path='/state/fleets'),'46',catalog)
    for path,cap in [('/state/civilizations/0/research/progress',64),('/state/civilizations/0/research/completed_technologies',64),
       ('/state/colonies/0/society/workforce_assignments',8),('/state/observations/0/environment/stellar_scope/events',128),
       ('/state/observations/0/hazard_scopes',256),('/state/observations/0/hazard_scopes/0/events',128)]:
        negative(cases,full,catalog,'nested_cap'+path,path,[None]*(cap+1),expected=dict(admitted=False,stage=5,error='record_capacity',path=path))
    # Authentic accumulation boundary: one observed deposit U64 max passes, two overflow.
    overflow=profiles.build(catalog=catalog,deposits=2,worlds=1,amount=g.U64)
    for observation in overflow['state']['observations']: observation['remaining_deposit']='0'
    add_case(cases,'projection.deposit_sum_overflow',c.encoded(overflow),dict(admitted=False),'8-34;4105-4107',catalog,'rejection_only')
    # All base typed reference families: absence and a different actually installed kind.
    refpaths=[('/state/stars/0/system',full['state']['worlds'][0]['id']),('/state/worlds/0/system',full['state']['civilizations'][0]['id']),
              ('/state/deposits/0/world',full['state']['systems'][0]['id']),('/state/fleets/0/location/world',full['state']['systems'][0]['id']),
              ('/state/fleets/0/owner',full['state']['worlds'][0]['id'])]
    for path,foreign in refpaths:
        for tag,ident in [('missing_reference','ff'*16),('wrong_reference_kind',foreign)]:
            negative(cases,full,catalog,tag+path,path,ident,expected=dict(admitted=False,stage=8,error=tag),clause='4211')
    for path in ['/state/civilizations/0/policy','/state/fleets/0/hulls/0/definition','/state/stars/0/archetype','/state/worlds/0/archetype']:
        negative(cases,full,catalog,'catalog_reference'+path,path,'unknown',expected=dict(admitted=False,stage=8,error='missing_reference'),clause='4211')
    for key,code,value in [('kind','unsupported_kind','OTHER'),('format_version','unsupported_format_version',4),('rules_version','unsupported_rules_version',4)]:
        negative(cases,full,catalog,'header.'+key,'/'+key,value,expected=dict(admitted=False,stage=4,error=code,path='/'+key))
    for path in ('/seed','/generator/generator_id','/generator/generator_version'):
        value={'/seed':'01'*32,'/generator/generator_id':'changed','/generator/generator_version':2}[path]
        negative(cases,full,catalog,'integrity'+path,path,value,expected=dict(admitted=False,stage=9,error='integrity_mismatch'),clause='3742-3813')
    negative(cases,full,catalog,'catalog_mismatch','/catalog_hash','ff'*32,expected=dict(admitted=False,stage=8,error='catalog_mismatch',path='/catalog_hash'))
    duplicate=copy.deepcopy(full)
    duplicate['state']['fleets'][0]['hulls'][0]['id']=duplicate['state']['fleets'][0]['id']
    add_case(cases,'identity.nested_duplicate',c.encoded(duplicate),dict(admitted=False,stage=7,error='duplicate_identity'),'4210',catalog)
    for name in ('systems','stars','worlds'):
        negative(cases,full,catalog,'order.'+name,'/state/'+name,list(reversed(full['state'][name])),expected=dict(admitted=False,stage=5,error='noncanonical_bytes',path='/state/'+name+'/1'))
    negative(cases,full,catalog,'key.allocation_duplicate','/allocations',[full['allocations'][0]]+full['allocations'],expected=dict(admitted=False,stage=5,error='duplicate_key',path='/allocations/1'))
    # Parser diagnostics are explicit and do not depend on valid document semantics.
    malformed=[('utf8',b'\xff',1,'noncanonical_bytes'),('BOM',b'\xef\xbb\xbf{}',2,'noncanonical_bytes'),
       ('trailing',c.encoded(full)+b'x',2,'noncanonical_bytes'),('depth65',b'['*65+b']'*65,2,'depth_capacity'),
       ('syntax_before_depth',b'!'+b'['*65,2,'noncanonical_bytes'),('depth_before_syntax',b'['*65+b'!',2,'depth_capacity'),
       ('duplicate_before_header',b'{"kind":"OTHER","k\\u0069nd":"OTHER"}',3,'duplicate_key'),
       ('whitespace',b' '+c.encoded(full),6,'noncanonical_bytes')]
    for name,raw,stage,error in malformed: add_case(cases,'parse.'+name,raw,dict(admitted=False,stage=stage,error=error),'4197-4210',catalog,'exact_stage_error_only')
    raw=c.encoded(full)
    add_case(cases,'scalar.negative_zero',raw.replace(b'"population":200',b'"population":-0'),dict(admitted=False,stage=5,error='noncanonical_bytes',path='/state/colonies/0/society/population'),'4208',catalog)
    # Earlier declared stages win against simultaneous later faults.
    bad=g.mutation(full,'/catalog_hash','ff'*32)
    add_case(cases,'precedence.canonical_before_catalog',b' '+c.encoded(bad),dict(admitted=False,stage=6,error='noncanonical_bytes'),'4209-4211',catalog,'exact_stage_error_only')
    bad=g.mutation(full,'/rules_version',4);bad=g.mutation(bad,'/unknown',0)
    add_case(cases,'precedence.header_before_unknown',c.encoded(bad),dict(admitted=False,stage=4,error='unsupported_rules_version',path='/rules_version'),'4207-4208',catalog)
    bad=g.mutation(full,'/state/colonies/0/society/population',10001)
    add_case(cases,'precedence.domain_before_canonical',b' '+c.encoded(bad),dict(admitted=False,stage=5,error='invalid_range',path='/state/colonies/0/society/population'),'4208-4209',catalog)
    bad=g.mutation(full,'/seed','01'*32);bad=g.mutation(bad,'/catalog_hash','ff'*32)
    add_case(cases,'precedence.catalog_before_integrity',c.encoded(bad),dict(admitted=False,stage=8,error='catalog_mismatch',path='/catalog_hash'),'4211-4212',catalog)
    # Additional initial-profile semantic witnesses; no fabricated prior payments.
    negative(cases,full,catalog,'initial.owner_mismatch','/state/colonies/0/owner',full['state']['systems'][0]['id'],clause='4100-4103')
    negative(cases,full,catalog,'initial.self_loop','/state/lanes/0/system_b',full['state']['lanes'][0]['system_a'],clause='4101')
    needs=dict(evaluated_tick='0',population_before=200,food_need=2,energy_need=1,food_paid=0,energy_paid=0,population_after=198,outcome='decline',causes=['energy_shortage','food_shortage'])
    negative(cases,full,catalog,'initial.prior_needs','/state/colonies/0/society/last_needs',needs,clause='4104')
    for name,path,value in [
       ('supply_payment','/state/fleets/0/supply/last_success',dict(tick='0',world=full['state']['colonies'][0]['world'],supplies_paid=1,deadline='100')),
       ('supply_attempt','/state/fleets/0/supply/last_attempt',dict(tick='0',world=full['state']['colonies'][0]['world'],required=1,paid=0,outcome='unavailable')),
       ('order','/state/fleets/0/order',dict(kind='explore',target_world=None,not_before_tick='0',lane_path=[],objective=None,defense_request=None)),
       ('blockade','/state/fleets/0/blockade',dict(world=full['state']['colonies'][0]['world'],war_a='01'*16,war_b='02'*16,accepted_tick='0'))]:
        negative(cases,full,catalog,'initial.prior_'+name,path,value,clause='4104')
    # Remap the complete civilization reference graph; zero namespace is explicitly prohibited.
    old=full['state']['civilizations'][0]['id']
    def zero_actor(value):
        if isinstance(value,str):return '00'*16 if value==old else value
        if isinstance(value,list):return [zero_actor(v) for v in value]
        if isinstance(value,dict):return {k:zero_actor(v) for k,v in value.items()}
        return value
    add_case(cases,'initial.zero_civilization_namespace',c.encoded(zero_actor(full)),dict(admitted=False),'3757-3760',catalog,'rejection_only')
    bad=copy.deepcopy(full);obs=copy.deepcopy(bad['state']['observations'][0])
    remote=next(w for w in bad['state']['worlds'] if w['system']!=bad['state']['worlds'][home]['system'])
    obs['world']=remote['id'];obs['environment']['stellar_scope']['system']=remote['system']
    bad['state']['observations'].append(obs);bad['state']['observations'].sort(key=lambda row:(row['civilization'],row['world']))
    add_case(cases,'visibility.self_authorizing_remote',c.encoded(bad),dict(admitted=False),'4109-4124',catalog,'rejection_only')
    # No fabricated SHA collision: duplicate identity has its own real input above.
    return cases,positive,outputs


def structural_cases():
    """All currently declared bootstrap leaf/union shapes, explicitly not admission."""
    registry={name:value for name,value in vars(g).items() if isinstance(value,tuple) and value and value[0] in ('record','tag','enum','array','option')}
    cases=[]
    def sample(schema):
        if isinstance(schema,str): return {'u16':0,'u32':0,'u64':'0','i32':'0','ID':'00'*16,'Digest':'00'*32,'Slug':'a','Name':'N','tag':'sample','bool':False,'unsupported':None}[schema]
        tag=schema[0]
        if tag=='bounded': return schema[2]
        if tag=='record': return {k:sample(v) for k,v in schema[1].items()}
        if tag=='tag':
            key=next(iter(schema[1])); value=sample(schema[1][key]);value['type']=key;return value
        if tag=='enum': return schema[1][0]
        if tag=='array': return []
        return None
    def verify(name,value,schema,expected):
        try:
            g.check(c.Parser(c.encoded(value).decode('utf8')).parse(),schema)
            observed=dict(structurally_valid=True)
        except g.Rejected as e: observed=dict(structurally_valid=False,stage=e.stage,error=e.error,path=e.path)
        for k,v in expected.items(): assert observed.get(k)==v,(name,observed,expected)
        cases.append(dict(id=name,value=value,expected=expected,strength='structural_shape_only_not_genesis_admission',source=SCHEMA+':8-88;194-509;984-1224;1679-1835;3961-4008;4197-4216'))
    for name,schema in registry.items():
        if name in ('STATE','GENESIS'): continue
        base=sample(schema)
        verify(name+'.shape',base,schema,dict(structurally_valid=True))
        if schema[0]=='tag':
            for variant,child in schema[1].items():
                value=sample(child);value['type']=variant
                verify(name+'.'+variant,value,schema,dict(structurally_valid=True))
                for key in child[1]:
                    bad=copy.deepcopy(value);del bad[key]
                    verify(name+'.'+variant+'.missing.'+key,bad,schema,dict(structurally_valid=False,stage=5,error='noncanonical_bytes',path='/'+key))
            verify(name+'.bad_tag',{'type':'unknown'},schema,dict(structurally_valid=False,stage=5,error='invalid_enum',path='/type'))
        if schema[0]=='enum':
            for variant in schema[1]: verify(name+'.'+variant,variant,schema,dict(structurally_valid=True))
    for scalar,values,bad in [('u16',[0,65535],[(65536,'invalid_range'),(-1,'invalid_range'),(True,'invalid_range')]),
          ('u32',[0,2**32-1],[(2**32,'invalid_range'),(-1,'invalid_range'),(True,'invalid_range')]),
          ('u64',['0',str(g.U64)],[('00','noncanonical_bytes'),('-0','noncanonical_bytes'),(str(g.U64+1),'invalid_range'),(0,'invalid_range')]),
          ('ID',['00'*16,'ff'*16],[('AA'*16,'noncanonical_bytes'),('a'*31,'invalid_range')]),
          ('Digest',['00'*32,'ff'*32],[('AA'*32,'noncanonical_bytes'),('a'*63,'invalid_range')]),
          ('Name',['A','𐐀'*64,'�','é','é'],[('','invalid_range'),('a'*65,'invalid_range'),(' a','invalid_range'),('a ','invalid_range'),('a  b','invalid_range'),('\x7f','invalid_range'),('\x85','invalid_range')]),
          ('Slug',['a','a'+'0_'*31+'x'],[('A','invalid_range'),('a'*65,'invalid_range')])]:
        for i,value in enumerate(values):verify(scalar+'.valid.'+str(i),value,scalar,dict(structurally_valid=True))
        for i,(value,error) in enumerate(bad):verify(scalar+'.invalid.'+str(i),value,scalar,dict(structurally_valid=False,stage=5,error=error,path=''))
    return cases


def full_ledger(sources,cases):
    """Lossless line inventory: pending rows cannot be mistaken for closed predicates."""
    rows=[]
    for name,source in sources.items():
        section=''; line_id=0
        for i,line in enumerate((p.FREEZE/name).read_text().splitlines(),1):
            if not line.strip():continue
            if line.startswith('#'):section=line.lstrip('# ')
            rows.append(dict(id=name+':'+str(i),source_sha256=source['sha256'],section=section,text=line,
                             status='pending_semantic_mapping_and_independent_review',fixture_ids=[]))
    return dict(status='Partial: full-source line inventory; exhaustive semantic decomposition remains pending',
       source_rows=rows,
       foundation_cases=[dict(id=x['id'],source=x['source'],expected_strength=x['strength'],derivation=x.get('derivation','derive.structural_cases'),review='pending') for x in cases],
       required_later_packages=[dict(family=x,status='pending',fixture_ids=[]) for x in [
          'genuine boundaries 1–49 and Actor T50 phases1–9; Actor32',
          '200-boundary conditional baseline/candidate forecasts and admission',
          'scouting/economy/environment/population/research/conservation',
          'genuine diplomacy/War/PaidSupplyDefense/Peace',
          'creator/history/fork/revisions/18 issued kinds in genuine histories',
          'Archive/Notebook/Transfer/Coordinator full documents and all union variants',
          'bounded work/cancel/stale/CAS/native/web/accessibility/performance',
          'all original full-corpus and approval checklist annex cases']],
       richer_initial_contract_boundary=dict(status='unadopted_source_contract',source=SCHEMA+':4088-4143',
          families=['facilities','construction_jobs','hull_jobs','routes','shipments','hazards','stellar_events','relations','agreements','wars','offers','commitments','demand_requests','defense_requests','shared_intelligence','settlement_claims','occupations'],
          explanation='Explicit B0–B7 adopted only. Richer chronology, sources and successful initialization predicates need their own reviewed table. Unsupported helper paths are unexamined, never admitted or wire-error evidence.'),
       remaining_foundation_gaps=[
          'Full semantic decomposition and per-predicate source mapping for all eight sources is pending; the line inventory is deliberately not represented as that proof.',
          'FIX1 declares every State row shape, but exhaustive scalar/local/cross-field/reference/history-origin semantics and their complete matrix remain pending; richer initialization remains unadopted.',
          'Source explicitly leaves precise stage7 semantic and stage8 linkage matrix to full validator matrix; multi-failure semantics beyond specified order remain rejection-only.',
          'All positive cap witnesses and cap+1 coverage for every nested roster are not yet complete.',
          'Mixed/non-scout/travelling bootstrap initialization beyond the adopted docked-scout profile is not silently inferred from the visibility-only tests.',
          'Full schema source not read in its entirety during this implementation; source line inventory preserves unreviewed areas pending separate review.'])


def products():
    sources=p.check_sources(); catalog=c.core_catalog()
    catoutputs=c.products(sources)
    cases,positive,out=genesis_cases(catalog)
    shapes=structural_cases()
    out.update({'catalog/'+name:raw for name,raw in catoutputs.items()})
    out['genesis-cases.json']=dump(cases)
    out['structural-cases.json']=dump(shapes)
    out['fix1-regression-cases.json']=dump(fix1_cases.derive(catalog))
    out['foundation-predicate-coverage.json']=dump(predicate_matrix.matrix())
    seals={name:g.sealed(doc) for name,doc in positive.items()}
    out['initial-state-and-root-identities.json']=dump(seals)
    out['visibility-leaf-cases.json']=dump(visibility_cases.derive(catalog))
    context=[]
    for name in ('empty','inhabited'):
        good=seals[name];assert g.context_matches(positive[name],good)
        for path,value in [('/root_branch/id','ff'*16),('/root_branch/fork_state_digest','ff'*32),
                           ('/history_view/tick','1'),('/allocator_marks/accepted_sequence_high_water','1'),
                           ('/allocator_marks/branch_sequence_high_water','1')]:
            bad=g.mutation(good,path,value);assert not g.context_matches(positive[name],bad)
            context.append(dict(id=name+path,context=bad,expected_binding_matches=False,strength='derived_context_only_not_archive_admission'))
    out['root-binding-cases.json']=dump(context)
    # Literal complete frames for all18 kinds; only actual populated kinds are admission.
    out['all18-genesis-frames.json']=dump([dict(kind=k,name=name,ordinal=str(n),
        frame_hex=g.allocation_frame(positive['inhabited'],k,n).hex(),
        full_digest=g.sha(g.allocation_frame(positive['inhabited'],k,n)),
        id=g.sha(g.allocation_frame(positive['inhabited'],k,n))[:32],
        strength='framing_only_no_richer_genesis_or_history_admission',source=SCHEMA+':3742-3822')
        for k,name in enumerate(g.KINDS,1) for n in (1,g.U64)])
    out['full-package-coverage.json']=dump(full_ledger(sources,cases+shapes))
    # Raw cap tests remain explicit byte constructors; 8MiB whitespace/syntax bounds
    # are not disguised as maximal admitted canonical documents.
    capchecks=[]
    for n,stage,error in [(g.CAP,1,'noncanonical_bytes'),(g.CAP+1,0,'byte_capacity')]:
        raw=b'\xff'*n; actual=g.outcome(raw,catalog)
        assert actual['stage']==stage and actual['error']==error
        capchecks.append(dict(input=dict(repeat_byte_hex='ff',count=n),identity=p.identity(raw),
                              expected=dict(stage=stage,error=error),strength='raw_stage_boundary_not_positive_document'))
    out['raw-cap-cases.json']=dump(capchecks)
    out['helper_hashes.json']=dump({f.name:p.identity(f.read_bytes()) for f in sorted(HERE.glob('*.py'))})
    out['summary.json']=dump(dict(status='Partial foundation, independent derivation and literal review required',
        genesis_cases=len(cases),complete_bootstrap_profiles=len(positive),structural_cases=len(shapes),
        source_files=8,protected_old_files=66,corrected_pack_sha256=g.PACK,
        limitations='See full-package-coverage.json; no full-package, human or runtime approval.'))
    p.check_sources()
    return out


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('mode',choices=['generate','verify'])
    mode=parser.parse_args().mode
    output=products()
    for name,raw in output.items():
        path=HERE/name
        if mode=='generate':
            path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
        elif not path.exists() or path.read_bytes()!=raw:raise SystemExit('STOP: output drift '+name)
    print(json.dumps(dict(mode=mode,checks='passed',scope='Partial independent foundation; NOT full task completion',
                         summary=json.loads(output['summary.json']),files={name:p.identity(raw) for name,raw in output.items()}),indent=2))

if __name__=='__main__': main()
