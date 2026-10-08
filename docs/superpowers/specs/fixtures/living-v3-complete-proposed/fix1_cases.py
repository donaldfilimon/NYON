"""Written diagnostic controls and F1/F2/F3/F4 regression literals, round1."""
import base64
import copy
import catalog as c
import genesis as g
import profiles


def derive(catalog):
    cases=[]
    def check(name,raw,expected,source,strength='exact_declared_diagnostic'):
        actual=g.outcome(raw,catalog)
        for key,value in expected.items():assert actual.get(key)==value,(name,expected,actual)
        cases.append(dict(id=name,input_base64=base64.b64encode(raw).decode(),expected=expected,
                          source='2026-10-01-living-v3-complete-schema.md:'+source,strength=strength,
                          derivation='fix1_cases.derive',independent_review='pending'))
    base=profiles.build(catalog=catalog)
    wrong=base['state']['worlds'][0]['id']
    bad=copy.deepcopy(base);bad['state']['stars'][0]['system']=wrong;bad['state']['worlds'][0]['system']='ff'*16
    check('F1.reviewer_missing_before_wrong',c.encoded(bad),dict(admitted=False,stage=8,error='missing_reference',path='/state/worlds/0/system'),'4211')
    bad=copy.deepcopy(base);bad['state']['stars'][0]['system']='ff'*16;bad['state']['worlds'][0]['system']='ee'*16
    check('F1.same_family_schema_order',c.encoded(bad),dict(admitted=False,stage=8,error='missing_reference',path='/state/stars/0/system'),'4211-4222')
    for family,value in [('missing_reference','ff'*16),('wrong_reference_kind',wrong)]:
        bad=profiles.build(catalog=catalog,stars=12)
        for index in (10,2):bad['state']['stars'][index]['system']=value
        check('F1.numeric_'+family,c.encoded(bad),dict(admitted=False,stage=8,error=family,path='/state/stars/2/system'),'4211-4222')
    check('F3.invalid_utf8_second_byte',b' \xff',dict(admitted=False,stage=1,error='noncanonical_bytes',path='',offset=1),'4204')
    check('F3.syntax_multibyte_offset','["Ω",!]'.encode(),dict(admitted=False,stage=2,error='noncanonical_bytes',path='',offset=6),'4205')
    check('F3.depth65',b'['*65+b']'*65,dict(admitted=False,stage=2,error='depth_capacity',path='',offset=64),'4205')
    raw=c.encoded(base);name=base['state']['systems'][0]['name']
    # A literal H is replaced by its alternative JSON escape spelling. The
    # independently located raw token gives the expected first-difference offset.
    token=('"name":"'+name+'"').encode();at=raw.index(token)+len(b'"name":"')
    altered=raw[:at]+b'\\u0048'+raw[at+1:]
    check('F3.nested_name_alternate_escape',altered,dict(admitted=False,stage=6,error='noncanonical_bytes',path='/state/systems/0/name',offset=at),'4209;4221-4222')
    altered=b' '+raw
    check('F3.leading_whitespace',altered,dict(admitted=False,stage=6,error='noncanonical_bytes',path='',offset=0),'4209')
    bad=g.mutation(base,'/state/worlds',[None]*513)
    check('F3.record_cap',c.encoded(bad),dict(admitted=False,stage=5,error='record_capacity',path='/state/worlds',actual=513,limit=512),'4208')
    # Cap byte constructor avoids duplicating an8MiB malformed literal in JSON.
    actual=g.outcome(b'\xff'*(g.CAP+1),catalog)
    expected=dict(admitted=False,stage=0,error='byte_capacity',path='',actual=g.CAP+1,limit=g.CAP)
    assert actual==expected
    cases.append(dict(id='F3.raw_byte_cap',input_constructor=dict(byte_hex='ff',count=g.CAP+1),expected=expected,source='2026-10-01-living-v3-complete-schema.md:4203',strength='exact_declared_diagnostic'))
    reason=dict(type='needs_reserved',world='00'*16,resource='energy',physical_stock=10,held=10,required=1)
    def structural(name,value,expected):
        original=copy.deepcopy(value)
        try:
            parsed=g.check(c.Parser(c.encoded(value).decode()).parse(),g.BLOCKED)
            actual=dict(structurally_valid=True)
        except g.Rejected as err:actual=dict(structurally_valid=False,stage=err.stage,error=err.error,path=err.path,**err.details)
        for key,value in expected.items():assert actual.get(key)==value,(name,expected,actual)
        cases.append(dict(id=name,value=original,expected=expected,source='2026-10-01-living-v3-complete-schema.md:4645-4650',strength='structural_only_not_initial_admission'))
    # No generated observation or producer is assumed by these independent rows.
    for name,candidate,expected in [('F2.positive',reason,dict(structurally_valid=True))]:structural(name,candidate,expected)
    for key in reason:
        candidate=copy.deepcopy(reason);del candidate[key]
        structural('F2.missing_'+key,candidate,dict(structurally_valid=False,stage=5,error='noncanonical_bytes',path='/'+key))
    candidate=dict(reason,extra=0);structural('F2.unknown',candidate,dict(structurally_valid=False,stage=5,error='unknown_field',path='/extra'))
    for key in ('physical_stock','held','required'):
        candidate=dict(reason,**{key:True});structural('F2.bool_'+key,candidate,dict(structurally_valid=False,stage=5,error='invalid_range',path='/'+key))
        candidate=dict(reason,**{key:2**32});structural('F2.width_'+key,candidate,dict(structurally_valid=False,stage=5,error='invalid_range',path='/'+key))
    candidate=dict(reason,resource='unknown');structural('F2.resource_tag',candidate,dict(structurally_valid=False,stage=5,error='invalid_enum',path='/resource'))
    g.needs_reserved_semantics(reason)
    for name,candidate,path in [('stock_domain',dict(reason,physical_stock=10001),'/physical_stock'),('held_gt_stock',dict(reason,held=11),'/held'),('not_reserved',dict(reason,held=0),'/required')]:
        try:g.needs_reserved_semantics(candidate)
        except g.Rejected as err:assert (err.stage,err.error,err.path)==(7,'invalid_range',path)
        else:raise AssertionError(name)
        cases.append(dict(id='F2.semantic_'+name,value=candidate,expected=dict(stage=7,error='invalid_range',path=path),source='2026-10-01-living-v3-complete-schema.md:4645-4650',strength='local_semantic_only_not_authenticated_guard'))
    bad=g.mutation(base,'/state/colonies/0/hub/blocked_reasons',[dict(reason,world=base['state']['colonies'][0]['world'])])
    check('F2.no_bootstrap_prior_attempt',c.encoded(bad),dict(admitted=False),'4104','rejection_only')
    return cases
