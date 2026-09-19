# Chapter 23: Standard Library Core Contracts (Tier-1 Baseline)

## 1. Specification Rules

### 1.1 Overview & Architecture Guarantees
The Alya Standard Library (`std/*`) is strictly limited to **low-level operating system syscall abstractions and core runtime primitives**. It is embedded directly within the `alya` binary with zero external dependencies.

#### Guarantees
- **Zero External Dependencies**: Standard library modules never require package installations or network access.
- **Cross-Platform Parity**: Identical APIs across Linux (ELF64), Windows (PE/COFF), and macOS (Mach-O).
- **Zero Duplication**: Alya strictly forbids having a "toy" version in stdlib and a "real" version in packages. If a domain is protocol-heavy or fast-evolving (like HTTP, TLS/Crypto, SQL, or full CLI frameworks), it belongs strictly in official standalone packages (`alya-lang/*`).
- **Bare-Metal Compatibility (`--no-std`)**: Applications targeting microcontrollers, bare-metal kernels, or custom embedded runtimes can pass `--no-std` to completely detach the standard library.

> 📖 **Architectural Policy & Deduplication Governance:** For detailed boundary rules, the 100-line pruning threshold, and anti-duplication governance between `std/*` and `Lib/*`, consult [`ECOSYSTEM_ARCHITECTURE.md`](../ECOSYSTEM_ARCHITECTURE.md).

---

### 1.2 The 14 Core Standard Library Modules

```text
┌────────────────────────────────────────────────────────────────────────┐
│                  THE 14 CANONICAL CORE MODULES (std/*)                 │
├───────────────────┬───────────────────┬────────────────────────────────┤
│ 1.  std/fs        │ 6.  std/net       │ 11. std/str                    │
│ 2.  std/path      │ 7.  std/sync      │ 12. std/collections            │
│ 3.  std/os        │ 8.  std/time      │ 13. std/console                │
│ 4.  std/process   │ 9.  std/mem       │ 14. std/test                   │
│ 5.  std/io        │ 10. std/math      │                                │
└───────────────────┴───────────────────┴────────────────────────────────┘
```

---

### 1.3 Module API Contracts

#### 1. `std/fs` (Filesystem I/O)
Handles cross-platform file reading, writing, and directory traversal:
```alya
function fs.read_string(path: string) -> string
function fs.write_string(path: string, content: string) -> bool
function fs.append_string(path: string, content: string) -> bool
function fs.exists(path: string) -> bool
function fs.remove(path: string) -> bool
function fs.mkdir(path: string, recursive: bool = true) -> bool
function fs.read_dir(path: string) -> string[]
function fs.glob(pattern: string) -> string[]
```

#### 2. `std/path` (Path Manipulation)
Provides portable path operations without hardcoding slashes:
```alya
function path.join(...parts: string[]) -> string
function path.basename(p: string) -> string
function path.dirname(p: string) -> string
function path.ext(p: string) -> string
function path.is_absolute(p: string) -> bool
function path.normalize(p: string) -> string
function path.glob_match(pattern: string, text: string) -> bool
```

#### 3. `std/os` (Process & Environment)
Exposes environment variables, platform identities, and command-line arguments:
```alya
function os.args() -> string[]
function os.has_arg(flag: string) -> bool
function os.arg_at(index: int, default_val: string = "") -> string
function os.getenv(name: string) -> string?
function os.setenv(name: string, value: string) -> bool
function os.pid() -> int
function os.cwd() -> string
function os.set_cwd(path: string) -> bool
function os.exit(code: int = 0)
function os.platform() -> string      # "windows", "linux", "macos"
function os.arch() -> string          # "x64", "arm64", "x86"
enum os.OS { Windows, Linux, MacOS, Unknown }
enum os.Arch { X64, X86, ARM64, Unknown }
function os.current_os() -> os.OS
function os.current_arch() -> os.Arch
```

#### 4. `std/process` (Subprocess & IPC)
Spawns and controls child processes with standard stream piping:
```alya
struct ProcessOutput
    exit_code: int
    stdout: string
    stderr: string
end

function process.run(program: string, args: string[] = []) -> ProcessOutput
function process.spawn(program: string, args: string[] = []) -> ProcessHandle
```

#### 5. `std/io` (Streams & Standard I/O)
Provides standard stream handles and stream abstractions:
```alya
# Standard handles
const io.stdin: Reader
const io.stdout: Writer
const io.stderr: Writer

function io.copy(src: Reader, dst: Writer) -> int
function io.read_all(r: Reader) -> string
```

#### 6. `std/net` (Raw Sockets)
Low-level operating system TCP/UDP sockets (HTTP protocols belong in `pkg/http`):
```alya
function net.tcp_connect(host: string, port: int) -> Socket
function net.tcp_listen(host: string, port: int) -> ServerSocket
function net.tcp_accept(server: ServerSocket) -> Socket
function net.send(sock: Socket, data: string) -> int
function net.recv(sock: Socket, max_bytes: int) -> string
function net.close(sock: Socket)
function net.poll(sock: Socket, timeout_ms: int) -> bool
```

#### 7. `std/sync` (Threads & Concurrency Primitives)
Unifies native OS worker threads and synchronization structures:
```alya
# Native OS worker thread
function sync.spawn(func: || -> any) -> ThreadHandle
function sync.join(handle: ThreadHandle) -> any

# Synchronization structures
struct Mutex
    function Mutex.new() -> Mutex
    function Mutex.lock(self)
    function Mutex.unlock(self)
end

struct WaitGroup
    function WaitGroup.new() -> WaitGroup
    function WaitGroup.add(self, delta: int)
    function WaitGroup.done(self)
    function WaitGroup.wait(self)
end

struct Channel[T]
    function Channel[T].new(capacity: int = 0) -> Channel[T]
    function Channel[T].send(self, val: T)
    function Channel[T].recv(self) -> T
    function Channel[T].close(self)
end
```

#### 8. `std/time` (Clocks & Timers)
High-precision monotonic timestamps and sleep operations:
```alya
function time.now() -> int              # Epoch seconds
function time.now_millis() -> int       # Epoch milliseconds
function time.now_nanos() -> int        # Monotonic CPU nanoseconds
function time.sleep(millis: int)
```

#### 9. `std/mem` (Memory & Arenas)
Low-level memory blocks and high-speed Arena allocators:
```alya
struct Arena
    function Arena.new(initial_capacity: int = 65536) -> Arena
    function Arena.alloc(self, size: int) -> ptr
    function Arena.reset(self)
    function Arena.destroy(self)
end

function mem.copy(dst: ptr, src: ptr, bytes: int)
function mem.set(dst: ptr, byte_val: u8, count: int)
function mem.aligned_alloc(bytes: int, alignment: int = 32) -> ptr
function mem.aligned_free(p: ptr)
```

#### 10. `std/math` (Math, Randomness & SIMD Primitives)
Mathematical calculations, hardware-seeded pseudo-randomness, and SIMD-accelerated vector primitives:
```alya
const math.PI = 3.141592653589793
const math.E  = 2.718281828459045

function math.sqrt(x: float) -> float
function math.pow(base: float, exp: float) -> float
function math.sin(rad: float) -> float
function math.cos(rad: float) -> float
function math.floor(x: float) -> float
function math.ceil(x: float) -> float
function math.abs(x: float) -> float
function math.random() -> float                      # Monotonic 0.0 to 1.0 float
function math.rand_int(min: int, max: int) -> int   # Inclusive random integer

# SIMD-Accelerated Vector & AI Primitives
function math.dot_product(a: float[], b: float[]) -> float
function math.cosine_similarity(a: float[], b: float[]) -> float
function math.vector_norm(v: float[]) -> float
function math.sum_f64(arr: float[]) -> float
function math.lerp(a: float, b: float, t: float) -> float
```

#### 11. `std/str` (String Algorithms & Encodings)
Essential string utilities and reversible binary encodings:
```alya
function str.trim(s: string) -> string
function str.split(s: string, delimiter: string) -> string[]
function str.join(items: string[], delimiter: string) -> string
function str.replace(s: string, search: string, replacement: string) -> string
function str.to_upper(s: string) -> string
function str.to_lower(s: string) -> string
function str.base64_encode(s: string) -> string
function str.base64_decode(s: string) -> string
function str.hex_encode(s: string) -> string
function str.hex_decode(s: string) -> string
```

#### 12. `std/collections` (Data Structures)
Production-grade standard collection types:
```alya
struct Stack[T]
struct Queue[T]
struct Set[T]
struct PriorityQueue[T]
```

#### 13. `std/console` (Terminal Control & Styling)
Terminal encoding (UTF-8), title, clear, cursor, and ANSI color formatting:
```alya
function console.utf8() -> bool
function console.clear()
function console.title(t: string)
function console.beep()

# Styling & Colors
function console.color(fg_code: int, text: string) -> string
function console.bold(text: string) -> string
function console.red(text: string) -> string
function console.green(text: string) -> string
function console.yellow(text: string) -> string
function console.blue(text: string) -> string
```

#### 14. `std/test` (Verification & Benchmarking)
Unified test assertions, mock runners, and micro-benchmarking timers:
```alya
function test.assert(condition: bool, message: string = "Assertion failed")
function test.assert_eq[T](actual: T, expected: T, message: string = "")
function test.run_bench(name: string, warmup: int, iterations: int, task: || -> void)
```
