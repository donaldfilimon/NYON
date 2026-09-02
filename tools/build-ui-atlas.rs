use std::{
    env, fs,
    path::{Path, PathBuf},
};

use fontdue::{Font, FontSettings};
use resvg::{tiny_skia, usvg};
use serde::Serialize;
use sha2::{Digest, Sha256};

const ATLAS_WIDTH: usize = 1024;
const ATLAS_HEIGHT: usize = 1024;
const CELL_SIZE: usize = 64;
const CELLS_PER_ROW: usize = ATLAS_WIDTH / CELL_SIZE;
const FONT_PX: f32 = 40.0;
const FONT_ORIGIN_X: i32 = 8;
const FONT_BASELINE_Y: i32 = 48;
const ICON_SIZE: u32 = 40;
const ICON_INSET: i32 = 12;
const EDGE_THRESHOLD: u8 = 96;
const SDF_SPREAD: i32 = 8;

const ICONS: [(&str, &str); 15] = [
    ("play", "play.svg"),
    ("pause", "pause.svg"),
    ("speed", "gauge.svg"),
    ("reset", "rotate-ccw.svg"),
    ("settings", "settings.svg"),
    ("help", "circle-question-mark.svg"),
    ("crosshair", "crosshair.svg"),
    ("save", "save.svg"),
    ("load", "folder-open.svg"),
    ("check", "check.svg"),
    ("close", "x.svg"),
    ("zoom", "zoom-in.svg"),
    ("field-atmosphere", "wind.svg"),
    ("field-hydrosphere", "droplets.svg"),
    ("field-topology", "mountain.svg"),
];

const INTER_ARCHIVE_URL: &str =
    "https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip";
const INTER_ARCHIVE_SHA256: &str =
    "9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e";
const LUCIDE_ARCHIVE_URL: &str =
    "https://github.com/lucide-icons/lucide/archive/4aec3f892fd6c23063bc2fead83c899b5d412b1c.tar.gz";
const LUCIDE_ARCHIVE_SHA256: &str =
    "d8033570e13ff1e4a4efa9e1432e09a8fc007aab14bebe887dbde44858b7290e";

#[derive(Serialize)]
struct AtlasMetrics {
    format_version: u32,
    atlas: AtlasDescriptor,
    generator: GeneratorDescriptor,
    entries: Vec<AtlasEntry>,
}

#[derive(Serialize)]
struct AtlasDescriptor {
    width: u32,
    height: u32,
    format: &'static str,
}

#[derive(Serialize)]
struct GeneratorDescriptor {
    cell_size: u32,
    font_px: f32,
    font_origin_x: i32,
    font_baseline_y: i32,
    icon_size: u32,
    edge_threshold: u8,
    sdf_spread: u32,
    ordering: &'static str,
}

#[derive(Serialize)]
struct AtlasEntry {
    key: String,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    codepoint: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_weight: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'static str>,
    atlas_bounds: Bounds,
    advance: f32,
}

#[derive(Clone, Copy, Serialize)]
struct Bounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Serialize)]
struct HashManifest {
    format_version: u32,
    algorithm: &'static str,
    archives: Vec<ArchiveHash>,
    files: Vec<FileHash>,
}

#[derive(Serialize)]
struct ArchiveHash {
    name: &'static str,
    url: &'static str,
    sha256: &'static str,
    committed: bool,
}

#[derive(Serialize)]
struct FileHash {
    path: String,
    sha256: String,
    role: &'static str,
}

struct Outputs {
    atlas: Vec<u8>,
    metrics: Vec<u8>,
    hashes: Vec<u8>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ui atlas builder: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (root, check) = arguments()?;
    let outputs = build(&root)?;
    let ui_dir = root.join("assets/ui");

    if check {
        check_output(&ui_dir.join("atlas.r8"), &outputs.atlas)?;
        check_output(&ui_dir.join("atlas-metrics.json"), &outputs.metrics)?;
        check_output(&ui_dir.join("hashes.json"), &outputs.hashes)?;
        println!("UI atlas, metrics, and hashes are reproducible");
    } else {
        fs::write(ui_dir.join("atlas.r8"), outputs.atlas).map_err(io_error)?;
        fs::write(ui_dir.join("atlas-metrics.json"), outputs.metrics).map_err(io_error)?;
        fs::write(ui_dir.join("hashes.json"), outputs.hashes).map_err(io_error)?;
        println!("Wrote assets/ui/atlas.r8, atlas-metrics.json, and hashes.json");
    }

    Ok(())
}

fn arguments() -> Result<(PathBuf, bool), String> {
    let mut root = env::current_dir().map_err(io_error)?;
    let mut check = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--root" => {
                root = PathBuf::from(args.next().ok_or("--root requires a path")?);
            }
            "--help" | "-h" => {
                println!(
                    "usage: cargo run --release --manifest-path tools/ui-atlas/Cargo.toml --locked -- [--check] [--root PATH]"
                );
                std::process::exit(0);
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    Ok((root, check))
}

fn build(root: &Path) -> Result<Outputs, String> {
    let regular_path = root.join("assets/ui/fonts/Inter-Regular.ttf");
    let semibold_path = root.join("assets/ui/fonts/Inter-SemiBold.ttf");
    let regular_bytes = fs::read(&regular_path).map_err(io_error)?;
    let semibold_bytes = fs::read(&semibold_path).map_err(io_error)?;
    let regular = Font::from_bytes(regular_bytes, FontSettings::default())
        .map_err(|error| format!("cannot parse {}: {error}", regular_path.display()))?;
    let semibold = Font::from_bytes(semibold_bytes, FontSettings::default())
        .map_err(|error| format!("cannot parse {}: {error}", semibold_path.display()))?;

    let entry_count = 95 * 2 + ICONS.len();
    let row_count = entry_count.div_ceil(CELLS_PER_ROW);
    if row_count * CELL_SIZE > ATLAS_HEIGHT {
        return Err(format!("{entry_count} entries exceed the fixed atlas"));
    }

    let mut atlas = vec![0_u8; ATLAS_WIDTH * ATLAS_HEIGHT];
    let mut entries = Vec::with_capacity(entry_count);
    let mut index = 0_usize;
    for (font, weight) in [(&regular, "regular"), (&semibold, "semibold")] {
        for codepoint in 0x20_u32..=0x7e {
            let character = char::from_u32(codepoint).expect("printable ASCII is valid Unicode");
            let (metrics, bitmap) = font.rasterize(character, FONT_PX);
            let mut mask = vec![0_u8; CELL_SIZE * CELL_SIZE];
            let x = FONT_ORIGIN_X + metrics.xmin;
            let y = FONT_BASELINE_Y - metrics.ymin - metrics.height as i32;
            blit_alpha(
                &mut mask,
                CELL_SIZE,
                &bitmap,
                metrics.width,
                metrics.height,
                x,
                y,
            )?;
            write_cell(&mut atlas, index, &signed_distance(&mask));
            entries.push(AtlasEntry {
                key: format!("{weight}:U+{codepoint:04X}"),
                kind: "glyph",
                codepoint: Some(codepoint),
                font_weight: Some(weight),
                icon: None,
                atlas_bounds: cell_bounds(index),
                advance: metrics.advance_width,
            });
            index += 1;
        }
    }

    for (semantic_name, file_name) in ICONS {
        let svg_path = root.join("assets/ui/icons").join(file_name);
        let svg = fs::read_to_string(&svg_path).map_err(io_error)?;
        let tree = usvg::Tree::from_str(&svg, &usvg::Options::default())
            .map_err(|error| format!("cannot parse {}: {error}", svg_path.display()))?;
        let mut pixmap = tiny_skia::Pixmap::new(CELL_SIZE as u32, CELL_SIZE as u32)
            .ok_or("cannot allocate icon pixmap")?;
        let scale_x = ICON_SIZE as f32 / tree.size().width();
        let scale_y = ICON_SIZE as f32 / tree.size().height();
        let transform = tiny_skia::Transform::from_scale(scale_x, scale_y)
            .post_translate(ICON_INSET as f32, ICON_INSET as f32);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        let (pixels, remainder) = pixmap.data().as_chunks::<4>();
        debug_assert!(remainder.is_empty());
        let mask: Vec<u8> = pixels.iter().map(|rgba| rgba[3]).collect();
        write_cell(&mut atlas, index, &signed_distance(&mask));
        entries.push(AtlasEntry {
            key: format!("icon:{semantic_name}"),
            kind: "icon",
            codepoint: None,
            font_weight: None,
            icon: Some(semantic_name),
            atlas_bounds: cell_bounds(index),
            advance: ICON_SIZE as f32,
        });
        index += 1;
    }

    let metrics = AtlasMetrics {
        format_version: 1,
        atlas: AtlasDescriptor {
            width: ATLAS_WIDTH as u32,
            height: ATLAS_HEIGHT as u32,
            format: "r8_unorm",
        },
        generator: GeneratorDescriptor {
            cell_size: CELL_SIZE as u32,
            font_px: FONT_PX,
            font_origin_x: FONT_ORIGIN_X,
            font_baseline_y: FONT_BASELINE_Y,
            icon_size: ICON_SIZE,
            edge_threshold: EDGE_THRESHOLD,
            sdf_spread: SDF_SPREAD as u32,
            ordering: "regular ASCII U+0020..U+007E; semibold ASCII U+0020..U+007E; frozen icons in source order",
        },
        entries,
    };
    let mut metrics_bytes = serde_json::to_vec_pretty(&metrics).map_err(json_error)?;
    metrics_bytes.push(b'\n');
    let hashes = hash_manifest(root, &atlas, &metrics_bytes)?;
    let mut hash_bytes = serde_json::to_vec_pretty(&hashes).map_err(json_error)?;
    hash_bytes.push(b'\n');

    Ok(Outputs {
        atlas,
        metrics: metrics_bytes,
        hashes: hash_bytes,
    })
}

fn blit_alpha(
    target: &mut [u8],
    target_width: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    x: i32,
    y: i32,
) -> Result<(), String> {
    if source_width == 0 || source_height == 0 {
        return Ok(());
    }
    if source.len() != source_width * source_height {
        return Err("font rasterizer returned an invalid bitmap length".to_owned());
    }
    let target_height = target.len() / target_width;
    for source_y in 0..source_height {
        for source_x in 0..source_width {
            let target_x = x + source_x as i32;
            let target_y = y + source_y as i32;
            if target_x < 0
                || target_y < 0
                || target_x >= target_width as i32
                || target_y >= target_height as i32
            {
                return Err(format!(
                    "rasterized glyph leaves its {target_width}x{target_height} cell"
                ));
            }
            target[target_y as usize * target_width + target_x as usize] =
                source[source_y * source_width + source_x];
        }
    }
    Ok(())
}

fn signed_distance(alpha: &[u8]) -> Vec<u8> {
    let inside: Vec<bool> = alpha.iter().map(|value| *value >= EDGE_THRESHOLD).collect();
    if !inside.iter().any(|value| *value) {
        return vec![0; alpha.len()];
    }

    let mut sdf = vec![0_u8; alpha.len()];
    for y in 0..CELL_SIZE as i32 {
        for x in 0..CELL_SIZE as i32 {
            let current_inside = inside[y as usize * CELL_SIZE + x as usize];
            let mut minimum_squared = (SDF_SPREAD * SDF_SPREAD) as u32;
            for delta_y in -SDF_SPREAD..=SDF_SPREAD {
                for delta_x in -SDF_SPREAD..=SDF_SPREAD {
                    let squared = (delta_x * delta_x + delta_y * delta_y) as u32;
                    if squared >= minimum_squared {
                        continue;
                    }
                    let sample_x = x + delta_x;
                    let sample_y = y + delta_y;
                    let sample_inside = sample_x >= 0
                        && sample_y >= 0
                        && sample_x < CELL_SIZE as i32
                        && sample_y < CELL_SIZE as i32
                        && inside[sample_y as usize * CELL_SIZE + sample_x as usize];
                    if sample_inside != current_inside {
                        minimum_squared = squared;
                    }
                }
            }
            let distance = (minimum_squared as f32).sqrt().min(SDF_SPREAD as f32);
            let normalized = (distance + 0.5).min(SDF_SPREAD as f32) / SDF_SPREAD as f32;
            let value = if current_inside {
                128.0 + normalized * 127.0
            } else {
                128.0 - normalized * 128.0
            };
            sdf[y as usize * CELL_SIZE + x as usize] = value.round() as u8;
        }
    }
    sdf
}

fn write_cell(atlas: &mut [u8], index: usize, cell: &[u8]) {
    let bounds = cell_bounds(index);
    for row in 0..CELL_SIZE {
        let source_start = row * CELL_SIZE;
        let target_start = (bounds.y as usize + row) * ATLAS_WIDTH + bounds.x as usize;
        atlas[target_start..target_start + CELL_SIZE]
            .copy_from_slice(&cell[source_start..source_start + CELL_SIZE]);
    }
}

fn cell_bounds(index: usize) -> Bounds {
    Bounds {
        x: ((index % CELLS_PER_ROW) * CELL_SIZE) as u32,
        y: ((index / CELLS_PER_ROW) * CELL_SIZE) as u32,
        width: CELL_SIZE as u32,
        height: CELL_SIZE as u32,
    }
}

fn hash_manifest(root: &Path, atlas: &[u8], metrics: &[u8]) -> Result<HashManifest, String> {
    let mut files = Vec::new();
    let source_files = [
        ("assets/ui/fonts/Inter-Regular.ttf", "source font"),
        ("assets/ui/fonts/Inter-SemiBold.ttf", "source font"),
        ("assets/ui/licenses/Inter-OFL.txt", "source license"),
        ("assets/ui/licenses/Lucide-ISC.txt", "source license"),
    ];
    for (path, role) in source_files {
        let bytes = fs::read(root.join(path)).map_err(io_error)?;
        files.push(FileHash {
            path: path.to_owned(),
            sha256: sha256(&bytes),
            role,
        });
    }
    for (_, file_name) in ICONS {
        let path = format!("assets/ui/icons/{file_name}");
        let bytes = fs::read(root.join(&path)).map_err(io_error)?;
        files.push(FileHash {
            path,
            sha256: sha256(&bytes),
            role: "source icon",
        });
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));
    files.push(FileHash {
        path: "assets/ui/atlas-metrics.json".to_owned(),
        sha256: sha256(metrics),
        role: "generated metrics",
    });
    files.push(FileHash {
        path: "assets/ui/atlas.r8".to_owned(),
        sha256: sha256(atlas),
        role: "generated atlas",
    });

    Ok(HashManifest {
        format_version: 1,
        algorithm: "sha256",
        archives: vec![
            ArchiveHash {
                name: "Inter 4.1 release archive",
                url: INTER_ARCHIVE_URL,
                sha256: INTER_ARCHIVE_SHA256,
                committed: false,
            },
            ArchiveHash {
                name: "Lucide pinned source archive",
                url: LUCIDE_ARCHIVE_URL,
                sha256: LUCIDE_ARCHIVE_SHA256,
                committed: false,
            },
        ],
        files,
    })
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn check_output(path: &Path, expected: &[u8]) -> Result<(), String> {
    let actual = fs::read(path).map_err(io_error)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{} differs; regenerate with the builder",
            path.display()
        ))
    }
}

fn io_error(error: std::io::Error) -> String {
    error.to_string()
}

fn json_error(error: serde_json::Error) -> String {
    error.to_string()
}
