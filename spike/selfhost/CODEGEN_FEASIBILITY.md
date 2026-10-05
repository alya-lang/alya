# Spike: codegen port feasibility (self-hosting step 3)

Date: 2026-10-04. Status: INVESTIGATION, slice 1a DONE (2026-10-05).

## What codegen is (measured)

`src/codegen/` ≈ **53.2k LOC / ~74 files**:

| Part | LOC | Port character |
|---|---|---|
| `analysis/` (DCE, CallIndex, inference ×5, predicates, type_checker) | ~12.0k | emergent whole-program analysis, hardest |
| `expr.rs` + `say.rs` + `stmt/` emission | ~10.3k | giant match-tree, silent-miscompile risk |
| `runtime/x64` + `runtime/arm64` asm templates | ~20.9k | hand-tuned asm strings, mostly VERBATIM port |
| `arch/x64` + `arch/arm64` emitters | ~3.8k | per-OS/ABI discipline, exact |
| `mod.rs` + context + peephole + tests | ~4.6k | plumbing + text passes |

Pipeline (spec/COMPILER_PIPELINE.md): lex → parse_raw → imports →
inline/enum/const/generic/dynspec → CallIndex → typecheck → DCE →
inference → asm emission → peephole → gcc link → run.

## Structural good news

1. **No register allocation.** Everything is `rbp`-relative stack slots +
   one accumulator (`%rax`/`x0`) + fixed scratch regs. No live ranges,
   no graph coloring, no spilling.
2. **Runtime is data.** `runtime/*::emit` concatenates hand-written asm
   strings; a port carries the same TEXT (modulo verified segments).
   Nothing is computed at compile time except labels/IDs.
3. **Emission is string pushing.** `out.push_str(format!(...))` maps
   1:1 onto Alya string concat (with the throughput caveat below).
4. **Link is a command line.** gcc arg builder + exit code; `std/process`
   covers it with a quoting wrapper.
5. **Lexer+parser already in Alya** (44/44 each): the frontend half of
   the pipeline exists as differential-tested reference.

## Structural bad news

1. **Inference is emergent** (`inference/*` 5.5k + `predicates` 1.8k +
   `type_checker` 2.4k): fixpoint marker propagation over CallIndex with
   bare/`::`/`__` aliasing. No spec beyond the code; fidelity must be
   differential, and divergence is silent miscompile.
2. **Minimal `say 1+2` still concatenates the whole runtime** (~12k asm
   lines, mostly dead, linker-stripped). A naive port carries all of
   `runtime/*` from day one (or implements `--no-std` + dead-stripping
   first — itself a project).
3. **Dual-arch/OS matrix** (`.global` vs `.globl _`, calling conventions,
   shadow space, `adrp`, codesign, `-no-pie`/`-lws2_32`) must be exact or
   binaries fault on entry. The differential harness needs per-target
   execution (Docker x64/arm64 exist).
4. **Scale estimate**: ~53k Rust LOC → roughly 30–45k Alya lines.
   Multi-month, even sliced. This is not a "next slice" like the parser
   was; it is a project.

## Capability verdict (Alya as vehicle)

OK: file writing, dirs, CLI args, env read, 64-bit bit ops (logical),
maps (impl has rehash), u64/float-print/unicode/determinism (all fixed).
WORKAROUND (spike first): 1 MB emission throughput (proven only to
~40 KB; try array+join vs `ByteBuffer` vs `+=`), 10k-entry map timing,
f64-bit patterns via `Arena write_float/read_int` helper, gcc quoting +
temp names, external timing. BLOCKER (narrow): real stderr
(`io_stderr_write` → stdout; use log file), `set_env` (no codegen;
shell-prefix instead), NUL-binary emission (out of scope for asm+gcc).
Active traps (coding rules from day one): aliased imports, homogeneous
arrays/structs discipline is now compiler-enforced for struct fields
(#87 fixed) but NOT for mixed arrays — keep structs; never feed runtime
text through `"..."/f"..."`.

## 1a result (2026-10-05): emission vehicle decided

`spike/selfhost/emit_bench.alya` (`all|plus|join|buffer`, N lines,
`mode|len|checksum|ms` stats; internal `now_ms` + byte-exact len+sum):

| N (lines) | Bytes | plus (`+=`) | join | buffer |
|---|---|---|---|---|
| 10 | 270 | ok, 0 ms | ok, 0 ms | ok, 0 ms |
| 2000 | 61017 | ok, 63 ms | ok, 0 ms | ok, 15 ms |
| 30000 (~1 MB) | 959706 | OOM stable-store | ok, 16 ms | ok, 15 ms |
| 100000 (~3.2 MB) | 3231424 | — (dead) | ok, 47 ms | ok, 63 ms |
| 300000 (~10 MB) | 9972049 | — (dead) | ok, 171 ms | ok, 188 ms |

Byte-exact: all modes agree wherever all succeed (e.g. N=2000:
`61017|4222267`; N=30000 join=buffer `959706|65655231`).
`+=` dies between N=2000 (61 KB ok) and N=2500: each intermediate
concat pins stable-store memory, so total is O(n^2) against the 64 MB
`alya_str_stable` region (`src/codegen/runtime/data.rs`) — a clean
`stable store overflow` error, not corruption. join/buffer scale
linearly (~15 ms/MB) with 10x headroom over the 1 MB target.

Verdict: codegen emission MUST use array+join (fastest at every size)
or ByteBuffer (same complexity, ~same speed); `+=` loops are banned
for emission. No `alya fmt`/`clippy` fallout (spike-only, fmt clean).

## 1b result (2026-10-05): maps are not the risk

`spike/selfhost/map_bench.alya` (CallIndex proxy: prebuilt unique
`pkgmodNN::fnI__TM` keys, pure map ops timed, `map|n|ms_ins|ms_lookup|
ms_update|ok|sum` stats; fmt clean):

| N | insert | lookup | update | ok |
|---|---|---|---|---|
| 100 | 0 ms | 0 ms | 0 ms | 1 |
| 10000 | 0 ms | 0 ms | 0 ms | 1 (`50005000`) |
| 100000 | 16 ms | 0 ms | 0 ms | 1 |
| 300000 | 62 ms | 31 ms | 32 ms | 1 |

10k ops are sub-quantum (<15.6 ms) on every pass — the ms clock
cannot even resolve them. Linear scaling above the quantum (~2 ms
insert / ~1 ms lookup per 10k) gives 30x headroom over any realistic
whole-program CallIndex. Rehash is invisible; checksums exact, no
nondeterminism. Verdict: map throughput is NOT a self-host risk.

## 1c result (2026-10-05): f64 bit-patterns exact, one quirk documented

`spike/selfhost/f64bits_bench.alya` (Arena `write_float` + `read_int`
round-trip; per-check `t_name|0/1` lines + `f64|ok|passed|total`;
fmt clean, byte-identical across runs — no timing on any line):

- Result: `f64|1|32|32`. All edge values constructible and
  bit-exact: `0.0` (bits 0), `-0.0` via `0.0 * -1.0` (bits INT64_MIN,
  numerically `== 0.0` but bit-distinct), `+inf` via `1e308 * 10.0`
  (bits `0x7FF0...`, prints `inf`), `-inf` via `0.0 - inf`,
  NaN via `inf - inf` (bits `0xFFF8...`, two independent sources
  bit-identical), min subnormal `5e-324` (bits 1, exp field 0),
  1 ulp (`1.0000000000000002 - 1.0` bits differ by exactly 1).
  Every value survives a write/read round-trip bit-identically.
- Sign-bit-safe helpers only: exp is extracted after masking with
  INT64_MAX, mantissa with `2^52-1` — exact under both arithmetic
  and logical shr, no INT64_MIN negation anywhere.
- QUIRK, FIXED (#91): float ==,<,<= on NaN used to follow raw setcc
  on unordered (all true); now IEEE (all false except !=). NaN must
  still be detected via BITS (exp 2047 + mant != 0), never via
  ==/!= — and a codegen port must replicate IEEE exactly here.
- Verdict: the `write_float`/`read_int` vehicle carries every f64
  bit pattern exactly. 1c DONE; capability spikes 1a–1c all closed.

## Thin slice 2a result (2026-10-05): tool-run loop proven

`spike/selfhost/link_probe.alya` (fmt clean): writes a minimal x64
Windows asm (puts-based, `.asciz` so no NUL literal is needed),
links it with host gcc via `std/process` capture, runs the exe,
checks exit codes + stdout. Result: `link|1|171|625|20` (stable
across runs; stdout is the 20-byte `hi from alya-spike\r\n` —
MSVCRT text mode appends `\r`, both endings accepted).
`temp_spike_hi.*` temps are gitignored by name and deleted by the
probe (no strays). Verdict: file writing, gcc invocation, process
capture, and the differential-execution loop mechanics all work
from Alya. Next: 2b, an S-expr → asm emitter slice
(`say` + int arithmetic, x64).

## Recommendation (staged, no full-port commitment)

1. **Capability spikes in Alya** (days, kill the unknowns first):
   a. 1 MB emission benchmark (array+join vs `ByteBuffer` vs `+=`,
      byte-exact + timed).
   b. 10k string→int map insert/lookup timing.
   c. `f64bits` helper e2e (0.0/-0.0/inf/NaN/subnormals/1 ulp).
2. **Thin end-to-end slice** (weeks): functions + ints + say, x64,
   single OS, runtime templates verbatim, gcc wrapper, differential
   execution (Alya-backend binary output vs Rust-backend binary output
   on the same programs). Validates the whole bootstrap loop before
   any inference work.
3. **Then decide**: inference port (big) vs. hybrid (Alya frontend +
   Rust codegen via S-expr bridge) vs. stop.

The hybrid deserves explicit mention: `alya ast` already emits the full
parse tree; a Rust driver could read Alya-produced S-expr and run the
EXISTING codegen. That stages self-hosting value (Alya-written
frontend in production use) without re-implementing inference.
