#!/usr/bin/env bash
# bench/callgrind/run.sh - build the callgrind corpus with alya --release,
# verify each checksum, measure callgrind Ir + native wall time per workload.
# Emits a markdown table (stdout + $GITHUB_STEP_SUMMARY) and results.json.
#
# Usage (from the repo root):
#   ALYA_BIN=./target/debug/alya bench/callgrind/run.sh
#
# Env:
#   ALYA_BIN     path to the alya binary (default: alya on PATH)
#   CALLGRIND_OUT dir for binaries + callgrind files (default: $TMPDIR/alya-callgrind)
#   CORPUS_DIR   dir holding the *.alya workloads (default: <repo>/bench/callgrind)
#   CALLGRIND_TOLERATE_MISSING=1  skip (instead of fail) workloads that do not
#                build on this compiler (used for the base run in CI gating)
#
# Design notes are in bench/callgrind/README.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$ROOT/../.." && pwd)"
ALYA="${ALYA_BIN:-alya}"
CORPUS="${CORPUS_DIR:-$ROOT/bench/callgrind}"
OUTDIR="${CALLGRIND_OUT:-${TMPDIR:-/tmp}/alya-callgrind}"
TOLERATE="${CALLGRIND_TOLERATE_MISSING:-0}"
mkdir -p "$OUTDIR"

declare -A EXPECTED=(
  [fib_rec]=121393
  [fib_binet]=5358285900000
  [loop_sum]=31999996000000
  [str_build]=8000
  [map_churn]=64008000
  [array_ops]=3200040000
  [trig_loop]=21506
  [struct_fields]=15000675000
  [closure]=3200200002
  [alloc_loop]=64032003
)
# Normalization units for the Ir/op column (work units per workload).
declare -A OPS=(
  [fib_rec]=392835
  [fib_binet]=820000
  [loop_sum]=8000000
  [str_build]=8000
  [map_churn]=16001
  [array_ops]=160002
  [trig_loop]=20000
  [struct_fields]=200000
  [closure]=80001
  [alloc_loop]=8001
)
ORDER="fib_rec fib_binet loop_sum str_build map_churn array_ops trig_loop struct_fields closure alloc_loop"

table="| workload | checksum | Ir | Ir/op | wall ms |"$'\n'
table+="|---|---|---|---|---|"$'\n'
json="{"

first=1
for w in $ORDER; do
  echo "==> $w: building" >&2
  if ! "$ALYA" build "$CORPUS/$w.alya" -o "$OUTDIR/$w" --release --quiet; then
    if [ "$TOLERATE" = 1 ]; then
      echo "==> $w: skipped (does not build on this compiler)" >&2
      continue
    fi
    exit 1
  fi
  echo "==> $w: running (checksum + wall)" >&2
  start=$(date +%s%N)
  got=$("$OUTDIR/$w")
  end=$(date +%s%N)
  wall_ms=$(( (end - start) / 1000000 ))
  if [ "$got" != "${EXPECTED[$w]}" ]; then
    if [ "$TOLERATE" = 1 ]; then
      echo "==> $w: skipped (checksum differs on this compiler)" >&2
      continue
    fi
    echo "MISMATCH $w: got [$got] want [${EXPECTED[$w]}]" >&2
    exit 1
  fi
  echo "==> $w: callgrind..." >&2
  timeout 1800 valgrind --tool=callgrind "--callgrind-out-file=$OUTDIR/callgrind.out.$w" "$OUTDIR/$w" > /dev/null 2> "$OUTDIR/vg.$w.log"
  ir=$(grep -oE "I +refs: +[0-9,]+" "$OUTDIR/vg.$w.log" | grep -oE "[0-9,]+$" | tr -d ',')
  if [ -z "$ir" ]; then
    echo "FAILED to parse Ir for $w (see $OUTDIR/vg.$w.log)" >&2
    exit 1
  fi
  perop=$(awk "BEGIN {printf \"%.1f\", $ir / ${OPS[$w]}}")
  table+="| $w | $got | $ir | $perop | $wall_ms |"$'\n'
  if [ "$first" -eq 1 ]; then first=0; else json+=","; fi
  json+="\"$w\":{\"checksum\":\"$got\",\"ir\":$ir,\"wall_ms\":$wall_ms,\"ops\":${OPS[$w]}}"
done
json+="}"

printf '%s' "$table"
printf '%s' "$json" > "$OUTDIR/results.json"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  printf '%s' "$table" >> "$GITHUB_STEP_SUMMARY"
fi
