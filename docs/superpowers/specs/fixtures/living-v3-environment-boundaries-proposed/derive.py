#!/usr/bin/env python3
"""Written-formula scope ties and positive-roster limits; no authority admission."""
import copy
import hashlib
import json
from pathlib import Path
import sys

BASE = Path(__file__).resolve().parent
SPECS = BASE.parent.parent
FREEZE = {
    "2026-10-01-living-v3-complete-schema.md": "9a05423608640515a6797fc3e73167a7213266d2e790f4d47ee73e0589996d73",
    "2026-10-02-living-v3-scoring.md": "c8bd14e773cc59fc8c9f7eb78b1eea6b5a32f648c6955b00ffe757f34c34976d",
    "2026-10-01-living-ecology-rules.md": "003659ee5eacd9b952344066eb6024964b630273505f598a86a6b3dfd3287fc1",
}


def canon(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode()


def ident(value):
    return f"{value:032x}"


def event(family, ordinal, scope_id, observer=4, observed=20, end=200):
    value = {"id": ident(1000 + ordinal)}
    if family == "stellar":
        value.update(star=ident(500 + scope_id), system=ident(scope_id))
    else:
        value.update(definition="fixture_half", lane=ident(scope_id))
    value.update(warning_tick="0", start_tick="0", end_tick=str(end))
    if family == "stellar":
        value["habitability_impact"] = 0
    value.update(freight_factor={"numerator": 1, "denominator": 2},
                 originating_observer=ident(observer), observed_tick=str(observed))
    return value


def scope(family, scope_id, children, observer=4, observed=20):
    return {"system" if family == "stellar" else "lane": ident(scope_id),
            "originating_observer": ident(observer), "observed_tick": str(observed),
            "events": children}


def rank(value):
    return (-int(value["observed_tick"]), bytes.fromhex(value["originating_observer"]), canon(value))


def evaluate(family, sources, tick=40):
    key = "system" if family == "stellar" else "lane"
    winners = {}
    for source in sources:
        assert len(source["events"]) <= 128
        assert int(source["observed_tick"]) <= tick
        ids = [child["id"] for child in source["events"]]
        assert ids == sorted(set(ids))
        for child in source["events"]:
            assert child[key] == source[key]
            assert child["originating_observer"] == source["originating_observer"]
            assert child["observed_tick"] == source["observed_tick"]
            assert int(child["warning_tick"]) <= int(child["observed_tick"]) < int(child["end_tick"])
        previous = winners.get(source[key])
        if previous is None or rank(source) < rank(previous):
            winners[source[key]] = source
    chosen_scopes = [winners[k] for k in sorted(winners)]
    assert len(chosen_scopes) <= (64 if family == "stellar" else 256)
    children = {}
    for source in chosen_scopes:
        for child in source["events"]:
            previous = children.get(child["id"])
            if previous is None or rank(child) < rank(previous):
                children[child["id"]] = child
    identities = [children[k] for k in sorted(children)]
    positives = [child for child in identities if tick < int(child["end_tick"])]
    ok = len(positives) <= 128
    # On fault these are diagnostic private records, never a published actor payload.
    fragment = {"winning_scopes": chosen_scopes, "identity_winners": identities,
                "positive_records_before_capacity": positives}
    raw = canon(fragment)
    return {"outcome": "ready_fragment" if ok else "record_capacity",
            "positive_count": len(positives), "published": False,
            "diagnostic_fragment": fragment, "canonical_fragment_hex": raw.hex(),
            "diagnostic_sha256": hashlib.sha256(raw).hexdigest()}


def corpus():
    for filename, digest in FREEZE.items():
        assert hashlib.sha256((SPECS / filename).read_bytes()).hexdigest() == digest
    cases = []
    for family in ("stellar", "hazard"):
        first128 = [event(family, n, 1) for n in range(128)]
        separate = scope(family, 2, [event(family, 128, 2)])
        lower_observer_empty = scope(family, 1, [], observer=3)
        lower_observer_positive = scope(family, 1, [event(family, 0, 1, observer=3)], observer=3)
        higher_observer_empty = scope(family, 1, [], observer=5)
        same_tick_positive = scope(family, 1, [event(family, 0, 1)])
        same_tick_empty = scope(family, 1, [])
        assert canon(same_tick_empty) < canon(same_tick_positive)  # ']' precedes '{'.
        moved = scope(family, 2, [event(family, 0, 2)])
        newer_expired = scope(family, 2, [event(family, 0, 2, observed=30, end=35)], observed=30)
        specs = [
            ("positive_128", [scope(family, 1, first128)], 128, "ready_fragment"),
            ("positive_129", [scope(family, 1, first128), separate], 129, "record_capacity"),
            ("scope_supersession_before_cap", [scope(family, 1, first128), scope(family, 1, [], observed=30), separate], 1, "ready_fragment"),
            ("expiry_before_cap", [scope(family, 1, first128), scope(family, 2, [event(family, 128, 2, end=35)])], 128, "ready_fragment"),
            ("observer_empty_wins", [same_tick_positive, lower_observer_empty], 0, "ready_fragment"),
            ("observer_positive_wins", [lower_observer_positive, higher_observer_empty], 1, "ready_fragment"),
            ("canonical_scope_empty_wins", [same_tick_empty, same_tick_positive], 0, "ready_fragment"),
            ("canonical_child_scope_one_wins", [moved, same_tick_positive], 1, "ready_fragment"),
            ("newer_expired_identity_no_fallback", [same_tick_positive, newer_expired], 0, "ready_fragment"),
        ]
        for name, sources, count, outcome in specs:
            result = evaluate(family, sources)
            assert (result["positive_count"], result["outcome"]) == (count, outcome)
            assert evaluate(family, list(reversed(sources))) == result
            assert evaluate(family, sources + copy.deepcopy(sources)) == result
            if name == "canonical_child_scope_one_wins":
                assert result["diagnostic_fragment"]["identity_winners"][0]["system" if family == "stellar" else "lane"] == ident(1)
            cases.append({"name": family + "_" + name, "capture_tick": "40",
                          "source_scope_fragments": sources, "expected": result,
                          "checks": ["reversed_source_order", "duplicate_sources"]})
    return {"status": "PROPOSED_DIAGNOSTIC_FRAGMENT_ONLY", "source_freeze": FREEZE,
            "scope": "Synthetic scope/child formula fragments, not admitted State/Observation/history or a full actor payload. No authority code imported.",
            "hash_contract": "SHA256 of literal diagnostic fragment; not STATE/OBSERVATION/trace/Receipt digest.",
            "cases": cases, "remaining": ["Authenticated source histories and full actor capture", "Per-array selection amendments", "Full six-gap and platform qualification"]}


def main():
    assert len(sys.argv) == 2 and sys.argv[1] in ("generate", "verify")
    raw = (json.dumps(corpus(), ensure_ascii=False, indent=2) + "\n").encode()
    target = BASE / "generatedvectors.json"
    if sys.argv[1] == "generate":
        target.write_bytes(raw)
    else:
        assert target.read_bytes() == raw, "generated_corpus_drift"
    print(sys.argv[1] + ": 18 environmental tie/capacity fragments")


if __name__ == "__main__":
    main()
