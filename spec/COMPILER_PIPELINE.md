# Alya Compiler Native Pipeline & Algorithmic Architecture

> **Authoritative Specification: Multi-Pass Compilation, Tree-Shaking, and Linear-Time Inference.**

To sustain Alya's **~2,000,000 lines/second** compilation throughput and prevent binary bloat, the `alya` compiler operates as a strictly ordered, multi-pass pipeline. This document defines the engineering invariants, algorithmic requirements, and optimization passes that all compiler implementations must follow.

---

## 1. The Compilation Pipeline Overview

```text
Source Code (.alya files)
        │
   [Pass 1: Parallel Lexing & Parsing]
        │ Emits individual file ASTs with source location spans
        ▼
   [Pass 2: Whole-Program AST Aggregation & Module Resolution]
        │ Resolves imports (std/*, local files, alya-lang/* packages) into a unified AST
        ▼
   [Pass 3: O(N) Call-Graph & CallIndex Indexing]
        │ Single linear sweep indexing all call sites: HashMap[Symbol, List[CallArgs]]
        ▼
   [Pass 4: Root Reachability & Tree-Shaking (DCE)]
        │ Prunes unreferenced functions/types starting from roots: main(), @test, @export
        ▼
   [Pass 5: Linear-Time Type Inference via CallIndex]
        │ Fixed-point type propagation using O(1) CallIndex lookups (NO recursive AST walks)
        ▼
   [Pass 6: Hardware-Aware Direct-to-Assembly Codegen]
        │ Branch fusion, zero-cycle idioms, unsigned bounds checks, direct GNU/Mach-O asm
        ▼
   [Pass 7: Native Host Linker Driver]
        │ Invocates host linker (MinGW ld / Apple ld64 / GNU ld)
        ▼
Standalone Native Executable (.exe / ELF / Mach-O)
```

---

## 2. Invariant 1: Eliminating Transitive Import Bloat (Tree-Shaking & Reachability)

### 2.1 The Problem: Transitive Dependency Explosion
When an Alya program imports a high-level package (e.g. `import "alya-lang/http"`), the import resolution graph pulls in all underlying transitive modules:
- `http` -> depends on `net`, `crypto` (AES, SHA, HMAC), `compress` (Zlib, Brotli), and `event` (libuv bindings).
- The whole-program AST is populated with **over 500 functions and tens of thousands of AST nodes**, even if the user application only makes a 40-line HTTP cookie test.
- **The Failure Mode Without Tree-Shaking**:
  Compiling and emitting assembly for all 500+ uncalled transitive functions results in **over 129,000 lines of emitted assembly** for a 40-line program, causing massive binary bloat and multi-second compilation times.

### 2.2 The Invariant: Pre-Inference Root-Based Tree-Shaking
Before Type Inference (Pass 5) and Code Generation (Pass 6), the compiler MUST execute a Reachability Analysis sweep:

1. **Identify Roots**:
   - For applications: The entry point `main()` function.
   - For test suites (`alya test`): All `test "..." ... end` blocks and functions decorated with `@test`.
   - For libraries: All symbols marked with `pub` or `@export`.
2. **Breadth-First / Depth-First Traversal (BFS/DFS)**:
   - Traverse the call-graph starting from root symbols.
   - Mark every called function, referenced method, instantiated struct, and used global variable as `REACHABLE`.
3. **Dead-Code Elimination (DCE)**:
   - Completely strip all unreached function declarations and unused type definitions from the active AST.
   - **Result**: A 40-line application that only calls `http.parse_cookie()` compiles ONLY `parse_cookie` and its direct dependencies (~15 functions total), reducing emitted assembly from 129,000+ lines to under 2,000 lines.

---

## 3. Invariant 2: Linear O(N) CallIndex vs O(N³) Recursive AST Traversal

### 3.1 The Problem: The O(N³) Recursive Walk Catastrophe
In languages with local type inference and flexible signatures, function parameter types often depend on how and where those functions are called across the codebase.

- **The Naive Traversal Anti-Pattern**:
  In unoptimized compiler architectures, the type inference engine runs multiple iterations (e.g. 5 passes). In each iteration, for every function, and for every parameter, the compiler recursively walks the entire AST (`collect_all_call_args`, `find_call_arg`) to locate call sites.
- **The Algorithmic Cost**:
  $$\text{Complexity} = \text{Passes} \times \text{Functions} \times \text{Parameters} \times \text{AST Nodes}$$
  $$5 \times 500 \text{ functions} \times 3 \text{ params} \times 5,000 \text{ nodes} = \mathbf{37,500,000\text{ to }100,000,000+\text{ recursive node visits!}}$$
  This naive recursive searching caused compile times to degrade from sub-second to **26+ seconds** for simple test suites.

### 3.2 The Invariant: The Single-Pass `CallIndex` Table
Alya mandates that type inference MUST NOT perform repeated recursive AST traversals. Instead, Pass 3 constructs an in-memory `CallIndex` data structure in a single linear $O(N)$ sweep over the AST:

```rust
// Canonical Rust implementation structure inside the compiler:
pub struct CallIndex {
    // Maps function/method symbol name to all observed argument expressions across call sites
    calls_by_symbol: HashMap<String, Vec<Vec<Expr>>>,
    // Maps struct constructor calls to field initialization expressions
    struct_instantiations: HashMap<String, Vec<HashMap<String, Expr>>>,
}
```

#### Operational Workflow:
1. **Pass 3 (Index Construction - $O(N)$)**:
   - Performs a single linear visit through all AST statements.
   - Whenever an `Expr::Call { callee, args }` is encountered, it records `args` into `calls_by_symbol[callee]`.
   - Takes $< 5\text{ ms}$ even on very large ASTs.
2. **Pass 5 (Type Inference Queries - $O(1)$)**:
   - When resolving the parameter types of a function `fn process(data, options)`, the inference engine does NOT walk the AST.
   - It performs an instant $O(1)$ hash map lookup: `call_index.get("process")`.
   - Iterates through the pre-collected call arguments and infers types directly.
3. **Performance Impact**:
   - Replaces 100,000,000 recursive tree walks with direct memory lookups.
   - **Codegen and inference latency drops from 26 seconds to $< 100\text{ ms}$ ($< 0.1\text{ s}$)**.

---

## 4. Hardware-Aware Codegen Invariants (Pass 6)

When emitting GNU/Mach-O assembly, the code generator must apply machine-level micro-optimizations directly:

1. **Branch Fusion**:
   - Compare and jump statements inside `if` and `while` conditions must be fused into a single relational jump (`cmp` + `jge`/`jl`) without generating intermediate boolean byte flags.
2. **Single Unsigned Bounds Checks**:
   - Array index checks (`arr[i]`) must use a single unsigned comparison (`jae` on x86/x64, `b.hs` on ARM64) to validate both negative index bounds and length overflow in a single CPU cycle.
3. **Zero-Cycle Register Zeroing**:
   - Emits `xor %eax, %eax` on x86/x64 instead of `mov $0, %rax`, taking advantage of CPU register-renaming zero-latency idioms.
4. **Compile-Time Struct Field Offsets**:
   - Field access (`point.x`, `user.id`) must resolve to static byte offsets at compile time, eliminating runtime hash-map or dictionary indirection.

---

## 5. Pipeline Performance Benchmarks & Targets

Every release of the `alya` compiler is verified against these operational performance budgets:

| Compilation Phase | Time Budget (10k LOC) | Algorithmic Complexity |
|---|---|:---:|
| **Pass 1: Lexing & Parsing** | $< 15\text{ ms}$ | $O(N)$ |
| **Pass 2: Whole-Program Module Aggregation** | $< 10\text{ ms}$ | $O(M)$ where $M$ is file count |
| **Pass 3: CallIndex Construction** | $< 5\text{ ms}$ | $O(N)$ |
| **Pass 4: Tree-Shaking Reachability (DCE)** | $< 10\text{ ms}$ | $O(V + E)$ on CallGraph |
| **Pass 5: Linear Type Inference** | $< 25\text{ ms}$ | $O(K)$ over reachable symbols |
| **Pass 6: Direct Codegen (x64 / ARM64)** | $< 30\text{ ms}$ | $O(N_{\text{reachable}})$ |
| **Pass 7: Host Linker Driver** | $< 40\text{ ms}$ | External host OS latency |
| **Total Compilation Cycle** | **$< 135\text{ ms}$** | **Phenomenal DX** |
