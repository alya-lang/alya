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

**Alya** is designed to provide clean, readable syntax inspired by natural language without compromising runtime execution speed. Written in Rust, the `alyac` compiler generates native GNU and Mach-O assembly directly—bypassing heavy intermediate representation (IR) or LLVM overhead—and links with system toolchains to produce standalone, blazing-fast native binaries.

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
- 🛠️ **Built-in Developer Tooling**: In-place code formatter (`alyac fmt`) and test runner (`alyac test`) built directly into the compiler binary—no external dependencies needed.
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
alyac run examples/hello.alya

# Run with microsecond execution and compiler stage profiling
alyac run examples/hello.alya --time

# Compile directly to a standalone binary
alyac build examples/calculator.alya -o calculator

# Format source files across project in-place (or --check in CI)
alyac fmt .

# Discover and run test suites across the project
alyac test

# Check syntax only without code generation
alyac check examples/calculator.alya
```

---

## Performance

Alya is engineered for rapid compilation and high-performance native execution across all operating systems and architectures.

### Cross-Language Execution Benchmark (Median of 5 runs)

| Category | Benchmark | C (GCC -O2) | Alya (Native) | Bun (JS JIT) | Python 3.12 | Alya vs Bun | Alya vs Python |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| `Algorithms` | **Recursive Fibonacci (n=30)** | `14.3 ms` | **`18.1 ms`** | `30.8 ms` | `131.1 ms` | **1.7x faster** | **7.3x faster** |
| `Algorithms` | **Quicksort (50k items)** | `23.6 ms` | **`117.2 ms`** | `47.0 ms` | `1604.6 ms` | `2.5x slower` | **13.7x faster** |
| `Algorithms` | **Sieve of Eratosthenes (50k)** | `11.9 ms` | **`15.4 ms`** | `29.8 ms` | `46.2 ms` | **1.9x faster** | **3.0x faster** |
| `Algorithms` | **Collatz (100k limit)** | `24.7 ms` | **`84.7 ms`** | `60.6 ms` | `796.5 ms` | `1.4x slower` | **9.4x faster** |
| `Algorithms` | **Binary Search (100k items)** | `14.6 ms` | **`20.9 ms`** | `31.2 ms` | `121.1 ms` | **1.5x faster** | **5.8x faster** |
| `Collections` | **Binary Trees (Depth 14)** | `246.9 ms` | **`482.2 ms`** | `142.6 ms` | `2371.7 ms` | `3.4x slower` | **4.9x faster** |
| `Collections` | **Hash Map (20k entries)** | `14.8 ms` | **`27.3 ms`** | `34.7 ms` | `45.0 ms` | **1.3x faster** | **1.6x faster** |
| `Numeric` | **Mandelbrot Fractal (200×100)** | `14.3 ms` | **`18.4 ms`** | `31.3 ms` | `114.7 ms` | **1.7x faster** | **6.2x faster** |
| `Numeric` | **Matrix Multiply (120×120)** | `11.6 ms` | **`19.0 ms`** | `31.8 ms` | `163.7 ms` | **1.7x faster** | **8.6x faster** |
| `Numeric` | **Monte Carlo (500k iters)** | `60.2 ms` | **`21.6 ms`** | `38.0 ms` | `181.4 ms` | **1.8x faster** | **8.4x faster** |
| `Strings` | **FNV-1a String Hash (50k)** | `15.1 ms` | **`20.4 ms`** | `41.8 ms` | `386.3 ms` | **2.0x faster** | **18.9x faster** |

> 📊 For full cross-platform benchmark results (Linux, macOS, Windows), compiler throughput benchmarks, and reproduction instructions, see **[alya-lang/benchmarks](https://github.com/alya-lang/benchmarks)**.

---

## Platform Support

| Operating System | x86 (32-bit) | x64 (64-bit) | ARM64 (AArch64) |
| :--------------- | :----------: | :----------: | :-------------: |
| **Linux**        | ✅ Supported | ✅ Fully Supported (ELF64) | ✅ Supported |
| **macOS**        | ❌ Deprecated by Apple | ✅ Fully Supported (Mach-O) | ✅ Fully Supported (Apple Silicon) |
| **Windows**      | ✅ Supported | ✅ Fully Supported (MinGW-w64) | ⚠️ Cross-compiler required |

---

## Documentation

- 📚 **[Alya Documentation Wiki](docs/README.md)**: Structured 8-chapter guide progressing from beginner concepts to advanced compiler architectures.
- 📖 **[Single-Page Language Guide](docs/language-guide.md)**: Quick full-language reference and syntax cheat-sheet.
- 📱 **[Applications Showcase](apps/README.md)**: Real-world apps (HTTP server, benchmark tool, port scanner, Conway's Game of Life, Snake, TicTacToe) and macOS `.app` bundling guide.
- 🗺️ **[Project Roadmap](ROADMAP.md)**: Architectural vision, completed milestones, and the 4 future pillars (Package Manager, C FFI, LSP, Cycle Collector).
- 🧪 **[Code Examples](examples/)**: 50+ practical programs, algorithms, interactive terminal apps, and self-hosting compiler prototypes.
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
