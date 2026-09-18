# Contributing to Alya

Thank you for your interest in contributing to the **Alya Programming Language**! 🚀

Alya is designed to be an intuitive, lightweight, and modern multi-platform compiled programming language. Whether you want to fix a bug, add a language feature, improve compiler performance, or enhance documentation, your contributions are warmly welcome.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Development Setup](#development-setup)
- [Project Architecture](#project-architecture)
- [Development Workflow](#development-workflow)
  - [Building](#building)
  - [Running the Compiler](#running-the-compiler)
  - [Testing](#testing)
  - [Linting and Formatting](#linting-and-formatting)
- [Adding New Language Features](#adding-new-language-features)
- [Commit Message Guidelines](#commit-message-guidelines)
- [Submitting a Pull Request](#submitting-a-pull-request)

---

## Code of Conduct

Please review and adhere to our [Code of Conduct](CODE_OF_CONDUCT.md) in all project spaces, issues, pull requests, and discussions.

---

## Development Setup

### Prerequisites

1. **Rust**: Rust stable (1.75+ recommended). Install via [rustup](https://rustup.rs/):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. **C Toolchain (GCC / MinGW)**: Required for assembling generated assembly code:
   - **Linux**: `sudo apt install gcc build-essential`
   - **Windows**: Install MinGW-w64 (via MSYS2 or WinLibs) and ensure `gcc` is in your `PATH`.
   - **macOS**: Install Xcode Command Line Tools: `xcode-select --install`.

### Clone the Repository

```bash
git clone https://github.com/alya-lang/alya.git
cd Alya
```

---

## Project Architecture

The compiler codebase in `src/` is organized into modular subsystems:

- [`src/lexer/`](src/lexer/): Lexical analysis. Converts raw `.alya` source code into tokens (`Token`, `TokenKind`).
  - `cursor.rs`: Source code navigation and position tracking.
  - `token.rs`: Token definitions and operator classifications.
  - `reader.rs`: Number, string, identifier, and symbol readers.
- [`src/parser/`](src/parser/): Syntactic analysis. Parses tokens into an Abstract Syntax Tree (`Program`, `Stmt`, `Expr`).
  - `ast.rs`: AST node definitions.
  - `expr.rs`: Expression parsing with operator precedence.
  - `stmt.rs`: Statement parsing (assignments, loops, conditionals, functions, `try...catch`).
- [`src/codegen/`](src/codegen/): Target assembly generation (X64, X86, ARM64).
  - `runtime/`: System and target-specific runtime routines (`say`, math intrinsics, exit handlers).
  - `expr.rs`: Code generation for arithmetic, logic, and function calls.
  - `stmt.rs`: Code generation for control flow and exception handling.
  - `say.rs`: Printing and string formatting assembly generators.
- [`src/diagnostics/`](src/diagnostics/): Friendly and pretty-printed error messages with source code snippets.
- [`src/cli/`](src/cli/): Command-line argument parsing and flag handling (`alyac run`, `build`, `check`, etc.).
- [`src/driver/`](src/driver/): Compilation orchestrator and GCC linker invocation.

---

## Development Workflow

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Running the Compiler

```bash
# Run source code directly
cargo run -- run spec/syntax/toolchain_example.alya

# Build an executable
cargo run -- build spec/syntax/when.alya -o calc

# Inspect AST or Tokens
cargo run -- ast spec/syntax/loops.alya
cargo run -- tokens spec/syntax/variables.alya
```

### Testing

Always ensure that all unit tests and integration tests pass:

```bash
# Run all tests
cargo test

# Run end-to-end compiler execution tests specifically
cargo test --test e2e_execution

# Verify canonical language specification tests and execution
cargo test --test golden_spec_tests
```

### Linting and Formatting

We enforce strict formatting and clippy lints in CI:

```bash
# Format code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Run clippy with warnings treated as errors
cargo clippy --all-targets --all-features -- -D warnings
```

---

## Adding New Language Features

When adding a new syntax or feature:

1. **Tokens**: Add any new keywords or symbols to `src/lexer/token.rs` and update the lexer reader in `src/lexer/reader.rs`.
2. **AST & Parser**: Extend `Stmt` or `Expr` in `src/parser/ast.rs`, and parse them in `src/parser/stmt.rs` or `src/parser/expr.rs`.
3. **Code Generator**: Emit assembly for each target architecture in `src/codegen/`.
4. **Unit Tests**: Add unit tests in `src/lexer/tests.rs` and `src/parser/tests.rs`.
5. **E2E Tests**: Add an executable test case in `tests/e2e_execution.rs`.
6. **Specification**: Add or update canonical test fixtures in `spec/syntax/` and documentation in `spec/chapters/`.
7. **Documentation**: Document the feature in `README.md`.

---

## Commit Message Guidelines

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` A new language or compiler feature
- `fix:` A bug fix
- `docs:` Documentation only changes
- `refactor:` A code change that neither fixes a bug nor adds a feature
- `perf:` A code change that improves performance
- `test:` Adding missing tests or correcting existing tests
- `ci:` Changes to CI configuration files and scripts
- `chore:` Changes to build process or auxiliary tools

Example:
```text
feat(codegen): add 64-bit modulo-by-zero runtime check
fix(lexer): correct escape sequence handling in multiline strings
```

---

## Submitting a Pull Request

1. Fork the repository and create your feature branch:
   ```bash
   git checkout -b feat/my-new-feature
   ```
2. Commit your changes following the commit message guidelines.
3. Verify that `cargo fmt`, `cargo clippy`, and `cargo test` pass with zero warnings or errors.
4. Push to your branch and open a Pull Request against the `develop` branch.
5. Fill out the PR template completely.

Thank you for helping make Alya better!
