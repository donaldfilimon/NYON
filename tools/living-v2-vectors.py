#!/usr/bin/env python3
"""Independently derive and verify the Living Galaxy V2 identity vectors.

This script exists so that
`crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json` is never
regenerated from the Rust implementation's own output. Every formula below was
transcribed from section 10 of
`docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` -- the domain
literals including their terminating NUL, the little-endian fixed widths, and
the field order -- not from `crates/nyon-workshop-core/src/living/ids.rs`. A
corpus regenerated from the code under test is a recording of that code rather
than evidence about it.

Usage:

    python3 tools/living-v2-vectors.py verify [vectors.json]
        Recompute every digest in the corpus from the inputs the corpus itself
        lists, and report each row as OK or MISMATCH. Exit status is 1 if any
        row disagrees.

    python3 tools/living-v2-vectors.py creator REVISION_HEX KIND LOCAL
        Print `creator_entity_digest` and `entity_id` for those inputs.

    python3 tools/living-v2-vectors.py autonomous CATALOG_HEX SEED_HEX \\
            BRANCH_HEX TICK PHASE ACTOR_HEX INTENT KIND LOCAL
        Print `autonomous_entity_digest` and `entity_id` for those inputs.

    python3 tools/living-v2-vectors.py receipt PAYLOAD_JSON_FILE
        Print the canonical bytes, `receipt_digest`, and the event ids of
        ordinals 0 and 1 for a `LivingReceiptPayloadV2` document.
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys

RULES_VERSION = 2

DOMAINS = {
    "pack": b"NYON-LIVING-PACK-V2\0",
    "state": b"NYON-LIVING-STATE-V2\0",
    "archive_integrity": b"NYON-LIVING-ARCHIVE-INTEGRITY-V2\0",
    "receipt": b"NYON-LIVING-RECEIPT-V2\0",
    "revision": b"NYON-LIVING-REVISION-V2\0",
    "creator_entity": b"NYON-LIVING-CREATOR-ENTITY-V2\0",
    "auto_entity": b"NYON-LIVING-AUTO-ENTITY-V2\0",
    "root_branch": b"NYON-LIVING-ROOT-BRANCH-V2\0",
    "fork_branch": b"NYON-LIVING-FORK-BRANCH-V2\0",
    "event": b"NYON-LIVING-EVENT-V2\0",
    "claim": b"NYON-LIVING-CLAIM-V2\0",
}

DEFAULT_VECTORS = (
    pathlib.Path(__file__).resolve().parent.parent
    / "crates/nyon-workshop-core/tests/fixtures/living-v2/vectors.json"
)


def sha(domain: bytes, *parts: bytes) -> str:
    digest = hashlib.sha256()
    digest.update(domain)
    for part in parts:
        digest.update(part)
    return digest.hexdigest()


def u16(value: int) -> bytes:
    return int(value).to_bytes(2, "little")


def u32(value: int) -> bytes:
    return int(value).to_bytes(4, "little")


def u64(value: int) -> bytes:
    return int(value).to_bytes(8, "little")


def hexb(text: str) -> bytes:
    return bytes.fromhex(text)


def revision_id(catalog: str, seed: str, parent: str | None, tick: int,
                ordinal: int, command_hex: str) -> str:
    command = hexb(command_hex)
    tag = b"\x01" + hexb(parent) if parent is not None else b"\x00"
    return sha(
        DOMAINS["revision"],
        u32(RULES_VERSION),
        hexb(catalog),
        hexb(seed),
        tag,
        u64(tick),
        u64(ordinal),
        u64(len(command)),
        command,
    )


def creator_entity_digest(revision: str, kind: int, local: int) -> str:
    return sha(DOMAINS["creator_entity"], hexb(revision), u16(kind), u16(local))


def autonomous_entity_digest(catalog: str, seed: str, branch: str, tick: int,
                             phase: int, actor: str, intent: int, kind: int,
                             local: int) -> str:
    return sha(
        DOMAINS["auto_entity"],
        u32(RULES_VERSION),
        hexb(catalog),
        hexb(seed),
        hexb(branch),
        u64(tick),
        u16(phase),
        hexb(actor),
        u16(intent),
        u16(kind),
        u16(local),
    )


def root_branch_digest(catalog: str, seed: str, manifest: str) -> str:
    return sha(
        DOMAINS["root_branch"],
        u32(RULES_VERSION),
        hexb(catalog),
        hexb(seed),
        hexb(manifest),
    )


def fork_branch_digest(parent: str, revision: str, tick: int, ordinal: int) -> str:
    return sha(
        DOMAINS["fork_branch"], hexb(parent), hexb(revision), u64(tick), u64(ordinal)
    )


def event_digest(receipt: str, ordinal: int) -> str:
    return sha(DOMAINS["event"], hexb(receipt), u16(ordinal))


def claim_rank(seed: str, world: str, close_tick: int, civilization: str) -> str:
    return sha(
        DOMAINS["claim"],
        u32(RULES_VERSION),
        hexb(seed),
        hexb(world),
        u64(close_tick),
        hexb(civilization),
    )


class Report:
    def __init__(self) -> None:
        self.failures = 0

    def check(self, label: str, expected: str, actual: str) -> None:
        if expected == actual:
            print(f"OK       {label}")
            return
        self.failures += 1
        print(f"MISMATCH {label}\n  stored     {expected}\n  recomputed {actual}")


def verify(path: pathlib.Path) -> int:
    corpus = json.loads(path.read_text())
    report = Report()

    if corpus["rules_version"] != RULES_VERSION:
        print(f"MISMATCH rules_version: {corpus['rules_version']}")
        report.failures += 1

    for row in corpus["domains"]:
        name = row["name"]
        domain = DOMAINS[name]
        report.check(f"domain {name} bytes", row["bytes_hex"], domain.hex())
        report.check(
            f"domain {name} empty-object digest",
            row["empty_object_digest"],
            sha(domain, b"{}"),
        )

    payload = corpus["payload_hashes"]
    body = hexb(payload["canonical_bytes_hex"])
    report.check("payload catalog_hash", payload["catalog_hash"], sha(DOMAINS["pack"], body))
    report.check("payload state_digest", payload["state_digest"], sha(DOMAINS["state"], body))
    report.check(
        "payload archive_integrity",
        payload["archive_integrity"],
        sha(DOMAINS["archive_integrity"], body),
    )
    report.check(
        "payload receipt_digest", payload["receipt_digest"], sha(DOMAINS["receipt"], body)
    )

    seed = corpus["revisions"][0]["genesis_seed"]
    for row in corpus["revisions"]:
        report.check(
            f"revision {row['label']}",
            row["revision_id"],
            revision_id(
                row["catalog_hash"],
                row["genesis_seed"],
                row["parent"],
                row["tick"],
                row["ordinal"],
                row["command_bytes_hex"],
            ),
        )

    tag = corpus["optional_tag"]
    report.check("optional absent tag", tag["absent_hex"], "00")
    report.check("optional present prefix", tag["present_prefix_hex"], "01")
    report.check(
        "optional present example",
        tag["present_example_hex"],
        "01" + corpus["revisions"][0]["revision_id"],
    )

    for row in corpus["creator_entities"]:
        digest = creator_entity_digest(
            row["revision_id"], row["entity_kind"], row["batch_local_id"]
        )
        report.check(f"creator {row['label']} digest", row["entity_digest"], digest)
        report.check(f"creator {row['label']} id", row["entity_id"], digest[:32])

    catalog = payload["catalog_hash"]
    for row in corpus["autonomous_entities"]:
        digest = autonomous_entity_digest(
            catalog,
            seed,
            row["branch_id"],
            row["tick"],
            row["phase"],
            row["actor_id"],
            row["intent_ordinal"],
            row["entity_kind"],
            row["local_id"],
        )
        report.check(f"autonomous {row['label']} digest", row["entity_digest"], digest)
        report.check(f"autonomous {row['label']} id", row["entity_id"], digest[:32])

    root = corpus["branches"]["root"]
    digest = root_branch_digest(
        root["catalog_hash"], root["genesis_seed"], root["genesis_manifest_digest"]
    )
    report.check("root branch digest", root["branch_digest"], digest)
    report.check("root branch id", root["branch_id"], digest[:32])

    fork = corpus["branches"]["fork"]
    digest = fork_branch_digest(
        fork["parent_branch_id"],
        fork["fork_revision"],
        fork["fork_tick"],
        fork["branch_ordinal"],
    )
    report.check("fork branch digest", fork["branch_digest"], digest)
    report.check("fork branch id", fork["branch_id"], digest[:32])

    for row in corpus["events"]:
        digest = event_digest(row["receipt_digest"], row["event_ordinal"])
        report.check(f"event ordinal {row['event_ordinal']} digest", row["event_digest"], digest)
        report.check(f"event ordinal {row['event_ordinal']} id", row["event_id"], digest[:32])

    receipt = corpus["receipt_payload"]
    payload = {
        "tick": receipt["tick"],
        "applied_revisions": receipt["applied_revisions"],
        "event_payloads": receipt["event_payloads"],
        "state_digest": receipt["state_digest"],
    }
    body = json.dumps(payload, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    report.check("receipt payload canonical bytes", receipt["canonical_bytes_hex"], body.hex())
    digest = sha(DOMAINS["receipt"], body)
    report.check("receipt payload digest", receipt["receipt_digest"], digest)
    for row in receipt["events"]:
        derived = event_digest(digest, row["ordinal"])
        report.check(
            f"receipt event ordinal {row['ordinal']} digest", row["event_digest"], derived
        )
        report.check(f"receipt event ordinal {row['ordinal']} id", row["event_id"], derived[:32])

    ranks = {}
    for row in corpus["claims"]:
        rank = claim_rank(
            seed, row["world_id"], row["claim_close_tick"], row["civilization_id"]
        )
        report.check(f"claim {row['label']}", row["claim_rank"], rank)
        ranks[row["label"]] = rank
    winner = min(ranks, key=lambda label: ranks[label])
    report.check("claim winner", corpus["claim_winner"], winner)

    print()
    if report.failures:
        print(f"{report.failures} row(s) disagree with the spec formulas")
        return 1
    print("every row in the corpus reproduces from the spec formulas")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    command = argv[1]
    if command == "verify":
        path = pathlib.Path(argv[2]) if len(argv) > 2 else DEFAULT_VECTORS
        return verify(path)
    if command == "creator":
        revision, kind, local = argv[2], int(argv[3]), int(argv[4])
        digest = creator_entity_digest(revision, kind, local)
        print(f"entity_digest {digest}")
        print(f"entity_id     {digest[:32]}")
        return 0
    if command == "autonomous":
        digest = autonomous_entity_digest(
            argv[2],
            argv[3],
            argv[4],
            int(argv[5]),
            int(argv[6]),
            argv[7],
            int(argv[8]),
            int(argv[9]),
            int(argv[10]),
        )
        print(f"entity_digest {digest}")
        print(f"entity_id     {digest[:32]}")
        return 0
    if command == "receipt":
        payload = json.loads(pathlib.Path(argv[2]).read_text())
        canonical = json.dumps(payload, separators=(",", ":"), ensure_ascii=False)
        body = canonical.encode("utf-8")
        digest = sha(DOMAINS["receipt"], body)
        print(f"canonical_bytes {canonical}")
        print(f"canonical_hex   {body.hex()}")
        print(f"receipt_digest  {digest}")
        for ordinal in (0, 1):
            event = event_digest(digest, ordinal)
            print(f"event {ordinal} digest {event}")
            print(f"event {ordinal} id     {event[:32]}")
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
