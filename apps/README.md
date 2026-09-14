# Alya Apps Collection

A curated collection of real-world, production-ready terminal applications, network services, games, and developer tools built entirely in **Alya**.

Each application showcases the language's capabilities: near-C execution performance, zero-dependency standard library (`std/net`, `std/thread`, `std/console`, `std/time`, `std/color`, `std/rand`, `std/str`, `std/os`, `std/fs`), and cross-platform native compilation.

---

## Application Showcase

| Application | Path | Category | Highlights & Standard Library |
| :--- | :--- | :--- | :--- |
| **[Game of Life](#1-conways-game-of-life-game_of_life)** | [`apps/game_of_life/`](game_of_life/main.alya) | Simulation / Graphics | Toroidal 2D grid, preset seeds, ANSI terminal animation, `std/console`, `std/time` |
| **[HTTP Benchmark](#2-http-benchmark-tool-http_bench)** | [`apps/http_bench/`](http_bench/main.alya) | Networking / Performance | Concurrent load generator (`wrk`/`ab` style), Keep-Alive reuse, worker pool, `std/net`, `std/thread` |
| **[HTTP Web Server](#3-native-http-web-server-http_server)** | [`apps/http_server/`](http_server/main.alya) | Backend / Networking | High-performance HTTP/1.1 server, REST JSON APIs, static file delivery, non-blocking I/O, `std/net` |
| **[Port Scanner](#4-tcp-port-scanner-port_scanner)** | [`apps/port_scanner/`](port_scanner/main.alya) | Security / Networking | Asynchronous TCP port prober, service identification, ASCII progress bar, `std/net`, `std/time` |
| **[Snake](#5-snake-arcade-game-snake)** | [`apps/snake/`](snake/main.alya) | Terminal Game / AI | Interactive manual mode (WASD), autonomous AI autopilot mode, high-score tracking, `std/color` |
| **[Tic Tac Toe](#6-tic-tac-toe-tictactoe)** | [`apps/tictactoe/`](tictactoe/main.alya) | Terminal Game / AI | Player-vs-Player and Player-vs-AI with unbeatable Minimax algorithm, ANSI board, `std/console` |
| **[Todo Manager](#7-terminal-todo-manager-todo)** | [`apps/todo/`](todo/src/main.alya) | Productivity / Tooling | Package manager integration (`term`), task DB, ANSI badges, progress bar, `std/fs` |

---

## Application Details & Usage

### 1. Conway's Game of Life (`game_of_life`)
A cellular automaton simulating Conway's 4 rules on a wrap-around toroidal 2D board with smooth ANSI animation.

* **Key Features**: Glider, R-pentomino, Blinker, and Random Soup seed presets; real-time generation and live population counters; smoke test mode.
* **Run**:
  ```bash
  # Run with default settings
  alyac run apps/game_of_life/main.alya

  # Glider preset for 80 generations
  alyac run apps/game_of_life/main.alya -- --pattern glider --gens 80

  # Random soup preset for 120 generations
  alyac run apps/game_of_life/main.alya -- --pattern random --gens 120

  # Automated smoke test (non-interactive)
  alyac run apps/game_of_life/main.alya -- --test
  ```

---

### 2. HTTP Benchmark Tool (`http_bench`)
A high-throughput HTTP benchmarking utility inspired by `wrk` and `autocannon`.

* **Key Features**: Multi-threaded worker pool via OS threads (`std/thread`), persistent TCP socket reuse (HTTP/1.1 Keep-Alive), non-blocking socket polling (`tcp_poll`), latency percentiles, and formatted ANSI throughput metrics (req/sec).
* **Run**:
  ```bash
  # Benchmark target with 1,000 requests over 4 concurrent connections
  alyac run apps/http_bench/main.alya -- -u http://127.0.0.1:8080/ -n 1000 -c 4

  # Keep-Alive benchmark with 5,000 requests over 8 workers
  alyac run apps/http_bench/main.alya -- -u http://127.0.0.1:8080/api/status -n 5000 -c 8 -k

  # Automated smoke test
  alyac run apps/http_bench/main.alya -- --test
  ```

---

### 3. Native HTTP Web Server (`http_server`)
A full-featured native HTTP/1.1 web server built with raw POSIX/Winsock sockets.

* **Key Features**: Static asset delivery (HTML, CSS, text), dynamic JSON REST endpoints (`/api/status`, `/api/echo`), ANSI colored access logging with HTTP status codes, and configurable worker threads.
* **Run**:
  ```bash
  # Start on default port (8080)
  alyac run apps/http_server/main.alya

  # Custom port and worker threads
  alyac run apps/http_server/main.alya -- --port 3000 --workers 4

  # Automated test mode
  alyac run apps/http_server/main.alya -- --test
  ```

---

### 4. TCP Port Scanner (`port_scanner`)
A rapid network reconnaissance tool that probes target hosts and services.

* **Key Features**: Probes top standard service ports (HTTP, HTTPS, SSH, MySQL, Postgres, Redis, etc.), measures round-trip connect latency, renders an in-place ASCII progress bar, and outputs a formatted results table.
* **Run**:
  ```bash
  # Scan localhost
  alyac run apps/port_scanner/main.alya -- -t 127.0.0.1

  # Automated simulation / smoke test
  alyac run apps/port_scanner/main.alya -- --test
  ```

---

### 5. Snake Arcade Game (`snake`)
A classic Snake game rendered directly inside the terminal using ANSI escape codes.

* **Key Features**: Interactive turn-based controls (`W`/`A`/`S`/`D` + `Enter`), collision mechanics, dynamic food spawning, high-score tracking, and an autonomous AI Autopilot mode that plays itself.
* **Controls**:
  * `W`: Move Up &nbsp;|&nbsp; `S`: Move Down &nbsp;|&nbsp; `A`: Move Left &nbsp;|&nbsp; `D`: Move Right
  * `Enter`: Maintain current heading
  * `Q`: Quit
* **Run**:
  ```bash
  # Interactive mode
  alyac run apps/snake/main.alya

  # Autonomous AI autopilot demonstration
  alyac run apps/snake/main.alya -- --demo

  # Automated test mode
  alyac run apps/snake/main.alya -- --test
  ```

---

### 6. Tic Tac Toe (`tictactoe`)
An interactive, ANSI-colored board game supporting two-player local matches and an unbeatable AI opponent.

* **Key Features**: Minimax decision tree algorithm with terminal state evaluation, round history and score retention, and robust input validation.
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
  # Launch game
  alyac run apps/tictactoe/main.alya
  ```

---

### 7. Terminal Todo Manager (`todo`)
A persistent terminal task manager and productivity tracker built using the **Alya Package Manager (`alyac pkg`)**.

* **Key Features**: Consumes the official [`term`](https://github.com/alya-lang/term) package (`import "term" as ui`) locked in `alya.lock`, auto-increment IDs, priority classification (🔴 HIGH, 🟡 MED, 🟢 LOW), category tags (`#core`, `#docs`, `#apps`), disk persistence (`todo.db`), real-time completion progress bar, and dual CLI / interactive REPL modes.
* **Run**:
  ```bash
  # Run as a package (automatically resolves alya.toml and dependencies)
  cd apps/todo
  alyac run

  # Inspect package dependencies and lockfile
  cd apps/todo
  alyac pkg list

  # Add new tasks
  alyac run apps/todo/src/main.alya -- add "Build C FFI engine" --pri high --tag core
  alyac run apps/todo/src/main.alya -- add "Write LSP docs" --pri med --tag docs

  # List tasks and view progress
  alyac run apps/todo/src/main.alya -- list

  # Mark task completed
  alyac run apps/todo/src/main.alya -- done 1

  # View productivity metrics
  alyac run apps/todo/src/main.alya -- stats

  # Automated test suite
  alyac run apps/todo/src/main.alya -- --test
  ```

---

## Building & Packaging Applications

### 1. Compile to a Standalone Executable

To produce a single, self-contained native binary that runs without the compiler or any runtime dependencies:

```bash
# On Linux / macOS
alyac build apps/http_bench/main.alya -o http_bench

# On Windows (produces http_bench.exe)
alyac build apps/http_bench/main.alya -o http_bench.exe
```

### 2. Package as a macOS Application Bundle (`.app`)

Alya features built-in macOS Application Bundling via the `--bundle` (or `--app`) flag:

```bash
# Basic bundle creation (produces Snake.app with default Alya App icon)
alyac build apps/snake/main.alya --bundle -o Snake.app

# Full bundle with custom Bundle ID and custom icon
alyac build apps/game_of_life/main.alya \
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

> 💡 **Default Icon**: When `--icon` is omitted, `alyac` automatically embeds the official **Alya Application Icon** (`assets/brand/icons/alya-app-dark.icns`) with zero external file dependencies.

---

## Automated Smoke Testing

All applications include an automated `--test` flag suitable for CI/CD test runners and automated verification without hanging on interactive terminal inputs:

```bash
alyac run apps/game_of_life/main.alya -- --test
alyac run apps/http_bench/main.alya -- --test
alyac run apps/http_server/main.alya -- --test
alyac run apps/port_scanner/main.alya -- --test
alyac run apps/snake/main.alya -- --test
alyac run apps/todo/main.alya -- --test
```
