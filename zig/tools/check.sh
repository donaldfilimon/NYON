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
# Make accidental Rust subprocess use fail even on hosts that have it installed.
mkdir "$cache/no-rust"
for command in cargo rustc rustup; do
    cat > "$cache/no-rust/$command" <<'EOF'
#!/bin/sh
echo "Rust tooling is forbidden in the required NYON Zig gate" >&2
exit 125
EOF
    chmod +x "$cache/no-rust/$command"
done
export PATH="$cache/no-rust:$PATH"
zig fmt --check build.zig src tools/check-planet-cli.zig
zig build test-bin
./zig-out/bin/nyon-zig-tests
zig build test-bin -Doptimize=ReleaseSafe
./zig-out/bin/nyon-zig-tests
zig build -Doptimize=ReleaseSafe
zig build check-planet-cli
zig build check-planet-wasm
python3 tools/check-classic-store-cli.py
python3 tools/check-surface-cli.py
# Historical Rust differential checks are optional reference tooling, never
# part of the required Zig gate.
