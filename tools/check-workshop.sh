#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly PROJECT_ROOT
readonly WEBGPU_DIR="${PROJECT_ROOT}/dist/webgpu"
readonly WEBGL_DIR="${PROJECT_ROOT}/dist/webgl"

cd "${PROJECT_ROOT}"

fail() {
  echo "Workshop web check failed: $*" >&2
  exit 1
}

verify_artifact_tree() {
  local artifact_dir="$1"
  local unexpected

  [[ -f "${artifact_dir}/nyon.js" ]] || fail "missing ${artifact_dir}/nyon.js"
  [[ -f "${artifact_dir}/nyon_bg.wasm" ]] || fail "missing ${artifact_dir}/nyon_bg.wasm"
  unexpected="$(
    find "${artifact_dir}" -mindepth 1 -maxdepth 1 \
      ! -name nyon.js \
      ! -name nyon_bg.wasm \
      -print
  )"
  [[ -z "${unexpected}" ]] || fail "unexpected artifact output: ${unexpected}"
  grep -Fq "nyon_bg.wasm" "${artifact_dir}/nyon.js" || \
    fail "${artifact_dir}/nyon.js does not load its colocated WebAssembly"
}

# Every input that is compiled or embedded into the browser artifact. `assets/`
# belongs here because shaders, the UI atlas, and the core pack reach the wasm
# through `include_str!`/`include_bytes!`. `web/` deliberately does not: the
# loader and page are served beside the artifact rather than compiled into it,
# and `node --check web/loader.js` below covers them.
readonly BUILD_INPUTS=(
  Cargo.toml
  Cargo.lock
  rust-toolchain.toml
  src
  crates
  assets
  tools/build-web-webgpu.sh
  tools/build-web-webgl.sh
)

# Existence and difference are not currency. On 2026-09-04 this script passed
# against a `dist/` that predated a half-applied source edit, so a green result
# was taken as evidence the artifacts reflected the tree when they did not.
# Refuse to certify an artifact older than any of its inputs.
verify_artifact_freshness() {
  local artifact_dir="$1"
  local reference="${artifact_dir}/nyon_bg.wasm"
  local newer

  # Both artifacts are `cargo build --lib`, so test, example, and bench targets
  # cannot reach them. `crates/nyon-workshop-core/fuzz` is pruned for a stronger
  # reason: it declares its own `[workspace]`, so it is not a member of this
  # build at all. It is matched by exact path, not by name, so a future `fuzz`
  # module elsewhere in the tree stays an input. Pruning these keeps a routine
  # test or fuzz-target edit from demanding a browser rebuild, which is what
  # would otherwise teach a reader to bypass this check.
  newer="$(
    find "${BUILD_INPUTS[@]}" \
      \( -name target -o -name tests -o -name examples -o -name benches \
         -o -path crates/nyon-workshop-core/fuzz \) -prune -o \
      -type f -newer "${reference}" -print
  )"
  [[ -z "${newer}" ]] || fail \
    "${artifact_dir} is stale; rebuild it with tools/build-web-webgpu.sh and tools/build-web-webgl.sh. Newer inputs:
${newer}"
}

bash -n tools/build-web-webgpu.sh
bash -n tools/build-web-webgl.sh
bash -n tools/check-workshop.sh

grep -Fq 'webgpu-backend = ["wgpu/webgpu"]' Cargo.toml || \
  fail "Cargo feature webgpu-backend must select wgpu/webgpu"
grep -Fq 'webgl-backend = ["wgpu/webgl"]' Cargo.toml || \
  fail "Cargo feature webgl-backend must select wgpu/webgl"
grep -Fq 'default-features = false' Cargo.toml || \
  fail "wgpu default features must be disabled for distinct browser artifacts"

grep -Fq 'feature = "webgpu-backend"' src/engine/gpu.rs || \
  fail "GPU instance selection does not reference webgpu-backend"
grep -Fq 'feature = "webgl-backend"' src/engine/gpu.rs || \
  fail "GPU instance selection does not reference webgl-backend"
grep -Fq 'wgpu::Backends::BROWSER_WEBGPU' src/engine/gpu.rs || \
  fail "GPU instance selection does not request Browser WebGPU"
grep -Fq 'wgpu::Backends::GL' src/engine/gpu.rs || \
  fail "GPU instance selection does not request WebGL2 through wgpu GL"

verify_artifact_tree "${WEBGPU_DIR}"
verify_artifact_tree "${WEBGL_DIR}"
verify_artifact_freshness "${WEBGPU_DIR}"
verify_artifact_freshness "${WEBGL_DIR}"
if cmp -s "${WEBGPU_DIR}/nyon_bg.wasm" "${WEBGL_DIR}/nyon_bg.wasm"; then
  fail "WebGPU and WebGL2 artifacts are byte-identical"
fi

if command -v node >/dev/null 2>&1; then
  node --check web/loader.js
fi

workshop_gpu_paths=()
for candidate in \
  src/presentation/workshop.rs \
  src/presentation/workshop \
  src/engine/workshop.rs \
  src/engine/workshop \
  src/workshop \
  assets/workshop; do
  if [[ -e "${candidate}" ]]; then
    workshop_gpu_paths+=("${candidate}")
  fi
done

if [[ "${#workshop_gpu_paths[@]}" -gt 0 ]]; then
  readonly FORBIDDEN_WEBGL_PATTERN='create_compute_pipeline|begin_compute_pass|dispatch_workgroups|StorageTexture|texture_storage_|draw_indirect|draw_indexed_indirect|multi_draw_indirect|subgroup|map_async|get_mapped_range'
  if grep -rnEi "${FORBIDDEN_WEBGL_PATTERN}" "${workshop_gpu_paths[@]}"; then
    fail "Workshop WebGL2 subset contains a forbidden GPU capability"
  fi
fi

cargo test --test workshop_web --test renderer_contracts --test shaders

echo "Workshop web bundles and static contracts passed."
echo "This check does not establish live browser startup, rendering, fallback, accessibility, performance, or runtime qualification."
