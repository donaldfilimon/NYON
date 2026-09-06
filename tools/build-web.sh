#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR

# Compatibility entry point: a browser release is now the pair of explicit
# backend artifacts selected by web/loader.js before graphics initialization.
"${SCRIPT_DIR}/build-web-webgpu.sh"
"${SCRIPT_DIR}/build-web-webgl.sh"
