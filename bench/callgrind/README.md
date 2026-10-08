# Callgrind instruction-count benchmarks

Deterministic, clock-independent codegen measurements: each workload in
`bench/callgrind/*.alya` does a fixed amount of work and prints an exact
integer checksum. `run.sh` builds every workload with `alya build --release`,
fails on checksum mismatch, then records valgrind callgrind `Ir` (instructions
retired) per workload. Because instruction counts don't depend on machine
speed or scheduling noise, runs are directly comparable across machines and
over time — the intended use is compiler regression tracking per PR.

## Workloads

| workload | stresses | checksum |
|---|---|---|
| `fib_rec` | deep integer recursion, call overhead (`fib(26)`) | 121393 |
| `fib_binet` | float libm path (`pow`/`sqrt`), 20k×0..=40 | 5358285900000 |
| `loop_sum` | tight integer loop + accumulator (8M) | 31999996000000 |
| `str_build` | repeated string concat, realloc path (8k) | 8000 |
| `map_churn` | map insert + lookup sweep, string keys (8k) | 64008000 |
| `array_ops` | array push growth + iteration (80k) | 3200040000 |
| `trig_loop` | libm `sin`/`cos`/`exp` throughput (20k) | 21506 |
| `struct_fields` | struct field read/write (200k) | 15000675000 |
| `closure` | closure alloc + higher-order call (80k) | 3200200002 |
| `alloc_loop` | short-lived small arrays, GC pressure (8k) | 64032003 |

Sizing rule: each workload runs ~0.2–1s natively (callgrind slows execution
~50–100×, so the full corpus costs a few minutes). Re-measure after
retargeting sizes; `int()`-truncated float totals need a wide margin to the
next integer (libm versions may differ in the last ulp).

Notes:
- `map_churn` uses string keys: int-keyed maps crash past 255 entries
  (alya-lang/alya#140). Switch back once fixed.
- `fib_binet`/`trig_loop` need the `pow_f`/`sin` stdlib natives.

## Running

```bash
# from the repo root, with an alya binary:
ALYA_BIN=./target/debug/alya bench/callgrind/run.sh
# outputs + callgrind files go to $TMPDIR/alya-callgrind (override: CALLGRIND_OUT=...)
```

CI runs this on every PR to `develop` (`.github/workflows/callgrind.yml`),
publishes the table to the job summary and uploads the raw
`callgrind.out.*` files. Future work (v2): run base + head and fail on
Ir growth beyond a threshold.
