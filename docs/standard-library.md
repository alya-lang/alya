# Chapter 7: Standard Library Reference

[← Error Handling](error-handling.md) • [Wiki Home](README.md) • [Next: Architecture & Internals →](architecture-and-internals.md)

---

## 1. Overview

Alya comes with a comprehensive, built-in standard library bundled directly into the compiler (`stdlib/`). You can import any module using `import "std/<module>"`. If standard library files are not present on disk, the `alya` compiler automatically extracts them from its embedded binary fallback table.

---

## 2. Module Catalog

### 🔤 `std/str` — Advanced String Utilities
```alya
import "std/str"
```
* `starts_with(s, prefix)`: Returns `1` if string starts with prefix, else `0`.
* `ends_with(s, suffix)`: Returns `1` if string ends with suffix, else `0`.
* `replace(s, old, new)`: Replaces occurrences of `old` with `new`.
* `str_repeat(s, count)`: Repeats string `s` `count` times.
* `pad_left(s, width, pad_char)`: Pads string on the left to `width`.
* `pad_right(s, width, pad_char)`: Pads string on the right to `width`.
* `capitalize(s)`: Converts the first letter to uppercase.
* `lines(s)`: Splits string by newline `\n` into an array of lines.
* `count_matches(s, sub)`: Counts occurrences of substring `sub`.
* `is_empty(s)`: Returns `1` if string is empty or whitespace.

---

### 📐 `std/math` — Math, Statistics & PRNG
```alya
import "std/math"
```
* **Trigonometry**: `sin(x)`, `cos(x)`, `tan(x)`, `hypot(a, b)`
* **Rounding & Truncation**: `round(x)`, `floor(x)`, `ceil(x)`, `trunc(x)`
* **Pseudo-Random Number Generator**: `rand_seed(seed)`, `rand_range(min, max)`
* **Array Statistics**: `sum(arr)`, `mean(arr)`, `median(arr)`
* **Utilities**: `clamp(val, min, max)`, `sign(x)`, `is_even(n)`, `is_odd(n)`

---

### 📁 `std/fs` & `std/path` — Filesystem & Path Utilities
```alya
import "std/fs"
import "std/path"
```
* **`std/fs`**:
  * `fs_exists(path)`: Check if file exists (`1` or `0`).
  * `fs_read(path)`: Read entire file content into string.
  * `fs_write(path, content)`: Overwrite or create file with content.
  * `fs_append(path, content)`: Append content to file.
  * `fs_size(path)`: File size in bytes.
  * `fs_remove(path)`: Delete file from disk.
  * `fs_mkdir(path)`: Create single directory.
  * `fs_rmdir(path)`: Remove empty directory.
  * `is_dir(path)` / `fs_is_dir(path)`: Returns `1` if path is an existing directory, else `0`.
  * `ensure_dir(path)`: Recursively creates parent directories if needed.
  * `list_dir_recursive(path)`: Recursively lists all files and directories in a directory tree.
  * `copy_dir_recursive(src, dest)`: Recursively copies an entire directory tree.
  * `remove_dir_recursive(path)`: Recursively removes a directory and all its contents.
  * `copy_file(src, dest)`, `move_file(src, dest)`.
  * `read_lines(path)`: Read file into array of line strings.
  * `write_lines(path, lines)`, `append_line(path, line)`, `append_lines(path, lines)`.
  * `file_lines_count(path)`: Fast line count of a file.
  * `file_basename(path)`: Extract file name component (e.g. `"foo/bar.txt"` -> `"bar.txt"`).
  * `file_extension(path)`: Extract file extension (e.g. `"foo/bar.txt"` -> `"txt"`).
  * `file_parent(path)`: Extract parent directory path (e.g. `"foo/bar.txt"` -> `"foo"`).
* **`std/path`**:
  * `path_join(dir, file)`: Normalize and join path segments.
  * `file_name(path)`, `file_ext(path)`, `file_stem(path)`, `parent_dir(path)`.
  * `is_absolute(path)`, `path_separator()`.

---

### 🔐 `std/hash` — Hashing & Encodings
```alya
import "std/hash"
```
* `fnv1a(str)`: 32-bit FNV-1a non-cryptographic hash integer.
* `djb2(str)`: DJB2 string hash integer.
* `hex_encode(str)`, `hex_decode(hex)`
* `base64_encode(str)`, `base64_decode(b64)`

---

### 📦 `std/collections` — High-Level Data Structures
```alya
import "std/collections"
```
* **Stack**: `stack_new()`, `stack_push(st, val)`, `stack_pop(st)`, `stack_peek(st)`, `stack_size(st)`
* **Queue**: `queue_new()`, `queue_push(q, val)`, `queue_pop(q)`, `queue_peek(q)`, `queue_size(q)`
* **Set**: `set_new()`, `set_add(s, val)`, `set_has(s, val)`, `set_remove(s, val)`, `set_size(s)`

---

### 🧪 `std/test` — Micro-Testing Framework
```alya
import "std/test"
```
* `test_suite("Name")`: Initialize a named test suite.
* `assert(condition, "message")`: General assertion.
* `assert_eq(actual, expected, "message")`: Integer equality assertion.
* `assert_ne(actual, unexpected, "message")`: Integer inequality assertion.
* `assert_str_eq(actual, expected, "message")`: String equality assertion.
* `test_summary()`: Prints pass/fail summary report.

---

### 🧠 `std/mem` — Arena Allocator & Raw Memory
```alya
import "std/mem"
```
High-performance linear allocation arena with instant bulk deallocation:
* `arena_new(capacity)`: Allocate a linear arena buffer.
* `arena_alloc_mem(arena, size)`: Fast O(1) pointer-bump allocation.
* `arena_clear(arena)`: Instant reset of arena offset without per-object free calls.
* `arena_free_all(arena)`: Release entire arena back to OS.

---

### 🌐 `std/net` — Networking, UDP & HTTP/HTTPS Client
```alya
import "std/net"
```
Production-grade networking supporting TCP & UDP sockets, socket timeouts, remote peer inspection, and an HTTP/HTTPS client:
* **TCP Socket Networking**:
  * `tcp_socket()`: Create a new TCP stream socket (`AF_INET`, `SOCK_STREAM`).
  * `tcp_connect(host, port)`: Connect to remote host; returns socket handle or `-1`.
  * `tcp_send(sock, data)`: Send string data through socket.
  * `tcp_recv(sock, max_bytes)`: Receive up to `max_bytes` from socket (default 4096).
  * `tcp_close(sock)`: Close open socket.
  * `tcp_listen(port, backlog)`: Create listening server socket bound to port.
  * `tcp_accept(server_sock)`: Accept incoming client connection.
  * `tcp_set_timeout(sock, ms)`: Set read/write timeouts (`SO_RCVTIMEO` / `SO_SNDTIMEO`) in milliseconds.
  * `tcp_peer_ip(sock)`: Get remote peer's IP address (e.g. `"127.0.0.1"`).
  * `tcp_peer_port(sock)`: Get remote peer's port number.
  * `tcp_peer_addr(sock)`: Get remote peer address formatted as `"ip:port"`.
* **UDP Datagram Networking**:
  * `udp_socket()`: Create a UDP datagram socket (`AF_INET`, `SOCK_DGRAM`).
  * `udp_bind(sock, port)`: Bind UDP socket to specified local port on `INADDR_ANY`.
  * `udp_send(sock, host, port, data)`: Send datagram to destination host and port.
  * `udp_recv(sock, max_bytes)`: Receive datagram packet up to `max_bytes`.
  * `udp_close(sock)`: Close UDP socket.
  * `udp_set_timeout(sock, ms)`: Set read/write timeouts on UDP socket in milliseconds.
* **HTTP & HTTPS Client**:
  * `http_get(url)`: Perform HTTP/HTTPS GET request; returns `HttpResponse` struct.
  * `http_post(url, body, content_type)`: Perform HTTP/HTTPS POST request with content-type (defaults to `application/json`).
  * `http_put(url, body, content_type)`: Perform HTTP/HTTPS PUT request.
  * `http_patch(url, body, content_type)`: Perform HTTP/HTTPS PATCH request.
  * `http_delete(url)`: Perform HTTP/HTTPS DELETE request.
  * `http_head(url)`: Perform HTTP/HTTPS HEAD request.
  * `http_request(method, host, port, path, headers, body)`: Configurable HTTP/HTTPS request with header map and body.
  * `http_parse_response(raw)`: Parse raw HTTP response string into `HttpResponse(status_code, status_text, headers, body)`.
  * `http_status_text(code)`: Returns standard status phrase for status code (e.g. `200` -> `"OK"`, `404` -> `"Not Found"`).
  * `basic_auth(username, password)`: Generates `"Basic <base64>"` header string.
  * **Response Status Helpers**:
    * `http_is_success(res)`: Returns `1` if status code is `2xx`, else `0`.
    * `http_is_redirect(res)`: Returns `1` if status code is `3xx`, else `0`.
    * `http_is_client_error(res)`: Returns `1` if status code is `4xx`, else `0`.
    * `http_is_server_error(res)`: Returns `1` if status code is `5xx`, else `0`.
    * `http_is_error(res)`: Returns `1` if response code is an error (`>= 400` or `< 0`).
  * **Production Hardening**: Includes automatic 10-second socket timeout, `Content-Length`-aware response parsing to avoid socket-close delays, and transparent HTTPS bridge.

---

### 🖥️ `std/console` — Terminal Control, Code Pages & TUI
```alya
import "std/console"
```
Cross-platform terminal control, ANSI sequences, Windows code page management, and UI/TUI components:
* `console_utf8()`: Switch console to UTF-8 (CP 65001) and enable ANSI virtual terminal processing.
* `console_clear()`: Clear terminal screen buffer.
* `console_title(title)`: Set terminal window title.
* `console_beep()`: Trigger audible console bell alert.
* `console_cursor_to(row, col)`: Position cursor at specified row and column.
* `console_cursor_hide()`, `console_cursor_show()`: Toggle cursor visibility.
* `console_output_cp()`, `console_input_cp()`: Inspect active code page.
* **Console Input Helpers**:
  * `console_prompt(prompt_text)`: Prompts user and reads full line from stdin.
  * `console_confirm(prompt_text, default_yes)`: Yes/No confirmation prompt (returns `1` or `0`).
* *(For rich TUI components, dynamic tables, boxes, charts, and progress bars, use official `alya-lang/term`)*

---

### 🔍 `std/glob` — Wildcard Matching & Filesystem Globbing
```alya
import "std/glob"
```
Wildcard pattern matching supporting `*`, `**`, `?`, character classes `[a-z]`, and negation `[!0-9]`:
* `glob_match(pattern, path)`: Match pattern against paths (respects directory separators).
* `glob_match_simple(pattern, text)` / `fnmatch(pattern, text)`: Standard string pattern match.
* `glob_filter(pattern, list)`: Filter an array of strings by glob pattern.
* `glob(pattern)`: Glob files in current working directory.
* `glob_dir(pattern, dir_path)`: Glob files within a specific target directory.
* `glob_escape(str)`: Escape pattern special characters.

---

### 🎲 `std/rand` — Lightweight PRNG & Range Generators
```alya
import "std/rand"
```
Lightweight pseudo-random number generator for scripting, ranges, floats, and probability checks:
* `rand_auto_seed()`: Seed PRNG using system time and high-resolution timer.
* `rand_seed_state(s)`: Explicitly seed the global PRNG with integer `s`.
* `rand_next()`: Pseudo-random integer in `[0, 2147483647]`.
* `rand_int(min, max)`: Random integer in inclusive range `[min, max]`.
* `rand_float()`: Random float in range `[0.0, 1.0)`.
* `rand_float_range(min, max)`: Random float in range `[min, max)`.
* `rand_bool()`: Boolean (1 or 0) with 50% probability.
* `rand_chance(pct)`: Returns 1 if event with probability `pct` (0..100) occurs.
* `rand_choice(arr)`: Randomly selected element from `arr`.

---

### 📦 Standalone Ecosystem Packages: `csv`, `url`, `http`, `crypto`, `uuid`, `term`, `logger`

Specialized domain libraries are maintained as official standalone packages rather than bundled compiler bloat. Install them into your project with `alya add`:

* **`uuid`**: RFC 4122 UUID v4, RFC 9562 UUID v7, ULID, and NanoID generator (`alya add uuid`). See [alya-lang/uuid](https://github.com/alya-lang/uuid).
* **`csv`**: RFC 4180 compliant CSV & TSV parser, serializer, and file I/O (`alya add csv`). See [alya-lang/csv](https://github.com/alya-lang/csv).
* **`url`**: WHATWG and RFC 3986 compliant URL and `UrlSearchParams` parser and builder (`alya add url`). See [alya-lang/url](https://github.com/alya-lang/url).
* **`http`**: HTTP Client & Server framework with middleware and routing (`alya add http`). See [alya-lang/http](https://github.com/alya-lang/http).
* **`crypto`**: AES, ChaCha20, SHA, HMAC, PBKDF2, Base64/Base64URL, and Constant-Time Equality (`alya add crypto`). See [alya-lang/crypto](https://github.com/alya-lang/crypto).
* **`term`**: Rich TUI toolkit with boxes, tables, charts, trees, and progress bars (`alya add term`). See [alya-lang/term](https://github.com/alya-lang/term).
* **`logger`**: Structured multi-target logging with file rotation, JSON formatting, and async sinks (`alya add logger`). See [alya-lang/logger](https://github.com/alya-lang/logger).

---

### 🎨 `std/color` — Terminal Styling & TrueColor
```alya
import "std/color"
```
ANSI escape codes, foreground/background colors, styles, and 24-bit TrueColor:
* **Foreground**: `color_red(s)`, `color_green(s)`, `color_blue(s)`, `color_yellow(s)`, `color_cyan(s)`, `color_magenta(s)`, `color_white(s)`, `color_gray(s)`.
* **Background**: `bg_red(s)`, `bg_green(s)`, `bg_blue(s)`, `bg_yellow(s)`.
* **Styles**: `style_bold(s)`, `style_dim(s)`, `style_italic(s)`, `style_underline(s)`.
* **TrueColor (RGB)**: `color_rgb(s, r, g, b)`, `bg_rgb(s, r, g, b)`.
* `strip_ansi(s)`: Strip ANSI escape sequences from styled text.

---

### 📝 `std/log` — Leveled Logging
```alya
import "std/log"
```
Structured leveled logging with colorized badges and timestamps:
* `log_debug(msg)`, `log_info(msg)`, `log_warn(msg)`, `log_error(msg)`, `log_fatal(msg)`
* `logger_new(name, level)`: Create isolated logger instance with configurable output.

---

### 💻 `std/os` & `std/time` — System & Timers
```alya
import "std/os"
import "std/time"
```
* **`std/os`**:
  * `os_name()`: Target OS (`"windows"`, `"linux"`, `"macos"`).
  * `arch_name()`: Target architecture (`"x64"`, `"arm64"`, `"x86"`).
  * `get_pid()` / `pid()`: Current process OS Process ID integer.
  * `get_cwd()` / `cwd()`: Current working directory path string.
  * `set_cwd(path)` / `chdir(path)`: Change current working directory.
  * `env(key)`: Retrieve environment variable value.
  * `env_or(key, default)`: Retrieve environment variable with fallback.
  * `os_exit(code)`: Terminate process with exit code.
* **`std/time`**:
  * `time()` / `now()`: Current Unix timestamp in seconds.
  * `clock_ms()` / `now_ms()`: High-resolution millisecond timer.
  * `sleep_ms(ms)` / `delay(ms)`: Suspend thread execution in milliseconds.
  * `epoch_to_date(epoch_sec)`: Converts epoch seconds to date map (`"year"`, `"month"`, `"day"`, `"hour"`, `"minute"`, `"second"`, `"weekday"`).
  * `date_ymd_to_epoch(year, month, day)`: Converts calendar Y-M-D to epoch seconds UTC.
  * `date_to_epoch(date_map)`: Converts date map to epoch seconds.
  * `format_iso(epoch_sec)`: Formats ISO 8601 string (`"1970-01-01T00:00:00Z"`).
  * `format_date(epoch_sec)`: Formats date string (`"YYYY-MM-DD"`).
  * `format_time_hhmmss(epoch_sec)`: Formats time string (`"HH:MM:SS"`).
  * `format_date_map(map)`: Formats date map into ISO 8601 string.
  * `month_name(m)`, `month_short_name(m)`: English month names.
  * `weekday_name(w)`, `weekday_short_name(w)`: English weekday names.
  * `iso_now()`, `date_now()`, `time_now()`: Current date/time formatted strings.

---

## 3. Progressive Examples

### Level 1: Pure & Minimal (String Formatting & Math)
```alya
import "std/str"
import "std/math"

let text = "alya"
say capitalize(text)               # "Alya"
say pad_left("42", 5, "0")          # "00042"

let angle = 0.0
say "cos(0) = " + round(cos(angle)) # 1
```

---

### Level 2: Practical & Idiomatic (Automated Test Suite)
Creating a robust unit test suite validating business rules:

```alya
import "std/test"
import "std/math"
import "std/str"

test_suite("Core Math & String Assertions")

# Math tests
assert_eq(clamp(15, 0, 10), 10, "Upper clamp bound")
assert_eq(clamp(-5, 0, 10), 0,  "Lower clamp bound")
assert_eq(sum([1, 2, 3, 4, 5]), 15, "Array sum")

# String tests
let greeting = "Hello, World!"
assert_eq(starts_with(greeting, "Hello"), 1, "starts_with")
assert_eq(ends_with(greeting, "World!"), 1, "ends_with")
assert_str_eq(replace("foo-bar-foo", "foo", "baz"), "baz-bar-baz", "string replace")

test_summary()
```

---

### Level 3: Advanced & Real-World (JSON Snapshot & Checksum Storage Pipeline)
A production-like data persistence system that serializes an entity to JSON, computes an FNV-1a checksum, writes it to a file path, and verifies integrity:

```alya
import "std/fs"
import "std/path"
import "std/hash"
import "std/json"

function save_entity_snapshot(dir, filename, id, name, score)
    # Ensure directory path is normalized
    let full_path = path_join(dir, filename)

    # Construct JSON payload
    let json_id    = json_number("id", id)
    let json_name  = json_string("name", name)
    let json_score = json_number("score", score)

    let payload = json_object([json_id, json_name, json_score])

    # Calculate checksum for data integrity
    let checksum = fnv1a(payload)
    say "[SNAPSHOT] Generated payload: {payload}"
    say "[SNAPSHOT] FNV-1a Checksum: {checksum}"

    # Write snapshot to disk
    fs_write(full_path, payload)

    # Verify write
    if fs_exists(full_path) == 1
        let read_back = fs_read(full_path)
        let verify_checksum = fnv1a(read_back)

        if checksum == verify_checksum
            say "[SUCCESS] File saved and verified at: {full_path}"
            return 1
        else
            say "[ERROR] Checksum mismatch during verification!"
            return 0
        end
    else
        say "[ERROR] Failed to create snapshot file!"
        return 0
    end
end

# Run the snapshot backup pipeline
save_entity_snapshot(".", "player_state.json", 101, "AlyaDev", 9850)

# Clean up scratch test file
if fs_exists("player_state.json")
    fs_remove("player_state.json")
    say "[CLEANUP] Scratch snapshot removed."
end
```

---

### Level 4: Real-World Systems & Observability with UUID v7 & Terminal Styling

A real-world example demonstrating `std/rand` (RFC 9562 UUID v7), `std/color` (ANSI terminal badges), and structured event streaming:

```alya
import "std/rand"
import "std/color"
import "std/time"

function emit_audit_log(service_name, action, details)
    # Generate time-ordered RFC 9562 UUID v7
    let event_id = uuid_v7()

    let badge = color_green("[AUDIT]")
    let service_tag = color_cyan("[" + service_name + "]")
    let id_str = color_yellow(event_id)

    say "{badge} {service_tag} ID:{id_str} Action:{action} -> {details}"
    return event_id
end

emit_audit_log("AUTH", "USER_LOGIN", "user=alice ip=192.168.1.100")
emit_audit_log("BILLING", "INVOICE_GENERATED", "amount=$250.00 currency=USD")
emit_audit_log("STORAGE", "SNAPSHOT_STORED", "target=s3://alya-backups/daily.tar.gz")
```

---

## 8. Threading (`std/thread`) & Synchronization (`std/sync`)

### 🧵 `std/thread` — OS Thread Management
Cross-platform OS thread creation and execution lifecycle (Win32 threads and POSIX `pthread`).

```alya
import "std/thread"

function worker(param)
    let id = thread_id()
    say "Worker running on OS thread {id} with param: {param}"
    return param * 2
end

# 1. Spawn worker thread
let handle = thread_spawn(worker, 21)

# 2. Join thread and retrieve 64-bit return value
let result = thread_join(handle)
say "Result from thread: {result}"    # 42
```

---

### 🔒 `std/sync` — Synchronization Primitives
Primitives for thread safety, mutual exclusion, queues, and concurrency control:

```alya
import "std/sync"

# 1. Mutex (Mutual Exclusion Lock)
let lock = mutex_new()
mutex_lock(lock)
# ... critical section ...
mutex_unlock(lock)
mutex_free(lock)

# 2. Thread-Safe Channel (Producer-Consumer Queue)
let ch = channel_new()
channel_send(ch, 100)
channel_send(ch, 200)
let v1 = channel_recv(ch, 1000)       # 100
let v2 = channel_try_recv(ch)         # 200
channel_close(ch)
channel_free(ch)

# 3. WaitGroup Synchronization
let wg = wait_group_new()
wait_group_add(wg, 2)
# Background workers call wait_group_done(wg)
wait_group_done(wg)
wait_group_done(wg)
let completed = wait_group_wait(wg, 5000) # 1 if counter reached 0
wait_group_free(wg)

# 4. Once (Single Initialization Guard)
let o = once_new()
if once_check(o) == 1
    say "Initialized exactly once across threads"
end
once_free(o)

# 5. RwLock (Reader-Writer Lock)
let rw = rwlock_new()
rwlock_read_lock(rw)
# ... concurrent reads ...
rwlock_read_unlock(rw)

rwlock_write_lock(rw)
# ... exclusive write ...
rwlock_write_unlock(rw)
rwlock_free(rw)
```
