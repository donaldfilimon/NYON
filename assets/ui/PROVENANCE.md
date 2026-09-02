# UI Asset Provenance

The committed UI atlas is generated entirely from the pinned source files in
this directory. The game embeds the generated atlas and metrics at compile time;
normal native and WebAssembly builds do not fetch or read runtime assets.

## Inter 4.1

- Upstream: https://github.com/rsms/inter
- Release: https://github.com/rsms/inter/releases/tag/v4.1
- Archive: https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip
- Archive SHA-256: `9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e`
- Imported files: `extras/ttf/Inter-Regular.ttf`,
  `extras/ttf/Inter-SemiBold.ttf`, and `LICENSE.txt`
- License: SIL Open Font License 1.1, retained verbatim as
  `licenses/Inter-OFL.txt`

## Lucide 1.27.0 asset pin

- Upstream: https://github.com/lucide-icons/lucide
- Exact commit: `4aec3f892fd6c23063bc2fead83c899b5d412b1c`
- Commit: https://github.com/lucide-icons/lucide/commit/4aec3f892fd6c23063bc2fead83c899b5d412b1c
- Archive: https://github.com/lucide-icons/lucide/archive/4aec3f892fd6c23063bc2fead83c899b5d412b1c.tar.gz
- Archive SHA-256: `d8033570e13ff1e4a4efa9e1432e09a8fc007aab14bebe887dbde44858b7290e`
- License: ISC with the upstream Feather-derived-icon notice, retained
  verbatim as `licenses/Lucide-ISC.txt`

Only the frozen interface subset is committed. Semantic controls map to source
SVGs as follows:

| Interface meaning | Pinned source SVG |
| --- | --- |
| play | `play.svg` |
| pause | `pause.svg` |
| speed | `gauge.svg` |
| reset | `rotate-ccw.svg` |
| settings | `settings.svg` |
| help | `circle-question-mark.svg` (aliases include `help-circle`) |
| crosshair | `crosshair.svg` |
| save | `save.svg` |
| load | `folder-open.svg` |
| check | `check.svg` |
| close | `x.svg` |
| zoom | `zoom-in.svg` |
| atmosphere field | `wind.svg` |
| hydrosphere field | `droplets.svg` |
| topology field | `mountain.svg` |

## Reproduction

The isolated builder pins every Rust dependency in
`tools/ui-atlas/Cargo.lock`. From the repository root, run:

```sh
cargo run --release --manifest-path tools/ui-atlas/Cargo.toml --locked
cargo run --release --manifest-path tools/ui-atlas/Cargo.toml --locked -- --check
```

Generation uses a fixed 1024-by-1024 R8 atlas, 64-pixel cells, 40-pixel font
rasterization, a fixed baseline/origin, a fixed 96/255 coverage threshold, and
an 8-pixel signed-distance spread. Entry order is printable ASCII for Inter
Regular, printable ASCII for Inter SemiBold, then the icon table above. The
builder also rewrites `hashes.json`; that manifest covers every committed
source and generated payload. Archive hashes are audit records because the
archives themselves are deliberately not committed.
