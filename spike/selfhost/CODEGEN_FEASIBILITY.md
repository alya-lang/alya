# Spike: codegen port feasibility (self-hosting step 3)

Date: 2026-10-04. Status: INVESTIGATION, no code.

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
