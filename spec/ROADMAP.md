# Alya Compiler Implementation Roadmap (v1.0 Spec Alignment)

> **Master Execution Plan: Bridging the Formal Language Specification (`Src/spec`) and the Native Compiler Implementation (`Src/alya`).**

This roadmap defines the sequenced implementation phases, actionable engineering tasks, and verification criteria required to bring the existing `alya` compiler into 100% compliance with the Alya Language Specification.

---

## 🗺️ Roadmap Architecture & Phase Progression

```text
┌────────────────────────────────────────────────────────────────────────┐
│                   ALYA COMPILER ROADMAP TO SPEC v1.0                   │
├────────────────────────────────────────────────────────────────────────┤
│ [Phase 0] Foundation & Alignment (Binary Name, Golden Test Harness)   │
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 1] Algorithmic Pipeline & Scalability (O(N) CallIndex, Tree-Shake)
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 2] Standard Library Consolidation & Modern Syntax (14 Modules)  │
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 3] Grammar, Lexer & Parser Conformance (Rune, 15-Level Pratt, Main)
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 4] Advanced Runtime & Systems (Weak ARC, Fibers, Cycle Collector)
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 5] Tooling & Ecosystem (LSP, DocGen, Linter, Resolution, Packages) │
├──────────────────────────────────┬─────────────────────────────────────┤
│                                  ▼
│ [Phase 6] Future Explorations (WebAssembly, GUI, SIMD, Gradual Typing)  │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 📋 Phase 0: Foundation & Toolchain Alignment

- [x] **0.1 Binary Naming Standard (`alya` as Primary Binary)**
  - Update `Src/alya/Cargo.toml` to define `name = "alya"` as the primary executable.
  - Maintain `alyac` as an alias or secondary entry point for backward compatibility.
  - Target files: `Cargo.toml`, `src/main.rs`.
  - Verification: `cargo build` produces `target/debug/alya` (and `alya.exe` on Windows).

- [x] **0.2 CLI Subcommand Unification**
  - Verify all primary CLI commands respond consistently under `alya`:
    - `alya run [file]`
    - `alya build [file] [--release] [-o output]`
    - `alya check [file]`
    - `alya test [path]`
    - `alya bench [path]`
    - `alya fmt [path]`
    - `alya repl`
    - `alya doc [path]`
    - `alya pkg [init|add|install|update|cache]`
  - Target files: `src/cli/mod.rs`, `src/driver/mod.rs`.
  - Verification: `alya --help` displays modern unified command list.

- [x] **0.3 Golden Spec Test Harness**
  - Create an end-to-end integration test (`tests/golden_spec_tests.rs`) in `Src/alya`.
  - The runner iterates over all 25 canonical test fixtures in `Src/spec/syntax/*.alya` and asserts successful compilation/execution.
  - Target files: `tests/golden_spec_tests.rs`.
  - Verification: `cargo test --test golden_spec_tests`.

---

## ⚡ Phase 1: Algorithmic Pipeline & Scalability (`COMPILER_PIPELINE.md`)

- [x] **1.1 Pass 3: Linear O(N) `CallIndex` Table Construction**
  - Construct an in-memory `CallIndex` hash table during Pass 3 of the compiler pipeline:
    `HashMap<String, Vec<Vec<Expr>>>` (mapping symbol names to observed argument expressions across all call sites).
  - Refactor `src/codegen/analysis/inference/` (`strings.rs`, `floats.rs`, `arrays.rs`, `maps.rs`):
    Replace nested multi-pass recursive AST scans (`collect_all_call_args`, `find_call_arg`) with $O(1)$ `CallIndex` lookups.
  - Target files: `src/codegen/analysis/inference/mod.rs`, `common.rs`, `strings.rs`, `floats.rs`, `arrays.rs`, `maps.rs`.
  - Verification: Inference latency drops from multi-second scans to $< 25\text{ ms}$; all existing 289 tests remain green.

- [x] **1.2 Pass 4: Root-Based Reachability & Tree-Shaking (DCE)**
  - Implement a call-graph reachability traversal starting from identified roots:
    - Executable entry points: `main()`.
    - Test runners: all `test "..." ... end` blocks and functions with `@test`.
    - Libraries: all `pub` or `@export` symbols.
  - Prune unreferenced functions, structs, and methods before type inference and codegen.
  - Target files: `src/codegen/analysis/dce.rs`, `src/codegen/mod.rs`.
  - Verification: Transitive packages (e.g. `http` pulling in `crypto` and `libuv`) emit $< 2,000$ lines of assembly instead of $> 120,000$ lines for small entry programs.

- [x] **1.3 Compilation Phase Latency Instrumentation (`--time`)**
  - Measure and report per-pass latency breakdown in `alya build --time` according to `COMPILER_PIPELINE.md` complexity budgets ($< 135\text{ ms}$ for 10k LOC).
  - Target files: `src/driver/mod.rs`.
  - Verification: Output shows breakdown for Lexer, Parser, Whole-Program AST, CallIndex, DCE, Inference, Codegen, Linker.

---

## 📦 Phase 2: Standard Library Consolidation (14 Core Modules)

- [x] **2.1 Prune & Decouple High-Level Protocols**
  - Decouple `cli.alya` and `log.alya` from stdlib (relegate to `alya-lang/cli` and `alya-lang/logger`).
  - Merge `color.alya` ANSI formatting into `std/console`.
  - Merge `glob.alya` into `std/path` and `std/fs`.
  - Merge `rand.alya` into `std/math` (`math.random()`, `math.rand_int()`).
  - Merge `thread.alya` into `std/sync`.
  - Merge `bench.alya` into `std/test`.
  - Target files: `Src/alya/stdlib/`.
  - Verification: Clean 14-module directory structure matching Chapter 23.

- [x] **2.2 Implement Missing Core Modules**
  - **`std/process`**: Subprocess spawning (`process.run`, `process.spawn`), argument passing, stdout/stderr piping, exit codes.
  - **`std/io`**: Standard stream handles (`io.stdin`, `io.stdout`, `io.stderr`), `Reader`/`Writer` interfaces, `io.copy`, `io.read_all`.
  - Target files: `Src/alya/stdlib/process.alya`, `Src/alya/stdlib/io.alya`.
  - Verification: E2E tests validating subprocess execution and stream piping.

- [x] **2.3 Bare-Metal / Embedded Mode (`--no-std`)**
  - Add `--no-std` compiler flag to detach standard library embedding and omit runtime initialization stubs.
  - Target files: `src/cli/mod.rs`, `src/driver/mod.rs`, `src/codegen/mod.rs`.
  - Verification: Compiling with `--no-std` emits assembly with zero libc or stdlib symbols.

- [x] **2.4 Standard Library Modern Syntax Refactoring**
  - Idiomatically refactor all 14 core standard library modules (`std/fs`, `std/path`, `std/os`, `std/process`, `std/io`, `std/net`, `std/sync`, `std/time`, `std/mem`, `std/math`, `std/str`, `std/collections`, `std/console`, `std/test`) to adopt v1.0 syntax idioms:
    - Deep destructuring patterns (`let { x, y } = point`, `let [first, ...rest] = list`).
    - Tagged unions for algebraic return types (`type Result[T, E] = Ok(T) | Err(E)`).
    - Comptime evaluations and assertions (`comptime ... end`).
    - Structured exception hierarchies (`try ... catch err: IOError`).
    - Pattern matching with `when` (relational predicates, ranges, value bindings).
  - Target files: `Src/alya/stdlib/*.alya`.
  - Verification: All stdlib tests and 20 golden spec tests pass with modern syntax without regressions.

---

## 🔤 Phase 3: Grammar, Lexer & Parser Conformance

- [x] **3.1 44 Master Reserved Keywords**
  - Audit lexer keyword table against `chapters/00_lexical_structure.md`.
  - Ensure hard keywords (`end`, `defer`, `spawn`, `select`, `weak`, `comptime`, `sizeof`, `alignof`, `typeof`) are strictly reserved.
  - Target files: `src/lexer/mod.rs`, `src/lexer/token.rs`.
  - Verification: Lexer unit tests for all 44 keywords.

- [x] **3.2 15-Level Pratt Expression Parser Conformance**
  - Verify and align expression parsing binding powers with `chapters/03_operators.md`:
    - Relational (`<`, `<=`, `>`, `>=`, `in`, `not in`, `is`).
    - Bitwise (`&`, `^`, `|`).
    - Logical (`and`, `or`).
    - Null coalescing (`??`) short-circuiting right-associative.
    - Ternary (`?:`) right-associative.
  - Target files: `src/parser/expr.rs`.
  - Verification: Precedence test cases matching `syntax/operators.alya`.

- [x] **3.3 32-Bit Unicode `rune` Literal Support**
  - Add single-quote character literal parsing: `'⌘'`, `'🚀'`, `'ğ'` as 32-bit `rune` codepoints.
  - Codegen emission for `rune` operations and `char_count()` vs `byte_length()`.
  - Target files: `src/lexer/mod.rs`, `src/parser/expr.rs`, `src/codegen/expr.rs`.
  - Verification: `syntax/strings_unicode.alya` compiles and passes assertions.

- [x] **3.4 Extended `when` Pattern Matching**
  - Support range arms (`is 10..20`, `is 10..=20`).
  - Support relational comparison arms (`is < 13`, `is >= 65`).
  - Support argumentless conditional cascade (`when => cond1 => ... else => ...`).
  - Target files: `src/parser/stmt/`, `src/codegen/stmt/`.
  - Verification: `syntax/when.alya` executes with complete branch coverage.

- [x] **3.5 Program Entry Point Protocol (`main` Signatures)**
  - Support 4 permissible `main` signatures:
    1. `function main()` (default exit code 0).
    2. `function main() -> int` (explicit exit code).
    3. `function main(args: string[])` (CLI arguments array).
    4. `function main(args: string[]) -> int` (CLI arguments + explicit exit code).
  - Target files: `src/codegen/mod.rs`, `src/codegen/runtime/`.
  - Verification: Test suite for each `main` signature verifying correct exit codes and argument injection.

---

## 🧬 Phase 4: Advanced Systems Runtime (Memory & Concurrency)

- [x] **4.1 Cycle-Breaking `weak` References**
  - Implement `weak` reference semantics for ARC heap pointers.
  - Provide safe runtime upgrade/downgrade check (returns `null` if referenced object was collected).
  - Target files: `src/ast/types.rs`, `src/codegen/runtime/arc.rs`.
  - Verification: `syntax/memory.alya` directory tree test runs with zero memory leaks.

- [x] **4.2 Lightweight Fibers & "Colorless Concurrency"**
  - Implement `spawn` fiber scheduler over native OS worker thread pools (M:N cooperative runtime).
  - Implement CSP `Channel[T]` (buffered/unbuffered rendezvous) and `select` multiplexer.
  - Target files: `src/codegen/runtime/`, `src/codegen/stmt/`.
  - Verification: `syntax/concurrency.alya` runs and multiplexes channels.

- [x] **4.3 Structural Duck-Typing Interfaces (Fat Pointers)**
  - Implement fat pointer dynamic dispatch (`data_ptr` + `vtable_ptr`) for implicit interface satisfaction.
  - Support dynamic interface querying (`val is Drawable`).
  - Target files: `src/codegen/analysis/`, `src/codegen/expr.rs`.
  - Verification: `syntax/interfaces.alya` runs polymorphic shape benchmarks.

- [x] **4.4 Zero-Cost Generics Monomorphization**
  - Type-parameterized structs (`Stack[T]`) and functions (`swap[T]`).
  - Compile-time interface constraints (`[T: Printable]`).
  - Target files: `src/parser/`, `src/codegen/analysis/`.
  - Verification: `syntax/generics.alya` compiles without runtime boxing.

- [ ] **4.5 Event Engine Thread Pool & Asynchronous Disk I/O (`Lib/EVENT_UV_ROADMAP.md` Phase 5)**
  - Implement offloaded asynchronous file system operations:
    - `fs_read_async(loop, path, on_complete)`
    - `fs_write_async(loop, path, data, on_complete)`
  - Background worker thread pool dispatcher:
    - Worker pool (`std/thread`) processing blocking file I/O and DNS lookups off the main event loop.
    - Thread-safe event notification to the reactor loop via `std/sync` Channels and wakeup event descriptors.
  - Target files: `Lib/uv/`, `Lib/event/`.
  - Verification: Asynchronous disk read/write benchmarks matching `EVENT_UV_ROADMAP.md` criteria.

- [ ] **4.6 Background / Scoped Cycle Collector (Bacon-Rajan Algorithm)**
  - Non-blocking, trial-deletion cycle detection algorithm to identify and sweep isolated cyclic reference islands.
  - Periodic and on-demand traversal sweeping unreachable cyclic graphs without global stop-the-world pauses.
  - Target files: `src/codegen/runtime/arc.rs`, `src/codegen/runtime/gc.rs`.
  - Verification: Cyclic graph structures (e.g. doubly-linked lists and parent-child tree loops) are completely reclaimed with zero leaks.

- [x] **4.7 Memory Diagnostics & Heap Trace Engine (`alya run --mem-trace`)**
  - Implement `--mem-trace` compiler and runtime instrumentation:
    - Live object counters and active allocation tracking.
    - Leak detection reporting upon program termination with allocation site attribution.
  - Target files: `src/codegen/runtime/alloc.rs`, `src/cli/mod.rs`.
  - Verification: `alya run test_alloc.alya --mem-trace` outputs formatted live reference summary and detects deliberate leaks.

- [ ] **4.8 M:N Cooperative Fiber Scheduler & Growable Stacks ("Colorless Concurrency")**
  - Architectural realization of Alya's Colorless Concurrency model (rejecting `async/await` function coloring):
    - Lightweight user-space green fibers with segmented/growable stacks.
    - M:N cooperative scheduler multiplexing thousands of fibers across native OS worker thread pools.
    - Sockets, timers, and channels automatically yield upon waiting, preserving natural, synchronous syntax with asynchronous throughput (`spawn handle_client(sock)`).
  - Target files: `src/codegen/runtime/fiber.rs`, `src/codegen/stmt/control.rs`.
  - Verification: Concurrency stress tests spawning 100,000 active fibers with sub-millisecond scheduling latency.

---

## 🛠️ Phase 5: Developer Tooling & Ecosystem

- [x] **5.1 Language Server Protocol (`alya lsp`)**
  - Real-time diagnostic parser for IDE integration (VS Code, Zed, Neovim).
  - Autocompletion, hover type tooltips, go-to-definition, symbol renaming.
  - Target files: `src/tools/lsp/` (new module).
  - Verification: Connects to VS Code via standard JSON-RPC over stdio.

- [x] **5.2 Automated Documentation Generator (`alya doc`)**
  - Ingestion of `##` Markdown docstrings from source files.
  - Static HTML / Markdown API documentation generation.
  - Target files: `src/tools/doc/` (new module).
  - Verification: Generates full HTML reference website for stdlib.

- [x] **5.3 Decoupled Official Packages Repository (`alya-lang/*`)**
  - Establish official GitHub repositories for decoupled packages:
    `alya-lang/http`, `alya-lang/crypto`, `alya-lang/sqlite`, `alya-lang/cli`, `alya-lang/logger`, `alya-lang/json`.
  - Verification: Packages install cleanly via `alya add alya-lang/http` and verify with SHA-256 in `alya.lock`.

- [ ] **5.4 Package Dependency Engine: Multi-Major Isolation & Diamond Dependency Resolution**
  - Implement Semantic Versioning coalescing for minor/patch dependencies (`^1.1.0` + `^1.4.0` -> `1.4.0`).
  - Major-segregated disk storage (`.alya/packages/<name>-v<major>/`) enabling side-by-side coexistence of distinct major versions (`z-v1` and `z-v2`).
  - Compiler symbol name mangling (`_Alya_<pkg>_v<major>_<symbol>`) preventing duplicate symbol collisions at link time.
  - Strict direct dependency isolation: compile-time rejection of implicit transitive dependency imports.
  - Native C-FFI safety constraint (`links = "<lib>"`): compile-time error detection for duplicate foreign C library linkages.
  - Target files: `src/tools/pkg/commands.rs`, `src/tools/pkg/discovery.rs`, `src/tools/pkg/manifest.rs`, `src/codegen/mod.rs`.
  - Verification: Chapter 24 Section 1.7 spec conformance and multi-major diamond dependency validation.

- [x] **5.5 Template Repository Modernization (`Lib/template`)**
  - **Prerequisite Archetype**: The `Lib/template` (`alya-lang/template`) repository MUST be revised first before any package repos, as it serves as the official package template and archetype.
  - Revise `alya.toml` manifest, canonical layout (`src/`, `tests/`), CI GitHub Actions workflows, documentation templates, and idiomatic v1.0 syntax usage.
  - Target files: `Lib/template/`.
  - Verification: Package generation via `create-package.ps1` produces starter packages matching the modern v1.0 spec standard.

- [ ] **5.6 Ecosystem Standalone Packages Modernization & Clean v0.1.0 Baseline**
  - Apply the modernized `Lib/template` archetype and v1.0 syntax features across all 20+ standalone ecosystem package repos (`Lib/http`, `Lib/crypto`, `Lib/sqlite`, `Lib/json`, `Lib/toml`, `Lib/yaml`, `Lib/event`, `Lib/uv`, `Lib/cli`, `Lib/logger`, `Lib/jwt`, `Lib/uuid`, etc.).
  - **Version Reset to 0.1.0**: Set `version = "0.1.0"` in all package manifests (`alya.toml`).
  - **Remote Tag & Release Purge**: Delete and purge all legacy Git tags and GitHub releases across remote `alya-lang/*` repositories to establish a clean, consistent `v0.1.0` release baseline.
  - Target files: `Lib/*/alya.toml`, `Lib/*/src/`, GitHub repository tags & releases.
  - Verification: All packages compile under modern syntax, install cleanly via `alya add`, and pass automated test suites.

- [ ] **5.7 Static Analysis & Linter Engine (`alya lint` & LSP/VS Code Integration)**
  - Implement standalone static code analysis linter (`src/tools/lint/`):
    - Dead/unreachable code detection following unconditional jumps, `return`, and `throw`.
    - Unused local variables, unreferenced function parameters, and redundant imports.
    - Idiomatic style recommendations (e.g. promoting `when` pattern matching over nested `if/elif`, destructuring tuples/structs).
  - Implement `--fix` flag for automatic in-place code refactoring.
  - Integrate linter rules directly into `alya lsp` diagnostics engine, streaming real-time warnings and quick fixes over JSON-RPC.
  - Update `Src/vscode-alya` client extension with diagnostics rendering, linting settings, and Quick Fix Code Actions.
  - Target files: `src/tools/lint/`, `src/tools/lsp/`, `Src/vscode-alya/`.
  - Verification: `alya lint` CLI test suite and real-time LSP diagnostic verification.

---

## 💡 Phase 6: Future Explorations & Long-Term Targets

- [ ] **6.1 WebAssembly Target (`wasm32-unknown-unknown`)**
  - Direct compilation of Alya source code to WebAssembly binaries (`.wasm`) for browser sandboxes, Cloudflare Workers, and edge compute runtimes.
  - Native WASI (WebAssembly System Interface) runtime stubs for file I/O and console output.
  - Target files: `src/codegen/arch/wasm32/`, `src/driver/mod.rs`.
  - Verification: `alya build app.alya --target wasm32` executes under Node.js / Wasmtime.

- [ ] **6.2 Optional Type Annotations & Static Gradual Typing**
  - Gradual typing syntax: `let x: int = 42`, `function add(a: int, b: int) -> int`.
  - Type-checker validation pass verifying parameter and return type assignments at compile time.
  - Specialized JIT/AOT code generation leveraging explicit scalar types for zero-box register allocation.
  - Target files: `src/parser/`, `src/codegen/analysis/`.
  - Verification: Golden tests for static type mismatch rejections and optimized scalar emission.

- [ ] **6.3 Native GUI Toolkit Integration**
  - Lightweight direct bindings to native platform windowing APIs with zero heavy C++ runtime dependencies:
    - Windows: Win32 API and Direct2D/DirectWrite.
    - macOS: Cocoa / Metal runtime bindings.
    - Linux: Wayland and X11 protocols via native socket IPC.
  - Target files: `Lib/gui/`, `src/tools/bundle.rs`.
  - Verification: Cross-platform hello-world window example with event dispatch loop.

- [ ] **6.4 Explicit SIMD Vectorization Primitives**
  - First-class vector data types (`f64x4`, `f32x8`, `i32x8`, `i64x4`) with native operator overloading (`+`, `-`, `*`, `/`).
  - Direct machine instruction mapping to AVX2/AVX-512 on x64 and Neon on ARM64.
  - Target files: `src/codegen/arch/x64/ops.rs`, `src/codegen/arch/arm64/ops.rs`.
  - Verification: High-throughput Mandelbrot and matrix multiplication benchmarks achieving $> 4\times$ throughput speedup.

---

## 📊 Summary Progress Tracker

| Phase | Milestone | Priority | Status |
|:---:|---|:---:|:---:|
| **0** | **Foundation & Toolchain Alignment** | 🔴 Immediate | ✅ Complete |
| **1** | **Algorithmic Pipeline & Scalability (CallIndex, Tree-Shaking)** | 🔴 Immediate | ✅ Complete |
| **2** | **Standard Library Consolidation & Modern Syntax (14 Modules)** | 🟡 High | ✅ Complete |
| **3** | **Grammar, Lexer & Parser Conformance (Rune, Pratt, Main)** | 🟡 High | ✅ Complete |
| **4** | **Advanced Systems Runtime (Weak ARC, Fibers, Cycle Collector, Event)** | 🔵 Normal | 🟡 In Progress |
| **5** | **Developer Tooling & Ecosystem (Linter, LSP, Resolution, Packages)** | 🟢 Future | 🟡 In Progress |
| **6** | **Future Explorations & Targets (WASM, GUI, SIMD, Gradual Typing)** | 💡 Research | 📋 Planned |
