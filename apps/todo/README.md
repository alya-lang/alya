# Alya Todo Manager (`todo`)

A sleek, persistent terminal task manager and productivity tracker built in **Alya**.

Demonstrates the **Alya Package Manager (`alyac pkg`)** in action by consuming the external reusable package **`term`** declared in [`alya.toml`](alya.toml) and deterministically locked in [`alya.lock`](alya.lock).

---

## Features

- **Package Manager Integration**: Consumes the official [`term`](https://github.com/alya-lang/term) package (`import "term" as ui`) for standardized Unicode box borders and ANSI badges.
- **Persistent Disk Storage**: Saves your tasks into a local database file (`todo.db`) across terminal sessions.
- **Priority Badging**: Classify tasks by priority with distinctive ANSI badges:
  - 🔴 **HIGH**: Critical and urgent milestones
  - 🟡 **MED**: Normal tasks and features
  - 🟢 **LOW**: Nice-to-have improvements
- **Category Tagging**: Organize tasks by domain or team (e.g. `#core`, `#docs`, `#apps`, `#ui`).
- **Progress Tracking**: Real-time ASCII progress bar and completion rate metrics.
- **Productivity Dashboard**: Summary statistics breaking down completion percentages and priority distributions.
- **Dual Operational Modes**:
  - **CLI Mode**: Fast one-liner terminal commands for shell scripting and automation.
  - **Interactive REPL**: Focused, interactive command prompt (`alya-todo>`).
- **Zero External Middleware**: Standard library (`std/fs`, `std/str`, `std/color`, `std/console`, `std/os`, `std/time`) + pure Alya package ecosystem.

---

## Package Manifest & Lockfile

### `alya.toml`
```toml
[package]
name = "todo"
version = "1.0.0"
alya-version = "0.0.17"
entry = "src/main.alya"
description = "A sleek terminal task manager with priorities, tags, and persistence"
authors = ["Alya Language Contributors <https://github.com/alya-lang>"]
license = "MIT"
homepage = "https://github.com/alya-lang/alya"
repository = "https://github.com/alya-lang/alya"
keywords = ["alya", "alya-lang", "package", "todo", "cli", "app", "productivity"]

[dependencies]
term = { git = "https://github.com/alya-lang/term", tag = "v0.2.0" }
```

### Inspect Package Status
```bash
cd apps/todo
alyac pkg list
```

Output:
```text
Package: todo v1.0.0
Entry:   src/main.alya
About:   A sleek terminal task manager with priorities, tags, and persistence

Dependencies (1):
  • term             git: https://github.com/alya-lang/term (tag: v0.2.0) [locked: sha256:799a42f235...]
```

---

## Quick Start

### 1. Run as a Package
```bash
# Inside apps/todo (automatically resolves alya.toml, dependencies, and src/main.alya)
cd apps/todo
alyac run
```

### 2. Direct Compilation & Execution
```bash
# Interactive REPL shell
alyac run apps/todo/src/main.alya

# Add new tasks
alyac run apps/todo/src/main.alya -- add "Implement C FFI Engine" --pri high --tag core
alyac run apps/todo/src/main.alya -- add "Write LSP language server" --pri med --tag tooling
alyac run apps/todo/src/main.alya -- add "Refactor snake collision" --pri low --tag apps

# List all tasks
alyac run apps/todo/src/main.alya -- list

# Mark task #1 as done
alyac run apps/todo/src/main.alya -- done 1

# Filter pending tasks
alyac run apps/todo/src/main.alya -- list --pending

# View productivity metrics
alyac run apps/todo/src/main.alya -- stats

# Automated smoke test suite
alyac run apps/todo/src/main.alya -- --test
```

---

## Commands & Options

| Command | Arguments / Options | Description |
| :--- | :--- | :--- |
| `add` | `<title> [--pri high\|med\|low] [--tag <tag>]` | Add a new task with priority and tag |
| `list` | `[--all] [--pending] [--done] [--tag <tag>]` | Render formatted ANSI task table |
| `done` | `<id>` | Mark specified task as completed |
| `undone` | `<id>` | Reopen a completed task |
| `rm` / `delete` | `<id>` | Permanently remove a task |
| `clear` | *(none)* | Purge all completed tasks |
| `stats` | *(none)* | Show total tasks, completion percentage, and distribution |
| `help` | *(none)* | Display help banner and usage instructions |
| `exit` / `quit` | *(none)* | Exit interactive mode |

---

## Architecture & Structure

- [`alya.toml`](alya.toml): Project manifest declaring the `term` package dependency.
- [`alya.lock`](alya.lock): Cryptographic SHA-256 lockfile ensuring reproducible dependency resolution.
- [`src/main.alya`](src/main.alya): Application entry importing `term` as `ui`.
