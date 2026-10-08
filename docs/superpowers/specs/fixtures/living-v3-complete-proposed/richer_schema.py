"""Written State row grammar. Successful richer initialization remains unadopted.

Authority: corrected complete-schema194–509,984–1224,3399–3416,3620–3634,
3730–3740. No runtime records or constructor defaults are consulted.
"""

def declare(g):
    rec,array,enum,opt=g['rec'],g['array'],g['enum'],g['opt']
    resource=enum('alloy','energy','ore','food')
    reason=enum('capacity_unavailable','own_needs_reserve','no_supply','endpoint_changed','war_incompatible',
        'no_eligible_fleet','prior_obligation','request_expired','target_invalid','relations_below_threshold',
        'no_connection','no_known_demand','expired','recipient_refused','resource_reserved','missing_intelligence',
        'stale_intelligence','declaration_required')
    event_kind=enum(*('agreement_expired agreement_formed civilization_dormant civilization_reactivated colony_captured combat_round construction_cancelled construction_completed creator_intervention facility_blocked facility_produced fleet_arrived fleet_departed hazard_ended hazard_started hull_completed hull_destroyed refund_discarded settlement_claim_opened settlement_claim_resolved shipment_captured shipment_delivered shipment_dispatched shipment_returned shipment_waiting war_declared war_ended needs_paid needs_shortfall population_changed workforce_changed passengers_boarded passengers_unloaded passengers_lost research_progress technology_completed technology_revoked research_target_changed environment_updated stellar_warning stellar_started stellar_ended environment_capacity_omission demand_posted demand_expired request_capacity offer_created offer_considered offer_refused offer_expired commitment_created commitment_renewed commitment_ended obligation_changed defense_requested defense_request_ended intelligence_shared objective_changed peace_proposed peace_refused fleet_supplied fleet_supply_unavailable blockade_restriction intent_decided operation_changed route_attempted creator_cascade candidate_audit defender_objective_audit war_admission_audit needs_reserved declaration_hold retreat_blocked fleet_dispatch_blocked dispatch_reservation_blocked defense_request_attempted defense_request_considered defense_action_recorded'.split()))
    anchor=rec(domain='u32',catalog_hash='Digest',branch='ID',revision=opt('Digest'),tick='u64',state_digest='Digest',
        receipt_digest='Digest',event=opt(rec(phase='u16',ordinal='u32',id='ID',kind=event_kind)))
    snapshot=rec(domain='u32',catalog_hash='Digest',branch='ID',revision=opt('Digest'),tick='u64',state_digest='Digest',phase='u16',permitted_snapshot_digest='Digest')
    capture=rec(accepted_sequence='u64',operation_index='u32',application_tick='u64',mode=enum('paused','running'),snapshot=snapshot)
    amount=rec(resource='Slug',quantity='u32')
    amounts=array(amount,4,('resource',))
    recipe=rec(evaluated_tick='u64',staffed='bool',inputs=amounts,outputs=amounts,fertility_spent='u32',stress_added='u32',blocked=array(g['BLOCKED'],None,('@reason',)))
    facility=rec(id='ID',world='ID',definition='Slug',status=enum('operational','blocked'),paid_alloy='u32',hit_points='u32',
        next_due=opt('u64'),next_repair=opt('u64'),blocked_reasons=array(g['BLOCKED'],None,('@reason',)),disabled='bool',recipe_facts=opt(recipe))
    job=rec(id='ID',world='ID',owner='ID',definition='Slug',accepted_tick='u64',completion_tick='u64',paid_alloy='u32')
    hull_job=rec(**job[1],target_fleet=opt('ID'))
    route_kind=enum('aid','internal','trade')
    route=rec(id='ID',kind=route_kind,source_world='ID',destination_world='ID',resource=resource,batch_size='u32',cadence_ticks='u32',
        source_reserve='u32',source_owner='ID',receiver_owner='ID',next_due='u64',suspended='bool',commitment=opt('ID'))
    manifest=('tag',{'paid':rec(type='tag',snapshot=snapshot,route='ID',commitment=opt(rec(id='ID',created_tick='u64',terms_digest='Digest')),withdrawn=amount),
        'creator':rec(type='tag',capture=capture)})
    shipment=rec(id='ID',dispatch_owner='ID',intended_receiver='ID',source_world='ID',destination_world='ID',resource=resource,
        units='u32',departure_tick='u64',arrival_tick='u64',original_travel_ticks='u64',disposition=enum('outbound','returning','waiting_for_storage'),
        dispatch_kind=opt(route_kind),returning_to_source='bool',dispatch_provenance=manifest,creator_disposition_override=opt(capture))
    relation=rec(**{'from':'ID','to':'ID','base_disposition':'i32','reasons':rec(delivered_aid='i32',successful_trade='i32',
        adjacent_rival_settlement='i32',war_declared_against_it='i32',colony_captured_from_it='i32',aid_units_remainder='u32',trade_units_remainder='u32')})
    agreement=rec(kind=enum('nonaggression','trade','truce'),participant_a='ID',participant_b='ID',start_tick='u64',end_tick='u64')
    objective=rec(actor='ID',kind=enum('defend','secure_access','occupy'),target='ID',secure_endpoints=opt(rec(world_a='ID',world_b='ID')),
        consecutive_witness_count='u32',status=enum('pending','succeeded','invalid'),last_evaluated_tick=opt('u64'),witness=opt(anchor))
    peace=rec(proposer='ID',created_tick='u64',considered_tick=opt('u64'),status=enum('proposed','refused'),
        reason=opt(enum('objective_complete','objective_invalid','force_unachievable','war_expired','objective_still_achievable')))
    war=rec(participant_a='ID',participant_b='ID',declarer='ID',start_tick='u64',end_tick='u64',objectives=array(objective,None,('actor',)),peace=opt(peace))
    hazard=rec(id='ID',definition='Slug',lane='ID',warning_tick='u64',start_tick='u64',end_tick='u64')
    claim=rec(world='ID',civilization='ID',arrival_tick='u64',close_tick='u64')
    occupation=rec(world='ID',claimant='ID',progress_ticks='u32')
    terms=('tag',{'transit_access':rec(type='tag',participant_a='ID',participant_b='ID',duration_ticks='u32'),
        'resource_supply':rec(type='tag',source_world='ID',destination_world='ID',resource=resource,batch='u32',cadence_ticks='u32',source_reserve='u32',duration_ticks='u32'),
        'mutual_defense':rec(type='tag',participant_a='ID',participant_b='ID',duration_ticks='u32')})
    treaty=enum('transit_access','resource_supply','mutual_defense')
    demandkey=rec(actor='ID',world='ID',resource=resource)
    offer=rec(id='ID',proposer='ID',recipient='ID',kind=treaty,terms=terms,created_tick='u64',expires_tick='u64',demand=opt(demandkey),
        renewal_of=opt('ID'),last_consideration=opt(rec(tick='u64',disposition=enum('waiting','accepted','refused'),reason=opt(reason))))
    demand=rec(**demandkey[1],units='u32',tick='u64',expires_tick='u64',snapshot_digest='Digest')
    obligation=rec(status=enum('ready','fulfilled','waiting','suspended'),last_attempt_tick=opt('u64'),route=opt('ID'),request=opt('ID'),
        last_fulfillment=opt(anchor),reasons=array(reason,None,()))
    commitment=rec(id='ID',parties=g['PAIR'],kind=treaty,terms=terms,created_tick='u64',expires_tick='u64',obligation_status=obligation)
    facts=rec(**{k:v for k,v in g['OBS'][1].items() if k not in ('civilization','world','observed_tick')})
    defense=rec(id='ID',commitment='ID',requester='ID',defender='ID',target_world='ID',created_tick='u64',expires_tick='u64',
        disposition=enum('pending','accepted','refused'),originating_observer='ID',observed_tick='u64',observation=facts,threat=g['THREAT'])
    shared=rec(recipient='ID',originating_actor='ID',world='ID',observed_tick='u64',received_tick='u64',observation=facts)
    provenance=('tag',{'genesis':rec(type='tag',generator=rec(generator_id='Slug',generator_version='u32'),kind='u16',ordinal='u64'),
        'creator':rec(type='tag',capture=capture),
        'autonomous':rec(type='tag',snapshot=snapshot,phase='u16',actor=opt('ID'),subject_kind='u16',subject='ID',allocation_ordinal='u32')})
    stellar=rec(id='ID',star='ID',warning_tick='u64',start_tick='u64',end_tick='u64',habitability_impact=g['LOCAL'],freight_factor=g['FRACTION'],provenance=provenance)
    return dict(facilities=facility,construction_jobs=job,hull_jobs=hull_job,routes=route,shipments=shipment,
        relations=relation,agreements=agreement,wars=war,hazards=hazard,settlement_claims=claim,occupations=occupation,
        offers=offer,demand_requests=demand,commitments=commitment,defense_requests=defense,shared_intelligence=shared,stellar_events=stellar)
