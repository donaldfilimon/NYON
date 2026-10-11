#!/bin/sh
# Gate for the implemented Zig migration surface. Full NYON acceptance also
# requires the unported modules and runtime/host qualification in the ledger.
set -eu
cd "$(dirname "$0")/.."
expected=$(cat .zigversion)
actual=$(zig version)
if [ "$actual" != "$expected" ]; then
    echo "Expected Zig $expected, found $actual" >&2
    exit 1
fi
# Cache failure must not become a false source diagnosis or shared-cache purge.
cache=$(mktemp -d "${TMPDIR:-/tmp}/nyon-zig-check.XXXXXX")
trap 'rm -rf "$cache"' EXIT HUP INT TERM
export ZIG_GLOBAL_CACHE_DIR="$cache/global"
export ZIG_LOCAL_CACHE_DIR="$cache/local"
zig fmt --check build.zig src
zig build test-bin
./zig-out/bin/nyon-zig-tests
zig build test -Doptimize=ReleaseSafe
zig build -Doptimize=ReleaseSafe
python3 tools/check-classic-store-cli.py
python3 tools/check-classic-parity.py
