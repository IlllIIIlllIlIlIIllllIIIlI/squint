#!/usr/bin/env bash
# bench_compare.sh — Wall-clock comparison: squint vs sqlfluff vs sqlfmt
#
# Prerequisites:
#   cargo build --release
#   cargo install hyperfine   (or: sudo apt install hyperfine / brew install hyperfine)
#   pip install sqlfluff
#   pip install shandy-sqlfmt
#
# Usage:
#   ./scripts/bench_compare.sh              # all three fixture sizes
#   ./scripts/bench_compare.sh --only-medium

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FIXTURES="${PROJECT_ROOT}/benches/fixtures"
RESULTS="${PROJECT_ROOT}/results"
BINARY="${PROJECT_ROOT}/target/release/squint"

# ── Parse arguments ───────────────────────────────────────────────────────────

ONLY_MEDIUM=false
for arg in "$@"; do
    if [ "$arg" = "--only-medium" ]; then
        ONLY_MEDIUM=true
    fi
done

# ── Sanity checks ─────────────────────────────────────────────────────────────

if ! command -v hyperfine &>/dev/null; then
    echo "ERROR: hyperfine not found."
    echo "Install with:  cargo install hyperfine"
    echo "           or: sudo apt install hyperfine"
    echo "           or: brew install hyperfine"
    exit 1
fi

if [ ! -f "${BINARY}" ]; then
    echo "ERROR: release binary not found at ${BINARY}"
    echo "Build with:  cargo build --release"
    exit 1
fi

HAVE_SQLFLUFF=false
if command -v sqlfluff &>/dev/null; then
    HAVE_SQLFLUFF=true
else
    echo "NOTE: sqlfluff not found — skipping (pip install sqlfluff)"
fi

HAVE_SQLFMT=false
if command -v sqlfmt &>/dev/null; then
    HAVE_SQLFMT=true
else
    echo "NOTE: sqlfmt not found — skipping (pip install shandy-sqlfmt)"
fi

mkdir -p "${RESULTS}"

# ── Helper ────────────────────────────────────────────────────────────────────

run_suite() {
    local label="$1"
    local fixture_file="$2"

    echo ""
    echo "============================================================"
    echo " Benchmarking: ${label}  (${fixture_file})"
    echo "============================================================"

    local hf_args=(
        "--warmup" "3"
        "--runs"   "10"
        "--ignore-failure"
        "--export-markdown" "${RESULTS}/bench_${label}.md"
        "--export-json"     "${RESULTS}/bench_${label}.json"
        "--command-name" "squint"
        "${BINARY} --quiet ${fixture_file}"
    )

    if [ "${HAVE_SQLFLUFF}" = true ]; then
        hf_args+=(
            "--command-name" "sqlfluff"
            "sqlfluff lint --nocolor --quiet --dialect ansi ${fixture_file}"
        )
    fi

    if [ "${HAVE_SQLFMT}" = true ]; then
        hf_args+=(
            "--command-name" "sqlfmt"
            "sqlfmt --check --no-color ${fixture_file}"
        )
    fi

    hyperfine "${hf_args[@]}"
    echo "Results written to: ${RESULTS}/bench_${label}.md"
}

# ── Main ──────────────────────────────────────────────────────────────────────

if [ "${ONLY_MEDIUM}" = true ]; then
    run_suite "medium" "${FIXTURES}/medium.sql"
else
    run_suite "small"  "${FIXTURES}/small.sql"
    run_suite "medium" "${FIXTURES}/medium.sql"
    run_suite "large"  "${FIXTURES}/large.sql"
fi

echo ""
echo "All benchmarks complete. Results in: ${RESULTS}/"
