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

### 1.5 Gradual Typing Semantics & Compile-Time Type Checking

Alya features an ergonomic **gradual typing system** designed to combine the velocity of rapid dynamic prototyping with the reliability of static industrial verification.

#### 1. Dynamic Typing by Default (`any`)
- Variable declarations without explicit annotations (e.g. `let x = 42`) and unannotated function parameters default to dynamic typing (`any`).
- The `any` type acts as a universal bridge: it is **bidirectionally assignable** with all concrete static types. An untyped variable can be passed into a statically typed function, and a statically typed result can be stored into an untyped variable without ceremonial casting. This guarantees 100% backward compatibility for dynamic codebases.

#### 2. Static Compile-Time Enforcement
When types are explicitly annotated, the compiler activates strict static analysis during Pass 3.5:
- **Variable Declarations & Re-assignments**:
  ```alya
  let count: int = 10         # Statically verified as int
  count = 20                  # Statically verified
  count = "twenty"            # Compile-time TypeError: Type mismatch: expected int, got string
  ```
- **Function Boundaries**:
  Arguments passed at call sites must be statically compatible with parameter types. Values returned by `return` expressions must match the declared return type (`-> T`). Returning a value from a `void` function or returning nothing from a non-void function triggers a compile-time `TypeError`.
- **Struct Field Invariants**:
  Struct fields declared with types (`struct Point x: int, y: int end`) are statically validated during instantiation (`Point { x: 1, y: 2 }` or `Point(1, 2)`) and field mutations (`p.x = "invalid"` is rejected at compile time).
- **Subtyping & Interoperability Rules**:
  - Sized integers (e.g. `i32`, `u8`) are safely assignable to canonical `int`.
  - Floating-point `f32` is assignable to `float` / `f64`.
  - Literals `null` are assignable to nullable optionals (`T?`) or raw pointers (`ptr`).
  - Tuple literals `(T1, T2)` are checked component-by-component against target tuple signatures.

---

## 2. Memory Layout & Stack vs Heap Semantics

| Type | Allocation | Lifecycle | Reference Counted? |
| :--- | :--- | :--- | :--- |
| `int`, `i8`..`i64`, `u8`..`u64`, `float`, `bool` | Stack / Register | Copy-by-value | ❌ No |
| `string` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `array`, `map` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `struct` | Heap | ARC (`retain` / `release`) | ✅ Yes |
