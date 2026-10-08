"""Explicit typed State links, checked before initialization coverage classification."""

def collect(state, reference, slug):
    def one(row,path,fields):
        for key,kind in fields.items(): reference(row[key],kind,path+'/'+key)
    def terms(row,path):
        if row['type']=='resource_supply':one(row,path,dict(source_world=3,destination_world=3))
        else:one(row,path,dict(participant_a=5,participant_b=5))
    def observation(row,path):
        one(row,path,dict(owner=5))
        scope=row['environment']['stellar_scope'];sp=path+'/environment/stellar_scope'
        one(scope,sp,dict(system=1,originating_observer=5))
        for j,event in enumerate(scope['events']):
            ep=sp+'/events/'+str(j);one(event,ep,dict(id=18,star=2,system=1,originating_observer=5))
        for j,scope in enumerate(row['hazard_scopes']):
            sp=path+'/hazard_scopes/'+str(j);one(scope,sp,dict(lane=4,originating_observer=5))
            for k,event in enumerate(scope['events']):
                ep=sp+'/events/'+str(k);one(event,ep,dict(id=14,lane=4,originating_observer=5));slug(event['definition'],'hazard_definitions',ep+'/definition')
        for family,collection in [('facilities','industry_definitions'),('stationed_hulls','hull_definitions')]:
            for j,item in enumerate(row[family]):slug(item['definition'],collection,path+'/'+family+'/'+str(j)+'/definition')
        for j,t in enumerate(row['threats']):threat(t,path+'/threats/'+str(j))
    def threat(row,path):
        one(row,path,dict(enemy=5,target_world=3,originating_observer=5))
        if row['war'] is not None:one(row['war'],path+'/war',dict(participant_a=5,participant_b=5))
        for j,ident in enumerate(row['hostile_fleets']):reference(ident,10,path+'/hostile_fleets/'+str(j))
    for name in ('facilities','construction_jobs','hull_jobs'):
        for i,row in enumerate(state[name]):
            path='/state/'+name+'/'+str(i);one(row,path,dict(world=3))
            slug(row['definition'],'hull_definitions' if name=='hull_jobs' else 'industry_definitions',path+'/definition')
            if name!='facilities':one(row,path,dict(owner=5))
            if name=='hull_jobs':one(row,path,dict(target_fleet=10))
    for name in ('routes','shipments'):
        for i,row in enumerate(state[name]):
            path='/state/'+name+'/'+str(i);one(row,path,dict(source_world=3,destination_world=3))
            if name=='routes':one(row,path,dict(source_owner=5,receiver_owner=5,commitment=16))
            else:
                one(row,path,dict(dispatch_owner=5,intended_receiver=5))
                provenance=row['dispatch_provenance']
                if provenance['type']=='paid':
                    one(provenance,path+'/dispatch_provenance',dict(route=12))
                    if provenance['commitment'] is not None:one(provenance['commitment'],path+'/dispatch_provenance/commitment',dict(id=16))
                    slug(provenance['withdrawn']['resource'],'resources',path+'/dispatch_provenance/withdrawn/resource')
    for i,row in enumerate(state['relations']):one(row,'/state/relations/'+str(i),{'from':5,'to':5})
    for name in ('agreements','wars'):
        for i,row in enumerate(state[name]):
            path='/state/'+name+'/'+str(i);one(row,path,dict(participant_a=5,participant_b=5))
            if name=='wars':
                one(row,path,dict(declarer=5))
                for j,obj in enumerate(row['objectives']):
                    op=path+'/objectives/'+str(j);one(obj,op,dict(actor=5,target=4 if obj['kind']=='secure_access' else 3))
                    if obj['secure_endpoints'] is not None:one(obj['secure_endpoints'],op+'/secure_endpoints',dict(world_a=3,world_b=3))
                if row['peace'] is not None:one(row['peace'],path+'/peace',dict(proposer=5))
    for name,fields in [('hazards',dict(lane=4)),('settlement_claims',dict(world=3,civilization=5)),
                        ('occupations',dict(world=3,claimant=5)),('demand_requests',dict(actor=5,world=3))]:
        for i,row in enumerate(state[name]):
            path='/state/'+name+'/'+str(i);one(row,path,fields)
            if name=='hazards':slug(row['definition'],'hazard_definitions',path+'/definition')
    for name in ('offers','commitments'):
        for i,row in enumerate(state[name]):
            path='/state/'+name+'/'+str(i);terms(row['terms'],path+'/terms')
            if name=='offers':
                one(row,path,dict(proposer=5,recipient=5,renewal_of=16))
                if row['demand'] is not None:one(row['demand'],path+'/demand',dict(actor=5,world=3))
            else:
                one(row['parties'],path+'/parties',dict(participant_a=5,participant_b=5))
                one(row['obligation_status'],path+'/obligation_status',dict(route=12,request=17))
    for i,row in enumerate(state['defense_requests']):
        path='/state/defense_requests/'+str(i);one(row,path,dict(commitment=16,requester=5,defender=5,target_world=3,originating_observer=5))
        observation(row['observation'],path+'/observation');threat(row['threat'],path+'/threat')
    for i,row in enumerate(state['shared_intelligence']):
        path='/state/shared_intelligence/'+str(i);one(row,path,dict(recipient=5,originating_actor=5,world=3));observation(row['observation'],path+'/observation')
    for i,row in enumerate(state['stellar_events']):one(row,'/state/stellar_events/'+str(i),dict(star=2))
    for i,row in enumerate(state['observations']):observation(row,'/state/observations/'+str(i))
