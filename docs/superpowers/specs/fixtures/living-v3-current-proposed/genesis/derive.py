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

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[5]
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
CAT = HERE.parent
FREEZE = SDD / 'v3-current-release-source-freeze'
PINS = {
    'v3-current-release-source-freeze/manifest.json': 'd0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c',
    'v3-coordinated-amendment-current-source-manifest-2.json': 'bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f',
    'v3-coordinated-amendment-integrated-review.md': 'cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65',
    'v3-anchor-summary-rereview-1.md': '947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf',
    'v3-current-catalog-review.md': 'd556d3230b34a167c997bbbfb58e6a6308e517d5fa006b23c806dfb6f0ded9f6',
    'v3-current-genesis-admission-preflight.md': '2e7b88ed1a34fd5d7e90fad9ece7431ece19aed0fe45de9cc66d531a3c116793',
}
CAT_PINS = {
    'derive.py': 'c8d772c48420b1dcf1c3598ceba35bc6ab59baa08e87fb7bd5e2fb917757ae51',
    'core.catalog.json': 'efc762006d5abfa3e3a00d3aa9ba46b001e9e1951833b1c655ab6518b18f3f10',
    'vectors.json': 'e1f1acde5bd98c2795e81886cbf3e3db731bdcf8281ca437ecbfd3b4dacb6ca2',
    'predicate-coverage.json': 'edb40f42a3c07a00bf0f92e69f03ce77b2bfeae714f1a1d33c0b3229afd5a394',
    'README.md': '385502ca5144f82d778b4d91f271eedd49e7668a13efc2d9bd5257d82c18161b',
}
PACK = '9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'
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
SUPPORTED = set(ISSUED) | {'colonies', 'observations'}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def domain(name):
    return ('NYON_LIVING_V3_' + name).encode('ascii') + b'\0'


def canonical(value):
    # Declaration insertion order; no sort/default/Unicode normalization.
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf8')


def freshness():
    for name, digest in PINS.items():
        if sha((SDD / name).read_bytes()) != digest:
            raise SystemExit('STOP: dependency changed ' + name)
    manifest = json.loads((FREEZE / 'manifest.json').read_bytes())
    if len(manifest['sources']) != 8:
        raise SystemExit('STOP: eight-source roster changed')
    for name, expected in manifest['sources'].items():
        for file in (FREEZE / name, ROOT / 'docs/superpowers/specs' / name):
            raw = file.read_bytes()
            if len(raw) != expected['bytes'] or sha(raw) != expected['sha256']:
                raise SystemExit('STOP: source changed ' + str(file))
    for name, digest in CAT_PINS.items():
        if sha((CAT / name).read_bytes()) != digest:
            raise SystemExit('STOP: accepted CAT file changed ' + name)
    raw = (CAT / 'core.catalog.json').read_bytes()
    if len(raw) != 4264 or sha(domain('PACK') + raw) != PACK:
        raise SystemExit('STOP: accepted Catalog/PACK differs')
    return json.loads(raw), manifest['sources']


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
                                       return_credit=False, next_repair=None, passengers=dict(count=0))],
                           supply=dict(last_success=None, last_attempt=None, readiness='never_supplied'), blockade=None)]
    for name in ISSUED:
        state[name].sort(key=lambda row: bytes.fromhex(row['id']))
    state['observations'] = project(state, catalog)
    return doc


class Rejected(Exception):
    def __init__(self, stage, error, path):
        self.stage, self.error, self.path = stage, error, path


class Unsupported(Exception):
    def __init__(self, path):
        self.path = path


def fail(stage, error, path):
    raise Rejected(stage, error, path)


def ptr(path, key):
    return path + '/' + str(key).replace('~', '~0').replace('/', '~1')


def parse(raw):
    """Explicit stack tokenizer; duplicate pairs survive until syntax completes."""
    if len(raw) > CAP:
        fail(0, 'byte_capacity', '')
    try:
        text = raw.decode('utf8')
    except UnicodeDecodeError:
        fail(1, 'noncanonical_bytes', '')
    decoder = json.JSONDecoder()
    stack, root, done, pos, duplicates = [], None, False, 0, []

    def append(value):
        nonlocal root, done
        if not stack:
            root, done = value, True
        else:
            frame = stack[-1]
            if frame['object']:
                frame['value'].append((frame['key'], value))
            else:
                frame['value'].append(value)
            frame['state'] = 'comma'

    def token():
        nonlocal pos
        start = pos
        try:
            value, pos = decoder.raw_decode(text, pos)
        except (ValueError, RecursionError):
            fail(2, 'noncanonical_bytes', '')
        # Containers are never delegated to the recursive stdlib document parser.
        if isinstance(value, (dict, list)) or (isinstance(value, str) and
                any(0xd800 <= ord(c) <= 0xdfff for c in value)):
            fail(2, 'noncanonical_bytes', '')
        if isinstance(value, float) and not (-float('inf') < value < float('inf')):
            fail(2, 'noncanonical_bytes', '')
        return value, start

    while True:
        while pos < len(text) and text[pos] in ' \t\r\n':
            pos += 1
        if done and not stack:
            if pos != len(text):
                fail(2, 'noncanonical_bytes', '')
            break
        if pos >= len(text):
            fail(2, 'noncanonical_bytes', '')
        frame = stack[-1] if stack else None
        if frame and frame['state'] == 'comma':
            closing = '}' if frame['object'] else ']'
            if text[pos] == closing:
                pos += 1; stack.pop()
                append(('OBJECT', frame['value']) if frame['object'] else frame['value'])
                continue
            if text[pos] != ',':
                fail(2, 'noncanonical_bytes', '')
            pos += 1; frame['state'] = 'key' if frame['object'] else 'value'
            continue
        if frame and frame['object'] and frame['state'] in ('key', 'first_key'):
            if text[pos] == '}' and frame['state'] == 'first_key':
                pos += 1; stack.pop(); append(('OBJECT', [])); continue
            if text[pos] != '"':
                fail(2, 'noncanonical_bytes', '')
            key, offset = token()
            if key in frame['seen']:
                duplicates.append((offset, ptr(frame['path'], key)))
            frame['seen'].add(key); frame['key'] = key; frame['state'] = 'colon'
            continue
        if frame and frame['state'] == 'colon':
            if text[pos] != ':':
                fail(2, 'noncanonical_bytes', '')
            pos += 1; frame['state'] = 'value'; continue
        if frame and not frame['object'] and frame['state'] == 'first_value' and text[pos] == ']':
            pos += 1; stack.pop(); append([]); continue
        path = '' if not frame else ptr(frame['path'], frame['key'] if frame['object'] else len(frame['value']))
        if text[pos] in '{[':
            if len(stack) == 64:
                fail(2, 'depth_capacity', '')
            obj = text[pos] == '{'; pos += 1
            stack.append(dict(object=obj, value=[], seen=set(), key=None, path=path,
                              state='first_key' if obj else 'first_value'))
        else:
            value, _ = token(); append(value)
    if duplicates:
        fail(3, 'duplicate_key', min(duplicates)[1])

    def materialize(value):
        if isinstance(value, tuple):
            return {k: materialize(v) for k, v in value[1]}
        if isinstance(value, list):
            return [materialize(v) for v in value]
        return value
    return materialize(root)


# Fresh record declarations, not imported from any decoder or historical fixture.
def rec(**fields):
    return ('record', fields)


def array(item, cap, key=None, issued=False):
    return ('array', item, cap, key, issued)


def enum(*values):
    return ('enum', values)


def opt(item):
    return ('option', item)


ENV = rec(base_habitability='u32', stress='u32', fertility='u32', maximum_fertility='u32', next_environment_tick='u64')
SYSTEM = rec(id='ID', name='Name')
STAR = rec(id='ID', system='ID', archetype='Slug', name='Name', next_activity_tick='u64')
WORLD = rec(id='ID', system='ID', archetype='Slug', name='Name', owner=opt('ID'),
            inventory=rec(energy='u32', ore='u32', alloy='u32', food='u32'), environment=ENV)
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
    'technology_ineligible': rec(type='tag', technology='Slug')})
COLONY = rec(world='ID', owner='ID', hub=rec(energy_next_due='u64', ore_next_due='u64', fallback_next_due='u64',
             blocked_reasons=array(BLOCKED, None)), society=rec(population='u32',
             specialization=enum('balanced', 'agrarian', 'industrial', 'research'), next_needs_tick='u64',
             last_needs=opt(NEEDS), workforce_assignments=array('ID', 8, ())))
HULL = rec(id='ID', definition='Slug', hit_points='u32', return_credit='bool', next_repair=opt('u64'), passengers=rec(count='u32'))
LOCATION = ('tag', {'docked': rec(type='tag', world='ID'), 'travelling': rec(type='tag', origin_world='ID',
            destination_world='ID', departure_tick='u64', arrival_tick='u64')})
ORDER = rec(kind=enum('attack', 'defend', 'explore', 'move', 'return', 'settle', 'blockade'), target_world=opt('ID'),
            not_before_tick='u64', lane_path=array('ID', 256), objective=opt('unsupported'))
PAYMENT = rec(tick='u64', world='ID', supplies_paid='u32', deadline='u64')
ATTEMPT = rec(tick='u64', world='ID', required='u32', paid='u32', outcome=enum('paid', 'unavailable'))
FLEET = rec(id='ID', owner='ID', location=LOCATION, order=opt(ORDER), hulls=array(HULL, 16, ('id',), True),
            supply=rec(last_success=opt(PAYMENT), last_attempt=opt(ATTEMPT),
                       readiness=enum('supplied', 'supply_unavailable', 'never_supplied')),
            blockade=opt(rec(world='ID', war_a='ID', war_b='ID', accepted_tick='u64')))
SCOPE = rec(system='ID', originating_observer='ID', observed_tick='u64', events=array('unsupported', 128))
HAZARD_SCOPE = rec(lane='ID', originating_observer='ID', observed_tick='u64', events=array('unsupported', 128))
OBS = rec(civilization='ID', world='ID', observed_tick='u64', owner=opt('ID'), remaining_deposit='u64',
          facilities=array('unsupported', None), stationed_hulls=array(rec(definition='Slug', count='u32'), None, ('definition',)),
          operational_defense_batteries='u16', environment=rec(base_habitability='u32', stress='u32', fertility='u32',
          maximum_fertility='u32', stellar_scope=SCOPE), threats=array('unsupported', None),
          hazard_scopes=array(HAZARD_SCOPE, 256, ('lane',)))
ROWS = dict(systems=SYSTEM, stars=STAR, worlds=WORLD, lanes=LANE, civilizations=CIV, deposits=DEPOSIT,
            colonies=COLONY, fleets=FLEET, observations=OBS)
ALL_ISSUED_ARRAYS = set(ARRAYS) - {'colonies', 'relations', 'agreements', 'wars', 'observations',
    'settlement_claims', 'occupations', 'demand_requests', 'shared_intelligence'}
KEYS = dict.fromkeys(ALL_ISSUED_ARRAYS, ('id',))
KEYS.update(colonies=('world',), relations=('from', 'to'),
    agreements=('participant_a', 'participant_b', 'kind'), wars=('participant_a', 'participant_b'),
    observations=('civilization', 'world'), settlement_claims=('world', 'civilization'),
    occupations=('world',), demand_requests=('actor', 'world', 'resource'),
    shared_intelligence=('recipient', 'originating_actor', 'world'))
STATE = rec(**dict([('tick', 'u64'), ('branch_sequence', 'u64'), ('next_sequence', 'u64')]
                  + [(name, array(ROWS.get(name, 'unsupported'), limit, KEYS[name], name in ALL_ISSUED_ARRAYS))
                     for name, limit in zip(ARRAYS, LIMITS)] + [('counters', rec())]))
GENESIS = rec(kind='tag', format_version='u32', rules_version='u32', catalog_hash='Digest', seed='Digest',
              generator=rec(generator_id='Slug', generator_version='u32'),
              allocations=array(rec(kind='u16', ordinal='u64', id='ID'), None, ('kind', 'ordinal')), state=STATE)


def check(value, schema, path='', ordered=None):
    """Stage5 reconstructs required order without repairing arrays or values."""
    if isinstance(schema, str):
        if schema == 'unsupported':
            raise Unsupported(path)
        if schema.startswith('u'):
            width = int(schema[1:])
            if width == 64:
                if not isinstance(value, str):
                    fail(5, 'invalid_range', path)
                if not re.fullmatch(r'0|[1-9][0-9]*', value):
                    fail(5, 'noncanonical_bytes', path)
                if len(value) > 20 or int(value) > U64:
                    fail(5, 'invalid_range', path)
            elif type(value) is not int or not 0 <= value < 2**width:
                fail(5, 'invalid_range', path)
        elif schema == 'bool':
            if type(value) is not bool:
                fail(5, 'invalid_range', path)
        else:
            if not isinstance(value, str):
                fail(5, 'invalid_range', path)
            if schema in ('ID', 'Digest') and not re.fullmatch('[0-9a-f]{' + ('32' if schema == 'ID' else '64') + '}', value):
                fail(5, 'noncanonical_bytes', path)
            if schema == 'Slug' and not re.fullmatch('[a-z][a-z0-9_]{0,63}', value):
                fail(5, 'invalid_range', path)
            if schema == 'Name' and (not 1 <= len(value) <= 64 or len(value.encode('utf8')) > 256 or
                    any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value) or value.startswith(' ') or
                    value.endswith(' ') or '  ' in value):
                fail(5, 'invalid_range', path)
        return value
    tag = schema[0]
    if tag == 'option':
        return None if value is None else check(value, schema[1], path)
    if tag == 'enum':
        if not isinstance(value, str):
            fail(5, 'invalid_range', path)
        if value not in schema[1]:
            fail(5, 'invalid_enum', path)
        return value
    if tag == 'tag':
        if not isinstance(value, dict):
            fail(5, 'invalid_range', path)
        if 'type' not in value:
            fail(5, 'noncanonical_bytes', ptr(path, 'type'))
        if not isinstance(value['type'], str):
            fail(5, 'invalid_range', ptr(path, 'type'))
        if value['type'] not in schema[1]:
            fail(5, 'invalid_enum', ptr(path, 'type'))
        return check(value, schema[1][value['type']], path)
    if tag == 'record':
        if not isinstance(value, dict):
            fail(5, 'invalid_range', path)
        extra = set(value) - set(schema[1])
        if extra:
            fail(5, 'unknown_field', ptr(path, min(extra, key=lambda x: x.encode('utf8'))))
        result = {}
        for key, child in schema[1].items():
            if key not in value:
                fail(5, 'noncanonical_bytes', ptr(path, key))
            result[key] = check(value[key], child, ptr(path, key))
        return result
    if not isinstance(value, list):
        fail(5, 'invalid_range', path)
    if schema[2] is not None and len(value) > schema[2]:
        fail(5, 'record_capacity', path)
    result = [check(item, schema[1], ptr(path, i)) for i, item in enumerate(value)]
    previous = None
    for i, item in enumerate(result):
        keyfields = schema[3]
        if keyfields is None:
            continue
        key = tuple(int(item[k]) if k in ('kind', 'ordinal') else item[k] for k in keyfields) if keyfields else item
        if previous is not None:
            if key == previous and not schema[4]:
                fail(5, 'duplicate_key', ptr(path, i))
            if key < previous:
                fail(5, 'noncanonical_bytes', ptr(path, i))
        previous = key
    return result


def installed(state):
    rows = []
    for name in ARRAYS:
        if name in ISSUED:
            rows.extend((ISSUED[name], row['id'], '/state/' + name + '/' + str(i) + '/id')
                        for i, row in enumerate(state[name]))
        if name == 'fleets':
            rows.extend((11, hull['id'], '/state/fleets/' + str(i) + '/hulls/' + str(j) + '/id')
                        for i, fleet in enumerate(state[name]) for j, hull in enumerate(fleet['hulls']))
    return rows


def validate(raw, catalog):
    doc = parse(raw)
    if not isinstance(doc, dict):
        fail(4, 'invalid_range', '/kind')
    for key, expected in (('kind', 'NYON_LIVING_GENESIS_V3'), ('format_version', 3), ('rules_version', 3)):
        if key not in doc:
            fail(4, 'noncanonical_bytes', '/' + key)
        value = doc[key]
        if (key == 'kind' and not isinstance(value, str)) or (key != 'kind' and
                (type(value) is not int or not 0 <= value <= 2**32 - 1)):
            fail(4, 'invalid_range', '/' + key)
        if value != expected:
            fail(4, 'unsupported_kind' if key == 'kind' else 'unsupported_' + key, '/' + key)
    ordered = check(doc, GENESIS)
    if canonical(ordered) != raw:
        fail(6, 'noncanonical_bytes', '')  # Export only stage/error for alternate byte spelling.
    state = doc['state']; seen = set()
    for _, ident, path in installed(state):
        if ident in seen:
            fail(7, 'duplicate_identity', path)
        seen.add(ident)
    if any(state[key] != '0' for key in ('tick', 'branch_sequence', 'next_sequence')):
        fail(7, 'invalid_range', '/state')
    allocations = doc['allocations']
    if any(row['kind'] not in range(1, 19) or int(row['ordinal']) == 0 for row in allocations):
        fail(7, 'invalid_range', '/allocations')
    declared = [(r['kind'], r['id']) for r in allocations]
    live = [(kind, ident) for kind, ident, _ in installed(state)]
    if len(declared) != len(live) or set(declared) != set(live):
        fail(7, 'invalid_range', '/allocations')
    counts = {kind: sum(k == kind for k, _ in live) for kind in range(1, 19)}
    expected_counts = {1: 2, 2: 2, 3: 2, 4: 1, 5: 1, 6: 1, 10: 1, 11: 1}
    if live and any(counts[k] != expected_counts.get(k, 0) for k in counts):
        raise Unsupported('/state')
    definitions = {row['id']: row for row in catalog['hull_definitions']}
    if live and any(f['location']['type'] != 'docked' or any(h['definition'] in definitions and
            definitions[h['definition']]['role'] != 'scout' for h in f['hulls']) for f in state['fleets']):
        raise Unsupported('/state/fleets')
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
        require(lane['system_a'] < lane['system_b'], '/state/lanes/' + str(i))
    for i, fleet in enumerate(state['fleets']):
        prefix = '/state/fleets/' + str(i)
        require(bool(fleet['hulls']) and fleet['location']['type'] == 'docked' and fleet['order'] is None and fleet['blockade'] is None, prefix)
        require(fleet['supply'] == dict(last_success=None, last_attempt=None, readiness='never_supplied'), prefix + '/supply')
        for j, hull in enumerate(fleet['hulls']):
            if hull['definition'] in hulls:
                require(hull['hit_points'] == hulls[hull['definition']]['hit_points'] and hull['passengers']['count'] == 0 and
                        hull['next_repair'] is None and hull['return_credit'] is False, prefix + '/hulls/' + str(j))
    if issues:
        # These helper paths are private; exported semantic cases are rejection-only.
        fail(7, 'invalid_range', issues[0])
    if doc['catalog_hash'] != PACK:
        fail(8, 'catalog_mismatch', '/catalog_hash')
    bykind = {kind: {row['id'] for row in state[name]} for name, kind in ISSUED.items()}
    allids = {ident for _, ident, _ in installed(state)}
    def reference(ident, kind, path):
        if ident is None:
            return
        if ident not in bykind[kind]:
            fail(8, 'wrong_reference_kind' if ident in allids else 'missing_reference', path)
    def slug(value, collection, path):
        if value not in {row['id'] for row in catalog[collection]}:
            fail(8, 'missing_reference', path)
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
            reference(row['world'] if name != 'fleets' else row['location']['world'], 3, prefix + '/world')
            if name != 'deposits':
                reference(row['owner'], 5, prefix + '/owner')
            if name == 'fleets':
                for j, hull in enumerate(row['hulls']):
                    slug(hull['definition'], 'hull_definitions', prefix + '/hulls/' + str(j) + '/definition')
    for i, row in enumerate(allocations):
        if row['id'] != sha(allocation_frame(doc, row['kind'], int(row['ordinal'])))[:32]:
            fail(9, 'integrity_mismatch', '/allocations/' + str(i) + '/id')
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
        return dict(admitted=False, stage=error.stage, error=error.error, path=error.path)
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


def build(catalog, sources):
    empty, full = envelope(), inhabited(catalog)
    cases = []
    def add(name, raw, expectation, strength, predicate):
        actual = outcome(raw, catalog)
        if strength == 'rejection_only':
            assert actual.get('admitted') is False, (name, actual)
        else:
            for key, value in expectation.items():
                assert actual.get(key) == value, (name, actual, expectation)
        cases.append(dict(id=name, raw_base64=base64.b64encode(raw).decode(), strength=strength,
                          expectation=expectation, predicate=predicate))
    for name, doc in (('empty', empty), ('inhabited', full)):
        add('positive.' + name, canonical(doc), dict(admitted=True), 'complete_genesis_profile_admission', 'B0-B7')
    # Complete alternative choices demonstrate that fixture literals are not extra rules.
    choices = [('name_unicode', '/state/systems/0/name', 'Harbor Ω'),
               ('stocks_zero', '/state/worlds/' + str(next(i for i, w in enumerate(full['state']['worlds']) if w['owner'])) + '/inventory', dict(energy=0, ore=0, alloy=0, food=0)),
               ('population_zero', '/state/colonies/0/society/population', 0)]
    for name, path, value in choices:
        alt = mutation(full, path, value)
        add('positive.choice.' + name, canonical(alt), dict(admitted=True),
            'complete_genesis_profile_admission', 'valid-input-choice/B0-B7')
    # Noncontiguous positive per-kind ordinals are permitted; recompute every ID/reference.
    alt = copy.deepcopy(full); mapping = {}
    for row in alt['allocations']:
        prior = row['id']; row['ordinal'] = str(int(row['ordinal']) + 10)
        mapping[prior] = sha(allocation_frame(alt, row['kind'], int(row['ordinal'])))[:32]
    def remap(value):
        if isinstance(value, str): return mapping.get(value, value)
        if isinstance(value, list): return [remap(v) for v in value]
        if isinstance(value, dict): return {k: remap(v) for k, v in value.items()}
        return value
    alt = remap(alt)
    for name in ISSUED: alt['state'][name].sort(key=lambda r: r['id'])
    for lane in alt['state']['lanes']:
        lane['system_a'], lane['system_b'] = sorted((lane['system_a'], lane['system_b']))
    alt['state']['observations'] = project(alt['state'], catalog)
    add('positive.choice.noncontiguous_ordinals', canonical(alt), dict(admitted=True),
        'complete_genesis_profile_admission', 'positive unique ordinals need not be contiguous/B1')
    # Exact structural one-defect witnesses, independently specified expectations.
    for path in ['/seed', '/generator', '/allocations', '/state'] + ['/state/' + key for key in STATE[1]]:
        add('missing' + path.replace('/', '.'), canonical(mutation(full, path, delete=True)),
            dict(admitted=False, stage=5, error='noncanonical_bytes', path=path), 'exact_stage_error_path', 'required-field')
    existing_missing = {case['expectation'].get('path') for case in cases if case['id'].startswith('missing')}
    def members(value, path=''):
        if isinstance(value, dict):
            for key, child in value.items():
                childpath = ptr(path, key)
                if childpath not in existing_missing and childpath not in ('/kind', '/format_version', '/rules_version'):
                    add('missing' + childpath.replace('/', '.'), canonical(mutation(full, childpath, delete=True)),
                        dict(admitted=False, stage=5, error='noncanonical_bytes', path=childpath),
                        'exact_stage_error_path', 'required-field/no-null-default')
                members(child, childpath)
        elif isinstance(value, list):
            for i, child in enumerate(value): members(child, ptr(path, i))
    members(full)
    for key in ('kind', 'format_version', 'rules_version'):
        add('missing.header.' + key, canonical(mutation(full, '/' + key, delete=True)),
            dict(admitted=False, stage=4, error='noncanonical_bytes', path='/' + key),
            'exact_stage_error_path', 'required-envelope-header')
    for key, val, err in [('kind', 'OTHER', 'unsupported_kind'), ('format_version', 4, 'unsupported_format_version'),
                           ('rules_version', 4, 'unsupported_rules_version')]:
        add('header.' + key, canonical(mutation(full, '/' + key, val)),
            dict(admitted=False, stage=4, error=err, path='/' + key), 'exact_stage_error_path', 'envelope')
    for path, value, err in [('/state/hulls', [], 'unknown_field'), ('/seed', 'AA' * 32, 'noncanonical_bytes'),
                            ('/state/tick', '00', 'noncanonical_bytes'), ('/generator/generator_version', True, 'invalid_range'),
                            ('/state/worlds/0/inventory/food', -1, 'invalid_range'),
                            ('/state/worlds/0/name', ' Home', 'invalid_range'),
                            ('/state/civilizations/0/status', 'unknown', 'invalid_enum'),
                            ('/state/fleets/0/hulls/0/return_credit', 0, 'invalid_range')]:
        add('schema' + path.replace('/', '.'), canonical(mutation(full, path, value)),
            dict(admitted=False, stage=5, error=err, path=path), 'exact_stage_error_path', 'scalar/field-domain')
    for name in ('systems', 'stars', 'worlds'):
        add('sorting.' + name, canonical(mutation(full, '/state/' + name, list(reversed(full['state'][name])))),
            dict(admitted=False, stage=5, error='noncanonical_bytes', path='/state/' + name + '/1'),
            'exact_stage_error_path', 'strict-decoded-ID-order')
    add('canonical.whitespace', b' ' + canonical(full), dict(admitted=False, stage=6, error='noncanonical_bytes'),
        'exact_stage_error_only', 'canonical-bytes')
    add('encoding.invalid', b'\xff', dict(admitted=False, stage=1, error='noncanonical_bytes', path=''),
        'exact_stage_error_path', 'strict-UTF8')
    add('raw.cap', b'\xff' * (CAP + 1), dict(admitted=False, stage=0, error='byte_capacity', path=''),
        'exact_stage_error_path', '8MiB-before-encoding')
    add('duplicate.member', canonical(full).replace(b'"tick":"0"', b'"tick":"0","tick":"0"', 1),
        dict(admitted=False, stage=3, error='duplicate_key', path='/state/tick'), 'exact_stage_error_path', 'decoded-duplicate')
    # Global installed duplicate second occurrence in source traversal.
    same = copy.deepcopy(full); same['state']['stars'][0]['id'] = same['state']['systems'][0]['id']
    same['state']['stars'].sort(key=lambda row: row['id'])
    idx = next(i for i, row in enumerate(same['state']['stars']) if row['id'] == same['state']['systems'][0]['id'])
    add('identity.cross_kind', canonical(same), dict(admitted=False, stage=7, error='duplicate_identity',
        path='/state/stars/' + str(idx) + '/id'), 'exact_stage_error_path', 'global-installed-pass')
    nested = copy.deepcopy(full); nested['state']['fleets'][0]['hulls'][0]['id'] = nested['state']['fleets'][0]['id']
    add('identity.nested', canonical(nested), dict(admitted=False, stage=7, error='duplicate_identity',
        path='/state/fleets/0/hulls/0/id'), 'exact_stage_error_path', 'nested-hull-global-pass')
    home = next(i for i, w in enumerate(full['state']['worlds']) if w['owner'] is not None)
    homepath = '/state/worlds/' + str(home)
    semantic = [('/state/tick', '1'), ('/state/branch_sequence', '1'), ('/state/next_sequence', '1'),
                ('/state/stars/0/next_activity_tick', '999'), (homepath + '/environment/next_environment_tick', '99'),
                (homepath + '/inventory/food', 10001), (homepath + '/environment/fertility', 10001),
                ('/state/civilizations/0/status', 'dormant'), ('/state/civilizations/0/research/target', 'adaptation'),
                ('/state/colonies/0/hub/energy_next_due', '11'), ('/state/colonies/0/hub/ore_next_due', '99'),
                ('/state/colonies/0/hub/fallback_next_due', '101'),
                ('/state/colonies/0/hub/blocked_reasons', [dict(type='disabled')]), ('/state/colonies/0/owner', full['state']['systems'][0]['id']),
                ('/state/colonies/0/society/next_needs_tick', '99'), ('/state/colonies/0/society/population', 10001),
                ('/state/colonies/0/society/workforce_assignments', [full['state']['fleets'][0]['id']]),
                ('/state/fleets/0/supply/readiness', 'supplied'), ('/state/fleets/0/hulls/0/hit_points', 9),
                ('/state/fleets/0/hulls/0/return_credit', True), ('/state/fleets/0/hulls/0/next_repair', '1'),
                ('/state/fleets/0/hulls/0/passengers/count', 1), ('/state/fleets/0/hulls', []),
                ('/allocations/0/ordinal', '0'), ('/allocations/0/kind', 19),
                ('/allocations', full['allocations'][:-1]), ('/state/observations', []),
                ('/state/observations/0/observed_tick', '1'), ('/state/observations/0/owner', None),
                ('/state/observations/0/remaining_deposit', '19999'), ('/state/observations/0/stationed_hulls/0/count', 2),
                ('/state/observations/0/operational_defense_batteries', 1),
                ('/state/observations/0/environment/stress', 1),
                ('/state/observations/0/environment/base_habitability', 9999),
                ('/state/observations/0/environment/fertility', 9999),
                ('/state/observations/0/environment/maximum_fertility', 9999),
                ('/state/observations/0/environment/stellar_scope/system', full['state']['systems'][1]['id']),
                ('/state/observations/0/hazard_scopes/0/observed_tick', '1'),
                ('/state/observations/0/hazard_scopes/0/originating_observer', full['state']['systems'][0]['id']), ('/state/observations/0/hazard_scopes', []),
                ('/state/observations/0/environment/stellar_scope/observed_tick', '1'),
                ('/state/observations/0/environment/stellar_scope/originating_observer', full['state']['systems'][0]['id']),
                ('/state/observations/0/hazard_scopes/0/lane', full['state']['systems'][0]['id'])]
    for n, (path, value) in enumerate(semantic):
        add('profile.' + str(n), canonical(mutation(full, path, value)), dict(admitted=False), 'rejection_only', path)
    reference_cases = [(homepath + '/system', 'ff' * 16), (homepath + '/system', full['state']['civilizations'][0]['id']),
                       ('/state/deposits/0/world', 'ff' * 16), ('/state/fleets/0/hulls/0/definition', 'unknown'),
                       ('/state/civilizations/0/policy', 'unknown'), ('/allocations/0/id', 'ff' * 16),
                       ('/seed', '01' * 32), ('/catalog_hash', '01' * 32)]
    for n, (path, value) in enumerate(reference_cases):
        add('linkage.' + str(n), canonical(mutation(full, path, value)), dict(admitted=False), 'rejection_only', 'typed/catalog/frame:' + path)
    actual_needs = dict(evaluated_tick='0', population_before=200, food_need=2, energy_need=2,
                        food_paid=2, energy_paid=2, population_after=200, outcome='steady', causes=['fully_paid'])
    for name, path, value in [('needs', '/state/colonies/0/society/last_needs', actual_needs),
                              ('supply_payment', '/state/fleets/0/supply/last_success', dict(tick='0', world=full['state']['colonies'][0]['world'], supplies_paid=1, deadline='100')),
                              ('supply_attempt', '/state/fleets/0/supply/last_attempt', dict(tick='0', world=full['state']['colonies'][0]['world'], required=1, paid=0, outcome='unavailable')),
                              ('order', '/state/fleets/0/order', dict(kind='explore', target_world=None, not_before_tick='0', lane_path=[], objective=None))]:
        add('prior.' + name, canonical(mutation(full, path, value)), dict(admitted=False), 'rejection_only', 'B4-no-prior-attempt')
    remote = copy.deepcopy(full); obs = copy.deepcopy(remote['state']['observations'][0])
    obs['world'] = next(w['id'] for w in remote['state']['worlds'] if w['owner'] is None)
    remote['state']['observations'].append(obs); remote['state']['observations'].sort(key=lambda r: (r['civilization'], r['world']))
    add('visibility.self_authorizing_remote', canonical(remote), dict(admitted=False), 'rejection_only', 'B5-base-only')
    unsupported = []
    for name in ARRAYS:
        if name not in SUPPORTED:
            # Deliberately unexamined richer payload; neither valid nor invalid claimed.
            raw = canonical(mutation(full, '/state/' + name, [{}]))
            unsupported.append(dict(collection=name, raw_base64=base64.b64encode(raw).decode(),
                                    classification='unsupported_family_unexamined', not_a_wire_error=True))
            assert outcome(raw, catalog).get('coverage') == 'unsupported_family'
    return empty, full, cases, unsupported


def visibility_checks(full, catalog):
    state = full['state']; actor = state['civilizations'][0]['id']; home = next(w for w in state['worlds'] if w['owner'])
    # These synthetic operand setups are predicate tests only; no complete Genesis admission.
    target = copy.deepcopy(home); target['id'] = 'aa' * 16; target['owner'] = None
    base = copy.deepcopy(state); base['worlds'].append(target); cases = []
    def record(name, s, w, expected):
        assert visibility(s, catalog, actor, w) is expected
        cases.append(dict(id=name, expected_visible=expected, strength='visibility_leaf_only',
                          state_operands=s, target_world=w, actor=actor))
    s = copy.deepcopy(base); s['colonies'] = []; s['fleets'] = []
    record('owned_non_colonized', s, home, True)
    s = copy.deepcopy(base); s['fleets'] = []; record('colony_system', s, target, True)
    s = copy.deepcopy(base); s['colonies'] = []; s['fleets'][0]['hulls'].append(dict(id='bb'*16, definition='escort'))
    record('mixed_scout', s, target, True)
    s = copy.deepcopy(base); s['colonies'] = []; s['fleets'][0]['hulls'][0]['definition'] = 'escort'
    record('non_scout_same_system_other_world', s, target, False)
    dock_world = dict(home, owner=None)
    s['worlds'] = [dock_world if w['id'] == home['id'] else w for w in s['worlds']]
    record('non_scout_docked_world', s, dock_world, True)
    s = copy.deepcopy(base); s['colonies'] = []; s['worlds'] = [dict(w, owner=None) for w in s['worlds']]
    s['fleets'][0]['location'] = dict(type='travelling', origin_world=home['id'], destination_world=target['id'], departure_tick='0', arrival_tick='1')
    record('travelling_scout', s, target, False)
    s = copy.deepcopy(base); s['colonies'] = []; s['fleets'] = []; s['observations'].append(dict(civilization=actor, world=target['id']))
    record('submitted_observation_not_source', s, target, False)
    return cases


def products(catalog, sources):
    empty, full, cases, unsupported = build(catalog, sources)
    profiles = {}
    for name, doc in [('empty', empty), ('inhabited', full)]:
        validate(canonical(doc), catalog)
        profiles[name] = sealed(doc)
    context_cases = []
    for name, original in profiles.items():
        for field, value in [('root_branch/id', 'ff' * 16), ('root_branch/fork_state_digest', 'ff' * 32),
                             ('root_branch/fork_tick', '1'), ('root_branch/head', 'ff' * 32),
                             ('history_view/branch', 'ff' * 16), ('history_view/revision', 'ff' * 32),
                             ('history_view/tick', '1'), ('allocator_marks/accepted_sequence_high_water', '1'),
                             ('allocator_marks/branch_sequence_high_water', '1')]:
            altered = mutation(original, '/' + field, value)
            doc = empty if name == 'empty' else full
            assert context_matches(doc, original) and not context_matches(doc, altered)
            context_cases.append(dict(id=name + '.' + field.replace('/', '.'), context=altered,
                                      expectation=dict(binding_matches=False),
                                      strength='derived_root_context_rejection_only',
                                      predicate='Exact independently derived Branch/View/zero initial marks; no Archive admission'))
    ledger = dict(status='Proposed selected-profile admission; general matrix Partial', source_pins=PINS,
                  accepted_catalog_files=CAT_PINS, source_files=sources, catalog_hash=PACK,
                  entity_kind_registry=[dict(kind=i+1, name=name, issued_in_inhabited=i+1 in set(ISSUED.values()) | {11})
                                        for i, name in enumerate(KINDS)],
                  state_fields=list(STATE[1]), field_declarations={},
                  complete_profiles=['empty', 'GEN-C-MIN11-HUB-DEPOSIT-DOCKED-SCOUT'],
                  positive_predicates=[dict(stage=stage, predicate=predicate, independently_checked=True) for stage, predicate in
                      [('B0', 'exact approved Catalog/header/scalars/fields/canonical'), ('B1', 'eleven frames/live bijection/global nested uniqueness/typed references/sorted keys'),
                       ('B2', 'zero counters/exact activity/environment/hub/needs clocks/local stocks/environment'),
                       ('B3', 'active iff owns colony/research empty/workforce recomputed empty'),
                       ('B4', 'fullHP/noRepair/noPassengers/noCredits/noNeeds/noSupply/noOrder/noBlockade'),
                       ('B5', 'base-only authorized visibility exact complete observations'),
                       ('B6', 'exact deposits/hulls/environment/empty complete incident scopes'),
                       ('B7', 'no repair/default/stateWrapper/genesis/root/view/highwater/context')]],
                  unsupported_family_payloads=unsupported,
                  gaps=['Richer bootstrap collections unexamined; unsupported is fixture coverage, never wire error.',
                        'Full stage7/8/9 multi-defect semantic matrix not frozen; semantic/linkage/profile negative expectations are rejection-only.',
                        'General initial environmental sources/jobs/paid provenance/war/treaty/arbitration not admitted.',
                        'Visibility synthetic leaf operands are not complete admitted alternate Genesis documents.',
                        'No standalone VIEW hash domain is declared; HistoryView canonical bytes/root binding only.',
                        'No initial ordinary Receipt/Capture/paid Shipment/Anchor. Full Archive/HIST/Actor/forecast/transfer/runtime separate.',
                        'Only eleven Genesis issuance frames across eight kinds exercised, all18 registry names recorded; other origins/families remain Partial.',
                        'Nonempty blocked/facility/threat/event nested rosters are not an exhaustive structural/ordering matrix; only selected one-defect blocked witness is qualified.',
                        'Fixture reader recursion after depth64 syntax bound is not a production bounded-job/poll/ownership proof.'])
    def declarations(schema, path):
        if isinstance(schema, tuple):
            if schema[0] == 'record':
                ledger['field_declarations'][path] = list(schema[1])
                for key, child in schema[1].items(): declarations(child, ptr(path, key))
            elif schema[0] == 'array':
                ledger['field_declarations'][path + ':array'] = dict(cap=schema[2], key=schema[3], issued=schema[4])
                declarations(schema[1], path + '/*')
            elif schema[0] == 'option': declarations(schema[1], path)
            elif schema[0] == 'tag':
                for tag, child in schema[1].items(): declarations(child, path + ':' + tag)
    declarations(GENESIS, '')
    positive_seals = {case['id']: sealed(json.loads(base64.b64decode(case['raw_base64'])))
                      for case in cases if case['expectation'].get('admitted') is True}
    ledger['raw_case_strength_counts'] = {strength: sum(c['strength'] == strength for c in cases)
                                         for strength in sorted({c['strength'] for c in cases})}
    ledger['scalar_predicates'] = []
    def scalar_paths(schema, value, path=''):
        if isinstance(schema, str):
            ledger['scalar_predicates'].append(dict(path=path, structural_domain=schema,
                exact_literal_is_fixture_choice=schema == 'Name', submitted_value=value,
                checked_on_complete_positive=True))
        elif schema[0] == 'option':
            if value is not None: scalar_paths(schema[1], value, path)
        elif schema[0] == 'record':
            for key, child in schema[1].items(): scalar_paths(child, value[key], ptr(path, key))
        elif schema[0] == 'array':
            for i, child in enumerate(value): scalar_paths(schema[1], child, ptr(path, i))
        elif schema[0] == 'tag': scalar_paths(schema[1][value['type']], value, path)
        elif schema[0] == 'enum':
            ledger['scalar_predicates'].append(dict(path=path, structural_domain='closed_enum',
                allowed_values=list(schema[1]), submitted_value=value, checked_on_complete_positive=True))
    scalar_paths(GENESIS, full)
    vectors = dict(status='Proposed GEN-C', catalog_hash=PACK, profiles=profiles, complete_positive_seals=positive_seals, cases=cases,
                   visibility_leaf_cases=visibility_checks(full, catalog), root_context_cases=context_cases)
    encode = lambda x: json.dumps(x, ensure_ascii=False, indent=2).encode('utf8') + b'\n'
    return {'empty.genesis.json': canonical(empty), 'inhabited.genesis.json': canonical(full),
            'vectors.json': encode(vectors), 'predicate-coverage.json': encode(ledger)}


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('mode', choices=('generate', 'verify'))
    args = parser.parse_args(); catalog, sources = freshness(); output = products(catalog, sources)
    freshness()  # Hard stop before writing or claiming exact verification.
    for name, raw in output.items():
        path = HERE / name
        if args.mode == 'generate': path.write_bytes(raw)
        elif not path.exists() or path.read_bytes() != raw: raise SystemExit('STOP: output differs ' + name)
    print(args.mode + ' passed: two sealed base profiles and valid input choices; ' + str(len(json.loads(output['vectors.json'])['cases'])) + ' raw cases')
    for name, raw in output.items(): print(name, len(raw), sha(raw))


if __name__ == '__main__':
    main()
