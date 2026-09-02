#!/usr/bin/env bash
set -euo pipefail

readonly TOOLCHAIN="nightly-2026-09-01"
readonly TARGET="wasm32-unknown-unknown"
readonly WASM_BINDGEN_VERSION="0.2.127"
readonly TOOL_ROOT="target/tools/wasm-bindgen-cli-${WASM_BINDGEN_VERSION}"
readonly WASM_BINDGEN="${TOOL_ROOT}/bin/wasm-bindgen"

rustup target add "${TARGET}" --toolchain "${TOOLCHAIN}"

if [[ ! -x "${WASM_BINDGEN}" ]] || \
   [[ "$("${WASM_BINDGEN}" --version)" != "wasm-bindgen ${WASM_BINDGEN_VERSION}" ]]; then
  rm -rf "${TOOL_ROOT}"
  cargo +"${TOOLCHAIN}" install \
    --locked \
    --version "${WASM_BINDGEN_VERSION}" \
    --root "${TOOL_ROOT}" \
    wasm-bindgen-cli
fi

cargo +"${TOOLCHAIN}" build --release --target "${TARGET}" --lib
rm -rf dist
mkdir -p dist
"${WASM_BINDGEN}" \
  --target web \
  --out-dir dist \
  --out-name intergalactic_warfare \
  "target/${TARGET}/release/intergalactic_warfare.wasm"
