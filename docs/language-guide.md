# Alya Language Guide & Tour

> 💡 **Looking for the modular, progressive guide?** Check out the **[Alya Documentation Wiki](README.md)** featuring 8 chapters from basic to advanced.

A comprehensive single-page reference for the syntax, features, and standard library of the **Alya** programming language.

---

## Table of Contents

1. [Hello World & Comments](#1-hello-world--comments)
2. [Variables, Constants & Operators](#2-variables-constants--operators)
3. [String Interpolation & Built-ins](#3-string-interpolation--built-ins)
4. [Interactive User Input](#4-interactive-user-input)
5. [Control Flow](#5-control-flow)
6. [Functions & Gradual Typing](#6-functions--gradual-typing)
7. [Enumerations (`enum`)](#7-enumerations-enum)
8. [Pattern Matching (`when`)](#8-pattern-matching-when)
9. [Exception Handling (`try ... catch`)](#9-exception-handling-try--catch)
10. [Arrays & Dynamic Methods](#10-arrays--dynamic-methods)
11. [Modules & Standard Library](#11-modules--standard-library)
12. [Floating-Point Numbers](#12-floating-point-numbers)
13. [Structs & Default Values](#13-structs--default-values)
14. [Command-Line Arguments (`args()`)](#14-command-line-arguments-args)
15. [Hash Maps & Dictionaries (`map()`)](#15-hash-maps--dictionaries-map)
16. [File I/O](#16-file-io)
17. [Character & String Utilities](#17-character--string-utilities)
18. [Self-Hosting Prototype](#18-self-hosting-prototype-compiler-in-alya)
19. [Benchmarking & Profiling](#19-benchmarking--performance-profiling)
20. [Developer CLI & Tooling (`fmt`, `test`)](#20-developer-cli--tooling-fmt-test)

---

### 1. Hello World & Comments

Alya supports Python-style `#`, C-style `//`, and multiline `/* ... */` comments.

```alya
# Single-line hash comment
// Single-line slash comment
/*
   Multi-line block comment
*/
say "Hello, World!"
```

---

### 2. Variables, Constants & Operators

#### Variables (`let`)
Variables are declared with `let`. Variable types (integers, floats, strings, arrays, maps, structs, null) are inferred automatically:

```alya
let name = "Alya"
let age = 1
let pi = 3.14159
let empty = null      # Null literal
```

#### Compile-Time Constants (`const`)
Constants are declared with `const` and are evaluated at compile time with constant folding. Constants can be defined at global or local scope, support comma-separated declarations, and cannot be reassigned or shadowed by `let`:

```alya
const APP_NAME = "Alya Server"
const VERSION = "0.0.18"
const PI = 3.14159
const MAX_CONNECTIONS = 500, TIMEOUT_SEC = 30
const BUFFER_SIZE = 1024 * 64   # Folded to 65536 at compile time

function get_limit()
    const LOCAL_LIMIT = 100
    return LOCAL_LIMIT * 2
end
```

#### Variable Swap & Multi-Assignment
Alya natively supports atomic variable swap and multiple assignment expressions without needing explicit temporary variables:

```alya
# Atomic variable swap
let a = 10
let b = 20
a, b = b, a           # a = 20, b = 10

# String swap
let first = "World"
let second = "Hello"
first, second = second, first   # first = "Hello", second = "World"

# Multi-variable assignment
let x = 0
let y = 0
let z = 0
x, y, z = 100, 200, 300

# 3-way circular rotation
x, y, z = z, x, y     # x = 300, y = 100, z = 200

# Multi-variable let declaration
let p = 1, q = 2
```

#### Arithmetic & Compound Operators
```alya
let a = 20
let b = 10
say a + b    # 30
say a - b    # 10
say a * b    # 200
say a / b    # 2

# Compound assignments
a += 5
say a        # 25
```

#### Bitwise Operators & Compound Assignments
```alya
let flags = 0b00001100
let mask  = 0b00001010
say flags & mask      # 8 (AND)
say flags | mask      # 14 (OR)
say flags ^ mask      # 6 (XOR)
say 1 << 4            # 16 (Shift left)
say 32 >> 2           # 8 (Shift right)

flags &= mask         # In-place bitwise compound assignment
```

#### Null Coalescing Operator (`??`)
```alya
let custom_port = null
let port = custom_port ?? 8080
say port              # 8080
```

---

### 3. String Interpolation & Built-ins

Expressions wrapped in `{}` inside string literals are automatically evaluated and interpolated:

```alya
let user = "Alice"
let score = 95
say "Player {user} scored {score} points!"

# Built-in helper functions
say abs(-42)            # 42
say max(10, 25)         # 25
say min(10, 25)         # 10
say sqrt(16)            # 4
say pow(2, 8)           # 256
say len("Hello Alya")   # 10

# String helpers (function or method call syntax)
let greeting = "   Hello, Alya!   "
say greeting.trim()                 # "Hello, Alya!"
say greeting.trim().upper()         # "HELLO, ALYA!"
say greeting.trim().lower()         # "hello, alya!"
say greeting.contains("Alya")       # 1
say greeting.trim().substring(0, 5) # "Hello"

# String splitting & joining
let csv = "apple,banana,cherry"
let fruits = csv.split(",")         # ["apple", "banana", "cherry"]
say fruits.join(" - ")              # "apple - banana - cherry"

# Explicit & automatic string conversions
let count = 42
say "Total items: " + count          # Auto-converts number to string: "Total items: 42"
say 100 + " percent completed"      # Auto-converts: "100 percent completed"
say str(count)                      # Explicit string conversion: "42"

# String-to-number parsing
say int("123")                      # Converts string to integer: 123
say float("3.14159")                # Converts string to float: 3.14159
say to_int("456")                   # Alias: 456
say to_float("99.9")                # Alias: 99.9

# Multiline and Raw strings
let multiline = """
Line 1
Line 2
"""
let raw = `C:\path\without\escapes`
```

---

### 4. Interactive User Input

Use `ask` to prompt the user for console input:

```alya
let username = ask "Enter your name: "
say "Welcome, " + username + "!"
```

---

### 5. Control Flow

Alya uses clean indentation and `end` delimiters for block termination:

```alya
# Conditionals
let grade = 85

if grade >= 90
    say "Grade: A"
elif grade >= 75
    say "Grade: B"
else
    say "Grade: C"
end

# Ternary operator & Inline if-expression
let status = grade >= 50 ? "Pass" : "Fail"
let tag = if grade >= 80 then "Distinction" else "Standard"

# While Loop
let counter = 0
while counter < 3
    say counter
    counter += 1
end

# For Loop (Range)
for i in 1..5
    say i
end

# For-each Loop (Array Iteration)
let items = ["apple", "banana", "cherry"]
for item in items
    say item
end

# Repeat Loop
let loops = 0
repeat
    loops += 1
    if loops >= 3
        break
    end
end

# Loop Control: break & continue
for i in 1..5
    if i == 2
        continue    # Skip iteration
    end
    if i == 4
        break       # Exit loop early
    end
    say i           # 1, 3
end
```

---

### 6. Functions & Gradual Typing

Functions are first-class, support default parameters, and return values using `return`:

```alya
function add(x, y)
    return x + y
end

# Default parameter values
function greet(name, greeting = "Hello", punctuation = "!")
    say "{greeting}, {name}{punctuation}"
end

greet("World")                     # "Hello, World!"
greet("Alice", "Welcome")          # "Welcome, Alice!"
greet("Bob", "Good morning", ".")  # "Good morning, Bob."
```

#### Gradual Typing (Optional Type Annotations)
Alya supports gradual typing: you can optionally annotate parameter and return types for clearer contracts and compiler type propagation. Supported types include `int`, `float`, `str`, and array types like `int[]`, `str[]`, or `float[]`:

```alya
# Fully typed function
function multiply(a: int, b: int) -> int
    return a * b
end

function calculate_area(radius: float) -> float
    const PI = 3.14159
    return PI * radius * radius
end

# Default parameters with type annotations
function format_greeting(name: str, prefix: str = "Welcome") -> str
    return prefix + ", " + name + "!"
end

# Mixed typed and untyped parameters (gradual)
function summarize(label: str, count, is_valid: int) -> str
    return label + ": " + str(count)
end

# Array type annotations
function array_len(items: int[]) -> int
    return len(items)
end
```

---

### 7. Enumerations (`enum`)

Alya provides first-class `enum` definitions. Enums can have auto-incrementing integer values (starting at `0`), explicit integer values, or string variant values. Variants can be accessed using either dot syntax (`Status.Active`) or scope resolution syntax (`Status::Active`):

```alya
# Auto-incrementing integer enum (0, 1, 2, 3)
enum TaskStatus
    Pending
    InProgress
    Completed
    Failed
end

# Explicit numeric values
enum HttpStatus
    Ok = 200
    Created = 201
    BadRequest = 400
    NotFound = 404
end

# String variant enum
enum LogLevel
    Debug = "DEBUG"
    Info = "INFO"
    Warn = "WARN"
    Error = "ERROR"
end

# Access via dot notation or scope resolution
let current = TaskStatus.InProgress
let code = HttpStatus::Ok
let level = LogLevel.Info

say TaskStatus.Pending       # 0
say HttpStatus::NotFound     # 404
say LogLevel.Info            # "INFO"
```

---

### 8. Pattern Matching (`when`)

A concise switch/match construct that works with numbers, strings, and enums:

```alya
let current_status = TaskStatus.InProgress

when current_status
    is TaskStatus.Pending then say "Status: Pending"
    is TaskStatus.InProgress then say "Status: In Progress"
    is TaskStatus.Completed then say "Status: Completed"
    else say "Status: Unknown"
end

# Pattern matching with values
let status_code = 200
when status_code
    is 200 then say "OK"
    is 404 then say "Not Found"
    else say "Other Code"
end
```

---

### 9. Exception Handling (`try ... catch`)

Structured error catching with built-in runtime protection for division by zero and index out of bounds:

```alya
try
    let dangerous = 10 / 0
    say "This will not run"
catch err
    say "Caught error: " + err
end

say "Program resumes normally!"
```

---

### 10. Arrays & Dynamic Methods

Dynamic arrays support literals, 0-based indexing, fast bounds safety, and methods:

```alya
# Array declaration and empty arrays
let numbers = [10, 20, 30, 40]
let empty = []
say numbers           # [10, 20, 30, 40]

# Length of an array (built-in function or method call)
say len(numbers)      # 4
say numbers.len()     # 4

# Dynamic methods: push and pop (method syntax or UFCS)
numbers.push(50)
push(numbers, 60)
say numbers           # [10, 20, 30, 40, 50, 60]

let last = numbers.pop()
say last              # 60
say numbers           # [10, 20, 30, 40, 50]

# Indexing (0-based read)
say numbers[0]        # 10
say numbers[1]        # 20

# Index assignment (write and compound operators)
numbers[2] = 99
numbers[0] += 5
say numbers           # [15, 20, 99, 40, 50]

# Safe bounds check
try
    say numbers[10]
catch err
    say "Caught error: " + err    # Caught error: index out of bounds
end
```

---

### 11. Modules & Standard Library

Split codebases across files and import functions with `import`:

```alya
# modules/math_utils.alya
function add(a, b)
    return a + b
end

function multiply(a, b)
    return a * b
end
```

```alya
# main.alya
import "modules/math_utils.alya"

let total = add(10, 20)
let product = multiply(total, 2)
say product    # 60
```

#### Pre-bundled Standard Library (`std/*`)

| Module | Description | Key Functions |
|---|---|---|
| `std/str` | Advanced string manipulation | `starts_with`, `ends_with`, `replace`, `str_repeat`, `pad_left`, `pad_right`, `capitalize`, `lines`, `count_matches`, `is_empty` |
| `std/path` | Cross-platform path handling | `path_join`, `file_name`, `file_ext`, `file_stem`, `parent_dir`, `is_absolute`, `path_separator` |
| `std/fs` | File system operations | `fs_exists`, `fs_read`, `fs_write`, `fs_append`, `fs_size`, `fs_mkdir`, `fs_remove`, `copy_file`, `move_file` |
| `std/math` | Trigonometry, stats & PRNG | `sin`, `cos`, `tan`, `hypot`, `round`, `floor`, `ceil`, `trunc`, `rand_range`, `rand_seed`, `sum`, `mean`, `median`, `clamp`, `sign`, `is_even`, `is_odd` |
| `std/net` | TCP & UDP socket networking | `tcp_connect`, `tcp_send`, `tcp_recv`, `tcp_close`, `tcp_listen`, `tcp_accept`, `udp_socket`, `udp_bind`, `udp_send`, `udp_recv` |
| `std/console` | Terminal control & code pages | `console_utf8`, `console_clear`, `console_title`, `console_beep`, `console_cursor_to`, `console_cursor_hide`, `console_cursor_show` |
| `std/glob` | Wildcard matching & globbing | `glob_match`, `glob_match_simple`, `glob_filter`, `glob`, `glob_dir`, `glob_escape` |
| `std/rand` | Lightweight PRNG & range generators | `rand_auto_seed`, `rand_seed_state`, `rand_next`, `rand_int`, `rand_float`, `rand_float_range`, `rand_bool`, `rand_chance`, `rand_choice` |
| `std/color` | ANSI colors & TrueColor RGB | `color_red`, `color_green`, `color_rgb`, `bg_rgb`, `style_bold`, `strip_ansi` |
| `std/log` | Leveled logging & formatting | `log_debug`, `log_info`, `log_warn`, `log_error`, `log_fatal`, `logger_new` |
| `std/hash` | Hashing & binary encoding | `djb2`, `fnv1a`, `hex_encode`, `hex_decode`, `base64_encode`, `base64_decode` |
| `std/collections` | High-level data structures | `Stack` (`stack_new`, `stack_push`, `stack_pop`), `Queue` (`queue_new`, `queue_push`), `Set` (`set_new`, `set_add`, `set_has`, `set_remove`) |
| `std/test` | Micro-testing framework | `test_suite`, `assert`, `assert_eq`, `assert_str_eq`, `test_summary` |
| `std/json` | JSON serialization | `json_number`, `json_string`, `json_bool`, `json_array`, `json_object`, `json_map`, `json_parse` |
| `std/time` | System clock & timers | `time`, `clock_ms` (monotonic millisecond timer), `sleep_ms` |
| `std/os` | Operating system interop | `os_name`, `arch_name`, `env`, `env_or`, `os_exit`, `exec` |
| `std/mem` | Low-level & arena allocator | `arena_new`, `arena_alloc_mem`, `arena_clear`, `alloc_mem`, `free_mem`, `peek_byte`, `poke_byte` |

> 📦 **Looking for `uuid`, `csv`, `url`, `http`, `crypto`, `term`, or `logger`?** These rich domain libraries are maintained as official standalone packages (`alyac add uuid`, `alyac add csv`, `alyac add url`, `alyac add http`, `alyac add crypto`, `alyac add term`, `alyac add logger`).

---

### 12. Floating-Point Numbers

First-class 64-bit IEEE 754 float support with hardware register acceleration (`xmm`/`d`):

```alya
let pi = 3.14159
let radius = 2.5
let area = pi * radius * radius
say "Area: {area}"

# Mixed integer-float arithmetic and conversions
let count = 4
let average = (10.0 + 20.0 + 30.5 + 40.5) / float(count)
say "Average: {average}"    # 25.25
say int(average)             # Float to integer: 25
say float("19.95")           # String to float: 19.95
say int("50")                # String to integer: 50
```

---

### 13. Structs & Default Values

Custom composite data types with field defaults, partial initialization, and both named and positional constructors:

```alya
# Struct with default field values
struct ServerConfig
    host = "127.0.0.1"
    port = 8080
    ssl = false
    max_clients = 1000
end

# Mixed required and default fields
struct User
    id
    name = "Anonymous"
    role = "member"
end

# 1. Full default initialization
let default_srv = ServerConfig {}
say default_srv.host        # "127.0.0.1"
say default_srv.port        # 8080

# 2. Partial override with named fields
let custom_srv = ServerConfig { port: 9000, ssl: true }
say custom_srv.host         # "127.0.0.1" (default preserved)
say custom_srv.port         # 9000 (overridden)
say custom_srv.ssl          # 1

# 3. Positional constructor with omitted trailing defaults
let pos_srv = ServerConfig("0.0.0.0", 3000)
say pos_srv.host            # "0.0.0.0"
say pos_srv.port            # 3000
say pos_srv.ssl             # 0 (default used)

# 4. Instantiation with mixed fields
let guest = User { id: 101 }
let admin = User { id: 102, name: "Alice", role: "admin" }
say "{guest.name} ({guest.role})"   # "Anonymous (member)"
say "{admin.name} ({admin.role})"   # "Alice (admin)"

# Field mutation & compound assignment
guest.name = "Bob"
guest.id += 1
```

---

### 14. Command-Line Arguments (`args()`)

```alya
let arguments = args()
say "Arguments count: {arguments.len()}"

for arg in arguments
    say "Argument: {arg}"
end
```

Run with arguments:
```bash
alyac run script.alya -- hello world 42
```

---

### 15. Hash Maps & Dictionaries (`map()`)

Associative key-value mappings:

```alya
let user = map()
user["name"] = "Alice"
user["role"] = "Admin"
user.set("level", 10)

say user["name"]            # Alice
say user.get("role")        # Admin
say user.contains("level")  # 1
say user.len()              # 3

# Keys and iteration
for key in user.keys()
    say "{key}: {user[key]}"
end

# Removing a key
user.remove("role")
say user                    # { "name": Alice, "level": 10 }
```

---

### 16. File I/O

Built-in native file system routines:

```alya
let filename = "output.txt"

# Writing and reading text files
write_file(filename, "Hello from Alya!")

if file_exists(filename)
    let content = read_file(filename)
    say "File Content: {content}"
end

# Deleting a file
delete_file(filename)
say file_exists(filename)   # 0
```

---

### 17. Character & String Utilities

```alya
let text = "Alya 2026"

# Character indexing
say text[0]                 # A
say char_at(text, 1)        # l

# ASCII conversions
let code = ord("A")         # 65
let ch = chr(66)            # B
say "{code} -> {ch}"

# Character classification
say is_alpha("A")           # 1
say is_digit("9")           # 1
say is_alnum("Z")           # 1
say is_space(" ")           # 1
```

---

### 18. Self-Hosting Prototype (Compiler in Alya)

Check out `examples/mini_compiler.alya` for a working compiler prototype written in Alya that compiles a subset of the language to native assembly:

```bash
# Compile and run the mini-compiler
alyac run examples/mini_compiler.alya

# Link the generated assembly
gcc mini_output.s -o mini_program.exe
./mini_program.exe
```

---

### 19. Benchmarking & Performance Profiling

Alya features built-in micro-benchmarking (`std/bench`) and stage profiling (`--time`):

```alya
import "std/bench"
import "std/math"

let runner = bench_runner("Alya Micro-Benchmarks")

bench_start(runner, 100000)
let i = 0
while i < 100000
    sin(0.5)
    i += 1
end
bench_stop(runner, "sin(0.5) calculation")

bench_summary(runner)
```

---

### 20. Developer CLI & Tooling (`fmt`, `test`)

The `alyac` compiler includes developer utilities directly out of the box:

```bash
# In-place code formatting
alyac fmt main.alya

# Format entire codebase
alyac fmt .

# CI dry-run verification
alyac fmt . --check

# Test suite discovery and execution
alyac test
```
