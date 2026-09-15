# Alya Language Guide & Tour

> 💡 **Looking for the modular, progressive guide?** Check out the **[Alya Documentation Wiki](README.md)** featuring 8 chapters from basic to advanced.

A comprehensive single-page reference for the syntax, features, and standard library of the **Alya** programming language.

---

## Table of Contents

1. [Hello World & Comments](#1-hello-world--comments)
2. [Variables & Arithmetic](#2-variables--arithmetic)
3. [String Interpolation & Built-ins](#3-string-interpolation--built-ins)
4. [Interactive User Input](#4-interactive-user-input)
5. [Control Flow](#5-control-flow)
6. [Functions](#6-functions)
7. [Pattern Matching (`when`)](#7-pattern-matching-when)
8. [Exception Handling (`try ... catch`)](#8-exception-handling-try--catch)
9. [Arrays & Dynamic Methods](#9-arrays--dynamic-methods)
10. [Modules & Standard Library](#10-modules--standard-library)
11. [Floating-Point Numbers](#11-floating-point-numbers)
12. [Structs & Custom Types](#12-structs--custom-types)
13. [Command-Line Arguments (`args()`)](#13-command-line-arguments-args)
14. [Hash Maps & Dictionaries (`map()`)](#14-hash-maps--dictionaries-map)
15. [File I/O](#15-file-io)
16. [Character & String Utilities](#16-character--string-utilities)
17. [Self-Hosting Prototype](#17-self-hosting-prototype-compiler-in-alya)
18. [Benchmarking & Profiling](#18-benchmarking--performance-profiling)
19. [Developer CLI & Tooling (`fmt`, `test`)](#19-developer-cli--tooling-fmt-test)

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

### 2. Variables, Types & Operators

Variables are declared with `let`. Variable types (integers, floats, strings, arrays, maps, structs, null) are inferred automatically.

```alya
let name = "Alya"
let age = 1
let pi = 3.14159
let empty = null      # Null literal

# Arithmetic
let a = 20
let b = 10
say a + b    # 30
say a - b    # 10
say a * b    # 200
say a / b    # 2

# Compound assignments
a += 5
say a        # 25

# Bitwise Operators & Compound Assignments
let flags = 0b00001100
let mask  = 0b00001010
say flags & mask      # 8 (AND)
say flags | mask      # 14 (OR)
say flags ^ mask      # 6 (XOR)
say 1 << 4            # 16 (Shift left)
say 32 >> 2           # 8 (Shift right)

flags &= mask         # In-place bitwise compound assignment

# Null Coalescing Operator (??)
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

### 6. Functions

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

let result = add(15, 30)
say result    # 45
```

---

### 7. Pattern Matching (`when`)

A concise switch/match construct:

```alya
let status_code = 2

when status_code
    is 1 then say "Status: Pending"
    is 2 then say "Status: Active"
    else say "Status: Unknown"
end
```

---

### 8. Exception Handling (`try ... catch`)

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

### 9. Arrays & Dynamic Methods

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

### 10. Modules & Standard Library

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
| `std/time` | System clock & timers | `time`, `clock_ms`, `sleep_ms` |
| `std/os` | Operating system interop | `os_name`, `arch_name`, `env`, `env_or`, `os_exit`, `exec` |
| `std/mem` | Low-level & arena allocator | `arena_new`, `arena_alloc_mem`, `arena_clear`, `alloc_mem`, `free_mem`, `peek_byte`, `poke_byte` |

> 📦 **Looking for `uuid`, `csv`, `url`, `http`, `crypto`, `term`, or `logger`?** These rich domain libraries are maintained as official standalone packages (`alyac add uuid`, `alyac add csv`, `alyac add url`, `alyac add http`, `alyac add crypto`, `alyac add term`, `alyac add logger`).

---

### 11. Floating-Point Numbers

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

### 12. Structs & Custom Types

Custom composite data types with named and positional constructors:

```alya
struct Point
    x
    y
end

# Named instantiation & field access
let p1 = Point { x: 10, y: 20 }
say p1                      # Point { x: 10, y: 20 }
say "Coords: ({p1.x}, {p1.y})"

# Field mutation & compound assignment
p1.x = 100
p1.y += 5
say p1                      # Point { x: 100, y: 25 }

# Positional constructor & function support
function distance_squared(pt)
    return pt.x * pt.x + pt.y * pt.y
end

let p2 = Point(3, 4)
say distance_squared(p2)    # 25
```

---

### 13. Command-Line Arguments (`args()`)

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

### 14. Hash Maps & Dictionaries (`map()`)

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

### 15. File I/O

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

### 16. Character & String Utilities

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

### 17. Self-Hosting Prototype (Compiler in Alya)

Check out `examples/mini_compiler.alya` for a working compiler prototype written in Alya that compiles a subset of the language to native assembly:

```bash
# Compile and run the mini-compiler
alyac run examples/mini_compiler.alya

# Link the generated assembly
gcc mini_output.s -o mini_program.exe
./mini_program.exe
```

---

### 18. Benchmarking & Performance Profiling

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

### 19. Developer CLI & Tooling (`fmt`, `test`)

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

