#!/usr/bin/env python3
"""Independent stdlib author corpus for the current empty-root fork family.
Not a production decoder, general collision oracle, or bounded job engine.
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
ROOT = HERE.parents[6]
CAT = HERE.parent.parent
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
FREEZE = SDD / 'v3-current-release-source-freeze'
CHECKPOINT = SDD / 'v3-current-history-approved-checkpoint-2'
PACK = '9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'
PINS = {
    'v3-current-release-source-freeze/manifest.json': 'd0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c',
    'v3-coordinated-amendment-current-source-manifest-2.json': 'bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f',
    'v3-coordinated-amendment-integrated-review.md': 'cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65',
    'v3-anchor-summary-rereview-1.md': '947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf',
    'v3-current-catalog-review.md': 'd556d3230b34a167c997bbbfb58e6a6308e517d5fa006b23c806dfb6f0ded9f6',
    'v3-current-genesis-review.md': 'b7167d88507540d7f4b5a19ceda44fe9ecf0425944cd64b7c065e23f0eaec042',
    'v3-current-genesis-wording-rereview.md': 'd993d81664fcb1b5f53bf55946a66065b26209959588c89f1ffeb36e423c0464',
    'v3-current-history-approved-checkpoint-2/manifest.json': 'e344d098fe6b0fe4752226ccfd22d31cea387bc3a47a9227301c08a5d240c046',
    'v3-current-fork-history-preflight.md': '10dfd1de4b44600848d81d6cd905a185ec8317d24710a8d146b45c452fd4ca60',
}
DEPENDENCIES = {
    'derive.py': 'c8d772c48420b1dcf1c3598ceba35bc6ab59baa08e87fb7bd5e2fb917757ae51',
    'core.catalog.json': 'efc762006d5abfa3e3a00d3aa9ba46b001e9e1951833b1c655ab6518b18f3f10',
    'vectors.json': 'e1f1acde5bd98c2795e81886cbf3e3db731bdcf8281ca437ecbfd3b4dacb6ca2',
    'predicate-coverage.json': 'edb40f42a3c07a00bf0f92e69f03ce77b2bfeae714f1a1d33c0b3229afd5a394',
    'README.md': '385502ca5144f82d778b4d91f271eedd49e7668a13efc2d9bd5257d82c18161b',
    'genesis/derive.py': 'f7e4a4b0f09397c9fe67b7d8d38a871e82ae5f44aa9971dc6fc67fe4edb35751',
    'genesis/empty.genesis.json': '4c5db9ebe76e19b36e76cd7333934f8697b81303001d99c10c27f35448ed03a0',
    'genesis/inhabited.genesis.json': '614d702ef8bcabe16493ee68ef79456541d1b26a3462a56b8e1bf6d4ec279d72',
    'genesis/vectors.json': '03a9ce6ba537a89132e3acddcd3fe2da8fb403699a7ddbb1f8cd0fa890ca65d9',
    'genesis/predicate-coverage.json': '1a37cb6494f4c0faadebf2979ba346da7e746715329e0410f9d486667cf81f9a',
    'genesis/README.md': '1dfd25ebff4d7db219014cacabb3fa22f3a4402ebd8609ed36c1a927618ee18d',
}
STATE_FIELDS = 'tick branch_sequence next_sequence systems stars worlds lanes civilizations deposits colonies facilities construction_jobs hull_jobs fleets routes shipments relations agreements wars observations hazards settlement_claims occupations offers demand_requests commitments defense_requests shared_intelligence stellar_events counters'.split()
LIMIT = (1 << 64) - 1


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def canonical(record):
    return json.dumps(record, ensure_ascii=False, separators=(',', ':')).encode('utf8')


def domain(name):
    return ('NYON_LIVING_V3_' + name).encode('ascii') + b'\0'


def u64(value):
    return struct.pack('<Q', int(value))


def length(raw):
    return struct.pack('<I', len(raw)) + raw


def optional(revision):
    return b'\0' if revision is None else b'\1' + bytes.fromhex(revision)


def checked_increment(value):
    value = int(value)
    if value == LIMIT:
        raise Rejected(10, 'invalid_range', '')
    return value + 1


def state_frame(state):
    return domain('STATE') + canonical(dict(format_version=3, rules_version=3, catalog_hash=PACK, state=state))


def state_hash(state):
    return digest(state_frame(state))


def seal(document, tag):
    document['integrity_sha256'] = digest(domain(tag) + canonical({k: v for k, v in document.items() if k != 'integrity_sha256'}))
    return document


def root_frame(genesis):
    generator = genesis['generator']
    return (domain('ROOT_BRANCH') + bytes.fromhex(PACK) + bytes.fromhex(genesis['seed']) +
            bytes.fromhex(digest(domain('GENESIS') + canonical(genesis))) +
            length(generator['generator_id'].encode()) + struct.pack('<I', generator['generator_version']))


def fork_frame(root, branch):
    return (domain('FORK_BRANCH') + bytes.fromhex(root) + bytes.fromhex(branch['parent']) +
            optional(branch['fork_revision']) + u64(branch['fork_tick']) +
            bytes.fromhex(branch['fork_state_digest']) + u64(branch['ordinal']))


def revision_frame(root, revision):
    return (domain('REVISION') + bytes.fromhex(root) + bytes.fromhex(revision['branch']) +
            optional(revision['parent']) + u64(revision['tick']) + u64(revision['ordinal']) +
            bytes([0 if revision['mode'] == 'paused' else 1]) +
            length(canonical(revision['capture'])) + length(canonical(revision['command'])))


def view(branch, revision, tick):
    return dict(branch=branch, revision=revision, tick=str(tick))


def view_key(value):
    return value['branch'], value['revision'], int(value['tick'])


def capture(branch, parent, state):
    h = state_hash(state)
    return dict(domain=3, catalog_hash=PACK, branch=branch, revision=parent, tick=state['tick'], state_digest=h, phase=1, permitted_snapshot_digest=h)


def change(before, after):
    return dict(operation_index=0, collection='systems', key=dict(type='systems', id=after['id']),
                before=None if before is None else dict(type='systems', value=copy.deepcopy(before)),
                after=dict(type='systems', value=copy.deepcopy(after)), resources=[], passenger_delta='0')


def make_receipt(state, revision, effects):
    events = [] if revision is None else [dict(phase=1, ordinal=0, provenance=dict(type='creator', revision=revision),
                                              kind='creator_intervention', facts=dict(type='change', changes=copy.deepcopy(effects)))]
    payload = dict(tick=state['tick'], applied_revisions=[] if revision is None else [revision], events=events, state_digest=state_hash(state))
    frame = domain('RECEIPT') + canonical(payload)
    h = digest(frame)
    public, event_frames = [], []
    for event in events:
        ef = domain('EVENT') + bytes.fromhex(h) + u64(state['tick']) + struct.pack('<HI', event['phase'], event['ordinal'])
        public.append(dict(id=digest(ef)[:32], **event))
        event_frames.append(dict(preimage_hex=ef.hex(), full_digest=digest(ef)))
    receipt = dict(tick=state['tick'], applied_revisions=payload['applied_revisions'], events=public, state_digest=payload['state_digest'], digest=h)
    return receipt, dict(payload=payload, preimage_hex=frame.hex(), event_frames=event_frames)


def anchor(branch, revision, state, receipt):
    return dict(domain=3, catalog_hash=PACK, branch=branch, revision=revision, tick=state['tick'],
                state_digest=state_hash(state), receipt_digest=receipt['digest'], event=None)


def anchor_key(row):
    event = row['event']
    return (row['branch'], '' if row['revision'] is None else row['revision'], int(row['tick']), row['receipt_digest'],
            (-1, -1, '') if event is None else (event['phase'], event['ordinal'], event['id']))


class Rejected(Exception):
    def __init__(self, stage, error, path):
        self.stage, self.error, self.path = stage, error, path


class Unsupported(Exception):
    """Author coverage boundary; never an authoritative wire error."""


def require(condition, path='', stage=10, error='replay_mismatch'):
    if not condition:
        raise Rejected(stage, error, path)


def pointer(path, token):
    return path + '/' + str(token).replace('~', '~0').replace('/', '~1')


def freshness():
    for name, expected in PINS.items():
        if digest((SDD / name).read_bytes()) != expected:
            raise SystemExit('STOP: frozen input changed ' + name)
    for name, expected in DEPENDENCIES.items():
        if digest((CAT / name).read_bytes()) != expected:
            raise SystemExit('STOP: accepted CAT/GEN changed ' + name)
    manifest = json.loads((CHECKPOINT / 'manifest.json').read_bytes())
    for entry in manifest['files']:
        for path in (ROOT / entry['path'], CHECKPOINT / Path(entry['path']).name):
            raw = path.read_bytes()
            if len(raw) != entry['bytes'] or digest(raw) != entry['sha256']:
                raise SystemExit('STOP: corrected HIST checkpoint changed ' + str(path))
    sources = json.loads((FREEZE / 'manifest.json').read_bytes())['sources']
    if len(sources) != 8:
        raise SystemExit('STOP: source roster changed')
    for name, entry in sources.items():
        for path in (FREEZE / name, ROOT / 'docs/superpowers/specs' / name):
            raw = path.read_bytes()
            if len(raw) != entry['bytes'] or digest(raw) != entry['sha256']:
                raise SystemExit('STOP: source bytes changed ' + str(path))
    catalog_raw = (CAT / 'core.catalog.json').read_bytes()
    if len(catalog_raw) != 4264 or digest(domain('PACK') + catalog_raw) != PACK:
        raise SystemExit('STOP: Catalog/PACK')
    return (json.loads(catalog_raw), json.loads((CAT / 'genesis/empty.genesis.json').read_bytes()),
            json.loads((HERE.parent / 'archive.json').read_bytes()), json.loads((HERE.parent / 'notebook.json').read_bytes()),
            dict(sources=sources, checkpoint=manifest))


def apply_revision(root, branch, parent, state, revision, expected_kind):
    require(revision['branch'] == branch and revision['parent'] == parent and revision['tick'] == state['tick'] and revision['mode'] == 'paused', '/revisions')
    require(canonical(revision['capture']) == canonical(capture(branch, parent, state)), '/revisions/capture')
    if revision['command']['type'] != 'creator_batch' or len(revision['command']['operations']) != 1:
        raise Unsupported('Other command family or operation count')
    op = revision['command']['operations'][0]
    require(op['type'] == expected_kind, '/revisions/command/operations/0/type')
    s = copy.deepcopy(state)
    issuance = None
    if expected_kind == 'create_system':
        require(not s['systems'] and op['local'] > 0, '/revisions/command')
        issuance = (domain('CREATOR_ENTITY') + bytes.fromhex(root) + bytes.fromhex(branch) + b'\2' +
                    u64(revision['ordinal']) + struct.pack('<IHH', 0, 1, op['local']))
        row = dict(id=digest(issuance)[:32], name=op['name'])
        effects = [change(None, row)]
        s['systems'] = [row]
    else:
        require(len(s['systems']) == 1 and op['target']['type'] == 'existing' and op['target']['id'] == s['systems'][0]['id'], '/revisions/command/operations/0/target')
        before = copy.deepcopy(s['systems'][0])
        s['systems'][0]['name'] = op['name']
        effects = [change(before, s['systems'][0])]
    require(canonical(effects) == canonical(revision['command']['reviewed_effects']), '/revisions/command/reviewed_effects')
    require(int(revision['ordinal']) > int(s['next_sequence']), '/revisions/ordinal')
    require(digest(revision_frame(root, revision)) == revision['id'], '/revisions/id', 9, 'integrity_mismatch')
    s['next_sequence'] = revision['ordinal']
    receipt, rf = make_receipt(s, revision['id'], effects)
    audit = dict(view=view(branch, revision['id'], s['tick']), capture=revision['capture'], command=revision['command'],
                 revision_preimage_hex=revision_frame(root, revision).hex(), state=copy.deepcopy(s), state_preimage_hex=state_frame(s).hex(), receipt=receipt, receipt_frame=rf)
    if issuance is not None:
        audit['issuance'] = dict(origin=dict(branch=branch, accepted_sequence=revision['ordinal'], operation_index=0, kind=1, local=op['local']),
                                 preimage_hex=issuance.hex(), full_digest=digest(issuance))
    return s, receipt, audit


def replay_root(archive, genesis):
    if canonical(archive['genesis']) != canonical(genesis):
        raise Unsupported('Different Genesis family; no blanket admission prohibition')
    root = digest(root_frame(genesis))[:32]
    roots = [b for b in archive['branches'] if b['parent'] is None]
    require(len(roots) == 1, '/branches')
    b = roots[0]
    require(b['id'] == root, '/branches/id', 9, 'integrity_mismatch')
    require(b['fork_revision'] is None and b['fork_tick'] == '0' and b['ordinal'] == '0' and b['fork_state_digest'] == state_hash(genesis['state']), '/branches')
    revisions = [r for r in archive['revisions'] if r['branch'] == root]
    if len(revisions) != 2:
        raise Unsupported('Other parent revision count')
    first = [r for r in revisions if r['parent'] is None]
    require(len(first) == 1, '/revisions')
    first = first[0]
    second = [r for r in revisions if r['parent'] == first['id']]
    require(len(second) == 1, '/revisions')
    second = second[0]
    if first['ordinal'] != '1' or second['ordinal'] != '2':
        raise Unsupported('Other parent accepted-sequence profile')
    state = copy.deepcopy(genesis['state'])
    views = {view_key(view(root, None, 0)): (copy.deepcopy(state), None)}
    audits = []
    parent = None
    for revision, kind in ((first, 'create_system'), (second, 'edit_system')):
        state, receipt, audit = apply_revision(root, root, parent, state, revision, kind)
        audits.append(audit)
        views[view_key(audit['view'])] = (copy.deepcopy(state), receipt)
        parent = revision['id']
    state['tick'] = '1'
    receipt, rf = make_receipt(state, None, [])
    audits.append(dict(view=view(root, parent, 1), state=copy.deepcopy(state), state_preimage_hex=state_frame(state).hex(), receipt=receipt, receipt_frame=rf))
    views[view_key(view(root, parent, 1))] = (copy.deepcopy(state), receipt)
    require(b['head'] == parent and b['tip_tick'] == '1', '/branches')
    return root, first, second, views, audits


def validate_seal(document, tag, path):
    expected = copy.deepcopy(document)
    seal(expected, tag)
    require(expected['integrity_sha256'] == document['integrity_sha256'], pointer(path, 'integrity_sha256'), 9, 'integrity_mismatch')


def replay_archive(archive, genesis, parent_only=False):
    require(archive['catalog_hash'] == PACK, '/catalog_hash', 8, 'catalog_mismatch')
    validate_seal(archive, 'ARCHIVE', '')
    root, first, second, views, audits = replay_root(archive, genesis)
    if parent_only:
        require(len(archive['branches']) == 1 and len(archive['revisions']) == 2, '/branches')
    else:
        if len(archive['branches']) != 2 or len(archive['revisions']) != 3:
            raise Unsupported('Other retained branch/revision count')
        child = next(b for b in archive['branches'] if b['parent'] is not None)
        require(child['parent'] == root and child['fork_revision'] == first['id'] and child['fork_tick'] == '0' and int(child['ordinal']) > 0, '/branches')
        parent_state, _ = views[view_key(view(root, first['id'], 0))]
        require(child['fork_state_digest'] == state_hash(parent_state), '/branches/fork_state_digest')
        require(child['id'] == digest(fork_frame(root, child))[:32], '/branches/id', 9, 'integrity_mismatch')
        require(child['id'] != root, '/branches/id')
        copied = copy.deepcopy(parent_state)
        copied['branch_sequence'] = child['ordinal']
        fork_view = view(child['id'], first['id'], 0)
        views[view_key(fork_view)] = (copy.deepcopy(copied), None)
        local = [r for r in archive['revisions'] if r['branch'] == child['id']]
        if len(local) != 1:
            raise Unsupported('Other child publication count')
        revision = local[0]
        require(int(revision['ordinal']) > int(second['ordinal']), '/revisions/ordinal')
        copied, receipt, audit = apply_revision(root, child['id'], first['id'], copied, revision, 'edit_system')
        views[view_key(audit['view'])] = (copy.deepcopy(copied), receipt)
        audits.append(audit)
        require(child['head'] == revision['id'], '/branches/head')
        if child['tip_tick'] != '0':
            raise Unsupported('Later child evolution outside this family')
    require(int(archive['accepted_sequence_high_water']) >= max(int(r['ordinal']) for r in archive['revisions']), '/accepted_sequence_high_water', 8, 'allocator_regression')
    require(int(archive['branch_sequence_high_water']) >= max(int(b['ordinal']) for b in archive['branches']), '/branch_sequence_high_water', 8, 'allocator_regression')
    active = view_key(archive['active_view'])
    require(active in views, '/active_view', 8, 'invalid_history_view')
    require(archive['final_tick'] == archive['active_view']['tick'] and archive['final_digest'] == state_hash(views[active][0]), '/final_digest')
    for a in archive['anchors']:
        key = view_key(a)
        require(key in views, '/anchors', 8, 'invalid_history_view')
        state, receipt = views[key]
        require(receipt is not None and canonical(a) == canonical(anchor(a['branch'], a['revision'], state, receipt)), '/anchors', 10, 'forged_evidence')
    return views, audits


def validate_notebook(notebook, archive, genesis, views):
    require(notebook['catalog_hash'] == PACK, '/catalog_hash', 8, 'catalog_mismatch')
    validate_seal(notebook, 'NOTEBOOK', '')
    require(notebook['genesis_digest'] == digest(domain('GENESIS') + canonical(genesis)) and notebook['archive_integrity_sha256'] == archive['integrity_sha256'], '/archive_integrity_sha256')
    require(int(notebook['next_ordinal']) > 0, '/next_ordinal')
    for b in notebook['bookmarks']:
        key = view_key(b['view'])
        require(key in views, '/bookmarks/view', 8, 'invalid_history_view')
        require(0 < int(b['ordinal']) < int(notebook['next_ordinal']) and b['digest'] == state_hash(views[key][0]), '/bookmarks/digest')
    if notebook['experiments'] or notebook['evidence']:
        raise Unsupported('Notebook experiments/evidence')


def build(catalog, genesis, parent_archive, parent_notebook, name='Gamma', label='Fork Gamma'):
    parent_views, parent_audits = replay_archive(parent_archive, genesis, True)
    validate_notebook(parent_notebook, parent_archive, genesis, parent_views)
    root, first, _, _, _ = replay_root(parent_archive, genesis)
    original = parent_views[view_key(view(root, first['id'], 0))][0]
    f = checked_increment(parent_archive['branch_sequence_high_water'])
    s = checked_increment(parent_archive['accepted_sequence_high_water'])
    child = dict(id='', parent=root, fork_revision=first['id'], fork_tick='0', fork_state_digest=state_hash(original), ordinal=str(f), head=first['id'], tip_tick='0')
    ff = fork_frame(root, child)
    child['id'] = digest(ff)[:32]
    state = copy.deepcopy(original)
    state['branch_sequence'] = str(f)
    initial = copy.deepcopy(state)
    op = dict(type='edit_system', target=dict(type='existing', id=state['systems'][0]['id']), name=name)
    after = dict(id=state['systems'][0]['id'], name=name)
    effects = [change(state['systems'][0], after)]
    revision = dict(id='', branch=child['id'], parent=first['id'], tick='0', ordinal=str(s), mode='paused', capture=capture(child['id'], first['id'], state),
                    command=dict(type='creator_batch', operations=[op], reviewed_effects=effects))
    revision['id'] = digest(revision_frame(root, revision))
    state, receipt, publication = apply_revision(root, child['id'], first['id'], state, revision, 'edit_system')
    child['head'] = revision['id']
    selected = view(child['id'], revision['id'], 0)
    archive = copy.deepcopy(parent_archive)
    archive['revisions'] = sorted(archive['revisions'] + [revision], key=lambda r: r['id'])
    archive['branches'] = sorted(archive['branches'] + [child], key=lambda b: b['id'])
    archive['active_view'], archive['final_tick'], archive['final_digest'] = selected, '0', state_hash(state)
    archive['accepted_sequence_high_water'], archive['branch_sequence_high_water'] = str(s), str(f)
    archive['anchors'] = sorted(archive['anchors'] + [anchor(child['id'], revision['id'], state, receipt)], key=anchor_key)
    seal(archive, 'ARCHIVE')
    notebook = copy.deepcopy(parent_notebook)
    ordinal = notebook['next_ordinal']
    notebook['archive_integrity_sha256'] = archive['integrity_sha256']
    notebook['next_ordinal'] = str(checked_increment(ordinal))
    notebook['bookmarks'].append(dict(ordinal=ordinal, label=label, view=copy.deepcopy(selected), digest=state_hash(state)))
    seal(notebook, 'NOTEBOOK')
    transfer = seal(dict(kind='NYON_LIVING_TRANSFER_V3', format_version=3, rules_version=3, catalog=copy.deepcopy(catalog), archive=archive, notebook=notebook, selected_view=copy.deepcopy(selected)), 'TRANSFER')
    trace = dict(status='Proposed FORK-HIST audit artifact, not portable fields', root_preimage_hex=root_frame(genesis).hex(), parent_publications=parent_audits,
                 fork_preimage_hex=ff.hex(), fork_full_digest=digest(ff), fork_initial=dict(view=view(child['id'], first['id'], 0), state=initial, state_preimage_hex=state_frame(initial).hex(), receipt=None),
                 fork_acceptance=dict(queue_empty=True, branch_high_water=str(f), accepted_high_water=parent_archive['accepted_sequence_high_water']),
                 child_envelope=dict(domain=3, catalog_hash=PACK, branch=child['id'], expected_committed_revision=first['id'], expected_tick='0', expected_state_digest=state_hash(initial), expected_pending_sequence='0', mode='paused', command=revision['command']),
                 child_publication=publication,
                 original_issuer_origin=parent_audits[0]['issuance']['origin'],
                 archive_preimage_hex=(domain('ARCHIVE') + canonical({k: v for k, v in archive.items() if k != 'integrity_sha256'})).hex(),
                 notebook_preimage_hex=(domain('NOTEBOOK') + canonical({k: v for k, v in notebook.items() if k != 'integrity_sha256'})).hex(),
                 transfer_preimage_hex=(domain('TRANSFER') + canonical({k: v for k, v in transfer.items() if k != 'integrity_sha256'})).hex())
    return archive, notebook, transfer, trace


# Fresh narrow source declarations: schema1574-1614,1723-1759,2575-2617.
def rec(**fields):
    return ('record', fields)


def opt(child):
    return ('option', child)


def arr(child, cap, key=None):
    return ('array', child, cap, key)


SYSTEM = rec(id='ID', name='Name')
VIEW = rec(branch='ID', revision=opt('Digest'), tick='u64')
SNAPSHOT = rec(domain='u32', catalog_hash='Digest', branch='ID', revision=opt('Digest'), tick='u64', state_digest='Digest', phase='u16', permitted_snapshot_digest='Digest')
RECORD = ('tagged', {'systems': rec(type='text', value=SYSTEM)})
KEY = ('tagged', {'systems': rec(type='text', id='ID')})
TARGET = ('tagged', {'existing': rec(type='text', id='ID'), 'local': rec(type='text', local='u16')})
OPERATION = ('tagged', {'create_system': rec(type='text', local='u16', name='Name'), 'edit_system': rec(type='text', target=TARGET, name='Name')})
CHANGE = rec(operation_index=opt('u32'), collection='text', key=KEY, before=opt(RECORD), after=opt(RECORD), resources=arr('unsupported', None), passenger_delta='score')
COMMAND = ('tagged', {'creator_batch': rec(type='text', operations=arr(OPERATION, 1024), reviewed_effects=arr(CHANGE, None))})
REVISION = rec(id='Digest', branch='ID', parent=opt('Digest'), tick='u64', ordinal='u64', mode=('enum', ['paused', 'running']), capture=SNAPSHOT, command=COMMAND)
BRANCH = rec(id='ID', parent=opt('ID'), fork_revision=opt('Digest'), fork_tick='u64', fork_state_digest='Digest', ordinal='u64', head=opt('Digest'), tip_tick='u64')
STATE = rec(**{field: 'u64' if field in STATE_FIELDS[:3] else rec() if field == 'counters' else arr(SYSTEM if field == 'systems' else 'unsupported', 64 if field == 'systems' else None, 'id' if field == 'systems' else None) for field in STATE_FIELDS})
GENESIS = rec(kind='text', format_version='u32', rules_version='u32', catalog_hash='Digest', seed='Digest', generator=rec(generator_id='Slug', generator_version='u32'), allocations=arr('unsupported', None), state=STATE)
ANCHOR = rec(domain='u32', catalog_hash='Digest', branch='ID', revision=opt('Digest'), tick='u64', state_digest='Digest', receipt_digest='Digest', event=opt('unsupported'))
ARCHIVE = rec(kind='text', format_version='u32', rules_version='u32', catalog_hash='Digest', genesis=GENESIS, revisions=arr(REVISION, 10000, 'id'), branches=arr(BRANCH, 128, 'id'), active_view=VIEW, final_tick='u64', final_digest='Digest', accepted_sequence_high_water='u64', branch_sequence_high_water='u64', anchors=arr(ANCHOR, 16384, 'anchor'), integrity_sha256='Digest')
BOOKMARK = rec(ordinal='u64', label='Name', view=VIEW, digest='Digest')
NOTEBOOK = rec(kind='text', format_version='u32', rules_version='u32', catalog_hash='Digest', genesis_digest='Digest', archive_integrity_sha256='Digest', next_ordinal='u64', bookmarks=arr(BOOKMARK, 1024, 'ordinal'), experiments=arr('unsupported', 256), evidence=arr('unsupported', 4096), integrity_sha256='Digest')
TRANSFER = rec(kind='text', format_version='u32', rules_version='u32', catalog='catalog_dependency', archive=ARCHIVE, notebook=NOTEBOOK, selected_view=VIEW, integrity_sha256='Digest')
SCHEMAS = dict(archive=ARCHIVE, notebook=NOTEBOOK, transfer=TRANSFER)
CAPS = dict(archive=64 * 1024 * 1024, notebook=8 * 1024 * 1024, transfer=96 * 1024 * 1024)


def shape(value, declaration, catalog, path=''):
    if isinstance(declaration, str):
        if declaration == 'unsupported':
            raise Unsupported(path)
        if declaration == 'catalog_dependency':
            if canonical(value) != canonical(catalog):
                raise Unsupported('Different Catalog family')
            return value
        if declaration in ('u16', 'u32'):
            require(type(value) is int and 0 <= value < 2 ** int(declaration[1:]), path, 5, 'invalid_range')
        else:
            require(isinstance(value, str), path, 5, 'invalid_range')
            if declaration in ('u64', 'score'):
                regex = '0|[1-9][0-9]*' if declaration == 'u64' else '0|-?[1-9][0-9]*'
                require(re.fullmatch(regex, value) is not None, path, 5, 'noncanonical_bytes')
                require(len(value) <= 20 and (0 <= int(value) <= LIMIT if declaration == 'u64' else -(1 << 63) <= int(value) < 1 << 63), path, 5, 'invalid_range')
            elif declaration in ('ID', 'Digest'):
                require(re.fullmatch('[0-9a-f]{' + ('32' if declaration == 'ID' else '64') + '}', value) is not None, path, 5, 'noncanonical_bytes')
            elif declaration == 'Slug':
                require(re.fullmatch('[a-z][a-z0-9_]{0,63}', value) is not None, path, 5, 'invalid_range')
            elif declaration == 'Name':
                require(1 <= len(value) <= 64 and len(value.encode()) <= 256 and not value.startswith(' ') and not value.endswith(' ') and '  ' not in value and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in value), path, 5, 'invalid_range')
        return value
    kind = declaration[0]
    if kind == 'option':
        return None if value is None else shape(value, declaration[1], catalog, path)
    if kind == 'enum':
        require(value in declaration[1], path, 5, 'invalid_enum')
        return value
    if kind == 'tagged':
        require(isinstance(value, dict) and 'type' in value, pointer(path, 'type'), 5, 'noncanonical_bytes')
        require(isinstance(value['type'], str), pointer(path, 'type'), 5, 'invalid_range')
        if value['type'] not in declaration[1]:
            raise Unsupported('Unexamined tag at ' + path)
        return shape(value, declaration[1][value['type']], catalog, path)
    if kind == 'record':
        require(isinstance(value, dict), path, 5, 'invalid_range')
        extras = set(value) - set(declaration[1])
        require(not extras, pointer(path, min(extras, key=lambda k: k.encode())) if extras else path, 5, 'unknown_field')
        ordered = {}
        for field, child in declaration[1].items():
            require(field in value, pointer(path, field), 5, 'noncanonical_bytes')
            ordered[field] = shape(value[field], child, catalog, pointer(path, field))
        return ordered
    require(isinstance(value, list), path, 5, 'invalid_range')
    if declaration[2] is not None:
        require(len(value) <= declaration[2], path, 5, 'record_capacity')
    ordered = [shape(child, declaration[1], catalog, pointer(path, i)) for i, child in enumerate(value)]
    key = declaration[3]
    if key is not None:
        keys = [anchor_key(v) if key == 'anchor' else int(v[key]) if key == 'ordinal' else v[key] for v in ordered]
        for i in range(1, len(keys)):
            # ID equality is not exported as a global installed/issuance result.
            require(keys[i] >= keys[i - 1], pointer(path, i), 5, 'noncanonical_bytes')
            if keys[i] == keys[i - 1]:
                if path.endswith('/systems'):
                    raise Rejected(7, 'duplicate_identity', pointer(path, i))
                require(False, pointer(path, i), 5, 'duplicate_key')
    return ordered


def header(document, family, path=''):
    for field, expected in [('kind', 'NYON_LIVING_' + family.upper() + '_V3'), ('format_version', 3), ('rules_version', 3)]:
        require(field in document, pointer(path, field), 4, 'noncanonical_bytes')
        value = document[field]
        require(isinstance(value, str) if field == 'kind' else type(value) is int and 0 <= value < 1 << 32, pointer(path, field), 4, 'invalid_range')
        require(value == expected, pointer(path, field), 4, 'unsupported_kind' if field == 'kind' else 'unsupported_' + field)


class Pairs(list):
    pass


def decode(raw, cap):
    require(len(raw) <= cap, '', 0, 'byte_capacity')
    try:
        text = raw.decode('utf8')
        parsed = json.loads(text, object_pairs_hook=Pairs)
    except (UnicodeError, ValueError, RecursionError):
        raise Rejected(2, 'noncanonical_bytes', '') from None
    def materialize(v, path='', depth=1):
        require(depth <= 64, path, 2, 'depth_capacity')
        if isinstance(v, Pairs):
            output = {}
            for k, child in v:
                require(k not in output, pointer(path, k), 3, 'duplicate_key')
                output[k] = materialize(child, pointer(path, k), depth + 1 if isinstance(child, list) else depth)
            return output
        if isinstance(v, list):
            return [materialize(child, pointer(path, i), depth + 1 if isinstance(child, list) else depth) for i, child in enumerate(v)]
        return v
    return materialize(parsed)


def admit(raw, family, catalog, genesis, attachment=None):
    doc = decode(raw, CAPS[family])
    require(isinstance(doc, dict), '', 4, 'invalid_range')
    header(doc, family)
    ordered = shape(doc, SCHEMAS[family], catalog)
    require(canonical(ordered) == raw, '', 6, 'noncanonical_bytes')
    if family == 'archive':
        header(doc['genesis'], 'genesis', '/genesis')
        return replay_archive(doc, genesis)
    if family == 'notebook':
        require(attachment is not None, '/archive_integrity_sha256', 8, 'missing_reference')
        views, audits = replay_archive(attachment, genesis)
        validate_notebook(doc, attachment, genesis, views)
        return views, audits
    # All embedded constants precede inner replay/link usage; diagnostics not
    # promoted from this author's private ordering to general decoder semantics.
    header(doc['archive'], 'archive', '/archive')
    header(doc['notebook'], 'notebook', '/notebook')
    header(doc['archive']['genesis'], 'genesis', '/archive/genesis')
    validate_seal(doc, 'TRANSFER', '')
    views, audits = replay_archive(doc['archive'], genesis)
    validate_notebook(doc['notebook'], doc['archive'], genesis, views)
    require(view_key(doc['selected_view']) in views, '/selected_view', 8, 'invalid_history_view')
    return views, audits


def outcome(raw, family, catalog, genesis, attachment):
    try:
        admit(raw, family, catalog, genesis, attachment)
        return dict(admitted=True)
    except Rejected as error:
        return dict(admitted=False, stage=error.stage, error=error.error, path=error.path)
    except Unsupported as error:
        return dict(coverage='unsupported_family', detail=str(error))


def mutate(document, path, value=None, delete=False):
    result = copy.deepcopy(document)
    tokens = [t.replace('~1', '/').replace('~0', '~') for t in path.split('/')[1:]]
    row = result
    for token in tokens[:-1]:
        row = row[int(token)] if isinstance(row, list) else row[token]
    key = int(tokens[-1]) if isinstance(row, list) else tokens[-1]
    if delete:
        del row[key]
    else:
        row[key] = value
    return result


def replace_links(record, mapping):
    if isinstance(record, dict):
        return {k: replace_links(v, mapping) for k, v in record.items()}
    if isinstance(record, list):
        return [replace_links(v, mapping) for v in record]
    return mapping.get(record, record) if isinstance(record, str) else record


def reframe_archive(archive):
    """Reframe submitted inputs and outer seal, not repair any false source fact."""
    archive = copy.deepcopy(archive)
    root = next(b['id'] for b in archive['branches'] if b['parent'] is None)
    child = next(b for b in archive['branches'] if b['parent'] is not None)
    new_child = digest(fork_frame(root, child))[:32]
    archive = replace_links(archive, {child['id']: new_child})
    rows = sorted(archive['revisions'], key=lambda r: (r['branch'] != root, int(r['ordinal'])))
    for original in rows:
        row = next(r for r in archive['revisions'] if r['id'] == original['id'])
        archive = replace_links(archive, {row['id']: digest(revision_frame(root, row))})
    archive['branches'].sort(key=lambda b: b['id'])
    archive['revisions'].sort(key=lambda r: r['id'])
    archive['anchors'].sort(key=anchor_key)
    return seal(archive, 'ARCHIVE')


def trace_matches(trace, archive, genesis):
    try:
        views, audits = replay_archive(archive, genesis)
        child = next(b for b in archive['branches'] if b['parent'] is not None)
        state, receipt = views[view_key(trace['fork_initial']['view'])]
        return (receipt is None and trace['fork_initial']['receipt'] is None and canonical(state) == canonical(trace['fork_initial']['state']) and
                trace['fork_initial']['state_preimage_hex'] == state_frame(state).hex() and
                trace['fork_preimage_hex'] == fork_frame(next(b['id'] for b in archive['branches'] if b['parent'] is None), child).hex() and
                canonical(trace['parent_publications']) == canonical(audits[:3]) and canonical(trace['child_publication']) == canonical(audits[3]))
    except (Rejected, KeyError):
        return False


def products(catalog, genesis, parent_archive, parent_notebook, pins):
    archive, notebook, transfer, trace = build(catalog, genesis, parent_archive, parent_notebook)
    cases = []
    def add(identifier, family, doc, expected, strength, attachment=None, raw=None):
        raw = canonical(doc) if raw is None else raw
        actual = outcome(raw, family, catalog, genesis, attachment)
        if strength == 'rejection_only':
            require(actual.get('admitted') is False, identifier)
        else:
            require(all(actual.get(k) == v for k, v in expected.items()), identifier)
        cases.append(dict(id=identifier, entry_point=family, raw_base64=base64.b64encode(raw).decode(), expectation=expected, strength=strength,
                          attachment_archive_base64=None if attachment is None else base64.b64encode(canonical(attachment)).decode()))
    for prefix, (a, n, t, _) in [('base', (archive, notebook, transfer, trace)), ('unicode', build(catalog, genesis, parent_archive, parent_notebook, 'Gamma Ω', 'Fork β'))]:
        for family, doc in [('archive', a), ('notebook', n), ('transfer', t)]:
            add('positive.' + prefix + '.' + family, family, doc, dict(admitted=True), 'complete_fork_history_admission', a if family == 'notebook' else None)
    views, _ = replay_archive(archive, genesis)
    child = next(b for b in archive['branches'] if b['parent'] is not None)
    root = next(b for b in archive['branches'] if b['parent'] is None)
    for title, selected in [('root_future', view(root['id'], root['head'], 1)), ('root_fork_parent', view(root['id'], child['fork_revision'], 0)), ('fork_only', view(child['id'], child['fork_revision'], 0))]:
        alt = copy.deepcopy(transfer)
        alt['selected_view'] = selected
        seal(alt, 'TRANSFER')
        add('positive.transfer.' + title, 'transfer', alt, dict(admitted=True), 'complete_fork_history_admission')
    holes = copy.deepcopy(notebook)
    holes['next_ordinal'] = '9'
    seal(holes, 'NOTEBOOK')
    add('positive.notebook.mark_holes', 'notebook', holes, dict(admitted=True), 'complete_fork_history_admission', archive)
    extra = copy.deepcopy(notebook)
    selected = view(child['id'], child['fork_revision'], 0)
    extra['bookmarks'].append(dict(ordinal='3', label='Fork only', view=selected, digest=state_hash(views[view_key(selected)][0])))
    extra['next_ordinal'] = '4'
    seal(extra, 'NOTEBOOK')
    add('positive.notebook.fork_only_bookmark', 'notebook', extra, dict(admitted=True), 'complete_fork_history_admission', archive)
    root_active = copy.deepcopy(archive)
    selected = view(root['id'], root['head'], 1)
    root_active['active_view'], root_active['final_tick'], root_active['final_digest'] = selected, '1', state_hash(views[view_key(selected)][0])
    seal(root_active, 'ARCHIVE')
    add('positive.archive.root_active', 'archive', root_active, dict(admitted=True), 'complete_fork_history_admission')
    for family, doc in [('archive', archive), ('notebook', notebook), ('transfer', transfer)]:
        attachment = archive if family == 'notebook' else None
        def walk(value, path=''):
            if isinstance(value, dict):
                for k, v in value.items():
                    target = pointer(path, k)
                    if target.startswith('/catalog/'):
                        continue
                    expected = dict(admitted=False, stage=4 if target in ('/kind', '/format_version', '/rules_version') else 5, error='noncanonical_bytes', path=target)
                    add('missing.' + family + target.replace('/', '.'), family, mutate(doc, target, delete=True), expected, 'exact_stage_error_path', attachment)
                    walk(v, target)
            elif isinstance(value, list):
                for i, v in enumerate(value):
                    walk(v, pointer(path, i))
        walk(doc)
        for field, value, error in [('kind', 'OTHER', 'unsupported_kind'), ('format_version', 4, 'unsupported_format_version'), ('rules_version', 4, 'unsupported_rules_version')]:
            add('header.' + family + '.' + field, family, mutate(doc, '/' + field, value), dict(admitted=False, stage=4, error=error, path='/' + field), 'exact_stage_error_path', attachment)
        add('integrity.' + family, family, mutate(doc, '/integrity_sha256', 'ff' * 32), dict(admitted=False, stage=9, error='integrity_mismatch', path='/integrity_sha256'), 'exact_stage_error_path', attachment)
        add('canonical.' + family, family, doc, dict(admitted=False, stage=6, error='noncanonical_bytes'), 'exact_stage_error_only', attachment, b' ' + canonical(doc))
    child_index = archive['branches'].index(child)
    root_index = archive['branches'].index(root)
    rindex = next(i for i, r in enumerate(archive['revisions']) if r['branch'] == child['id'])
    child_prefix = '/branches/' + str(child_index)
    root_prefix = '/branches/' + str(root_index)
    revision_prefix = '/revisions/' + str(rindex)
    mutations = [
        ('fork.parent_digest_is_child', child_prefix + '/fork_state_digest', state_hash(trace['fork_initial']['state'])),
        ('fork.wrong_tick', child_prefix + '/fork_tick', '1'),
        ('fork.wrong_parent_revision', child_prefix + '/fork_revision', root['head']),
        ('fork.zero_ordinal', child_prefix + '/ordinal', '0'),
        ('fork.head_root_future', child_prefix + '/head', root['head']),
        ('root.future_changed', root_prefix + '/tip_tick', '0'),
        ('root.head_rewound', root_prefix + '/head', child['fork_revision']),
        ('child.parent_root_future', revision_prefix + '/parent', root['head']),
        ('child.reused_global_ordinal', revision_prefix + '/ordinal', '2'),
        ('child.capture_parent_digest', revision_prefix + '/capture/state_digest', child['fork_state_digest']),
        ('child.capture_permitted_parent', revision_prefix + '/capture/permitted_snapshot_digest', child['fork_state_digest']),
        ('child.capture_root_branch', revision_prefix + '/capture/branch', root['id']),
        ('child.capture_wrong_tick', revision_prefix + '/capture/tick', '1'),
        ('child.capture_wrong_phase', revision_prefix + '/capture/phase', 5),
        ('child.capture_null_parent', revision_prefix + '/capture/revision', None),
        ('child.reviewed_wrong_before', revision_prefix + '/command/reviewed_effects/0/before/value/name', 'Beta'),
        ('child.reviewed_header_operation', revision_prefix + '/command/reviewed_effects/0/operation_index', 1),
        ('child.fake_system_id', revision_prefix + '/command/operations/0/target/id', 'ff' * 16),
        ('child.final_parent_digest', '/final_digest', child['fork_state_digest']),
        ('child.final_fork_only_digest', '/final_digest', state_hash(trace['fork_initial']['state'])),
        ('marks.accepted_regression', '/accepted_sequence_high_water', '2'),
        ('marks.branch_regression', '/branch_sequence_high_water', '0'),
    ]
    for title, path, value in mutations:
        changed = reframe_archive(mutate(archive, path, value))
        add('replay.' + title, 'archive', changed, dict(admitted=False), 'rejection_only')
    leaked = copy.deepcopy(trace['fork_initial']['state'])
    leaked['next_sequence'] = '2'
    changed = copy.deepcopy(archive)
    changed['revisions'][rindex]['capture']['state_digest'] = state_hash(leaked)
    changed['revisions'][rindex]['capture']['permitted_snapshot_digest'] = state_hash(leaked)
    add('replay.child.capture_leaked_sibling_mark', 'archive', reframe_archive(changed), dict(admitted=False), 'rejection_only')
    for field in ('branches', 'revisions', 'anchors'):
        changed = copy.deepcopy(archive)
        changed[field] = list(reversed(changed[field]))
        seal(changed, 'ARCHIVE')
        add('sorted.' + field, 'archive', changed, dict(admitted=False, stage=5, error='noncanonical_bytes', path='/' + field + '/1'), 'exact_stage_error_path')
    for title, path, value in [('stale_attachment', '/archive_integrity_sha256', parent_archive['integrity_sha256']), ('child_parent_state', '/bookmarks/1/digest', child['fork_state_digest']), ('child_pre_edit_state', '/bookmarks/1/digest', state_hash(trace['fork_initial']['state'])), ('unreachable_child', '/bookmarks/1/view/tick', '1'), ('zero_mark', '/next_ordinal', '0')]:
        changed = seal(mutate(notebook, path, value), 'NOTEBOOK')
        add('notebook.' + title, 'notebook', changed, dict(admitted=False), 'rejection_only', archive)
    fake = copy.deepcopy(archive)
    fake['anchors'].append(dict(domain=3, catalog_hash=PACK, branch=child['id'], revision=child['fork_revision'], tick='0', state_digest=state_hash(trace['fork_initial']['state']), receipt_digest=parent_archive['anchors'][-1]['receipt_digest'], event=None))
    fake['anchors'].sort(key=anchor_key)
    seal(fake, 'ARCHIVE')
    add('replay.fork_only_fake_receipt', 'archive', fake, dict(admitted=False), 'rejection_only')
    for title, path, value in [('unreachable', '/selected_view/tick', '2'), ('archive_kind', '/archive/kind', 'OTHER'), ('notebook_version', '/notebook/rules_version', 4), ('genesis_kind', '/archive/genesis/kind', 'OTHER')]:
        changed = mutate(transfer, path, value)
        seal(changed['archive'], 'ARCHIVE')
        changed['notebook']['archive_integrity_sha256'] = changed['archive']['integrity_sha256']
        seal(changed['notebook'], 'NOTEBOOK')
        seal(changed, 'TRANSFER')
        add('transfer.' + title, 'transfer', changed, dict(admitted=False), 'rejection_only')
    require(trace_matches(trace, archive, genesis))
    trace_cases = []
    for path, value in [('/fork_initial/state/branch_sequence', '0'), ('/fork_initial/state/next_sequence', '2'), ('/fork_initial/receipt', trace['child_publication']['receipt']), ('/child_publication/state/next_sequence', '1'), ('/child_publication/state/branch_sequence', '0'), ('/child_publication/receipt/state_digest', child['fork_state_digest']), ('/child_publication/receipt/digest', 'ff' * 32), ('/child_publication/receipt/events/0/id', 'ff' * 16)]:
        altered = mutate(trace, path, value)
        require(not trace_matches(altered, archive, genesis))
        trace_cases.append(dict(id=path, mutated_trace=altered, expectation=dict(exact_replay_matches=False), strength='trace_binding_rejection_only'))
    declarations = {}
    def enumerate_schema(schema, path):
        if isinstance(schema, str):
            declarations[path] = dict(primitive=schema)
        elif schema[0] == 'record':
            declarations[path] = dict(required_fields=list(schema[1]))
            for name, child_schema in schema[1].items():
                enumerate_schema(child_schema, pointer(path, name))
        elif schema[0] == 'array':
            declarations[path] = dict(cap=schema[2], key=schema[3], unsupported_children=schema[1] == 'unsupported')
            enumerate_schema(schema[1], path + '/*')
        elif schema[0] == 'option':
            enumerate_schema(schema[1], path + '?')
        elif schema[0] == 'tagged':
            declarations[path] = dict(covered_tags=list(schema[1]), other_tags_unexamined=True)
            for name, child_schema in schema[1].items():
                enumerate_schema(child_schema, path + ':' + name)
        else:
            declarations[path] = dict(enum=schema[1])
    for family, schema in SCHEMAS.items():
        enumerate_schema(schema, family)
    later_child = copy.deepcopy(archive)
    later_child['branches'][child_index]['tip_tick'] = '1'
    seal(later_child, 'ARCHIVE')
    scope_checks = {}
    for name, document in [('admitted_parent_without_fork', parent_archive), ('unqualified_later_child_tick', later_child)]:
        classification = outcome(canonical(document), 'archive', catalog, genesis, None)
        require(classification.get('coverage') == 'unsupported_family', name)
        scope_checks[name] = dict(coverage='unsupported_family', wire_error_claim=False)
    positive_checks = {
        'source_and_dependency_freshness': 'Live and immutable source8, CAT/GEN bytes, corrected HIST8 and its four reports must match their pinned hashes before deriving and before publication.',
        'complete_canonical_records': 'All 30 State fields and every populated required descendant are present in declaration order; scalar widths/spelling, Name grammar, typed keys and supported tags are checked without sorting input.',
        'catalog_and_empty_genesis': 'Exact admitted Catalog bytes reconstruct PACK; exact admitted empty Genesis is a dependency, not a global richer-Genesis prohibition.',
        'root_and_parent_ancestry': 'ROOT_BRANCH is independently framed; paused S1 CreateSystem and S2 EditSystem authenticate their real chain, captures, effects, wrapped States and publications, then ordinary1 preserves headers and changes tick only.',
        'inherited_creator_origin': 'System kind1/creator flag2/root issuer/S1/op0/local1 full frame is reproduced; child edit targets the same inherited ID and creates no entity.',
        'fork_frame': 'F1 binds original root(R1,0) digest, exact parent cursor and u64 ordinal; child-adjusted digest never substitutes for parent digest.',
        'fork_header_only': 'Child copies parent State and changes only branch_sequence1; historical next_sequence1/tick0 persist; no fork Revision, Receipt, Event or Anchor.',
        'separate_acceptance_capture': 'Audit Envelope binds child published R1/t0/Df0/pending0; Capture is child pre-operation phase1 with matching permitted digest; fork mark1 does not advance accepted mark2.',
        'child_global_sequence_and_changes': 'Next document accepted sequence3 applies one EditSystem Gamma with exact Alpha before/Gamma after Change, operation_index0, no resources/passengers/header facts; resulting State keeps branch_sequence1 and sets next_sequence3.',
        'revision_and_receipt_frames': 'All Revision IDs use full32 frames; each paused publication has one direct intervention and no cascade, ID-free Receipt payload seals first, then EventID binds original Receipt/tick/phase/ordinal.',
        'branch_tables_and_retained_tips': 'Root headR2/tip1 and all original root inputs survive; child headR3/tip0 uses inherited cross-branch parentR1; both exact tips replay irrespective of active selection.',
        'document_marks_and_active_view': 'Marks dominate retained ordinals; historical State headers never inherit sibling marks; active final_tick/digest describes the exact reachable view rather than maximum sibling tip.',
        'whole_receipt_anchors': 'Four full-key-sorted descriptors regenerate three original root Receipts and one child Receipt, including exact State/receipt digests and null event.',
        'notebook_binding_and_metadata': 'Exact Catalog/Genesis/Archive attachment, original root Bookmark, child Bookmark digest and valid Unicode labels; ordinals positive/unique/below next_ordinal, holes legal and fork-only Bookmark needs no Receipt.',
        'transfer_linkage_and_selection': 'Complete embedded Catalog/Archive/Notebook and constants verified before downstream replay links; selected retained root future/parent/child pre-edit/final views resolve independently of active_view.',
        'integrity_and_unsupported_scope': 'Own Archive/Notebook/Transfer domain seals exclude only own final integrity member; unsupported tags, richer Genesis/Catalog or nonempty unexamined collections are coverage boundaries, never invented wire errors.',
    }
    coverage = dict(status='Proposed narrow FORK-HIST; broader history Partial', pins=PINS, cat_gen_dependencies=DEPENDENCIES, accepted_hist_checkpoint=pins['checkpoint'], sources=pins['sources'], declarations=declarations,
                    strengths={strength: sum(c['strength'] == strength for c in cases) for strength in sorted({c['strength'] for c in cases})}, trace_binding_count=len(trace_cases),
                    scope_classification_checks=scope_checks,
                    positive_admission_checks={name: dict(predicate=predicate, scope='selected empty two-Branch family only; capacities exercised below limits', positive_cases=[c['id'] for c in cases if c['strength'] == 'complete_fork_history_admission']) for name, predicate in positive_checks.items()},
                    source_predicates={'parent_replay': 'schema2433-2485,2630-2648', 'fork_frame': 'schema2405-2408', 'fork_headers': 'schema2489-2495', 'cross_branch_parent': 'schema2489,2495,2632-2639',
                                       'global_marks': 'schema2422-2440,2493,2637', 'receipt': 'schema2329-2340,2460-2485; wire149-154', 'anchors': 'schema2491,2503-2511,2642-2648',
                                       'notebook': 'schema2497-2511,2654-2659', 'transfer': 'schema2614-2617,2692-2697', 'collision_boundaries': 'schema2512-2569,2824'},
                    gaps=['One empty root/two paused parent operations/ordinary1/one separately accepted child fork/one paused child edit only; other shapes unsupported coverage, not invalid wire.',
                          'Generic Branch origin/tag and cross-namespace collision diagnostics unbound; no new allocation kind, no synthetic collision oracle.',
                          'Inherited single creator origin checked from parent frame; no general retained-history collision scanner/performance or transient removed allocation proof.',
                          'Author JSON materialization is recursive/buffered and not production syntax/depth/duplicate first-fault ordering or bounded-memory/poll/cancellation evidence.',
                          'No general stage7-10 multi-fault diagnostic ordering claim; semantic cases export rejection only.',
                          'No running queue, combined fork/edit failure, forks beyond two branches, child ordinary tick, cascades/removals, experiments/Evidence or durability.',
                          'Fork-only selected view/Bookmark has no Receipt; no VIEW digest/portable queue/checkpoint/cache flag.'])
    pretty = lambda value: json.dumps(value, ensure_ascii=False, indent=2).encode() + b'\n'
    return dict(archive=canonical(archive), notebook=canonical(notebook), transfer=canonical(transfer), trace=pretty(trace), vectors=pretty(dict(status='Proposed FORK-HIST', cases=cases, trace_binding_cases=trace_cases)), coverage=pretty(coverage))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('mode', choices=('generate', 'verify'))
    args = parser.parse_args()
    catalog, genesis, parent_archive, parent_notebook, pins = freshness()
    output = products(catalog, genesis, parent_archive, parent_notebook, pins)
    freshness()
    files = {'archive': 'archive.json', 'notebook': 'notebook.json', 'transfer': 'transfer.json', 'trace': 'trace.json', 'vectors': 'vectors.json', 'coverage': 'predicate-coverage.json'}
    for key, name in files.items():
        path = HERE / name
        if args.mode == 'generate':
            path.write_bytes(output[key])
        elif not path.exists() or path.read_bytes() != output[key]:
            raise SystemExit('STOP: generated bytes differ ' + name)
    print(args.mode + ' passed: ' + str(len(json.loads(output['vectors'])['cases'])) + ' raw cases')
    for key, name in files.items():
        print(name, len(output[key]), digest(output[key]))


if __name__ == '__main__':
    main()
