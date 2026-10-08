"""Typed foundation decomposition, separate from pending whole-package text index."""
import copy
import catalog as c
import genesis as g
SCHEMA='2026-10-01-living-v3-complete-schema.md'

def sample(schema):
    if isinstance(schema,str):
        if schema=='unsupported':raise LookupError('unimplemented richer record')
        return {'u16':0,'u32':0,'u64':'0','i32':'0','ID':'00'*16,'Digest':'00'*32,'Slug':'a','Name':'N','tag':'sample','bool':False}[schema]
    tag=schema[0]
    if tag=='bounded':return schema[2]
    if tag=='record':return {k:sample(v) for k,v in schema[1].items()}
    if tag=='tag':
        name=next(iter(schema[1]));value=sample(schema[1][name]);value['type']=name;return value
    if tag=='enum':return schema[1][0]
    if tag=='array':return []
    return None

def matrix():
    cases=[];rows=[]
    def execute(case_id,schema,value,expect):
        try:
            g.check(c.Parser(c.encoded(value).decode()).parse(),schema)
            got={'structurally_valid':True}
        except g.Rejected as err:
            got={'structurally_valid':False,'stage':err.stage,'error':err.error,'path':err.path}
        for k,v in expect.items():assert got.get(k)==v,(case_id,expect,got)
        cases.append(dict(id=case_id,value=value,expected=expect,strength='structural_only_not_full_document_admission'))
    def visit(schema,path):
        row=dict(id=path or '/',source=SCHEMA+':8-88;194-509;984-1224;1736-1835;3961-4008;4197-4216;4645-4650',
                 declaration=schema,derivation='genesis.check',case_ids=[],independent_review='pending')
        rows.append(row)
        if schema=='unsupported':
            row['status']='pending_unimplemented_richer_record_shape';return
        base=sample(schema)
        def test(suffix,value,expect):
            name=(path or '/')+':'+suffix
            execute(name,schema,value,expect);row['case_ids'].append(name)
        test('positive',base,{'structurally_valid':True})
        row['status']='structural_cases_derived_unreviewed'
        if isinstance(schema,str):
            test('wrong_container',{},dict(structurally_valid=False,stage=5,error='invalid_range'))
            return
        kind=schema[0]
        if kind=='bounded':
            for n in (schema[2],schema[3]):test('endpoint_'+str(n),n,dict(structurally_valid=True))
            test('upper_plus_one',schema[3]+1,dict(structurally_valid=False,stage=5,error='invalid_range'))
        elif kind=='record':
            test('unknown',dict(base,a_unknown=0),dict(structurally_valid=False,stage=5,error='unknown_field',path='/a_unknown'))
            for key,child in schema[1].items():
                bad=copy.deepcopy(base);del bad[key]
                test('missing_'+key,bad,dict(structurally_valid=False,stage=5,error='noncanonical_bytes',path='/'+key))
                visit(child,path+'/'+key)
        elif kind=='enum':
            for value in schema[1]:test('variant_'+value,value,dict(structurally_valid=True))
            test('unknown_variant','unknown_variant',dict(structurally_valid=False,stage=5,error='invalid_enum'))
        elif kind=='tag':
            for name,child in schema[1].items():
                value=sample(child);value['type']=name
                test('variant_'+name,value,dict(structurally_valid=True))
                visit(child,path+'<'+name+'>')
            test('unknown_variant',{'type':'unknown_variant'},dict(structurally_valid=False,stage=5,error='invalid_enum',path='/type'))
            test('unknown_member_precedes_tag',{'type':'unknown_variant','a_unknown':0},dict(structurally_valid=False,stage=5,error='unknown_field',path='/a_unknown'))
        elif kind=='option':
            visit(schema[1],path+'?')
            try:test('nonnull',sample(schema[1]),dict(structurally_valid=True))
            except LookupError:row['nonnull']='pending_richer_shape'
        elif kind=='array':
            row['cap']=schema[2];row['key']=schema[3];row['issued_identity_deferred']=schema[4]
            if schema[2] is not None:test('cap_plus_one',[None]*(schema[2]+1),dict(structurally_valid=False,stage=5,error='record_capacity'))
            visit(schema[1],path+'/*')
    visit(g.GENESIS,'')
    semantic=[dict(id=name,source=SCHEMA+':'+ref,derivation=location,status='implemented_unreviewed',
                   positive='all complete positive profiles with applicable source rows',negative_selector=negative,
                   expected_strength=strength,independent_review='pending') for name,ref,location,negative,strength in [
      ('installed_global_identity','4208-4210','genesis.installed/validate','identity.nested_duplicate','exact_stage_error'),
      ('global_hull_cap2048','46','genesis.validate','cap.global_hulls','exact_stage_error_path'),
      ('allocation_kind_positive_ordinal_sorted_bijection','3742-3772;4101','genesis.validate/allocation_frame','semantic / key.allocation_duplicate','mixed_exact_and_rejection_only'),
      ('zero_initial_counters','4102','genesis.validate','semantic.0 through semantic.2','rejection_only'),
      ('initial_stellar_clock','4102','genesis.validate','semantic.3','rejection_only'),
      ('inventory_environment_population_domains','1113-1120;4102','genesis.check/validate','semantic local domain and predicate upper_plus_one cases','exact_structural_domain'),
      ('fertility_not_above_maximum','984-997;4102','genesis.validate','semantic maximum_fertility9999','rejection_only'),
      ('status_owned_colony_iff_active','4103','genesis.validate','semantic civilization status','rejection_only'),
      ('empty_initial_research','4103','genesis.validate','semantic research target/progress/completed','rejection_only'),
      ('civilization_nonzero','3757-3760','genesis.validate','initial.zero_civilization_namespace','rejection_only'),
      ('colony_world_owner_matches','4100-4103','genesis.validate','initial.owner_mismatch','rejection_only'),
      ('initial_hub_clocks_and_no_attempt','4102-4104','genesis.validate','semantic hub clocks/blocked','rejection_only'),
      ('initial_society_clock_null_needs_empty_assignments','4102-4104','genesis.validate','semantic population/clock/workforce; initial.prior_needs','partial'),
      ('lane_distinct_endpoints','4101','genesis.validate','initial.self_loop','rejection_only'),
      ('nonempty_docked_scout_fleet','4088-4104','genesis.validate','semantic hulls empty; pending travelling classification','partial'),
      ('full_hp_no_credit_repair_passenger_origin','4104;1126-1129','genesis.validate','semantic hull mutations','rejection_only'),
      ('null_initial_supply_order_blockade','4104','genesis.validate','semantic readiness; initial.prior_supply_payment/attempt/order/blockade','partial'),
      ('catalog_exact_link','1689-1691;4211','genesis.validate','catalog_mismatch / positive.metadata_catalog_link','exact_stage_error_path'),
      ('typed_base_catalog_references','4101;4211','genesis.validate','missing_reference / wrong_reference_kind / catalog_reference','exact_stage_error'),
      ('derived_observation_complete_projection','4105-4124','genesis.project/validate','projection / semantic observations empty','rejection_only'),
      ('no_observation_self_authorization','4111-4118','genesis.visibility','visibility leaf matrix / visibility.self_authorizing_remote','rejection_only'),
      ('observation_deposit_checked_sum','8-34;4106','genesis.project/validate','projection.deposit_sum_overflow','rejection_only'),
      ('framed_integrity','3742-3822;4212','genesis.allocation_frame/validate','integrity seed/generator changes','exact_stage_error'),
      ('state_genesis_root_sealing','3742-3822','genesis.sealed','root-binding-cases.json','rejection_only')]]
    return dict(status='Partial typed matrix; pending rows are not closed',structural_predicates=rows,
                semantic_predicates=semantic,cases=cases,
                later_full_package_inventory='full-package-coverage.json; no whole-game completeness claim')
