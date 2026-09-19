# Alya Zero-Setup Toolchain & Distribution Architecture Roadmap

This technical specification and roadmap outlines the multi-phase architecture for Alya's zero-setup compilation toolchain, automated provisioning, standalone distribution, and the long-term migration to a self-hosted native linker engine.

---

## Status Legend

| Status | Meaning |
| :---: | :--- |
| 🚧 | **In Progress** — Active implementation underway. |
| 📋 | **Planned** — Prioritized technical specification ready for implementation. |
| 💡 | **Exploration** — Conceptual research and architectural design. |
| ✅ | **Completed** — Implemented, tested, and verified. |

---

## Architectural Vision: Smart Toolchain Fallback

Alya programs compile to native GNU/Mach-O assembly and coordinate with C object files (e.g., C-FFI, SQLite3). Rather than forcing users to manually install MinGW-w64 or configure system `PATH` variables, `alya` resolves the assembler and linker toolchain using a multi-tier fallback mechanism:

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                       alya compile / run / build                       │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                 ┌───────────────────┴───────────────────┐
                 ▼                                       ▼
       [1. System Toolchain]                   [2. Local Toolchain]
    Is `gcc` / `clang` in PATH?            Does `~/.alya/toolchain` exist?
                 │                                       │
            YES ─┼─► Use Host Compiler              YES ─┼─► Use Cached Toolchain
            NO   │                                  NO   │
                 └───────────────────┬───────────────────┘
                                     │
                                     ▼
                        [3. Automatic Provisioning]
                    Prompt: "Download toolchain? [Y/n]"
                                     │
                         YES ────────┼──────── NO
                         │                     │
                         ▼                     ▼
               Download & Extract        Exit with Friendly
             Minimal Toolchain (~18MB)    Diagnostic Guide
```

---

## Foundation Milestones (Completed)

### Phase 1: Multi-Platform Toolchain Setup (`Src/toolchain`) ✅
**Objective**: Build and maintain an ultra-minimal, portable toolchain suite across Windows (x64), Linux (x64 & ARM64), and macOS (Apple Silicon & Intel), stripped of non-essential binaries, headers, and documentation.

- [x] **Directory Structure Initialization**:
  - Setup `Src/toolchain` repository workspace.
  - Automated build scripts: `package.ps1` (Windows), `package-linux.sh` (Linux), `package-macos.sh` (macOS), `verify.ps1`.
- [x] **Minimal Toolchain Assembly**:
  - **Windows (x64)**: Curate `gcc.exe`, `as.exe`, `ld.exe`, `ar.exe`, `strip.exe`, `objdump.exe`, CRT `crt2.o`, `ws2_32`, `kernel32`, and C headers.
  - **Linux (x64 & ARM64)**: 100% statically-linked musl-GCC and binutils (`gcc`, `as`, `ld`), independent of host glibc.
  - **macOS (Apple Silicon & Intel)**: Mach-O Clang and LLD driver with ad-hoc codesigning.
- [x] **Release Metadata & Multi-Platform Manifest**:
  - Multi-platform `toolchain.json` schema documenting all 5 architecture targets, SHA-256 checksums, and extraction paths.
  - Verification tests passing on isolated GNU assembly and C/Winsock2 linking.

---

### Phase 2: `alya` Smart Multi-Platform Toolchain Resolver ✅
**Objective**: Integrate automatic toolchain resolution and zero-friction provisioning directly into the `alya` compiler across all operating systems.

- [x] **Resolver Engine (`src/driver/toolchain.rs`)**:
  - `detect_system_toolchain()`: Verify existing host `gcc`/`clang` via `PATH` lookup and `--version` check.
  - `detect_local_toolchain()`: Inspect standard local path (`~/.alya/toolchain/bin/`).
  - **macOS Handler**: Detect Apple Command Line Tools; trigger automated `xcode-select --install` if absent.
  - **Linux Handler**: Detect system package managers (`apt`, `dnf`, `pacman`, `apk`) and provide instant install commands, or fall back to static musl toolchain.
- [x] **Seamless Auto-Downloader**:
  - Interactive terminal prompt on first compile failure:
    `"C/Assembly build tools not detected. Download minimal toolchain (~18 MB)? [Y/n]"`
  - Headless/CI support via `--yes` or `ALYA_TOOLCHAIN_AUTO_INSTALL=1`.
  - Download stream with terminal progress indicator and SHA-256 checksum verification.
  - Automatic decompression into `~/.alya/toolchain`.
- [x] **Driver Integration**:
  - Updated [`src/driver/runner.rs`](../src/driver/runner.rs) and [`src/driver/c_builder.rs`](../src/driver/c_builder.rs) to use the resolved toolchain binary path instead of bare `"gcc"`.
  - Automated ad-hoc codesigning on macOS Apple Silicon ARM64.
- [x] **CLI Management Commands**:
  - `alya toolchain status`: Display active compiler path and version.
  - `alya toolchain install`: Pre-emptively fetch the latest toolchain without waiting for compile errors.
  - `alya toolchain clean`: Purge the cached toolchain directory.

---

### Phase 3: CI/CD Pipeline & Multi-Platform Distribution ✅
**Objective**: Automate reproducible multi-platform toolchain builds, automated testing, and release asset hosting.

- [x] **GitHub Actions Matrix**:
  - Parallel build runners: `windows-latest`, `ubuntu-latest` (Linux x64 + ARM64), `macos-latest` (Apple Silicon + Intel).
  - Automated weekly builds and releases under `alya-lang/toolchain`.
  - Automated post-build verification tests on each OS before uploading artifacts.
  - Automated `SHA256SUMS.txt` generation and multi-platform `toolchain.json` manifest synchronization.
- [x] **Asset Mirroring & Fallback URLs**:
  - Primary download: GitHub Releases API.
  - Secondary fallback CDN mirrors (jsDelivr, raw GitHub content) to guarantee 100% uptime during rate limiting or outages.
  - `ALYA_TOOLCHAIN_URL` environment variable support for custom air-gapped/enterprise mirrors.

---

### Phase 4: Standalone Offline Bundles & Package Managers ✅
**Objective**: Provide 100% offline-ready distributions for restricted or air-gapped environments.

- [x] **Dual Release Artifacts**:
  - `alya-windows-x64.zip` (compact ~8 MB compiler; fetches toolchain on demand).
  - `alya-windows-x64-standalone.zip` (~25 MB complete package with pre-bundled toolchain; zero internet required).
  - Portable sibling detection: `alya` automatically detects adjacent `toolchain/bin/` with zero configuration.
- [x] **Package Manager Distribution**:
  - WinGet manifest template (`packages/distribution/winget/alya.yaml`).
  - Scoop bucket recipe (`packages/distribution/scoop/alya.json`).

---

## Active & Long-Term Roadmap (Remaining Milestones)

### Phase 5: Long-Term Self-Hosting & Native PE/ELF Linker Engine 💡
**Objective**: Transition Alya to full compiler self-sufficiency, completely removing external GCC/MinGW/Clang and LLD dependencies.

#### 5.1 Self-Hosted Compiler Core (`alya.alya`) 📋
- Port the `alya` compiler frontend (lexer, Pratt parser, AST, symbol resolver, and type checker) from Rust to native Alya code.
- Prerequisite: Language feature stability following v1.0 specifications (ARC, Bacon-Rajan Cycle Collector, Gradual Typing from `ROADMAP.md` Phase 6.2).
- Three-stage bootstrap verification:
  1. **Stage 0**: Existing Rust compiler (`target/release/alya`) compiles `alya.alya` -> generates `alya-stage1`.
  2. **Stage 1**: `alya-stage1` compiles `alya.alya` -> generates `alya-stage2`.
  3. **Stage 2 Verification**: `alya-stage1` and `alya-stage2` binaries must be bit-for-bit identical (reproducible compiler build).

#### 5.2 Direct In-Compiler Object Emitter (PE-COFF & ELF64) 💡
- Replace intermediate textual assembly emission (`.s`) and external assembler invocations (`as.exe` / `gcc -c`) with direct in-memory binary object generation.
- **Windows (x64 PE-COFF)**:
  - Direct binary emission of `.text`, `.rdata`, `.data`, `.pdata`, and `.xdata` (structured exception handling) sections into `.obj` format.
  - Generation of COFF symbol tables, string tables, and section relocations (`IMAGE_REL_AMD64_ADDR64`, `IMAGE_REL_AMD64_REL32`).
- **Linux (x64 & ARM64 ELF64)**:
  - Direct emission of `.text`, `.rodata`, `.data`, `.bss` sections into `.o` ELF64 format.
  - Generation of ELF symbol tables (`.symtab`, `.strtab`) and relocation entries (`R_X86_64_PC32`, `R_X86_64_64`, `R_AARCH64_ADR_PREL_PG_HI21`).

#### 5.3 Pure Native Linker Engine 💡
- Implement a zero-dependency, pure Alya linker engine capable of linking object files and foreign C static libraries directly into standalone executables.
- **PE Executable Generation (Windows)**:
  - Emission of standard DOS stub and `IMAGE_NT_HEADERS64`.
  - Base relocation (`.reloc`) table generation and image rebasing.
  - Import Address Table (IAT) synthesis for dynamic linking against Windows subsystem libraries (`kernel32.dll`, `ws2_32.dll`, `msvcrt.dll`, `ntdll.dll`).
- **ELF Executable Generation (Linux)**:
  - Emission of ELF header, Program Headers (`PT_LOAD`, `PT_DYNAMIC`, `PT_INTERP`), and Section Headers.
  - Support for both static linking (musl libc) and dynamic linking (glibc dynamic loader `/lib64/ld-linux-x86-64.so.2`).
- **End State**: Zero external dependencies. A single `alya.exe` binary will compile, assemble, and link native executables out of the box with zero downloads and zero external toolchain prerequisites.

---

## 🔗 Relationship to Language Spec (`ROADMAP.md`)

This toolchain roadmap complements [`Src/alya/spec/ROADMAP.md`](ROADMAP.md):
- **[`ROADMAP.md`](ROADMAP.md)** defines *the language features, standard library, and runtime semantics* (ARC, Bacon-Rajan GC, Fibers, Parser, LSP, Linter).
- **`TOOLCHAIN_ROADMAP.md`** defines *how the code is transformed, packaged, linked, and distributed* to end users across platforms with zero friction.
