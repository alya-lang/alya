# Chapter 02: Types & Type System

## 1. Specification Rules

### 1.1 Primitive Types

#### 1. Boolean (`bool`)
- **Type**: `bool`
- **Literals**: `true` and `false`.
- **Strict Type Safety**: Integers `0` and `1` are **not** implicitly converted to `bool` in typed positions (a function expecting `bool` rejects an `int` argument at compile time).
- **Conditional Truthiness**: In conditional positions (`if`, `while`, `when` guards), the truthiness matrix of Chapter 04 §1.2 applies instead: `false`, `null`, and numeric zero (`0`, `0.0`) are falsy; all other values (including `1`, `-1`, non-empty and empty strings, instantiated objects) are truthy. For strict and robust codebases, explicit comparisons (e.g., `count == 0` or `user is not null`) remain idiomatic and recommended.

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
- Values travel as raw f64 bits in 64-bit value slots, so `std/mem` raw `peek`/`poke` round-trips are exact on 64-bit targets.
- Comparison and arithmetic fast paths materialize a call/binary-expression left operand from its slot before operating, so e.g. `read_float(p, 0) == 3.14` evaluates correctly. The x86 backend uses 32-bit value slots: float loads, stores, and comparisons truncate there (known limitation).

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
- Conversions are explicit by convention: `int(expr)`, `float(expr)`, `str(expr)`, `bool(expr)`, or string interpolation `f"count: {25}"`.
- The `+` operator stringifies its right operand when the left operand is a string (`"count: " + 25` evaluates to `"count: 25"`). Relying on this implicit stringification is discouraged; prefer interpolation (`f"count: {25}"`) or explicit `str(25)` for readability.

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

### 1.6 Explicit SIMD Vector Primitives & Tensor Engine (Roadmap Phase 6.4 Specification)

For numerical computing, 3D graphics, digital signal processing (DSP), and machine learning tensor operations, Alya specifies first-class **Explicit SIMD (Single Instruction, Multiple Data) vector types**:

#### 1. Hardware-Mapped Vector Types
- **`f64x4`**: 256-bit vector holding four 64-bit IEEE 754 floats (maps directly to AVX2/AVX-512 `%ymm` / `%zmm` registers on x64, Neon pairs on ARM64).
- **`f32x8`**: 256-bit vector holding eight 32-bit single-precision floats.
- **`i32x8`**: 256-bit vector holding eight 32-bit signed integers.
- **`i64x4`**: 256-bit vector holding four 64-bit signed integers.

#### 2. Vector Arithmetic & FMA3
- **Operator Overloading**: Native operators `+`, `-`, `*`, `/` perform single-cycle element-wise parallel execution across all lanes without scalar loop overhead.
- **Fused Multiply-Add (FMA)**: `v1.fma(v2, v3)` or `(v1 * v2) + v3` compiles directly to hardware FMA3 (`vfmadd213pd`) executing a simultaneous multiply-accumulate in a single clock cycle with zero intermediate rounding error.

#### 3. Swizzle, Shuffle & Horizontal Reduction
- **Cross-Lane Shuffling**: `v.shuffle(indices)` and swizzle operations rearrange lane data in-register without round-tripping through stack memory.
- **Horizontal Reduction**: `v.sum_horizontal()`, `v.min()`, and `v.max()` collapse all vector lanes into a single scalar value using hardware reduction tree instructions (`vhaddpd`).

#### 4. Memory Layout & Alignment
- SIMD vector types occupy 256 bits (32 bytes) in CPU vector registers or 32-byte aligned stack slots.
- Heap buffers backing vectors or N-D tensors utilize `std/mem.aligned_alloc(size, 32)` enabling zero-penalty `vmovapd` streaming loads.

---

## 2. Memory Layout & Stack vs Heap Semantics

| Type | Allocation | Lifecycle | Reference Counted? |
| :--- | :--- | :--- | :--- |
| `int`, `i8`..`i64`, `u8`..`u64`, `float`, `bool` | Stack / Register | Copy-by-value | ❌ No |
| `f64x4`, `f32x8`, `i32x8`, `i64x4` | Stack / Vector Register (%ymm) | Copy-by-value | ❌ No |
| `string` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `array`, `map` | Heap | ARC (`retain` / `release`) | ✅ Yes |
| `struct` | Heap | ARC (`retain` / `release`) | ✅ Yes |
