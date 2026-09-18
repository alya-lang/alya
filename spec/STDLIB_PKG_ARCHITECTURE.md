# Alya Standard Library (Stdlib) vs Package (Pkg) Architecture & Deduplication Governance

This document establishes the official architectural guidelines, boundary rules, deduplication action plan, and future design policies for distinguishing between **Embedded Standard Library Modules (`std/*`)** and **Official Standalone Packages (`Lib/*` / `alya-lang/*`)**.

---

## 1. Context & Motivation

As the Alya programming language and its package ecosystem expand, clear boundaries are critical to:
1. **Preserve Compiler Binary Lightness**: The compiler (`alya`) embeds standard library modules directly via `include_str!`. Uncontrolled growth in `std/*` directly bloats the compiler binary.
2. **Prevent Duplication (DRY Principle)**: Multiple modules independently reimplementing the same utility (e.g. Hex encoding, Base64, UUID v4) causes maintenance drift, inconsistent test vectors, and developer confusion.
3. **Decouple Release Cycles**: Core language features and syscalls must remain ultra-stable, while domain-specific algorithms (cryptography, parsers, compression codecs) must iterate rapidly with independent semantic versioning (`v0.1.0`, `v0.5.0`).

---

## 2. The 3-Tier Classification Model

Alya adheres strictly to the 3-Tier Classification Model:

```text
┌────────────────────────────────────────────────────────────────────────┐
│                          ALL ALYA MODULES                              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼────────────────────────────┐
        ▼                           ▼                            ▼
 [TIER 1: CORE STDLIB]     [TIER 2: HYBRID CORE]      [TIER 3: STANDALONE PKG]
 Embedded in compiler;      Minimal core in stdlib;    Decoupled domain packages
 zero external deps.        rich API in package.       managed via alya.toml.
 ─────────────────────────  ─────────────────────────  ─────────────────────────
 • os, fs, path, time       • rand (core LCG PRNG)     • crypto (SHA, HMAC, AES)
 • math, str, mem           • cli  (raw args/flags)    • compress (Brotli, Zstd)
 • collections, sync        • net  (raw TCP/UDP)       • uuid   (v4, v7, ULID)
 • test, console            • log  (console ANSI)      • csv, url, http, jwt...
 • io, process              • json (basic parse/str)   • toml, sqlite, yaml...
```

### Tier 1: Core Standard Library (`std/*`)
- **Location**: `Src/alya/stdlib/` (14 canonical modules per Chapter 23).
- **Criteria**: Fundamental OS syscall abstractions and core data structure intrinsics.
- **Rule**: Never externalized. Zero external dependencies. Highly conservative API stability.

### Tier 2: Hybrid Modules
- **Criteria**: Modules where basic scripting needs a trivial built-in tool, but production applications require deep, specialized functionality.
- **Rule**: The `std/*` version is strictly pruned to **< 100 lines** and wraps only raw OS/libc primitives. The full-featured counterpart lives in `Lib/*` (`alya-lang/*`).
  - `std/math` (random LCG PRNG) ⟷ `Lib/rand` (SplitMix64, PCG32, distributions, sampling).
  - `std/os` (raw args accessors) ⟷ `Lib/cli` (flags, subcommands, auto-help).
  - `std/net` (~200 lines raw sockets) ⟷ `Lib/http` (HTTP 1.1, routing, middleware).
  - `std/console` (ANSI colors) ⟷ `Lib/logger` (JSON, file rotation).
  - `std/json` (~120 lines basic parser) ⟷ `Lib/json` (AST DOM, schema, pretty-print).

### Tier 3: Standalone Domain Packages (`Lib/*` / `alya-lang/*`)
- **Criteria**: Domain-heavy libraries (cryptography, compression, databases, file formats).
- **Rule**: Completely unbundled from compiler binary. Installed via `alya add <pkg>` and tracked in `alya.lock`.

---

## 3. Current Inconsistencies & Duplication Audit

A systematic code audit identified the following 4 areas of overlap across `std/hash`, `std/rand`, `Lib/rand`, `Lib/compress`, and `Lib/crypto`:

### Issue A: Hex and Base64 Encoding Duplication
- **Resolution**:
  - `std/hash` contains non-cryptographic hash table hashing algorithms exclusively (`djb2`, `sdbm`, `fnv1`, `fnv1a`, `fnv1a64`, `murmur3`, `crc32`, `adler32`, `hash_combine`).
  - Encoding functions (`base64_encode`, `base64_decode`, `hex_encode`, `hex_decode`, `is_hex`, `is_base64`) are cleanly located in `std/str`.
  - Byte-oriented cryptographic encodings live in `Lib/crypto`.

### Issue B: Multi-Package UUID v4 Duplication
- **Resolution**:
  - `Lib/uuid`: Canonical RFC 4122 UUID v4, RFC 9562 UUID v7, ULID, and NanoID generator.
  - Removed duplicate `random_uuid` from `Lib/crypto` and duplicate UUID generators from `Lib/rand`.

### Issue C: Checksum Duplication (CRC-32 & Adler-32)
- **Resolution**:
  - `std/hash`: Fast pure-Alya polynomial string checksums (`crc32(s)`, `adler32(s)`).
  - `Lib/compress`: High-throughput C-accelerated (`miniz` FFI) byte-stream verification for RFC 1950/1952 containers (`crc32(bytes)`, `adler32(bytes)`).

### Issue D: Randomness & Entropy Delegation
- **Resolution**:
  - `std/math`: Lightweight global LCG PRNG (`random()`, `rand_int()`).
  - `Lib/rand`: Multi-engine PRNG (SplitMix, PCG, Xorshift), statistical distributions, array sampling, and raw byte generation (`bytes(count)`).
  - `Lib/crypto`: Cryptographic entropy helpers (`random_bytes(count)`).

---

## 4. Ongoing Governance & Anti-Duplication Rules

To prevent future code duplication between `std/*` and `Lib/*`, all new contributions and refactors must enforce the following six golden rules:

### Rule 1: The Canonical Home Principle (Single Source of Truth)
Every algorithm or capability must have exactly **ONE canonical home**:
- If it is an OS syscall or core runtime data structure ➔ **`std/*` (Tier 1)**.
- If it is a domain protocol, format, or cryptographic cipher ➔ **Dedicated `Lib/*` package (Tier 3)**.
- A package must never reimplement an algorithm that already has an official canonical package (e.g. Do not implement UUID in `crypto` when `uuid` exists; do not implement JSON in `http` when `json` exists).

### Rule 2: Strict Acyclic Dependency Graph (DAG)
Packages must adhere to a strict top-down dependency hierarchy:
```text
           [rand]
          /      \
    [crypto]     [uuid]
       |
     [jwt]
```
- Standard library modules (`std/*`) can **NEVER** depend on standalone packages (`Lib/*`).
- Standalone packages can depend on `std/*` and on upstream Tier 3 packages declared in `alya.toml`.
- Circular dependencies between packages are strictly forbidden.

### Rule 3: The 100-Line Pruning Threshold for Tier 2
Any hybrid module admitted into `std/*` (e.g. `std/net`, `std/json`):
- Must not exceed **100-150 lines of code**.
- Must contain zero external dependencies.
- Must provide only the absolute minimum interface needed for quick scripts.
- Advanced features (clustering, schemas, distributions, rotation) **must be rejected from stdlib** and directed to the official standalone package.

### Rule 4: Separation of Hashing and Encoding
- **Hashing** is a one-way mathematical reduction ($M \rightarrow H$).
- **Encoding** is a reversible, bijective data representation ($D \leftrightarrow E$).
- These two concerns must never be combined into the same module. Hash modules must only calculate digest integers or raw digests; formatting to hex/base64 belongs in string or encoding utilities.

### Rule 5: Zero Backward-Compatibility Burden (Clean-Break Policy)
- Alya is an evolving, pre-v1.0 modern language. **No backward-compatibility shims, transitional aliases, `compat.alya` layers, or deprecation warnings are maintained.**
- When an API or module is refactored, split, or moved:
  - The old code or alias is deleted immediately and completely.
  - No legacy wrappers (`compat.alya`, `*_legacy`, `*_compat`) are kept in packages.
  - Missing modules simply fail with standard `Cannot find module` errors; no redirection or transition messages.
  - Eliminating backward-compatibility bloat keeps the compiler fast, codebases lean, and documentation unambiguous.

### Rule 6: Code Review & Linter Check for New Packages
Before any new package is admitted to `Lib/` or `alya-lang/`:
- Cross-reference existing `std/*` modules and official packages.
- If duplicate functions exist, either:
  1. Add the upstream package as a dependency in `alya.toml`, or
  2. Delete the duplicate and use the canonical package directly.

---

## 5. Summary Matrix

| Capability | Stdlib (`std/*`) | Package (`Lib/*`) | Policy & Action |
| :--- | :--- | :--- | :--- |
| **Non-crypto Hash** (Murmur, FNV, DJB2) | `std/hash` ✅ | None | Canonical home in `std/hash`. |
| **Crypto Hash & Ciphers** (SHA, AES, ChaCha) | None | `Lib/crypto` ✅ | Canonical home in `Lib/crypto`. Never put in `std/hash`. |
| **Basic PRNG** (LCG range, choice) | `std/math` ✅ | `Lib/rand` ✅ | Tier 2 Hybrid. `std/math` pruned; `Lib/rand` full-featured. |
| **UUID (v4, v7, ULID)** | None | `Lib/uuid` ✅ | Canonical home in `Lib/uuid`. |
| **Hex & Base64** | `std/str` ✅ | `Lib/crypto` ✅ | String helpers in `std/str`, byte arrays in `Lib/crypto`. |
| **Checksums (CRC-32, Adler-32)** | `std/hash` (string) | `Lib/compress` (bytes, C FFI) | Dual-tier justified by FFI decompression speed requirement. |
| **Compression** (Brotli, Zstd, LZ, Huffman) | None | `Lib/compress` ✅ | Canonical home in `Lib/compress`. |
