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

## Thin slice 2b result (2026-10-05): Alya compiles Alya, differentially

Artifacts: `spike/selfhost/emit_asm.alya` (S-expr → x64 Windows asm;
`(say INT-EXPR)` with `(num)` / `(binop +|-|*)` / `(unop -)`;
push/pop eval, always-`movabs`, `printf("%lld\n")` minimal runtime;
anything else fails loudly) + `spike/selfhost/diff_emit.alya`
(Alya differential driver: per program `.alya` → lexer → parser →
emitter → gcc → exe vs `alya run` reference, byte-exact stdout).
Result: `emit|9|0|9` — 9/9 programs identical (precedence,
left-assoc, parens, negatives, unary minus, 10^12 `movabs`, multi-say).
Scope notes: `/`/`%` deferred (div-zero error semantics need their
own probe); no lets/functions yet; Windows program paths need
`\` under cmd (driver converts `/` via `to_win_path` — args with
`/` are fine, only the program token breaks). Temps
(`temp_emit_*.*`) gitignored + deleted per program, fmt clean.

## Thin slice 2c/2d/2e result (2026-10-05): div, lets, functions

`emit_asm.alya` v2 (fixed 256-byte frames, `push %rbp` form;
`let` slots with shadowing; `alya_fn_`-prefixed functions, ≤4 args
via left-to-right push + `mov OFF(%rsp)` + cleanup; `/`/`%` via
`idiv` with a divisor check replicating the reference div-zero
exactly — stdout message + exit 1 through C `exit`, so stdio
flushes; `diff_emit.alya` now triple-compares exit+stdout+stderr).
Result: `emit|19|0|19` — all 19 differential (old 9 intact, plus
truncating div/mod with negatives, div-zero + mod-zero error cases,
lets, shadowing, multi-arg functions, nested calls, zero-arg
functions, arg order via non-commutative `sub`).
Two bugs caught by the harness itself: an emitter double-eat of
`)` (review catch) and a `sub $248` frame breaking 16-alignment
after `push %rbp` (every backend exe died `0xC0000005`; fixed to
256). Deferred: strings, assign, nested functions, 5+ args, 29+
vars, fault cases (no crash-handler runtime in this slice).
Step 2 (thin end-to-end slice, x64) is DONE as scoped.

## Step 3 decision analysis (2026-10-05): full port vs hybrid vs stop

Measured inputs (2026-10-05, `alya 0.0.20`, Windows x64):

| Area | Rust LOC | Alya status |
|---|---|---|
| lexer + parser + ast | ~15.3k (5+15+3 files) | DONE, ~7.9k lines, 44/44 + 44/44 differential |
| codegen (analysis+emission+runtime+arch) | ~53.5k (81 files) | thin slice only (say/int/let/fn, 19/19) |
| driver (cli/build/toolchain/run) | ~3.9k | untouched |
| tools (fmt/lint/lsp/pkg/repl/...) | ~33.1k | untouched |
| total compiler | ~109k (174 files) | — |

Frontend perf on `stdlib/test.alya` (32 KB, 4208 tokens): Rust
`alya ast` ~18 ms; prebuilt Alya lexer ~20 ms + prebuilt Alya
parser ~28 ms (~54x faster than the 1500 ms first measured — the
gap was self-inflicted `+=`/char_at accumulation in `load_dump`
plus per-byte string building in `qs`/`json_decode`, fixed with
native `str.split` + array-join discipline; re-verified 49/49 +
54/54 + 63/63 differential on str/test/functions). Total Alya
frontend ~48 ms vs Rust ~18 ms (~2.7x) — no longer a decision
factor for hybrid. Resolved 2026-10-06 (alya-lang/alya#97): the
`index_of`-based `split3` anomaly (identical fields in isolation,
corrupt full parse) was ring-buffer clobbering — `str.split` pieces
lived in the wrapping ring without a stable dup, so temp volume past
the ~950KB wrap overwrote earlier dump lines. Fixed at production
(split immortalizes pieces) plus a tag-gated `str_store` on dynamic
push stores. The char scan stays (never the hot path).

### Option A: full port (inference + codegen + driver + tools in Alya)

- Cost: ~90k Rust LOC remaining → est. 40–50k Alya lines at the
  parser's 0.5x ratio. Multi-month even sliced; the analysis core
  (~12k: CallIndex/inference/DCE) is emergent — divergence is
  silent miscompile, fidelity only differential.
- Benefit: true self-hosting (no Rust to build Alya), maximum
  dogfooding, language credibility.
- Risk: dual-arch exactness, permanent two-implementation drift
  burden, slowest payoff.

### Option B: hybrid (Alya frontend → S-expr → existing Rust pipeline)

- Shape: lower Alya-produced S-expr to Rust AST nodes, then run
  imports/expansion/CallIndex/typecheck/DCE/inference/codegen
  UNCHANGED. Bridge is mechanical (~2–4k Rust lines); spans need a
  dump-format extension (positions already tracked in parser.alya).
- Cost: weeks (bridge + spans + 2-stage build + Tst/Test wiring),
  plus permanent 3-place sync discipline (Rust AST change → dump +
  Alya parser + Rust reader) enforced by the existing differential
  harnesses, plus the frontend perf tax above.
- Benefit: Alya frontend in PRODUCTION use (largest real Alya
  program dogfooded daily); reuses 100% of spike work; reversible
  (flag-gated); de-risks any later full port.
- Acceptance before default-flip: diff_lex/diff_parse green over
  all Lib/* + App/* via the ecosystem runner; span parity on
  diagnostics; frontend perf already within budget (~2.7x after
  the parser join-discipline fix).

### Option C: stop (spike complete, no production commitment)

- Cost: zero. Artifacts stay as regression guards (wired into CI
  2026-10-06: `spike-lex-parse` on Linux + `spike-emit` on Windows).
- Benefit: findings already paid for themselves (#91, emission
  discipline, feasibility data).
- Cost of stopping: the bootstrap loop and hybrid path stay
  hypothetical.

### Recommendation: B, staged, with stop criteria

Ship the hybrid behind a flag (default off), promote to default
only on the acceptance gates above. Revisit A vs C once hybrid is
green: A only with explicit strategic will + resourcing, C if the
perf tax or sync burden proves uneconomic. Do NOT start A
directly — its emergent core is exactly the risk the spikes
quarantined.

## Bridge design (2026-10-06): S-expr → Rust AST reader

Exploration before any hybrid commitment. Verdict up front: the
assumed "span format extension" is NOT needed.

### Key finding: spans are absent end-to-end

- `src/ast/` nodes carry zero span fields (`expr.rs`/`stmt.rs` have
  no `span`/`line`/`col`); the whole pipeline downstream of `parse()`
  works without positions.
- Post-parse diagnostics carry no positions either: type-checker
  errors are plain strings (`TypeError: ...`, no line/col), and
  import resolution emits no positioned diagnostics.
- The only positioned diagnostics are parse errors, produced by the
  frontend itself — and the Alya frontend already reports them with
  position parity (`diff_parse.py`: 4/4 negative files agree on
  positions; `0|0|error|LINE:COL` contract).

Consequence: the bridge is a pure function
`canonical-dump-text → ast::Program`, the exact inverse of
`src/driver/ast_sexpr.rs`. No dump-format change, no AST change.

### Reader spec (`src/driver/sexpr_reader.rs`, new, ~1.5k lines)

Mechanical inverse of `ast_sexpr.rs`, sharing its atom tables where
possible. Tricky atoms and their rules:

- Strings: exact inverse of `qs`/`esc_char` — JSON-unescape incl.
  `\uXXXX` and surrogate pairs. NUL needs no rule: the dumper
  truncates at the first NUL, so the reader never sees one.
- Floats: parse decimal/`inf`/`NaN` text to f64; shortest-roundtrip
  (`{:?}`) guarantees text identity. Tests compare by bit value.
- Integers: decimal → i64; overflow behavior must match the Rust
  parser's (open: verify `>i64::MAX` literal handling on both sides).
- `_` → None, `true`/`false` → bool, folded desugars pass through
  untouched (the dump already contains `(call "m" ...)`,
  `(unop not (binop in ...))` etc. — the reader must NOT re-desugar).
- `0|0|error|...` lines never reach the reader: the driver checks
  the frontend exit code first and surfaces the Alya-side diagnostic.

### Pipeline wiring (flag-gated, default off)

`alya build --frontend alya` (name TBD): stage-0 Rust-built `alya`
compiles `lexer.alya`+`parser.alya` once (prebuilt binaries, the same
ones the perf numbers — ~20 ms + ~28 ms on 32 KB — were measured
with); the driver runs frontend → dump → `sexpr_reader` →
`resolve_imports` and continues UNCHANGED. Reversible by flag
removal; no existing path is touched.

### Acceptance gates (before any default-flip discussion)

1. Round-trip: dump → read → dump byte-identical over the full
   differential corpus (new CI job, extends `diff_parse.py`).
2. Execution: bridge-compiled programs behave byte-identically to
   direct-compiled ones over Lib/* + App/* via the ecosystem runner.
3. Errors: position parity (have); message TEXT parity explicitly
   out of scope initially (owner decision: normalize later or accept
   Alya-side wording).

### Sync discipline (the permanent cost)

Rust AST change → three places, enforced by (1): dump arm in
`ast_sexpr.rs` + reader arm in `sexpr_reader.rs` + `parser.alya`
arm. A CI assertion enumerates `Expr`/`Stmt` variants against
round-trip coverage so a new variant cannot slip through silently.

### Effort and risks

Reader + harness + build plumbing: weeks, not months (~1.5k Rust
lines, mostly match arms). Residual unknowns: float/int edge texts
(`1e999`, huge ints), reader stack depth on adversarial nesting,
frontend-binary distribution (checked in vs rebuilt — Tst/Test
builds from source anyway). None threatens the shape above.

### First-slice result (2026-10-07): reader + round-trip gate green

Artifacts: `src/driver/sexpr_reader.rs` (canonical dump → `Program`;
JSON-unescape incl. surrogate pairs, `i128` ints, `inf`/`NaN` floats,
`_`/`true`/`false`, all `Expr`/`Stmt` arms incl. the `(_)`-vs-`()` import
quirk; never re-desugars) + `tests/sexpr_roundtrip_tests.rs` (dump →
read → dump byte-identical + `Program` equality over `spec/syntax` +
`stdlib`) + 6 unit tests in-reader (arith, float bits/specials,
unicode, all stmt shapes, rejections). Result: **44/44 corpus files
round-trip, 0 skips**. One documented loss: NUL-containing strings
(`lexical.alya` `\0Done`) — the dumper truncates at the first NUL (same
rule as `diff_parse.py`), so AST equality holds modulo NUL-truncation
and re-dump stays byte-identical. Next: flag-gated pipeline wiring
(`--frontend alya`, stage-0 prebuilt frontend binaries) for gate 2
(execution parity over Lib/* + App/*).

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
