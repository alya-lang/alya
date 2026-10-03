# Chapter 24: Toolchain, CLI & Package Manager (`alya`)

## 1. Specification Rules

### 1.1 The Single-Binary Philosophy
Following Go's and Zig's unified toolchain model, Alya rejects fractured external ecosystems. The single standalone binary (`alya`) encapsulates the entire development lifecycle: compiler, package manager, formatter, test runner, REPL, and bundler.

### 1.2 Environment Variables & Global Paths
- **`ALYA_HOME`**: Global root directory (defaults to `~/.alya` on Unix or `%USERPROFILE%\.alya` on Windows).
- **`~/.alya/cache`**: Global immutable package repository cache storing clean checkouts.
- **`~/.alya/toolchain`**: Local portable C linker/compiler toolchain (e.g. MinGW GCC on Windows).

---

### 1.3 Core Build & Execution Subcommands

#### 1. `alya run [file] [-- args]`
Compiles the target file into a temporary executable and runs it immediately.
- If run inside a directory containing `alya.toml` without arguments, it automatically locates the package entry point (`src/main.alya`).

#### 2. `alya build [file] [flags]`
Compiles source files into a standalone native executable:
- **`--release`**: Enables optimizations (branch fusion, dead-code elimination, unboxing) and strips `@test` and `bench` blocks.
- **`--arch <arch>`**: Cross-compilation target architecture (`x64`, `arm64`).
- **`--os <os>`**: Cross-compilation target OS (`windows`, `linux`, `macos`).
- **`-o, --output <path>`**: Explicit output binary path.
- **`--bundle`**: macOS application bundle generation (generates `.app`, `Info.plist`, and multi-resolution `.icns`).
- **`--time`**: Displays phase-by-phase compilation latency breakdown.

#### 3. `alya check [file]`
Runs lexical analysis, parsing, and type inference without emitting assembly, providing sub-millisecond feedback on syntax or type errors.

---

### 1.4 Package Management (`alya pkg` / Built-in Package Commands)

#### 1. Lifecycle Commands
- **`alya init [path] [--name <name>] [--lib]`**: Initializes a new Alya package with starter template, `alya.toml`, and `.gitignore`.
- **`alya add <package> [options]`**: Adds a dependency to `alya.toml` and updates `alya.lock`:
  - `--path <path>`: Local path dependency.
  - `--git <url> [--tag <tag>] [--branch <branch>]`: Remote Git repository dependency.
- **`alya install`**: Resolves the complete dependency graph, downloads missing packages to `~/.alya/cache`, and writes a deterministic `alya.lock`.
- **`alya update [package]`**: Checks remote repositories for semver-compatible updates.
- **`alya pkg cache clean`**: Purges downloaded package caches to free disk space.

---

### 1.5 Package Manifest Specification (`alya.toml`)
The project configuration file uses standard TOML:

```toml
[package]
name = "my_service"
version = "0.1.0"
alya-version = ">=0.0.18"
entry = "src/main.alya"
authors = ["Alice <alice@alya.dev>"]
license = "MIT"
description = "High-performance microservice in Alya"

[dependencies]
# Standalone package from GitHub
http = { git = "https://github.com/alya-lang/http", tag = "v0.3.0" }

# Local workspace path dependency
utils = { path = "../shared/utils" }

# Semver registry package
crypto = "^0.2.1"

# Optional dependency with edge-controlled features
tls = { version = "^0.4.0", optional = true, default-features = false, features = ["rustls"] }
```

#### 1.5.1 Feature tables (`[features]`) and edge control
- `[features]` maps a feature name to a member list. Each member is one of:
  - `name` — enables the same-named feature or optional dependency `name` in the *same* package (whichever exists; both when both exist);
  - `dep:name` — enables optional dependency `name` explicitly, without touching any same-named feature;
  - `name/feat` — enables feature `feat` on dependency `name`, and implicitly enables `name` itself.
- No weak (`name?/feat`) syntax in v1: `name/feat` always enables `name`.
- Every member must name a known local feature, a declared dependency, or a `dep:…` / `…/…` target whose left side is a declared dependency; anything else is a manifest error. The feature-to-feature graph must be acyclic (self-edges like `feat = ["feat"]` name the same-name optional dependency idiom and are exempt from the cycle check).
- Dependency edges accept three keys: `optional = true|false` (default `false`), `default-features = true|false` (default `true`), and `features = ["f", …]` (features enabled on the dependency whenever this edge is active; each must be a valid feature name).
- CLI parity: `--features` / `--no-default-features` apply to the entry package only (workspace roots fan out per member: each member acts as its own entry; unknown names are errors per member). There is no `package:feature` selector in v1.

---

### 1.6 Cryptographic Lockfile Specification (`alya.lock`)
Ensures strictly reproducible and tamper-proof builds across all developer machines and CI pipelines:
- Generated exclusively by the compiler package manager.
- Contains flat, alphabetically sorted entries for all direct and transitive dependencies.
- **Pure-Rust SHA-256 Verification**: Every dependency entry stores cryptographic content checksums verified before build execution.

```toml
# THIS FILE IS AUTOMATICALLY GENERATED BY ALYA. DO NOT EDIT MANUALLY.
version = 1

[[package]]
name = "crypto"
version = "0.2.1"
source = "git+https://github.com/alya-lang/crypto#v0.2.1"
checksum = "a1b2c3d4e5f6...7890abcdef"

[[package]]
name = "rand"
version = "0.1.4"
source = "git+https://github.com/alya-lang/rand#v0.1.4"
checksum = "f0e1d2c3b4a5...1234567890"
```

---

### 1.7 Dependency Resolution & Diamond Dependency Specification

Alya solves the classic **"Diamond Dependency Problem"** (e.g. Package `A` depends on `B` and `C`, where `B` depends on `Z@1.0.0` and `C` depends on `Z@2.0.0`) through a hybrid **Major-Segregation & Semantic Coalescing** model.

```text
        ┌────────────────────────────────────────────────────────┐
        │                     Application (A)                    │
        └───────────────────────────┬────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
            ┌───────────────┐               ┌───────────────┐
            │   Package B   │               │   Package C   │
            └───────┬───────┘               └───────┬───────┘
                    │                               │
                    ▼ (requires ^1.0.0)             ▼ (requires ^2.0.0)
            ┌───────────────┐               ┌───────────────┐
            │   Z (v1.x.x)  │               │   Z (v2.x.x)  │
            │  (.alya/z-v1) │               │  (.alya/z-v2) │
            └───────────────┘               └───────────────┘
```

#### 1. Minor & Patch Coalescing (Zero Duplication)
When dependencies request different minor or patch versions within the same major version series (e.g. `B` requires `Z@^1.1.0` while `C` requires `Z@^1.4.0`):
- SemVer guarantees backward compatibility across minor and patch releases.
- The Alya package resolver selects the single highest mutually compatible version (`1.4.0`).
- Only a single copy is downloaded to `.alya/packages/z/` and compiled into the binary, eliminating bloat.

#### 2. Major Version Segregation (Multi-Major Coexistence)
When incompatible major versions are required transitively (e.g. `Z@1.x.x` vs `Z@2.x.x`):
- **Segregated Storage**: Packages are isolated on disk per major version:
  ```text
  .alya/packages/
  ├── b/
  ├── c/
  ├── z-v1/      # Contains Z 1.x.x for package B
  └── z-v2/      # Contains Z 2.x.x for package C
  ```
- **Context-Aware Import Resolution**: During module resolution, each package's `import "z"` resolves to the major version declared in its own `alya.toml` manifest.
- **Compiler Symbol Mangling**: The code generator appends the major version to native object symbols:
  - `Z@1.x.x` emits symbol: `_Alya_z_v1_<func_name>`
  - `Z@2.x.x` emits symbol: `_Alya_z_v2_<func_name>`
  This prevents linker duplicate symbol collisions and allows multiple major versions to safely coexist in the same native executable.
- **Nominal Type Incompatibility**: Types with the same name across different major versions (e.g. `z-v1::User` and `z-v2::User`) are treated as nominally distinct types by the type checker; assigning one to the other is a compile-time type error.

#### 3. Strict Direct Dependency Isolation
Alya enforces strict module boundaries:
- An application or library module can **only** import packages explicitly declared in its own `alya.toml`.
- Transitive dependencies (e.g. `Z` used by `B`) cannot be imported directly by `A` unless `Z` is explicitly declared in `A`'s `alya.toml`. This prevents implicit transitive dependency leakage and fragile builds.

#### 4. Native C-FFI Safety Constraint (`links`)
When a package wraps a native C library (such as SQLite, OpenSSL, or libuv), global C symbols cannot be mangled with major version tags:
- Such packages must declare a unique native link identifier in `alya.toml`:
  ```toml
  [package]
  name = "sqlite"
  version = "2.1.0"
  links = "sqlite3"
  ```
- **Safety Invariant**: The Alya resolver strictly prohibits having multiple major versions of packages that declare the same `links` key in a single dependency graph.
- If a conflict occurs, the toolchain halts at resolution time with a clear diagnostic:
  `Error: Duplicate native C library link 'sqlite3' required by both 'z-v1' and 'z-v2'. Align dependency versions to resolve.`

#### 5. Feature unification (`dep:feature` propagation)
- Features are additive and converge to a fixpoint: the resolver walks the dependency graph from the entry, and each node accumulates the union of every request aimed at it. Enabling the same feature twice is idempotent; cycles terminate because the active set only grows.
- A node's unified set is: its `default` feature (unless disabled — see below), closed transitively over its own `[features]` table, plus every `dep/feat` and edge-`features` request from any active parent, plus CLI `--features` on the entry node.
- Defaults rule: a dependency's `default` feature stays enabled unless *every* incoming active edge sets `default-features = false` (entry `--no-default-features` disables the entry default only). One edge keeping defaults is enough to keep them.
- Optional dependencies stay out unless switched on: by a local active feature naming them (`name` / `dep:name`), by any `name/feat` request, or by being non-optional. A `name/feat` request targeting an inactive optional dependency activates it.
- Strict targets at install time: every `dep/feat` and edge-`features` name must exist in the target manifest (as a feature or an optional dependency); `alya install` fails otherwise, naming the requesting package. The compiler stays lenient for unresolvable leaves (they fall back to defaults and fail later at import resolution with the precise missing-package diagnostic).
- Unification is per resolved node: major-segregated copies (`z-v1`, `z-v2`) unify independently; each evaluates its own `@cfg(feature = …)` against its own unified set. Unknown feature names in `@cfg` stay false and never break foreign packages.
- The lockfile records versions/sources only — unified features are a resolution-time view, not locked state. The incremental build-cache key must cover the unified per-node set (a feature flip rebuilds affected nodes).

---

### 1.8 Integrated Developer Tooling

#### 1. Code Formatter (`alya fmt`)
- `alya fmt [path]`: Formats `.alya` source files in-place according to standard language styling rules (4 spaces indentation, operator spacing, canonical keyword capitalization).
- `alya fmt --check [path]`: Returns exit code `1` if unformatted files are detected (for CI pipelines).
- Project excludes live in `.alyafmt` (or the `[fmt]` section of `alya.toml`), discovered walking upwards; `exclude` entries are additive to the built-in fixture skips, an explicitly named file is always honored, and `# fmt: off` / `# fmt: on` suppress formatting for a line range.

#### 2. Test Runner (`alya test`) & Benchmarks (`alya bench`)
- `alya test [path]`: Automatically discovers and executes all `@test` functions and `test "..." ... end` blocks across the project, by filename (`test_*`, `*_test`) AND by content (any file declaring suite entries). `alya bench [path]` mirrors this for `@bench` functions and `bench` blocks.
- Project excludes live in `.alyatest` (or the `[test]` / `[bench]` sections of `alya.toml`); `spec/negative`-style fixture directories are never entered.
- Emits pass/fail metrics, assertion failure diffs, and execution times.

#### 3. Interactive Shell (`alya repl`)
- `alya repl`: Starts an immediate Read-Eval-Print-Loop supporting multi-line blocks, instant expression evaluation, and live symbol inspection.

#### 4. Documentation Generator (`alya doc`)
- `alya doc [path] [-o <dir>]`: Extracts `##` Markdown docstrings from source files and generates structured Markdown documentation or static HTML reference websites.

#### 5. Static Code Linter (`alya lint`)
- `alya lint [path] [--fix]`: Performs static semantic analysis and code smell detection inspired by Rust's Clippy and Go's `golangci-lint`:
  - **Unused Symbols (`unused-var`, `unused-import`)**: Flags unread local variables, unused function parameters, and redundant imports.
  - **Dead & Unreachable Code (`dead-code`)**: Identifies unreachable statements following unconditional `return`, `throw`, or infinite loops.
  - **Idiomatic Style & Anti-Patterns (`idiomatic-style`)**: Recommends modern syntax upgrades (e.g. suggesting `when` pattern matching over long `if/elif` chains, destructuring tuples/structs).
  - **Automated Quick Fixes (`--fix`)**: Automatically refactors and cleans up safe warnings in-place.
  - **LSP & IDE Integration**: Directly streams warning/info diagnostics to `alya lsp` over JSON-RPC, powering real-time squiggly underlinings and Quick Fix Code Actions in the VS Code extension (`vscode-alya`).
  - **CI Quality Gate**: `alya lint --check` exits with non-zero status if warnings or errors are detected.
