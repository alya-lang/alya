# Chapter 02: Types & Type System

## 1. Specification Rules

### 1.1 Primitive Types

#### 1. Boolean (`bool`)
- **Type**: `bool`
- **Literals**: `true` and `false`.
- **Strict Type Safety**: Integers `0` and `1` are **not** implicitly converted to `bool`. Expressions in conditional statements (`if`, `while`, `when`) must evaluate strictly to boolean values.

#### 2. Default Integer (`int`)
- **Canonical 64-bit Integer**: `int` is the default integer type throughout Alya.
- On modern 64-bit architectures (x64, ARM64), `int` is directly mapped to a 64-bit signed two's complement integer (`i64`), matching CPU register width for zero-overhead arithmetic.

#### 3. Sized Numeric Types (Systems & FFI)
For low-level systems programming, memory-constrained buffers, and C interop, explicit fixed-width integer types are supported:

| Type | Bit Width | Signed? | Value Range / Purpose |
| :--- | :---: | :---: | :--- |
| **`int`** / **`i64`** | 64 | Signed | $-2^{63}$ to $2^{63}-1$ (Default canonical integer) |
| **`i32`** | 32 | Signed | $-2^{31}$ to $2^{31}-1$ (Standard C `int` interop) |
| **`i16`** | 16 | Signed | $-32,768$ to $32,767$ (Audio / sensor streams) |
| **`i8`** | 8 | Signed | $-128$ to $127$ |
| **`u8`** / **`byte`** | 8 | Unsigned | $0$ to $255$ (Binary protocols, raw byte buffers) |
| **`u16`** | 16 | Unsigned | $0$ to $65,535$ |
| **`u32`** | 32 | Unsigned | $0$ to $4,294,967,295$ (Hashes, bitmasks) |
| **`u64`** | 64 | Unsigned | $0$ to $2^{64}-1$ |
| **`usize`** / **`isize`** | Pointer | Arch | Pointer-sized integers for memory offsets and indexing |

#### 4. Floating-Point (`float`)
- 64-bit IEEE 754 double-precision floating-point number.
- Single-precision `f32` is available for graphics and FFI interop.

#### 5. String (`string`)
- UTF-8 immutable character sequence, reference-counted (ARC) on the heap.

#### 6. Null (`null`)
- Distinct singleton representing the absence of a value (see Chapter 19 for Null Safety).

---

### 1.2 Composite Types
- **Arrays**: Homogeneous typed (`int[]`, `string[]`) or generic dynamic (`array`).
- **Maps**: Key-value associative tables (`map[K, V]`).
- **Tuples**: Fixed-length heterogeneous groupings: `(T1, T2, ...)`.
- **Structs**: Nominal user-defined record structures (defined via `struct`).

---

### 1.3 Type Checking (`is` / `is not`)
- Runtime introspection operator: `expr is Type` evaluates to `bool`.
- Negated form: `expr is not Type`.
- Null checks: `expr is null` and `expr is not null`.

---

### 1.4 Explicit Conversions
- Conversions are strictly explicit: `int(expr)`, `float(expr)`, `str(expr)`, `bool(expr)`.
- Implicit type coercions are prohibited to prevent silent bugs (e.g. `"count: " + 25` is a compile error; `f"count: {25}"` or `"count: " + str(25)` must be used).

---

## 2. Memory Layout & Stack vs Heap Semantics

| Type | Allocation | Lifecycle | Reference Counted? |
| :--- | :--- | :--- | :--- |
| `int`, `i8`..`i64`, `u8`..`u64`, `float`, `bool` | Stack / Register | Copy-by-value | ❌ No |
| `string` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `array`, `map` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `struct` | Heap | ARC (`retain` / `release`) | ✅ Yes |
