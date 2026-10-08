#!/usr/bin/env bash
# bench/callgrind/run.sh - build the callgrind corpus with alya --release,
# verify each checksum, measure callgrind Ir per workload, emit a table.
#
# Usage (from the repo root):
#   ALYA_BIN=./target/debug/alya bench/callgrind/run.sh
#
# Env:
#   ALYA_BIN     path to the alya binary (default: alya on PATH)
#   CALLGRIND_OUT dir for binaries + callgrind files (default: $TMPDIR/alya-callgrind)
#
# Design notes are in bench/callgrind/README.md.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$ROOT/../.." && pwd)"
ALYA="${ALYA_BIN:-alya}"
OUTDIR="${CALLGRIND_OUT:-${TMPDIR:-/tmp}/alya-callgrind}"
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
ORDER="fib_rec fib_binet loop_sum str_build map_churn array_ops trig_loop struct_fields closure alloc_loop"

table="| workload | checksum | Ir |"$'\n'
table+="|---|---|---|"$'\n'

for w in $ORDER; do
  echo "==> $w: building" >&2
  "$ALYA" build "$ROOT/bench/callgrind/$w.alya" -o "$OUTDIR/$w" --release --quiet
  got=$("$OUTDIR/$w")
  if [ "$got" != "${EXPECTED[$w]}" ]; then
    echo "MISMATCH $w: got [$got] want [${EXPECTED[$w]}]" >&2
    exit 1
  fi
  echo "==> $w: checksum OK, callgrind..." >&2
  timeout 1800 valgrind --tool=callgrind "--callgrind-out-file=$OUTDIR/callgrind.out.$w" "$OUTDIR/$w" > /dev/null 2> "$OUTDIR/vg.$w.log"
  ir=$(grep -oE "I +refs: +[0-9,]+" "$OUTDIR/vg.$w.log" | grep -oE "[0-9,]+$" | tr -d ',')
  if [ -z "$ir" ]; then
    echo "FAILED to parse Ir for $w (see $OUTDIR/vg.$w.log)" >&2
    exit 1
  fi
  table+="| $w | $got | $ir |"$'\n'
done

printf '%s' "$table"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  printf '%s' "$table" >> "$GITHUB_STEP_SUMMARY"
fi
