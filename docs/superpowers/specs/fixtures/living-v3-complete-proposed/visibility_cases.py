"""Source-derived B5 predicate controls, explicitly not broader profile admission."""
import copy
import genesis as g
import profiles

def derive(catalog):
    base=profiles.build(catalog=catalog,systems=1,stars=1,worlds=2,lanes=0,colonies=0)
    state=base['state'];actor=state['civilizations'][0]['id']
    dock=state['fleets'][0]['location']['world'];target=next(w for w in state['worlds'] if w['id']!=dock)
    cases=[]
    def check(name,state,world,expected):
        assert g.visibility(state,catalog,actor,world)==expected
        cases.append(dict(id=name,actor=actor,world=world,state_operands=state,expected_visible=expected,
                          strength='visibility_predicate_only_not_richer_genesis_admission',
                          source='2026-10-01-living-v3-complete-schema.md:4109-4124'))
    no_assets=copy.deepcopy(state);no_assets['fleets']=[]
    check('no_source',no_assets,target,False)
    owned=copy.deepcopy(no_assets);world=next(w for w in owned['worlds'] if w['id']==target['id']);world['owner']=actor
    check('owned_world_without_colony',owned,world,True)
    colony=copy.deepcopy(no_assets)
    colony['colonies']=[dict(world=dock,owner=actor)]
    check('own_colony_reveals_system',colony,target,True)
    check('docked_scout_reveals_system',state,target,True)
    mixed=copy.deepcopy(state);mixed['fleets'][0]['hulls'].append(dict(id='ff'*16,definition='escort'))
    check('mixed_scout_reveals_system',mixed,target,True)
    nonscout=copy.deepcopy(state);nonscout['fleets'][0]['hulls'][0]['definition']='escort'
    check('non_scout_other_world_hidden',nonscout,target,False)
    check('non_scout_docked_world_visible',nonscout,next(w for w in state['worlds'] if w['id']==dock),True)
    travelling=copy.deepcopy(state);travelling['fleets'][0]['location']=dict(type='travelling',origin_world=dock,destination_world=target['id'],departure_tick='0',arrival_tick='1')
    check('travelling_scout_no_sight',travelling,target,False)
    forged=copy.deepcopy(no_assets);forged['observations']=[dict(civilization=actor,world=target['id'])]
    check('submitted_observation_no_authority',forged,target,False)
    return cases
