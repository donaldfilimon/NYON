"""Fresh B0–B7 explicit profile builders; no generator runs in admission."""
import copy
import genesis as g
from catalog import core_catalog, encoded

def build(*, systems=2, stars=2, worlds=2, civilizations=1, deposits=1, colonies=1,
          lanes=1, fleets=1, hulls_per_fleet=1, population=200, seed='00'*32,
          generator_id='fixture_general', generator_version=1, ordinal_start=1,
          catalog=None, name='Harbor', resource='ore', amount=20000):
    catalog = catalog or core_catalog()
    doc = g.envelope()
    doc['catalog_hash'] = g.sha(g.domain('PACK') + encoded(catalog))
    doc['seed'] = seed
    doc['generator'] = dict(generator_id=generator_id, generator_version=generator_version)
    state = doc['state']
    next_ord = {}
    def issue(kind):
        ordinal = next_ord.get(kind, ordinal_start)
        next_ord[kind] = ordinal + 1
        ident = g.sha(g.allocation_frame(doc, kind, ordinal))[:32]
        doc['allocations'].append(dict(kind=kind, ordinal=str(ordinal), id=ident))
        return ident
    ss = [issue(1) for _ in range(systems)]
    cs = [issue(5) for _ in range(civilizations)]
    ws = [issue(3) for _ in range(worlds)]
    state['systems'] = [dict(id=i, name=name) for i in ss]
    state['stars'] = [dict(id=issue(2), system=ss[i % systems], archetype='main_sequence', name=name,
                           next_activity_tick='1000') for i in range(stars)]
    state['worlds'] = [dict(id=w, system=ss[i % systems], archetype='temperate', name=name,
                           owner=cs[i % civilizations] if i < colonies else None,
                           inventory=dict(energy=100, ore=100, alloy=120, food=100), environment=g.environment())
                       for i,w in enumerate(ws)]
    pairs = [(a,b) for i,a in enumerate(sorted(ss)) for b in sorted(ss)[i+1:]]
    assert lanes <= len(pairs)
    state['lanes'] = [dict(id=issue(4),system_a=a,system_b=b,distance_units='100') for a,b in pairs[:lanes]]
    state['civilizations'] = [dict(id=c, name=name, policy='neutral',
                                  research=dict(target=None,progress=[],completed_technologies=[]),
                                  status='active' if any(w['owner']==c for w in state['worlds'][:colonies]) else 'dormant') for c in cs]
    state['deposits'] = [dict(id=issue(6),world=ws[i % worlds],resource=resource,remaining_units=str(amount)) for i in range(deposits)]
    state['colonies'] = [dict(world=w['id'], owner=w['owner'],
                             hub=dict(energy_next_due='10',ore_next_due='100',fallback_next_due='100',blocked_reasons=[]),
                             society=dict(population=population,specialization='balanced',next_needs_tick='100',last_needs=None,workforce_assignments=[]))
                         for w in state['worlds'][:colonies]]
    hp = next(h['hit_points'] for h in catalog['hull_definitions'] if h['role']=='scout')
    state['fleets'] = [dict(id=issue(10),owner=cs[i % civilizations],location=dict(type='docked',world=ws[i % worlds]),order=None,
                           hulls=sorted([dict(id=issue(11),definition='scout',hit_points=hp,return_credit=False,next_repair=None,
                                              passengers=dict(count=0,voyage_origin=None)) for _ in range(hulls_per_fleet)],key=lambda h:h['id']),
                           supply=dict(last_success=None,last_attempt=None,readiness='never_supplied'),blockade=None) for i in range(fleets)]
    for name in g.ISSUED: state[name].sort(key=lambda row:row['id'])
    state['colonies'].sort(key=lambda row:row['world'])
    state['observations'] = g.project(state,catalog)
    doc['allocations'].sort(key=lambda row:(row['kind'],int(row['ordinal'])))
    return doc

def remove_rows(doc, *names):
    doc = copy.deepcopy(doc)
    removed = {r['id'] for name in names for r in doc['state'][name] if 'id' in r}
    for name in names: doc['state'][name]=[]
    doc['allocations']=[a for a in doc['allocations'] if a['id'] not in removed]
    return doc
