# Alya Project Roadmap

This document outlines the evolutionary milestones, architectural goals, and feature roadmap for the **Alya** programming language.

---

## Vision & Philosophy

Alya is designed to balance the ergonomics of an expressive, readable language with the raw performance of a direct-to-assembly native compiler.

* **Zero Middleware**: Native GNU/Mach-O assembly generation without LLVM or intermediate representations (IR).
* **Batteries Included**: Comprehensive standard library (`std/net`, `std/thread`, `std/fs`, `std/mem`...) requiring zero external runtime dependencies.
* **Instant Productivity**: Built-in developer tooling (`fmt`, `test`, `repl`, `--time`, `--bundle`) embedded directly into the single `alyac` binary.

---

## Status Overview

| Status | Meaning |
| :---: | :--- |
| ✅ | **Completed** — Implemented, verified with unit/e2e tests, and available in stable releases. |
| 🚧 | **In Progress** — Active implementation or architecture design underway. |
| 📋 | **Planned** — Core roadmap target with prioritized technical specification. |
| 💡 | **Exploration** — Conceptual research and RFC stage. |

---

## Milestone Progress

### 🟢 Completed Milestones (v0.0.5)

- [x] **Core Language Specification & Parser** ✅
  - Dynamic type system with compile-time struct type inference.
  - Expressive control flow (`if`/`elif`/`else`, `when` pattern matching, `while`, `for .. in`, `repeat N`).
  - First-class functions, default parameters, multi-return values, and recursion.
  - Robust exception handling (`try ... catch ... finally`, `throw`).
  - Composite data structures: dynamic arrays, hash maps, structs, and tuples.
- [x] **Direct Native Assembly Codegen** ✅
  - Multi-architecture assembly emitters: **ARM64** (Apple Silicon Mach-O & Linux ELF64), **x64** (Windows MinGW, Linux ELF64, macOS Mach-O), and **x86** (32-bit).
  - Codegen optimizations: branch fusion, immediate range splitting (`movz`/`movk`), zero-cycle idioms.
  - Sub-millisecond parser throughput (~2M lines/sec) and near-C execution performance.
- [x] **Batteries-Included Standard Library** ✅
  - Networking (`std/net` with raw TCP/UDP socket I/O and non-blocking `tcp_poll`; HTTP unbundled to `alya-lang/http`).
  - Concurrency (`std/thread` with native OS worker threads; `std/sync` with `Mutex`, `Channel`, `WaitGroup`, `Once`, `RwLock`).
  - System I/O (`std/fs`, `std/path`, `std/os`, `std/time`, `std/console`, `std/color`).
  - Utilities (`std/rand` with core PRNG/LCG, `std/json` basic parser/stringifier, `std/glob`, `std/hash`, `std/collections`, `std/test`, `std/mem` Arena allocator).
  - Official Standalone Packages (`csv`, `url`, `http`, `crypto`, `rand`, `uuid`, `jwt`, `mime`, `cli`, `logger`, `json`, `toml`, `dotenv`, `semver`, `sqlite`).
- [x] **Integrated Tooling & Platform Packaging** ✅
  - In-place code formatter (`alyac fmt`).
  - Test runner (`alyac test`).
  - Interactive REPL shell (`alyac repl`).
  - Windows executable resource embedding (`winres`, `alyac.exe` PE icon and metadata).
  - macOS Application Bundling (`alyac build --bundle`, automatic `Info.plist`, `.app` structure, and multi-resolution Apple `.icns` packaging).
  - Official brand asset suite (`alya-file`, `alyac`, `alya-app`, `alya-icon`).
  - Real-world application collection (`apps/` featuring HTTP server, benchmark tool, port scanner, Conway's Game of Life, Snake, and TicTacToe).

---

## Upcoming Milestones: The 5 Strategic Pillars

> 🚀 **Active Compiler Implementation Roadmap:** All granular phase-by-phase implementation tasks, golden test requirements, and future milestones (Phases 0 through 6) are tracked in [`spec/ROADMAP.md`](spec/ROADMAP.md).

The next evolution of Alya transitions the project from a complete standalone language to an extensible, industry-grade ecosystem.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                             ALYA NEXT LEVEL                              │
├─────────────────────┬────────────────────┬───────────────────────────────┤
│ 1. Package Manager  │ 2. C FFI Engine    │ 3. Language Server (LSP)     │
│    (alyac pkg)      │    (extern "C")    │    (Editor Intel & IDEs)      │
├─────────────────────┴────────────────────┴───────────────────────────────┤
│ 4. Memory Resilience & Cycle Detection (Weak Refs & Graph Reclamation)   │
├──────────────────────────────────────────────────────────────────────────┤
│ 5. Concurrency Strategy: "Colorless Concurrency" & Reactor Event Loop    │
└──────────────────────────────────────────────────────────────────────────┘
```

---

### Pillar 1: Package Manager & Dependency Ecosystem (`alyac pkg`) ✅

Enable community library sharing, versioned dependency resolution, and automated build workflows.

#### Completed Capabilities
- **Project Manifest (`alya.toml`)**:
  - Pure Rust built-in TOML parser and serializer for `[package]` and `[dependencies]`.
  - Supports local path dependencies (`{ path = "..." }`), remote Git repositories (`{ git = "...", tag = "...", branch = "..." }`), and version constraints.
  - Compiler compatibility guard (`alya-version = "0.0.5"`) enforcing minimum required compiler version for packages.
  - Rich package metadata (`homepage`, `repository`, `keywords`, `authors`, `license`).
- **Deterministic Lockfile (`alya.lock`)**:
  - Embedded pure Rust SHA-256 cryptographic verification (FIPS 180-4 / RFC 6234).
  - Reproducible builds recording resolved dependencies, entry points, sources, and content checksums.
- **CLI Subcommands & Shortcuts**:
  - `alyac init [path] [--name <name>] [--lib]`: Generate starter package with `alya.toml`, entry file, and `.gitignore`.
  - `alyac add <name> [--path <path>] [--git <url>] [--tag <tag>] [--branch <branch>]`: Add dependency and automatically lock.
  - `alyac install`: Resolve, fetch, and lock all dependencies declared in `alya.toml`.
  - `alyac pkg [init|add|install|list|update]`: Complete package lifecycle manager.
- **Automatic Entry Point Discovery**:
  - Running `alyac run`, `alyac build`, or `alyac check` without an input file inside any package directory automatically locates `alya.toml` and compiles its designated entry file.
- **Compiler Module Resolution Integration**:
  - Native compiler import engine seamlessly resolves package imports (`import "pkg"` / `import "pkg/sub" as alias`) through the manifest, with clear diagnostic errors directing users to `alyac install` if dependencies are missing.
- **Global Package Cache & Storage (`~/.alya/cache`)**:
  - Global package repository cache storing clean checkouts with `.alya-source` metadata.
  - Zero-network project cloning skipping `.git` overhead (`skip_git: true`) for instant local package provisioning.
  - Storage management commands: `alyac pkg cache`, `alyac pkg clean`, and `alyac pkg cache clean`.
- **Transitive Dependency Resolution (Queue / BFS)**:
  - Recursive multi-tier dependency tree resolution locking all nested packages in a flat, deterministic `alya.lock`.
  - Compiler import resolver walks up parent directories to seamlessly bind transitive sub-dependencies.

---

> 📦 **Standard Library vs Package Architecture:** Detailed 3-tier boundary rules, deduplication governance, and official package ecosystem directories are maintained in [`spec/STDLIB_PKG_ARCHITECTURE.md`](spec/STDLIB_PKG_ARCHITECTURE.md) and [`spec/README.md`](spec/README.md).

---

### Pillar 2: Foreign Function Interface (C FFI Engine) ✅

Enable direct interoperability with existing C, C++, and system libraries without writing glue code or wrappers.

#### Completed Capabilities
- **External Declaration Syntax (`extern "C"`)**:
  - Declaration of foreign C ABI function signatures with parameter types and return type (`extern "C" ... end`).
  - Optional library specification clause (`extern "C" from "lib" ... end`).
  - Native compile-time automatic linker flag appending (`-l<lib>`).
- **Standard Platform ABI Calling Conventions**:
  - Windows x64 (Microsoft x64 Calling Convention with 32-byte shadow space and 16-byte stack alignment).
  - Linux/POSIX x64 (System V AMD64 ABI: `rdi`, `rsi`, `rdx`, `rcx`, `r8`, `r9`).
  - ARM64 AAPCS (`x0`–`x7` registers).
  - x86 (cdecl calling convention).
- **Seamless Zero-Copy Data Marshalling**:
  - Null-terminated string passing (`str` -> `const char*`).
  - Integer and pointer argument passing (`i32`, `i64`, `ptr`, `null`).
  - String return value auto-inference (`fn_ret_str`) for zero-boilerplate `say` and string operations.
- **Linker & Toolchain Integration**:
  - GCC and Clang linker driver integration automatically passing `-l<lib>` dependencies collected from AST.
  - Multi-platform E2E test suite (`tests/e2e_ffi.rs`) verifying native execution on Windows and Linux.
- **Bundled C Source Compilation & Caching Engine (`[build] c-sources`)**:
  - Automated discovery and compilation of embedded C amalgamation sources (e.g. SQLite3).
  - Fast disk caching in `~/.alya/c_obj` reusing `.o` artifacts for sub-second builds.
  - 100% standalone binary generation with zero external DLLs, `.so`, or `.dylib` requirements.

#### Example Usage
```alya
# 1. Standard libc functions (no extra library flag needed)
extern "C"
    function puts(s: str) -> i32
    function abs(n: i32) -> i32
    function strlen(s: str) -> i32
end

puts("Hello from native C FFI!")
say abs(-42)
say strlen("Alya Language")

# 2. External shared library (links -lsqlite3 automatically)
extern "C" from "sqlite3"
    function sqlite3_libversion() -> str
end

say sqlite3_libversion()
```

---

### Pillar 3: Language Server Protocol (Alya LSP) & IDE Toolchain ✅

Provide modern IDE capabilities and official editor extensions across VS Code and beyond.

#### Completed Capabilities
- **Dedicated LSP Subcommand (`alya lsp`)**:
  - Microsoft Language Server Protocol v3.17 server running over JSON-RPC (stdio).
  - Real-time diagnostic parser reporting syntax and type errors on `textDocument/didOpen` and `didChange`.
  - Context-aware autocompletion for keywords, identifiers, and language constructs.
  - Hover tooltips providing type signatures and documentation info.
  - Go to definition resolving symbol declarations across the file.
- **Official VS Code Extension (`vscode-alya` v0.3.0)**:
  - Official GitHub repository: [`alya-lang/vscode-alya`](https://github.com/alya-lang/vscode-alya).
  - Standalone `.vsix` releases with automated GitHub Actions CI/CD packaging and release pipelines.
  - **Official VS Code Test Explorer**: Full integration with VS Code's Testing sidebar via the `TestController API`.
  - **Native Assembly & AST Inspector**: Side-by-side assembly output inspection (`alya build -S`) and AST hierarchy inspection (`alya ast`).
  - **CodeLens Integration**: One-click `▶ Run` above `function main()` and `🧪 Run Test` above `test` blocks.
  - **In-Place Formatter**: Native Format on Save (`editor.formatOnSave`) and Format Document (`Shift+Alt+F`) via `alya fmt`.
  - **Brand File Icons & Theme**: Embedded vector Dark/Light SVG icons and dedicated `Alya File Icons` theme.
  - **Rich Snippets Suite**: Tab-triggered templates for all modern language idioms (`fn`, `struct`, `when`, `extern "C"`, `spawn`, `try`).
  - **Interactive Status Bar**: Real-time compiler version indicator, LSP process health status, and 1-click quick actions menu.

---

### Pillar 4: Memory Resilience & Cycle Detection (ARC Enhancements) 📋

> 📋 **Active Spec Roadmap:** Tracked under [`spec/ROADMAP.md`](spec/ROADMAP.md) **Phase 4.6 (Bacon-Rajan Cycle Collector)** and **Phase 4.7 (`--mem-trace` Heap Trace Engine)**.

Enhance Alya's Automatic Reference Counting (ARC) with advanced cyclic graph reclamation and memory profiling.

#### Goals & Architecture
- **The Cyclic Reference Problem**:
  - Currently, ARC immediately frees objects when reference counts reach zero with zero latency.
  - Self-referencing structures (e.g., node `A` references node `B`, and `B` references `A`) keep reference counts above zero, causing memory leaks upon disconnection.
- **Weak References (`weak_ref`)**:
  - Introduce weak reference primitives (`weak_ref(obj)`) that observe targets without incrementing strong reference counts.
  - Safe dereferencing (`weak_upgrade(w)`) returning `null` if the target was already freed.
- **Background / Scoped Cycle Collector**:
  - Non-blocking, trial-deletion cycle detection algorithm (inspired by Bacon-Rajan).
  - Triggered periodically or on-demand to identify and sweep isolated cyclic reference islands.
- **Memory Diagnostics & Heap Trace**:
  - `alyac run <file> --mem-trace`: Detailed heap allocation counter, live reference inspector, and leak detector output on program exit.

---

### Pillar 5: Concurrency Strategy — "Colorless Concurrency" over `async/await` 📋

> 📋 **Active Spec Roadmap:** Tracked under [`spec/ROADMAP.md`](spec/ROADMAP.md) **Phase 4.5 (Reactor Event Thread Pool)** and **Phase 4.8 (M:N Fiber Scheduler & Growable Stacks)**.

A fundamental architectural decision of Alya is the deliberate rejection of the `async/await` paradigm ("What Color is Your Function?").

#### Why Alya Rejects `async/await` (Architectural Non-Goals)
1. **The Function Coloring Plague**: Marking a function `async` forces all callers to become `async`, spreading viral syntax annotations throughout the codebase.
2. **Library Fragmentation**: Eliminates the split ecosystem seen in Rust and Python (`sync-sqlite` vs `async-sqlite`, `requests` vs `aiohttp`). In Alya, functions are universal, ergonomic, and colorless.
3. **Compiler Simplicity & Zero-Cost Runtimes**: Avoids heavyweight compiler state-machine lowering, heap allocations for futures (`Box<dyn Future>`), and complex runtime executor bloat.

#### The 3-Tier Concurrency Architecture

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                      ALYA 3-TIER CONCURRENCY MODEL                       │
├───────────────────────┬──────────────────────────┬───────────────────────┤
│ Tier 1: OS Threads    │ Tier 2: Reactor Pattern  │ Tier 3: Green Threads │
│ (std/thread + sync)   │ (Lib/event - epoll/IOCP) │ (spawn / Goroutines)  │
├───────────────────────┼──────────────────────────┼───────────────────────┤
│ • CPU-bound compute   │ • I/O-bound networking   │ • Universal fibers    │
│ • OS-level threads    │ • Single-thread loop     │ • M:N scheduler       │
│ • Mutex/Channel/Lock  │ • C100K event demux      │ • Colorless syntax    │
│ • Status: ✅ Stable   │ • Status: 📋 Roadmap     │ • Status: 💡 Research │
└───────────────────────┴──────────────────────────┴───────────────────────┘
```

1. **Layer 1: Native OS Threads & Synchronization (`std/thread`, `std/sync`)** ✅
   * Direct Win32 and POSIX `pthread` OS threads for CPU-heavy tasks.
   * Zero-allocation synchronization primitives: `Mutex`, `Channel`, `WaitGroup`, `Once`, `RwLock`.
2. **Layer 2: Reactor Event Loop (`Lib/event`)** 📋
   * Single-threaded non-blocking I/O multiplexer powered by kernel events (`epoll` on Linux, `WSAPoll`/`IOCP` on Windows, `kqueue` on macOS).
   * High-concurrency network servers (HTTP, WebSocket) scaling to 100,000+ active connections per core without OS thread overhead.
   * Tracked in [`Lib/EVENT_UV_ROADMAP.md`](../../Lib/EVENT_UV_ROADMAP.md).
3. **Layer 3: Colorless Green Threads / Fibers (`spawn` / Goroutines)** 💡
   * Long-term runtime evolution: Lightweight user-space green threads with segmented/growable stacks and an M:N cooperative scheduler.
   * Clean, natural, synchronous syntax:
     ```alya
     spawn handle_client(sock)
     ```
   * Sockets and channels automatically yield to the runtime scheduler upon waiting, preserving 100% synchronous, colorless code ergonomics with asynchronous speed.

---

## Future Explorations 💡

> 💡 **Active Spec Roadmap:** Tracked under [`spec/ROADMAP.md`](spec/ROADMAP.md) **Phase 6 (Future Explorations: WebAssembly, Type Annotations, Native GUI, SIMD Vectorization)**.

- **Optional Type Annotations (`let x: int`, `fn add(a: int, b: int): int`)**: Gradual typing for high-performance JIT/AOT code generation and compile-time contract enforcement.
- **WebAssembly Target (`wasm32-unknown-unknown`)**: Compile Alya code directly to WebAssembly for browser sandboxes and edge compute runtimes.
- **Native GUI Toolkit Integration**: Direct bindings to lightweight native windowing (e.g., Cocoa on macOS, Win32/DirectX on Windows, Wayland/X11 on Linux).
- **SIMD Vectorization**: Explicit vector primitives (`f64x4`, `i32x8`) mapping directly to AVX2/AVX-512 and ARM Neon instructions.

---

## Contributing to the Roadmap

Have ideas or want to champion a roadmap milestone?
- Review [CONTRIBUTING.md](CONTRIBUTING.md) to get started with the codebase.
- Open an issue or discussion on [GitHub](https://github.com/alya-lang/alya/issues) tagged with `[RFC]` or `[Roadmap]`.
