#!/usr/bin/env python3
"""Proposed V3 formulas transcribed from the written specs, without Rust input.

This is a subset corpus: domain/empty-state framing, demographics and ecology.
It does not validate an implemented V3 codec or complete the schema review.
Run `python3 derive.py verify` or `python3 derive.py generate`.
"""
import hashlib
import json
import pathlib
import sys


def canonical(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")).encode("utf-8")


def demographic(population, food, energy, capacity):
    food_need = (population + 99) // 100
    energy_need = (population + 199) // 200
    food_paid = min(food, food_need)
    energy_paid = min(energy, energy_need)
    food -= food_paid
    energy -= energy_paid
    before = population
    shortage = food_paid < food_need or energy_paid < energy_need
    if shortage or population > capacity:
        population -= min(population, max(1, (population + 99) // 100))
    elif population > 0 and population < capacity:
        growth = min(capacity - population, max(1, population // 100))
        if food >= (population + growth + 99) // 100:
            population += growth
    return {
        "population": population, "food": food, "energy": energy,
        "food_paid": food_paid, "energy_paid": energy_paid,
        "food_shortage": food_need - food_paid,
        "energy_shortage": energy_need - energy_paid,
        "delta": population - before, "workers": population // 100,
    }


def ecology(base, stress, impact, capacity, resilient):
    penalty = stress // 2 + min(10000, impact)
    if resilient:
        penalty //= 2
    effective = max(0, base - penalty)
    return {"effective_habitability": effective,
            "effective_capacity": capacity * effective // 10000}


def stellar(seed, star, boundary):
    frame = (b"NYON_LIVING_V3_STELLAR_ACTIVITY" + bytes.fromhex(seed)
             + bytes.fromhex(star) + boundary.to_bytes(8, "little"))
    digest = hashlib.sha256(frame).digest()
    draw = int.from_bytes(digest[:2], "little")
    return {"input_hex": frame.hex(), "digest": digest.hex(), "draw": draw,
            "flare": draw < 6554, "next_activity_tick": str(boundary + 1000),
            "warning_tick": str(boundary), "start_tick": str(boundary + 100),
            "end_tick": str(boundary + 200)}


def vectors():
    fields = [
        "systems", "stars", "worlds", "lanes", "civilizations", "deposits",
        "colonies", "facilities", "construction_jobs", "hull_jobs", "fleets",
        "routes", "shipments", "relations", "agreements", "wars",
        "observations", "hazards", "settlement_claims", "occupations", "offers",
        "commitments", "defense_requests", "shared_intelligence", "stellar_events",
    ]
    state = {"tick": "0", "branch_sequence": "0", "next_sequence": "0"}
    state.update((field, []) for field in fields)
    state["counters"] = {}
    encoded = canonical(state)
    digest = hashlib.sha256(b"NYON_LIVING_V3_STATE\0" + encoded).digest()
    generator = b"empty_fixture"
    # Deliberately independent framing inputs; not a valid complete pack/genesis.
    frame = (b"NYON_LIVING_V3_ROOT_BRANCH\0" + bytes([0x11]) * 32
             + bytes([0x22]) * 32 + bytes([0x33]) * 32
             + len(generator).to_bytes(4, "little") + generator
             + (1).to_bytes(4, "little"))
    branch_digest = hashlib.sha256(frame).digest()
    inputs = [
        {"population": 500, "food": 20, "energy": 10, "capacity": 1000},
        {"population": 500, "food": 10, "energy": 10, "capacity": 1000},
        {"population": 500, "food": 0, "energy": 10, "capacity": 1000},
        {"population": 500, "food": 20, "energy": 10, "capacity": 400},
        {"population": 0, "food": 20, "energy": 10, "capacity": 1000},
        {"population": 1000, "food": 1000, "energy": 1000, "capacity": 1000},
    ]
    for population in (1, 99, 100, 199, 200, 10000):
        for food, energy in ((10000, 10000), (0, 10000), (10000, 0), (0, 0)):
            inputs.append({"population": population, "food": food,
                           "energy": energy, "capacity": 10000})
    ecology_inputs = [
        {"base": 10000, "stress": 0, "impact": 0, "capacity": 2000,
         "resilient": False},
        {"base": 8000, "stress": 101, "impact": 2000, "capacity": 2000,
         "resilient": False},
        {"base": 8000, "stress": 101, "impact": 2000, "capacity": 2000,
         "resilient": True},
        {"base": 1000, "stress": 10000, "impact": 20000, "capacity": 1000,
         "resilient": True},
    ]
    stellar_inputs = [{"seed": "00" * 32, "star": "01" * 16,
                       "boundary": boundary} for boundary in range(1000, 21000, 1000)]
    return {
        "status": "proposed_subset_not_implementation_evidence",
        "empty_state": {"canonical_utf8": encoded.decode("utf-8"),
                        "canonical_bytes": len(encoded), "state_digest": digest.hex()},
        "root_branch_frame": {"input_hex": frame.hex(), "digest": branch_digest.hex(),
                              "branch_id": branch_digest[:16].hex()},
        "demographics": [{"input": case, "expected": demographic(**case)} for case in inputs],
        "ecology": [{"input": case, "expected": ecology(**case)} for case in ecology_inputs],
        "stellar": [{"input": case, "expected": stellar(**case)} for case in stellar_inputs],
    }


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ("verify", "generate"):
        raise SystemExit("usage: derive.py verify|generate")
    target = pathlib.Path(__file__).resolve().with_name("vectors.json")
    expected = canonical(vectors()) + b"\n"
    if sys.argv[1] == "generate":
        target.write_bytes(expected)
        print(f"generated {len(expected)} bytes of proposed subset vectors")
    elif target.read_bytes() != expected:
        raise SystemExit("proposed vectors differ from independent formulas")
    else:
        corpus = vectors()
        print(f"verified proposed domains, {len(corpus['demographics'])} demographic, "
              f"{len(corpus['ecology'])} ecology and {len(corpus['stellar'])} stellar examples")


if __name__ == "__main__":
    main()
