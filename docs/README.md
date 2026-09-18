# Alya Documentation Wiki

Welcome to the official **Alya Programming Language** documentation wiki. This guide is organized progressively—from the simplest fundamental concepts to production-grade architectural patterns.

---

## 🧭 Syllabus & Learning Path

```text
Getting Started ──► Language Basics ──► Control Flow ──► Functions & Modules
                                                                │
Architecture ◄── Standard Library ◄── Error Handling ◄── Data Structures
```

| Chapter | Topic | Highlights | Complexity |
|:---|:---|:---|:---:|
| **[1. Getting Started](getting-started.md)** | Toolchain & Workflow | Installing `alyac`, compiling binaries, running scripts, CLI tools (`fmt`, `test`, `--time`, `--arch`) | 🟢 Beginner |
| **[2. Language Basics](basics.md)** | Syntax & Fundamentals | Variables (`let`), `null`, bitwise operators, ternary `? :`, null coalescing `??`, strings (`"""..."""`, \`...\`), `say`, `ask` | 🟢 Beginner |
| **[3. Control Flow](control-flow.md)** | Decision & Iteration | `if`/`elif`/`else`, inline `if` & ternary, `while`, `for .. in`, `repeat`, `break`/`continue`, `when` | 🟢 Beginner |
| **[4. Functions & Modules](functions-and-modules.md)** | Code Organization | Defining functions, default parameters, recursion, file imports (`import`), cycle prevention | 🟡 Intermediate |
| **[5. Data Structures](data-structures.md)** | Collections & Structs | Dynamic arrays, Hash Maps (`map()`), structs (`struct Point`), multi-pass type inference | 🟡 Intermediate |
| **[6. Error Handling](error-handling.md)** | Safety & Exceptions | Structured `try ... catch ... finally`, runtime guards (div-by-zero, bounds), custom `throw` | 🟡 Intermediate |
| **[7. Standard Library Reference](standard-library.md)** | Batteries Included | Complete catalog: `std/net`, `std/console`, `std/glob`, `std/rand` (UUID v4/v7), `std/time`, `std/str`, `std/math`, `std/fs`... | 🔴 Advanced |
| **[8. Architecture & Internals](architecture-and-internals.md)** | Compiler & Codegen | Pipeline, ARM64/x64/x86 codegen, Branch Fusion, Struct Inference, ARM64 Immediate Range Splitting | 🔴 Advanced |

---

## 🎯 Progressive Pedagogy

Every topic in this wiki is structured in three progressive tiers:
1. **Level 1 — Pure & Minimal**: The simplest, single-purpose form with zero boilerplate.
2. **Level 2 — Practical & Idiomatic**: Real-world usage combining control flow and built-ins.
3. **Level 3 — Advanced & Real-World**: Complex, robust implementations handling edge cases and performance considerations.

---

## ⚡ Quick Links
* 🚀 **GitHub Repository**: [alya-lang/alya](https://github.com/alya-lang/alya)
* 🗺️ **Project Roadmap**: [ROADMAP.md](../ROADMAP.md)
* 📱 **Applications Showcase**: [apps/](../apps/README.md)
* 📐 **Language Specification**: [spec/](../spec/)
* 📦 **Releases & Downloads**: [GitHub Releases](https://github.com/alya-lang/alya/releases)
