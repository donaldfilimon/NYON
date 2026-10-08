#!/usr/bin/env python3
"""GEN-C fresh literal construction and bounded selected-profile fixture oracle.

No historical generator or accepted CAT helper is imported. This is not a
production decoder or an exhaustive richer-Genesis semantic matrix.
"""
import argparse
import base64
import copy
import hashlib
import json
import re
import struct
from pathlib import Path

from catalog import encoded as canonical, Parser, Reject, Node, core_catalog
PACK = hashlib.sha256(b'NYON_LIVING_V3_PACK\0' + canonical(core_catalog())).hexdigest()
CAP = 8 * 1024 * 1024
U64 = 2**64 - 1
KINDS = ('system', 'star', 'world', 'lane', 'civilization', 'deposit',
         'facility', 'construction_job', 'hull_job', 'fleet', 'hull', 'route',
         'shipment', 'hazard', 'offer', 'commitment', 'defense_request', 'stellar_event')
ARRAYS = ('systems', 'stars', 'worlds', 'lanes', 'civilizations', 'deposits',
          'colonies', 'facilities', 'construction_jobs', 'hull_jobs', 'fleets',
          'routes', 'shipments', 'relations', 'agreements', 'wars', 'observations',
          'hazards', 'settlement_claims', 'occupations', 'offers', 'demand_requests',
          'commitments', 'defense_requests', 'shared_intelligence', 'stellar_events')
LIMITS = (64, 128, 512, 256, 16, 1024, 512, 2048, 4096, 512, 256, 2048,
          4096, 240, 256, 120, 8192, 128, 8192, 512, 64, 1024, 256, 256, 4096, 128)
ISSUED = {'systems': 1, 'stars': 2, 'worlds': 3, 'lanes': 4,
          'civilizations': 5, 'deposits': 6, 'fleets': 10}
BASE_ISSUED = dict(ISSUED)
ISSUED.update(facilities=7, construction_jobs=8, hull_jobs=9, routes=12, shipments=13, hazards=14, offers=15, commitments=16, defense_requests=17, stellar_events=18)
SUPPORTED = set(BASE_ISSUED) | {'colonies', 'observations'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def domain(name):
    return ('NYON_LIVING_V3_' + name).encode('ascii') + b'\0'



def g_frame(genesis):
    generator = genesis['generator']; label = generator['generator_id'].encode('utf8')
    return (bytes.fromhex(genesis['catalog_hash']) + bytes.fromhex(genesis['seed'])
            + struct.pack('<I', len(label)) + label
            + struct.pack('<I', generator['generator_version']))


def allocation_frame(genesis, kind, ordinal):
    return domain('GENESIS_ENTITY') + g_frame(genesis) + struct.pack('<BHQ', 3, kind, ordinal)


def empty_state():
    return dict([('tick', '0'), ('branch_sequence', '0'), ('next_sequence', '0')]
                + [(name, []) for name in ARRAYS] + [('counters', {})])


def envelope():
    return {'kind': 'NYON_LIVING_GENESIS_V3', 'format_version': 3, 'rules_version': 3,
            'catalog_hash': PACK, 'seed': '00' * 32,
            'generator': {'generator_id': 'fixture_minimal', 'generator_version': 1},
            'allocations': [], 'state': empty_state()}


def environment():
    return dict(base_habitability=10000, stress=0, fertility=10000,
                maximum_fertility=10000, next_environment_tick='100')


def visibility(state, catalog, actor, world):
    worlds = {row['id']: row for row in state['worlds']}
    roles = {row['id']: row['role'] for row in catalog['hull_definitions']}
    if world['owner'] == actor:
        return True
    if any(c['owner'] == actor and worlds[c['world']]['system'] == world['system']
           for c in state['colonies']):
        return True
    for fleet in state['fleets']:
        if fleet['owner'] != actor or fleet['location']['type'] != 'docked':
            continue
        dock = worlds[fleet['location']['world']]
        if dock['id'] == world['id']:
            return True
        if dock['system'] == world['system'] and any(roles[h['definition']] == 'scout'
                                                    for h in fleet['hulls']):
            return True
    return False


def project(state, catalog):
    """Pure B5/B6: never consult submitted observations or history/actor captures."""
    output = []
    for actor in state['civilizations']:
        for world in state['worlds']:
            if not visibility(state, catalog, actor['id'], world):
                continue
            counts = {}
            for fleet in state['fleets']:
                if fleet['location'] == {'type': 'docked', 'world': world['id']}:
                    for hull in fleet['hulls']:
                        counts[hull['definition']] = counts.get(hull['definition'], 0) + 1
            env = {key: world['environment'][key] for key in
                   ('base_habitability', 'stress', 'fertility', 'maximum_fertility')}
            env['stellar_scope'] = dict(system=world['system'], originating_observer=actor['id'],
                                       observed_tick='0', events=[])
            scopes = [dict(lane=lane['id'], originating_observer=actor['id'], observed_tick='0', events=[])
                      for lane in state['lanes'] if world['system'] in (lane['system_a'], lane['system_b'])]
            output.append(dict(civilization=actor['id'], world=world['id'], observed_tick='0',
                               owner=world['owner'], remaining_deposit=str(sum(int(d['remaining_units'])
                               for d in state['deposits'] if d['world'] == world['id'])),
                               facilities=[], stationed_hulls=[dict(definition=k, count=v)
                               for k, v in sorted(counts.items())], operational_defense_batteries=0,
                               environment=env, threats=[], hazard_scopes=scopes))
    return sorted(output, key=lambda row: (row['civilization'], row['world']))


def inhabited(catalog):
    doc = envelope()
    ordinals = [(1, 1), (1, 2), (2, 1), (2, 2), (3, 1), (3, 2),
                (4, 1), (5, 1), (6, 1), (10, 1), (11, 1)]
    ids = {}
    for kind, ordinal in ordinals:
        ident = sha(allocation_frame(doc, kind, ordinal))[:32]
        ids[kind, ordinal] = ident
        doc['allocations'].append(dict(kind=kind, ordinal=str(ordinal), id=ident))
    s1, s2, w1, w2, actor = (ids[key] for key in ((1, 1), (1, 2), (3, 1), (3, 2), (5, 1)))
    state = doc['state']
    state['systems'] = [dict(id=s1, name='Home System'), dict(id=s2, name='Remote System')]
    state['stars'] = [dict(id=ids[2, i], system=system, archetype='main_sequence', name=name,
                           next_activity_tick='1000') for i, system, name in
                      ((1, s1, 'Home Star'), (2, s2, 'Remote Star'))]
    state['worlds'] = [dict(id=w1, system=s1, archetype='temperate', name='Home', owner=actor,
                            inventory=dict(energy=100, ore=100, alloy=120, food=100), environment=environment()),
                       dict(id=w2, system=s2, archetype='barren', name='Reach', owner=None,
                            inventory=dict(energy=0, ore=0, alloy=0, food=0), environment=environment())]
    state['lanes'] = [dict(id=ids[4, 1], system_a=min(s1, s2), system_b=max(s1, s2), distance_units='100')]
    state['civilizations'] = [dict(id=actor, name='Harbor', policy='neutral',
                                   research=dict(target=None, progress=[], completed_technologies=[]), status='active')]
    state['deposits'] = [dict(id=ids[6, 1], world=w1, resource='ore', remaining_units='20000')]
    state['colonies'] = [dict(world=w1, owner=actor,
                             hub=dict(energy_next_due='10', ore_next_due='100', fallback_next_due='100', blocked_reasons=[]),
                             society=dict(population=200, specialization='balanced', next_needs_tick='100',
                                          last_needs=None, workforce_assignments=[]))]
    scout = next(row for row in catalog['hull_definitions'] if row['id'] == 'scout')
    state['fleets'] = [dict(id=ids[10, 1], owner=actor, location=dict(type='docked', world=w1), order=None,
                           hulls=[dict(id=ids[11, 1], definition='scout', hit_points=scout['hit_points'],
                                       return_credit=False, next_repair=None, passengers=dict(count=0, voyage_origin=None))],
                           supply=dict(last_success=None, last_attempt=None, readiness='never_supplied'), blockade=None)]
    for name in ISSUED:
        state[name].sort(key=lambda row: bytes.fromhex(row['id']))
    state['observations'] = project(state, catalog)
    return doc


class Rejected(Exception):
    def __init__(self, stage, error, path, **details):
        self.stage, self.error, self.path = stage, error, path
        self.details = details


class Unsupported(Exception):
    def __init__(self, path):
        self.path = path


def fail(stage, error, path, **details):
    raise Rejected(stage, error, path, **details)


def ptr(path, key):
    return path + '/' + str(key).replace('~', '~0').replace('/', '~1')


def parse(raw):
    if len(raw) > CAP:
        fail(0, 'byte_capacity', '', actual=len(raw), limit=CAP)
    try:
        parser = Parser(raw.decode('utf8'))
        node = parser.parse()
        node.byte_offsets = parser.byte_offsets
        return node
    except UnicodeDecodeError as exc:
        fail(1, 'noncanonical_bytes', '', offset=exc.start)
    except Reject as exc:
        result = dict(exc.result)
        fail(result.pop('stage'), result.pop('error'), result.pop('path'), **result)


def canonical_difference(raw, expected, node):
    first = next((i for i, (a, b) in enumerate(zip(raw, expected)) if a != b), min(len(raw), len(expected)))
    offsets = node.byte_offsets
    path = ''
    pending = [node]
    while pending:
        current = pending.pop()
        if offsets[current.start] <= first < offsets[current.end]:
            path = current.path
            if current.kind == 'object': pending.extend(child for _, _, child in current.value)
            elif current.kind == 'array': pending.extend(current.value)
    fail(6, 'noncanonical_bytes', path, offset=first)


# Fresh record declarations, not imported from any decoder or historical fixture.
def rec(**fields):
    return ('record', fields)


def array(item, cap, key=None, issued=False):
    return ('array', item, cap, key, issued)


def bounded(item, minimum, maximum):
    return ('bounded', item, minimum, maximum)


def enum(*values):
    return ('enum', values)


def opt(item):
    return ('option', item)


LOCAL = bounded('u32', 0, 10000)
ENV = rec(base_habitability=LOCAL, stress=LOCAL, fertility=LOCAL, maximum_fertility=LOCAL, next_environment_tick='u64')
SYSTEM = rec(id='ID', name='Name')
STAR = rec(id='ID', system='ID', archetype='Slug', name='Name', next_activity_tick='u64')
WORLD = rec(id='ID', system='ID', archetype='Slug', name='Name', owner=opt('ID'),
            inventory=rec(energy=LOCAL, ore=LOCAL, alloy=LOCAL, food=LOCAL), environment=ENV)
LANE = rec(id='ID', system_a='ID', system_b='ID', distance_units='u64')
RESEARCH = rec(target=opt('Slug'), progress=array(rec(technology='Slug', points='u32'), 64, ('technology',)),
               completed_technologies=array('Slug', 64, ()))
CIV = rec(id='ID', name='Name', policy='Slug', research=RESEARCH, status=enum('active', 'dormant'))
DEPOSIT = rec(id='ID', world='ID', resource=enum('energy', 'ore', 'alloy', 'food'), remaining_units='u64')
NEEDS = rec(evaluated_tick='u64', population_before='u32', food_need='u32', energy_need='u32',
            food_paid='u32', energy_paid='u32', population_after='u32', outcome=enum('growth', 'decline', 'steady'),
            causes=array(enum('food_shortage', 'energy_shortage', 'above_capacity', 'growth_reserve_unavailable',
                              'zero_population', 'at_capacity', 'fully_paid'), 7, ()))
BLOCKED = ('tag', {'disabled': rec(type='tag'),
    'missing_deposit': rec(type='tag', required='u32', available='u32'),
    'missing_input': rec(type='tag', resource=enum('energy', 'ore', 'alloy', 'food'), required='u32', available='u32'),
    'output_full': rec(type='tag', resource=enum('energy', 'ore', 'alloy', 'food')),
    'workforce_unavailable': rec(type='tag', required='u32', available='u32'),
    'environmental_limit': rec(type='tag', effective_habitability='u32', fertility='u32'),
    'missing_research_target': rec(type='tag'),
    'technology_ineligible': rec(type='tag', technology='Slug'),
    'needs_reserved': rec(type='tag', world='ID', resource=enum('energy','ore','alloy','food'), physical_stock='u32', held='u32', required='u32')})
COLONY = rec(world='ID', owner='ID', hub=rec(energy_next_due='u64', ore_next_due='u64', fallback_next_due='u64',
             blocked_reasons=array(BLOCKED, None, ('@reason',))), society=rec(population=LOCAL,
             specialization=enum('balanced', 'agrarian', 'industrial', 'research'), next_needs_tick='u64',
             last_needs=opt(NEEDS), workforce_assignments=array('ID', 8, ())))
HULL = rec(id='ID', definition='Slug', hit_points='u32', return_credit='bool', next_repair=opt('u64'), passengers=rec(count='u32', voyage_origin=opt('ID')))
LOCATION = ('tag', {'docked': rec(type='tag', world='ID'), 'travelling': rec(type='tag', origin_world='ID',
            destination_world='ID', departure_tick='u64', arrival_tick='u64')})
ORDER = rec(kind=enum('attack', 'defend', 'explore', 'move', 'return', 'settle', 'blockade'), target_world=opt('ID'),
            not_before_tick='u64', lane_path=array('ID', 256), objective=opt(rec(war_a='ID', war_b='ID', actor='ID')), defense_request=opt(rec(request='ID', request_created_tick='u64', commitment='ID', commitment_created_tick='u64', threat_observation_digest='Digest')))
PAYMENT = rec(tick='u64', world='ID', supplies_paid='u32', deadline='u64')
ATTEMPT = rec(tick='u64', world='ID', required='u32', paid='u32', outcome=enum('paid', 'unavailable'))
FLEET = rec(id='ID', owner='ID', location=LOCATION, order=opt(ORDER), hulls=array(HULL, 16, ('id',), True),
            supply=rec(last_success=opt(PAYMENT), last_attempt=opt(ATTEMPT),
                       readiness=enum('supplied', 'supply_unavailable', 'never_supplied')),
            blockade=opt(rec(world='ID', war_a='ID', war_b='ID', accepted_tick='u64')))
FRACTION = rec(numerator='u32', denominator='u32')
KNOWN_EVENT = rec(id='ID', star='ID', system='ID', warning_tick='u64', start_tick='u64', end_tick='u64',
                  habitability_impact='u32', freight_factor=FRACTION, originating_observer='ID', observed_tick='u64')
KNOWN_HAZARD = rec(id='ID', definition='Slug', lane='ID', warning_tick='u64', start_tick='u64', end_tick='u64',
                   freight_factor=FRACTION, originating_observer='ID', observed_tick='u64')
PAIR = rec(participant_a='ID', participant_b='ID')
THREAT = rec(enemy='ID', target_world='ID', war=opt(PAIR), observed_tick='u64', originating_observer='ID',
             observation_snapshot_digest='Digest', hostile_escort_count='u32', occupation='bool',
             hostile_fleets=array('ID', None, ()))
SCOPE = rec(system='ID', originating_observer='ID', observed_tick='u64', events=array(KNOWN_EVENT, 128, ('id',)))
HAZARD_SCOPE = rec(lane='ID', originating_observer='ID', observed_tick='u64', events=array(KNOWN_HAZARD, 128, ('id',)))
SUMMARY = rec(definition='Slug', count='u32')
OBS = rec(civilization='ID', world='ID', observed_tick='u64', owner=opt('ID'), remaining_deposit='u64',
          facilities=array(SUMMARY, None, ('definition',)), stationed_hulls=array(SUMMARY, None, ('definition',)),
          operational_defense_batteries='u16', environment=rec(base_habitability=LOCAL, stress=LOCAL, fertility=LOCAL,
          maximum_fertility=LOCAL, stellar_scope=SCOPE), threats=array(THREAT, None, ('enemy', 'target_world', 'observed_tick', 'originating_observer')),
          hazard_scopes=array(HAZARD_SCOPE, 256, ('lane',)))
ROWS = dict(systems=SYSTEM, stars=STAR, worlds=WORLD, lanes=LANE, civilizations=CIV, deposits=DEPOSIT,
            colonies=COLONY, fleets=FLEET, observations=OBS)
from richer_schema import declare as richer_declarations
ROWS.update(richer_declarations(globals()))
ALL_ISSUED_ARRAYS = set(ARRAYS) - {'colonies', 'relations', 'agreements', 'wars', 'observations',
    'settlement_claims', 'occupations', 'demand_requests', 'shared_intelligence'}
KEYS = dict.fromkeys(ALL_ISSUED_ARRAYS, ('id',))
KEYS.update(colonies=('world',), relations=('from', 'to'),
    agreements=('participant_a', 'participant_b', 'kind'), wars=('participant_a', 'participant_b'),
    observations=('civilization', 'world'), settlement_claims=('world', 'civilization'),
    occupations=('world',), demand_requests=('actor', 'world', 'resource'),
    shared_intelligence=('recipient', 'originating_actor', 'world'))
STATE = rec(**dict([('tick', 'u64'), ('branch_sequence', 'u64'), ('next_sequence', 'u64')]
                  + [(name, array(ROWS[name], limit, KEYS[name], name in ALL_ISSUED_ARRAYS))
                     for name, limit in zip(ARRAYS, LIMITS)] + [('counters', rec())]))
GENESIS = rec(kind='tag', format_version='u32', rules_version='u32', catalog_hash='Digest', seed='Digest',
              generator=rec(generator_id='Slug', generator_version='u32'),
              allocations=array(rec(kind='u16', ordinal='u64', id='ID'), None, ('kind', 'ordinal')), state=STATE)


def check(node, schema, path='', ordered=None):
    """Declaration-ordered stage5 over tokenizer nodes; no float/negative-zero loss."""
    value = node.value
    if isinstance(schema, str):
        if schema == 'unsupported':
            raise Unsupported(path)
        if schema.startswith('u'):
            width = int(schema[1:])
            if width == 64:
                if node.kind != 'string': fail(5, 'invalid_range', path)
                if not re.fullmatch(r'0|[1-9][0-9]*', value): fail(5, 'noncanonical_bytes', path)
                if len(value) > 20 or int(value) > U64: fail(5, 'invalid_range', path)
            else:
                if node.kind != 'number' or not re.fullmatch(r'-?(0|[1-9][0-9]*)', value): fail(5, 'invalid_range', path)
                if value == '-0': fail(5, 'noncanonical_bytes', path)
                if len(value) > 10 or value.startswith('-') or int(value) >= 2**width: fail(5, 'invalid_range', path)
                value = int(value)
        elif schema == 'i32':
            if node.kind != 'string': fail(5, 'invalid_range', path)
            if not re.fullmatch(r'0|-?[1-9][0-9]*', value): fail(5, 'noncanonical_bytes', path)
            if len(value) > 11 or not -(2**31) <= int(value) < 2**31: fail(5, 'invalid_range', path)
        elif schema == 'bool':
            if node.kind != 'bool': fail(5, 'invalid_range', path)
        else:
            if node.kind != 'string': fail(5, 'invalid_range', path)
            if schema in ('ID', 'Digest'):
                width = 32 if schema == 'ID' else 64
                if len(value) != width: fail(5, 'invalid_range', path)
                if not re.fullmatch('[0-9a-f]{' + str(width) + '}', value): fail(5, 'noncanonical_bytes', path)
            if schema == 'Slug' and not re.fullmatch('[a-z][a-z0-9_]{0,63}', value): fail(5, 'invalid_range', path)
            if schema == 'Name' and (not 1 <= len(value) <= 64 or len(value.encode('utf8')) > 256 or
                    any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value) or value.startswith(' ') or
                    value.endswith(' ') or '  ' in value): fail(5, 'invalid_range', path)
        return value
    tag = schema[0]
    if tag == 'bounded':
        result = check(node, schema[1], path)
        if not schema[2] <= result <= schema[3]: fail(5, 'invalid_range', path)
        return result
    if tag == 'option': return None if node.kind == 'null' else check(node, schema[1], path)
    if tag == 'enum':
        if node.kind != 'string': fail(5, 'invalid_range', path)
        if value not in schema[1]: fail(5, 'invalid_enum', path)
        return value
    if tag in ('tag', 'record'):
        if node.kind != 'object': fail(5, 'invalid_range', path)
        members = {key: child for key, _, child in value}
        if tag == 'tag':
            discriminator = members.get('type')
            chosen = schema[1].get(discriminator.value) if discriminator and discriminator.kind == 'string' else None
            allowed = set(chosen[1]) if chosen else {k for variant in schema[1].values() for k in variant[1]}
        else: allowed = set(schema[1])
        extra = set(members) - allowed
        if extra: fail(5, 'unknown_field', ptr(path, min(extra, key=lambda x: x.encode('utf8'))))
        if tag == 'tag':
            if discriminator is None: fail(5, 'noncanonical_bytes', ptr(path, 'type'))
            if discriminator.kind != 'string': fail(5, 'invalid_range', ptr(path, 'type'))
            if chosen is None: fail(5, 'invalid_enum', ptr(path, 'type'))
            return check(node, chosen, path)
        result = {}
        for key, child in schema[1].items():
            if key not in members: fail(5, 'noncanonical_bytes', ptr(path, key))
            result[key] = check(members[key], child, ptr(path, key))
        return result
    if node.kind != 'array': fail(5, 'invalid_range', path)
    if schema[2] is not None and len(value) > schema[2]: fail(5, 'record_capacity', path, actual=len(value), limit=schema[2])
    result = [check(item, schema[1], ptr(path, i)) for i, item in enumerate(value)]
    previous = None
    keys_seen = set()
    for i, item in enumerate(result):
        keyfields = schema[3]
        if keyfields is None: continue
        if keyfields == ('@reason',): key = (item['type'], canonical(item))
        else: key = tuple(int(item[k]) if k in ('ordinal', 'observed_tick') or (k == 'kind' and type(item[k]) is int) else item[k] for k in keyfields) if keyfields else item
        if key in keys_seen and not schema[4]: fail(5, 'duplicate_key', ptr(path, i))
        if previous is not None and key < previous: fail(5, 'noncanonical_bytes', ptr(path, i))
        keys_seen.add(key)
        previous = key
    return result

def needs_reserved_semantics(value, path=''):
    # Schema4645–4650: widths are structural; this local cross-field predicate
    # does not authenticate a producer or permit a prior-attempt bootstrap.
    if value['physical_stock'] > 10000:
        fail(7, 'invalid_range', ptr(path, 'physical_stock'))
    if value['held'] > value['physical_stock']:
        fail(7, 'invalid_range', ptr(path, 'held'))
    if value['required'] <= value['physical_stock'] - value['held']:
        fail(7, 'invalid_range', ptr(path, 'required'))


def installed(state):
    rows = []
    for name in ARRAYS:
        if name not in ISSUED: continue
        for i,row in enumerate(state[name]):
            rows.append((ISSUED[name],row['id'],'/state/'+name+'/'+str(i)+'/id'))
            if name == 'fleets':
                rows.extend((11,hull['id'],'/state/fleets/'+str(i)+'/hulls/'+str(j)+'/id')
                            for j,hull in enumerate(row['hulls']))
    return rows


def validate(raw, catalog):
    node = parse(raw)
    if node.kind != 'object': fail(4, 'invalid_range', '/kind')
    members = {key: child for key, _, child in node.value}
    for key, expected in (('kind', 'NYON_LIVING_GENESIS_V3'), ('format_version', 3), ('rules_version', 3)):
        if key not in members: fail(4, 'noncanonical_bytes', '/' + key)
        child = members[key]
        try: value = check(child, 'tag' if key == 'kind' else 'u32', '/' + key)
        except Rejected as err: fail(4, err.error, err.path)
        if value != expected: fail(4, 'unsupported_kind' if key == 'kind' else 'unsupported_' + key, '/' + key)
    doc = check(node, GENESIS)
    expected_bytes = canonical(doc)
    if expected_bytes != raw: canonical_difference(raw, expected_bytes, node)
    state = doc['state']; seen = set()
    for _, ident, path in installed(state):
        if ident in seen:
            fail(7, 'duplicate_identity', path)
        seen.add(ident)
    if sum(len(f['hulls']) for f in state['fleets']) > 2048:
        fail(7, 'record_capacity', '/state/fleets', actual=sum(len(f['hulls']) for f in state['fleets']), limit=2048)
    if any(state[key] != '0' for key in ('tick', 'branch_sequence', 'next_sequence')):
        fail(7, 'invalid_range', '/state')
    allocations = doc['allocations']
    if any(row['kind'] not in range(1, 19) or int(row['ordinal']) == 0 for row in allocations):
        fail(7, 'invalid_range', '/allocations')
    declared = [(r['kind'], r['id']) for r in allocations]
    live = [(kind, ident) for kind, ident, _ in installed(state)]
    if len(declared) != len(live) or set(declared) != set(live):
        fail(7, 'invalid_range', '/allocations')
    richer = any(state[name] for name in ARRAYS if name not in SUPPORTED) or any(
        f['location']['type'] != 'docked' or any(h['definition'] in {d['id'] for d in catalog['hull_definitions'] if d['role'] != 'scout'} for h in f['hulls'])
        for f in state['fleets'])
    if not richer:
        definitions = {row['id']: row for row in catalog['hull_definitions']}
        # No selected-record-count equality: B1 gives general bounded explicit base rows.
        issues = []
        def require(ok, path):
            if not ok:
                issues.append(path)
        hulls = {row['id']: row for row in catalog['hull_definitions']}
        resources = {row['id']: row for row in catalog['resources']}
        for i, star in enumerate(state['stars']):
            require(star['next_activity_tick'] == '1000', '/state/stars/' + str(i) + '/next_activity_tick')
        for i, world in enumerate(state['worlds']):
            require(all(q <= resources[r]['storage_limit'] and q <= 10000 for r, q in world['inventory'].items()), '/state/worlds/' + str(i) + '/inventory')
            env = world['environment']
            require(all(env[k] <= 10000 for k in ('base_habitability', 'stress', 'fertility', 'maximum_fertility')) and
                    env['fertility'] <= env['maximum_fertility'], '/state/worlds/' + str(i) + '/environment')
            require(env['next_environment_tick'] == '100', '/state/worlds/' + str(i) + '/environment/next_environment_tick')
        for i, civ in enumerate(state['civilizations']):
            owns = any(c['owner'] == civ['id'] for c in state['colonies'])
            require(civ['status'] == ('active' if owns else 'dormant'), '/state/civilizations/' + str(i) + '/status')
            require(civ['research'] == dict(target=None, progress=[], completed_technologies=[]), '/state/civilizations/' + str(i) + '/research')
            require(civ['id'] != '00' * 16, '/state/civilizations/' + str(i) + '/id')
        worlds = {w['id']: w for w in state['worlds']}
        for i, colony in enumerate(state['colonies']):
            prefix = '/state/colonies/' + str(i)
            if colony['world'] in worlds:
                require(colony['owner'] == worlds[colony['world']]['owner'], prefix + '/owner')
            require(colony['hub'] == dict(energy_next_due='10', ore_next_due='100', fallback_next_due='100', blocked_reasons=[]), prefix + '/hub')
            society = colony['society']
            require(society['population'] <= 10000 and society['next_needs_tick'] == '100' and society['last_needs'] is None and
                    society['workforce_assignments'] == [], prefix + '/society')
        for i, lane in enumerate(state['lanes']):
            require(lane['system_a'] != lane['system_b'], '/state/lanes/' + str(i))
        for i, fleet in enumerate(state['fleets']):
            prefix = '/state/fleets/' + str(i)
            require(bool(fleet['hulls']) and fleet['location']['type'] == 'docked' and fleet['order'] is None and fleet['blockade'] is None, prefix)
            require(fleet['supply'] == dict(last_success=None, last_attempt=None, readiness='never_supplied'), prefix + '/supply')
            for j, hull in enumerate(fleet['hulls']):
                if hull['definition'] in hulls:
                    require(hull['hit_points'] == hulls[hull['definition']]['hit_points'] and hull['passengers'] == dict(count=0, voyage_origin=None) and
                            hull['next_repair'] is None and hull['return_credit'] is False, prefix + '/hulls/' + str(j))
        if issues:
            # These helper paths are private; exported semantic cases are rejection-only.
            fail(7, 'invalid_range', issues[0])
    if doc['catalog_hash'] != sha(domain('PACK') + canonical(catalog)):
        fail(8, 'catalog_mismatch', '/catalog_hash')
    bykind = {kind: {row['id'] for row in state[name]} for name, kind in ISSUED.items()}
    bykind[11] = {h['id'] for f in state['fleets'] for h in f['hulls']}
    allids = {ident for _, ident, _ in installed(state)}
    reference_failures = []
    def reference(ident, kind, path):
        if ident is None:
            return
        if ident not in bykind[kind]:
            reference_failures.append(('wrong_reference_kind' if ident in allids else 'missing_reference', path))
    def slug(value, collection, path):
        if value not in {row['id'] for row in catalog[collection]}:
            reference_failures.append(('missing_reference', path))
    for name in ('stars', 'worlds'):
        for i, row in enumerate(state[name]):
            reference(row['system'], 1, '/state/' + name + '/' + str(i) + '/system')
            slug(row['archetype'], 'star_archetypes' if name == 'stars' else 'world_archetypes', '/state/' + name + '/' + str(i) + '/archetype')
            if name == 'worlds':
                reference(row['owner'], 5, '/state/worlds/' + str(i) + '/owner')
    for i, row in enumerate(state['lanes']):
        for key in ('system_a', 'system_b'):
            reference(row[key], 1, '/state/lanes/' + str(i) + '/' + key)
    for i, row in enumerate(state['civilizations']):
        slug(row['policy'], 'policy_definitions', '/state/civilizations/' + str(i) + '/policy')
    for name in ('deposits', 'colonies', 'fleets'):
        for i, row in enumerate(state[name]):
            prefix = '/state/' + name + '/' + str(i)
            if name != 'fleets': reference(row['world'], 3, prefix + '/world')
            elif row['location']['type'] == 'docked': reference(row['location']['world'], 3, prefix + '/location/world')
            else:
                for key in ('origin_world','destination_world'): reference(row['location'][key],3,prefix+'/location/'+key)
            if name != 'deposits':
                reference(row['owner'], 5, prefix + '/owner')
            if name == 'fleets':
                for j, hull in enumerate(row['hulls']):
                    slug(hull['definition'], 'hull_definitions', prefix + '/hulls/' + str(j) + '/definition')
    for i, row in enumerate(state['observations']):
        prefix = '/state/observations/' + str(i)
        reference(row['civilization'], 5, prefix + '/civilization')
        reference(row['world'], 3, prefix + '/world')
        reference(row['owner'], 5, prefix + '/owner')
        scope = row['environment']['stellar_scope']
        reference(scope['system'], 1, prefix + '/environment/stellar_scope/system')
        reference(scope['originating_observer'], 5, prefix + '/environment/stellar_scope/originating_observer')
        for j, scope in enumerate(row['hazard_scopes']):
            reference(scope['lane'], 4, prefix + '/hazard_scopes/' + str(j) + '/lane')
            reference(scope['originating_observer'], 5, prefix + '/hazard_scopes/' + str(j) + '/originating_observer')
        for j, hull in enumerate(row['stationed_hulls']):
            slug(hull['definition'], 'hull_definitions', prefix + '/stationed_hulls/' + str(j) + '/definition')
    from richer_references import collect as richer_reference_checks
    richer_reference_checks(state, reference, slug)
    if reference_failures:
        # Walk the already schema-ordered document, giving arrays numeric input
        # indices. Error family is the outer key, never traversal encounter order.
        order = {}
        def rank_paths(value, path=''):
            order[path] = len(order)
            if isinstance(value, dict):
                for key, child in value.items(): rank_paths(child, ptr(path, key))
            elif isinstance(value, list):
                for i, child in enumerate(value): rank_paths(child, ptr(path, i))
        rank_paths(doc)
        priority = {'missing_reference': 0, 'wrong_reference_kind': 1}
        error, path = min(reference_failures, key=lambda item: (priority[item[0]], order[item[1]]))
        fail(8, error, path)
    for i, row in enumerate(allocations):
        if row['id'] != sha(allocation_frame(doc, row['kind'], int(row['ordinal'])))[:32]:
            fail(9, 'integrity_mismatch', '/allocations/' + str(i) + '/id')
    if richer:
        raise Unsupported('/state')
    # B1 sources are authenticated before any B5 source read; a profile check,
    # not a claim of exhaustive production stage7/8/9 multi-defect precedence.
    expected = project(state, catalog)
    if any(int(row['remaining_deposit']) > U64 for row in expected):
        fail(7, 'invalid_range', '/state/observations')
    if canonical(state['observations']) != canonical(expected):
        fail(7, 'invalid_range', '/state/observations')
    return doc


def sealed(doc):
    state = canonical(doc['state']); genesis = canonical(doc)
    wrapper = canonical(dict(format_version=3, rules_version=3, catalog_hash=doc['catalog_hash'], state=doc['state']))
    state_preimage, genesis_preimage = domain('STATE') + wrapper, domain('GENESIS') + genesis
    digest = sha(genesis_preimage); generator = doc['generator']; label = generator['generator_id'].encode('utf8')
    root_frame = (domain('ROOT_BRANCH') + bytes.fromhex(doc['catalog_hash']) + bytes.fromhex(doc['seed'])
                  + bytes.fromhex(digest) + struct.pack('<I', len(label)) + label
                  + struct.pack('<I', generator['generator_version']))
    root = sha(root_frame)[:32]
    branch = dict(id=root, parent=None, fork_revision=None, fork_tick='0', fork_state_digest=sha(state_preimage),
                  ordinal='0', head=None, tip_tick='0')
    view = dict(branch=root, revision=None, tick='0')
    frames = [dict(kind=row['kind'], ordinal=row['ordinal'], frame_hex=allocation_frame(doc, row['kind'], int(row['ordinal'])).hex(),
                   full_digest=sha(allocation_frame(doc, row['kind'], int(row['ordinal']))), id=row['id']) for row in doc['allocations']]
    return dict(G_hex=g_frame(doc).hex(), issuances=frames, state_bytes_base64=base64.b64encode(state).decode(),
                state_wrapper_bytes_base64=base64.b64encode(wrapper).decode(),
                state_preimage_base64=base64.b64encode(state_preimage).decode(), state_digest=sha(state_preimage),
                genesis_preimage_base64=base64.b64encode(genesis_preimage).decode(), genesis_digest=digest,
                root_preimage_hex=root_frame.hex(), root_full_digest=sha(root_frame), root_branch=branch,
                history_view=view, history_view_bytes_base64=base64.b64encode(canonical(view)).decode(),
                allocator_marks=dict(accepted_sequence_high_water='0', branch_sequence_high_water='0'),
                revisions=[], anchors=[], queue_harness=dict(pending_commands=0, pending_tail='0'))


def context_matches(doc, context):
    expected = sealed(doc)
    return all(context.get(key) == expected[key] for key in
               ('root_branch', 'history_view', 'allocator_marks', 'revisions', 'anchors'))


def outcome(raw, catalog):
    try:
        validate(raw, catalog)
        return dict(admitted=True)
    except Rejected as error:
        return dict(admitted=False, stage=error.stage, error=error.error, path=error.path, **error.details)
    except Unsupported as unsupported:
        return dict(coverage='unsupported_family', first_unexamined_path=unsupported.path)


def mutation(doc, path, value=None, delete=False):
    result = copy.deepcopy(doc); cursor = result
    keys = path.split('/')[1:]
    for key in keys[:-1]:
        cursor = cursor[int(key)] if isinstance(cursor, list) else cursor[key]
    key = int(keys[-1]) if isinstance(cursor, list) else keys[-1]
    if delete:
        del cursor[key]
    else:
        cursor[key] = value
    return result

