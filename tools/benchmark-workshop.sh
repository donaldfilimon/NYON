#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly SCRIPT_DIR
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
readonly PROJECT_ROOT
readonly BUILD_ROOT="${PROJECT_ROOT}/target/workshop-authority-benchmark"
readonly BENCHMARK_BIN="${BUILD_ROOT}/release/examples/workshop_authority_benchmark"

mode="full"
output=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --smoke)
      mode="smoke"
      shift
      ;;
    --output)
      if [[ "$#" -lt 2 ]]; then
        echo "--output requires a path" >&2
        exit 64
      fi
      output="$2"
      shift 2
      ;;
    --help|-h)
      echo "Usage: tools/benchmark-workshop.sh [--smoke] [--output PATH]"
      echo "Full mode records 100 warmup and 10,000 measured declared-capacity steps."
      echo "Smoke mode exercises the same fixture with 10 warmup and 50 measured steps."
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 64
      ;;
  esac
done

cd "${PROJECT_ROOT}"

if [[ -z "${output}" ]]; then
  run_stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  readonly run_stamp
  output="${PROJECT_ROOT}/target/qualification/workshop-authority-${mode}-${run_stamp}.json"
fi
mkdir -p -- "$(dirname "${output}")"

cargo build \
  --release \
  --target-dir "${BUILD_ROOT}" \
  -p nyon-workshop-core \
  --example workshop_authority_benchmark

sha256_file() {
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    sha256sum "$1" | awk '{print $1}'
  fi
}

detect_cpu() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    sysctl -n machdep.cpu.brand_string 2>/dev/null || sysctl -n hw.model 2>/dev/null || echo unknown
  elif command -v lscpu >/dev/null 2>&1; then
    lscpu | awk -F: '/Model name/{sub(/^[[:space:]]+/, "", $2); print $2; exit}'
  else
    echo unknown
  fi
}

detect_os_version() {
  if command -v sw_vers >/dev/null 2>&1; then
    printf 'macOS %s build %s' "$(sw_vers -productVersion)" "$(sw_vers -buildVersion)"
  elif [[ -r /etc/os-release ]]; then
    awk -F= '/^PRETTY_NAME=/{gsub(/^"|"$/, "", $2); print $2; exit}' /etc/os-release
  else
    uname -srv
  fi
}

detect_power_mode() {
  if command -v pmset >/dev/null 2>&1; then
    power_source="$(pmset -g batt | sed -n '1p')"
    low_power="$(pmset -g custom | awk '/lowpowermode/{print $2; exit}')"
    printf '%s; lowpowermode=%s' "${power_source:-unknown}" "${low_power:-unknown}"
  elif [[ -r /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]]; then
    printf 'governor=%s' "$(< /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
  else
    echo unknown
  fi
}

git_dirty="false"
if [[ -n "$(git status --porcelain --untracked-files=all)" ]]; then
  git_dirty="true"
fi

warmup_steps="100"
measured_steps="10000"
allow_short="0"
if [[ "${mode}" == "smoke" ]]; then
  warmup_steps="10"
  measured_steps="50"
  allow_short="1"
fi

artifact_sha256="$(sha256_file "${BENCHMARK_BIN}")"
host_label="$(uname -n) $(uname -srm)"
os_version="$(detect_os_version)"
cpu="$(detect_cpu)"
power_mode="$(detect_power_mode)"
git_commit="$(git rev-parse HEAD)"
rustc_version="$(rustc -Vv | tr '\n' ';')"
target_triple="$(rustc -vV | awk '/^host:/{print $2}')"
generated_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

NYON_WORKSHOP_BENCH_ALLOW_SHORT="${allow_short}" \
NYON_WORKSHOP_BENCH_WARMUP_STEPS="${warmup_steps}" \
NYON_WORKSHOP_BENCH_MEASURED_STEPS="${measured_steps}" \
NYON_WORKSHOP_BENCH_OUTPUT="${output}" \
NYON_WORKSHOP_BENCH_GENERATED_UTC="${generated_utc}" \
NYON_WORKSHOP_BENCH_HOST="${host_label}" \
NYON_WORKSHOP_BENCH_OS_VERSION="${os_version}" \
NYON_WORKSHOP_BENCH_CPU="${cpu}" \
NYON_WORKSHOP_BENCH_POWER_MODE="${power_mode}" \
NYON_WORKSHOP_BENCH_GIT_COMMIT="${git_commit}" \
NYON_WORKSHOP_BENCH_GIT_DIRTY="${git_dirty}" \
NYON_WORKSHOP_BENCH_ARTIFACT_SHA256="${artifact_sha256}" \
NYON_WORKSHOP_BENCH_RUSTC="${rustc_version}" \
NYON_WORKSHOP_BENCH_TARGET="${target_triple}" \
"${BENCHMARK_BIN}"

echo "This report is one-host performance evidence only; it does not establish runtime or release qualification."
