# Alya Language Specification (Spec) & Architectural Constitution

> **"Go's engineering robustness meets Ruby & Python's syntactic elegance."**

This repository serves as the official, authoritative language specification, architectural blueprint, and canonical reference suite for the **Alya** programming language.

---

## 🧭 Executive Summary & Core Manifesto

Modern software development often forces an uncomfortable tradeoff:
- **Dynamic Scripting Languages (Python, Ruby)** offer world-class developer happiness, natural readability, and rapid prototyping—at the cost of heavy interpreters, high memory footprints, GIL bottlenecks, runtime type surprises, and sluggish execution.
- **Systems & Modern Static Languages (C++, Rust, Go)** offer bare-metal speed, static predictability, and low resource usage—often at the cost of verbose syntax, complex type gymnastics, ceremonial boilerplate, or steep cognitive overhead.

**Alya was created to eliminate this dichotomy.**

Alya combines the natural, expressive, human-friendly ergonomics of **Python and Ruby** with the unwavering pragmatism, predictable performance, and mechanical sympathy of **Go**. It compiles directly to native machine code without intermediate runtimes or virtual machines, delivering immediate startup times, low memory consumption, and near-C execution velocity.

```text
               ┌────────────────────────────────────────────────────────┐
               │                     THE ALYA AXIS                      │
               └───────────────────────────┬────────────────────────────┘
                                           │
          ┌────────────────────────────────┴────────────────────────────────┐
          ▼                                                                 ▼
 ┌──────────────────────────────────┐                     ┌──────────────────────────────────┐
 │      SYNTACTIC ELEGANCE          │                     │      ENGINEERING ROBUSTNESS      │
 │       (Ruby & Python)            │                     │               (Go)               │
 ├──────────────────────────────────┤                     ├──────────────────────────────────┤
 │ • Clean 'end' blocks, no braces  │                     │ • Strict signature boundaries    │
 │ • Natural keywords: say, when    │                     │ • Pure data structs + UFCS       │
 │ • Expressive pattern matching    │                     │ • Deterministic 'defer' teardown │
 │ • English logic: and, or, not    │                     │ • Zero-middleware native codegen │
 │ • String interpolation: f"{x}"   │                     │ • Single-binary toolchain        │
 │ • Slicing & ranges: 0..10        │                     │ • Standard library discipline    │
 └──────────────────────────────────┘                     └──────────────────────────────────┘
```

---

## 🏛️ The Two Halves of Alya's Soul

### Part I: The Syntactic Elegance of Ruby & Python

Alya believes that code is read far more often than it is written. Syntax should serve human understanding rather than compiler convenience:

1. **Zero Visual Noise**: No mandatory semicolons, no curly brace forests (`{}`), and no redundant parentheses around conditions. Blocks are opened naturally and closed with `end`.
2. **First-Class Expressiveness**:
   - `say` statement for immediate, clean terminal output without ceremonial imports.
   - Formatted string interpolation (`f"User #{user.id}: {user.name}"`).
   - Range notation (`0..10` and `start..end`) for intuitive sequences and slicing.
   - Native membership operators (`in` / `not in`) for arrays, strings, and maps.
   - Readable logical operators (`and`, `or`, `not`) with proper short-circuiting.
   - Null-safety conveniences (`??` null coalescing, `?.` optional navigation).
3. **Pattern Matching (`when`)**:
   - Subsumes both `switch` and conditional cascades.
   - Supports targeted matching (`when x / is 1, 2 / is 3..10 / else`), relational predicates (`is > 100`), and argumentless boolean evaluation.
   - Usable both as an imperative statement and as a functional expression (`let grade = when ...`).

### Part II: The Rigor & Engineering Robustness of Go

Where dynamic languages surrender predictability, Alya enforces industrial discipline:

1. **Strict Signature Boundaries**: Function parameters, return types, and struct fields must declare explicit types. Public API contracts are statically validated at compile time.
2. **Deterministic Local Type Inference**: Inside function and block scopes, local variables enjoy clean, zero-ambiguity type inference. No dynamic type guessing at runtime.
3. **Composition Over Inheritance**: Alya completely rejects classical classes, deep inheritance trees, polymorphic virtual tables, and fragile base classes. Instead:
   - **Pure Data Structs**: Structs contain state and define precise memory layout.
   - **Uniform Function Call Syntax (UFCS)**: Methods are pure functions scoped to the struct (`function Point.distance(self, other)`), callable seamlessly via `point.distance(other)`.
4. **Deterministic Teardown (`defer`)**: Resources (sockets, file descriptors, memory buffers) are released via LIFO `defer` statements executed reliably on scope exit, whether completing normally or through an exception.
5. **One Obvious Way**: Design choices are deliberate. Alya resists adding duplicate syntax for the same semantic concept, preventing codebase fragmentation.

---

## ⚙️ Architecture & Direct-to-Assembly Native Engine

Unlike modern languages that depend on LLVM, JVM, or heavy intermediate representations (IR), Alya utilizes a clean, bespoke, direct-to-assembly compiler architecture:

```text
Source Code (.alya)
        │
   [Pass 1: Lexer & Parser]       ──► Parallel file tokenization and AST construction
        │
   [Pass 2: Whole-Program AST]    ──► Aggregates imports (std/*, local files, packages)
        │
   [Pass 3: CallIndex Construction]──► O(N) single-pass indexing: HashMap[Symbol, Args]
        │
   [Pass 4: Tree-Shaking (DCE)]   ──► BFS reachability sweep from roots (main, @test, @export)
        │
   [Pass 5: Linear Type Inference]──► O(1) CallIndex lookups (eliminates O(N³) recursive tree walks)
        │
   [Pass 6: Code Generator]       ──► Direct GNU/Mach-O assembly (x64, ARM64, x86)
        │
   [Pass 7: Linker Driver]        ──► Native host linker produces final standalone binary
        │
Standalone Native Executable (.exe / ELF / Mach-O)
```

> 📖 **Read the Complete Engine Specification:** [`COMPILER_PIPELINE.md`](COMPILER_PIPELINE.md) documents the algorithmic invariants: how **Tree-Shaking** eliminates transitive import bloat (e.g. from `http` and `libuv`), and how the **$O(N)$ `CallIndex` table** replaced $O(N^3)$ recursive AST walks to drop compilation times from 26 seconds to $<0.1$ seconds.

### Key Low-Level Engineering Highlights

- **Extreme Compilation Throughput**: The Rust-based parser, $O(N)$ CallIndex table, and single-pass codegen achieve **~2,000,000 lines/second** throughput, providing sub-second test-and-run turnaround.
- **Root-Based Tree-Shaking (DCE)**: Prunes unused transitive library functions before type inference and codegen, preventing package bloat.
- **Direct Multi-Target Codegen**: Emits native assembly for:
  - **x64 (x86_64)**: System V AMD64 (Linux, macOS) & Microsoft x64 ABI (Windows MinGW).
  - **ARM64 (aarch64)**: Apple Silicon Mach-O & Linux ELF64.
  - **x86 (i686)**: 32-bit compatibility mode.
- **Hardware-Aware Codegen Optimizations**:
  - **Branch Fusion**: Fuses comparisons and jumps in conditional paths (`cmp` + `jge`), eliminating redundant intermediate flags.
  - **Single Unsigned Bounds Checks**: Compares array indices with a single unsigned check (`jae` / `b.hs`), capturing negative numbers and overflow in one cycle.
  - **Zero-Cycle Register Zeroing**: Emits `xor %eax, %eax` on x86/x64, recognized directly in the CPU register rename stage with zero latency.
  - **Compile-Time Struct Offsets**: Multi-pass fixed-point inference statically resolves struct field offsets, eliminating runtime dictionary lookups.
- **Memory Model**:
  - Primitives (`int`, `float`, `bool`) are stack-allocated, copy-by-value.
  - Heap composites (`string`, `array`, `map`, `struct`) are managed via deterministic **Automatic Reference Counting (ARC)** and high-speed Arena allocators.

---

## 📦 Standard Library vs Package Architecture: 3-Tier Governance

To ensure compiler binary lightness, rapid community evolution, zero duplicate "toy clones", and zero external runtime dependencies, Alya organizes code according to a strict boundary:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                          ALL ALYA MODULES                              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
        ┌───────────────────────────┴────────────────────────────┐
        ▼                                                        ▼
 [CORE STDLIB (std/*) - 14 MODULES]             [STANDALONE PACKAGES (alya-lang/*)]
 Embedded in compiler binary; zero deps.         Decoupled domain packages via alya.toml.
 Fully supports bare-metal with --no-std.        No duplicate "toy" versions in stdlib.
 ───────────────────────────────────────         ─────────────────────────────────────────
 • std/fs, std/path, std/os                      • http, url, jwt, websocket
 • std/process, std/io, std/net                  • crypto (SHA, AES, HMAC), uuid
 • std/sync, std/time, std/mem                   • cli (argparse, subcommands), logger
 • std/math (with PRNG), std/str                 • json, toml, yaml, csv
 • std/collections, std/console, std/test        • sql (sqlite, postgres), compress (zstd)
```

1. **Core Standard Library (`std/*`)**:
   - Embedded directly into the `alya` binary.
   - Strictly limited to operating system syscall abstractions, memory management, and runtime primitives.
   - Zero external dependencies. Complete cross-platform parity (Linux, macOS, Windows).
   - Can be completely omitted for bare-metal / embedded development via `--no-std`.
2. **Standalone Ecosystem Packages (`Lib/*` / `alya-lang/*`)**:
   - Protocol-heavy, rapidly evolving libraries (HTTP, JSON DOM, Crypto, CLI parsing, SQL engines) live purely as packages.
   - Managed declaratively through `alya.toml`, locked deterministically in `alya.lock` with SHA-256 cryptographic verification.
   - Global package caching in `~/.alya/cache` with zero-network cloning.

> 📖 **Read the Complete Boundary Governance:** [`STDLIB_PKG_ARCHITECTURE.md`](STDLIB_PKG_ARCHITECTURE.md) documents the 3-tier classification model, the 100-line pruning rule, deduplication audit, and the 6 golden anti-duplication rules between `std/*` and `Lib/*`.

---

## 🔌 Foreign Function Interface (C FFI Engine)

Alya provides native C interop with zero glue code or manual C wrappers:

```alya
extern "C" from "sqlite3"
    function sqlite3_libversion() -> str
    function sqlite3_open(filename: str, ppDb: ptr) -> i32
end

extern "C"
    function puts(message: str) -> i32
    function abs(n: i32) -> i32
end
```

- Automatic platform ABI conformity (System V AMD64, Microsoft x64, ARM64 AAPCS).
- Type-safe C scalar declarations (`i8`..`i64`, `u8`..`u64`, `f32`, `f64`, `ptr`, `str`, `void`).
- Idiomatic encapsulation: Foreign bindings are wrapped in safe, ergonomic Alya functions.

---

## 🗺️ The 5 Strategic Pillars & Roadmap to 1.0

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                             ALYA ROADMAP                                 │
├─────────────────────┬────────────────────┬───────────────────────────────┤
│ 1. Package Manager  │ 2. C FFI Engine    │ 3. Language Server (LSP)     │
│    (alya pkg)   [✅]│    (extern "C")[✅]│    (Editor Intel & IDEs) [✅] │
├─────────────────────┴────────────────────┴───────────────────────────────┤
│ 4. Memory Resilience & Cycle Detection (Weak Refs & Graph Reclamation)[✅]│
├──────────────────────────────────────────────────────────────────────────┤
│ 5. Concurrency Strategy: "Colorless Concurrency" & Reactor Event Loop [✅]│
└──────────────────────────────────────────────────────────────────────────┘
```

1. **Pillar 1: Package Manager (`alya pkg`)** ✅
   - Declarative manifests (`alya.toml`), cryptographic lockfiles (`alya.lock`), global caching (`~/.alya/cache`), and transitive BFS dependency resolution.
2. **Pillar 2: Foreign Function Interface (`extern "C"`)** ✅
   - Seamless C ABI calling conventions, dynamic library linking (`from "lib"`), zero-copy marshalling.
3. **Pillar 3: Language Server Protocol (LSP)** ✅
   - Complete IDE support: autocompletion, real-time diagnostics, hover types, definition jumping, and workspace refactoring.
4. **Pillar 4: Memory Resilience & Cycle Detection** ✅
   - Cycle-safe ARC runtime: weak references (`weak`), graph-cycle sweep reclamation, and zero-allocation memory arenas.
5. **Pillar 5: Concurrency Strategy ("Colorless Concurrency")** ✅
   - Asynchronous I/O without the "function color" problem (no `async`/`await` bifurcation).
   - High-throughput reactor event loop powering lightweight green fibers over native OS thread pools.

> 🚀 **Compiler Implementation Master Plan:** Consult [`ROADMAP.md`](ROADMAP.md) for the 6-phase engineering checklist (Phases 0 through 5), tracking every compiler task from foundation and CallIndex $O(N)$ inference to stdlib consolidation and LSP.

---

## 🛡️ Existing Compiler Assets & Proven Strengths

Before redesigning or expanding language features, the design team maintains a strict inventory of capabilities that already work with exceptional efficiency in the current compiler.

> 📖 **Read the Full Asset Audit:** [`COMPILER_STRENGTHS.md`](COMPILER_STRENGTHS.md) documents the existing `alya` toolchain's battle-tested features (direct multi-arch GNU/Mach-O assembly, branch fusion, single unsigned bounds checks, zero-cycle idioms, ~2M lines/sec throughput, embedded package manager, and single-binary tooling) that must be preserved and guarded against regression.

---

## 📚 Specification Chapters & Canonical Syntax Matrix

Every syntax rule in Alya is specified with formal grammar rules (EBNF) and accompanied by canonical test fixtures:

| # | Chapter / Domain | Specification Contract | Canonical Syntax Fixture | Status |
|:---:|---|---|---|:---:|
| **00** | **Lexical Structure & Literals** | [`chapters/00_lexical_structure.md`](chapters/00_lexical_structure.md) | [`syntax/lexical.alya`](syntax/lexical.alya) | ✅ Complete |
| **01** | **Variables, Mutability & Constants** | [`chapters/01_variables.md`](chapters/01_variables.md) | [`syntax/variables.alya`](syntax/variables.alya) | ✅ Complete |
| **02** | **Types & Type System** | [`chapters/02_types.md`](chapters/02_types.md) | [`syntax/types.alya`](syntax/types.alya) | ✅ Complete |
| **03** | **Operators & Expressions** | [`chapters/03_operators.md`](chapters/03_operators.md) | [`syntax/operators.alya`](syntax/operators.alya) | ✅ Complete |
| **04** | **Conditionals (`if` / `elif` / `else`)** | [`chapters/04_conditionals_if.md`](chapters/04_conditionals_if.md) | [`syntax/if.alya`](syntax/if.alya) | ✅ Complete |
| **05** | **Pattern Matching (`when`)** | [`chapters/05_pattern_matching_when.md`](chapters/05_pattern_matching_when.md) | [`syntax/when.alya`](syntax/when.alya) | ✅ Complete |
| **06** | **Loops & Iteration (`while`, `for`, `repeat`)** | [`chapters/06_loops.md`](chapters/06_loops.md) | [`syntax/loops.alya`](syntax/loops.alya) | ✅ Complete |
| **07** | **Functions, Closures & `defer`** | [`chapters/07_functions.md`](chapters/07_functions.md) | [`syntax/functions.alya`](syntax/functions.alya) | ✅ Complete |
| **08** | **Structs & Methods (Object Model)** | [`chapters/08_structs.md`](chapters/08_structs.md) | [`syntax/structs.alya`](syntax/structs.alya) | ✅ Complete |
| **09** | **Enums & Discriminants** | [`chapters/09_enums.md`](chapters/09_enums.md) | [`syntax/enums.alya`](syntax/enums.alya) | ✅ Complete |
| **10** | **Error Handling (`try` / `catch` / `throw`)** | [`chapters/10_error_handling.md`](chapters/10_error_handling.md) | [`syntax/error_handling.alya`](syntax/error_handling.alya) | ✅ Complete |
| **11** | **Modules, Packages & Visibility (`pub`)** | [`chapters/11_modules.md`](chapters/11_modules.md) | [`syntax/modules.alya`](syntax/modules.alya) | ✅ Complete |
| **12** | **FFI & Low-Level Interop (`extern`)** | [`chapters/12_ffi.md`](chapters/12_ffi.md) | [`syntax/ffi.alya`](syntax/ffi.alya) | ✅ Complete |
| **13** | **Collections, Slicing & Comprehensions** | [`chapters/13_collections_and_slicing.md`](chapters/13_collections_and_slicing.md) | [`syntax/collections.alya`](syntax/collections.alya) | ✅ Complete |
| **14** | **Interfaces & Structural Polymorphism** | [`chapters/14_interfaces.md`](chapters/14_interfaces.md) | [`syntax/interfaces.alya`](syntax/interfaces.alya) | ✅ Complete |
| **15** | **Generics & Parametric Polymorphism** | [`chapters/15_generics.md`](chapters/15_generics.md) | [`syntax/generics.alya`](syntax/generics.alya) | ✅ Complete |
| **16** | **Memory Model, ARC & Lifecycle** | [`chapters/16_memory_model.md`](chapters/16_memory_model.md) | [`syntax/memory.alya`](syntax/memory.alya) | ✅ Complete |
| **17** | **Concurrency, Fibers & Channels** | [`chapters/17_concurrency.md`](chapters/17_concurrency.md) | [`syntax/concurrency.alya`](syntax/concurrency.alya) | ✅ Complete |
| **18** | **Attributes, Directives & Comptime** | [`chapters/18_attributes_and_comptime.md`](chapters/18_attributes_and_comptime.md) | [`syntax/attributes.alya`](syntax/attributes.alya) | ✅ Complete |
| **19** | **Null Safety & Optionals (`T?`)** | [`chapters/19_null_safety.md`](chapters/19_null_safety.md) | [`syntax/null_safety.alya`](syntax/null_safety.alya) | ✅ Complete |
| **20** | **Operator Overloading & Special Methods** | [`chapters/20_operator_overloading.md`](chapters/20_operator_overloading.md) | [`syntax/operators_overloading.alya`](syntax/operators_overloading.alya) | ✅ Complete |
| **21** | **Strings, Unicode & Runes** | [`chapters/21_strings_and_unicode.md`](chapters/21_strings_and_unicode.md) | [`syntax/strings_unicode.alya`](syntax/strings_unicode.alya) | ✅ Complete |
| **22** | **Testing, Assertions & Benchmarking** | [`chapters/22_testing_and_benchmarking.md`](chapters/22_testing_and_benchmarking.md) | [`syntax/testing.alya`](syntax/testing.alya) | ✅ Complete |
| **23** | **Standard Library Core Contracts (Tier-1)** | [`chapters/23_stdlib_core_contracts.md`](chapters/23_stdlib_core_contracts.md) | [`syntax/stdlib_contracts.alya`](syntax/stdlib_contracts.alya) | ✅ Complete |
| **24** | **Toolchain, CLI & Package Manager (`alya`)** | [`chapters/24_toolchain_and_cli.md`](chapters/24_toolchain_and_cli.md) | [`syntax/toolchain_example.alya`](syntax/toolchain_example.alya) | ✅ Complete |

---

## ⚖️ The "Spec-First" Development Constitution

To prevent parser fragility, compiler regressions, and ad-hoc feature sprawl:

1. **No Implementation Without Specification**: No new keyword, token, or grammar rule may be added to the `alya` compiler before it is documented in this specification repository with EBNF rules and accepted by the design team.
2. **Golden Test Fixtures**: All files in `syntax/*.alya` serve as the golden test suite. Every release of `alya` must parse and compile these files without error.
3. **Semantic Versioning & Evolution**: Syntax breaking changes are strictly forbidden between minor compiler versions once a specification chapter reaches `Stable` status.
