# Alya Standard Library (Stdlib) vs Package (Pkg) Architecture & Deduplication Governance

This document establishes the official architectural guidelines, boundary rules, deduplication action plan, and future design policies for distinguishing between **Embedded Standard Library Modules (`std/*`)** and **Official Standalone Packages (`Lib/*` / `alya-lang/*`)**.

---

## 1. Context & Motivation

As the Alya programming language and its package ecosystem expand, clear boundaries are critical to:
1. **Preserve Compiler Binary Lightness**: The compiler (`alyac`) embeds standard library modules directly via `include_str!`. Uncontrolled growth in `std/*` directly bloats the compiler binary.
2. **Prevent Duplication (DRY Principle)**: Multiple modules independently reimplementing the same utility (e.g. Hex encoding, Base64, UUID v4) causes maintenance drift, inconsistent test vectors, and developer confusion.
3. **Decouple Release Cycles**: Core language features and syscalls must remain ultra-stable, while domain-specific algorithms (cryptography, parsers, compression codecs) must iterate rapidly with independent semantic versioning (`v0.1.0`, `v0.5.0`).

---

## 2. The 3-Tier Classification Model

Alya adheres strictly to the 3-Tier Classification Model defined in [`ROADMAP.md`](ROADMAP.md):

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
 • collections, thread      • net  (raw TCP/UDP)       • uuid   (v4, v7, ULID)
 • test, bench, console     • log  (console ANSI)      • csv, url, http, jwt...
 • hash (non-crypto lookup) • json (basic parse/str)   • toml, sqlite, yaml...
```

### Tier 1: Core Standard Library (`std/*`)
- **Location**: `Src/alya/stdlib/`
- **Criteria**: Fundamental OS syscall abstractions and core data structure intrinsics.
- **Rule**: Never externalized. Zero external dependencies. Highly conservative API stability.

### Tier 2: Hybrid Modules
- **Criteria**: Modules where basic scripting needs a trivial built-in tool, but production applications require deep, specialized functionality.
- **Rule**: The `std/*` version is strictly pruned to **< 100 lines** and wraps only raw OS/libc primitives. The full-featured counterpart lives in `Lib/*` (`alya-lang/*`).
  - `std/rand` (~85 lines LCG PRNG) ⟷ `Lib/rand` (SplitMix64, PCG32, distributions, sampling).
  - `std/cli` (~100 lines raw args) ⟷ `Lib/cli` (flags, subcommands, auto-help).
  - `std/net` (~200 lines raw sockets) ⟷ `Lib/http` (HTTP 1.1, routing, middleware).
  - `std/log` (~80 lines ANSI console) ⟷ `Lib/logger` (JSON, file rotation).
  - `std/json` (~120 lines basic parser) ⟷ `Lib/json` (AST DOM, schema, pretty-print).

### Tier 3: Standalone Domain Packages (`Lib/*` / `alya-lang/*`)
- **Criteria**: Domain-heavy libraries (cryptography, compression, databases, file formats).
- **Rule**: Completely unbundled from compiler binary. Installed via `alyac add <pkg>` and tracked in `alya.lock`.

---

## 3. Current Inconsistencies & Duplication Audit

A systematic code audit identified the following 4 areas of overlap across `std/hash`, `std/rand`, `Lib/rand`, `Lib/compress`, and `Lib/crypto`:

### Issue A: Hex and Base64 Encoding Duplication
- **Current State**:
  - `Src/alya/stdlib/hash.alya` defines: `hex_encode`, `hex_decode`, `to_hex`, `from_hex`, `is_hex`, `base64_encode`, `base64_decode`, `to_base64`, `from_base64`, `is_base64`.
  - `Lib/crypto/src/encodings/hex.alya` and `base64.alya` define identical functionality, extended with byte array (`[u8]`) variants (`bytes_to_hex`, `hex_to_bytes`, `base64_encode_bytes`).
  - `Lib/compress/src/types.alya` defines its own `bytes_to_hex` and `bytes_from_hex`.
- **Root Cause**: `std/hash` was used as a dumping ground for string encoding helpers because no dedicated `std/encoding` existed in Tier 1.
- **Problem**: Hashing is irreversible digesting; encoding is reversible representation. Bundling encoding inside `hash` violates single responsibility and forces packages to re-implement byte-level encodings.

### Issue B: Multi-Package UUID v4 Duplication
- **Current State**:
  - `Lib/uuid/src/core/v4.alya`: Canonical RFC 4122 UUID v4 generator.
  - `Lib/rand/src/core/identifiers.alya`: Re-implements `uuid_v4()` using internal RNG.
  - `Lib/crypto/src/random/entropy.alya`: Re-implements `random_uuid()` by generating 16 bytes and applying UUID v4 version/variant bitmasks.
- **Root Cause**: "Convenience creep" — developers added UUID generation wherever random bytes were available.
- **Problem**: Triple maintenance overhead. Any UUID format bug or RFC update must be fixed in 3 separate repositories.

### Issue C: Checksum Duplication (CRC-32 & Adler-32)
- **Current State**:
  - `Src/alya/stdlib/hash.alya`: Pure Alya string-based polynomial loop (`crc32(s)`, `adler32(s)`).
  - `Lib/compress/src/checksum.alya`: C-accelerated (`miniz` FFI) byte-based implementation (`crc32(bytes)`, `adler32(bytes)`).
- **Root Cause**: Compression formats (GZIP RFC 1952, ZLIB RFC 1950) require high-throughput checksum verification (> 1,000,000 ops/s) on byte streams, which pure interpreted/unoptimized loops cannot deliver for multi-megabyte streams.
- **Status**: Justified by performance requirements, but the naming and target types (string vs bytes) should be clearly separated.

### Issue D: Randomness & Entropy Delegation
- **Current State**:
  - `std/rand`: Simple global LCG PRNG (`rand_int`, `rand_float`, `rand_choice`).
  - `Lib/rand`: Multi-engine PRNG, statistical distributions, array sampling, and raw byte generation (`bytes(count)`).
  - `Lib/crypto`: Imports `rand` package and calls `rand::bytes(count)`.
- **Status**: The relationship between `std/rand` and `Lib/rand` is **correct** per Tier 2. The relationship between `Lib/rand` and `Lib/crypto` is **correct** per the package dependency graph (`crypto` depends on `rand`).

---

## 4. Action Plan & Refactoring Steps (Completed)

### Phase 1: Stdlib Clarification & Pruning
- [x] **Prune `std/hash`**:
  - Kept in `std/hash`: Non-cryptographic hash table hashing algorithms exclusively (`djb2`, `sdbm`, `fnv1`, `fnv1a`, `fnv1a64`, `murmur3_32`, `murmur3`, `jenkins`, `elf_hash`, `crc16`, `crc32`, `crc64`, `adler32`, `hash_combine`).
  - Relocated Encoding: `base64_encode`, `base64_decode`, `hex_encode`, `hex_decode`, `is_hex`, `is_base64` moved cleanly to `std/str`.
  - Updated `std/net` imports to use `std/str`.
- [x] **Preserve `std/rand` Pruned State**:
  - Kept `std/rand` strictly under 85 lines as a lightweight, zero-dependency LCG wrapper (`rand_seed_state`, `rand_auto_seed`, `rand_next`, `rand_int`, `rand_float`, `rand_float_range`, `rand_bool`, `rand_chance`, `rand_choice`).

### Phase 2: Package Deduplication Refactoring
- [x] **Clean `Lib/crypto` Boundaries**:
  - Removed `random_uuid()` from `Lib/crypto/src/random/entropy.alya` and its export in `src/lib.alya`.
  - Retained `random_bytes(count)` and `random_hex(count)` as legitimate cryptographic entropy helpers.
  - Canonical home for UUIDs is `alya-lang/uuid`.
- [x] **Prune Duplicate Identifiers from `Lib/rand`**:
  - Removed `src/core/identifiers.alya` (`uuid_v4`, `uuid_v7`, `ulid`, `nanoid`) from `Lib/rand`.
  - Removed `rng_uuid_v4`, `rng_uuid_v7`, `rng_ulid`, `rng_nanoid` from `Lib/rand/src/lib.alya`.
  - Updated tests, benchmarks, examples, and documentation to reference `alya-lang/uuid`.
- [x] **Clean `Lib/url` Struct Layout**:
  - Removed duplicate `url_*` struct fields (`url_scheme`, `url_host`, etc.) from `struct Url` in `Lib/url/src/types.alya`.
  - Updated `parser.alya`, `normalize.alya`, `path.alya`, and `query.alya` to strictly use the canonical 8-field layout.
- [x] **Harmonize Checksums**:
  - Documented dual-tier boundary: `std/hash` provides fast pure-Alya polynomial string checksums (`crc32`, `adler32`), while `Lib/compress` provides C-accelerated (`miniz` FFI) byte-stream verification for RFC 1950/1952 containers.

### Phase 3: Purging Legacy Compatibility Layers Across Packages (Clean Break)
Per Rule 5 (Clean Break / Zero Backward Compatibility), all transitional shims and backward compatibility layers were permanently eliminated:
- [x] **`Lib/cli`**: Deleted `src/core/compat.alya` and `tests/test_compat.alya`; updated README.
- [x] **`Lib/logger`**: Deleted `src/core/compat.alya` and `tests/test_compat.alya`; updated README.
- [x] **`Lib/term`**: Deleted `src/compat.alya` and `tests/test_compat.alya`; updated README.
- [x] **`Lib/csv`**: Deleted legacy stdlib aliases in `src/lib.alya` and `tests/test_legacy.alya`.
- [x] **`Lib/json`**: Deleted `std_json_*` legacy aliases in `src/lib.alya`, `tests/test_compat.alya`, and updated README.
- [x] **`Lib/rand`**: Deleted `rand_*` and `ulid_generate` legacy aliases in `src/lib.alya` and `tests/test_basic.alya`.
- [x] **Compiler (`Src/alya`)**: Removed hardcoded `"has moved to a standalone package"` compile errors in `src/parser/mod.rs` and updated compiler tests.

### Phase 4: Full Stdlib & Package Method Deduplication (Completed)
- [x] **`std/console` Pruning**: Removed high-level TUI widgets (`console_table`, `console_progress_bar`, `console_spinner_char`) from `Src/alya/stdlib/console.alya` (reduced to 94 lines). High-level UI/TUI belongs exclusively to `alya-lang/term`.
- [x] **`Lib/term` ANSI Deduplication**: Refactored `Lib/term/src/style.alya` to delegate all ANSI color and text decoration formatting directly to `std/color` (`import "std/color" as c`), eliminating duplicate ANSI escape code arrays and styling logic across the ecosystem.
- [x] **`Lib/logger` Disambiguation**: Renamed `struct Logger` to `struct AppLogger` in `Lib/logger/src/types.alya` to resolve global struct identifier collisions with `std/log`. Exposed clean namespaced top-level methods (`info`, `warn`, `error`, `debug`, etc.).
- [x] **`std/json` Pruning**: Removed redundant typed array helpers (`json_array_of_strings`, `json_array_of_numbers`, `json_array_of_bools`), map helper `json_string_map`, ad-hoc substring getters (`json_get_string`, `json_get_number`, `_parse_int`), and pretty printer `json_pretty` from `Src/alya/stdlib/json.alya`. Preserved clean, fast core serializers and recursive descent parser.
- [x] **Compiler Analysis Table Cleanup**: Removed obsolete stdlib symbols (`json_pretty`, `json_get_string`, `json_string_map`, `uuid_v4`, `uuid_v4_simple`, `uuid_v7`, `uuid_v7_at`, `uuid_v7_simple`, `uuid_v7_simple_at`, `ulid_generate`, `ulid_at`, `console_progress_bar`, `console_spinner_char`) from `Src/alya/src/codegen/analysis/predicates.rs` and `strings.rs`.
- [x] **Documentation Modernization**: Updated `standard-library.md` and `language-guide.md` to accurately reflect pruned stdlib modules and direct developers to official standalone packages (`alya-lang/uuid`, `alya-lang/term`, `alya-lang/logger`).

---

## 5. Ongoing Governance & Anti-Duplication Rules

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
Any hybrid module admitted into `std/*` (e.g. `std/cli`, `std/net`, `std/rand`, `std/log`, `std/json`):
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

## 6. Summary Matrix

| Capability | Stdlib (`std/*`) | Package (`Lib/*`) | Policy & Action |
| :--- | :--- | :--- | :--- |
| **Non-crypto Hash** (Murmur, FNV, DJB2) | `std/hash` ✅ | None | Canonical home in `std/hash`. |
| **Crypto Hash & Ciphers** (SHA, AES, ChaCha) | None | `Lib/crypto` ✅ | Canonical home in `Lib/crypto`. Never put in `std/hash`. |
| **Basic PRNG** (LCG range, choice) | `std/rand` ✅ | `Lib/rand` ✅ | Tier 2 Hybrid. `std/rand` pruned (<85 lines); `Lib/rand` full-featured. |
| **UUID (v4, v7, ULID)** | None | `Lib/uuid` ✅ | Canonical home in `Lib/uuid`. Remove duplicate `random_uuid` from `Lib/crypto`. |
| **Hex & Base64** | `std/hash` ❌ | `Lib/crypto` ✅ | Prune from `std/hash`. Relocate to `std/str` or dedicated encoding. |
| **Checksums (CRC-32, Adler-32)** | `std/hash` (string) | `Lib/compress` (bytes, C FFI) | Dual-tier justified by FFI decompression speed requirement. |
| **Compression** (Brotli, Zstd, LZ, Huffman) | None | `Lib/compress` ✅ | Canonical home in `Lib/compress`. |
