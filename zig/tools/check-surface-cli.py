#!/usr/bin/env python3
"""Independent integer cube seam witnesses; no Rust comparison tool."""
from pathlib import Path
import subprocess

binary = Path(__file__).resolve().parents[1] / "zig-out/bin/nyon-zig"

def run(side, cell):
    return subprocess.run(
        [str(binary), "surface-neighbors", str(side), str(cell)],
        capture_output=True, timeout=10,
    )

# Positive-Z x=3,y=1: east folds onto +X x=0,y=1.
result = run(4, 71)
assert result.returncode == 0, result.stderr
assert result.stdout == b"4\t70\t75\t67\n", result.stdout
# Positive-Z x=2,y=3: north folds onto +Y x=2,y=0.
result = run(4, 78)
assert result.returncode == 0, result.stderr
assert result.stdout == b"79\t77\t34\t74\n", result.stdout
for side, cell in [(0, 0), (4, 96), (4, -1), (65536, 0)]:
    assert run(side, cell).returncode != 0, (side, cell)
print("Surface CLI: 2 independent seam vectors and 4 invalid-input cases passed")
