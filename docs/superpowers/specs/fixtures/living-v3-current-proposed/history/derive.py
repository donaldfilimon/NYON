#!/usr/bin/env python3
"""Fresh HIST-C root-only paused-System journey fixture construction/replay.
Standard-library author oracle; not a general production decoder or job engine.
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
PARENT = HERE.parent
SDD = ROOT / '.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion'
FREEZE = SDD / 'v3-current-release-source-freeze'
PACK = '9720f69feae4b1b04cc3459b4a156153c284eae8658197d922a142e634191da8'
PINS = {
    'v3-current-release-source-freeze/manifest.json': 'd0ab6b0d86808e6f9f3fabfeb503e1a143d7b108c1e49468bb94dd88ec206c8c',
    'v3-coordinated-amendment-current-source-manifest-2.json': 'bab14622dfb118e45801f228e1153309641f72b3a50a8d84eb3cd0466a46bd3f',
    'v3-coordinated-amendment-integrated-review.md': 'cd485f38a06964734e435bb245025579a40ee59da176cc3f1c1c85a07a98ca65',
    'v3-anchor-summary-rereview-1.md': '947111491e1467c6a122817c0a57c34d9afe384c4f3ae4ded8ff275608ca7ecf',
    'v3-current-catalog-review.md': 'd556d3230b34a167c997bbbfb58e6a6308e517d5fa006b23c806dfb6f0ded9f6',
    'v3-current-genesis-review.md': 'b7167d88507540d7f4b5a19ceda44fe9ecf0425944cd64b7c065e23f0eaec042',
    'v3-current-genesis-wording-rereview.md': 'd993d81664fcb1b5f53bf55946a66065b26209959588c89f1ffeb36e423c0464',
    'v3-current-genesis-wording-correction.md': 'a9258cc4cf37b1b11b2304938860460e3cb1abec6bbcaa91e5cb310a4f656e02',
    'v3-current-genesis-write-report.md': '54e07e9f619fc5f78a373f4c94976d2f92c05a4b25a878c20b2017a66172e274',
    'v3-current-history-admission-preflight.md': '904b574d06d318c6802744fe0569ec8065c63c2072aca8e2af1f20f476d33522',
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
FIELDS = 'tick branch_sequence next_sequence systems stars worlds lanes civilizations deposits colonies facilities construction_jobs hull_jobs fleets routes shipments relations agreements wars observations hazards settlement_claims occupations offers demand_requests commitments defense_requests shared_intelligence stellar_events counters'.split()


def H(raw): return hashlib.sha256(raw).hexdigest()
def D(name): return ('NYON_LIVING_V3_' + name).encode('ascii') + b'\0'
def C(value): return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode('utf8')
def L(raw): return struct.pack('<I', len(raw)) + raw
def Q(value): return struct.pack('<Q', int(value))
def O(value): return b'\0' if value is None else b'\1' + bytes.fromhex(value)
def p(path, token): return path + '/' + str(token).replace('~', '~0').replace('/', '~1')


def freshness():
    for name,digest in PINS.items():
        if H((SDD/name).read_bytes()) != digest: raise SystemExit('STOP: frozen report/source dependency changed '+name)
    for name,digest in DEPENDENCIES.items():
        if H((PARENT/name).read_bytes()) != digest: raise SystemExit('STOP: accepted corpus dependency changed '+name)
    manifest=json.loads((FREEZE/'manifest.json').read_bytes())
    if len(manifest['sources']) != 8: raise SystemExit('STOP: source roster changed')
    for name,expected in manifest['sources'].items():
        for file in (FREEZE/name,ROOT/'docs/superpowers/specs'/name):
            raw=file.read_bytes()
            if len(raw)!=expected['bytes'] or H(raw)!=expected['sha256']: raise SystemExit('STOP: source changed '+str(file))
    catraw=(PARENT/'core.catalog.json').read_bytes();genraw=(PARENT/'genesis/empty.genesis.json').read_bytes()
    if len(catraw)!=4264 or H(D('PACK')+catraw)!=PACK:raise SystemExit('STOP: admitted Catalog/PACK changed')
    return json.loads(catraw),json.loads(genraw),manifest['sources']


def state_frame(state): return D('STATE')+C(dict(format_version=3,rules_version=3,catalog_hash=PACK,state=state))
def state_digest(state): return H(state_frame(state))
def root_frame(genesis):
    generator=genesis['generator']
    return (D('ROOT_BRANCH')+bytes.fromhex(PACK)+bytes.fromhex(genesis['seed'])+
                                    bytes.fromhex(H(D('GENESIS')+C(genesis)))+L(generator['generator_id'].encode('utf8'))+
                                    struct.pack('<I',generator['generator_version']))

def creator_frame(root,branch,sequence,local):
    return D('CREATOR_ENTITY')+bytes.fromhex(root)+bytes.fromhex(branch)+b'\2'+Q(sequence)+struct.pack('<IHH',0,1,local)

def revision_frame(root,revision):
    return (D('REVISION')+bytes.fromhex(root)+bytes.fromhex(revision['branch'])+O(revision['parent'])+
                                    Q(revision['tick'])+Q(revision['ordinal'])+bytes([0 if revision['mode']=='paused' else 1])+
                                    L(C(revision['capture']))+L(C(revision['command'])))

def integrity(document,name):
    return H(D(name)+C({key:value for key,value in document.items() if key!='integrity_sha256'}))

def reseal(document,name):
    document['integrity_sha256']=integrity(document,name);return document

def snapshot(root,parent,state):
    digest=state_digest(state)
    return dict(domain=3,catalog_hash=PACK,branch=root,revision=parent,tick=state['tick'],
                                                    state_digest=digest,phase=1,permitted_snapshot_digest=digest)

def envelope(root,parent,state,command):
    return dict(domain=3,catalog_hash=PACK,branch=root,expected_committed_revision=parent,
                                                    expected_tick=state['tick'],expected_state_digest=state_digest(state),
                                                    expected_pending_sequence='0',mode='paused',command=command)

def change(before,after):
    row=after if after is not None else before
    return dict(operation_index=0,collection='systems',key=dict(type='systems',id=row['id']),
                                                    before=None if before is None else dict(type='systems',value=copy.deepcopy(before)),
                                                    after=None if after is None else dict(type='systems',value=copy.deepcopy(after)),resources=[],passenger_delta='0')

def receipt(state,revision,changes):
    events=[]
    if revision is not None:
        events=[dict(phase=1,ordinal=0,provenance=dict(type='creator',revision=revision),
                                                            kind='creator_intervention',facts=dict(type='change',changes=copy.deepcopy(changes)))]
    payload=dict(tick=state['tick'],applied_revisions=[] if revision is None else [revision],events=events,state_digest=state_digest(state))
    preimage=D('RECEIPT')+C(payload);digest=H(preimage);public=[];frames=[]
    for event in events:
        frame=D('EVENT')+bytes.fromhex(digest)+Q(state['tick'])+struct.pack('<HI',event['phase'],event['ordinal'])
        public.append(dict(id=H(frame)[:32],**event));frames.append(dict(preimage_hex=frame.hex(),full_digest=H(frame),id=H(frame)[:32]))
    return dict(tick=payload['tick'],applied_revisions=payload['applied_revisions'],events=public,
                                                    state_digest=payload['state_digest'],digest=digest),dict(payload=payload,preimage_base64=base64.b64encode(preimage).decode(),event_frames=frames)

def anchor(root,parent,state,public_receipt):
    return dict(domain=3,catalog_hash=PACK,branch=root,revision=parent,tick=state['tick'],
                                                    state_digest=state_digest(state),receipt_digest=public_receipt['digest'],event=None)

def anchor_key(row):
    e=row['event']
    return (row['branch'],'' if row['revision'] is None else row['revision'],int(row['tick']),row['receipt_digest'],
                                    (-1,-1,'') if e is None else (e['phase'],e['ordinal'],e['id']))


def construct(catalog,genesis,first='Alpha',second='Beta',label='Boundary One'):
    root=H(root_frame(genesis))[:32];state=copy.deepcopy(genesis['state']);parent=None;revisions=[];publications=[];preparations=[]
    for seq,name in ((1,first),(2,second)):
        before=copy.deepcopy(state);capture=snapshot(root,parent,before)
        if seq==1:
            frame=creator_frame(root,root,str(seq),1);ident=H(frame)[:32]
            operation=dict(type='create_system',local=1,name=name);prior=None;row=dict(id=ident,name=name)
        else:
            frame=None;operation=dict(type='edit_system',target=dict(type='existing',id=ident),name=name)
            prior=copy.deepcopy(before['systems'][0]);row=dict(id=ident,name=name)
        changes=[change(prior,row)];command=dict(type='creator_batch',operations=[operation],reviewed_effects=changes)
        state['systems']=[row];state['next_sequence']=str(seq)
        rev=dict(id='00'*32,branch=root,parent=parent,tick='0',ordinal=str(seq),mode='paused',capture=capture,command=command)
        rev['id']=H(revision_frame(root,rev));revisions.append(rev)
        public,frames=receipt(state,rev['id'],changes)
        preparations.append(dict(accepted_sequence=str(seq),application_tick='0',envelope=envelope(root,parent,before,command)))
        publications.append(dict(name='paused'+str(seq),view=dict(branch=root,revision=rev['id'],tick='0'),
                state=copy.deepcopy(state),state_preimage_base64=base64.b64encode(state_frame(state)).decode(),
                revision_preimage_base64=base64.b64encode(revision_frame(root,rev)).decode(),
                creator_issuance=None if frame is None else dict(preimage_hex=frame.hex(),full_digest=H(frame),id=ident,
                        origin=dict(branch=root,accepted_sequence='1',operation_index=0,kind=1,local=1)),receipt=public,receipt_frames=frames,
                accepted_sequence_high_water=str(seq),branch_sequence_high_water='0'))
        parent=rev['id']
    state['tick']='1';public,frames=receipt(state,None,[])
    publications.append(dict(name='ordinary1',view=dict(branch=root,revision=parent,tick='1'),state=copy.deepcopy(state),
                state_preimage_base64=base64.b64encode(state_frame(state)).decode(),revision_preimage_base64=None,creator_issuance=None,
                receipt=public,receipt_frames=frames,accepted_sequence_high_water='2',branch_sequence_high_water='0'))
    anchors=[anchor(root,pub['view']['revision'],pub['state'],pub['receipt']) for pub in publications];anchors.sort(key=anchor_key)
    branch=dict(id=root,parent=None,fork_revision=None,fork_tick='0',fork_state_digest=state_digest(genesis['state']),ordinal='0',head=parent,tip_tick='1')
    view=dict(branch=root,revision=parent,tick='1')
    archive=reseal(dict(kind='NYON_LIVING_ARCHIVE_V3',format_version=3,rules_version=3,catalog_hash=PACK,
                    genesis=copy.deepcopy(genesis),revisions=sorted(revisions,key=lambda r:r['id']),branches=[branch],active_view=view,
                    final_tick='1',final_digest=state_digest(state),accepted_sequence_high_water='2',branch_sequence_high_water='0',anchors=anchors),'ARCHIVE')
    notebook=reseal(dict(kind='NYON_LIVING_NOTEBOOK_V3',format_version=3,rules_version=3,catalog_hash=PACK,
                    genesis_digest=H(D('GENESIS')+C(genesis)),archive_integrity_sha256=archive['integrity_sha256'],next_ordinal='2',
                    bookmarks=[dict(ordinal='1',label=label,view=copy.deepcopy(view),digest=state_digest(state))],experiments=[],evidence=[]),'NOTEBOOK')
    transfer=reseal(dict(kind='NYON_LIVING_TRANSFER_V3',format_version=3,rules_version=3,catalog=copy.deepcopy(catalog),
                    archive=copy.deepcopy(archive),notebook=copy.deepcopy(notebook),selected_view=copy.deepcopy(view)),'TRANSFER')
    trace=dict(genesis_preimage_base64=base64.b64encode(D('GENESIS')+C(genesis)).decode(),
                    genesis_state_preimage_base64=base64.b64encode(state_frame(genesis['state'])).decode(),
                    root_preimage_hex=root_frame(genesis).hex(),root_full_digest=H(root_frame(genesis)),root_id=root,
                    command_preparations=preparations,publications=publications,queue_harness=dict(pending_commands=0,pending_tail='0'),
                    archive_preimage_base64=base64.b64encode(D('ARCHIVE')+C({k:v for k,v in archive.items() if k!='integrity_sha256'})).decode(),
                    notebook_preimage_base64=base64.b64encode(D('NOTEBOOK')+C({k:v for k,v in notebook.items() if k!='integrity_sha256'})).decode(),
                    transfer_preimage_base64=base64.b64encode(D('TRANSFER')+C({k:v for k,v in transfer.items() if k!='integrity_sha256'})).decode())
    return archive,notebook,transfer,trace


class Reject(Exception):
    def __init__(self,stage,error,path):self.stage,self.error,self.path=stage,error,path
class Unsupported(Exception):pass
def reject(stage,error,path):raise Reject(stage,error,path)


def parse(raw,cap):
    # Explicit bounded syntax scan before stdlib parse. No document parser sees depth>64.
    if len(raw)>cap:reject(0,'byte_capacity','')
    try:text=raw.decode('utf8')
    except UnicodeDecodeError:reject(1,'noncanonical_bytes','')
    decoder=json.JSONDecoder();stack=[];position=0;complete=False;duplicates=[]
    def put(value):
        nonlocal complete
        if not stack:complete=True;return
        f=stack[-1]
        if f['obj']:
            f['pairs'].append((f['key'],value))
        else:f['pairs'].append(value)
        f['status']='comma'
    def scalar():
        nonlocal position
        offset=position
        try:v,position=decoder.raw_decode(text,position)
        except (ValueError,RecursionError):reject(2,'noncanonical_bytes','')
        if isinstance(v,(dict,list)) or (isinstance(v,str) and any(0xd800<=ord(c)<=0xdfff for c in v)):reject(2,'noncanonical_bytes','')
        if isinstance(v,float) and not (-float('inf')<v<float('inf')):reject(2,'noncanonical_bytes','')
        return v,offset
    while True:
        while position<len(text) and text[position] in ' \r\n\t':position+=1
        if complete and not stack:
            if position!=len(text):reject(2,'noncanonical_bytes','')
            break
        if position>=len(text):reject(2,'noncanonical_bytes','')
        f=stack[-1] if stack else None
        if f and f['status']=='comma':
            close='}' if f['obj'] else ']'
            if text[position]==close:position+=1;stack.pop();put(None);continue
            if text[position]!=',':reject(2,'noncanonical_bytes','')
            position+=1;f['status']='key' if f['obj'] else 'value';continue
        if f and f['obj'] and f['status'] in ('key','first_key'):
            if text[position]=='}' and f['status']=='first_key':position+=1;stack.pop();put(None);continue
            if text[position]!='"':reject(2,'noncanonical_bytes','')
            key,offset=scalar()
            if key in f['seen']:duplicates.append((offset,p(f['path'],key)))
            f['seen'].add(key);f['key']=key;f['status']='colon';continue
        if f and f['status']=='colon':
            if text[position]!=':':reject(2,'noncanonical_bytes','')
            position+=1;f['status']='value';continue
        if f and not f['obj'] and f['status']=='first_value' and text[position]==']':position+=1;stack.pop();put(None);continue
        path='' if f is None else p(f['path'],f['key'] if f['obj'] else len(f['pairs']))
        if text[position] in '{[':
            if len(stack)==64:reject(2,'depth_capacity','')
            obj=text[position]=='{';position+=1;stack.append(dict(obj=obj,path=path,key=None,pairs=[],seen=set(),status='first_key' if obj else 'first_value'))
        else:v,_=scalar();put(v)
    if duplicates:reject(3,'duplicate_key',min(duplicates)[1])
    # The scan proved bounded syntax and no duplicates; materialization is fixture-only.
    return json.loads(text)


def record(**fields):return ('record',fields)
def option(schema):return ('option',schema)
def array(schema,cap,key=None):return ('array',schema,cap,key)
def enum(*tags):return ('enum',tags)
def tagged(variants):return ('tagged',variants)
VIEW=record(branch='ID',revision=option('Digest'),tick='u64')
SNAP=record(domain='u32',catalog_hash='Digest',branch='ID',revision=option('Digest'),tick='u64',state_digest='Digest',phase='u16',permitted_snapshot_digest='Digest')
SYSTEM=record(id='ID',name='Name')
RECORD=tagged({'systems':record(type='tag',value=SYSTEM)})
KEY=tagged({'systems':record(type='tag',id='ID')})
CHANGE=record(operation_index=option('u32'),collection='tag',key=KEY,before=option(RECORD),after=option(RECORD),resources=array('unsupported',None),passenger_delta='score')
TARGET=tagged({'existing':record(type='tag',id='ID'),'local':record(type='tag',local='u16')})
OP=tagged({'create_system':record(type='tag',local='u16',name='Name'),'edit_system':record(type='tag',target=TARGET,name='Name')})
COMMAND=tagged({'creator_batch':record(type='tag',operations=array(OP,1024),reviewed_effects=array(CHANGE,None))})
REVISION=record(id='Digest',branch='ID',parent=option('Digest'),tick='u64',ordinal='u64',mode=enum('paused','running'),capture=SNAP,command=COMMAND)
BRANCH=record(id='ID',parent=option('ID'),fork_revision=option('Digest'),fork_tick='u64',fork_state_digest='Digest',ordinal='u64',head=option('Digest'),tip_tick='u64')
ANCHOR=record(domain='u32',catalog_hash='Digest',branch='ID',revision=option('Digest'),tick='u64',state_digest='Digest',receipt_digest='Digest',event=option('unsupported'))
STATE=record(**dict([(n,'u64') for n in FIELDS[:3]]+[(n,array(SYSTEM if n=='systems' else 'unsupported',64 if n=='systems' else None,('id',) if n=='systems' else None)) for n in FIELDS[3:-1]]+[('counters',record())]))
GENESIS=record(kind='tag',format_version='u32',rules_version='u32',catalog_hash='Digest',seed='Digest',generator=record(generator_id='Slug',generator_version='u32'),allocations=array('unsupported',None),state=STATE)
ARCHIVE=record(kind='tag',format_version='u32',rules_version='u32',catalog_hash='Digest',genesis=GENESIS,revisions=array(REVISION,10000,('id',)),branches=array(BRANCH,128,('id',)),active_view=VIEW,final_tick='u64',final_digest='Digest',accepted_sequence_high_water='u64',branch_sequence_high_water='u64',anchors=array(ANCHOR,16384,'anchor'))
BOOKMARK=record(ordinal='u64',label='Name',view=VIEW,digest='Digest')
NOTEBOOK=record(kind='tag',format_version='u32',rules_version='u32',catalog_hash='Digest',genesis_digest='Digest',archive_integrity_sha256='Digest',next_ordinal='u64',bookmarks=array(BOOKMARK,1024,('ordinal',)),experiments=array('unsupported',256),evidence=array('unsupported',4096))
ARCHIVE[1]['integrity_sha256']='Digest';NOTEBOOK[1]['integrity_sha256']='Digest'
TRANSFER=record(kind='tag',format_version='u32',rules_version='u32',catalog='dependency_catalog',archive=ARCHIVE,notebook=NOTEBOOK,selected_view=VIEW,integrity_sha256='Digest')
SCHEMAS=dict(archive=ARCHIVE,notebook=NOTEBOOK,transfer=TRANSFER)
CAPS=dict(archive=64*1024*1024,notebook=8*1024*1024,transfer=96*1024*1024)


def shape(value,schema,path='',catalog=None):
    if isinstance(schema,str):
        if schema=='unsupported':raise Unsupported(path)
        if schema=='dependency_catalog':
            if C(value)!=C(catalog):raise Unsupported(path+' (non-admitted Catalog dependency)')
            return value
        if schema.startswith('u'):
            width=int(schema[1:])
            if width==64:
                if not isinstance(value,str):reject(5,'invalid_range',path)
                if not re.fullmatch('0|[1-9][0-9]*',value):reject(5,'noncanonical_bytes',path)
                if len(value)>20 or int(value)>2**64-1:reject(5,'invalid_range',path)
            elif type(value)is not int or not 0<=value<2**width:reject(5,'invalid_range',path)
        elif schema=='score':
            if not isinstance(value,str):reject(5,'invalid_range',path)
            if not re.fullmatch('0|-?[1-9][0-9]*',value):reject(5,'noncanonical_bytes',path)
            if len(value)>20 or not -(2**63)<=int(value)<=2**63-1:reject(5,'invalid_range',path)
        else:
            if not isinstance(value,str):reject(5,'invalid_range',path)
            if schema in ('Digest','ID') and not re.fullmatch('[0-9a-f]{'+('64' if schema=='Digest' else '32')+'}',value):reject(5,'noncanonical_bytes',path)
            if schema=='Slug' and not re.fullmatch('[a-z][a-z0-9_]{0,63}',value):reject(5,'invalid_range',path)
            if schema=='Name' and (not 1<=len(value)<=64 or len(value.encode('utf8'))>256 or value.startswith(' ') or value.endswith(' ') or '  ' in value or any(ord(c)<32 or 127<=ord(c)<=159 for c in value)):reject(5,'invalid_range',path)
        return value
    tag=schema[0]
    if tag=='option':return None if value is None else shape(value,schema[1],path,catalog)
    if tag=='enum':
        if not isinstance(value,str):reject(5,'invalid_range',path)
        if value not in schema[1]:reject(5,'invalid_enum',path)
        return value
    if tag=='tagged':
        if not isinstance(value,dict):reject(5,'invalid_range',path)
        if 'type' not in value:reject(5,'noncanonical_bytes',p(path,'type'))
        if not isinstance(value['type'],str):reject(5,'invalid_range',p(path,'type'))
        if value['type']not in schema[1]:raise Unsupported(p(path,'type'))
        return shape(value,schema[1][value['type']],path,catalog)
    if tag=='record':
        if not isinstance(value,dict):reject(5,'invalid_range',path)
        extra=set(value)-set(schema[1])
        if extra:reject(5,'unknown_field',p(path,min(extra,key=lambda key:key.encode('utf8'))))
        result={}
        for key,child in schema[1].items():
            if key not in value:reject(5,'noncanonical_bytes',p(path,key))
            result[key]=shape(value[key],child,p(path,key),catalog)
        return result
    if not isinstance(value,list):reject(5,'invalid_range',path)
    if schema[2] is not None and len(value)>schema[2]:reject(5,'record_capacity',path)
    result=[shape(row,schema[1],p(path,i),catalog) for i,row in enumerate(value)]
    last=None
    for i,row in enumerate(result):
        keyfields=schema[3]
        if keyfields is None:continue
        key=anchor_key(row) if keyfields=='anchor' else tuple(int(row[k]) if k=='ordinal' else row[k] for k in keyfields)
        if last is not None and key<=last:reject(5,'duplicate_key' if key==last else 'noncanonical_bytes',p(path,i))
        last=key
    return result


def header(doc,family):
    if not isinstance(doc,dict):reject(4,'invalid_range','/kind')
    for key,expected in [('kind','NYON_LIVING_'+family.upper()+'_V3'),('format_version',3),('rules_version',3)]:
        if key not in doc:reject(4,'noncanonical_bytes','/'+key)
        value=doc[key]
        if (key=='kind' and not isinstance(value,str)) or (key!='kind' and (type(value)is not int or not 0<=value<=2**32-1)):reject(4,'invalid_range','/'+key)
        if value!=expected:reject(4,'unsupported_kind' if key=='kind' else 'unsupported_'+key,'/'+key)


def validate_integrity(doc,name,path=''):
    if integrity(doc,name)!=doc['integrity_sha256']:reject(9,'integrity_mismatch',p(path,'integrity_sha256'))

def embedded_header(document,family,path):
                # Exact constants apply to embedded consumers too; no diagnostic stage/path
                # strength is exported for these semantic witnesses.
                expected = [('kind','NYON_LIVING_'+family.upper()+'_V3'),('format_version',3),('rules_version',3)]
                for key,value in expected:
                                if document[key] != value:reject(7,'invalid_range',p(path,key))


def replay_archive(archive,genesis,path=''):
    embedded_header(archive,'archive',path)
    embedded_header(archive['genesis'],'genesis',path+'/genesis')
    if C(archive['genesis'])!=C(genesis):raise Unsupported(p(path,'genesis')+' (only admitted empty GEN)')
    if len(archive['branches'])!=1 or len(archive['revisions'])!=2 or archive['final_tick']!='1':raise Unsupported(path+' (only root/two paused/boundary1 family)')
    if archive['catalog_hash']!=PACK:reject(8,'catalog_mismatch',p(path,'catalog_hash'))
    root=H(root_frame(genesis))[:32];branch=archive['branches'][0];revisions={r['id']:r for r in archive['revisions']}
    if branch['parent'] is not None or branch['fork_revision'] is not None or branch['fork_tick']!='0' or branch['ordinal']!='0':raise Unsupported(path+'/branches/0'+' (nonroot history)')
    if branch['head']not in revisions:reject(8,'invalid_history_view',path+'/branches/0/head')
    tail=revisions[branch['head']]
    if tail['parent']not in revisions:reject(8,'invalid_history_view',p(path,'revisions'))
    first=revisions[tail['parent']];ordered=[first,tail]
    if first['parent']is not None or len({r['id'] for r in ordered})!=2:reject(8,'invalid_graph',p(path,'revisions'))
    if branch['id']!=root:reject(9,'integrity_mismatch',path+'/branches/0/id')
    if branch['fork_state_digest']!=state_digest(genesis['state']):reject(10,'replay_mismatch',path+'/branches/0/fork_state_digest')
    if int(archive['accepted_sequence_high_water'])<max(int(r['ordinal']) for r in ordered):reject(8,'allocator_regression',p(path,'accepted_sequence_high_water'))
    validate_integrity(archive,'ARCHIVE',path)
    for index,r in enumerate(archive['revisions']):
        if r['id']!=H(revision_frame(root,r)):reject(9,'integrity_mismatch',path+'/revisions/'+str(index)+'/id')
    state=copy.deepcopy(genesis['state']);parent=None;views={(root,None,0):(copy.deepcopy(state),None)};issuances=[];previous=0
    for r in ordered:
        if r['mode']!='paused':raise Unsupported(path+' (running prefix not covered)')
        if r['branch']!=root or r['tick']!='0' or r['capture']!=snapshot(root,parent,state):reject(10,'replay_mismatch',p(path,'revisions'))
        seq=int(r['ordinal'])
        if seq<=previous or seq==0:reject(10,'replay_mismatch',p(path,'revisions'))
        ops=r['command']['operations']
        if len(ops)!=1:raise Unsupported(path+' (multi-operation not covered)')
        op=ops[0]
        if parent is None:
            if op['type']!='create_system':raise Unsupported(path+' (first operation not create_system)')
            frame=creator_frame(root,root,r['ordinal'],op['local']);ident=H(frame)[:32]
            # The captured empty root has no live/retained issuance; exactly one origin
            # is admitted, then retained unchanged by the edit. No general scan claimed.
            if state['systems'] or any(i['id']==ident for i in issuances):reject(10,'duplicate_identity',path)
            issuances.append(dict(id=ident,full_digest=H(frame),origin=(root,r['ordinal'],0,1,op['local'])))
            prior=None;row=dict(id=ident,name=op['name'])
        else:
            if op['type']!='edit_system' or op['target']['type']!='existing':raise Unsupported(path+' (second operation not existing edit_system)')
            if op['target']['id']!=state['systems'][0]['id']:reject(8,'missing_reference',p(path,'revisions'))
            prior=copy.deepcopy(state['systems'][0]);row=dict(id=prior['id'],name=op['name'])
        changes=[change(prior,row)]
        if r['command']['reviewed_effects']!=changes:reject(10,'replay_mismatch',p(path,'revisions'))
        state['systems']=[row];state['next_sequence']=r['ordinal']
        pub,_=receipt(state,r['id'],changes);parent=r['id'];previous=seq
        views[root,parent,0]=(copy.deepcopy(state),pub)
    state['tick']='1';pub,_=receipt(state,None,[]);views[root,parent,1]=(copy.deepcopy(state),pub)
    final_view=dict(branch=root,revision=parent,tick='1')
    if branch['tip_tick']!='1' or archive['active_view']!=final_view or archive['final_digest']!=state_digest(state):reject(10,'replay_mismatch',path)
    for i,a in enumerate(archive['anchors']):
        view=(a['branch'],a['revision'],int(a['tick']))
        if view not in views:reject(8,'invalid_history_view',path+'/anchors/'+str(i))
        target,actual_receipt=views[view]
        if actual_receipt is None or a!=anchor(root,a['revision'],target,actual_receipt):reject(10,'forged_evidence',path+'/anchors/'+str(i))
    return views


def replay_notebook(notebook,archive,genesis,views,path=''):
    embedded_header(notebook,'notebook',path)
    if notebook['catalog_hash']!=PACK:reject(8,'catalog_mismatch',p(path,'catalog_hash'))
    validate_integrity(notebook,'NOTEBOOK',path)
    if notebook['genesis_digest']!=H(D('GENESIS')+C(genesis)) or notebook['archive_integrity_sha256']!=archive['integrity_sha256']:reject(10,'replay_mismatch',path)
    if int(notebook['next_ordinal'])==0:reject(8,'allocator_regression',p(path,'next_ordinal'))
    for i,b in enumerate(notebook['bookmarks']):
        ordinal=int(b['ordinal']);view=(b['view']['branch'],b['view']['revision'],int(b['view']['tick']))
        if ordinal==0 or ordinal>=int(notebook['next_ordinal']):reject(8,'allocator_regression',path+'/bookmarks/'+str(i)+'/ordinal')
        if view not in views:reject(8,'invalid_history_view',path+'/bookmarks/'+str(i)+'/view')
        if b['digest']!=state_digest(views[view][0]):reject(10,'replay_mismatch',path+'/bookmarks/'+str(i)+'/digest')


def admit(raw,family,catalog,genesis,attachment=None):
    doc=parse(raw,CAPS[family]);header(doc,family);ordered=shape(doc,SCHEMAS[family],catalog=catalog)
    if C(ordered)!=raw:reject(6,'noncanonical_bytes','')
    if family=='archive':replay_archive(doc,genesis);return doc
    if family=='notebook':
        if attachment is None:raise Unsupported('Notebook requires exact supplied Archive attachment')
        views=replay_archive(attachment,genesis);replay_notebook(doc,attachment,genesis,views);return doc
    archive=doc['archive'];notebook=doc['notebook']
    embedded_header(archive,'archive','/archive');embedded_header(notebook,'notebook','/notebook')
    validate_integrity(doc,'TRANSFER')
    views=replay_archive(archive,genesis,'/archive');replay_notebook(notebook,archive,genesis,views,'/notebook')
    selected=doc['selected_view']
    if (selected['branch'],selected['revision'],int(selected['tick'])) not in views:reject(8,'invalid_history_view','/selected_view')
    return doc


def outcome(raw,family,catalog,genesis,attachment=None):
    try:admit(raw,family,catalog,genesis,attachment);return dict(admitted=True)
    except Reject as e:return dict(admitted=False,stage=e.stage,error=e.error,path=e.path)
    except Unsupported as e:return dict(coverage='unsupported_family_unexamined',detail=str(e),not_a_wire_error=True)

def mutated(doc,path,value=None,delete=False):
    result=copy.deepcopy(doc);cursor=result;parts=path.split('/')[1:]
    for key in parts[:-1]:cursor=cursor[int(key)] if isinstance(cursor,list) else cursor[key]
    key=int(parts[-1]) if isinstance(cursor,list) else parts[-1]
    if delete:del cursor[key]
    else:cursor[key]=value
    return result


def trace_matches(trace,archive,genesis):
                views=replay_archive(archive,genesis)
                if len(trace['publications']) != 3:return False
                for publication in trace['publications']:
                                view=publication['view'];key=(view['branch'],view['revision'],int(view['tick']))
                                if key not in views:return False
                                state,public_receipt=views[key]
                                if C(publication['state']) != C(state) or C(publication['receipt']) != C(public_receipt):return False
                return True


def products(catalog,genesis,sources):
    archive,notebook,transfer,trace=construct(catalog,genesis);cases=[]
    def add(name,family,doc,expectation,strength,attachment=None,raw=None):
        raw=C(doc) if raw is None else raw;actual=outcome(raw,family,catalog,genesis,attachment)
        if strength=='rejection_only':assert actual.get('admitted')is False,(name,actual)
        else:
            for key,value in expectation.items():assert actual.get(key)==value,(name,actual,expectation)
        cases.append(dict(id=name,entry_point=family,raw_base64=base64.b64encode(raw).decode(),expectation=expectation,
                                                                                strength=strength,attachment_archive_base64=None if attachment is None else base64.b64encode(C(attachment)).decode()))
    for family,doc in [('archive',archive),('notebook',notebook),('transfer',transfer)]:
        add('positive.'+family,family,doc,dict(admitted=True),'complete_history_admission',archive if family=='notebook' else None)
    alt_a,alt_n,alt_t,_=construct(catalog,genesis,'Alpha Ω','Beta β','Boundary Ω')
    for family,doc in [('archive',alt_a),('notebook',alt_n),('transfer',alt_t)]:
        add('positive.unicode.'+family,family,doc,dict(admitted=True),'complete_history_admission',alt_a if family=='notebook' else None)
    historical=copy.deepcopy(transfer);historical['selected_view']['tick']='0';reseal(historical,'TRANSFER')
    add('positive.transfer.retained_selection','transfer',historical,dict(admitted=True),'complete_history_admission')
    # Valid mark holes are permitted; do not require current marks == live maximum+1.
    holes=copy.deepcopy(notebook);holes['next_ordinal']='9';reseal(holes,'NOTEBOOK')
    add('positive.notebook.mark_holes','notebook',holes,dict(admitted=True),'complete_history_admission',archive)
    for family,doc in [('archive',archive),('notebook',notebook),('transfer',transfer)]:
        attach=archive if family=='notebook' else None
        def members(value,path=''):
            if isinstance(value,dict):
                for key,child in value.items():
                    target=p(path,key)
                    # Embedded complete immutable Catalog bytes are dependency, not this reader's Catalog matrix.
                    if family=='transfer' and target.startswith('/catalog/'):continue
                    stage=4 if target in ('/kind','/format_version','/rules_version') else 5
                    add('missing.'+family+target.replace('/','.'),family,mutated(doc,target,delete=True),
                                    dict(admitted=False,stage=stage,error='noncanonical_bytes',path=target),'exact_stage_error_path',attach)
                    members(child,target)
            elif isinstance(value,list):
                for i,child in enumerate(value):members(child,p(path,i))
        members(doc)
        for key,value,error in [('kind','OTHER','unsupported_kind'),('format_version',4,'unsupported_format_version'),('rules_version',4,'unsupported_rules_version')]:
            add('header.'+family+'.'+key,family,mutated(doc,'/'+key,value),dict(admitted=False,stage=4,error=error,path='/'+key),'exact_stage_error_path',attach)
        add('integrity.'+family,family,mutated(doc,'/integrity_sha256','ff'*32),dict(admitted=False,stage=9,error='integrity_mismatch',path='/integrity_sha256'),'exact_stage_error_path',attach)
        add('canonical.'+family,family,doc,dict(admitted=False,stage=6,error='noncanonical_bytes'),'exact_stage_error_only',attach,raw=b' '+C(doc))
    for name,path,value,error in [('name','/bookmarks/0/label',' Boundary','invalid_range'),('u64','/next_ordinal','02','noncanonical_bytes'),('digest','/bookmarks/0/digest','AA'*32,'noncanonical_bytes')]:
        add('scalar.'+name,'notebook',mutated(notebook,path,value),dict(admitted=False,stage=5,error=error,path=path),'exact_stage_error_path',archive)
    for family,doc,field in [('archive',archive,'catalog_hash'),('notebook',notebook,'catalog_hash')]:
        changed=mutated(doc,'/'+field,'01'*32);reseal(changed,family.upper())
        add('catalog.'+family,family,changed,dict(admitted=False,stage=8,error='catalog_mismatch',path='/'+field),'exact_stage_error_path',archive if family=='notebook' else None)
    add('sorted.revisions','archive',mutated(archive,'/revisions',list(reversed(archive['revisions']))),dict(admitted=False,stage=5,error='noncanonical_bytes',path='/revisions/1'),'exact_stage_error_path')
    add('sorted.anchors','archive',mutated(archive,'/anchors',list(reversed(archive['anchors']))),dict(admitted=False,stage=5,error='noncanonical_bytes',path='/anchors/1'),'exact_stage_error_path')
    # Reseal outer integrity and, where commands/captures change, Revision ID and all
    # reference bindings too. Thus replay faults cannot be hidden by stale hashes.
    for i,rev in enumerate(archive['revisions']):
        for suffix,value in [('capture/state_digest','ff'*32),('capture/permitted_snapshot_digest','ff'*32),
                                                                                            ('capture/phase',2),('capture/tick','1'),('command/reviewed_effects/0/passenger_delta','1'),
                                                                                            ('command/reviewed_effects/0/resources',[]),('command/reviewed_effects/0/operation_index',None)]:
            if suffix.endswith('resources'):continue
            changed=mutated(archive,'/revisions/'+str(i)+'/'+suffix,value);target=changed['revisions'][i];prior=target['id'];target['id']=H(revision_frame(trace['root_id'],target))
            for r in changed['revisions']:
                if r['parent']==prior:
                    r['parent']=target['id'];r['capture']['revision']=target['id'];r['id']=H(revision_frame(trace['root_id'],r))
            # Infer changed retained tail by ordinal; replace every supplied selected/anchor cursor.
            ids={r['ordinal']:r['id'] for r in changed['revisions']};oldids={r['id']:r['ordinal'] for r in archive['revisions']}
            changed['branches'][0]['head']=ids['2'];changed['active_view']['revision']=ids['2']
            for a in changed['anchors']:a['revision']=ids[oldids[a['revision']]]
            changed['revisions'].sort(key=lambda r:r['id']);changed['anchors'].sort(key=anchor_key);reseal(changed,'ARCHIVE')
            add('replay.revision.'+str(i)+'.'+suffix.replace('/','.'),'archive',changed,dict(admitted=False),'rejection_only')
    for path,value in [('/final_digest','ff'*32),('/accepted_sequence_high_water','1'),('/branches/0/fork_state_digest','ff'*32),
                                                                                ('/anchors/0/state_digest','ff'*32),('/anchors/0/receipt_digest','ff'*32),('/anchors/0/domain',2),
                                                                                ('/active_view/tick','0'),('/branches/0/tip_tick','0')]:
        changed=mutated(archive,path,value);changed['anchors'].sort(key=anchor_key);reseal(changed,'ARCHIVE')
        add('replay.archive'+path.replace('/','.'),'archive',changed,dict(admitted=False),'rejection_only')
    for path,value in [('/genesis_digest','ff'*32),('/archive_integrity_sha256','ff'*32),('/next_ordinal','1'),
                                                                                ('/bookmarks/0/ordinal','0'),('/bookmarks/0/digest','ff'*32),('/bookmarks/0/view/tick','2')]:
        changed=mutated(notebook,path,value);reseal(changed,'NOTEBOOK')
        add('replay.notebook'+path.replace('/','.'),'notebook',changed,dict(admitted=False),'rejection_only',archive)
    changed=mutated(transfer,'/selected_view/tick','2');reseal(changed,'TRANSFER')
    add('selected.transfer','transfer',changed,dict(admitted=False),'rejection_only')
    for family,key,value in [('archive','format_version',4),('archive','kind','OTHER'),('notebook','rules_version',4)]:
        changed=copy.deepcopy(transfer);changed[family][key]=value;reseal(changed[family],family.upper())
        if family=='archive':
            changed['notebook']['archive_integrity_sha256']=changed['archive']['integrity_sha256'];reseal(changed['notebook'],'NOTEBOOK')
        reseal(changed,'TRANSFER')
        add('embedded.header.'+family+'.'+key,'transfer',changed,dict(admitted=False),'rejection_only')
    # Public Receipt/Event bytes are harness replay outputs, not an invented portable transport field.
    harness=[]
    for index,pub in enumerate(trace['publications']):
        for suffix,value in [('receipt/state_digest','ff'*32),('receipt/digest','ff'*32),('state/next_sequence','0')]:
            altered=mutated(trace,'/publications/'+str(index)+'/'+suffix,value)
            assert trace_matches(trace,archive,genesis) and not trace_matches(altered,archive,genesis)
            harness.append(dict(id=str(index)+'.'+suffix.replace('/','.'),mutated_trace=altered,
                                                                                            expectation=dict(exact_replay_matches=False),strength='trace_binding_rejection_only'))
    if trace['publications'][0]['receipt']['events']:
        altered=mutated(trace,'/publications/0/receipt/events/0/id','ff'*16)
        assert not trace_matches(altered,archive,genesis)
        harness.append(dict(id='event_id',mutated_trace=altered,expectation=dict(exact_replay_matches=False),strength='trace_binding_rejection_only'))
    coverage=dict(status='Proposed HIST-C narrow empty-root admission; broader matrix Partial',source_pins=PINS,accepted_dependencies=DEPENDENCIES,source_files=sources,
            complete_positive_count=sum(c['expectation'].get('admitted')is True for c in cases),
            strengths={s:sum(c['strength']==s for c in cases) for s in sorted({c['strength'] for c in cases})},
            source_contracts={'captures':'schema2446-2458 pre-command paused phase1 D0/D1',
                'changes':'schema1723-1740 and2438-2442 complete direct snapshots; no header Changes',
                'frames':'schema2370-2423; CreatorEntity flag2, Revision full32, STATE wrapper and root',
                'publication':'schema2460-2485 one intervention/revision, no cascade, no duplicate operations',
                'receipt':'schema2329-2340; wire149-154 ID-free payload then Event IDs',
                'archive':'schema2575-2589/2630-2648/2692-2697 exact retained chronology/root/view/three receipt Anchors',
                'notebook':'schema2497-2511/2654-2659 common positive ordinals, state-digest Bookmark, exact Archive attachment',
                'transfer':'schema2614-2617 integrity plus exact inner inputs/selected view'},
            gaps=['Only empty Genesis root and two paused single-System creator operations followed by ordinary1 are qualified; other histories unsupported coverage, not wire errors.',
                'No inhabited Hub/autonomous receipt policy selected; T50 outside this empty family.',
                'Only one creator System issuance origin; no general bounded retained-history collision scanner/performance claim.',
                'No fork/running queue/multi-operation/cascade/removal/transient sibling history or unselected branch fixture.',
                'No general decoder multi-fault semantic/path matrix. Rejection-only cases do not export private helper ordering.',
                'Other Catalog metadata/Genesis profiles depend on their independent families; immutable exact core/empty inputs only here.',
                'Notebook experiments/evidence and event-specific Anchors are unexamined; no causal completion/durability assertion.',
                'No standalone VIEW domain, initial Receipt, portable queue, checkpoint, cache flag or receipt bytes in Archive.',
                'Bounded-depth Python materialization/schema recursion is not production poll/memory/cancellation/ownership evidence.'])
    coverage['declarations']={}
    def declarations(schema,path):
        if isinstance(schema,str):
            coverage['declarations'][path]=dict(primitive=schema)
        elif schema[0]=='record':
            coverage['declarations'][path]=dict(fields=list(schema[1]))
            for key,child in schema[1].items():declarations(child,p(path,key))
        elif schema[0]=='array':
            coverage['declarations'][path]=dict(cap=schema[2],key=schema[3],
                unexamined_children=schema[1]=='unsupported')
            declarations(schema[1],path+'/*')
        elif schema[0]=='option':declarations(schema[1],path+'?')
        elif schema[0]=='tagged':
            coverage['declarations'][path]=dict(supported_tags=list(schema[1]),other_tags_unexamined=True)
            for tag,child in schema[1].items():declarations(child,path+':'+tag)
        elif schema[0]=='enum':coverage['declarations'][path]=dict(closed_enum=list(schema[1]))
    for family,schema in SCHEMAS.items():declarations(schema,family)
    coverage['trace_binding_count']=len(harness)
    coverage['positive_predicates']=[
        'Immutable independently admitted core Catalog and exact empty Genesis; no initial Receipt.',
        'Pre-command paused phase1 captures and stale-check envelopes use exact completed parent/digest/empty queue.',
        'One creator System issuance flag2/sequence/op/kind/local; edit retains same live ID and origin.',
        'Exact full direct Changes with resources[]/passenger_delta0 and no orchestration header Change.',
        'Revision parent chronology authenticated independently of RevisionID-sorted portable array.',
        'State next_sequence updates after each operation; ordinary1 preserves branch0/next2 and has no authored revision/event.',
        'Three ID-free Receipt payloads sealed before Event IDs, no self-Receipt Anchor in state/Revision/payload.',
        'Full-key sorted exact whole-Receipt Anchors authenticate their original published views, not current head replacement.',
        'Bookmark State digest and common positive ordinal/mark plus exact attached Archive integrity authenticate.',
        'Transfer inner versions/Catalog/history/Notebook/selected retained view and all domain integrity links authenticate.'
    ]
    vectors=dict(status='Proposed HIST-C',cases=cases,trace_binding_cases=harness)
    pretty=lambda value:json.dumps(value,ensure_ascii=False,indent=2).encode()+b'\n'
    return {'archive.json':C(archive),'notebook.json':C(notebook),'transfer.json':C(transfer),
                                    'trace.json':pretty(trace),'vectors.json':pretty(vectors),'predicate-coverage.json':pretty(coverage)}


def main():
    args=argparse.ArgumentParser();args.add_argument('mode',choices=('generate','verify'));mode=args.parse_args().mode
    catalog,genesis,sources=freshness();output=products(catalog,genesis,sources);freshness()
    for name,raw in output.items():
        file=HERE/name
        if mode=='generate':file.write_bytes(raw)
        elif not file.exists() or file.read_bytes()!=raw:raise SystemExit('STOP: generated output differs '+name)
    print(mode+' passed: '+str(len(json.loads(output['vectors.json'])['cases']))+' raw cases')
    for name,raw in output.items():print(name,len(raw),H(raw))


if __name__=='__main__':main()
