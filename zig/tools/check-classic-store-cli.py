#!/usr/bin/env python3
"""Qualify explicit Classic CLI stores in owned temporary profiles.

Builds the pinned ReleaseSafe binary first; no Rust oracle or real saved profile
is opened. Canonical expectations come from the existing codec probes.
"""
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ZIG_ROOT = Path(__file__).resolve().parents[1]
BINARY = ZIG_ROOT / "zig-out/bin/nyon-zig"


def snapshot():
    paths = list((ZIG_ROOT / "src").rglob("*.zig"))
    paths += list((ZIG_ROOT / "tools").glob("*.py"))
    paths += [ZIG_ROOT / "tools/check.sh", ZIG_ROOT / "build.zig", ZIG_ROOT / ".zigversion"]
    return {str(p.relative_to(ZIG_ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}


def main():
    sources = snapshot()
    version = subprocess.run(["zig", "version"], check=True, capture_output=True, timeout=30).stdout.decode().strip()
    assert version == (ZIG_ROOT / ".zigversion").read_text().strip(), "compiler pin mismatch"
    with tempfile.TemporaryDirectory(prefix="nyon-task2b-cli-") as owned:
        root = Path(owned)
        env = dict(os.environ, ZIG_GLOBAL_CACHE_DIR=str(root / "cache/global"), ZIG_LOCAL_CACHE_DIR=str(root / "cache/local"))
        built = subprocess.run(["zig", "build", "-Doptimize=ReleaseSafe"], cwd=ZIG_ROOT, env=env, capture_output=True, timeout=600)
        if built.returncode:
            raise SystemExit(f"ReleaseSafe build failed ({built.returncode}):\n{built.stderr[-4000:].decode(errors='replace')}")
        binary_hash = hashlib.sha256(BINARY.read_bytes()).hexdigest()
        trap = root / "unused-environment"
        env.update(HOME=str(trap), APPDATA=str(trap), XDG_DATA_HOME=str(trap), NYON_PROFILE_DIR=str(trap))
        cases = 0

        def run(*args, success=True):
            nonlocal cases
            result = subprocess.run([str(BINARY), *map(str, args)], cwd=root, env=env, capture_output=True, timeout=30)
            cases += 1
            assert (result.returncode == 0) == success, (args[0], result.returncode, result.stdout[:500], result.stderr[:500])
            assert b"OutOfMemory" not in result.stdout + result.stderr, "unexpected resource failure"
            return result.stdout

        def probe(mode, payload=None):
            return run(mode, *([] if payload is None else [payload]))

        scenario_ok = probe("scenario-default", "4947573120260902")
        assert scenario_ok.startswith(b"OK\t")
        scenario_record = json.loads(scenario_ok.split(b"\t", 2)[2])
        scenario_json = scenario_ok.split(b"\t", 2)[2].strip().decode()
        preferences_ok = probe("preferences-default")
        preferences_record = json.loads(preferences_ok.split(b"\t", 1)[1])
        preferences_json = preferences_ok.split(b"\t", 1)[1].strip().decode()
        assert run("trace", "4947573120260902", "0") == b"0\t67D96E983D6C9330\t-\t-\n"
        help_output = run("help")
        for mode in ("scenario-store-load", "scenario-store-save", "preferences-store-load", "preferences-store-save"):
            assert mode.encode() in help_output

        profile = root / "nested/profile"
        assert run("scenario-store-load", profile) == b"ABSENT\n"
        assert run("preferences-store-load", profile) == preferences_ok
        assert not profile.parent.exists(), "absent/default loads created directories"
        assert run("scenario-store-save", profile, scenario_json) == scenario_ok
        assert run("scenario-store-load", profile) == scenario_ok
        changed = dict(preferences_record, ui_scale_percent=130, motion="REDUCED", high_contrast=True, graphics_quality="LOW", onboarding_completed=True)
        changed_json = json.dumps(changed, separators=(",", ":"))
        changed_ok = probe("preferences-json", changed_json)
        assert run("preferences-store-save", profile, changed_json) == changed_ok
        assert run("preferences-store-load", profile) == changed_ok
        scenario_file = profile / "scenario-v1.json"
        preferences_file = profile / "preferences-v1.json"
        assert scenario_file.read_bytes() == scenario_json.encode()
        assert preferences_file.read_bytes() == changed_ok.split(b"\t", 1)[1].strip()

        def rejected_scenario(record_or_json, category, detail=b""):
            payload = record_or_json if isinstance(record_or_json, str) else json.dumps(record_or_json, separators=(",", ":"))
            before = (scenario_file.read_bytes(), preferences_file.read_bytes())
            compact = probe("scenario-json", payload)
            assert compact == b"ERR\t" + category + b"\n", compact
            diagnostic = run("scenario-store-save", profile, payload, success=False)
            assert diagnostic.startswith(b"ERR\tCODEC\t" + category), diagnostic
            assert detail in diagnostic
            assert b"OK\t" not in diagnostic and len(diagnostic.splitlines()) == 1
            assert before == (scenario_file.read_bytes(), preferences_file.read_bytes())

        rejected_scenario("{", b"Json", b"cause=")
        invalid = copy.deepcopy(scenario_record); invalid["format_version"] = 2
        rejected_scenario(invalid, b"Version")
        invalid = copy.deepcopy(scenario_record); invalid["worlds"][0]["energy"] = "01"
        rejected_scenario(invalid, b"CanonicalNumber", b"field=world.energy\tkind=unsigned decimal")
        invalid = copy.deepcopy(scenario_record); invalid["rules"]["tick_hz"] = 0
        rejected_scenario(invalid, b"Validation", b"err:tick_rate:rules.tick_hz:")
        for invalid_payload, category, detail in (("{", b"Json", b"cause="), (json.dumps(dict(changed, format_version=2)), b"Version", b""),
                                                   (json.dumps(dict(changed, ui_scale_percent=101)), b"UiScale", b"value=101")):
            before = (scenario_file.read_bytes(), preferences_file.read_bytes())
            assert probe("preferences-json", invalid_payload) == b"ERR\t" + category + b"\n"
            diagnostic = run("preferences-store-save", profile, invalid_payload, success=False)
            assert diagnostic.startswith(b"ERR\tCODEC\t" + category) and detail in diagnostic
            assert b"OK\t" not in diagnostic and len(diagnostic.splitlines()) == 1
            assert before == (scenario_file.read_bytes(), preferences_file.read_bytes())

        # Padded exact caps are valid inputs and canonicalize on explicit save.
        for namespace, file, cap, payload, expected in (("scenario", scenario_file, 65536, scenario_json, scenario_ok),
                                                       ("preferences", preferences_file, 16384, changed_json, changed_ok)):
            padded = payload.encode() + b" " * (cap - len(payload.encode()))
            assert len(padded) == cap
            file.write_bytes(padded)
            sibling = preferences_file if namespace == "scenario" else scenario_file
            sibling_before = sibling.read_bytes()
            assert run(f"{namespace}-store-load", profile) == expected
            assert file.read_bytes() == padded, "load wrote canonical/default record"
            assert run(f"{namespace}-store-save", profile, padded.decode()) == expected
            canonical_before = file.read_bytes()
            too_large = padded + b" "
            diagnostic = run(f"{namespace}-store-save", profile, too_large.decode(), success=False)
            assert diagnostic.startswith(b"ERR\tCODEC\tTooLarge"), diagnostic
            assert file.read_bytes() == canonical_before and sibling.read_bytes() == sibling_before
            for raw, kind, detail in ((b"\xff", b"io", b"cause=InvalidUtf8"), (too_large, b"oversized_slot", f"max_bytes={cap}".encode())):
                file.write_bytes(raw)
                diagnostic = run(f"{namespace}-store-load", profile, success=namespace == "preferences")
                prefix = b"DEFAULT" if namespace == "preferences" else b"ERR"
                assert diagnostic.startswith(prefix + b"\tSTORE\t" + kind) and detail in diagnostic, diagnostic
                if namespace == "preferences":
                    assert diagnostic.splitlines()[1] + b"\n" == preferences_ok
                    assert len(diagnostic.splitlines()) == 2
                else:
                    assert len(diagnostic.splitlines()) == 1
                assert file.read_bytes() == raw and sibling.read_bytes() == sibling_before
            file.write_bytes(canonical_before)

        for raw, category, detail in ((b"{", b"Json", b"cause="), (json.dumps(dict(changed, format_version=2)).encode(), b"Version", b""),
                                     (json.dumps(dict(changed, ui_scale_percent=101)).encode(), b"UiScale", b"value=101")):
            preferences_file.write_bytes(raw)
            before = scenario_file.read_bytes()
            diagnostic = run("preferences-store-load", profile)
            assert diagnostic.startswith(b"DEFAULT\tCODEC\t" + category) and detail in diagnostic
            assert diagnostic.splitlines()[1] + b"\n" == preferences_ok and len(diagnostic.splitlines()) == 2
            assert preferences_file.read_bytes() == raw and scenario_file.read_bytes() == before
        preferences_file.write_bytes(changed_ok.split(b"\t", 1)[1].strip())
        # A raw semantic failure stays typed on scenario load as well as save.
        scenario_file.write_text(json.dumps(invalid))
        diagnostic = run("scenario-store-load", profile, success=False)
        assert diagnostic.startswith(b"ERR\tCODEC\tValidation") and b"tick_rate" in diagnostic
        assert scenario_file.read_text() == json.dumps(invalid)
        scenario_file.write_text(scenario_json)

        for namespace, payload in (("scenario", scenario_json), ("preferences", preferences_json)):
            for operation in ("load", "save"):
                mode = f"{namespace}-store-{operation}"
                extra = [payload] if operation == "save" else []
                for bad_profile in ("", "relative-profile", "./relative-profile"):
                    diagnostic = run(mode, bad_profile, *extra, success=False)
                    assert diagnostic == b"ERR\tPROFILE\tInvalidProfileDirectory\n"
                assert run(mode, success=False) == b"ERR\tARGUMENT\tInvalidArguments\n"
        assert not (root / "relative-profile").exists()

        # Real rename/persist failure: destination is a directory with sentinel.
        for namespace, payload in (("scenario", scenario_json), ("preferences", changed_json)):
            failed_profile = root / f"failed-{namespace}"
            target = failed_profile / f"{namespace}-v1.json"
            target.mkdir(parents=True)
            sentinel = target / "sentinel"; sentinel.write_bytes(b"keep target")
            sibling = failed_profile / ("preferences-v1.json" if namespace == "scenario" else "scenario-v1.json")
            sibling.write_bytes(b"keep sibling")
            diagnostic = run(f"{namespace}-store-save", failed_profile, payload, success=False)
            assert diagnostic.startswith(b"ERR\tSTORE\tio\t") and b"cause=-" not in diagnostic
            assert b"OK\t" not in diagnostic and len(diagnostic.splitlines()) == 1
            assert sentinel.read_bytes() == b"keep target" and sibling.read_bytes() == b"keep sibling"
            assert sorted(p.name for p in failed_profile.iterdir()) == sorted([target.name, sibling.name]), "owned temporary remains"
        assert not trap.exists(), "probe consulted environment profile"
        assert not list(root.rglob(".nyon-classic-*")), "owned store temporary remains"
        assert sources == snapshot(), "source changed during CLI qualification"
        assert binary_hash == hashlib.sha256(BINARY.read_bytes()).hexdigest(), "binary changed during CLI qualification"
        print(f"Classic store CLI: {cases} process checks passed; ReleaseSafe build exit=0; compiler={version}; skips=0")
        print(f"Zig binary sha256={binary_hash}")
        print(f"Source inputs sha256={hashlib.sha256(json.dumps(sources, sort_keys=True).encode()).hexdigest()}")
        print("Owned profiles cleaned; native process readback only, no directory crash durability/browser/Windows/Linux runtime proof")


if __name__ == "__main__":
    main()
