#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly PROJECT_ROOT
readonly TOOLCHAIN="nightly-2026-09-01"
readonly TARGET="wasm32-unknown-unknown"
readonly BACKEND_FEATURE="webgpu-backend"
readonly WASM_BINDGEN_VERSION="0.2.127"
readonly TOOL_ROOT="${PROJECT_ROOT}/target/tools/wasm-bindgen-cli-${WASM_BINDGEN_VERSION}"
readonly WASM_BINDGEN="${TOOL_ROOT}/bin/wasm-bindgen"
readonly BUILD_ROOT="${PROJECT_ROOT}/target/workshop-web/webgpu"
readonly WASM_INPUT="${BUILD_ROOT}/${TARGET}/release/nyon.wasm"
readonly OUTPUT_DIR="${PROJECT_ROOT}/dist/webgpu"

cd "${PROJECT_ROOT}"

cleanup_staging() {
  if [[ -n "${STAGING_DIR:-}" && -d "${STAGING_DIR}" ]]; then
    rm -rf -- "${STAGING_DIR}"
  fi
}

rustup target add "${TARGET}" --toolchain "${TOOLCHAIN}"

if [[ ! -x "${WASM_BINDGEN}" ]] || \
   [[ "$("${WASM_BINDGEN}" --version 2>/dev/null || true)" != "wasm-bindgen ${WASM_BINDGEN_VERSION}" ]]; then
  rm -rf -- "${TOOL_ROOT}"
  cargo +"${TOOLCHAIN}" install \
    --locked \
    --version "${WASM_BINDGEN_VERSION}" \
    --root "${TOOL_ROOT}" \
    wasm-bindgen-cli
fi

# Cargo must map this package feature exclusively to wgpu's official
# `webgpu` feature. Disabling defaults prevents the WebGL backend from leaking
# into this qualification artifact.
cargo +"${TOOLCHAIN}" build \
  --release \
  --target "${TARGET}" \
  --target-dir "${BUILD_ROOT}" \
  --lib \
  --no-default-features \
  --features "${BACKEND_FEATURE}"

mkdir -p "${PROJECT_ROOT}/dist"
STAGING_DIR="$(mktemp -d "${PROJECT_ROOT}/dist/.webgpu-build.XXXXXX")"
readonly STAGING_DIR
trap cleanup_staging EXIT
"${WASM_BINDGEN}" \
  --target web \
  --no-typescript \
  --out-dir "${STAGING_DIR}" \
  --out-name nyon \
  "${WASM_INPUT}"

test -f "${STAGING_DIR}/nyon.js"
test -f "${STAGING_DIR}/nyon_bg.wasm"
rm -rf -- "${OUTPUT_DIR}"
mv "${STAGING_DIR}" "${OUTPUT_DIR}"
trap - EXIT
