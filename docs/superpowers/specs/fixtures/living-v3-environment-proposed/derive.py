#!/usr/bin/env python3
"""Independent Proposed environmental scope/identity/expiry formula subset."""
import hashlib
import json
from pathlib import Path
import sys

BASE = Path(__file__).resolve().parent
SPECS = BASE.parent.parent
FREEZE = {'2026-10-01-living-v3-complete-schema.md': '9a05423608640515a6797fc3e73167a7213266d2e790f4d47ee73e0589996d73', '2026-10-02-living-v3-scoring.md': 'c8bd14e773cc59fc8c9f7eb78b1eea6b5a32f648c6955b00ffe757f34c34976d', '2026-10-01-living-ecology-rules.md': '003659ee5eacd9b952344066eb6024964b630273505f598a86a6b3dfd3287fc1'}


def canonical(value):
    # Fixtures contain ASCII identifiers, no escapes or control bytes.
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf-8')


def ident(n):
    return f'{n:032x}'


def tick(n):
    return str(n)  # Tick is canonical decimal-string u64.


def child(family, target, observed, end, factor):
    common = {'id': ident(3)}
    if family == 'stellar':
        common.update(star=ident(10 if target == ident(1) else 11), system=target)
    else:
        common.update(definition='fixture_hazard_half' if factor == (1, 2) else 'fixture_hazard_quarter', lane=target)
    common.update(warning_tick='0', start_tick='0', end_tick=tick(end))
    if family == 'stellar':
        common.update(habitability_impact=0)
    common.update(freight_factor={'numerator': factor[0], 'denominator': factor[1]},
                  originating_observer=ident(4), observed_tick=tick(observed))
    return common


def scope(family, target, observed, events):
    return {('system' if family == 'stellar' else 'lane'): target,
            'originating_observer': ident(4), 'observed_tick': tick(observed),
            'events': events}


def rank(v):
    return (-int(v['observed_tick']), bytes.fromhex(v['originating_observer']), canonical(v))


def select(family, scopes, capture_tick):
    key = 'system' if family == 'stellar' else 'lane'
    winners = {}
    for s in scopes:
        assert int(s['observed_tick']) <= capture_tick
        for e in s['events']:
            assert e[key] == s[key]
            assert e['observed_tick'] == s['observed_tick']
            assert e['originating_observer'] == s['originating_observer']
            assert int(e['warning_tick']) <= int(s['observed_tick']) < int(e['end_tick'])
        if s[key] not in winners or rank(s) < rank(winners[s[key]]):
            winners[s[key]] = s
    winning = [winners[k] for k in sorted(winners)]
    identities = {}
    for s in winning:
        for e in s['events']:  # Deliberately includes now-expired retained children.
            if e['id'] not in identities or rank(e) < rank(identities[e['id']]):
                identities[e['id']] = e
    selected = [identities[k] for k in sorted(identities)]
    positive = [e for e in selected if int(e['end_tick']) > capture_tick]
    assert len(positive) <= 128
    return winning, selected, positive


def forecast(family, positive, forecast_tick, lane, systems):
    batch = 10
    steps = []
    for e in positive:
        target_match = e['system'] in systems if family == 'stellar' else e['lane'] == lane
        if target_match and int(e['start_tick']) <= forecast_tick < int(e['end_tick']):
            before = batch
            f = e['freight_factor']
            batch = batch * f['numerator'] // f['denominator']
            steps.append({'event': e['id'], 'before': before, 'after': batch})
    return {'tick': tick(forecast_tick), 'input_batch': 10, 'steps': steps, 'output_batch': batch}


def corpus():
    for name, expected in FREEZE.items():
        assert hashlib.sha256((SPECS / name).read_bytes()).hexdigest() == expected, 'source_freeze_changed'
    cases = []
    for family in ('stellar', 'hazard'):
        old = scope(family, ident(1), 10, [child(family, ident(1), 10, 200, (1, 2))])
        new = scope(family, ident(2), 20, [child(family, ident(2), 20, 30, (1, 4))])
        for name, T, sources in [('before_expiry', 25, [old, new]),
                                 ('at_expiry', 30, [old, new]),
                                 ('after_expiry', 40, [old, new]),
                                 ('new_source_evicted', 40, [old]),
                                 ('new_source_refreshed_empty', 40, [old, scope(family, ident(2), 40, [])]),
                                 ('old_scope_refreshed_empty', 40, [scope(family, ident(1), 40, []), new])]:
            winning, identities, positive = select(family, sources, T)
            fragment = {'known_hazards': positive if family == 'hazard' else [],
                        'known_stellar_events': positive if family == 'stellar' else [],
                        'known_hazard_scopes': winning if family == 'hazard' else [],
                        'known_stellar_scopes': winning if family == 'stellar' else []}
            raw = canonical(fragment)
            outputs = [forecast(family, positive, T + 1, ident(1), [ident(1), ident(9)]),
                       forecast(family, positive, T + 1, ident(2), [ident(2), ident(9)])]
            expected = [10, 2] if name == 'before_expiry' else [5, 10] if name in ('new_source_evicted', 'new_source_refreshed_empty') else [10, 10]
            assert [f['output_batch'] for f in outputs] == expected
            assert len(positive) == (1 if name in ('before_expiry', 'new_source_evicted', 'new_source_refreshed_empty') else 0)
            cases.append({'name': family + '_' + name, 'capture_tick': tick(T),
                          'retained_source_scopes': sources, 'winning_scopes': winning,
                          'identity_winners_before_expiry_filter': identities,
                          'positive_roster': positive,
                          'forecast_lanes': [{'lane': ident(1), 'systems': [ident(1), ident(9)]},
                                             {'lane': ident(2), 'systems': [ident(2), ident(9)]}],
                          'forecasts': outputs, 'canonical_actor_environment_fragment_utf8': raw.decode(),
                          'canonical_fragment_hex': raw.hex(),
                          'fragment_sha256': hashlib.sha256(raw).hexdigest()})
    return {'status': 'PROPOSED_PARTIAL_GAP6_WRITTEN_FORMULA_SUBSET', 'source_freeze': FREEZE,
            'inputs': 'Synthetic source-fragment cases, not full valid State/Observation documents or engine outputs. Referenced topology and immutable fixture_hazard_half(1/2) and fixture_hazard_quarter(1/4) definitions supplied as symbolic context.',
            'hash_contract': 'SHA256 of exact canonical actor environmental fragment, diagnostic content hash only. NOT full permitted_snapshot_digest, trace digest, Receipt or State digest.',
            'cases': cases,
            'remaining': ['Full valid actor observation/state documents and normative whole actor hash', 'Authenticated observation replay and source eviction mechanics', 'All integrated I1-I6 and full six-gap vectors/runtime qualification']}


def main():
    assert len(sys.argv) == 2 and sys.argv[1] in ('generate', 'verify')
    raw = (json.dumps(corpus(), ensure_ascii=False, indent=2) + '\n').encode()
    target = BASE / 'generatedvectors.json'
    if sys.argv[1] == 'generate':
        target.write_bytes(raw)
    else:
        assert target.read_bytes() == raw, 'generated_corpus_drift'
    print(sys.argv[1] + ': 12 environmental scope/identity/expiry fragments')


if __name__ == '__main__':
    main()
