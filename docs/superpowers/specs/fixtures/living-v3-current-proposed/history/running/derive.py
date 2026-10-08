#!/usr/bin/env python3
"""Proposed RUN-HIST selected-family derivation, never a production decoder.
Source formulas only; immutable admitted parent bytes are replay inputs.
"""
import argparse
import base64
import copy
import hashlib
import json
import re
import struct
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[6]
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
FREEZE = SDD / 'v3-current-release-source-freeze'
HIST = SDD / 'v3-current-history-approved-checkpoint-2'
CAT = HERE.parent.parent
PINS = {
 'v3-current-release-source-freeze/manifest.json': 'd0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c',
 'v3-coordinated-amendment-current-source-manifest-2.json': 'bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f',
 'v3-current-catalog-review.md': 'd556d3230b34a167c997bbbfb58e6a6308e517d5fa006b23c806dfb6f0ded9f6',
 'v3-current-genesis-review.md': 'b7167d88507540d7f4b5a19ceda44fe9ecf0425944cd64b7c065e23f0eaec042',
 'v3-current-genesis-wording-rereview.md': 'd993d81664fcb1b5f53bf55946a66065b26209959588c89f1ffeb36e423c0464',
 'v3-current-history-approved-checkpoint-2/manifest.json': 'e344d098fe6b0fe4752226ccfd22d31cea387bc3a47a9227301c08a5d240c046',
 'v3-current-fork-history-review.md': 'e3d863caae932bd3c83760d789e1865aae0a23db0be8f97ee4a4fe7346367cac',
 'v3-current-running-queue-preflight.md': '9590031cc62ca116f8d058f411a5baf47fe7e73702d031c49d88f3e3b340f22b',
 'v3-coordinated-amendment-integrated-review.md': 'cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65',
 'v3-anchor-summary-rereview-1.md': '947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf',
 'v3-current-fork-history-checkpoint-1/manifest.json': 'ab8d674e8dbf111f81cb3e2db58e4dc807bc40a6f7adb15adda7ec7b916fbf6a',
}
INPUTS = {
 'derive.py': 'c8d772c48420b1dcf1c3598ceba35bc6ab59baa08e87fb7bd5e2fb917757ae51',
 'vectors.json': 'e1f1acde5bd98c2795e81886cbf3e3db731bdcf8281ca437ecbfd3b4dacb6ca2',
 'predicate-coverage.json': 'edb40f42a3c07a00bf0f92e69f03ce77b2bfeae714f1a1d33c0b3229afd5a394',
 'README.md': '385502ca5144f82d778b4d91f271eedd49e7668a13efc2d9bd5257d82c18161b',
 'genesis/derive.py': 'f7e4a4b0f09397c9fe67b7d8d38a871e82ae5f44aa9971dc6fc67fe4edb35751',
 'genesis/inhabited.genesis.json': '614d702ef8bcabe16493ee68ef79456541d1b26a3462a56b8e1bf6d4ec279d72',
 'genesis/vectors.json': '03a9ce6ba537a89132e3acddcd3fe2da8fb403699a7ddbb1f8cd0fa890ca65d9',
 'genesis/predicate-coverage.json': '1a37cb6494f4c0faadebf2979ba346da7e746715329e0410f9d486667cf81f9a',
 'genesis/README.md': '1dfd25ebff4d7db219014cacabb3fa22f3a4402ebd8609ed36c1a927618ee18d',
 'core.catalog.json': 'efc762006d5abfa3e3a00d3aa9ba46b001e9e1951833b1c655ab6518b18f3f10',
 'genesis/empty.genesis.json': '4c5db9ebe76e19b36e76cd7333934f8697b81303001d99c10c27f35448ed03a0',
}
FIELDS = 'tick branch_sequence next_sequence systems stars worlds lanes civilizations deposits colonies facilities construction_jobs hull_jobs fleets routes shipments relations agreements wars observations hazards settlement_claims occupations offers demand_requests commitments defense_requests shared_intelligence stellar_events counters'.split()
PACK = '9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'


def C(x):
 return json.dumps(x, ensure_ascii=False, separators=(',', ':')).encode('utf8')


def H(x):
 return hashlib.sha256(x).hexdigest()


def D(n):
 return ('NYON_LIVING_V3_' + n).encode() + b'\0'


def Q(x):
 return struct.pack('<Q', int(x))


def L(x):
 return struct.pack('<I', len(x)) + x


def O(x):
 return b'\0' if x is None else b'\1' + bytes.fromhex(x)


def inc(x):
 need(0 <= int(x) < 2**64-1, '', 10, 'invalid_range')
 return str(int(x) + 1)


class Rejected(Exception):
 def __init__(self, stage, error, path):
  self.stage, self.error, self.path = stage, error, path


class Unsupported(Exception):
 """Coverage outcome, never a wire rejection."""


def need(ok, path='', stage=10, error='replay_mismatch'):
 if not ok:
  raise Rejected(stage, error, path)


def ptr(p, k):
 return p + '/' + str(k).replace('~', '~0').replace('/', '~1')


def freshness():
 for name, digest in PINS.items():
  if H((SDD/name).read_bytes()) != digest:
   raise SystemExit('STOP pinned report/source changed: ' + name)
 for name, digest in INPUTS.items():
  if H((CAT/name).read_bytes()) != digest:
   raise SystemExit('STOP accepted bytes changed: ' + name)
 manifest = json.loads((FREEZE/'manifest.json').read_bytes())
 need(len(manifest['sources']) == 8)
 for name, entry in manifest['sources'].items():
  for p in (FREEZE/name, ROOT/'docs/superpowers/specs'/name):
   need(p.stat().st_size == entry['bytes'] and H(p.read_bytes()) == entry['sha256'])
 hm = json.loads((HIST/'manifest.json').read_bytes())
 for entry in hm['files']:
  for p in (ROOT/entry['path'], HIST/Path(entry['path']).name):
   need(p.stat().st_size == entry['bytes'] and H(p.read_bytes()) == entry['sha256'])
 fm = json.loads((SDD/'v3-current-fork-history-checkpoint-1/manifest.json').read_bytes())
 for name, entry in fm['files'].items():
  live = SDD/name if name.endswith('report.md') else HERE.parent/'fork'/name
  for p in (live, SDD/'v3-current-fork-history-checkpoint-1'/name):
   need(p.stat().st_size == entry['bytes'] and H(p.read_bytes()) == entry['sha256'])
 return manifest, hm


def state_frame(s):
 return D('STATE') + C(dict(format_version=3, rules_version=3, catalog_hash=PACK, state=s))


def state_digest(s):
 return H(state_frame(s))


def root_frame(g):
 return D('ROOT_BRANCH') + bytes.fromhex(PACK) + bytes.fromhex(g['seed']) + bytes.fromhex(H(D('GENESIS')+C(g))) + L(g['generator']['generator_id'].encode()) + struct.pack('<I', g['generator']['generator_version'])


def revision_frame(root, r):
 return D('REVISION') + bytes.fromhex(root) + bytes.fromhex(r['branch']) + O(r['parent']) + Q(r['tick']) + Q(r['ordinal']) + bytes([{'paused': 0, 'running': 1}[r['mode']]]) + L(C(r['capture'])) + L(C(r['command']))


def seal(d, name):
 d['integrity_sha256'] = H(D(name) + C({k:v for k,v in d.items() if k != 'integrity_sha256'}))
 return d


def check_seal(d, name, p=''):
 need(d['integrity_sha256'] == H(D(name)+C({k:v for k,v in d.items() if k != 'integrity_sha256'})), ptr(p, 'integrity_sha256'), 9, 'integrity_mismatch')


def view(b, r, t):
 return dict(branch=b, revision=r, tick=str(t))


def vk(v):
 return v['branch'], v['revision'], int(v['tick'])


def anchor_key(a):
 e = a['event']
 return a['branch'], a['revision'] or '', int(a['tick']), a['receipt_digest'], (-1, -1, '') if e is None else (e['phase'], e['ordinal'], e['id'])


def snapshot(s, branch, parent):
 h = state_digest(s)
 return dict(domain=3, catalog_hash=PACK, branch=branch, revision=parent, tick=s['tick'], state_digest=h, phase=1, permitted_snapshot_digest=h)


def change(old, new):
 return dict(operation_index=0, collection='systems', key=dict(type='systems', id=new['id']), before=None if old is None else dict(type='systems', value=copy.deepcopy(old)), after=dict(type='systems', value=copy.deepcopy(new)), resources=[], passenger_delta='0')


def execute(s, r, root, parent, mode):
 """Precondition: working tick already set for running phase1."""
 need(r['branch'] == root and r['parent'] == parent and r['tick'] == s['tick'] and r['mode'] == mode)
 need(r['capture'] == snapshot(s, root, parent), '/capture')
 need(int(r['ordinal']) > int(s['next_sequence']))
 cmd = r['command']
 need(cmd['type'] == 'creator_batch' and len(cmd['operations']) == 1)
 op = cmd['operations'][0]
 old = copy.deepcopy(s['systems'][0]) if s['systems'] else None
 issuance = None
 if op['type'] == 'create_system':
  need(old is None)
  frame = D('CREATOR_ENTITY') + bytes.fromhex(root)*2 + b'\2' + Q(r['ordinal']) + struct.pack('<IHH', 0, 1, op['local'])
  new = dict(id=H(frame)[:32], name=op['name'])
  issuance = dict(origin=dict(branch=root, accepted_sequence=r['ordinal'], operation_index=0, kind=1, local=op['local']), preimage_hex=frame.hex(), full_digest=H(frame))
 elif op['type'] == 'edit_system':
  need(old is not None and op['target'] == dict(type='existing', id=old['id']))
  new = dict(id=old['id'], name=op['name'])
 else:
  raise Unsupported('Other command family')
 ch = change(old, new)
 need(cmd['reviewed_effects'] == [ch], '/reviewed_effects')
 rf = revision_frame(root, r)
 need(r['id'] == H(rf), '/id', 9, 'integrity_mismatch')
 out = copy.deepcopy(s)
 out['systems'] = [new]
 out['next_sequence'] = r['ordinal']
 audit = dict(capture=copy.deepcopy(r['capture']), command=copy.deepcopy(cmd), revision_preimage_hex=rf.hex(), result_state=copy.deepcopy(out), state_preimage_hex=state_frame(out).hex(), change=ch)
 if issuance:
  audit['issuance'] = issuance
 return out, audit


def receipt(s, applied, changes):
 events = [dict(phase=1, ordinal=i, provenance=dict(type='creator', revision=r), kind='creator_intervention', facts=dict(type='change', changes=[copy.deepcopy(ch)])) for i, (r, ch) in enumerate(changes)]
 payload = dict(tick=s['tick'], applied_revisions=sorted(applied), events=events, state_digest=state_digest(s))
 frame = D('RECEIPT') + C(payload)
 digest = H(frame)
 frames, public = [], []
 for e in events:
  ef = D('EVENT') + bytes.fromhex(digest) + Q(s['tick']) + struct.pack('<HI', e['phase'], e['ordinal'])
  frames.append(dict(preimage_hex=ef.hex(), full_digest=H(ef)))
  public.append(dict(id=H(ef)[:32], **e))
 return dict(tick=s['tick'], applied_revisions=payload['applied_revisions'], events=public, state_digest=payload['state_digest'], digest=digest), dict(payload=payload, preimage_hex=frame.hex(), event_frames=frames)


def descriptor(b, r, s, q):
 return dict(domain=3, catalog_hash=PACK, branch=b, revision=r, tick=s['tick'], state_digest=state_digest(s), receipt_digest=q['digest'], event=None)


def publication(b, r, s, q, f):
 return dict(view=view(b, r, s['tick']), state=copy.deepcopy(s), state_preimage_hex=state_frame(s).hex(), receipt=q, receipt_frame=f)


def replay(a, g, allow_parent=False):
 check_seal(a, 'ARCHIVE')
 need(a['catalog_hash'] == PACK, '/catalog_hash', 8, 'catalog_mismatch')
 if a['genesis'] != g:
  raise Unsupported('Other Genesis source family')
 if a['branch_sequence_high_water'] != '0':
  raise Unsupported('Unexamined reserved-fork mark family')
 if len(a['branches']) != 1 or len(a['revisions']) not in ((2,4) if allow_parent else (4,)):
  raise Unsupported('Only root two paused plus two running revisions')
 root = H(root_frame(g))[:32]
 b = a['branches'][0]
 need(b['id'] == root and b['parent'] is None and b['fork_revision'] is None and b['fork_tick'] == b['ordinal'] == '0' and b['fork_state_digest'] == state_digest(g['state']))
 rows = sorted(a['revisions'], key=lambda x:int(x['ordinal']))
 if [r['ordinal'] for r in rows[:2]] != ['1','2'] or any(r['mode'] != 'paused' for r in rows[:2]):
  raise Unsupported('Unexamined parent ordinal/mode family')
 if len(rows) == 4 and any(r['mode'] != 'running' or r['command']['operations'][0]['type'] != 'edit_system' for r in rows[2:]):
  raise Unsupported('Only two running System edits')
 s = copy.deepcopy(g['state'])
 views = {vk(view(root,None,0)): (copy.deepcopy(s), None)}
 pubs, ops = [], []
 parent = None
 for r in rows[:2]:
  s, audit = execute(s,r,root,parent,'paused')
  q, f = receipt(s,[r['id']],[(r['id'], audit['change'])])
  pub = publication(root,r['id'],s,q,f)
  pub['operation_audit'] = audit
  pubs.append(pub); views[vk(pub['view'])] = (copy.deepcopy(s),q); parent=r['id']
 s['tick'] = '1'
 q,f = receipt(s,[],[])
 pub = publication(root,parent,s,q,f)
 pubs.append(pub); views[vk(pub['view'])] = (copy.deepcopy(s),q)
 committed = copy.deepcopy(s)
 if len(rows) == 4:
  if b['tip_tick'] != '2':
   raise Unsupported('Other ordinary tip chronology')
  s['tick'] = inc(s['tick'])
  need(int(rows[2]['ordinal']) > int(rows[1]['ordinal']) and int(rows[3]['ordinal']) > int(rows[2]['ordinal']))
  for r in rows[2:]:
   s,audit = execute(s,r,root,parent,'running'); ops.append(audit); parent=r['id']
  q,f = receipt(s,[r['id'] for r in rows[2:]],[(r['id'],x['change']) for r,x in zip(rows[2:],ops)])
  pub = publication(root,parent,s,q,f)
  pubs.append(pub); views[vk(pub['view'])] = (copy.deepcopy(s),q)
 else:
  need(b['tip_tick'] == '1')
 need(b['head'] == parent)
 need(int(a['accepted_sequence_high_water']) >= max(int(r['ordinal']) for r in rows), '/accepted_sequence_high_water',8,'allocator_regression')
 need(a['branch_sequence_high_water'] == '0')
 if len(rows)==4 and a['active_view']==view(root,rows[2]['id'],2):
  raise Unsupported('Intermediate running selected-view matrix unqualified')
 need(vk(a['active_view']) in views, '/active_view',8,'invalid_history_view')
 need(a['final_tick'] == a['active_view']['tick'] and a['final_digest'] == state_digest(views[vk(a['active_view'])][0]))
 for an in a['anchors']:
  need(vk(an) in views, '/anchors',8,'invalid_history_view')
  st,q = views[vk(an)]
  need(q is not None and an == descriptor(root,an['revision'],st,q), '/anchors',10,'forged_evidence')
 return dict(root=root, views=views, publications=pubs, running_operations=ops, committed=committed, unexamined_intermediate_view=None if len(rows)==2 else view(root,rows[2]['id'],2))


def notebook(n,a,g,rv):
 check_seal(n,'NOTEBOOK')
 need(n['catalog_hash'] == PACK and n['genesis_digest'] == H(D('GENESIS')+C(g)) and n['archive_integrity_sha256'] == a['integrity_sha256'])
 if n['experiments'] or n['evidence']:
  raise Unsupported('Experiments/Evidence not examined')
 need(int(n['next_ordinal']) > 0)
 ords = [int(b['ordinal']) for b in n['bookmarks']]
 need(len(set(ords)) == len(ords) and all(0 < x < int(n['next_ordinal']) for x in ords))
 for b in n['bookmarks']:
  if b['view']==rv['unexamined_intermediate_view']:
   raise Unsupported('Intermediate running Bookmark matrix unqualified')
  need(vk(b['view']) in rv['views'], '/bookmarks',8,'invalid_history_view')
  need(b['digest'] == state_digest(rv['views'][vk(b['view'])][0]), '/bookmarks',10,'forged_evidence')


# Declared schema-derived shapes. Unexamined enum/array payloads are coverage-only.
def record(**fields):
 return ('record',fields)


def option(s):
 return ('option',s)


def array(s,cap,key=None):
 return ('array',s,cap,key)


SYSTEM=record(id='id',name='name')
VIEW=record(branch='id',revision=option('digest'),tick='u64')
SNAP=record(domain='u32',catalog_hash='digest',branch='id',revision=option('digest'),tick='u64',state_digest='digest',phase='u16',permitted_snapshot_digest='digest')
KEY=('tag',{'systems':record(type='text',id='id')})
ROW=('tag',{'systems':record(type='text',value=SYSTEM)})
TARGET=('tag',{'existing':record(type='text',id='id'),'local':record(type='text',local='u16')})
OP=('tag',{'create_system':record(type='text',local='u16',name='name'),'edit_system':record(type='text',target=TARGET,name='name')})
CHANGE=record(operation_index=option('u32'),collection='text',key=KEY,before=option(ROW),after=option(ROW),resources=array('outside',4),passenger_delta='score')
CMD=('tag',{'creator_batch':record(type='text',operations=array(OP,1024),reviewed_effects=array(CHANGE,None))})
REV=record(id='digest',branch='id',parent=option('digest'),tick='u64',ordinal='u64',mode=('enum',['paused','running']),capture=SNAP,command=CMD)
STATE=record(**{k:'u64' if k in FIELDS[:3] else record() if k=='counters' else array(SYSTEM if k=='systems' else 'outside',64 if k=='systems' else None,'id' if k=='systems' else None) for k in FIELDS})
GEN=record(kind='text',format_version='u32',rules_version='u32',catalog_hash='digest',seed='digest',generator=record(generator_id='slug',generator_version='u32'),allocations=array('outside',None),state=STATE)
BR=record(id='id',parent=option('id'),fork_revision=option('digest'),fork_tick='u64',fork_state_digest='digest',ordinal='u64',head=option('digest'),tip_tick='u64')
AN=record(domain='u32',catalog_hash='digest',branch='id',revision=option('digest'),tick='u64',state_digest='digest',receipt_digest='digest',event=option('outside'))
ARC=record(kind='text',format_version='u32',rules_version='u32',catalog_hash='digest',genesis=GEN,revisions=array(REV,10000,'id'),branches=array(BR,128,'id'),active_view=VIEW,final_tick='u64',final_digest='digest',accepted_sequence_high_water='u64',branch_sequence_high_water='u64',anchors=array(AN,16384,'anchor'),integrity_sha256='digest')
BM=record(ordinal='u64',label='name',view=VIEW,digest='digest')
NB=record(kind='text',format_version='u32',rules_version='u32',catalog_hash='digest',genesis_digest='digest',archive_integrity_sha256='digest',next_ordinal='u64',bookmarks=array(BM,1024,'ordinal'),experiments=array('outside',256),evidence=array('outside',4096),integrity_sha256='digest')
TR=record(kind='text',format_version='u32',rules_version='u32',catalog= 'catalog',archive=ARC,notebook=NB,selected_view=VIEW,integrity_sha256='digest')
SHAPES=dict(archive=ARC,notebook=NB,transfer=TR)
CAPS=dict(archive=64*1024*1024,notebook=8*1024*1024,transfer=96*1024*1024)


def shape(x,sc,cat,path=''):
 if isinstance(sc,str):
  if sc=='outside':raise Unsupported(path)
  if sc=='catalog':
   if C(x)!=C(cat):raise Unsupported('Other Catalog profile')
   return x
  if sc in ['u16','u32']:
   need(type(x) is int and 0<=x<2**int(sc[1:]),path,5,'invalid_range')
  else:
   need(isinstance(x,str),path,5,'invalid_range')
   if sc in ['u64','score']:
    need(re.fullmatch('0|[1-9][0-9]*' if sc=='u64' else '0|-?[1-9][0-9]*',x) is not None,path,5,'noncanonical_bytes')
    need(len(x)<=20 and (int(x)<2**64 if sc=='u64' else -(2**63)<=int(x)<2**63),path,5,'invalid_range')
   elif sc in ['id','digest']:
    need(re.fullmatch('[0-9a-f]{'+('32' if sc=='id' else '64')+'}',x) is not None,path,5,'noncanonical_bytes')
   elif sc=='slug':need(re.fullmatch('[a-z][a-z0-9_]{0,63}',x) is not None,path,5,'invalid_range')
   elif sc=='name':need(1<=len(x)<=64 and len(x.encode())<=256 and not x.startswith(' ') and not x.endswith(' ') and '  ' not in x and not any(ord(c)<32 or 127<=ord(c)<=159 for c in x),path,5,'invalid_range')
  return x
 tag=sc[0]
 if tag=='option':return None if x is None else shape(x,sc[1],cat,path)
 if tag=='enum':
  need(isinstance(x,str) and x in sc[1],path,5,'invalid_enum');return x
 if tag=='tag':
  need(isinstance(x,dict) and 'type' in x,ptr(path,'type'),5,'noncanonical_bytes')
  need(isinstance(x['type'],str),ptr(path,'type'),5,'invalid_range')
  if x['type'] not in sc[1]:raise Unsupported('Other tag '+path)
  return shape(x,sc[1][x['type']],cat,path)
 if tag=='record':
  need(isinstance(x,dict),path,5,'invalid_range')
  extra=set(x)-set(sc[1]);need(not extra,ptr(path,min(extra,key=lambda k:k.encode())) if extra else path,5,'unknown_field')
  out={}
  for k,v in sc[1].items():
   need(k in x,ptr(path,k),5,'noncanonical_bytes');out[k]=shape(x[k],v,cat,ptr(path,k))
  return out
 need(isinstance(x,list),path,5,'invalid_range')
 if sc[2] is not None:need(len(x)<=sc[2],path,5,'record_capacity')
 out=[shape(v,sc[1],cat,ptr(path,i)) for i,v in enumerate(x)]
 if sc[3]:
  keys=[anchor_key(v) if sc[3]=='anchor' else int(v[sc[3]]) if sc[3]=='ordinal' else v[sc[3]] for v in out]
  for i in range(1,len(keys)):
   need(keys[i]>=keys[i-1],ptr(path,i),5,'noncanonical_bytes')
   if keys[i]==keys[i-1]:
    need(False,ptr(path,i),7 if path.endswith('/systems') else 5,'duplicate_identity' if path.endswith('/systems') else 'duplicate_key')
 return out


def header(d,f,path=''):
 for k,expected in [('kind','NYON_LIVING_'+f.upper()+'_V3'),('format_version',3),('rules_version',3)]:
  need(k in d,ptr(path,k),4,'noncanonical_bytes')
  need(isinstance(d[k],str) if k=='kind' else type(d[k]) is int and 0<=d[k]<2**32,ptr(path,k),4,'invalid_range')
  need(d[k]==expected,ptr(path,k),4,'unsupported_kind' if k=='kind' else 'unsupported_'+k)


class Pairs(list):
 pass


def decode(raw,cap):
 need(len(raw)<=cap,'',0,'byte_capacity')
 try:
  value=json.loads(raw.decode('utf8'),object_pairs_hook=Pairs)
 except (ValueError,UnicodeError,RecursionError):
  raise Rejected(2,'noncanonical_bytes','') from None
 def materialize(x,path='',depth=1):
  need(depth<=64,path,2,'depth_capacity')
  if isinstance(x,Pairs):
   out={}
   for k,v in x:
    need(k not in out,ptr(path,k),3,'duplicate_key');out[k]=materialize(v,ptr(path,k),depth+1 if isinstance(v,list) else depth)
   return out
  if isinstance(x,list):return [materialize(v,ptr(path,i),depth+1 if isinstance(v,list) else depth) for i,v in enumerate(x)]
  return x
 return materialize(value)


def admit(raw,f,cat,g,attachment=None):
 d=decode(raw,CAPS[f]);need(isinstance(d,dict),'',4,'invalid_range');header(d,f)
 ordered=shape(d,SHAPES[f],cat);need(C(ordered)==raw,'',6,'noncanonical_bytes')
 if f=='archive':
  header(d['genesis'],'genesis','/genesis');return replay(d,g)
 if f=='notebook':
  need(attachment is not None,'/archive_integrity_sha256',8,'missing_reference')
  rv=replay(attachment,g);notebook(d,attachment,g,rv);return rv
 header(d['archive'],'archive','/archive');header(d['notebook'],'notebook','/notebook');header(d['archive']['genesis'],'genesis','/archive/genesis')
 check_seal(d,'TRANSFER');rv=replay(d['archive'],g);notebook(d['notebook'],d['archive'],g,rv)
 if d['selected_view']==rv['unexamined_intermediate_view']:
  raise Unsupported('Intermediate running Transfer matrix unqualified')
 need(vk(d['selected_view']) in rv['views'],'/selected_view',8,'invalid_history_view');return rv


def outcome(raw,f,cat,g,attachment=None):
 try:admit(raw,f,cat,g,attachment);return dict(admitted=True)
 except Rejected as e:return dict(admitted=False,stage=e.stage,error=e.error,path=e.path)
 except Unsupported as e:return dict(coverage='unsupported_family',detail=str(e))


def edit_revision(root,parent,s,ordinal,name):
 op=dict(type='edit_system',target=dict(type='existing',id=s['systems'][0]['id']),name=name)
 new=dict(id=s['systems'][0]['id'],name=name)
 cmd=dict(type='creator_batch',operations=[op],reviewed_effects=[change(s['systems'][0],new)])
 r=dict(id='',branch=root,parent=parent,tick=s['tick'],ordinal=ordinal,mode='running',capture=snapshot(s,root,parent),command=cmd)
 r['id']=H(revision_frame(root,r));return r


def build(cat,g,pa,pn,names=('Delta','Epsilon'),label='Running Epsilon'):
 parent_replay=replay(pa,g,allow_parent=True);root=parent_replay['root'];committed=copy.deepcopy(parent_replay['committed']);s=copy.deepcopy(committed);s['tick']=inc(s['tick'])
 rows=[];audits=[];parent=pa['branches'][0]['head'];mark=pa['accepted_sequence_high_water']
 submissions=[]
 for name in names:
  ordinal=inc(mark);r=edit_revision(root,parent,s,ordinal,name)
  envelope=dict(domain=3,catalog_hash=PACK,branch=root,expected_committed_revision=pa['branches'][0]['head'],expected_tick=committed['tick'],expected_state_digest=state_digest(committed),expected_pending_sequence='0' if not rows else rows[-1]['ordinal'],mode='running',command=copy.deepcopy(r['command']))
  accepted=dict(accepted_sequence=ordinal,application_tick=s['tick'],envelope=envelope)
  before=copy.deepcopy(s);s,audit=execute(s,r,root,parent,'running')
  submissions.append(dict(accepted=accepted,revision=copy.deepcopy(r),pre_operation_state=before,state_preimage_hex=state_frame(before).hex(),published_view=view(root,pa['branches'][0]['head'],committed['tick']),published_state_digest=state_digest(committed),accepted_high_water=ordinal,pending_tail=ordinal,boundary_receipt=None))
  rows.append(r);audits.append(audit);parent=r['id'];mark=ordinal
 q,f=receipt(s,[r['id'] for r in rows],[(r['id'],a['change']) for r,a in zip(rows,audits)])
 pub=publication(root,parent,s,q,f)
 a=copy.deepcopy(pa);a['revisions'].extend(rows);a['revisions'].sort(key=lambda r:r['id']);a['branches'][0]['head']=parent;a['branches'][0]['tip_tick']=s['tick'];a['active_view']=copy.deepcopy(pub['view']);a['final_tick']=s['tick'];a['final_digest']=state_digest(s);a['accepted_sequence_high_water']=mark;a['anchors'].append(descriptor(root,parent,s,q));a['anchors'].sort(key=anchor_key);seal(a,'ARCHIVE')
 n=copy.deepcopy(pn);n['archive_integrity_sha256']=a['integrity_sha256'];ordinal=n['next_ordinal'];n['bookmarks'].append(dict(ordinal=ordinal,label=label,view=copy.deepcopy(pub['view']),digest=state_digest(s)));n['next_ordinal']=inc(ordinal);seal(n,'NOTEBOOK')
 t=seal(dict(kind='NYON_LIVING_TRANSFER_V3',format_version=3,rules_version=3,catalog=copy.deepcopy(cat),archive=a,notebook=n,selected_view=copy.deepcopy(pub['view'])),'TRANSFER')
 trace=dict(status='Proposed RUN-HIST private audit, not portable fields',root_preimage_hex=root_frame(g).hex(),parent_publications=parent_replay['publications'],published_before=dict(view=view(root,pa['branches'][0]['head'],committed['tick']),state=committed,state_preimage_hex=state_frame(committed).hex()),submissions=submissions,running_operations=audits,boundary_publication=pub,publication_cursor=dict(branch=root,revision=parent,tick=s['tick'],state_digest=state_digest(s),pending_tail='0',accepted_high_water=mark,branch_high_water='0'),archive_preimage_hex=(D('ARCHIVE')+C({k:v for k,v in a.items() if k!='integrity_sha256'})).hex(),notebook_preimage_hex=(D('NOTEBOOK')+C({k:v for k,v in n.items() if k!='integrity_sha256'})).hex(),transfer_preimage_hex=(D('TRANSFER')+C({k:v for k,v in t.items() if k!='integrity_sha256'})).hex())
 return a,n,t,trace


def setpath(d,path,value=None,delete=False):
 d=json.loads(C(d));parts=path.split('/')[1:];x=d  # break all audit aliases before one-field mutation
 for k in parts[:-1]:x=x[int(k)] if isinstance(x,list) else x[k]
 k=parts[-1]
 if delete:del x[k]
 elif isinstance(x,list):x[int(k)]=value
 else:x[k]=value
 return d


def relink(x,old,new):
 if isinstance(x,dict):return {k:relink(v,old,new) for k,v in x.items()}
 if isinstance(x,list):return [relink(v,old,new) for v in x]
 return new if x==old else x


def reframe(a):
 """Reframe only submitted ID operands/links/seal; do not repair captures/facts."""
 a=copy.deepcopy(a);root=a['branches'][0]['id']
 for ordinal in sorted(int(r['ordinal']) for r in a['revisions']):
  # Mutations may reuse an ordinal; input-index order deliberately preserved.
  indices=[i for i,r in enumerate(a['revisions']) if int(r['ordinal'])==ordinal]
  for i in indices:
   r=a['revisions'][i];old=r['id'];a=relink(a,old,H(revision_frame(root,r)))
 a['revisions'].sort(key=lambda r:r['id']);a['anchors'].sort(key=anchor_key);return seal(a,'ARCHIVE')


def trace_matches(trace,a,g):
 try:
  rv=replay(a,g)
  need(trace['parent_publications']==rv['publications'][:3]);need(trace['running_operations']==rv['running_operations']);need(trace['boundary_publication']==rv['publications'][3]);s=copy.deepcopy(rv['committed']);root=rv['root'];parent=rv['publications'][2]['view']['revision']
  rows=sorted(a['revisions'],key=lambda r:int(r['ordinal']))[2:]
  need(trace['published_before']==dict(view=view(root,parent,s['tick']),state=s,state_preimage_hex=state_frame(s).hex()))
  s['tick']=inc(s['tick'])
  for i,(r,sub) in enumerate(zip(rows,trace['submissions'])):
   envelope=dict(domain=3,catalog_hash=PACK,branch=root,expected_committed_revision=rv['publications'][2]['view']['revision'],expected_tick='1',expected_state_digest=state_digest(rv['committed']),expected_pending_sequence='0' if i==0 else rows[i-1]['ordinal'],mode='running',command=r['command'])
   need(sub==dict(accepted=dict(accepted_sequence=r['ordinal'],application_tick='2',envelope=envelope),revision=r,pre_operation_state=s,state_preimage_hex=state_frame(s).hex(),published_view=rv['publications'][2]['view'],published_state_digest=state_digest(rv['committed']),accepted_high_water=r['ordinal'],pending_tail=r['ordinal'],boundary_receipt=None))
   s,_=execute(s,r,root,parent,'running');parent=r['id']
  need(len(trace['submissions'])==2)
  need(trace['publication_cursor']==dict(branch=root,revision=parent,tick='2',state_digest=state_digest(s),pending_tail='0',accepted_high_water=a['accepted_sequence_high_water'],branch_high_water='0'))
  return True
 except (Rejected,KeyError):return False


def products(cat,g,pa,pn,source_manifest,hist_manifest):
 a,n,t,tr=build(cat,g,pa,pn);cases=[]
 def add(identifier,f,doc,expectation,strength,attachment=None,raw=None):
  raw=C(doc) if raw is None else raw;actual=outcome(raw,f,cat,g,attachment)
  if strength=='rejection_only':need(actual.get('admitted') is False,identifier)
  else:need(all(actual.get(k)==v for k,v in expectation.items()),identifier)
  cases.append(dict(id=identifier,entry_point=f,raw_base64=base64.b64encode(raw).decode(),expectation=expectation,strength=strength,attachment_archive_base64=None if attachment is None else base64.b64encode(C(attachment)).decode()))
 for title,(aa,nn,tt,_) in [('base',(a,n,t,tr)),('unicode',build(cat,g,pa,pn,('Delta Ω','Epsilon β'),'Running β')),('id_order',build(cat,g,pa,pn,('Delta','Omega'),'Running Omega'))]:
  for f,d in [('archive',aa),('notebook',nn),('transfer',tt)]:add('positive.'+title+'.'+f,f,d,dict(admitted=True),'complete_running_history_admission',aa if f=='notebook' else None)
 omega=build(cat,g,pa,pn,('Delta','Omega'),'Running Omega')
 need([r['ordinal'] for r in omega[0]['revisions'] if r['mode']=='running']==['4','3'])
 need([e['provenance']['revision'] for e in omega[3]['boundary_publication']['receipt']['events']]==[r['id'] for r in sorted(omega[0]['revisions'],key=lambda r:int(r['ordinal']))[2:]])
 rv=replay(a,g);rows=sorted(a['revisions'],key=lambda r:int(r['ordinal']));root=rv['root']
 for title,v in [('root_create',view(root,rows[0]['id'],0)),('root_edit',view(root,rows[1]['id'],0)),('root_ordinary',view(root,rows[1]['id'],1))]:
  alt=copy.deepcopy(t);alt['selected_view']=v;seal(alt,'TRANSFER');add('positive.transfer.'+title,'transfer',alt,dict(admitted=True),'complete_running_history_admission')
 holes=copy.deepcopy(n);holes['next_ordinal']='9';seal(holes,'NOTEBOOK');add('positive.notebook.mark_holes','notebook',holes,dict(admitted=True),'complete_running_history_admission',a)
 oldactive=copy.deepcopy(a);oldactive['active_view']=view(root,rows[1]['id'],1);oldactive['final_tick']='1';oldactive['final_digest']=state_digest(rv['committed']);seal(oldactive,'ARCHIVE');add('positive.archive.prior_active','archive',oldactive,dict(admitted=True),'complete_running_history_admission')
 for f,d in [('archive',a),('notebook',n),('transfer',t)]:
  attachment=a if f=='notebook' else None
  def walk(x,path=''):
   if isinstance(x,dict):
    for k,v in x.items():
     p=ptr(path,k)
     if p.startswith('/catalog/'):continue
     add('missing.'+f+p.replace('/','.'),f,setpath(d,p,delete=True),dict(admitted=False,stage=4 if p in ['/kind','/format_version','/rules_version'] else 5,error='noncanonical_bytes',path=p),'exact_stage_error_path',attachment);walk(v,p)
   elif isinstance(x,list):
    for i,v in enumerate(x):walk(v,ptr(path,i))
  walk(d)
  for k,val,err in [('kind','OTHER','unsupported_kind'),('format_version',4,'unsupported_format_version'),('rules_version',4,'unsupported_rules_version')]:add('header.'+f+'.'+k,f,setpath(d,'/'+k,val),dict(admitted=False,stage=4,error=err,path='/'+k),'exact_stage_error_path',attachment)
  add('integrity.'+f,f,setpath(d,'/integrity_sha256','ff'*32),dict(admitted=False,stage=9,error='integrity_mismatch',path='/integrity_sha256'),'exact_stage_error_path',attachment)
  add('canonical.'+f,f,d,dict(admitted=False,stage=6,error='noncanonical_bytes'),'exact_stage_error_only',attachment,b' '+C(d))
 ri={r['ordinal']:a['revisions'].index(r) for r in rows};p3='/revisions/'+str(ri['3']);p4='/revisions/'+str(ri['4'])
 muts=[('first_capture_committed_digest',p3+'/capture/state_digest',state_digest(rv['committed'])),('first_permitted_committed_digest',p3+'/capture/permitted_snapshot_digest',state_digest(rv['committed'])),('first_capture_tick',p3+'/capture/tick','1'),('first_revision_tick',p3+'/tick','1'),('first_capture_phase',p3+'/capture/phase',7),('second_parent_committed',p4+'/parent',rows[1]['id']),('second_capture_committed_parent',p4+'/capture/revision',rows[1]['id']),('second_capture_first_digest',p4+'/capture/state_digest',rows[2]['capture']['state_digest']),('second_permitted_first_digest',p4+'/capture/permitted_snapshot_digest',rows[2]['capture']['permitted_snapshot_digest']),('second_before_old_name',p4+'/command/reviewed_effects/0/before/value/name','Beta'),('second_operation_index',p4+'/command/reviewed_effects/0/operation_index',1),('second_fake_target',p4+'/command/operations/0/target/id','ff'*16),('first_parent_null',p3+'/parent',None),('second_parent_null',p4+'/parent',None),('final_prefix_digest','/final_digest',rows[3]['capture']['state_digest']),('accepted_regression','/accepted_sequence_high_water','3'),('head_first_running','/branches/0/head',rows[2]['id'])]
 for title,p,val in muts:add('replay.'+title,'archive',reframe(setpath(a,p,val)),dict(admitted=False),'rejection_only')
 # Full final State cannot substitute for an earlier original capture.
 final=tr['boundary_publication']['state'];wrong=copy.deepcopy(a)
 wrong['revisions'][ri['3']]['capture']['state_digest']=state_digest(final);wrong['revisions'][ri['3']]['capture']['permitted_snapshot_digest']=state_digest(final)
 add('replay.first_capture_final_state','archive',reframe(wrong),dict(admitted=False),'rejection_only')
 for field in ['revisions','anchors']:
  alt=copy.deepcopy(a);alt[field].reverse();seal(alt,'ARCHIVE');add('sorted.'+field,'archive',alt,dict(admitted=False,stage=5,error='noncanonical_bytes',path='/'+field+'/1'),'exact_stage_error_path')
 for title,p,val in [('stale_attachment','/archive_integrity_sha256',pa['integrity_sha256']),('final_prefix_digest','/bookmarks/1/digest',rows[3]['capture']['state_digest']),('unreachable_future','/bookmarks/1/view/tick','3'),('zero_mark','/next_ordinal','0')]:
  alt=seal(setpath(n,p,val),'NOTEBOOK');add('notebook.'+title,'notebook',alt,dict(admitted=False),'rejection_only',a)
 for title,p,val in [('future_view','/selected_view/tick','3'),('archive_kind','/archive/kind','OTHER'),('notebook_rules','/notebook/rules_version',4),('genesis_kind','/archive/genesis/kind','OTHER')]:
  alt=setpath(t,p,val);seal(alt['archive'],'ARCHIVE');alt['notebook']['archive_integrity_sha256']=alt['archive']['integrity_sha256'];seal(alt['notebook'],'NOTEBOOK');seal(alt,'TRANSFER');add('transfer.'+title,'transfer',alt,dict(admitted=False),'rejection_only')
 need(trace_matches(tr,a,g));trace_cases=[]
 mutations=[('/submissions/0/accepted/envelope/expected_tick','2'),('/submissions/1/accepted/envelope/expected_committed_revision',rows[2]['id']),('/submissions/1/accepted/envelope/expected_pending_sequence','0'),('/submissions/0/boundary_receipt',tr['boundary_publication']['receipt']),('/submissions/1/pre_operation_state/next_sequence','2'),('/running_operations/1/result_state/next_sequence','3'),('/boundary_publication/receipt/applied_revisions',list(reversed(tr['boundary_publication']['receipt']['applied_revisions']))),('/boundary_publication/receipt/events/0/ordinal',1),('/boundary_publication/receipt/events/1/provenance/revision',rows[2]['id']),('/boundary_publication/receipt/digest','ff'*32),('/boundary_publication/receipt/events/1/id','ff'*16),('/publication_cursor/pending_tail','4')]
 for p,val in mutations:
  alt=setpath(tr,p,val);need(not trace_matches(alt,a,g));trace_cases.append(dict(id=p,mutated_trace=alt,expectation=dict(exact_replay_matches=False),strength='trace_binding_rejection_only'))
 declarations={}
 def decl(sc,path):
  if isinstance(sc,str):declarations[path]=dict(primitive=sc)
  elif sc[0]=='record':
   declarations[path]=dict(required_fields=list(sc[1]))
   for k,v in sc[1].items():decl(v,ptr(path,k))
  elif sc[0]=='option':declarations[path]=dict(required_option=True);decl(sc[1],path+'?')
  elif sc[0]=='array':declarations[path]=dict(cap=sc[2],key=sc[3]);decl(sc[1],path+'/*')
  elif sc[0]=='tag':
   declarations[path]=dict(covered_tags=list(sc[1]),other_tags_unexamined=True)
   for k,v in sc[1].items():decl(v,path+':'+k)
  else:declarations[path]=dict(enum=sc[1])
 for f,sc in SHAPES.items():decl(sc,f)
 scope={}
 prefix_active=setpath(a,'/active_view',view(root,rows[2]['id'],2))
 prefix_active['final_digest']=state_digest(tr['running_operations'][0]['result_state']);seal(prefix_active,'ARCHIVE')
 mixed=setpath(a,p4+'/mode','paused');mixed=reframe(mixed)
 for title,doc in [('accepted_parent_only',pa),('later_tip3',seal(setpath(a,'/branches/0/tip_tick','3'),'ARCHIVE')),('intermediate_running_view',prefix_active),('other_mode_family',mixed),('reserved_fork_mark',seal(setpath(a,'/branch_sequence_high_water','1'),'ARCHIVE'))]:
  out=outcome(C(doc),'archive',cat,g);need(out.get('coverage')=='unsupported_family');scope[title]=dict(coverage='unsupported_family',wire_error_claim=False)
 ledger=dict(status='Proposed RUN-HIST selected family; independent review pending',pins=PINS,accepted_bytes=INPUTS,source_manifest=source_manifest,accepted_hist_manifest=hist_manifest,declarations=declarations,strengths=dict(sorted(Counter(x['strength'] for x in cases).items())),trace_binding_count=len(trace_cases),scope_classification_checks=scope,predicates=dict(parent_history='Replay original paused S1/S2 and ordinary1, preserve original Archive rows/Anchors/Bookmark.',running_sources='Working T2 set before phase1; second capture includes S3 result and historical header; committed envelopes remain rootR2/t1.',sequence_and_chronology='DocumentS3/S4, logical R2->R3->R4, only complete final tick2 publication.',creator_facts='One exact systems Change/intervention each; ordinals0/1; no header/semantic/cascade duplication.',receipt='Seal full final ID-free payload, sort applied IDs separately from event application order, then derive Event IDs.',archive='Retained parent/tip ancestry and highwater plus exact final or prior selected State; full declaration/key ordering.',notebook_transfer='Exact reattachment, State Bookmarks/common mark/holes, selected reachable view independent active.',trace='Private acceptance/tail/working prefix audits only; no portable queue or early boundary receipt.'),gaps=['Only empty root/two paused parent revisions/ordinary1/two running System edits/ordinary2; other shapes coverage-only.', 'No abandoned-tail, child ordinary, Undo/redo, partial queue clearing or lifecycle race qualification.', 'No general Branch/entity collision scanner, parser/depth/duplicate offset or stage7-10 multifault priority qualification.', 'Author helper recursively materializes/buffers; no bounded production poll/memory/cancel/durability proof.', 'No receipt bytes or queue in portable Archive; intermediate running selected-view matrix unqualified.', 'No maximum-capacity negatives or inhabited phase2/phase5/Actor/forecast/gameplay completion.'])
 need(len({x['id'] for x in cases})==len(cases))
 return {'archive.json':C(a),'notebook.json':C(n),'transfer.json':C(t),'trace.json':json.dumps(tr,ensure_ascii=False,indent=2).encode()+b'\n','vectors.json':json.dumps(dict(status='Proposed RUN-HIST',cases=cases,trace_binding_cases=trace_cases),ensure_ascii=False,indent=2).encode()+b'\n','predicate-coverage.json':json.dumps(ledger,ensure_ascii=False,indent=2).encode()+b'\n'}


def main():
 parser=argparse.ArgumentParser();parser.add_argument('mode',choices=['generate','verify']);mode=parser.parse_args().mode
 sm,hm=freshness();cat=json.loads((CAT/'core.catalog.json').read_bytes());g=json.loads((CAT/'genesis/empty.genesis.json').read_bytes());need(H(D('PACK')+C(cat))==PACK)
 pa=json.loads((HERE.parent/'archive.json').read_bytes());pn=json.loads((HERE.parent/'notebook.json').read_bytes());rv=replay(pa,g,allow_parent=True);notebook(pn,pa,g,rv)
 output=products(cat,g,pa,pn,sm,hm);freshness()
 for name,raw in output.items():
  p=HERE/name
  if mode=='generate':p.write_bytes(raw)
  elif p.read_bytes()!=raw:raise SystemExit('STOP output differs '+name)
  print(name,len(raw),H(raw))
 vectors=json.loads(output['vectors.json']);print(mode+' passed: '+str(len(vectors['cases']))+' raw; '+str(len(vectors['trace_binding_cases']))+' trace cases')


if __name__=='__main__':main()
