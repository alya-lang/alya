# Alya Apps Collection

A curated collection of real-world, production-ready terminal applications, network services, games, and developer tools built entirely in **Alya**.

Each application showcases the language's capabilities: near-C execution performance, modular package ecosystem (`term`, `url`, `mime`, `rand`, `crypto`), high-level abstractions, and cross-platform native compilation to standalone binaries.

Every project in this directory is a first-class **Alya package** equipped with its own `alya.toml`, locked dependencies (`alya.lock`), and automated CI/CD smoke test support (`--test`).

---

## Application Showcase

| Application | Path | Category | Highlights & Ecosystem Packages / Stdlib |
| :--- | :--- | :--- | :--- |
| **[Game of Life](#1-conways-game-of-life-game_of_life)** | [`apps/game_of_life/`](game_of_life/main.alya) | Simulation / Graphics | Toroidal 2D grid, preset seeds, ANSI animation, `term`, `rand`, `std/time` |
| **[HTTP Benchmark](#2-http-benchmark-tool-http_bench)** | [`apps/http_bench/`](http_bench/main.alya) | Networking / Performance | Concurrent load generator (`wrk`/`ab` style), Keep-Alive reuse, worker pool, `url`, `term`, `std/net`, `std/thread` |
| **[HTTP Web Server](#3-native-http-web-server-http_server)** | [`apps/http_server/`](http_server/main.alya) | Backend / Networking | High-performance HTTP/1.1 server, REST JSON APIs, static file delivery with auto MIME detection, non-blocking I/O, `mime`, `term`, `std/net` |
| **[Port Scanner](#4-tcp-port-scanner-port_scanner)** | [`apps/port_scanner/`](port_scanner/main.alya) | Security / Networking | Asynchronous TCP port prober, service identification, progress bar and table formatting with `term`, `std/net`, `std/time` |
| **[Snake](#5-snake-arcade-game-snake)** | [`apps/snake/`](snake/main.alya) | Terminal Game / AI | Interactive manual mode (WASD), autonomous AI autopilot mode, high-score tracking, `term`, `rand`, `std/console` |
| **[Tic Tac Toe](#6-tic-tac-toe-tictactoe)** | [`apps/tictactoe/`](tictactoe/main.alya) | Terminal Game / AI | Player-vs-Player and Player-vs-AI with unbeatable Minimax algorithm, Unicode scoreboard and box UI with `term`, `rand` |
| **[Todo Manager](#7-terminal-todo-manager-todo)** | [`apps/todo/`](todo/main.alya) | Productivity / Tooling | Package manager integration, task DB, ANSI badges, progress bar, `term`, `crypto`, `rand`, `std/fs` |

---

## Application Details & Usage

### 1. Conway's Game of Life (`game_of_life`)
A cellular automaton simulating Conway's 4 rules on a wrap-around toroidal 2D board with smooth ANSI animation and styled terminal callout boxes.

* **Ecosystem Packages**: `term` (banners & UI), `rand` (stochastic soup generation).
* **Key Features**: Glider, R-pentomino, Blinker, and Random Soup seed presets; real-time generation and live population counters; smoke test mode.
* **Run**:
  ```bash
  # Run as a package from its directory
  cd apps/game_of_life && alya run

  # Or run directly via file path
  alya run apps/game_of_life/main.alya

  # Glider preset for 80 generations
  alya run apps/game_of_life/main.alya -- --pattern glider --gens 80

  # Random soup preset for 120 generations
  alya run apps/game_of_life/main.alya -- --pattern random --gens 120

  # Automated smoke test (non-interactive)
  alya run apps/game_of_life/main.alya -- --test
  ```

---

### 2. HTTP Benchmark Tool (`http_bench`)
A high-throughput HTTP benchmarking utility inspired by `wrk` and `autocannon`.

* **Ecosystem Packages**: `url` (URL parsing, hostname/port/scheme extraction), `term` (table formatting & metrics).
* **Key Features**: Multi-threaded worker pool via OS threads (`std/thread`), persistent TCP socket reuse (HTTP/1.1 Keep-Alive), non-blocking socket polling (`tcp_poll`), latency percentiles, and formatted throughput metrics (req/sec).
* **Run**:
  ```bash
  # Run as a package
  cd apps/http_bench && alya run

  # Benchmark target with 1,000 requests over 4 concurrent connections
  alya run apps/http_bench/main.alya -- -u http://127.0.0.1:8080/ -n 1000 -c 4

  # Keep-Alive benchmark with 5,000 requests over 8 workers
  alya run apps/http_bench/main.alya -- -u http://127.0.0.1:8080/api/status -n 5000 -c 8 -k

  # Automated smoke test
  alya run apps/http_bench/main.alya -- --test
  ```

---

### 3. Native HTTP Web Server (`http_server`)
A full-featured native HTTP/1.1 web server built with raw POSIX/Winsock sockets and automated MIME content-type resolution.

* **Ecosystem Packages**: `mime` (file extension & Content-Type mapping), `term` (banners & HTTP access logging).
* **Key Features**: Static asset delivery (HTML, CSS, JS, images), dynamic JSON REST endpoints (`/api/status`, `/api/echo`), ANSI colored access logging with HTTP status codes, and configurable worker threads.
* **Run**:
  ```bash
  # Run as a package on default port (8080)
  cd apps/http_server && alya run

  # Custom port and worker threads
  alya run apps/http_server/main.alya -- --port 3000 --workers 4

  # Automated test mode
  alya run apps/http_server/main.alya -- --test
  ```

---

### 4. TCP Port Scanner (`port_scanner`)
A rapid network reconnaissance tool that probes target hosts and services.

* **Ecosystem Packages**: `term` (progress bar & tabular report rendering).
* **Key Features**: Probes top standard service ports (HTTP, HTTPS, SSH, MySQL, Postgres, Redis, etc.), measures round-trip connect latency, renders a dynamic terminal progress bar, and outputs a formatted results table.
* **Run**:
  ```bash
  # Run as a package
  cd apps/port_scanner && alya run

  # Scan localhost
  alya run apps/port_scanner/main.alya -- -t 127.0.0.1

  # Automated simulation / smoke test
  alya run apps/port_scanner/main.alya -- --test
  ```

---

### 5. Snake Arcade Game (`snake`)
A classic Snake game rendered directly inside the terminal with clean box formatting and AI autoplay.

* **Ecosystem Packages**: `term` (callout boxes & color styling), `rand` (stochastic food placement).
* **Key Features**: Interactive turn-based controls (`W`/`A`/`S`/`D` + `Enter`), collision mechanics, dynamic food spawning, high-score tracking, and an autonomous AI Autopilot mode that plays itself.
* **Controls**:
  * `W`: Move Up &nbsp;|&nbsp; `S`: Move Down &nbsp;|&nbsp; `A`: Move Left &nbsp;|&nbsp; `D`: Move Right
  * `Enter`: Maintain current heading
  * `Q`: Quit
* **Run**:
  ```bash
  # Interactive mode
  cd apps/snake && alya run

  # Autonomous AI autopilot demonstration
  alya run apps/snake/main.alya -- --demo

  # Automated test mode
  alya run apps/snake/main.alya -- --test
  ```

---

### 6. Tic Tac Toe (`tictactoe`)
An interactive, ANSI-colored board game supporting two-player local matches and an unbeatable AI opponent.

* **Ecosystem Packages**: `term` (table scoreboard, callout banners, color styling), `rand` (randomized first-player selection).
* **Key Features**: Minimax decision tree algorithm with terminal state evaluation, round history and score retention in Unicode tables, robust input validation, and non-interactive smoke testing.
* **Controls**: Enter numbers `1`–`9` corresponding to the 3×3 grid:
  ```text
   1 | 2 | 3
  ---+---+---
   4 | 5 | 6
  ---+---+---
   7 | 8 | 9
  ```
* **Run**:
  ```bash
  # Launch interactive game
  cd apps/tictactoe && alya run

  # Automated smoke test
  alya run apps/tictactoe/main.alya -- --test
  ```

---

### 7. Terminal Todo Manager (`todo`)
A persistent terminal task manager and productivity tracker built using the **Alya Package Manager (`alya pkg`)**.

* **Ecosystem Packages**: `term` (tables, status badges, progress bars), `crypto` (task hashing), `rand` (task identifiers).
* **Key Features**: Auto-increment IDs, priority classification (🔴 HIGH, 🟡 MED, 🟢 LOW), category tags (`#core`, `#docs`, `#apps`), disk persistence, real-time completion progress bar, and dual CLI / interactive REPL modes.
* **Run**:
  ```bash
  # Run as a package (automatically resolves alya.toml and dependencies)
  cd apps/todo && alya run

  # Inspect package dependencies and lockfile
  cd apps/todo && alya pkg list

  # Add new tasks
  alya run apps/todo/main.alya -- add "Build C FFI engine" --pri high --tag core
  alya run apps/todo/main.alya -- add "Write LSP docs" --pri med --tag docs

  # List tasks and view progress
  alya run apps/todo/main.alya -- list

  # Mark task completed
  alya run apps/todo/main.alya -- done 1

  # View productivity metrics
  alya run apps/todo/main.alya -- stats

  # Automated test suite
  alya run apps/todo/main.alya -- --test
  ```

---

## Package Management & Dependencies

All applications can be managed using standard Alya package commands:

```bash
# Install / restore all locked packages
cd apps/http_bench
alya install

# List dependencies and versions
alya pkg list

# Run package entry point
alya run
```

---

## Building Standalone Binaries

To produce a single, self-contained native executable that runs without the compiler or any runtime dependencies:

```bash
# On Linux / macOS
alya build apps/http_bench/main.alya -o http_bench

# On Windows (produces http_bench.exe)
alya build apps/http_bench/main.alya -o http_bench.exe
```

### Packaging as a macOS Application Bundle (`.app`)

Alya features built-in macOS Application Bundling via the `--bundle` (or `--app`) flag:

```bash
# Basic bundle creation (produces Snake.app with default Alya App icon)
alya build apps/snake/main.alya --bundle -o Snake.app

# Full bundle with custom Bundle ID and custom icon
alya build apps/game_of_life/main.alya \
  --bundle \
  -o GameOfLife.app \
  --bundle-id com.company.gameoflife \
  --icon path/to/custom.icns
```

The resulting bundle adheres strictly to Apple's macOS directory specifications:
```text
GameOfLife.app/
└── Contents/
    ├── Info.plist                  # Configured with bundle ID, executable name, and version
    ├── MacOS/
    │   └── GameOfLife              # Native Mach-O binary
    └── Resources/
        └── AppIcon.icns            # Multi-resolution Apple ICNS (16px to 512px Retina)
```

> 💡 **Default Icon**: When `--icon` is omitted, `alya` automatically embeds the official **Alya Application Icon** (`assets/brand/icons/alya-app-dark.icns`) with zero external file dependencies.

---

## Automated Smoke Testing

All 7 applications include an automated `--test` flag suitable for CI/CD test runners and verification without hanging on interactive inputs:

```bash
alya run apps/game_of_life/main.alya -- --test
alya run apps/http_bench/main.alya -- --test
alya run apps/http_server/main.alya -- --test
alya run apps/port_scanner/main.alya -- --test
alya run apps/snake/main.alya -- --test
alya run apps/tictactoe/main.alya -- --test
alya run apps/todo/main.alya -- --test
```
