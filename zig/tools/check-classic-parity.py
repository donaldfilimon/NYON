#!/usr/bin/env python3
"""Compare Classic digests, ordered reports and codecs with the current Rust game.

This does not derive new expected goldens. Rust remains a migration oracle;
the frozen initial digest is checked independently before differential traces.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
ZIG_ROOT = ROOT / "zig"
SEEDS = (0, 1, 0x4947573120260902, 0xFFFFFFFFFFFFFFFF, 0x123456789ABCDEF0, 0xDEADBEEF)


def run(command, *, cwd=ROOT, env=None):
    return subprocess.run(command, cwd=cwd, env=env, check=True, capture_output=True, timeout=600).stdout


def check_codecs(oracle, candidate):
    cases = 0
    digest = hashlib.sha256()
    def compare(arguments):
        nonlocal cases
        expected = run([str(oracle), *arguments])
        actual = run([str(candidate), *arguments])
        if expected != actual:
            raise SystemExit(f"codec parity mismatch case={cases} mode={arguments[0]} input={arguments[-1][:250]!r}\nRust: {expected[:300]!r}\nZig:  {actual[:300]!r}")
        digest.update(actual)
        cases += 1
        return expected
    def scenario_case(value):
        return compare(["scenario-json", json.dumps(value, separators=(",", ":"))])
    defaults = []
    for seed in SEEDS:
        row = compare(["scenario-default", f"{seed:016X}"])
        record = json.loads(row.split(b"\t", 2)[2])
        defaults.append(record)
        scenario_case(record)
    base = defaults[2]
    def changed(path, value):
        record = json.loads(json.dumps(base))
        target = record
        for key in path[:-1]:
            target = target[key]
        target[path[-1]] = value
        scenario_case(record)
    for key in ("format_version", "rules_version", "generator_version"):
        for value in (0, 2, "1", 1.0, -1, 4294967296, None, True):
            changed([key], value)
    for seed in ("0", "494757312026090a", "G947573120260902", " 947573120260902", "FFFFFFFFFFFFFFFF"):
        changed(["seed"], seed)
    for key in base["rules"]:
        for value in ("", "00", "-1", "18446744073709551616", None, True):
            changed(["rules", key], value)
    for key, value in (("tick_hz", 0), ("tick_hz", 59), ("win_world_count", 1), ("win_world_count", 8),
                       ("ai_period_ticks", "0"), ("hazard_period_ticks", "0"), ("hazard_duration_ticks", "0"),
                       ("hazard_duration_ticks", "2701"), ("hazard_first_tick", "18446744073709551615"),
                       ("maximum_energy", "0"), ("maximum_defense", "0"), ("minimum_launch", "0"),
                       ("minimum_launch", "250001"), ("field_raise_cost", "10000"), ("field_lower_refund", "20000"),
                       ("base_fleet_speed", "18446744073709551615")):
        changed(["rules", key], value)
    for key, values in {
        "name": ("", "aster", " ASTER", "ASTER  VALE", "ABCDEFGHIJKLMNOPQRSTUVWXY", "é"),
        "x": (-1, 10001, "1037", 1037.0, 2147483648),
        "owner": ("NEUTRAL", "INVALID", {"UNION": None}, {"UNION": []}),
        "energy": ("250001", "01", "-1", "18446744073709551616", 80000),
        "defense": ("0", "100001"), "atmosphere": (11, -1, "6", 6.0),
        "hydrosphere": (11,), "topology": (11,),
    }.items():
        for value in values:
            changed(["worlds", 0, key], value)
    for missing in base:
        record = dict(base)
        del record[missing]
        scenario_case(record)
    for extra in ("extra", "__proto__"):
        scenario_case(dict(base, **{extra: 1}))
    changed(["worlds"], base["worlds"][:-1])
    changed(["worlds"], base["worlds"] + [base["worlds"][0]])
    for index in range(7):
        record = json.loads(json.dumps(base))
        record["worlds"][index]["name"] = "SAME"
        scenario_case(record)
    record = json.loads(json.dumps(base))
    record["worlds"][1]["x"], record["worlds"][1]["y"] = record["worlds"][0]["x"], record["worlds"][0]["y"]
    scenario_case(record)
    record = json.loads(json.dumps(base))
    record["rules"] = list(record["rules"].values())
    record["worlds"] = [list(world.values()) for world in record["worlds"]]
    scenario_case(list(record.values()))
    canonical = json.dumps(base, separators=(",", ":"))
    for payload in ("", "{}", "[]", canonical + "{}", canonical.replace('"format_version":1', '"format_version":-0', 1),
                    canonical.replace('"x":1037', '"x":-0', 1), canonical.replace('"seed":', '"seed":"4947573120260902","seed":', 1),
                    canonical + " " * (65536 - len(canonical)), canonical + " " * (65537 - len(canonical))):
        compare(["scenario-json", payload])
    prefs = json.loads(compare(["preferences-default"]).split(b"\t", 1)[1])
    for scale in (85, 100, 115, 130):
        for motion in ("FULL", "REDUCED"):
            for contrast in (False, True):
                for quality in ("AUTO", "LOW", "HIGH"):
                    for onboarding in (False, True):
                        value = dict(prefs, ui_scale_percent=scale, motion=motion, high_contrast=contrast, graphics_quality=quality, onboarding_completed=onboarding)
                        compare(["preferences-json", json.dumps(value, separators=(",", ":"))])
    for key, values in {"format_version": (0, 2, -1, "1", 1.0, 4294967296),
                        "ui_scale_percent": (0, 101, -1, "100", 100.0, 65536),
                        "motion": ("INVALID", {"FULL": None}, {"FULL": []}, True),
                        "graphics_quality": ("INVALID", {"AUTO": None}, {"AUTO": 0}),
                        "high_contrast": ("false", 0, None), "onboarding_completed": ("false", 0, None)}.items():
        for value in values:
            compare(["preferences-json", json.dumps(dict(prefs, **{key: value}), separators=(",", ":"))])
        missing = dict(prefs)
        del missing[key]
        compare(["preferences-json", json.dumps(missing, separators=(",", ":"))])
    pref_json = json.dumps(prefs, separators=(",", ":"))
    for payload in ("", "{}", "[]", pref_json + "{}", pref_json.replace('"format_version":1', '"format_version":-0', 1),
                    pref_json.replace('"motion":', '"motion":"FULL","motion":', 1),
                    json.dumps(list(prefs.values())), json.dumps(dict(prefs, unknown=True)),
                    pref_json + " " * (16384 - len(pref_json)), pref_json + " " * (16385 - len(pref_json))):
        compare(["preferences-json", payload])
    print(f"Codec parity: {cases} matching canonical records/identities or rejection classes; sha256={digest.hexdigest()}", flush=True)
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true", help="Use caller-built binaries; does not prove input freshness")
    parser.add_argument("--ticks", type=int, default=12_000)
    options = parser.parse_args()
    if not 0 <= options.ticks <= 100_000:
        parser.error("ticks must be in 0..100000")
    expected_version = (ZIG_ROOT / ".zigversion").read_text().strip()
    actual_version = run(["zig", "version"]).decode().strip()
    if actual_version != expected_version:
        raise SystemExit(f"compiler mismatch: expected {expected_version}, found {actual_version}")
    def source_snapshot():
        paths = list((ROOT / "src/game").rglob("*.rs")) + list((ROOT / "src/scenario").rglob("*.rs"))
        paths += list((ZIG_ROOT / "src").rglob("*.zig"))
        paths += list((ROOT / "src/preferences").rglob("*.rs"))
        paths += [ROOT / "examples/zig_classic_oracle.rs", ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "rust-toolchain.toml", ZIG_ROOT / "build.zig", ZIG_ROOT / ".zigversion", ZIG_ROOT / "tools/check.sh", ZIG_ROOT / "tools/check-classic-store-cli.py", Path(__file__).resolve()]
        return {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(paths)}
    sources = source_snapshot()
    oracle = ROOT / "target/debug/examples/zig_classic_oracle"
    if not options.skip_build:
        cargo_output = run(["cargo", "build", "--locked", "--target-dir", str(ROOT / "target"), "--message-format=json", "--example", "zig_classic_oracle"])
        artifacts = [json.loads(line) for line in cargo_output.splitlines()]
        executables = [item["executable"] for item in artifacts if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == "zig_classic_oracle" and item.get("executable")]
        if len(executables) != 1:
            raise SystemExit("Cargo did not identify exactly one built Classic oracle executable")
        oracle = Path(executables[0])
        with tempfile.TemporaryDirectory(prefix="nyon-zig-parity-") as cache:
            env = dict(os.environ, ZIG_GLOBAL_CACHE_DIR=f"{cache}/global", ZIG_LOCAL_CACHE_DIR=f"{cache}/local")
            run(["zig", "build", "-Doptimize=ReleaseSafe"], cwd=ZIG_ROOT, env=env)
    candidate = ZIG_ROOT / "zig-out/bin/nyon-zig"
    binary_hashes = [hashlib.sha256(path.read_bytes()).hexdigest() for path in (oracle, candidate)]
    initial = run([str(candidate), "trace", "4947573120260902", "0"])
    if initial != b"0\t67D96E983D6C9330\t-\t-\n":
        raise SystemExit(f"frozen RulesV1 genesis mismatch: {initial!r}")
    rows = 0
    transitions = 0
    report_events = 0
    combined = hashlib.sha256()
    for seed in SEEDS:
        for scripted in (False, True):
            arguments = [f"{seed:016X}", str(options.ticks)] + (["scripted"] if scripted else [])
            reference = run([str(oracle), *arguments])
            actual = run([str(candidate), "trace", *arguments])
            if actual != reference:
                ref_rows, actual_rows = reference.splitlines(), actual.splitlines()
                for index in range(max(len(ref_rows), len(actual_rows))):
                    expected = ref_rows[index] if index < len(ref_rows) else b"<missing>"
                    received = actual_rows[index] if index < len(actual_rows) else b"<missing>"
                    if expected != received:
                        raise SystemExit(f"seed={seed:016X} scripted={scripted} row={index}\nRust: {expected!r}\nZig:  {received!r}")
                raise SystemExit("trace byte mismatch")
            lines = actual.splitlines()
            if len(lines) != options.ticks + 1:
                raise SystemExit("incorrect trace row count")
            rows += len(lines)
            previous_tick = 0
            for row in lines[1:]:
                tick, _, _, events = row.split(b"\t")
                current_tick = int(tick)
                transitions += current_tick != previous_tick
                previous_tick = current_tick
                if events != b"-":
                    report_events += len(events.split(b",")) - 1
            combined.update(actual)
            print(f"PASS seed={seed:016X} scripted={str(scripted).lower()} rows={len(lines)} sha256={hashlib.sha256(actual).hexdigest()}", flush=True)
    check_codecs(oracle, candidate)
    if sources != source_snapshot():
        raise SystemExit("source changed during parity; run again after the source is stable")
    if binary_hashes != [hashlib.sha256(path.read_bytes()).hexdigest() for path in (oracle, candidate)]:
        raise SystemExit("executable changed during parity; run again after builds are stable")
    print(f"Classic parity: {rows} matching rows, {transitions} actual tick transitions, {report_events} events; sha256={combined.hexdigest()}")
    print(f"Rust oracle sha256={binary_hashes[0]}")
    print(f"Zig binary sha256={binary_hashes[1]} compiler={actual_version}")
    print(f"Source inputs sha256={hashlib.sha256(json.dumps(sources, sort_keys=True, separators=(',', ':')).encode()).hexdigest()}")
    if options.skip_build:
        print("Partial freshness: --skip-build used existing binaries; matching source snapshots do not prove those binaries came from that source")


if __name__ == "__main__":
    main()
