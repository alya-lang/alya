# Callgrind instruction-count benchmarks

Deterministic, clock-independent codegen measurements: each workload in
`bench/callgrind/*.alya` does a fixed amount of work and prints an exact
integer checksum. `run.sh` builds every workload with `alya build --release`,
fails on checksum mismatch, then records valgrind callgrind `Ir` (instructions
retired) plus native wall time per workload. Because instruction counts don't
depend on machine speed or scheduling noise, runs are directly comparable
across machines and over time — the intended use is compiler regression
tracking per PR.

## Workloads

| workload | stresses | ops unit | checksum |
|---|---|---|---|
| `fib_rec` | deep integer recursion, call overhead (`fib(26)`) | 392835 calls | 121393 |
| `fib_binet` | float libm path (`pow`/`sqrt`), 20k×0..=40 | 820000 fibs | 5358285900000 |
| `loop_sum` | tight integer loop + accumulator (8M) | 8000000 iters | 31999996000000 |
| `str_build` | repeated string concat, realloc path (8k) | 8000 concats | 8000 |
| `map_churn` | map insert + lookup sweep (8k) | 16001 ops | 64008000 |
| `array_ops` | array push growth + iteration (80k) | 160002 ops | 3200040000 |
| `trig_loop` | libm `sin`/`cos`/`exp` throughput (20k) | 20000 iters | 21506 |
| `struct_fields` | struct field read/write (200k) | 200000 iters | 15000675000 |
| `closure` | closure alloc + higher-order call (80k) | 80001 iters | 3200200002 |
| `alloc_loop` | short-lived small arrays, GC pressure (8k) | 8001 iters | 64032003 |

Sizing rule: each workload runs ~0.2–1s natively (callgrind slows execution
~50–100×, so the full corpus costs a few minutes). Re-measure after
retargeting sizes; `int()`-truncated float totals need a wide margin to the
next integer (libm versions may differ in the last ulp).

## Running

```bash
# from the repo root, with an alya binary:
ALYA_BIN=./target/debug/alya bench/callgrind/run.sh
# binaries + callgrind files go to $TMPDIR/alya-callgrind (override: CALLGRIND_OUT=...)
# results.json (checksum/ir/wall_ms/ops per workload) lands next to them.
```

CI runs this on every PR to `develop` (`.github/workflows/callgrind.yml`):
head and base both run, `compare.py` prints a delta table (`Ir/op` included)
and records regressions; `topfns.sh` annotates the top-5 functions of
regressed workloads inside collapsible summary blocks; the job fails when any
common workload grows beyond `IR_LIMIT_PCT` (default 5.0, override via manual
dispatch). Raw `callgrind.out.*` files are uploaded for both sides.

`compare.py` and `topfns.sh` are standalone helpers:

```bash
python3 bench/callgrind/compare.py base.json head.json [limit_pct] [regressed_out]
bench/callgrind/topfns.sh <callgrind.out> [n=5]
```
