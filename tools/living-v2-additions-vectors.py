#!/usr/bin/env python3
"""Independent formulas for approved-scope Living additive wire shapes.

Transcribed from 2026-10-01-living-v2-wire-additions-proposal.md, not Rust.
Existing tools/living-v2-vectors.py and its corpus are never rewritten.
Usage: verify [decision.json], verify-creator, verify-genesis-forks, or
verify-observations, verify-decision-scrap, or verify-notebook or verify-shipments; print variants
output the independently computed bytes. Verification checks exact full bytes and
new domain-separated integrity; it is independent from the Rust implementation.
"""
import hashlib
import json
import pathlib
import sys

DOMAIN = b"NYON-LIVING-DECISION-RECEIPT-V2\0"
DEFAULT = pathlib.Path(__file__).resolve().parent.parent / "crates/nyon-workshop-core/tests/fixtures/living-v2-additions/decision.json"

def canonical(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")

def vector():
    ordinary = {"tick": 50, "applied_revisions": [], "event_payloads": [], "state_digest": "44" * 32}
    ordinary_digest = hashlib.sha256(b"NYON-LIVING-RECEIPT-V2\0" + canonical(ordinary)).hexdigest()
    payload = {
        "kind": "NYON_LIVING_DECISION_RECEIPT", "format_version": 2, "rules_version": 2,
        "catalog_hash": "11" * 32, "branch": "22" * 16, "tick": 50,
        "committed_state_before": "33" * 32, "state_after": "44" * 32,
        "receipt_digest": ordinary_digest,
        "intents": [{
            "phase": 8, "actor": "66" * 16, "ordinal": 0,
            "action": {"type": "construction", "world": "77" * 16, "definition": "foundry"},
            "target": "77" * 16, "score": "-10", "tier": "ordinary",
            "snapshot_tick": 50, "snapshot_digest": "88" * 32, "observed_tick": 49,
            "outcome": "rejected", "reasons": [{"type": "insufficient_resource",
                "world": "77" * 16, "resource": "alloy", "required": 20, "available": 10, "reserve": 0}]
        }]
    }
    digest = hashlib.sha256(DOMAIN + canonical(payload)).hexdigest()
    return canonical(dict(payload, integrity_sha256=digest))

def decision_scrap_vector():
    payload=json.loads(vector())
    del payload["integrity_sha256"]
    payload["intents"][0]["action"]["scrap"]="99"*16
    digest=hashlib.sha256(DOMAIN+canonical(payload)).hexdigest()
    return canonical(dict(payload,integrity_sha256=digest))

def decision_route_vector(adjust=False):
    payload = json.loads(vector())
    del payload["integrity_sha256"]
    intent = payload["intents"][0]
    intent["action"] = {"type":"route", "source":"99"*16, "destination":"77"*16, "resource":"ore"}
    if adjust:
        intent["action"]["existing_route"] = "aa"*16
    intent["score"] = "70"
    intent["observed_tick"] = None
    intent["outcome"] = "accepted"
    intent["reasons"] = [{"type":"accepted", "rule":"useful_scheduled_supply"}]
    digest = hashlib.sha256(DOMAIN + canonical(payload)).hexdigest()
    return canonical(dict(payload, integrity_sha256=digest))

def creator_vector():
    ref = lambda byte: {"type": "existing", "id": byte * 16}
    target = ref("11")
    operations = [
      {"type":"edit_system","target":target,"name":"System"},
      {"type":"edit_star","target":target,"system":ref("22"),"archetype":"yellow_star","name":"Star"},
      {"type":"edit_world","target":target,"system":ref("22"),"archetype":"temperate","name":"World"},
      {"type":"edit_lane","target":target,"system_a":ref("22"),"system_b":ref("33"),"distance_units":100},
      {"type":"edit_deposit","target":target,"resource":"ore","remaining_units":1000},
      {"type":"edit_civilization","target":target,"name":"Civilization","policy":"neutral"},
      {"type":"set_facility_enabled","target":target,"enabled":False},
      {"type":"scrap_facility","target":target,"cascade":{"type":"reject_if_referenced"}},
      {"type":"cancel_construction_job","target":target},
      {"type":"cancel_hull_job","target":target},
      {"type":"edit_fleet","target":target,"owner":ref("22"),"world":ref("33")},
      {"type":"edit_hull","target":target,"definition":"scout","hit_points":1,"return_credit":True},
      {"type":"split_fleet","local":0,"source":target,"hulls":[ref("44")]},
      {"type":"merge_fleets","target":target,"source":ref("22")},
      {"type":"edit_route","target":target,"kind":"internal","source_world":ref("22"),"destination_world":ref("33"),"resource":"ore","batch_size":10,"cadence_ticks":10,"source_reserve":20,"source_owner":ref("44"),"receiver_owner":ref("44")},
      {"type":"set_route_suspended","target":target,"suspended":False},
      {"type":"edit_hazard","target":target,"lane":ref("22"),"definition":"ion_storm","start_tick":50,"end_tick":100},
      {"type":"cancel_hazard","target":target},
      {"type":"remove_entity","target":target,"cascade":{"type":"remove_reviewed","dependents":[],"derived":[{"type":"relation","from":"11"*16,"to":"22"*16}]}}
    ]
    return canonical({"type":"creator_batch","operations":operations})

def creator_revision(data):
    # Existing section-10 revision framing, applied to new command bytes.
    framed = b"NYON-LIVING-REVISION-V2\0" + (2).to_bytes(4,"little") + bytes.fromhex("11"*32) + bytes(32) + b"\0" + (50).to_bytes(8,"little") + (1).to_bytes(8,"little") + len(data).to_bytes(8,"little") + data
    return hashlib.sha256(framed).hexdigest()

def genesis_forks_vector():
    rows=[]
    for tick,ordinal in [(0,1),(50,2),(0,18446744073709551615),(18446744073709551615,1)]:
        parent="11"*16;genesis="22"*32
        digest=hashlib.sha256(b"NYON-LIVING-GENESIS-FORK-V2\0"+bytes.fromhex(parent)+bytes.fromhex(genesis)+tick.to_bytes(8,"little")+ordinal.to_bytes(8,"little")).hexdigest()
        rows.append({"parent":parent,"genesis_digest":genesis,"tick":tick,"ordinal":ordinal,"digest":digest,"branch_id":digest[:32]})
    return canonical(rows)

def observations_vector():
    # Approved sparse field extends the existing observation sequence at the end.
    legacy={"civilization":"11"*16,"world":"22"*16,"observed_tick":50,
            "owner":"33"*16,"remaining_deposit":1000,
            "facilities":[{"definition":"defense_battery","count":2}],"stationed_hulls":[]}
    rows=[]
    for enabled in [None,0,1]:
        payload=dict(legacy)
        if enabled is not None:
            payload["operational_defense_batteries"]=enabled
        rows.append({"observation":payload,"sha256":hashlib.sha256(canonical(payload)).hexdigest()})
    return canonical(rows)

def shipments_vector():
    legacy={"id":"11"*16,"dispatch_owner":"22"*16,"intended_receiver":"33"*16,
            "source_world":"44"*16,"destination_world":"55"*16,"resource":"ore","units":100,
            "departure_tick":50,"arrival_tick":60,"disposition":"outbound"}
    rows=[]
    for kind in [None,"aid","trade"]:
        payload=dict(legacy)
        if kind is not None:
            payload["dispatch_kind"]=kind
        rows.append({"shipment":payload,"sha256":hashlib.sha256(canonical(payload)).hexdigest()})
    waiting=dict(legacy, disposition="waiting_for_storage", dispatch_kind="trade", returning_to_source=True)
    rows.append({"shipment":waiting,"sha256":hashlib.sha256(canonical(waiting)).hexdigest()})
    return canonical(rows)

def notebook_vector():
    view=lambda branch:{"branch":branch*16,"revision":None,"tick":50}
    integer=lambda value:{"type":"integer","value":str(value)}
    payload={"kind":"NYON_LIVING_NOTEBOOK","format_version":2,"rules_version":2,
             "catalog_hash":"11"*32,"genesis_digest":"22"*32,"archive_integrity_sha256":"33"*32,
             "bookmarks":[{"ordinal":1,"label":"Left","view":view("44"),"digest":"66"*32},
                          {"ordinal":2,"label":"Right","view":view("55"),"digest":"77"*32}],
             "experiments":[{"ordinal":1,"label":"Measured outcomes","left":1,"right":2,"comparison_tick":50,
                             "observations":[
                               {"metric":"inventory_energy","entity":"88"*16,"left":integer(20),"right":integer(30)},
                               {"metric":"owner","entity":"88"*16,"left":{"type":"entity","value":"99"*16},"right":{"type":"entity","value":None}},
                               {"metric":"colony_count","entity":None,"left":integer(1),"right":integer(2)},
                               {"metric":"facility_count","entity":"aa"*16,"left":{"type":"absent"},"right":integer(1)}]}]}
    integrity=hashlib.sha256(b"NYON-LIVING-NOTEBOOK-V2\0"+canonical(payload)).hexdigest()
    return canonical(dict(payload,integrity_sha256=integrity))

def notebook_evidence_vector():
    payload=json.loads(notebook_vector())
    del payload["integrity_sha256"]
    payload["evidence"]={"next_ordinal":3,"records":[
        {"ordinal":1,"bookmark":1,"experiment":"autonomous_settlement","receipt_digest":"aa"*32,
         "events":[{"ordinal":0,"id":"bb"*16,"kind":"settlement_claim_resolved"}]},
        {"ordinal":2,"bookmark":2,"experiment":"dormant_recovery","receipt_digest":"cc"*32,
         "events":[{"ordinal":3,"id":"dd"*16,"kind":"civilization_reactivated"}]}]}
    digest=hashlib.sha256(b"NYON-LIVING-NOTEBOOK-V2\0"+canonical(payload)).hexdigest()
    return canonical(dict(payload,integrity_sha256=digest))

def notebook_corridor_attempts_vector():
    # Ordered shape transcribed from the Corridor evidence contract, not Rust.
    payload=json.loads(notebook_vector())
    del payload["integrity_sha256"]
    payload["evidence"]={"next_ordinal":2,"records":[{
        "ordinal":1,"bookmark":1,"experiment":"corridor_impact",
        "receipt_digest":"aa"*32,"events":[],
        "route_attempts":[{"route":"bb"*16,"hazard":"cc"*16}]}]}
    digest=hashlib.sha256(b"NYON-LIVING-NOTEBOOK-V2\0"+canonical(payload)).hexdigest()
    return canonical(dict(payload,integrity_sha256=digest))

def empty_state_vector():
    state = {"tick":0,"accepted_sequence":0,"branch_sequence":0}
    for field in ["systems","stars","worlds","lanes","civilizations","deposits","colonies","facilities","construction_jobs","hull_jobs","fleets","routes","shipments","relations","agreements","wars","observations","hazards","settlement_claims","occupations"]:
        state[field]=[]
    state["counters"]={}
    return canonical({"state":state,"state_digest":hashlib.sha256(b"NYON-LIVING-STATE-V2\0"+canonical(state)).hexdigest()})

def production_facts_vector():
    payload = {"tick":10,"applied_revisions":[],"event_payloads":[{"ordinal":0,"provenance":{"type":"autonomous","phase":5},"kind":"facility_produced"}],"state_digest":"44"*32}
    receipt_digest=hashlib.sha256(b"NYON-LIVING-RECEIPT-V2\0"+canonical(payload)).digest()
    event_id=hashlib.sha256(b"NYON-LIVING-EVENT-V2\0"+receipt_digest+(0).to_bytes(2,"little")).digest()[:16].hex()
    facts={"actor":"11"*16,"entity":"22"*16,"resources":[{"role":"input","resource":"energy","quantity":2},{"role":"output","resource":"ore","quantity":4}],"blocked_reasons":[],"affected":[],"deposit_input":{"world":"55"*16,"units":4}}
    record={"tick":10,"state_digest":"44"*32,"receipt_digest":receipt_digest.hex(),"events":[{"ordinal":0,"event_id":event_id,"facts":facts}]}
    record["integrity_sha256"]=hashlib.sha256(b"NYON-LIVING-EVENT-FACTS-V2\0"+canonical(record)).hexdigest()
    return canonical(record)

def event_facts_vector():
    ordinary_payload = {"tick": 50, "applied_revisions": [], "event_payloads": [{"ordinal": 0, "provenance": {"type": "autonomous", "phase": 5}, "kind": "facility_blocked"}, {"ordinal": 1, "provenance": {"type": "autonomous", "phase": 5}, "kind": "construction_completed"}], "state_digest": "44" * 32}
    receipt_digest = hashlib.sha256(b"NYON-LIVING-RECEIPT-V2\0" + canonical(ordinary_payload)).digest()
    events = []
    for ordinal, facts in enumerate([
        {"actor": "11"*16, "entity": "22"*16, "resources": [], "blocked_reasons": [{"type":"missing_input","resource":"energy","required":4,"available":0}], "affected": []},
        {"actor": "11"*16, "entity": "33"*16, "resources": [{"role":"paid","resource":"alloy","quantity":60}], "blocked_reasons": [], "affected": ["55"*16]},
    ]):
        event_id = hashlib.sha256(b"NYON-LIVING-EVENT-V2\0" + receipt_digest + ordinal.to_bytes(2,"little")).digest()[:16].hex()
        events.append({"ordinal":ordinal,"event_id":event_id,"facts":facts})
    payload = {"tick":50,"state_digest":"44"*32,"receipt_digest":receipt_digest.hex(),"events":events}
    payload["integrity_sha256"] = hashlib.sha256(b"NYON-LIVING-EVENT-FACTS-V2\0" + canonical(payload)).hexdigest()
    return canonical(payload)

def trade_facts_vector(unknown=False):
    # Independently transcribed from the reviewed 2026-10-01 trade fact contract.
    ordinary_payload = {
        "tick": 50, "applied_revisions": [], "event_payloads": [
            {"ordinal": 0, "provenance": {"type": "autonomous", "phase": 3}, "kind": "shipment_delivered"},
            {"ordinal": 1, "provenance": {"type": "autonomous", "phase": 7}, "kind": "agreement_formed"},
        ], "state_digest": "44" * 32,
    }
    receipt_digest = hashlib.sha256(b"NYON-LIVING-RECEIPT-V2\0" + canonical(ordinary_payload)).digest()
    delivery = {
        "source_world": "33" * 16, "destination_world": "44" * 16,
        "dispatch_owner": "11" * 16, "intended_receiver": "22" * 16,
        "recipient": "22" * 16, "resource": "ore", "units": 10,
        "dispatch_kind": None if unknown else "trade",
        "source_system": None if unknown else "55" * 16,
        "destination_system": None if unknown else "66" * 16,
    }
    agreement = {
        "kind": "trade", "participant_a": "11" * 16, "participant_b": "22" * 16,
        "start_tick": 50, "end_tick": 1250,
    }
    facts = [
        {"actor": None, "entity": None, "resources": [], "blocked_reasons": [], "affected": [], "delivery": delivery},
        {"actor": None, "entity": None, "resources": [], "blocked_reasons": [], "affected": [], "agreement": agreement},
    ]
    events = []
    for ordinal, value in enumerate(facts):
        event_id = hashlib.sha256(b"NYON-LIVING-EVENT-V2\0" + receipt_digest + ordinal.to_bytes(2, "little")).digest()[:16].hex()
        events.append({"ordinal": ordinal, "event_id": event_id, "facts": value})
    payload = {"tick": 50, "state_digest": "44" * 32, "receipt_digest": receipt_digest.hex(), "events": events}
    payload["integrity_sha256"] = hashlib.sha256(b"NYON-LIVING-EVENT-FACTS-V2\0" + canonical(payload)).hexdigest()
    return canonical(payload)

def omitted_facts_vector():
    # Independent sparse capacity witness: ordinary ordinal 1 remains intact.
    payload = json.loads(trade_facts_vector())
    del payload["integrity_sha256"]
    payload["events"] = payload["events"][:1]
    payload["omitted"] = {"reason": "capacity", "ranges": [[1, 1]]}
    payload["integrity_sha256"] = hashlib.sha256(b"NYON-LIVING-EVENT-FACTS-V2\0" + canonical(payload)).hexdigest()
    return canonical(payload)

def main():
    if len(sys.argv) > 1 and sys.argv[1] in ["print-notebook-corridor-attempts", "verify-notebook-corridor-attempts"]:
        expected=notebook_corridor_attempts_vector()
        if sys.argv[1].startswith("print-"):
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "notebook-corridor-attempts.json"
        if path.read_bytes() != expected:
            raise SystemExit("Corridor locator vector differs from independent formula")
        print("Corridor locator vector verified")
        return 0
    if len(sys.argv)>1 and sys.argv[1] in ["print-omitted-facts", "verify-omitted-facts"]:
        expected = omitted_facts_vector()
        if sys.argv[1].startswith("print"):
            sys.stdout.buffer.write(expected)
            return 0
        path = DEFAULT.parent / "omitted-facts.json"
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0
    if len(sys.argv)>1 and sys.argv[1] in ["print-trade-facts", "verify-trade-facts", "print-trade-facts-unknown", "verify-trade-facts-unknown"]:
        unknown = sys.argv[1].endswith("unknown")
        expected = trade_facts_vector(unknown)
        if sys.argv[1].startswith("print"):
            sys.stdout.buffer.write(expected)
            return 0
        path = DEFAULT.parent / ("trade-facts-unknown.json" if unknown else "trade-facts.json")
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0
    if len(sys.argv)>1 and sys.argv[1] in ["print-production-facts", "verify-production-facts"]:
        expected=production_facts_vector()
        if sys.argv[1]=="print-production-facts":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "production-facts.json"
        if path.read_bytes()!=expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0

    if len(sys.argv)>1 and sys.argv[1] in ["print-empty-state", "verify-empty-state"]:
        expected=empty_state_vector()
        if sys.argv[1]=="print-empty-state":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "empty-state.json"
        if path.read_bytes()!=expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0

    if len(sys.argv)>1 and sys.argv[1] in ["print-event-facts", "verify-event-facts"]:
        expected=event_facts_vector()
        if sys.argv[1]=="print-event-facts":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "event-facts.json"
        if path.read_bytes()!=expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0

    if len(sys.argv) > 1 and sys.argv[1] in ["print-decision-route", "verify-decision-route", "print-decision-route-adjust", "verify-decision-route-adjust"]:
        adjust = sys.argv[1].endswith("adjust")
        expected = decision_route_vector(adjust)
        if sys.argv[1].startswith("print"):
            sys.stdout.buffer.write(expected)
            return 0
        path = DEFAULT.parent / ("decision-route-adjust.json" if adjust else "decision-route.json")
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0
    if len(sys.argv) > 1 and sys.argv[1] in ["print-shipments","verify-shipments"]:
        expected=shipments_vector()
        if sys.argv[1] == "print-shipments":
            sys.stdout.buffer.write(expected)
            return
        path=DEFAULT.parent / "shipments.json"
        if path.read_bytes() != expected:
            raise SystemExit("shipment vectors differ from independent formulas")
        print("shipment vectors verified")
        return
    if len(sys.argv) > 1 and sys.argv[1] in ["print-notebook-evidence","verify-notebook-evidence"]:
        expected=notebook_evidence_vector()
        if sys.argv[1] == "print-notebook-evidence":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "notebook-evidence.json"
        if path.read_bytes() != expected:
            raise SystemExit("notebook evidence differs from independent formula")
        print("notebook evidence vector verified")
        return 0
    if len(sys.argv) > 1 and sys.argv[1] in ["print-notebook","verify-notebook"]:
        expected=notebook_vector()
        if sys.argv[1] == "print-notebook":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "notebook.json"
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        print("integrity_sha256="+json.loads(expected)["integrity_sha256"])
        return 0
    if len(sys.argv) > 1 and sys.argv[1] in ["print-decision-scrap","verify-decision-scrap"]:
        expected=decision_scrap_vector()
        if sys.argv[1] == "print-decision-scrap":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "decision-scrap.json"
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        print("integrity_sha256="+json.loads(expected)["integrity_sha256"])
        return 0
    if len(sys.argv) > 1 and sys.argv[1] in ["print-observations","verify-observations"]:
        expected=observations_vector()
        if sys.argv[1] == "print-observations":
            sys.stdout.buffer.write(expected)
            return 0
        path=DEFAULT.parent / "observations.json"
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "print-genesis-forks":
        sys.stdout.buffer.write(genesis_forks_vector())
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "verify-genesis-forks":
        path = DEFAULT.parent / "genesis-forks.json"
        expected = genesis_forks_vector()
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "print-creator":
        sys.stdout.buffer.write(creator_vector())
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "verify-creator":
        path = DEFAULT.parent / "creator-command.json"
        expected = creator_vector()
        if path.read_bytes() != expected:
            print(f"MISMATCH: {path}")
            return 1
        print(f"OK: {path}; {len(expected)} canonical bytes")
        print("revision_id=" + creator_revision(expected))
        return 0
    if len(sys.argv) > 1 and sys.argv[1] == "print":
        sys.stdout.buffer.write(vector())
        return 0
    if len(sys.argv) > 1 and sys.argv[1] != "verify":
        raise SystemExit("usage: living-v2-additions-vectors.py verify [decision.json] | verify-creator | verify-genesis-forks | verify-observations | verify-decision-scrap | verify-notebook | verify-shipments | print[-kind]")
    path = pathlib.Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT
    actual = path.read_bytes()
    expected = vector()
    if actual != expected:
        print(f"MISMATCH: {path}")
        return 1
    print(f"OK: {path}; {len(expected)} canonical bytes")
    print("integrity_sha256=" + json.loads(expected)["integrity_sha256"])
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
