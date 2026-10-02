#!/usr/bin/env python3
"""Independent PROPOSED V3 wire/hash subset, from written specs only.

No Rust imports, engine outputs, simulation acceptance or scoring vectors.
Generate outputs with `generate`; never edit generated vectors by hand.
"""
from __future__ import annotations
import copy
import hashlib
import json
from pathlib import Path
import re
import sys

BASE = Path(__file__).resolve().parent
SPECS = BASE.parent.parent
FREEZE = {
    "2026-10-01-living-expansion-wire.md": "a6d0cd2319127a10c93642626b88a89b25a57dcbbfef91800a2ed1da0320b1d3",
    "2026-10-01-living-v3-complete-schema.md": "2b08b4438201e9729374d8969827aab3861aaf991371a507b2935a6cd2669d28",
}
COLLECTIONS = ["systems", "stars", "worlds", "lanes", "civilizations", "deposits",
    "colonies", "facilities", "construction_jobs", "hull_jobs", "fleets", "routes",
    "shipments", "relations", "agreements", "wars", "observations", "hazards",
    "settlement_claims", "occupations", "offers", "demand_requests", "commitments",
    "defense_requests", "shared_intelligence", "stellar_events"]
CAPS = [64,128,512,256,16,1024,512,2048,4096,512,256,2048,4096,240,256,120,
        8192,128,8192,512,64,1024,256,256,4096,128]
TAGS = {n: i for i,n in enumerate(["system","star","world","lane","civilization",
    "deposit","facility","construction_job","hull_job","fleet","hull","route",
    "shipment","hazard","offer","commitment","defense_request","stellar_event"],1)}
U64_MAX = 2**64-1

class Reject(ValueError):
    pass

def reject(reason):
    raise Reject(reason)

def check(ok, reason):
    if not ok:
        reject(reason)

def H(frame):
    return hashlib.sha256(frame).hexdigest()

def D(suffix):
    return ("NYON_LIVING_V3_"+suffix).encode("ascii")+b"\0"

def le(value, width):
    check(type(value) is int and 0 <= value < 2**(width*8), "invalid_range")
    return value.to_bytes(width,"little")

def length(value):
    return le(len(value),4)+value

def optional_revision(value):
    return b"\0" if value is None else b"\1"+bytes.fromhex(value)

def quoted(value):
    out=['"']
    for c in value:
        n=ord(c)
        if c in ('"','\\'):
            out.append('\\'+c)
        elif n<32:
            out.append('\\u%04x'%n)
        else:
            check(not 0xD800<=n<=0xDFFF,"invalid_unicode")
            out.append(c)
    out.append('"')
    return ''.join(out)

def canonical(value):
    def encode(v):
        if v is None: return 'null'
        if type(v) is bool: return 'true' if v else 'false'
        if type(v) is int: return str(v)
        if type(v) is str: return quoted(v)
        if type(v) is list: return '['+','.join(encode(x) for x in v)+']'
        if type(v) is dict:
            return '{'+','.join(quoted(k)+':'+encode(x) for k,x in v.items())+'}'
        reject("invalid_type")
    return encode(value).encode('utf-8')

def pairs(items):
    result={}
    for key,value in items:
        check(key not in result,"duplicate_key")
        result[key]=value
    return result

def parse(raw,cap):
    check(cap is None or len(raw)<=cap,"byte_capacity")
    try:
        value=json.loads(raw.decode('utf-8'),object_pairs_hook=pairs,
                         parse_float=lambda _:reject("invalid_number"),
                         parse_constant=lambda _:reject("invalid_number"))
    except (UnicodeDecodeError,json.JSONDecodeError):
        reject("malformed_bytes")
    def depth(v, n=1):
        check(n<=64,"depth_capacity")
        if type(v) is dict:
            for x in v.values(): depth(x,n+1)
        elif type(v) is list:
            for x in v: depth(x,n+1)
    depth(value)
    check(canonical(value)==raw,"noncanonical_bytes")
    return value

def fields(v, names):
    check(type(v) is dict,"invalid_type")
    expected=names.split()
    check(not (set(v)-set(expected)),"unknown_field")
    check(not (set(expected)-set(v)),"missing_field_local")
    check(list(v)==expected,"field_order_local")

def hexfield(v,n):
    check(type(v) is str and re.fullmatch('[0-9a-f]{%d}'%(2*n),v) is not None,
          "invalid_hex")

def tick(v):
    check(type(v) is str and re.fullmatch('0|[1-9][0-9]*',v) is not None,
          "noncanonical_decimal")
    check(int(v)<=U64_MAX,"invalid_range")

def unsigned(v,max_value=2**32-1):
    check(type(v) is int and 0<=v<=max_value,"invalid_range")

def name(v):
    check(type(v) is str,"invalid_type")
    check(1<=len(v)<=64 and len(v.encode('utf-8'))<=256,"name_capacity")
    check(not any(ord(c)<32 or 127<=ord(c)<=159 for c in v),"name_control")
    check(not v.startswith(' ') and not v.endswith(' ') and '  ' not in v,
          "name_spacing")

def slug(v):
    check(type(v) is str and re.fullmatch('[a-z][a-z0-9_]{0,63}',v) is not None,
          "invalid_slug")

def sorted_keys(values,key):
    check(type(values) is list,"invalid_type")
    keys=[key(x) for x in values]
    check(all(a<b for a,b in zip(keys,keys[1:])),"unsorted_or_duplicate_key")

def digest_record(suffix,record):
    return H(D(suffix)+canonical(record))

def seal(suffix,payload):
    return {**payload,"integrity_sha256":digest_record(suffix,payload)}

def frozen_catalog():
    text=(SPECS/"2026-10-01-living-v3-complete-schema.md").read_text()
    value=json.loads(text.rsplit('```json\n',1)[1].split('\n```',1)[0])
    value['kind']='NYON_LIVING_PACK_V3'
    value['format_version']=3; value['rules_version']=3
    # Author-chosen input label, explicitly not a shipped V3 pack.
    value['pack_id']='nyon_living_core_v3_wire_fixture'
    value['resources'].append({'id':'food','name':'Food','storage_limit':10000})
    value['resources'].sort(key=lambda x:x['id'])
    for item in value['industry_definitions']:
        item['workforce_units']=0 if item['id']=='defense_battery' else 1
        item['stress_per_operation']={'extractor':1,'foundry':2}.get(item['id'],0)
    additions=[
      {'id':'farm','name':'Farm','alloy_cost':30,'build_ticks':150,'slot_cost':1,
       'max_per_colony':8,'effect':{'type':'recipe','period_ticks':10,
        'deposit_units':0,'inputs':[{'resource':'energy','quantity':2}],
        'outputs':[{'resource':'food','quantity':20}]},
       'workforce_units':1,'stress_per_operation':0},
      {'id':'habitation','name':'Habitation','alloy_cost':30,'build_ticks':150,
       'slot_cost':1,'max_per_colony':8,
       'effect':{'type':'habitation','base_capacity':1000},
       'workforce_units':0,'stress_per_operation':0},
      {'id':'research_lab','name':'Research Lab','alloy_cost':60,'build_ticks':300,
       'slot_cost':1,'max_per_colony':8,
       'effect':{'type':'research','period_ticks':20,
        'inputs':[{'resource':'alloy','quantity':1},{'resource':'energy','quantity':2}],
        'points':1},'workforce_units':1,'stress_per_operation':0},
    ]
    value['industry_definitions']+=additions
    value['industry_definitions'].sort(key=lambda x:x['id'])
    for item in value['hull_definitions']:
        item['role']={'colony_ark':'ark','escort':'escort','scout':'scout'}[item['id']]
    value['technology_definitions']=[
      {'id':'agronomy','name':'Agronomy','prerequisites':[],'point_cost':100,
       'effect':{'type':'farm_base_output','units':25}},
      {'id':'efficient_logistics','name':'Efficient Logistics','prerequisites':[],
       'point_cost':100,'effect':{'type':'departure_energy_scale','numerator':3,'denominator':4}},
      {'id':'resilient_habitation','name':'Resilient Habitation',
       'prerequisites':['agronomy'],'point_cost':150,
       'effect':{'type':'habitability_penalty_scale','numerator':1,'denominator':2}},
    ]
    return value

def state_empty():
    v={'tick':'0','branch_sequence':'0','next_sequence':'0'}
    v.update((x,[]) for x in COLLECTIONS);v['counters']={}
    return v

def state_payload(catalog_hash,state):
    return {'format_version':3,'rules_version':3,'catalog_hash':catalog_hash,'state':state}

def inventory(v):
    fields(v,'energy ore alloy food')
    for x in v.values(): unsigned(x,10000)

def research(v):
    fields(v,'target progress completed_technologies')
    if v['target'] is not None: slug(v['target'])
    sorted_keys(v['progress'],lambda x:x['technology'])
    for p in v['progress']:
        fields(p,'technology points');slug(p['technology']);unsigned(p['points'])
    sorted_keys(v['completed_technologies'],lambda x:x)
    for x in v['completed_technologies']:slug(x)

def environment(v):
    fields(v,'base_habitability stress fertility maximum_fertility next_environment_tick')
    for key in list(v)[:-1]:unsigned(v[key],10000)
    check(v['fertility']<=v['maximum_fertility'],'invalid_range')
    tick(v['next_environment_tick'])

def supply(v,current_tick):
    fields(v,'last_success last_attempt readiness')
    success=v['last_success'];attempt=v['last_attempt']
    if success is not None:
        fields(success,'tick world supplies_paid deadline')
        tick(success['tick']);hexfield(success['world'],16)
        unsigned(success['supplies_paid']);tick(success['deadline'])
        check(int(success['deadline'])==int(success['tick'])+100,'invalid_supply_deadline')
        check(success['supplies_paid']>0,'invalid_supply_payment')
    if attempt is not None:
        fields(attempt,'tick world required paid outcome');tick(attempt['tick'])
        hexfield(attempt['world'],16);unsigned(attempt['required']);unsigned(attempt['paid'])
        check(attempt['outcome'] in ('paid','unavailable'),'invalid_enum')
        check(attempt['required']>0,'invalid_supply_payment')
        if attempt['outcome']=='unavailable':check(attempt['paid']==0,'unpaid_success')
        else:
            check(attempt['paid']==attempt['required'],'partial_supply_payment')
            check(success is not None and success['tick']==attempt['tick']
                  and success['world']==attempt['world']
                  and success['supplies_paid']==attempt['paid'],'invalid_supply_payment')
    if success is not None and current_tick<int(success['deadline']):expected='supplied'
    elif success is not None or attempt is not None:expected='supply_unavailable'
    else:expected='never_supplied'
    check(v['readiness']==expected,'invalid_supply_readiness')

# Empty and minimal populated structural State subset: no mechanics validator.
def state(v):
    fields(v,'tick branch_sequence next_sequence '+' '.join(COLLECTIONS)+' counters')
    for k in ('tick','branch_sequence','next_sequence'):tick(v[k])
    fields(v['counters'],'')
    for key,cap in zip(COLLECTIONS,CAPS):
        check(type(v[key]) is list,'invalid_type');check(len(v[key])<=cap,'record_capacity')
    for key in ('systems','stars','worlds','civilizations','fleets'):
        sorted_keys(v[key],lambda x:x['id'])
    for x in v['systems']:fields(x,'id name');hexfield(x['id'],16);name(x['name'])
    for x in v['stars']:
        fields(x,'id system archetype name next_activity_tick')
        hexfield(x['id'],16);hexfield(x['system'],16);slug(x['archetype']);name(x['name'])
        tick(x['next_activity_tick'])
    for x in v['worlds']:
        fields(x,'id system archetype name owner inventory environment')
        hexfield(x['id'],16);hexfield(x['system'],16);slug(x['archetype']);name(x['name'])
        if x['owner'] is not None:hexfield(x['owner'],16)
        inventory(x['inventory']);environment(x['environment'])
    for x in v['civilizations']:
        fields(x,'id name policy research status');hexfield(x['id'],16);name(x['name'])
        slug(x['policy']);research(x['research']);check(x['status'] in ('active','dormant'),'invalid_enum')
    for x in v['colonies']:
        fields(x,'world owner hub society');hexfield(x['world'],16);hexfield(x['owner'],16)
        fields(x['hub'],'energy_next_due ore_next_due fallback_next_due blocked_reasons')
        for key in list(x['hub'])[:-1]:tick(x['hub'][key])
        check(x['hub']['blocked_reasons']==[],'outside_subset')
        q=x['society'];fields(q,'population specialization next_needs_tick last_needs workforce_assignments')
        unsigned(q['population'],10000);tick(q['next_needs_tick'])
        check(q['specialization'] in ('balanced','agrarian','industrial','research'),'invalid_enum')
        check(q['last_needs'] is None and q['workforce_assignments']==[],'outside_subset')
    for x in v['fleets']:
        fields(x,'id owner location order hulls supply blockade');hexfield(x['id'],16);hexfield(x['owner'],16)
        fields(x['location'],'type world');check(x['location']['type']=='docked','outside_subset')
        hexfield(x['location']['world'],16)
        check(x['order'] is None and x['blockade'] is None,'outside_subset')
        sorted_keys(x['hulls'],lambda h:h['id']);check(1<=len(x['hulls'])<=16,'record_capacity')
        for h in x['hulls']:
            fields(h,'id definition hit_points return_credit next_repair passengers')
            hexfield(h['id'],16);slug(h['definition']);unsigned(h['hit_points'])
            check(type(h['return_credit']) is bool,'invalid_type')
            if h['next_repair'] is not None:tick(h['next_repair'])
            fields(h['passengers'],'count');unsigned(h['passengers']['count'],10000)
        supply(x['supply'],int(v['tick']))
    implemented={'systems','stars','worlds','civilizations','colonies','fleets'}
    for key in COLLECTIONS:
        if key not in implemented:check(v[key]==[],'outside_subset')
    systems={x['id'] for x in v['systems']};worlds={x['id']:x for x in v['worlds']}
    actors={x['id'] for x in v['civilizations']}
    known_ids={x['id'] for key in ('systems','stars','worlds','civilizations','fleets') for x in v[key]}
    known_ids.update(h['id'] for f in v['fleets'] for h in f['hulls'])
    def reference(identity,required):
        if identity not in required:
            reject('wrong_reference_kind' if identity in known_ids else 'missing_reference')
    for x in v['stars']+v['worlds']:reference(x['system'],systems)
    for x in v['worlds']:
        if x['owner'] is not None:reference(x['owner'],actors)
    for x in v['colonies']:
        reference(x['world'],worlds);reference(x['owner'],actors)
        check(worlds[x['world']]['owner']==x['owner'],'ownership_mismatch')
    for x in v['fleets']:
        reference(x['owner'],actors);reference(x['location']['world'],worlds)

def versions(v):
    check(v['format_version']==3,'unsupported_format_version')
    check(v['rules_version']==3,'unsupported_rules_version')

def catalog(v):
    fields(v,'kind format_version rules_version pack_id pack_version star_archetypes world_archetypes resources industry_definitions hull_definitions hazard_definitions policy_definitions technology_definitions')
    check(v['kind']=='NYON_LIVING_PACK_V3','unsupported_kind')
    versions(v)
    slug(v['pack_id']);unsigned(v['pack_version']);check(v['pack_version']>0,'invalid_range')
    bounds={'star_archetypes':64,'world_archetypes':64,'resources':4,
            'industry_definitions':64,'hull_definitions':32,'hazard_definitions':32,
            'policy_definitions':32,'technology_definitions':64}
    for key,cap in bounds.items():
        check(len(v[key])<=cap,'record_capacity');sorted_keys(v[key],lambda x:x['id'])
        for x in v[key]:slug(x['id']);name(x['name'])
    for x in v['star_archetypes']+v['world_archetypes']:fields(x,'id name')
    check([x['id'] for x in v['resources']]==['alloy','energy','food','ore'],'invalid_resources')
    for x in v['resources']:
        fields(x,'id name storage_limit');check(x['storage_limit']==10000,'invalid_range')
    def amounts(a):
        sorted_keys(a,lambda x:x['resource'])
        for z in a:
            fields(z,'resource quantity');check(z['resource'] in ('alloy','energy','food','ore'),'missing_reference')
            unsigned(z['quantity'])
    for x in v['industry_definitions']:
        fields(x,'id name alloy_cost build_ticks slot_cost max_per_colony effect workforce_units stress_per_operation')
        for key in ('alloy_cost','build_ticks','slot_cost','max_per_colony','workforce_units','stress_per_operation'):unsigned(x[key])
        check(x['build_ticks']>0 and x['slot_cost']>0,'invalid_range')
        e=x['effect'];tag=e.get('type')
        layouts={'recipe':'type period_ticks deposit_units inputs outputs',
            'hull_assembly':'type concurrent_jobs','defense':'type hit_points damage_per_round',
            'habitation':'type base_capacity','research':'type period_ticks inputs points'}
        check(tag in layouts,'invalid_enum');fields(e,layouts[tag])
        if 'inputs' in e:amounts(e['inputs'])
        if 'outputs' in e:amounts(e['outputs'])
        for k,z in e.items():
            if k not in ('type','inputs','outputs'):unsigned(z)
        if 'period_ticks' in e:check(e['period_ticks']>0,'invalid_range')
    for x in v['hull_definitions']:
        fields(x,'id name alloy_cost build_ticks hit_points damage role')
        for k in ('alloy_cost','build_ticks','hit_points','damage'):unsigned(x[k])
        check(x['role'] in ('ark','escort','scout'),'invalid_enum')
    for x in v['hazard_definitions']:
        fields(x,'id name effect');e=x['effect'];fields(e,'type numerator denominator')
        check(e['type']=='freight_batch_scale','invalid_enum')
        unsigned(e['numerator']);unsigned(e['denominator']);check(e['denominator']>0,'invalid_range')
    text=(SPECS/'2026-10-01-living-v3-complete-schema.md').read_text()
    block=text.split('enum LivingIntentKindV3 {',1)[1].split('}',1)[0]
    intent_tags=re.findall(r'^    ([A-Z]\w*),$',block,re.M)
    declared={re.sub(r'(?<!^)(?=[A-Z])','_',x).lower() for x in intent_tags}
    declared.update('build_farm build_habitation build_research_lab select_research request_supply negotiate_access negotiate_supply negotiate_defense blockade adaptation'.split())
    for x in v['policy_definitions']:
        fields(x,'id name bonuses');sorted_keys(x['bonuses'],lambda b:b['intent'])
        for b in x['bonuses']:
            fields(b,'intent bonus');check(b['intent'] in declared,'invalid_enum');unsigned(b['bonus'])
    tech={x['id']:x for x in v['technology_definitions']}
    for x in tech.values():
        fields(x,'id name prerequisites point_cost effect');unsigned(x['point_cost']);check(x['point_cost']>0,'invalid_range')
        sorted_keys(x['prerequisites'],lambda z:z)
        for z in x['prerequisites']:check(z in tech,'missing_reference')
        e=x['effect'];tag=e.get('type')
        if tag=='farm_base_output':fields(e,'type units');unsigned(e['units'])
        elif tag in ('habitability_penalty_scale','departure_energy_scale'):
            fields(e,'type numerator denominator');unsigned(e['numerator']);unsigned(e['denominator']);check(e['denominator']>0,'invalid_range')
        else:reject('invalid_enum')
    def walk(t,seen):
        check(t not in seen,'invalid_graph')
        for z in tech[t]['prerequisites']:walk(z,seen|{t})
    for t in tech:walk(t,set())

def generator(v):
    fields(v,'generator_id generator_version');slug(v['generator_id']);unsigned(v['generator_version'])

def gframe(ch,seed,gen):
    return bytes.fromhex(ch)+bytes.fromhex(seed)+length(gen['generator_id'].encode())+le(gen['generator_version'],4)

def genesis_entity(ch,seed,gen,kind,ordinal):
    return D('GENESIS_ENTITY')+gframe(ch,seed,gen)+le(3,1)+le(kind,2)+le(ordinal,8)

def snapshot(ch,branch,sd,t='0',rev=None,phase=1):
    return {'domain':3,'catalog_hash':ch,'branch':branch,'revision':rev,'tick':t,
            'state_digest':sd,'phase':phase,'permitted_snapshot_digest':sd}

def snap(v):
    fields(v,'domain catalog_hash branch revision tick state_digest phase permitted_snapshot_digest')
    check(v['domain']==3,'unsupported_domain_local');hexfield(v['catalog_hash'],32);hexfield(v['branch'],16)
    if v['revision'] is not None:hexfield(v['revision'],32)
    tick(v['tick']);hexfield(v['state_digest'],32);hexfield(v['permitted_snapshot_digest'],32)
    unsigned(v['phase'],12);check(v['phase']>0,'invalid_range')

def root_frame(ch,seed,gd,gen):
    return D('ROOT_BRANCH')+bytes.fromhex(ch)+bytes.fromhex(seed)+bytes.fromhex(gd)+length(gen['generator_id'].encode())+le(gen['generator_version'],4)

def view(v):
    fields(v,'branch revision tick');hexfield(v['branch'],16);tick(v['tick'])
    if v['revision'] is not None:hexfield(v['revision'],32)

def integrity(suffix,v):
    hexfield(v['integrity_sha256'],32)
    check(list(v)[-1]=='integrity_sha256','field_order_or_schema')
    check(digest_record(suffix,{k:x for k,x in v.items() if k!='integrity_sha256'})==v['integrity_sha256'],'integrity_mismatch')

def genesis(v,ch):
    fields(v,'kind format_version rules_version catalog_hash seed generator allocations state')
    check(v['kind']=='NYON_LIVING_GENESIS_V3','unsupported_kind')
    versions(v)
    check(v['catalog_hash']==ch,'catalog_mismatch');hexfield(v['seed'],32);generator(v['generator']);state(v['state'])
    check(v['state']['tick']=='0','invalid_genesis_tick')
    sorted_keys(v['allocations'],lambda a:(a['kind'],int(a['ordinal'])))
    for a in v['allocations']:
        fields(a,'kind ordinal id');check(a['kind'] in TAGS.values(),'invalid_enum');tick(a['ordinal']);check(int(a['ordinal'])>0,'invalid_range');hexfield(a['id'],16)
        f=genesis_entity(ch,v['seed'],v['generator'],a['kind'],int(a['ordinal']))
        check(a['id']==H(f)[:32],'identity_mismatch')
    ids=[]
    for key in ('systems','stars','worlds','civilizations','fleets'):ids += [x['id'] for x in v['state'][key]]
    ids += [h['id'] for f in v['state']['fleets'] for h in f['hulls']]
    check(set(ids)=={a['id'] for a in v['allocations']},'allocation_roster_mismatch')

def archive(v,ch):
    fields(v,'kind format_version rules_version catalog_hash genesis revisions branches active_view final_tick final_digest accepted_sequence_high_water branch_sequence_high_water anchors integrity_sha256')
    check(v['kind']=='NYON_LIVING_ARCHIVE_V3','unsupported_kind')
    versions(v);check(v['catalog_hash']==ch,'catalog_mismatch')
    genesis(v['genesis'],ch);integrity('ARCHIVE',v);view(v['active_view'])
    tick(v['final_tick']);tick(v['accepted_sequence_high_water']);tick(v['branch_sequence_high_water'])
    check(v['revisions']==[] and v['anchors']==[],'outside_subset');check(len(v['branches'])==1,'outside_subset')
    b=v['branches'][0];fields(b,'id parent fork_revision fork_tick fork_state_digest ordinal head tip_tick')
    gd=digest_record('GENESIS',v['genesis']);expected=H(root_frame(ch,v['genesis']['seed'],gd,v['genesis']['generator']))[:32]
    sd=digest_record('STATE',state_payload(ch,v['genesis']['state']))
    check(b['id']==expected,'identity_mismatch');check(b['parent'] is None and b['fork_revision'] is None and b['head'] is None,'invalid_graph')
    check(b['fork_tick']==b['ordinal']==b['tip_tick']=='0','invalid_graph');check(b['fork_state_digest']==sd,'replay_mismatch')
    check(v['active_view']=={'branch':expected,'revision':None,'tick':'0'},'invalid_history_view')
    check(v['final_tick']=='0' and v['final_digest']==sd,'replay_mismatch')

def notebook(v,ch,a):
    fields(v,'kind format_version rules_version catalog_hash genesis_digest archive_integrity_sha256 next_ordinal bookmarks experiments evidence integrity_sha256')
    check(v['kind']=='NYON_LIVING_NOTEBOOK_V3','unsupported_kind');versions(v)
    check(v['catalog_hash']==ch,'catalog_mismatch');integrity('NOTEBOOK',v);tick(v['next_ordinal'])
    check(v['genesis_digest']==digest_record('GENESIS',a['genesis']),'integrity_mismatch')
    check(v['archive_integrity_sha256']==a['integrity_sha256'],'integrity_mismatch')
    check(v['experiments']==v['evidence']==[],'outside_subset')
    sorted_keys(v['bookmarks'],lambda x:int(x['ordinal']))
    check(len(v['bookmarks'])<=1024,'record_capacity')
    for b in v['bookmarks']:
        fields(b,'ordinal label view digest');tick(b['ordinal']);name(b['label']);view(b['view']);hexfield(b['digest'],32)
        check(0<int(b['ordinal'])<int(v['next_ordinal']),'invalid_ordinal')
        check(b['view']==a['active_view'],'outside_subset')
        check(b['digest']==a['final_digest'],'replay_mismatch')

def validate_document(raw,kind,context):
    caps={'catalog':2**20,'state':8*2**20,'genesis':8*2**20,'archive':64*2**20,'notebook':8*2**20,'transfer':96*2**20,'coordinator':None}
    v=parse(raw,caps[kind]);ch=context['catalog_hash']
    if kind=='catalog':catalog(v)
    elif kind=='state':
        fields(v,'format_version rules_version catalog_hash state');versions(v);check(v['catalog_hash']==ch,'catalog_mismatch');state(v['state'])
    elif kind=='genesis':genesis(v,ch)
    elif kind=='archive':archive(v,ch)
    elif kind=='notebook':notebook(v,ch,context['archive'])
    elif kind=='transfer':
        fields(v,'kind format_version rules_version catalog archive notebook selected_view integrity_sha256')
        check(v['kind']=='NYON_LIVING_TRANSFER_V3','unsupported_kind');versions(v)
        catalog(v['catalog']);actual=digest_record('PACK',v['catalog']);archive(v['archive'],actual);notebook(v['notebook'],actual,v['archive'])
        check(v['selected_view']==v['archive']['active_view'],'invalid_history_view');integrity('TRANSFER',v)
    elif kind=='coordinator':
        fields(v,'kind format_version authority rules_version slot catalog_hash view generation integrity_sha256')
        check(v['kind']=='NYON_SANDBOX_SELECTION_V3' and v['authority']=='living_v3','unsupported_kind');versions(v)
        hexfield(v['slot'],16);check(v['catalog_hash']==ch,'catalog_mismatch');view(v['view']);tick(v['generation']);integrity('COORDINATOR',v)
    return v

def creator_capture(v):
    fields(v,'accepted_sequence operation_index application_tick mode snapshot')
    tick(v['accepted_sequence']);unsigned(v['operation_index']);tick(v['application_tick'])
    check(v['mode'] in ('paused','running'),'invalid_enum');snap(v['snapshot'])

def supply_terms(v):
    fields(v,'type source_world destination_world resource batch cadence_ticks source_reserve duration_ticks')
    check(v['type']=='resource_supply','invalid_enum')
    hexfield(v['source_world'],16);hexfield(v['destination_world'],16)
    check(v['source_world']!=v['destination_world'],'invalid_reference')
    check(v['resource'] in ('energy','ore','alloy','food'),'invalid_enum')
    unsigned(v['batch'],10);check(v['batch']>0,'invalid_range')
    check(v['cadence_ticks']==10 and v['source_reserve']>=20 and v['duration_ticks']==1200,'invalid_terms')

def threat_record(v,ch):
    fields(v,'enemy target_world war observed_tick originating_observer observation_snapshot_digest hostile_escort_count occupation hostile_fleets')
    for k in ('enemy','target_world','originating_observer'):hexfield(v[k],16)
    tick(v['observed_tick']);unsigned(v['hostile_escort_count']);check(type(v['occupation']) is bool,'invalid_type')
    sorted_keys(v['hostile_fleets'],lambda x:x)
    for x in v['hostile_fleets']:hexfield(x,16)
    if v['war'] is not None:
        fields(v['war'],'participant_a participant_b')
        for x in v['war'].values():hexfield(x,16)
        check(v['war']['participant_a']<v['war']['participant_b'],'invalid_pair')
    payload={'format_version':3,'rules_version':3,'catalog_hash':ch,
      'originating_observer':v['originating_observer'],'target_world':v['target_world'],
      'observed_tick':v['observed_tick'],'enemy':v['enemy'],'war':v['war'],
      'hostile_escort_count':v['hostile_escort_count'],'occupation':v['occupation'],
      'hostile_fleets':v['hostile_fleets']}
    check(v['observation_snapshot_digest']==digest_record('THREAT_OBSERVATION',payload),'threat_digest_mismatch')

def manifest_record(v):
    if v.get('type')=='creator':
        fields(v,'type capture');creator_capture(v['capture']);return
    fields(v,'type snapshot route commitment withdrawn');check(v['type']=='paid','invalid_enum')
    snap(v['snapshot']);hexfield(v['route'],16)
    if v['commitment'] is not None:
        b=v['commitment'];fields(b,'id created_tick terms_digest')
        hexfield(b['id'],16);tick(b['created_tick']);hexfield(b['terms_digest'],32)
    fields(v['withdrawn'],'resource quantity')
    check(v['withdrawn']['resource'] in ('energy','ore','alloy','food'),'invalid_enum')
    unsigned(v['withdrawn']['quantity'],10000);check(v['withdrawn']['quantity']>0,'invalid_range')

def waiting_consideration(v):
    fields(v,'tick disposition reason');tick(v['tick'])
    check(v['disposition']=='waiting' and v['reason']=='capacity_unavailable','invalid_consideration')

def rejection_expectation(diagnostic):
    # Only diagnostics literally selected by the frozen closed enum assert an exact
    # transport tag. The schema does not define a mapping for every local predicate.
    # Missing fields/types/malformed Unicode/forged allocator bytes remain rejection-
    # only cases rather than inventing a new normative error or guessing a mapping.
    text=(SPECS/'2026-10-01-living-v3-complete-schema.md').read_text()
    block=text.split('Typed read/import/job errors are a closed enum: ',1)[1].split('.\nFailure includes',1)[0]
    normative=set(re.findall(r'[a-z][a-z_]+',block))
    exact=diagnostic if diagnostic in normative else None
    return {'must_reject':True,'local_diagnostic':diagnostic,
            'expected_wire_error':exact,
            'claim':'exact_wire_tag' if exact is not None else 'rejection_only'}

def corpus():
    for filename,expected in FREEZE.items():
        check(H((SPECS/filename).read_bytes())==expected,'source_freeze_changed')
    cat=frozen_catalog();catalog(cat);ch=digest_record('PACK',cat)
    gen={'generator_id':'wire_fixture','generator_version':1};seed='00'*32
    st=state_empty();wrapped=state_payload(ch,st);sd=digest_record('STATE',wrapped)
    gn={'kind':'NYON_LIVING_GENESIS_V3','format_version':3,'rules_version':3,
        'catalog_hash':ch,'seed':seed,'generator':gen,'allocations':[],'state':st}
    gd=digest_record('GENESIS',gn);rf=root_frame(ch,seed,gd,gen);root=H(rf)[:32]
    selected={'branch':root,'revision':None,'tick':'0'}
    ar=seal('ARCHIVE',{'kind':'NYON_LIVING_ARCHIVE_V3','format_version':3,'rules_version':3,
        'catalog_hash':ch,'genesis':gn,'revisions':[],
        'branches':[{'id':root,'parent':None,'fork_revision':None,'fork_tick':'0',
         'fork_state_digest':sd,'ordinal':'0','head':None,'tip_tick':'0'}],
        'active_view':selected,'final_tick':'0','final_digest':sd,
        'accepted_sequence_high_water':'0','branch_sequence_high_water':'0','anchors':[]})
    nb=seal('NOTEBOOK',{'kind':'NYON_LIVING_NOTEBOOK_V3','format_version':3,'rules_version':3,
        'catalog_hash':ch,'genesis_digest':gd,'archive_integrity_sha256':ar['integrity_sha256'],
        'next_ordinal':'1','bookmarks':[],'experiments':[],'evidence':[]})
    tr=seal('TRANSFER',{'kind':'NYON_LIVING_TRANSFER_V3','format_version':3,'rules_version':3,
        'catalog':cat,'archive':ar,'notebook':nb,'selected_view':selected})
    co=seal('COORDINATOR',{'kind':'NYON_SANDBOX_SELECTION_V3','format_version':3,
        'authority':'living_v3','rules_version':3,'slot':'11'*16,'catalog_hash':ch,
        'view':selected,'generation':'1'})
    ctx={'catalog_hash':ch,'archive':ar}
    docs=[]; negatives=[]; primitives=[]; hashes=[]
    def document(label,kind,value):
        raw=canonical(value);validate_document(raw,kind,ctx)
        docs.append({'label':label,'kind':kind,'canonical_utf8':raw.decode(),
                     'bytes':len(raw),'raw_sha256':H(raw)})
    for kind,value in [('catalog',cat),('state',wrapped),('genesis',gn),('archive',ar),
                       ('notebook',nb),('transfer',tr),('coordinator',co)]:
        document('synthetic_empty_'+kind,kind,value)
    # Synthetic IDs derive from exact genesis allocator; catalog references are explicit.
    ids={kind:H(genesis_entity(ch,seed,gen,TAGS[kind],1))[:32]
         for kind in ('system','star','world','civilization','fleet','hull')}
    ps=state_empty()
    ps['systems']=[{'id':ids['system'],'name':'Synthetic System'}]
    ps['stars']=[{'id':ids['star'],'system':ids['system'],
      'archetype':cat['star_archetypes'][0]['id'],'name':'Synthetic Star','next_activity_tick':'100'}]
    ps['worlds']=[{'id':ids['world'],'system':ids['system'],
      'archetype':cat['world_archetypes'][0]['id'],'name':'Synthetic World','owner':ids['civilization'],
      'inventory':{'energy':10,'ore':100,'alloy':100,'food':20},
      'environment':{'base_habitability':8000,'stress':0,'fertility':1000,
                     'maximum_fertility':1000,'next_environment_tick':'100'}}]
    ps['civilizations']=[{'id':ids['civilization'],'name':'Synthetic Civilization','policy':'neutral',
      'research':{'target':None,'progress':[],'completed_technologies':[]},'status':'active'}]
    ps['colonies']=[{'world':ids['world'],'owner':ids['civilization'],
      'hub':{'energy_next_due':'10','ore_next_due':'100','fallback_next_due':'100','blocked_reasons':[]},
      'society':{'population':500,'specialization':'balanced','next_needs_tick':'100',
                 'last_needs':None,'workforce_assignments':[]}}]
    ps['fleets']=[{'id':ids['fleet'],'owner':ids['civilization'],
      'location':{'type':'docked','world':ids['world']},'order':None,
      'hulls':[{'id':ids['hull'],'definition':'scout','hit_points':10,'return_credit':False,
                'next_repair':None,'passengers':{'count':0}}],
      'supply':{'last_success':None,'last_attempt':None,'readiness':'never_supplied'},'blockade':None}]
    pg={**gn,'allocations':[{'kind':TAGS[k],'ordinal':'1','id':ids[k]}
                           for k in sorted(ids,key=lambda k:TAGS[k])],'state':ps}
    document('synthetic_populated_genesis','genesis',pg)
    document('synthetic_populated_state','state',state_payload(ch,ps))
    boundary=state_empty();boundary['systems']=[{'id':f'{i:032x}','name':'Synthetic'} for i in range(64)]
    document('systems_capacity64_structural','state',state_payload(ch,boundary))
    marked={k:v for k,v in nb.items() if k!='integrity_sha256'}
    marked['next_ordinal']='2';marked['bookmarks']=[{'ordinal':'1','label':'Synthetic bookmark',
      'view':selected,'digest':sd}]
    marked=seal('NOTEBOOK',marked)
    document('t0_bookmark_exact_anchor','notebook',marked)
    def bad(label,kind,value,reason,raw=None):
        wire=canonical(value) if raw is None else raw
        try:validate_document(wire,kind,ctx)
        except Reject as exc:check(str(exc)==reason,'unexpected_rejection:'+label+':'+str(exc))
        else:reject('unexpected_acceptance:'+label)
        negatives.append({'label':label,'kind':kind,'raw_hex':wire.hex(), **rejection_expectation(reason)})
    def changed(value,key,replacement):
        v=copy.deepcopy(value);v[key]=replacement;return v
    bad('state_rules_version2','state',changed(wrapped,'rules_version',2),'unsupported_rules_version')
    bad('state_format_version2','state',changed(wrapped,'format_version',2),'unsupported_format_version')
    bad('state_catalog_mismatch','state',changed(wrapped,'catalog_hash','22'*32),'catalog_mismatch')
    bad('state_unknown_field','state',{**wrapped,'unknown':None},'unknown_field')
    bad('state_missing_field','state',{k:v for k,v in wrapped.items() if k!='state'},'missing_field_local')
    bad('state_field_order','state',dict(reversed(list(wrapped.items()))),'field_order_local')
    for t,r in [('00','noncanonical_decimal'),('-1','noncanonical_decimal'),
                ('18446744073709551616','invalid_range'),(0,'noncanonical_decimal')]:
        v=copy.deepcopy(wrapped);v['state']['tick']=t;bad('tick_'+str(t),'state',v,r)
    bad('catalog_enum','catalog',changed(cat,'kind','NYON_LIVING_PACK_V2'),'unsupported_kind')
    v=copy.deepcopy(cat);v['hull_definitions'][0]['role']='transport';bad('hull_enum','catalog',v,'invalid_enum')
    v=copy.deepcopy(cat);v['policy_definitions'][0]['bonuses']=[{'intent':'active','bonus':1}]
    bad('intent_enum_not_status','catalog',v,'invalid_enum')
    bad('archive_integrity','archive',changed(ar,'integrity_sha256','00'*32),'integrity_mismatch')
    v=copy.deepcopy(marked);v['bookmarks'][0]['digest']='00'*32
    v=seal('NOTEBOOK',{k:x for k,x in v.items() if k!='integrity_sha256'})
    bad('bookmark_replayed_digest_mismatch','notebook',v,'replay_mismatch')

    raw=canonical(wrapped)
    bad('whitespace','state',None,'noncanonical_bytes',raw=b' '+raw)
    bad('duplicate_key','state',None,'duplicate_key',raw=raw.replace(b'{',b'{"format_version":3,',1))
    bad('float','state',None,'invalid_number',raw=raw.replace(b'"format_version":3',b'"format_version":3.0',1))
    bad('invalid_utf8','state',None,'malformed_bytes',raw=b'\xff'+raw)
    bad('unicode_escape_equivalence','state',None,'noncanonical_bytes',raw=raw.replace(b'"state"',b'"st\\u0061te"'))
    v=copy.deepcopy(wrapped);v['state']['systems']=None;bad('array_null','state',v,'invalid_type')
    v=copy.deepcopy(wrapped);v['state']['systems']=[{'id':f'{i:032x}','name':'Synthetic'} for i in range(65)]
    bad('systems65','state',v,'record_capacity')
    v=copy.deepcopy(wrapped);v['state']['systems']=[{'id':'01'*16,'name':'One'},{'id':'00'*16,'name':'Two'}]
    bad('unsorted_ids','state',v,'unsorted_or_duplicate_key')
    v=copy.deepcopy(wrapped);v['state']['systems']=[{'id':'AA'*16,'name':'One'}]
    bad('uppercase_id','state',v,'invalid_hex')
    # Names exercise Unicode scalars and UTF-8 separately; labels are synthetic.
    for label,txt in [('emoji64','🌌'*64),('composed','Étoile'),('decomposed','E\u0301toile')]:
        v=copy.deepcopy(wrapped);v['state']['systems']=[{'id':'01'*16,'name':txt}]
        document(label,'state',v)
    for label,txt,reason in [('ascii65','a'*65,'name_capacity'),('control','a\n','name_control'),
                             ('padding',' a','name_spacing'),('double_space','a  b','name_spacing')]:
        v=copy.deepcopy(wrapped);v['state']['systems']=[{'id':'01'*16,'name':txt}]
        bad(label,'state',v,reason)
    v=state_payload(ch,copy.deepcopy(ps));v['state']['worlds'][0]['system']=ids['world']
    bad('wrong_kind_system_reference','state',v,'wrong_reference_kind')
    v=state_payload(ch,copy.deepcopy(ps));v['state']['colonies'][0]['owner']='ff'*16
    bad('missing_colony_owner','state',v,'missing_reference')
    v=copy.deepcopy(pg);v['allocations'][0]['id']='ff'*16
    bad('forged_genesis_allocation','genesis',v,'identity_mismatch')
    # Reject byte overflow before parsing, with a bounded descriptor rather than storing8 MiB.
    try:parse(b' '*(8*2**20+1),8*2**20)
    except Reject as exc:check(str(exc)=='byte_capacity','unexpected_capacity_rejection')
    else:reject('byte_overflow_accepted')
    negatives.append({'label':'state_byte_cap_plus1','kind':'state',
      'input_descriptor':{'repeat_hex':'20','count':8*2**20+1},**rejection_expectation('byte_capacity')})
    def primitive(label,value,validator,expected=None):
        try:validator(value)
        except Reject as exc:
            check(str(exc)==expected,'unexpected_primitive:'+label+':'+str(exc));outcome=expected
        else:check(expected is None,'unexpected_primitive_acceptance:'+label);outcome='valid_subset'
        primitives.append({'label':label,'input':value,'outcome':'accepted_subset' if expected is None else 'rejected',
                           **({'local_diagnostic':None,'expected_wire_error':None,'claim':'accepted_subset'} if expected is None else rejection_expectation(outcome))})
    primitive('u64_max',str(U64_MAX),tick)
    primitive('u64_overflow',str(U64_MAX+1),tick,'invalid_range')
    def hf(label,frame,truncate=False,payload=None):
        item={'label':label,'input_hex':frame.hex(),'sha256':H(frame)}
        if truncate:item['id']=H(frame)[:32]
        if payload is not None:item['canonical_payload_utf8']=canonical(payload).decode()
        hashes.append(item);return H(frame)
    hf('catalog_domain',D('PACK')+canonical(cat),payload=cat)
    hf('state_version_catalog_wrapped',D('STATE')+canonical(wrapped),payload=wrapped)
    hf('genesis_domain',D('GENESIS')+canonical(gn),payload=gn)
    hf('root_branch',rf,True)
    for kind in TAGS.values():hf('genesis_kind_'+str(kind),genesis_entity(ch,seed,gen,kind,1),True)
    gf=gframe(ch,seed,gen)
    primitives.append({'label':'genesis_G','input_hex':gf.hex(),'catalog_hash':ch,'seed':seed,'generator':gen})
    snapref=snapshot(ch,root,sd)
    capture={'accepted_sequence':'1','operation_index':0,'application_tick':'0','mode':'paused','snapshot':snapref}
    # Creator/local record capture predates revision; no placeholder/future hash.
    cf=D('CREATOR_ENTITY')+bytes.fromhex(root)*2+le(2,1)+le(1,8)+le(0,4)+le(18,2)+le(7,2)
    sid=hf('creator_stellar_id_pre_revision',cf,True)[:32]
    star=H(genesis_entity(ch,seed,gen,2,1))[:32]
    stellar={'id':sid,'star':star,'warning_tick':'10','start_tick':'20','end_tick':'30',
             'habitability_impact':1000,'freight_factor':{'numerator':1,'denominator':2},
             'provenance':{'type':'creator','capture':capture}}
    primitives.append({'label':'stellar_capture_acyclic_record','input':stellar,
                       'scope':'framing_only_star_reference_not_present_in_empty_genesis'})
    command={'type':'creator_batch','operations':[],'reviewed_effects':[]}
    # Empty batch validity is not asserted; exact hash framing is independent of command admission.
    rvframe=D('REVISION')+bytes.fromhex(root)*2+optional_revision(None)+le(0,8)+le(1,8)+le(0,1)+length(canonical(snapref))+length(canonical(command))
    rid=hf('revision_framing_only_unadmitted_empty_batch',rvframe,payload={'capture':snapref,'command':command})
    ff=D('FORK_BRANCH')+bytes.fromhex(root)*2+optional_revision(rid)+le(0,8)+bytes.fromhex(sd)+le(1,8)
    hf('fork_nonnull_parent_revision',ff,True)
    hf('fork_null_parent_revision',D('FORK_BRANCH')+bytes.fromhex(root)*2+optional_revision(None)+le(0,8)+bytes.fromhex(sd)+le(1,8),True)
    for namespace,actor in [(0,'00'*16),(1,'33'*16)]:
        af=D('AUTO_ENTITY')+bytes.fromhex(root)*2+le(namespace,1)+bytes.fromhex(actor)+le(100,8)+le(1 if namespace==0 else 9,2)+le(18 if namespace==0 else 10,2)+le(0,4)+le(2,2)+bytes.fromhex(star)+bytes.fromhex(sd)
        hf('autonomous_namespace_'+str(namespace)+'_framing_only',af,True)
    receipt={'tick':'0','applied_revisions':[],'events':[],'state_digest':sd}
    rd=hf('empty_receipt',D('RECEIPT')+canonical(receipt),payload=receipt)
    hf('event_receipt_bound_framing_only',D('EVENT')+bytes.fromhex(rd)+le(0,8)+le(1,2)+le(0,4),True)
    # Above event frame is not asserted to exist in the empty receipt.
    observer='33'*16;enemy='44'*16;world='55'*16
    threat_payload={'format_version':3,'rules_version':3,'catalog_hash':ch,
      'originating_observer':observer,'target_world':world,'observed_tick':'100','enemy':enemy,
      'war':None,'hostile_escort_count':2,'occupation':False,'hostile_fleets':['66'*16]}
    td=hf('threat_fact_acyclic',D('THREAT_OBSERVATION')+canonical(threat_payload),payload=threat_payload)
    threat={'enemy':enemy,'target_world':world,'war':None,'observed_tick':'100',
      'originating_observer':observer,'observation_snapshot_digest':td,
      'hostile_escort_count':2,'occupation':False,'hostile_fleets':['66'*16]}
    primitive('threat_acyclic_valid_shape',threat,lambda v:threat_record(v,ch))
    forged=copy.deepcopy(threat);forged['observation_snapshot_digest']=sd
    primitive('threat_cannot_substitute_state_digest',forged,lambda v:threat_record(v,ch),'threat_digest_mismatch')
    primitives.append({'label':'copied_threat_no_receive_time_refresh','input':threat,
                       'copy':copy.deepcopy(threat),'scope':'record_framing_not_visibility_simulation'})
    never={'last_success':None,'last_attempt':None,'readiness':'never_supplied'}
    first={'last_success':None,'last_attempt':{'tick':'100','world':world,'required':1,'paid':0,'outcome':'unavailable'},'readiness':'supply_unavailable'}
    paid={'last_success':{'tick':'100','world':world,'supplies_paid':1,'deadline':'200'},
      'last_attempt':{'tick':'100','world':world,'required':1,'paid':1,'outcome':'paid'},'readiness':'supplied'}
    failed=copy.deepcopy(paid);failed['last_attempt']={'tick':'150','world':world,'required':1,'paid':0,'outcome':'unavailable'}
    expired=copy.deepcopy(failed);expired['readiness']='supply_unavailable'
    for label,value,t in [('never',never,0),('first_unavailable',first,100),('paid',paid,100),('failed_prior_unexpired',failed,150),('failed_prior_expired',expired,200)]:
        primitive('supply_'+label,value,lambda v,t=t:supply(v,t))
    bads=copy.deepcopy(first);bads['last_attempt']['paid']=1
    primitive('supply_unavailable_cannot_claim_payment',bads,lambda v:supply(v,100),'unpaid_success')
    bads=copy.deepcopy(failed);bads['readiness']='supply_unavailable'
    primitive('supply_failure_does_not_expire_prior',bads,lambda v:supply(v,150),'invalid_supply_readiness')
    terms={'type':'resource_supply','source_world':world,
      'destination_world':'77'*16,'resource':'food','batch':4,'cadence_ticks':10,
      'source_reserve':20,'duration_ticks':1200}
    primitive('supply_terms_shape',terms,supply_terms)
    termdigest=hf('immutable_commitment_terms',D('TREATY_TERMS')+canonical(terms),payload=terms)
    binding={'id':'88'*16,'created_tick':'100','terms_digest':termdigest}
    manifest={'type':'paid','snapshot':snapref,'route':'99'*16,'commitment':binding,
              'withdrawn':{'resource':'food','quantity':4}}
    primitive('manifest_paid_shape',manifest,manifest_record)
    malformed=copy.deepcopy(manifest);malformed['commitment']['terms_digest']=None
    primitive('manifest_requires_terms_digest',malformed,manifest_record,'invalid_hex')
    primitive('creator_capture_shape',capture,creator_capture)
    primitive('manifest_creator_shape',{'type':'creator','capture':capture},manifest_record)
    primitives.append({'label':'manifest_original_binding','input':manifest,
      'different_commitment_id':'aa'*16,'same_id_different_terms':{**terms,'batch':5},
      'scope':'immutable_framing_only_no_fulfillment_simulation'})
    primitives.append({'label':'creator_manifest','input':{'type':'creator','capture':capture},
      'scope':'creator_provenance_never_paid_witness'})
    consideration={'tick':'200','disposition':'waiting','reason':'capacity_unavailable'}
    primitive('waiting_consideration_shape',consideration,waiting_consideration)
    primitive('waiting_cannot_masquerade_as_refusal',{**consideration,'disposition':'refused'},waiting_consideration,'invalid_consideration')
    primitives.append({'label':'offer_waiting_capacity_preserves_id','before':None,
      'after':consideration,'id':'bb'*16,'scope':'record_shape_not_acceptance_simulation'})
    return {'status':'PROPOSED_INDEPENDENT_WRITTEN_FORMULA_SUBSET',
      'source_freeze':FREEZE,'synthetic_input_notice':'Author-selected labels and inputs; no shipped V3 pack or Rust-derived expected outputs.',
      'scope':'T0 empty complete documents; selected structural/scalar negatives; literal framing bytes and digests; record-shape edge examples. Not full gap6.',
      'documents':docs,'negative_documents':negatives,'primitive_cases':primitives,'hash_vectors':hashes,
      'gaps':['Nonempty revisions/anchor reconstruction not validated','All nonempty entity families and typed events not exhaustively validated','Record shape cases do not simulate offer/manifest/threat visibility or stellar application','Revision empty batch and standalone event are framing-only, not admitted history','Full scoring/diplomacy/forecast/conservation simulations pending review','Coordinator byte cap absent from frozen source; no invented cap enforced']}

def main():
    check(len(sys.argv)==2 and sys.argv[1] in ('generate','verify'),'usage: derive.py generate|verify')
    result=corpus();raw=(json.dumps(result,ensure_ascii=False,indent=2)+'\n').encode()
    target=BASE/'generatedvectors.json'
    if sys.argv[1]=='generate':target.write_bytes(raw)
    else:check(target.read_bytes()==raw,'generated_corpus_drift')
    print(sys.argv[1]+': '+str(len(result['documents']))+' documents, '+str(len(result['negative_documents']))+' negatives, '+str(len(result['primitive_cases']))+' primitive/record cases, '+str(len(result['hash_vectors']))+' literal hash vectors')

if __name__=='__main__':main()
