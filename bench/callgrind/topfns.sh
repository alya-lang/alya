#!/usr/bin/env bash
# bench/callgrind/topfns.sh - top-N functions by self Ir from a callgrind.out
# file, printed as a markdown code block. Used by CI to explain regressions.
#
# Usage: topfns.sh <callgrind.out> [n=5]
set -euo pipefail

file="${1:?usage: topfns.sh <callgrind.out> [n]}"
n="${2:-5}"

echo '```'
callgrind_annotate "$file" 2>/dev/null \
  | grep -E "^[0-9, ]+\(" \
  | grep -v TOTALS \
  | sed 's/,//g' \
  | sort -k1,1nr \
  | head -n "$n"
echo '```'
