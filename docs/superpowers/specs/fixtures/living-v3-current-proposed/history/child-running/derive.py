#!/usr/bin/env python3
"""Proposed CHILD-RUN exact selected family, stdlib formulas, no imported oracle."""
import argparse, base64, copy, hashlib, json
from pathlib import Path
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[6]
SDD=ROOT/'.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
CAT=HERE.parent.parent
PARENT=HERE.parent/'fork'
PACK='9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'
C=lambda x:json.dumps(x,ensure_ascii=False,separators=(',',':')).encode()
H=lambda x:hashlib.sha256(x).hexdigest()
D=lambda n:('NYON_LIVING_V3_'+n).encode()+b'\0'
Q=lambda n:int(n).to_bytes(8,'little')
L=lambda x:len(x).to_bytes(4,'little')+x
O=lambda x:b'\0' if x is None else b'\1'+bytes.fromhex(x)
cp=copy.deepcopy

def seal(x,n):
 x['integrity_sha256']=H(D(n)+C({k:v for k,v in x.items() if k!='integrity_sha256'}));return x

def sf(s):return D('STATE')+C(dict(format_version=3,rules_version=3,catalog_hash=PACK,state=s))
def sd(s):return H(sf(s))
def view(b,r,t):return dict(branch=b,revision=r,tick=str(t))
def key(v):return v['branch'],v['revision'] or '',int(v['tick'])
def ak(a):return (*key(a),a['receipt_digest'],(-1,-1,'') if a['event'] is None else (a['event']['phase'],a['event']['ordinal'],a['event']['id']))
def snap(s,b,r):return dict(domain=3,catalog_hash=PACK,branch=b,revision=r,tick=s['tick'],state_digest=sd(s),phase=1,permitted_snapshot_digest=sd(s))
def rf(root,r):return D('REVISION')+bytes.fromhex(root)+bytes.fromhex(r['branch'])+O(r['parent'])+Q(r['tick'])+Q(r['ordinal'])+bytes([0 if r['mode']=='paused' else 1])+L(C(r['capture']))+L(C(r['command']))
def receipt(s,changes):
 e=[dict(phase=1,ordinal=i,provenance=dict(type='creator',revision=r),kind='creator_intervention',facts=dict(type='change',changes=[ch])) for i,(r,ch) in enumerate(changes)]
 p=dict(tick=s['tick'],applied_revisions=sorted(r for r,_ in changes),events=e,state_digest=sd(s));f=D('RECEIPT')+C(p);h=H(f);frames=[];out=[]
 for x in e:
  ef=D('EVENT')+bytes.fromhex(h)+Q(s['tick'])+x['phase'].to_bytes(2,'little')+x['ordinal'].to_bytes(4,'little');frames.append(dict(preimage_hex=ef.hex(),full_digest=H(ef)));out.append(dict(id=H(ef)[:32],**x))
 return dict(tick=s['tick'],applied_revisions=p['applied_revisions'],events=out,state_digest=sd(s),digest=h),dict(payload=p,preimage_hex=f.hex(),event_frames=frames)

def anchor(b,r,s,q):return dict(domain=3,catalog_hash=PACK,branch=b,revision=r,tick=s['tick'],state_digest=sd(s),receipt_digest=q['digest'],event=None)
def publication(b,r,s,changes):
 q,f=receipt(s,changes);return dict(view=view(b,r,s['tick']),state=cp(s),state_preimage_hex=sf(s).hex(),receipt=q,receipt_frame=f)

def change(old,new):return dict(operation_index=0,collection='systems',key=dict(type='systems',id=new['id']),before=None if old is None else dict(type='systems',value=cp(old)),after=dict(type='systems',value=cp(new)),resources=[],passenger_delta='0')

def apply(root,b,parent,s,ordinal,name,mode,create=False):
 before=cp(s);old=None if create else cp(s['systems'][0]);issuance=None
 if create:
  issuance=D('CREATOR_ENTITY')+bytes.fromhex(root)+bytes.fromhex(b)+b'\2'+Q(ordinal)+(0).to_bytes(4,'little')+(1).to_bytes(2,'little')+(1).to_bytes(2,'little')
  new=dict(id=H(issuance)[:32],name=name);op=dict(type='create_system',local=1,name=name)
 else:new=dict(id=old['id'],name=name);op=dict(type='edit_system',target=dict(type='existing',id=old['id']),name=name)
 ch=change(old,new);command=dict(type='creator_batch',operations=[op],reviewed_effects=[ch]);r=dict(id='',branch=b,parent=parent,tick=s['tick'],ordinal=str(ordinal),mode=mode,capture=snap(s,b,parent),command=command);frame=rf(root,r);r['id']=H(frame)
 s=cp(s);s['systems']=[new];s['next_sequence']=str(ordinal)
 audit=dict(revision=cp(r),source_state=before,source_state_preimage_hex=sf(before).hex(),revision_preimage_hex=frame.hex(),change=cp(ch),result_state=cp(s))
 if issuance:audit['issuance']=dict(preimage_hex=issuance.hex(),full_digest=H(issuance),origin=dict(branch=b,accepted_sequence=str(ordinal),operation_index=0,kind=1,local=1))
 return s,r,audit,ch

def inputs():
 m=json.loads((SDD/'v3-current-release-source-freeze/manifest.json').read_bytes());assert H((SDD/'v3-current-release-source-freeze/manifest.json').read_bytes())=='d0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c'
 for n,x in m['sources'].items():
  for p in (ROOT/'docs/superpowers/specs'/n,SDD/'v3-current-release-source-freeze'/n):assert len(p.read_bytes())==x['bytes'] and H(p.read_bytes())==x['sha256']
 fm=json.loads((SDD/'v3-current-fork-history-checkpoint-1/manifest.json').read_bytes())
 pins={}
 for n,x in fm['files'].items():
  p=SDD/n if n.endswith('report.md') else PARENT/n
  assert H(p.read_bytes())==x['sha256'] and len(p.read_bytes())==x['bytes'];pins[str(p.relative_to(ROOT))]=x
 for n,h in [('derive.py','9a188e905659c4700d0ea38cf0b9c74cdc6e11963b0c19f1873f18246ab463c6'),('vectors.json','b53197e0d8964c4783e135c952b9d8a69c4cdc11e9c9d8580e8d94b3df3b1397')]:assert H((HERE.parent/'running'/n).read_bytes())==h;pins[str((HERE.parent/'running'/n).relative_to(ROOT))]=dict(bytes=(HERE.parent/'running'/n).stat().st_size,sha256=h)
 cat=json.loads((CAT/'core.catalog.json').read_bytes());assert H(D('PACK')+C(cat))==PACK
 pa=json.loads((PARENT/'archive.json').read_bytes());pn=json.loads((PARENT/'notebook.json').read_bytes())
 return cat,pa,pn,m,pins

def build(cat,pa,pn):
 # Independently reconstruct inherited rows from literal written formulas, not trace assertions.
 g=cp(pa['genesis']);assert g['state']['systems']==[] and g['allocations']==[]
 gf=D('GENESIS')+C(g);rootf=D('ROOT_BRANCH')+bytes.fromhex(PACK)+bytes.fromhex(g['seed'])+bytes.fromhex(H(gf))+L(g['generator']['generator_id'].encode())+g['generator']['generator_version'].to_bytes(4,'little');root=H(rootf)[:32]
 s=cp(g['state']);views={key(view(root,None,0)):(cp(s),None)};pubs=[];audits=[];rows=[]
 for ordinal,name,create in [(1,'Alpha',True),(2,'Beta',False)]:
  s,r,a,ch=apply(root,root,None if create else rows[-1]['id'],s,ordinal,name,'paused',create);rows.append(r);audits.append(a);p=publication(root,r['id'],s,[(r['id'],ch)]);pubs.append(p);views[key(p['view'])]=(cp(s),p['receipt'])
 s['tick']='1';p=publication(root,rows[1]['id'],s,[]);pubs.append(p);views[key(p['view'])]=(cp(s),p['receipt'])
 rb=dict(id=root,parent=None,fork_revision=None,fork_tick='0',fork_state_digest=sd(g['state']),ordinal='0',head=rows[1]['id'],tip_tick='1')
 original=cp(views[key(view(root,rows[0]['id'],0))][0]);cb=dict(id='',parent=root,fork_revision=rows[0]['id'],fork_tick='0',fork_state_digest=sd(original),ordinal='1',head=rows[0]['id'],tip_tick='0')
 ff=D('FORK_BRANCH')+bytes.fromhex(root)*2+O(cb['fork_revision'])+Q(0)+bytes.fromhex(cb['fork_state_digest'])+Q(1);cb['id']=H(ff)[:32];child=cb['id'];s=original;s['branch_sequence']='1';fork_initial=cp(s);views[key(view(child,rows[0]['id'],0))]=(cp(s),None)
 s,r,a,ch=apply(root,child,rows[0]['id'],s,3,'Gamma','paused');rows.append(r);audits.append(a);p=publication(child,r['id'],s,[(r['id'],ch)]);pubs.append(p);views[key(p['view'])]=(cp(s),p['receipt']);cb['head']=r['id']
 base=cp(pa);base['revisions']=sorted(rows,key=lambda x:x['id']);base['branches']=sorted([rb,cb],key=lambda x:x['id']);base['anchors']=sorted([anchor(x['view']['branch'],x['view']['revision'],x['state'],x['receipt']) for x in pubs],key=ak);base['active_view']=cp(p['view']);base['final_tick']='0';base['final_digest']=sd(s);base['accepted_sequence_high_water']='3';base['branch_sequence_high_water']='1';seal(base,'ARCHIVE');assert C(base)==C(pa),'Inherited archive does not match independent formulas'
 for bm in pn['bookmarks']:assert bm['digest']==sd(views[key(bm['view'])][0])
 assert pn['archive_integrity_sha256']==pa['integrity_sha256'];assert seal(cp(pn),'NOTEBOOK')==pn
 # Genuine ordinary child boundary1, followed later by running edit at T2.
 s['tick']='1';ordinary=publication(child,r['id'],s,[]);pubs.append(ordinary);views[key(ordinary['view'])]=(cp(s),ordinary['receipt']);committed=cp(s);committed_revision=r['id'];s['tick']='2'
 s,r,a,ch=apply(root,child,committed_revision,s,4,'Delta','running');rows.append(r);audits.append(a)
 envelope=dict(domain=3,catalog_hash=PACK,branch=child,expected_committed_revision=committed_revision,expected_tick='1',expected_state_digest=sd(committed),expected_pending_sequence='0',mode='running',command=cp(r['command']))
 accepted=dict(accepted_sequence='4',application_tick='2',envelope=envelope)
 final=publication(child,r['id'],s,[(r['id'],ch)]);pubs.append(final);views[key(final['view'])]=(cp(s),final['receipt']);cb['head']=r['id'];cb['tip_tick']='2'
 archive=cp(base);archive['revisions']=sorted(rows,key=lambda x:x['id']);archive['branches']=sorted([rb,cb],key=lambda x:x['id']);archive['active_view']=cp(final['view']);archive['final_tick']='2';archive['final_digest']=sd(s);archive['accepted_sequence_high_water']='4';archive['anchors']=sorted([anchor(x['view']['branch'],x['view']['revision'],x['state'],x['receipt']) for x in pubs],key=ak);seal(archive,'ARCHIVE')
 n=cp(pn);n['archive_integrity_sha256']=archive['integrity_sha256']
 for ordinal,label,p in [(3,'Child Ordinary One',ordinary),(4,'Child Running Delta',final)]:n['bookmarks'].append(dict(ordinal=str(ordinal),label=label,view=cp(p['view']),digest=sd(p['state'])))
 n['next_ordinal']='5';seal(n,'NOTEBOOK')
 transfer=seal(dict(kind='NYON_LIVING_TRANSFER_V3',format_version=3,rules_version=3,catalog=cp(cat),archive=archive,notebook=n,selected_view=cp(final['view'])),'TRANSFER')
 trace=dict(status='Proposed construction audit only, not runtime queue proof or portable authority',root_frame=rootf.hex(),genesis_frame=gf.hex(),fork_frame=ff.hex(),fork_initial_state=fork_initial,operations=audits,publications=pubs,ordinary_child_publication=ordinary,published_before_submission=dict(view=view(child,committed_revision,1),state=committed,state_preimage_hex=sf(committed).hex()),accepted_command=accepted,staged_revision=cp(r),accepted_high_water='4',pending_tail='4',queued_boundary_receipt=None,published_cursor_while_queued=view(child,committed_revision,1),final_publication=final,final_pending_tail='0',root_branch_before=rb,root_branch_after=cp(rb),inherited_issuer=audits[0]['issuance']['origin'])
 return archive,n,transfer,trace,views

# Second independently spelled framing arithmetic, no calls to D/Q/L/O/rf/receipt/sf.
def second_formulas(trace,archive):
 enc=lambda x:json.dumps(x,ensure_ascii=False,separators=(',',':')).encode('utf-8')
 hh=lambda x:hashlib.sha256(x).hexdigest()
 g=archive['genesis'];gf=b'NYON_LIVING_V3_GENESIS\x00'+enc(g);assert gf.hex()==trace['genesis_frame']
 gi=g['generator']['generator_id'].encode();rootf=b'NYON_LIVING_V3_ROOT_BRANCH\x00'+bytes.fromhex(PACK)+bytes.fromhex(g['seed'])+bytes.fromhex(hh(gf))+len(gi).to_bytes(4,'little')+gi+int(g['generator']['generator_version']).to_bytes(4,'little');assert rootf.hex()==trace['root_frame']
 b=next(x for x in archive['branches'] if x['parent'] is not None)
 ff=b'NYON_LIVING_V3_FORK_BRANCH\x00'+bytes.fromhex(hh(rootf)[:32])+bytes.fromhex(b['parent'])+b'\x01'+bytes.fromhex(b['fork_revision'])+int(b['fork_tick']).to_bytes(8,'little')+bytes.fromhex(b['fork_state_digest'])+int(b['ordinal']).to_bytes(8,'little');assert ff.hex()==trace['fork_frame'] and hh(ff)[:32]==b['id']
 issuance=b'NYON_LIVING_V3_CREATOR_ENTITY\x00'+bytes.fromhex(hh(rootf)[:32])*2+b'\x02'+(1).to_bytes(8,'little')+(0).to_bytes(4,'little')+(1).to_bytes(2,'little')+(1).to_bytes(2,'little');assert issuance.hex()==trace['operations'][0]['issuance']['preimage_hex']
 for a in trace['operations']:
  r=a['revision'];parent=b'\x00' if r['parent'] is None else b'\x01'+bytes.fromhex(r['parent']);ca=enc(r['capture']);co=enc(r['command'])
  f=b'NYON_LIVING_V3_REVISION\x00'+bytes.fromhex(trace['root_branch_before']['id'])+bytes.fromhex(r['branch'])+parent+int(r['tick']).to_bytes(8,'little')+int(r['ordinal']).to_bytes(8,'little')+bytes([{'paused':0,'running':1}[r['mode']]])+len(ca).to_bytes(4,'little')+ca+len(co).to_bytes(4,'little')+co
  assert f.hex()==a['revision_preimage_hex'] and hh(f)==r['id']
 for p in trace['publications']:
  sf2=b'NYON_LIVING_V3_STATE\x00'+enc({'format_version':3,'rules_version':3,'catalog_hash':PACK,'state':p['state']});assert sf2.hex()==p['state_preimage_hex'];assert hh(sf2)==p['receipt']['state_digest']
  payload={'tick':p['receipt']['tick'],'applied_revisions':p['receipt']['applied_revisions'],'events':[{k:v for k,v in e.items() if k!='id'} for e in p['receipt']['events']],'state_digest':hh(sf2)};f=b'NYON_LIVING_V3_RECEIPT\x00'+enc(payload);assert f.hex()==p['receipt_frame']['preimage_hex'] and hh(f)==p['receipt']['digest']
  for e in p['receipt']['events']:
   ef=b'NYON_LIVING_V3_EVENT\x00'+bytes.fromhex(hh(f))+int(p['state']['tick']).to_bytes(8,'little')+int(e['phase']).to_bytes(2,'little')+int(e['ordinal']).to_bytes(4,'little');assert hh(ef)[:32]==e['id']
 for audit in trace['operations']:
  prefix=b'NYON_LIVING_V3_STATE\x00'+enc({'format_version':3,'rules_version':3,'catalog_hash':PACK,'state':audit['source_state']})
  assert prefix.hex()==audit['source_state_preimage_hex'] and hh(prefix)==audit['revision']['capture']['state_digest']==audit['revision']['capture']['permitted_snapshot_digest']
 assert archive['branches']==sorted(archive['branches'],key=lambda b:b['id']);assert archive['revisions']==sorted(archive['revisions'],key=lambda r:r['id'])


def products():
 cat,pa,pn,m,pins=inputs();a,n,t,tr,views=build(cat,pa,pn);second_formulas(tr,a);cases=[]
 def case(title,f,d,strength,reason=None):cases.append(dict(id=title,family=f,strength=strength,expected=dict(admitted=True) if strength=='admitted_selected_family' else dict(admitted=False,rejection_only=reason),raw_base64=base64.b64encode(C(d)).decode()))
 for title,f,d in [('final_archive','archive',a),('reattached_notebook','notebook',n),('final_transfer','transfer',t)]:case(title,f,d,'admitted_selected_family')
 # Exact earlier active/selected views do not alter retained tips or future.
 for i,(v,(s,q)) in enumerate(sorted(views.items())):
  av=dict(branch=v[0],revision=v[1] or None,tick=str(v[2]));x=cp(a);x['active_view']=av;x['final_tick']=s['tick'];x['final_digest']=sd(s);seal(x,'ARCHIVE');case('earlier_active_'+str(i),'archive',x,'admitted_selected_family');y=cp(t);y['selected_view']=av;seal(y,'TRANSFER');case('earlier_selected_'+str(i),'transfer',y,'admitted_selected_family')
 def mutate(title,f,d,fn,reason):x=cp(d);fn(x);seal(x,f.upper());case(title,f,x,'rejection_only',reason)
 child=tr['final_publication']['view']['branch'];local=lambda x:next(r for r in x['revisions'] if r['ordinal']=='4');cb=lambda x:next(b for b in x['branches'] if b['parent'] is not None);rb=lambda x:next(b for b in x['branches'] if b['parent'] is None)
 for field,value in [('tick','1'),('ordinal','3'),('mode','paused'),('parent',rb(a)['head']),('branch',rb(a)['id'])]:mutate('running_revision_'+field,'archive',a,lambda x,f=field,v=value:local(x).__setitem__(f,v),'Revision identity/capture/accepted chronology mismatch')
 for field,value in [('tick','1'),('revision',rb(a)['head']),('branch',rb(a)['id']),('phase',7),('state_digest',tr['final_publication']['receipt']['state_digest']),('permitted_snapshot_digest',tr['published_before_submission']['state_preimage_hex'][:64])]:mutate('running_capture_'+field,'archive',a,lambda x,f=field,v=value:local(x)['capture'].__setitem__(f,v),'Original running phase1 source mismatch')
 for field,value in [('accepted_sequence_high_water','3'),('branch_sequence_high_water','0'),('final_digest',pa['final_digest'])]:mutate(field,'archive',a,lambda x,f=field,v=value:x.__setitem__(f,v),'Allocator regression or independently replayed selected digest mismatch')
 for field,value in [('fork_state_digest',sd(tr['fork_initial_state'])),('fork_revision',rb(a)['head']),('ordinal','0'),('head',cb(pa)['head'])]:mutate('child_branch_'+field,'archive',a,lambda x,f=field,v=value:cb(x).__setitem__(f,v),'Fork origin/producing ordinal/local head mismatch')
 mutate('lost_root_future','archive',a,lambda x:rb(x).__setitem__('tip_tick','0'),'Existing root Bookmark/Anchor at1 becomes unreachable')
 mutate('wrong_running_before','archive',a,lambda x:local(x)['command']['reviewed_effects'][0]['before']['value'].__setitem__('name','Beta'),'Child replay before is Gamma, not sibling Beta')
 mutate('invented_fork_anchor','archive',a,lambda x:x['anchors'].append(dict(domain=3,catalog_hash=PACK,branch=child,revision=cb(x)['fork_revision'],tick='0',state_digest=sd(tr['fork_initial_state']),receipt_digest='0'*64,event=None)),'Fork-only view has no receipt')
 for field,value in [('digest',pa['final_digest']),('view',view(child,cb(a)['head'],1)),('ordinal','2')]:mutate('new_bookmark_'+field,'notebook',n,lambda x,f=field,v=value:x['bookmarks'][-1].__setitem__(f,v),'Bookmark view/digest or common ordinal uniqueness mismatch')
 mutate('old_notebook_attachment','notebook',n,lambda x:x.__setitem__('archive_integrity_sha256',pa['integrity_sha256']),'Notebook must bind extended exact Archive')
 mutate('unreachable_selected','transfer',t,lambda x:x.__setitem__('selected_view',view(child,cb(a)['head'],0)),'Running R4 cannot be completed at tick0')
 trace_cases=[]
 for title,path,value in [('child_counter_reset',['ordinary_child_publication','state','branch_sequence'],'0'),('ordinary_copies_global_mark',['ordinary_child_publication','state','next_sequence'],'4'),('queued_calls_prefix_published',['published_cursor_while_queued','tick'],'2'),('fake_queued_receipt',['queued_boundary_receipt'],tr['final_publication']['receipt']),('wrong_envelope_commit',['accepted_command','envelope','expected_committed_revision'],local(a)['id']),('wrong_queue_tail',['pending_tail'],'0'),('borrow_sibling',['operations',3,'source_state','systems',0,'name'],'Beta'),('fake_applied_ordinary',['ordinary_child_publication','receipt','applied_revisions'],[cb(pa)['head']])]:
  x=cp(tr);p=x
  for k in path[:-1]:p=p[k]
  p[path[-1]]=value;assert C(x)!=C(tr);trace_cases.append(dict(id=title,assertion='construction_trace_mismatch_only',path=path,value=value))
 assert len(set(x['id'] for x in cases))==len(cases)
 ledger=dict(status='Proposed selected-family author freeze; independent review pending',sources=m,dependency_pins=pins,successful_family='empty explicit Genesis -> retained root paused S1/S2 and root ordinary1 -> child paused S3/Gamma at0 -> child ordinary1 -> child running S4/Delta at2',normative_sections={'schema:2390-2415':'entity/Revision/Fork/State frames','schema:2422-2454':'historical counters/durable marks/running capture','schema:2458-2495':'creator event partition/fork replay/no fork receipt','schema:2497-2530':'Notebook common ordinal/reattachment','schema:2581-2692':'Archive/Notebook/Transfer exact carriers'},counts=dict(positive=sum(x['strength']=='admitted_selected_family' for x in cases),rejection_only=sum(x['strength']=='rejection_only' for x in cases),private_trace=len(trace_cases),views=len(views),publications=len(tr['publications'])),unsupported=['Different shape-valid metadata/Genesis/Catalog families are outside this exact helper, not invalid wire','Abandoned accepted tail and high-water holes not constructed','Intermediate running selected-view matrices and additional child/root evolution','Generic parser/depth/multifault/Branch collision/capacity diagnostics','Inhabited ecology/production/actor/forecast/combat/diplomacy','Runtime queue race/bounded poll/cancellation/durability'],second_formula_check='Revision/STATE/Receipt/Event reconstructed independently with inline arithmetic; no generator imported',classification_scope='All portable negatives rejection-only, no invented decoder precedence; private trace failures not wire rejections')
 return {'archive.json':C(a),'notebook.json':C(n),'transfer.json':C(t),'trace.json':json.dumps(tr,ensure_ascii=False,indent=2).encode()+b'\n','vectors.json':json.dumps(dict(status='Proposed CHILD-RUN',cases=cases,trace_binding_cases=trace_cases),ensure_ascii=False,indent=2).encode()+b'\n','predicate-coverage.json':json.dumps(ledger,indent=2).encode()+b'\n'}

def selected_family_check(raw,f,expected_archive,expected_notebook,expected_transfer,views):
 """Narrow source-reconstructed family, no general parser/stage priority claims.
 All expected normative rows come from build's independent replay, not an engine.
 """
 try:
  d=json.loads(raw.decode('utf8'))
  if C(d)!=raw:return False
  if f=='archive':
   if d.get('kind')!='NYON_LIVING_ARCHIVE_V3':return False
   e=cp(expected_archive)
   k=key(d['active_view'])
   if k not in views:return False
   e['active_view']=cp(d['active_view']);e['final_tick']=views[k][0]['tick'];e['final_digest']=sd(views[k][0])
   # Extra high-water reservations are legal, but this corpus has no causal hole claim.
   for field in ['accepted_sequence_high_water','branch_sequence_high_water']:
    if not isinstance(d[field],str) or not d[field].isdigit() or str(int(d[field]))!=d[field] or int(d[field])<int(e[field]) or int(d[field])>=2**64:return False
    e[field]=d[field]
   seal(e,'ARCHIVE');return C(d)==C(e)
  if f=='notebook':return C(d)==C(expected_notebook)
  if f=='transfer':
   k=key(d['selected_view'])
   if k not in views:return False
   e=cp(expected_transfer);e['selected_view']=cp(d['selected_view']);seal(e,'TRANSFER');return C(d)==C(e)
  return False
 except (KeyError,TypeError,ValueError,UnicodeError):return False

def main():
 p=argparse.ArgumentParser();p.add_argument('mode',choices=['generate','verify']);args=p.parse_args();out=products()
 for n,raw in out.items():
  if args.mode=='generate':(HERE/n).write_bytes(raw)
  else:assert (HERE/n).read_bytes()==raw,'Generated bytes differ: '+n
 v=json.loads(out['vectors.json']);cat,pa,pn,_,_=inputs();a,n,t,tr,views=build(cat,pa,pn)
 for case in v['cases']:
  actual=selected_family_check(base64.b64decode(case['raw_base64']),case['family'],a,n,t,views)
  assert actual==case['expected']['admitted'],case['id']
 for case in v['trace_binding_cases']:
  x=cp(tr);p=x
  for k in case['path'][:-1]:p=p[k]
  p[case['path'][-1]]=case['value'];assert C(x)!=C(tr),case['id']
 print(args.mode+' OK '+str(len(v['cases']))+' portable cases, '+str(len(v['trace_binding_cases']))+' construction trace cases; independently spelled frames agree')
if __name__=='__main__':main()
