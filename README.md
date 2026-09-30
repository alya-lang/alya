<div align="center">

# Alya

**A simple, fast, intuitive, and modern multi-platform compiled programming language.**

[![CI](https://github.com/alya-lang/alya/actions/workflows/ci.yml/badge.svg)](https://github.com/alya-lang/alya/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/alya-lang/alya?color=blue&label=License)](LICENSE)
[![Rust](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Falya-lang%2Falya%2Fmain%2FCargo.toml&query=%24.package.rust-version&label=Rust&color=orange&prefix=%3E%3D)](https://www.rust-lang.org/)
[![Compiler Version](https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Falya-lang%2Falya%2Fmain%2FCargo.toml&query=%24.package.version&label=Version&color=brightgreen)](Cargo.toml)
[![Target Architectures](https://img.shields.io/badge/Arch-x86%20%7C%20x64%20%7C%20ARM64-blueviolet)](#platform-support)

<p align="center">
  <a href="#syntax-at-a-glance">Syntax</a> •
  <a href="#key-highlights">Highlights</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#performance">Performance</a> •
  <a href="#platform-support">Platforms</a> •
  <a href="#documentation">Documentation</a>
</p>

</div>

---

## Overview

**Alya** is designed to provide clean, readable syntax inspired by natural language without compromising runtime execution speed. Written in Rust, the `alya` compiler generates native GNU and Mach-O assembly directly—bypassing heavy intermediate representation (IR) or LLVM overhead—and links with system toolchains to produce standalone, blazing-fast native binaries.

---

## Syntax at a Glance

```alya
# Define custom data structures
struct Player
    name
    score
end

# First-class functions with expressive conditionals
function rank_player(p)
    if p.score >= 90
        return "Master"
    elif p.score >= 75
        return "Expert"
    else
        return "Challenger"
    end
end

# Collections, iteration, and string interpolation
let team = [
    Player { name: "Alice", score: 95 },
    Player { name: "Bob", score: 82 }
]

for member in team
    let tier = rank_player(member)
    say "Player {member.name} scored {member.score} pts -> [{tier}]"
end
```

---

## Key Highlights

- ⚡ **Direct Native Codegen**: Emits clean assembly for **ARM64** (Apple Silicon & AArch64), **x64**, and **x86 (32-bit)** with branch fusion, immediate range splitting (`movz`/`movk`), and zero-cycle idioms.
- 🚀 **Near-C Execution Speed**: Runs within 1.0x–2.0x of C (GCC `-O2`) and outperforms JavaScript JIT engines (Bun / V8) without VM warmup delays.
- 🛠️ **Built-in Developer Tooling**: In-place code formatter (`alya fmt`) and test runner (`alya test`) built directly into the compiler binary—no external dependencies needed.
- 📚 **Batteries-Included Standard Library**: Built-in modules for `std/net` (TCP/UDP sockets), `std/console` (terminal control), `std/glob`, `std/rand` (SplitMix64, UUID v4/v7, ULID), `std/color`, `std/log`, `std/str`, `std/math`, `std/fs`, `std/path`, `std/json`, `std/hash`, `std/collections`, `std/test`, and `std/mem` (Arena allocator).
- 🛡️ **Safety Without Runtime Penalties**: Single-instruction unsigned bounds checks (`jae` / `b.hs`), division/modulo zero protection, null safety (`null`, `??`), and structured `try ... catch`.
- 🗺️ **First-Class Types & Static Inference**: Dynamic arrays (`[1, 2]`), hash maps (`map()`), 64-bit IEEE 754 floats (`f64`), composite structs (`struct Point ... end`), and compile-time multi-pass struct type inference.
- 🎯 **Lightweight Single-Pass Compiler**: Sub-millisecond parser throughput parsing ~2 million lines per second with rich diagnostics and execution profiling (`--time`).

---

## Quick Start

### 1. Installation

Download pre-built standalone binaries for Linux, macOS, and Windows from [GitHub Releases](https://github.com/alya-lang/alya/releases), or build from source with [Rust](https://rustup.rs/):

```bash
# Clone and build with Cargo
git clone https://github.com/alya-lang/alya.git
cd Alya
cargo install --path .
```

### 2. Run & Build Programs

```bash
# Compile and run immediately in one step
alya run spec/syntax/toolchain_example.alya

# Run with microsecond execution and compiler stage profiling
alya run spec/syntax/toolchain_example.alya --time

# Compile directly to a standalone binary
alya build spec/syntax/when.alya -o calculator

# Format source files across project in-place (or --check in CI)
alya fmt .

# Discover and run test suites across the project
alya test

# Check syntax only without code generation
alya check spec/syntax/when.alya
```

---

## Performance

Alya is engineered for rapid compilation and high-performance native execution across all operating systems and architectures.

### Cross-Language Execution Benchmark (Median of 10 runs)

| Category | Benchmark | C (GCC -O2) | Alya (Native) | Bun (JS JIT) | Python 3.12 | Alya vs Bun | Alya vs Python |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `Algorithms` | **Recursive Fibonacci (n=30)** | `2.3 ms` | **`8.7 ms`** | `12.1 ms` | `122.0 ms` | **1.4x faster** | **14.0x faster** |
| `Algorithms` | **Quicksort (50k items)** | `3.9 ms` | **`11.3 ms`** | `11.7 ms` | `96.8 ms` | **1.0x faster** | **8.6x faster** |
| `Algorithms` | **Sieve of Eratosthenes (50k)** | `1.0 ms` | **`1.7 ms`** | `6.2 ms` | `17.1 ms` | **3.8x faster** | **10.3x faster** |
| `Algorithms` | **Collatz (100k limit)** | `14.5 ms` | **`58.6 ms`** | `41.2 ms` | `738.3 ms` | `1.4x slower` | **12.6x faster** |
| `Algorithms` | **Binary Search (100k items)** | `3.6 ms` | **`7.5 ms`** | `13.7 ms` | `115.3 ms` | **1.8x faster** | **15.3x faster** |
| `Collections` | **Binary Trees (Depth 14)** | `126.5 ms` | **`406.4 ms`** | `96.3 ms` | `2715.4 ms` | `4.2x slower` | **6.7x faster** |
| `Collections` | **Hash Map (20k entries)** | `4.3 ms` | **`11.2 ms`** | `14.5 ms` | `23.5 ms` | **1.3x faster** | **2.1x faster** |
| `Numeric` | **Mandelbrot Fractal (200×100)** | `3.2 ms` | **`12.8 ms`** | `9.5 ms` | `121.5 ms` | `1.3x slower` | **9.5x faster** |
| `Numeric` | **Matrix Multiply (120×120)** | `1.3 ms` | **`8.4 ms`** | `12.1 ms` | `189.9 ms` | **1.4x faster** | **22.5x faster** |
| `Numeric` | **Monte Carlo (500k iters)** | `3.8 ms` | **`6.1 ms`** | `9.8 ms` | `171.5 ms` | **1.6x faster** | **28.1x faster** |
| `Strings` | **FNV-1a String Hash (50k)** | `4.7 ms` | **`8.9 ms`** | `12.0 ms` | `433.0 ms` | **1.4x faster** | **48.8x faster** |
| `Memory` | **Linked List (50k nodes)** | `3.0 ms` | **`2.9 ms`** | `6.7 ms` | `28.9 ms` | **2.3x faster** | **10.0x faster** |
| `Crypto` | **RC4 Cipher (100k bytes)** | `1.1 ms` | **`2.4 ms`** | `6.5 ms` | `34.0 ms` | **2.7x faster** | **14.3x faster** |
| `Bitwise` | **Popcount (100k ints)** | `2.3 ms` | **`3.8 ms`** | `8.5 ms` | `138.6 ms` | **2.2x faster** | **36.8x faster** |

> 📊 For full cross-platform benchmark results (Linux, macOS, Windows), compiler throughput benchmarks, and reproduction instructions, see **[alya-lang/benchmarks](https://github.com/alya-lang/benchmarks)**.

---

## Platform Support

| Operating System | x86 (32-bit) | x64 (64-bit) | ARM64 (AArch64) |
| :--------------- | :----------: | :----------: | :-------------: |
| **Linux**        | ✅ Supported | ✅ Fully Supported (ELF64) | ✅ Fully Supported (ELF64, native CI + release) |
| **macOS**        | ❌ Deprecated by Apple | ✅ Fully Supported (Mach-O, Intel + Apple Silicon) | ✅ Fully Supported (Apple Silicon) |
| **Windows**      | ✅ Supported | ✅ Fully Supported (MinGW-w64) | ✅ Fully Supported (native ARM64 toolchain + release) |

---

## Documentation

- 📚 **[Alya Documentation Wiki](docs/README.md)**: Structured 8-chapter guide progressing from beginner concepts to advanced compiler architectures.
- 📖 **[Single-Page Language Guide](docs/language-guide.md)**: Quick full-language reference and syntax cheat-sheet.
- 📱 **[Applications Showcase](apps/README.md)**: Real-world apps (HTTP server, benchmark tool, port scanner, Conway's Game of Life, Snake, TicTacToe) and macOS `.app` bundling guide.
- 📐 **[Language Specification](spec/)**: 25 canonical grammar and syntax test suites (Chapters 00–24) defining all language behaviors.
- ⚡ **[Benchmark Suite](https://github.com/alya-lang/benchmarks)**: Cross-language performance benchmark suite and automated runner.

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) and adhere to our [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

---

## License

This project is licensed under the [MIT License](LICENSE).
