# Chapter 8: Architecture & Compiler Internals

[← Standard Library](standard-library.md) • [Wiki Home](README.md)

---

## 1. Compiler Pipeline

The `alyac` compiler is written entirely in Rust and designed for high throughput and zero external compiler dependencies. Unlike compilers requiring LLVM or large runtime libraries, Alya uses a clean single-pass pipeline emitting native assembly directly:

```text
Source Code (.alya)
        │
   [1. Lexer]               ──► Generates token stream (keywords, identifiers, literals)
        │
   [2. Parser]              ──► Builds the Abstract Syntax Tree (AST)
        │
   [3. Import Resolver]     ──► Resolves multi-file dependencies & stdlib modules
        │
   [4. Type Inference]      ──► Infers variable types (Int, Float, String, Array, Map, Struct)
        │
   [5. Code Generator]      ──► Emits native GNU/Mach-O assembly (ARM64 / x64 / x86)
        │
   [6. Linker Driver]       ──► Invokes host GCC / Clang to produce final executable
        │
Executable Binary (.exe / elf / macho)
```

---

## 2. Supported Target Architectures & ABIs

Alya automatically targets the host platform by default, or can cross-generate assembly using `--arch <arch>` and `--os <os>`.

| Target Architecture | ABI Convention | Registers Used | Supported Operating Systems |
|---|---|---|---|
| **x64 (x86_64)** | System V AMD64 (Linux/macOS)<br>Microsoft x64 (Windows) | `%rax`, `%rdi`, `%rsi`, `%rdx`, `%rcx`, `%rbx`, `%xmm0-%xmm3` | Linux (ELF64), Windows (PE/COFF), macOS (Mach-O) |
| **ARM64 (aarch64)** | AAPCS64 (Linux)<br>Darwin ARM64 (Apple Silicon) | `x0-x7` (args/return), `x9-x15` (scratch), `d0-d3` (floats) | macOS (Apple Silicon M1–M4), Linux (AArch64) |
| **x86 (i686)** | cdecl | `%eax`, `%edx`, `%ecx`, `%ebx`, `%xmm0-%xmm1` | Linux (ELF32), Windows (MinGW 32-bit) |

---

## 3. Code Generation Optimizations

### A. Branch Fusion
In loop conditions and conditional branches (e.g. `while i < 50000`), traditional compilers without an optimizer emit:
1. `cmp` (compare registers)
2. `setl` (materialize boolean flag)
3. `movzbq` (zero-extend flag into register)
4. `test` (test register)
5. `jz` (conditional branch)

Alya implements **Branch Fusion**, analyzing relational comparisons directly within `if` and `while` codegen. It emits:
1. `cmp $50000, %rax`
2. `jge .Lend`

This eliminates 3 assembly instructions and intermediate memory/register operations on every iteration.

---

### B. Single Unsigned Bounds Check
When indexing an array (`arr[i]`), standard naive checks require:
* `i < 0` check (signed branch)
* `i >= length` check (signed branch)

Alya emits a single unsigned comparison (`jae` on x86/x64, `b.hs` on ARM64). Because negative signed integers wrap into astronomical values (`> 2^63 - 1`) in unsigned interpretation, any negative index automatically triggers the out-of-bounds handler, cutting bounds-check instructions and branch mispredictions in half.

---

### C. Zero-Cycle Register Zeroing
When loading constant `0` into registers on x86 and x64, Alya emits `xor %eax, %eax` rather than `mov $0, %rax`. Modern x86 processors recognize the `xor reg, reg` idiom directly in the register rename stage, executing it with zero latency (0 clock cycles) and avoiding immediate constant decoding.

---

### D. Direct Register & Immediate Arithmetic
Binary operations, bitwise masks, and float operations with immediate constants or local variables are emitted as direct instruction operands (`add $5, %rax`, `ubfx x0, x0, #0, #32`) rather than pushing and popping stack entries.

---

### E. Multi-Pass Fixed-Point Struct Type Inference
The compiler runs an interprocedural fixed-point analysis over the AST (up to 6 iterative passes). It propagates struct types through function returns, parameter positions, and local variables. This enables the code generator to statically calculate struct field byte offsets at compile time, eliminating dictionary hashing or dynamic runtime lookups.

---

### F. ARM64 Immediate Range Splitting (`movz` / `movk`)
Standard ARM64 instructions limit immediate operands to 16-bit values with shifts. When emitting immediate values that exceed standard ranges (e.g. large integer constants or buffer capacities), `alyac` automatically decomposes the constant into a `movz` (move with zero) instruction followed by `movk` (move with keep) instructions.

---

### G. SplitMix64 High-Entropy PRNG
The runtime PRNG uses the SplitMix64 algorithm, auto-seeded via high-resolution hardware cycle counters (`rdtsc` on x86/x64, `cntvct_el0` on ARM64) and system epoch timestamps. This completely resolves the low-bit periodicity defects common to simple linear congruential generators (LCG).

---

## 4. Progressive Examples

### Level 1: Pure & Minimal (Inspect Tokens & AST)
```bash
# View lexer tokens
alya tokens spec/syntax/toolchain_example.alya

# View parsed Abstract Syntax Tree
alya ast spec/syntax/toolchain_example.alya
```

---

### Level 2: Practical & Idiomatic (Inspect Assembly Output)
Generate and inspect the clean, comment-annotated assembly generated by Alya:

```bash
# Output assembly to a file (.s)
alya spec/syntax/functions.alya -S -o fib.s

# Target Apple Silicon ARM64 assembly
alya spec/syntax/functions.alya --arch arm64 -S -o fib_arm64.s
```

Excerpt of generated ARM64 assembly:
```asm
# Function: fibonacci
_fibonacci:
    stp x29, x30, [sp, -16]!
    mov x29, sp
    # Direct register comparison and fused branch:
    cmp x0, #1
    b.gt .Lfib_recurse
    # Base case return:
    ldp x29, x30, [sp], 16
    ret
```

---

### Level 3: Advanced & Real-World (Micro-Benchmarking Compiler Throughput)
Profile the single-pass compiler stages on large codebases using built-in profiling flags:

```bash
# Print microsecond timings across every compiler phase
alya run apps/http_server/main.alya --time
```

Run Criterion compiler throughput benchmarks across the synthetic 1,100+ line test harness:
```bash
cargo bench --bench compiler_bench
```
**Measured Throughput:**
* Lexer: `~52 MB/s`
* Parser: `~1.95 Million lines/second`
* Codegen: `~545,000 assembly lines/second`
