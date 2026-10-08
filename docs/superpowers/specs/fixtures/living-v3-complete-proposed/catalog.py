#!/usr/bin/env python3
"""COMPLETE-FOUNDATION-CATALOG: independent, stdlib-only Catalog construction and admission oracle.

This is a Proposed fixture oracle, not a production codec. No old corpus modules,
Rust serialization, assets or runtime output are imported. See README for scope.
"""

import argparse
import base64
import copy
import hashlib
import json
import re
from dataclasses import dataclass, field
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
FREEZE = SDD / 'v3-ia2-completion-lane-freeze-2'
FREEZE_HASH = '55ca0ede26a284142f6d1208fa57ebe83ecd6bcce59a99751751332d42d6d1eb'
DOMAIN = b'NYON_LIVING_V3_PACK\0'
CAP = 1024 * 1024
U32 = 2**32 - 1
KIND = 'NYON_LIVING_PACK_V3'
COLLECTIONS = ('star_archetypes', 'world_archetypes', 'resources',
               'industry_definitions', 'hull_definitions', 'hazard_definitions',
               'policy_definitions', 'technology_definitions')
INTENTS = ('build_ark', 'build_battery', 'build_escort', 'build_extractor',
           'build_foundry', 'build_scout', 'build_shipyard', 'build_solar_array',
           'defend', 'establish_trade_route', 'settle', 'supply', 'build_farm',
           'build_habitation', 'build_research_lab', 'select_research',
           'request_supply', 'negotiate_access', 'negotiate_supply',
           'negotiate_defense', 'blockade', 'adaptation')


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def source_check():
    from provenance import check_sources
    return check_sources()


def amount(resource, quantity):
    return {'resource': resource, 'quantity': quantity}


def recipe(period, deposit, inputs, outputs):
    return {'type': 'recipe', 'period_ticks': period, 'deposit_units': deposit,
            'inputs': [amount(r, q) for r, q in inputs],
            'outputs': [amount(r, q) for r, q in outputs]}


def core_catalog():
    """Literal source tables, freshly assembled in declared V3 field/key order.

    Inherited values: schema Frozen inherited initial catalog values.
    Additions: schema Catalog registry + societies Added definitions/Research.
    Workforce/stress/roles: schema required overrides, not inferred from names.
    """
    industries = []
    rows = [
        ('defense_battery', 'Defense Battery', 40, 200, 1,
         {'type': 'defense', 'hit_points': 40, 'damage_per_round': 4}, 0, 0),
        ('extractor', 'Extractor', 30, 150, 6,
         recipe(10, 4, [('energy', 2)], [('ore', 4)]), 1, 1),
        ('farm', 'Farm', 30, 150, 8,
         recipe(10, 0, [('energy', 2)], [('food', 20)]), 1, 0),
        ('foundry', 'Foundry', 40, 200, 6,
         recipe(20, 0, [('energy', 4), ('ore', 4)], [('alloy', 2)]), 1, 2),
        ('habitation', 'Habitation', 30, 150, 8,
         {'type': 'habitation', 'base_capacity': 1000}, 0, 0),
        ('research_lab', 'Research Lab', 60, 300, 8,
         {'type': 'research', 'period_ticks': 20,
          'inputs': [amount('alloy', 1), amount('energy', 2)], 'points': 1}, 1, 0),
        ('shipyard', 'Shipyard', 60, 300, 1,
         {'type': 'hull_assembly', 'concurrent_jobs': 1}, 1, 0),
        ('solar_array', 'Solar Array', 20, 100, 6,
         recipe(10, 0, [], [('energy', 8)]), 1, 0),
    ]
    for slug, name, cost, ticks, maximum, effect, workers, stress in rows:
        industries.append({'id': slug, 'name': name, 'alloy_cost': cost,
                           'build_ticks': ticks, 'slot_cost': 1,
                           'max_per_colony': maximum, 'effect': effect,
                           'workforce_units': workers, 'stress_per_operation': stress})
    hulls = [
        {'id': slug, 'name': name, 'alloy_cost': cost, 'build_ticks': ticks,
         'hit_points': hp, 'damage': damage, 'role': role}
        for slug, name, cost, ticks, hp, damage, role in [
            ('colony_ark', 'Colony Ark', 60, 200, 20, 0, 'ark'),
            ('escort', 'Escort', 20, 100, 10, 2, 'escort'),
            ('scout', 'Scout', 10, 50, 10, 0, 'scout')]]
    policies = [
        {'id': slug, 'name': name,
         'bonuses': [{'intent': intent, 'bonus': 20} for intent in intents]}
        for slug, name, intents in [
            ('expansionist', 'Expansionist', ['build_ark', 'settle']),
            ('guardian', 'Guardian', ['build_battery', 'build_escort', 'defend']),
            ('industrialist', 'Industrialist', ['build_extractor', 'build_foundry']),
            ('neutral', 'Neutral', []),
            ('trader', 'Trader', ['establish_trade_route', 'supply'])]]
    tech = [
        {'id': 'agronomy', 'name': 'Agronomy', 'prerequisites': [],
         'point_cost': 100, 'effect': {'type': 'farm_base_output', 'units': 25}},
        {'id': 'efficient_logistics', 'name': 'Efficient Logistics', 'prerequisites': [],
         'point_cost': 100,
         'effect': {'type': 'departure_energy_scale', 'numerator': 3, 'denominator': 4}},
        {'id': 'resilient_habitation', 'name': 'Resilient Habitation',
         'prerequisites': ['agronomy'], 'point_cost': 150,
         'effect': {'type': 'carrying_capacity_penalty_scale', 'numerator': 1, 'denominator': 2}},
    ]
    return {'kind': KIND, 'format_version': 3, 'rules_version': 3,
            'pack_id': 'nyon_living_core', 'pack_version': 1,
            'star_archetypes': [{'id': 'main_sequence', 'name': 'Main Sequence'},
                                {'id': 'red_dwarf', 'name': 'Red Dwarf'}],
            'world_archetypes': [{'id': 'barren', 'name': 'Barren'},
                                 {'id': 'temperate', 'name': 'Temperate'}],
            'resources': [{'id': r, 'name': r.title(), 'storage_limit': 10000}
                          for r in ['alloy', 'energy', 'food', 'ore']],
            'industry_definitions': industries, 'hull_definitions': hulls,
            'hazard_definitions': [{'id': 'ion_storm', 'name': 'Ion Storm',
                                    'effect': {'type': 'freight_batch_scale',
                                               'numerator': 1, 'denominator': 2}}],
            'policy_definitions': policies, 'technology_definitions': tech}


def quoted(value):
    # Exact schema escaping: no ASCII normalization or short control escapes.
    return '"' + ''.join('\\"' if c == '"' else '\\\\' if c == '\\'
                        else '\\u%04x' % ord(c) if ord(c) < 32 else c
                        for c in value) + '"'


def canonical(value):
    if isinstance(value, dict):
        return '{' + ','.join(quoted(k) + ':' + canonical(v) for k, v in value.items()) + '}'
    if isinstance(value, list):
        return '[' + ','.join(canonical(v) for v in value) + ']'
    if isinstance(value, str):
        return quoted(value)
    if value is None:
        return 'null'
    if type(value) is bool:
        return 'true' if value else 'false'
    if type(value) is int:
        return str(value)
    raise ValueError('No float or unspecified canonical value')


def encoded(value):
    return canonical(value).encode('utf-8')


def pointer(parent, token):
    return parent + '/' + str(token).replace('~', '~0').replace('/', '~1')


class Reject(Exception):
    def __init__(self, stage, error, path='', offset=None, **details):
        self.result = {'stage': stage, 'error': error, 'path': path}
        if offset is not None:
            self.result['offset'] = offset
        self.result.update(details)


@dataclass
class Node:
    kind: str
    value: object
    path: str
    start: int
    end: int = 0


class Parser:
    """Explicit stack; JSON containers never recurse, maximum depth checked first.

    Pairs retained until the stage3 pass. json.loads is used solely on already
    isolated string tokens, not documents/containers or duplicate dictionaries.
    """
    def __init__(self, text):
        self.text = text
        self.i = 0
        self.byte_offsets = [0]
        for c in text:
            self.byte_offsets.append(self.byte_offsets[-1] + len(c.encode('utf-8')))
        self.duplicates = []
        self.stack = []

    def reject(self, error='noncanonical_bytes', at=None):
        raise Reject(2, error, offset=self.byte_offsets[self.i if at is None else at])

    def whitespace(self):
        while self.i < len(self.text) and self.text[self.i] in ' \n\r\t':
            self.i += 1

    def string(self):
        start = self.i
        self.i += 1
        while self.i < len(self.text):
            c = self.text[self.i]
            if c == '"':
                self.i += 1
                token = self.text[start:self.i]
                try:
                    result = json.loads(token)
                except json.JSONDecodeError as err:
                    self.reject(at=start + err.pos)
                if any(0xD800 <= ord(x) <= 0xDFFF for x in result):
                    self.reject(at=start)
                return result, start
            if ord(c) < 32:
                self.reject()
            if c == '\\':
                self.i += 1
                if self.i >= len(self.text):
                    self.reject()
                if self.text[self.i] == 'u':
                    digits = self.text[self.i + 1:self.i + 5]
                    if len(digits) != 4 or not re.fullmatch('[a-fA-F0-9]{4}', digits):
                        self.reject()
                    self.i += 4
                elif self.text[self.i] not in '"\\/bfnrt':
                    self.reject()
            self.i += 1
        self.reject()

    def value(self, path):
        self.whitespace()
        start = self.i
        if self.i >= len(self.text):
            self.reject()
        c = self.text[self.i]
        if c in '[{':
            if len(self.stack) + 1 > 64:
                self.reject('depth_capacity')
            self.i += 1
            node = Node('object' if c == '{' else 'array', [], path, start)
            self.stack.append({'node': node, 'state': 'first', 'seen': set()})
            return node
        if c == '"':
            value, _ = self.string()
            return Node('string', value, path, start, self.i)
        for literal, kind, value in [('true', 'bool', True), ('false', 'bool', False),
                                     ('null', 'null', None)]:
            if self.text.startswith(literal, self.i):
                self.i += len(literal)
                return Node(kind, value, path, start, self.i)
        match = re.match(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?',
                         self.text[self.i:])
        if match:
            self.i += len(match.group())
            return Node('number', match.group(), path, start, self.i)
        self.reject()

    def parse(self):
        root = self.value('')
        while self.stack:
            frame = self.stack[-1]
            node, state = frame['node'], frame['state']
            self.whitespace()
            if self.i >= len(self.text):
                self.reject()
            c = self.text[self.i]
            closing = '}' if node.kind == 'object' else ']'
            if c == closing and state in ('first', 'comma'):
                self.i += 1
                node.end = self.i
                self.stack.pop()
                continue
            if state == 'comma':
                if c != ',':
                    self.reject()
                self.i += 1
                frame['state'] = 'next'
                continue
            if node.kind == 'object':
                if c != '"':
                    self.reject()
                key, key_start = self.string()
                self.whitespace()
                if self.i >= len(self.text) or self.text[self.i] != ':':
                    self.reject()
                self.i += 1
                child = self.value(pointer(node.path, key))
                node.value.append((key, key_start, child))
                if key in frame['seen']:
                    self.duplicates.append((key_start, child.path))
                frame['seen'].add(key)
            else:
                node.value.append(self.value(pointer(node.path, len(node.value))))
            frame['state'] = 'comma'
        self.whitespace()
        if self.i != len(self.text):
            self.reject()
        if self.duplicates:
            offset, path = min(self.duplicates)
            raise Reject(3, 'duplicate_key', path, self.byte_offsets[offset])
        return root


@dataclass(frozen=True)
class Spec:
    kind: str
    fields: tuple = ()
    child: object = None
    cap: object = None
    key: object = None
    variants: dict = field(default_factory=dict)
    choices: tuple = ()
    minimum: int = 0


def record(**fields):
    return Spec('object', fields=tuple(fields.items()))


def sorted_array(child, cap=None, key=None):
    return Spec('array', child=child, cap=cap, key=key)


NUMBER = Spec('u32')
NAME = Spec('name')
SLUG = Spec('slug')
STRING = Spec('string')
AMOUNT = record(resource=SLUG, quantity=NUMBER)
AMOUNTS = sorted_array(AMOUNT, 4, 'resource')
INDUSTRY_EFFECT = Spec('tagged', variants={
    'recipe': record(type=STRING, period_ticks=NUMBER, deposit_units=NUMBER,
                     inputs=AMOUNTS, outputs=AMOUNTS),
    'hull_assembly': record(type=STRING, concurrent_jobs=NUMBER),
    'defense': record(type=STRING, hit_points=NUMBER, damage_per_round=NUMBER),
    'habitation': record(type=STRING, base_capacity=NUMBER),
    'research': record(type=STRING, period_ticks=NUMBER, inputs=AMOUNTS, points=NUMBER)})
TECH_EFFECT = Spec('tagged', variants={
    'farm_base_output': record(type=STRING, units=NUMBER),
    'carrying_capacity_penalty_scale': record(type=STRING, numerator=NUMBER, denominator=NUMBER),
    'departure_energy_scale': record(type=STRING, numerator=NUMBER, denominator=NUMBER)})
HAZARD_EFFECT = Spec('tagged', variants={
    'freight_batch_scale': record(type=STRING, numerator=NUMBER, denominator=NUMBER)})
ARCHETYPE = record(id=SLUG, name=NAME)
RESOURCE = record(id=SLUG, name=NAME, storage_limit=NUMBER)
INDUSTRY = record(id=SLUG, name=NAME, alloy_cost=NUMBER, build_ticks=NUMBER,
                  slot_cost=NUMBER, max_per_colony=NUMBER, effect=INDUSTRY_EFFECT,
                  workforce_units=NUMBER, stress_per_operation=NUMBER)
HULL = record(id=SLUG, name=NAME, alloy_cost=NUMBER, build_ticks=NUMBER,
              hit_points=NUMBER, damage=NUMBER,
              role=Spec('unit', choices=('ark', 'escort', 'scout')))
HAZARD = record(id=SLUG, name=NAME, effect=HAZARD_EFFECT)
BONUS = record(intent=Spec('unit', choices=INTENTS), bonus=NUMBER)
POLICY = record(id=SLUG, name=NAME, bonuses=sorted_array(BONUS, key='intent'))
TECH = record(id=SLUG, name=NAME, prerequisites=sorted_array(SLUG, 64),
              point_cost=NUMBER, effect=TECH_EFFECT)
CATALOG = record(kind=STRING, format_version=NUMBER, rules_version=NUMBER,
                 pack_id=SLUG, pack_version=Spec('u32', minimum=1),
                 star_archetypes=sorted_array(ARCHETYPE, 64, 'id'),
                 world_archetypes=sorted_array(ARCHETYPE, 64, 'id'),
                 resources=sorted_array(RESOURCE, 4, 'id'),
                 industry_definitions=sorted_array(INDUSTRY, 64, 'id'),
                 hull_definitions=sorted_array(HULL, 32, 'id'),
                 hazard_definitions=sorted_array(HAZARD, 32, 'id'),
                 policy_definitions=sorted_array(POLICY, 32, 'id'),
                 technology_definitions=sorted_array(TECH, 64, 'id'))


def integer(node, stage=5, minimum=0):
    if node.kind != 'number' or not re.fullmatch(r'-?(0|[1-9][0-9]*)', node.value):
        raise Reject(stage, 'invalid_range', node.path)
    if node.value == '-0':
        raise Reject(stage, 'noncanonical_bytes', node.path)
    if len(node.value) > 10 or node.value.startswith('-'):
        raise Reject(stage, 'invalid_range', node.path)
    value = int(node.value)
    if not minimum <= value <= U32:
        raise Reject(stage, 'invalid_range', node.path)
    return value


def object_members(node, fields, stage=5):
    if node.kind != 'object':
        raise Reject(stage, 'invalid_range', node.path)
    found = {k: n for k, _, n in node.value}
    unknown = sorted(set(found) - set(fields), key=lambda s: s.encode('utf-8'))
    if unknown:
        raise Reject(stage, 'unknown_field', pointer(node.path, unknown[0]))
    return found


def structure(node, spec):
    kind = spec.kind
    if kind == 'u32':
        return integer(node, minimum=spec.minimum)
    if kind in ('string', 'slug', 'name', 'unit'):
        if node.kind != 'string':
            raise Reject(5, 'invalid_range', node.path)
        value = node.value
        if kind == 'slug' and not re.fullmatch('[a-z][a-z0-9_]{0,63}', value):
            raise Reject(5, 'invalid_range', node.path)
        if kind == 'name' and (not 1 <= len(value) <= 64 or len(value.encode('utf-8')) > 256
                               or any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value)
                               or value.startswith(' ') or value.endswith(' ') or '  ' in value):
            raise Reject(5, 'invalid_range', node.path)
        if kind == 'unit' and value not in spec.choices:
            raise Reject(5, 'invalid_enum', node.path)
        return value
    if kind == 'tagged':
        if node.kind != 'object':
            raise Reject(5, 'invalid_range', node.path)
        found = {k: n for k, _, n in node.value}
        tag = found.get('type')
        chosen = spec.variants.get(tag.value) if tag is not None and tag.kind == 'string' else None
        allowed = [key for key, _ in chosen.fields] if chosen else {
            key for variant in spec.variants.values() for key, _ in variant.fields}
        object_members(node, allowed)
        if tag is None:
            raise Reject(5, 'noncanonical_bytes', pointer(node.path, 'type'))
        if tag.kind != 'string':
            raise Reject(5, 'invalid_range', tag.path)
        if tag.value not in spec.variants:
            raise Reject(5, 'invalid_enum', tag.path)
        return structure(node, spec.variants[tag.value])
    if kind == 'object':
        found = object_members(node, [k for k, _ in spec.fields])
        result = {}
        for key, child in spec.fields:
            if key not in found:
                raise Reject(5, 'noncanonical_bytes', pointer(node.path, key))
            result[key] = structure(found[key], child)
        return result
    if node.kind != 'array':
        raise Reject(5, 'invalid_range', node.path)
    if spec.cap is not None and len(node.value) > spec.cap:
        raise Reject(5, 'record_capacity', node.path, actual=len(node.value), limit=spec.cap)
    result = [structure(n, spec.child) for n in node.value]
    seen = set()
    previous = None
    for i, row in enumerate(result):
        key = row[spec.key] if spec.key else row
        path = pointer(pointer(node.path, i), spec.key) if spec.key else pointer(node.path, i)
        if key in seen:
            raise Reject(5, 'duplicate_key', path)
        if previous is not None and previous > key:
            raise Reject(5, 'noncanonical_bytes', path)
        seen.add(key)
        previous = key
    return result


def finite_mechanics(value, expected, path=''):
    """Declaration-ordered semantic equality, with explicit fraction prerequisite."""
    if isinstance(expected, dict):
        if 'type' in expected and value['type'] != expected['type']:
            raise Reject(7, 'invalid_range', pointer(path, 'type'))
        if 'numerator' in expected:
            if value['denominator'] == 0:
                raise Reject(7, 'invalid_range', pointer(path, 'denominator'))
            if value['numerator'] > value['denominator']:
                raise Reject(7, 'invalid_range', pointer(path, 'numerator'))
        for key, fixed in expected.items():
            if key == 'name' or (path == '' and key in ('pack_id', 'pack_version')):
                continue
            finite_mechanics(value[key], fixed, pointer(path, key))
    elif isinstance(expected, list):
        if len(value) != len(expected):
            raise Reject(7, 'invalid_range', path)
        if expected and isinstance(expected[0], dict) and 'id' in expected[0]:
            if [x['id'] for x in value] != [x['id'] for x in expected]:
                raise Reject(7, 'invalid_range', path)
        for i, fixed in enumerate(expected):
            finite_mechanics(value[i], fixed, pointer(path, i))
    elif value != expected:
        raise Reject(7, 'invalid_range', path)


def references(catalog):
    resources = {row['id'] for row in catalog['resources']}
    for i, row in enumerate(catalog['industry_definitions']):
        for key in ('inputs', 'outputs'):
            for j, item in enumerate(row['effect'].get(key, [])):
                if item['resource'] not in resources:
                    raise Reject(8, 'missing_reference', f'/industry_definitions/{i}/effect/{key}/{j}/resource')
    technologies = {row['id']: row for row in catalog['technology_definitions']}
    for i, row in enumerate(catalog['technology_definitions']):
        for j, ref in enumerate(row['prerequisites']):
            if ref not in technologies:
                raise Reject(8, 'missing_reference', f'/technology_definitions/{i}/prerequisites/{j}')
    # Iterative colored DFS over the bounded literal finite prerequisite graph.
    done = set()
    for name in technologies:
        active = set()
        stack = [(name, False)]
        while stack:
            node, finish = stack.pop()
            if finish:
                active.remove(node)
                done.add(node)
            elif node not in done:
                if node in active:
                    raise Reject(8, 'invalid_graph', '/technology_definitions')
                active.add(node)
                stack.append((node, True))
                stack.extend((ref, False) for ref in reversed(technologies[node]['prerequisites']))


def admit(raw, expected_hash=None):
    try:
        if len(raw) > CAP:
            raise Reject(0, 'byte_capacity', actual=len(raw), limit=CAP)
        try:
            text = raw.decode('utf-8', errors='strict')
        except UnicodeDecodeError as err:
            raise Reject(1, 'noncanonical_bytes', offset=err.start) from err
        parser = Parser(text)
        node = parser.parse()
        if node.kind != 'object':
            raise Reject(4, 'invalid_range')
        headers = {k: n for k, _, n in node.value}
        for key, constant, error in [('kind', KIND, 'unsupported_kind'),
                                     ('format_version', 3, 'unsupported_format_version'),
                                     ('rules_version', 3, 'unsupported_rules_version')]:
            if key not in headers:
                raise Reject(4, 'noncanonical_bytes', '/' + key)
            child = headers[key]
            if key == 'kind':
                if child.kind != 'string':
                    raise Reject(4, 'invalid_range', child.path)
                value = child.value
            else:
                value = integer(child, 4)
            if value != constant:
                raise Reject(4, error, child.path)
        result = structure(node, CATALOG)
        canon = encoded(result)
        if raw != canon:
            first = next((i for i, (a, b) in enumerate(zip(raw, canon)) if a != b), min(len(raw), len(canon)))
            # Containing value path for string escape variants; parent/root for
            # layout differences. These are helper diagnostics, not extra wire.
            path = ''
            pending = [node]
            while pending:
                current = pending.pop()
                if current.kind in ('string', 'number'):
                    if parser.byte_offsets[current.start] <= first < parser.byte_offsets[current.end]:
                        path = current.path
                        break
                elif current.kind == 'object':
                    pending.extend(n for _, _, n in reversed(current.value))
                elif current.kind == 'array':
                    pending.extend(reversed(current.value))
            raise Reject(6, 'noncanonical_bytes', path, first)
        finite_mechanics(result, core_catalog())
        references(result)
        digest = sha(DOMAIN + canon)
        if expected_hash is not None and digest != expected_hash:
            raise Reject(8, 'catalog_mismatch', '/catalog_hash')
        return {'admitted': True, 'pack_sha256': digest}
    except Reject as err:
        return err.result


def leaves(value, path=''):
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, pointer(path, key))
    elif isinstance(value, list):
        for i, child in enumerate(value):
            yield from leaves(child, pointer(path, i))
    else:
        yield path, value


def containers(value, path=''):
    if isinstance(value, dict):
        yield {'path': path, 'kind': 'record', 'required_fields_in_order': list(value)}
        for key, child in value.items():
            yield from containers(child, pointer(path, key))
    elif isinstance(value, list):
        yield {'path': path, 'kind': 'array', 'full_positive_length': len(value),
               'empty_is_explicit': True}
        for i, child in enumerate(value):
            yield from containers(child, pointer(path, i))


def at(value, path):
    for token in path.split('/')[1:]:
        token = token.replace('~1', '/').replace('~0', '~')
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def schema_at(path):
    spec, value = CATALOG, core_catalog()
    for token in path.split('/')[1:]:
        token = token.replace('~1', '/').replace('~0', '~')
        if spec.kind == 'tagged':
            if token == 'type':
                return Spec('unit', choices=tuple(spec.variants))
            spec = spec.variants[value['type']]
        if spec.kind == 'array':
            spec, value = spec.child, value[int(token)]
        else:
            spec, value = dict(spec.fields)[token], value[token]
    return spec


def structural_predicate(spec):
    if spec.kind == 'u32':
        return {'type': 'JSON integer (not bool/float/string)', 'minimum': spec.minimum,
                'maximum': U32, 'canonical_unsigned_spelling': True}
    if spec.kind == 'name':
        return {'type': 'Name', 'unicode_scalars': [1, 64], 'utf8_bytes_maximum': 256,
                'forbidden_controls': ['U+0000..001F', 'U+007F..009F'],
                'forbidden_spaces': ['leading U+0020', 'trailing U+0020', 'doubled U+0020'],
                'normalization': 'none'}
    if spec.kind == 'slug':
        return {'type': 'Slug', 'pattern': '[a-z][a-z0-9_]{0,63}'}
    if spec.kind == 'unit':
        return {'type': 'closed tag string', 'allowed_values': list(spec.choices)}
    return {'type': 'String', 'bound': 'enclosing raw Catalog byte cap',
            'constant_owner': 'header stage4 or tagged parent'}


def changed(value, path, replacement):
    value = copy.deepcopy(value)
    parent, key = path.rsplit('/', 1)
    owner = at(value, parent)
    owner[int(key) if isinstance(owner, list) else key] = replacement
    return value


def document_cases():
    core = core_catalog()
    raw_core = encoded(core)
    cases = []

    def positive(name, value):
        raw = encoded(value)
        expected = {'admitted': True, 'pack_sha256': sha(DOMAIN + raw)}
        cases.append({'id': name, 'strength': 'complete_catalog_admission',
                      'raw_base64': base64.b64encode(raw).decode(), 'expected': expected})

    def negative(name, raw, stage, error, path='', strength='exact_stage_error_path', **details):
        expectation = {'stage': stage, 'error': error, **details}
        if path is not None:
            expectation['path'] = path
        cases.append({'id': name, 'strength': strength,
                      'raw_base64': base64.b64encode(raw).decode(),
                      'expected': expectation})

    positive('core', core)
    for path, value in leaves(core):
        name = path.strip('/').replace('/', '.')
        if path.endswith('/name'):
            positive('metadata.' + name, changed(core, path, 'Δ "\\" 🪐'))
            negative('name.control.' + name, encoded(changed(core, path, 'Bad\x7f')), 5, 'invalid_range', path)
        elif path == '/pack_id':
            positive('metadata.pack_id.maximum', changed(core, path, 'z' + '9_' * 31 + '0'))
        elif path == '/pack_version':
            positive('metadata.pack_version.maximum', changed(core, path, U32))
            negative('version.zero', encoded(changed(core, path, 0)), 5, 'invalid_range', path)
        elif path not in ('/kind', '/format_version', '/rules_version') and not path.endswith('/id'):
            if type(value) is int:
                replacement = value + 1
            elif path.endswith('/type'):
                continue  # Explicit type-variant cases below need their complete payload.
            elif path.endswith('/resource'):
                replacement = 'zz_absent_resource' if '/1/resource' in path else 'absent_resource'
            elif '/prerequisites/' in path:
                replacement = 'absent_technology'
            elif path.endswith('/role'):
                replacement = 'scout' if value != 'scout' else 'escort'
            elif path.endswith('/intent'):
                continue  # Exact alternate intent case below retains sorting.
            else:
                continue
            negative('mechanic.' + name, encoded(changed(core, path, replacement)), 7, 'invalid_range', path)
        if type(value) is int:
            stage = 4 if path in ('/format_version', '/rules_version') else 5
            negative('width.' + name, encoded(changed(core, path, U32 + 1)), stage, 'invalid_range', path)
            negative('type.' + name, encoded(changed(core, path, True)), stage, 'invalid_range', path)

    name_path = '/star_archetypes/0/name'
    for label, name in [('one', 'X'), ('64_scalar_256_byte', '🪐' * 64),
                        ('replacement_literal', '\ufffd'), ('decomposed', 'e\u0301'),
                        ('composed', 'é'), ('quote_backslash', 'A"\\Z'),
                        ('non_ascii_space', '\u00a0A\u00a0'), ('unicode_separator', 'A\u2028B')]:
        positive('name.valid.' + label, changed(core, name_path, name))
    for label, name in [('empty', ''), ('65_scalars', 'x' * 65), ('257_plus_bytes', '🪐' * 65),
                        ('leading_space', ' A'), ('trailing_space', 'A '), ('double_space', 'A  B'),
                        ('nul', 'A\0B'), ('c1', 'A\u009fB')]:
        negative('name.invalid.' + label, encoded(changed(core, name_path, name)), 5, 'invalid_range', name_path)
    renamed = copy.deepcopy(core)
    for collection in COLLECTIONS:
        for row in renamed[collection]:
            row['name'] = '界' + row['id']
    renamed['pack_id'], renamed['pack_version'] = 'all_metadata', U32
    positive('metadata.all_allowed_paths', renamed)
    positive('metadata.pack_id.minimum', changed(core, '/pack_id', 'a'))
    for slug in ['', 'A', 'a-b', 'a' * 65, 'a space']:
        negative('slug.invalid.' + repr(slug), encoded(changed(core, '/pack_id', slug)), 5, 'invalid_range', '/pack_id')
    for header, unsupported, error in [('kind', 'UNKNOWN', 'unsupported_kind'),
                                        ('format_version', 4, 'unsupported_format_version'),
                                        ('rules_version', 4, 'unsupported_rules_version')]:
        negative('header.unsupported.' + header, encoded(changed(core, '/' + header, unsupported)), 4, error, '/' + header)
        missing = copy.deepcopy(core)
        del missing[header]
        negative('header.missing.' + header, encoded(missing), 4, 'noncanonical_bytes', '/' + header)
        negative('header.type.' + header, encoded(changed(core, '/' + header, None)), 4, 'invalid_range', '/' + header)

    negative('raw.cap_plus_one_before_utf8', b'\xff' + b' ' * CAP, 0, 'byte_capacity', actual=CAP + 1, limit=CAP)
    negative('raw.cap_exact_whitespace', raw_core + b' ' * (CAP - len(raw_core)), 6, 'noncanonical_bytes')
    negative('raw.invalid_utf8', b'\xff', 1, 'noncanonical_bytes')
    negative('syntax.bom', b'\xef\xbb\xbf' + raw_core, 2, 'noncanonical_bytes')
    negative('syntax.trailing', raw_core + b'null', 2, 'noncanonical_bytes')
    negative('syntax.leading_zero_integer', raw_core.replace(b'"pack_version":1', b'"pack_version":01', 1),
             2, 'noncanonical_bytes')
    negative('syntax.unpaired_surrogate', b'{"kind":"\\ud800"}', 2, 'noncanonical_bytes')
    negative('syntax.depth65', b'[' * 65 + b']' * 65, 2, 'depth_capacity')
    depth64 = raw_core[:-1] + b',"z_extra":' + b'[' * 63 + b']' * 63 + b'}'
    negative('syntax.depth64_then_declared_schema', depth64, 5, 'unknown_field', '/z_extra')
    negative('syntax.first_fault_before_depth', b'![' + b'[' * 64, 2, 'noncanonical_bytes')
    negative('syntax.depth_before_later_fault', b'[' * 65 + b'!', 2, 'depth_capacity')
    negative('members.escaped_duplicate_before_header', b'{"kind":"UNKNOWN","k\\u0069nd":"SECOND"}', 3, 'duplicate_key', '/kind')
    negative('members.nested_duplicate', raw_core.replace(b'"storage_limit":10000', b'"storage_limit":10000,"storage_limit":1', 1), 3, 'duplicate_key', '/resources/0/storage_limit')
    unknown = copy.deepcopy(core)
    unknown['z_extra'], unknown['a_extra'] = 1, 2
    del unknown['pack_id']
    negative('schema.unknown_precedes_missing', encoded(unknown), 5, 'unknown_field', '/a_extra')
    missing = copy.deepcopy(core)
    del missing['industry_definitions'][0]['workforce_units']
    negative('schema.missing_required', encoded(missing), 5, 'noncanonical_bytes', '/industry_definitions/0/workforce_units')
    negative('canonical.leading_whitespace', b' ' + raw_core, 6, 'noncanonical_bytes')
    reordered = dict(reversed(list(core.items())))
    negative('canonical.root_field_order', encoded(reordered), 6, 'noncanonical_bytes',
             None, strength='exact_stage_error_only')
    escaped = encoded(changed(core, name_path, '\ufffd')).replace('\ufffd'.encode(), b'\\ufffd', 1)
    negative('canonical.escaped_replacement', escaped, 6, 'noncanonical_bytes', name_path)
    bad_ratio = encoded(changed(core, '/hazard_definitions/0/effect/denominator', 0))
    negative('canonical.before_semantic', b' ' + bad_ratio, 6, 'noncanonical_bytes')
    for path in ['/hazard_definitions/0/effect', '/technology_definitions/1/effect', '/technology_definitions/2/effect']:
        negative('fraction.zero_denominator.' + path, encoded(changed(core, path + '/denominator', 0)), 7, 'invalid_range', path + '/denominator')
        negative('fraction.amplification.' + path, encoded(changed(core, path + '/numerator', U32)), 7, 'invalid_range', path + '/numerator')
    half = changed(changed(core, '/hazard_definitions/0/effect/numerator', 2), '/hazard_definitions/0/effect/denominator', 4)
    negative('fraction.fixed_equivalent_2_4', encoded(half), 7, 'invalid_range', '/hazard_definitions/0/effect/numerator')
    both = changed(changed(core, '/hazard_definitions/0/effect/numerator', U32), '/hazard_definitions/0/effect/denominator', 0)
    negative('fraction.denominator_prerequisite', encoded(both), 7, 'invalid_range', '/hazard_definitions/0/effect/denominator')
    both = changed(changed(core, '/industry_definitions/0/alloy_cost', 41), '/resources/0/storage_limit', 9999)
    negative('semantic.schema_ordinal_priority', encoded(both), 7, 'invalid_range', '/resources/0/storage_limit')
    negative('quantity.zero_input', encoded(changed(core, '/industry_definitions/1/effect/inputs/0/quantity', 0)), 7, 'invalid_range', '/industry_definitions/1/effect/inputs/0/quantity')
    negative('quantity.zero_output', encoded(changed(core, '/industry_definitions/2/effect/outputs/0/quantity', 0)), 7, 'invalid_range', '/industry_definitions/2/effect/outputs/0/quantity')
    negative('enum.role', encoded(changed(core, '/hull_definitions/0/role', 'freighter')), 5, 'invalid_enum', '/hull_definitions/0/role')
    negative('enum.effect', encoded(changed(core, '/industry_definitions/0/effect/type', 'unknown')), 5, 'invalid_enum', '/industry_definitions/0/effect/type')
    negative('enum.intent', encoded(changed(core, '/policy_definitions/0/bonuses/0/intent', 'unknown')), 5, 'invalid_enum', '/policy_definitions/0/bonuses/0/intent')
    for intent in INTENTS:
        altered = copy.deepcopy(core)
        altered['policy_definitions'][3]['bonuses'] = [{'intent': intent, 'bonus': 20}]
        negative('policy.known_intent.' + intent, encoded(altered), 7, 'invalid_range', '/policy_definitions/3/bonuses')
    for tag, spec in INDUSTRY_EFFECT.variants.items():
        donor = next(row['effect'] for row in core['industry_definitions'] if row['effect']['type'] == tag)
        target = next(i for i, row in enumerate(core['industry_definitions']) if row['effect']['type'] != tag)
        negative('effect.known_variant.' + tag, encoded(changed(core, f'/industry_definitions/{target}/effect', donor)),
                 7, 'invalid_range', f'/industry_definitions/{target}/effect/type')
    for tag in TECH_EFFECT.variants:
        donor = next(row['effect'] for row in core['technology_definitions'] if row['effect']['type'] == tag)
        target = next(i for i, row in enumerate(core['technology_definitions']) if row['effect']['type'] != tag)
        negative('technology.known_variant.' + tag, encoded(changed(core, f'/technology_definitions/{target}/effect', donor)),
                 7, 'invalid_range', f'/technology_definitions/{target}/effect/type')
    for path in ['/pack_version', '/resources/0/storage_limit']:
        stem = '"' + path.rsplit('/', 1)[1] + '":'
        original = str(at(core, path))
        for label, spelling in [('negative', '-1'), ('float', '1.0'), ('exponent', '1e0')]:
            raw = raw_core.replace((stem + original).encode(), (stem + spelling).encode(), 1)
            negative('scalar.' + label + path, raw, 5, 'invalid_range', path)
    negative('header.first_constant_before_later_type',
             encoded(changed(changed(core, '/format_version', 4), '/rules_version', None)),
             4, 'unsupported_format_version', '/format_version')
    negative('header.before_unknown_member', encoded(changed(unknown, '/rules_version', 4)),
             4, 'unsupported_rules_version', '/rules_version')
    effect_unknown = changed(core, '/industry_definitions/0/effect', {'type': 'unknown', 'z': 0, 'a': 1})
    negative('enum.unknown_member_precedes_tag', encoded(effect_unknown), 5, 'unknown_field', '/industry_definitions/0/effect/a')
    negative('policy.changed_intent', encoded(changed(core, '/policy_definitions/0/bonuses/0/intent', 'adaptation')), 7, 'invalid_range', '/policy_definitions/0/bonuses/0/intent')
    for collection, cap in zip(COLLECTIONS, [64, 64, 4, 64, 32, 32, 32, 64]):
        missing = copy.deepcopy(core)
        missing[collection].pop()
        negative('roster.missing.' + collection, encoded(missing), 7, 'invalid_range', '/' + collection)
        extra = copy.deepcopy(core)
        extra_row = copy.deepcopy(extra[collection][-1])
        extra_row['id'] = 'zz_extra'
        extra[collection].append(extra_row)
        if collection == 'resources':
            # Four-resource exact count is a declared cap before child/equality.
            negative('roster.extra.' + collection, encoded(extra), 5, 'record_capacity', '/' + collection, actual=5, limit=4)
        else:
            negative('roster.extra.' + collection, encoded(extra), 7, 'invalid_range', '/' + collection)
        duplicate = copy.deepcopy(core)
        duplicate[collection].insert(1, copy.deepcopy(duplicate[collection][0]))
        if collection == 'resources':
            duplicate[collection].pop()
        negative('key.duplicate.' + collection, encoded(duplicate), 5, 'duplicate_key', '/' + collection + '/1/id')
        if len(core[collection]) > 1:
            descending = copy.deepcopy(core)
            descending[collection][0], descending[collection][1] = descending[collection][1], descending[collection][0]
            negative('key.descending.' + collection, encoded(descending), 5, 'noncanonical_bytes', '/' + collection + '/1/id')
        over = copy.deepcopy(core)
        over[collection] = [None] * (cap + 1)
        negative('cap.before_children.' + collection, encoded(over), 5, 'record_capacity', '/' + collection, actual=cap + 1, limit=cap)
    amounts = copy.deepcopy(core)
    amounts['industry_definitions'][3]['effect']['inputs'] *= 3
    negative('cap.amounts_before_children', encoded(amounts), 5, 'record_capacity', '/industry_definitions/3/effect/inputs', actual=6, limit=4)
    duplicate = copy.deepcopy(core)
    duplicate['industry_definitions'][3]['effect']['inputs'][1]['resource'] = 'energy'
    negative('key.amount_duplicate', encoded(duplicate), 5, 'duplicate_key', '/industry_definitions/3/effect/inputs/1/resource')
    duplicate = copy.deepcopy(core)
    duplicate['technology_definitions'][2]['prerequisites'] *= 2
    negative('key.prerequisite_duplicate', encoded(duplicate), 5, 'duplicate_key', '/technology_definitions/2/prerequisites/1')
    duplicate = copy.deepcopy(core)
    duplicate['policy_definitions'][0]['bonuses'][1]['intent'] = 'build_ark'
    negative('key.bonus_duplicate', encoded(duplicate), 5, 'duplicate_key', '/policy_definitions/0/bonuses/1/intent')
    swapped = copy.deepcopy(core)
    swapped['policy_definitions'][0]['bonuses'].reverse()
    negative('key.bonus_descending', encoded(swapped), 5, 'noncanonical_bytes', '/policy_definitions/0/bonuses/1/intent')
    prerequisite_cap = copy.deepcopy(core)
    prerequisite_cap['technology_definitions'][2]['prerequisites'] = [None] * 65
    negative('cap.prerequisites_before_children', encoded(prerequisite_cap), 5, 'record_capacity',
             '/technology_definitions/2/prerequisites', actual=65, limit=64)
    return cases


def products(sources):
    cases = document_cases()
    for case in cases:
        observed = admit(base64.b64decode(case['raw_base64']))
        for key, expected in case['expected'].items():
            if observed.get(key) != expected:
                raise AssertionError((case['id'], key, expected, observed))
    core = encoded(core_catalog())
    core_hash = sha(DOMAIN + core)
    metadata = next(c for c in cases if c['id'] == 'metadata.all_allowed_paths')
    altered = base64.b64decode(metadata['raw_base64'])
    link = admit(altered, expected_hash=core_hash)
    assert link['stage'] == 8 and link['error'] == 'catalog_mismatch'
    assert admit(altered, expected_hash=sha(DOMAIN + altered))['admitted']
    vectors = {'scope': 'COMPLETE-FOUNDATION-CATALOG Proposed complete finite Catalog only',
               'generator_sha256': sha(Path(__file__).read_bytes()),
               'source_manifest_sha256': FREEZE_HASH, 'sources': sources,
               'pack_domain_hex': DOMAIN.hex(), 'core_canonical_bytes': len(core),
               'core_pack_preimage_base64': base64.b64encode(DOMAIN + core).decode(),
               'core_pack_sha256': core_hash, 'cases': cases,
               'consumer_link_cases': [
                   {'id': 'matching_altered_pack', 'catalog_case': metadata['id'],
                    'expected_hash': sha(DOMAIN + altered), 'expected': {'admitted': True}},
                   {'id': 'mismatched_altered_pack', 'catalog_case': metadata['id'],
                    'expected_hash': core_hash, 'expected': {'stage': 8, 'error': 'catalog_mismatch'},
                    'strength': 'exact_consumer_check_stage_error_only',
                    'limitation': 'No consumer document is constructed; consumer-field pointer depends on its declared envelope.'}]}
    name_paths = [path for path, _ in leaves(core_catalog()) if path.endswith('/name')]
    ledger = []
    for path, value in leaves(core_catalog()):
        matches = [c['id'] for c in cases if c['id'].endswith(path.strip('/').replace('/', '.'))
                   or c['expected'].get('path') == path]
        metadata_path = path.endswith('/name') or path in ('/pack_id', '/pack_version')
        header_path = path in ('/kind', '/format_version', '/rules_version')
        semantic_owner = ('stage4 declared constant' if header_path else
                          'stage5 metadata domain; no fixed value equality' if metadata_path else
                          'stage7 exact definition roster, collection path' if path.endswith('/id') else
                          'stage7 exact finite value/domain, submitted field path')
        reference = {'applies': False}
        if path.endswith('/resource'):
            reference = {'applies': True, 'stage': 8, 'target': 'resources.id',
                         'permitted_finite_roster': ['alloy', 'energy', 'food', 'ore']}
        elif '/prerequisites/' in path:
            reference = {'applies': True, 'stage': 8, 'target': 'technology_definitions.id',
                         'graph': 'acyclic', 'earlier_exact_list_owner': 'stage7'}
        ledger.append({'path': path, 'type': 'u32' if type(value) is int else 'string/tag',
                       'family': 'metadata' if metadata_path else 'fixed_mechanics_or_header',
                       'structural_owner': 'stage4' if path in ('/kind', '/format_version', '/rules_version') else 'stage5',
                       'structural_predicate': structural_predicate(schema_at(path)),
                       'semantic_owner': semantic_owner,
                       'semantic_predicate': {'metadata_may_vary': True} if metadata_path else {'exact_finite_value': value},
                       'references': reference,
                       'examined': True, 'case_ids': matches,
                       'positive_support': 'all full admitted packs',
                       'negative_support': 'explicit case_ids if nonempty; otherwise enum/header/key/roster group cases',
                       'reference_owner': 'stage8 where applicable; altered fixed references usually reject earlier stage7'})
    coverage = {'scope': 'COMPLETE-FOUNDATION-CATALOG only, not full V3 admission', 'sources': sources,
                'generator_sha256': vectors['generator_sha256'],
                'counts': {'cases': len(cases), 'complete_positive_catalogs': sum('admitted' in c['expected'] for c in cases),
                           'negative_catalogs': sum('stage' in c['expected'] for c in cases),
                           'metadata_name_paths': len(name_paths), 'scalar_paths': len(ledger)},
                'positive_admission_ledger': [
                    {'stage': 0, 'checks': ['raw byte count <= Catalog1MiB']},
                    {'stage': 1, 'checks': ['strict UTF8, no replacement decoding']},
                    {'stage': 2, 'checks': ['complete bounded JSON syntax; explicit stack depth<=64; paired surrogate handling']},
                    {'stage': 3, 'checks': ['all decoded object keys unique before dictionary materialization']},
                    {'stage': 4, 'checks': ['kind PACK_V3, format3, rules3 in header order; widths/types']},
                    {'stage': 5, 'checks': ['all required ordered record/enum fields, no unknowns, all scalar grammars/widths/metadata bounds',
                                             'eight collection caps, amount4, prerequisite64, strict sorted unique keys']},
                    {'stage': 6, 'checks': ['complete canonical bytes equality, literalUnicode/no normalization']},
                    {'stage': 7, 'checks': ['exact complete finite definition/resource/effect/role/bonus/numeric/prerequisite profile',
                                             'valid positive pack_version, fraction prerequisite/nonamplification, actual listed ResourceAmount quantities']},
                    {'stage': 8, 'checks': ['all resource/technology references resolve; iterative acyclic prerequisite graph',
                                             'expected exact PACK hash only in optional external consumer-check context']},
                    {'stage': 9, 'checks': ['Catalog has no self-integrity field; compute PACK(domainNUL || complete canonical Catalog)']},
                    {'stage': 10, 'checks': [], 'applicability': 'Catalog contains no replay/history/issued record or witness; not claimed exercised'}],
                'scalar_predicates': ledger,
                'record_and_array_fields': [dict(row, **(
                    {'strict_key': schema_at(row['path']).key or 'slug value',
                     'structural_cap': schema_at(row['path']).cap,
                     'semantic_roster': 'exact finite snapshot, after canonical bytes'}
                    if row['kind'] == 'array' else {}))
                    for row in containers(core_catalog())],
                'record_variants': {'industry_effects': list(INDUSTRY_EFFECT.variants),
                                    'technology_effects': list(TECH_EFFECT.variants),
                                    'hazard_effects': list(HAZARD_EFFECT.variants),
                                    'hull_roles': list(HULL.fields[-1][1].choices),
                                    'intent_tags': list(INTENTS)},
                'uncovered_or_stopped': [
                    {'family': 'General V3 admission', 'status': 'out_of_scope',
                     'details': 'No Genesis/State/Actor/history/forecast/root/entity identity/Transfer/host proof constructed.'},
                    {'family': 'Catalog stage8 reference-negative ordering', 'status': 'earlier_guard_not_bypassable',
                     'details': 'Undefined resource/prerequisite changes violate stage7 fixed profile first; no invented missing_reference vector after earlier failure.'},
                    {'family': 'Consumer linkage paths', 'status': 'exact_stage_error_only',
                     'details': 'Actual consumer/root/identity documents are later scope; no /catalog_hash pointer is exported as universal envelope path.'},
                    {'family': 'Persisted diagnostic offset schema', 'status': 'not_adopted',
                     'details': 'Fixtures use bounded helper offsets only; no extra diagnostic wire fields or hashed failures invented.'},
                    {'family': 'Canonical reordered-member containing path', 'status': 'exact_stage_error_only',
                     'details': 'Full raw reordered object proves stage6/noncanonical_bytes; specific key-token versus enclosing-object pointer convention is not strengthened by this fixture.'},
                    {'family': 'All combinations of malformed child/key defects', 'status': 'partial',
                     'details': 'Positive traversal and isolated key errors are checked. No unreviewed simultaneous child-type versus duplicate-key encounter is exported as an exact multi-defect oracle.'},
                    {'family': 'Whole semantic matrix / runtime', 'status': 'partial',
                     'details': 'This finite Catalog predicate ledger cannot qualify other successful State/record families, performance or native/wasm codec behavior.'}]}
    dump = lambda x: (json.dumps(x, ensure_ascii=False, indent=2) + '\n').encode('utf-8')
    return {'vectors.json': dump(vectors), 'predicate-coverage.json': dump(coverage),
            'core.catalog.json': core}

