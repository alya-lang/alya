# Chapter 12: FFI & Low-Level Interop

## 1. Specification Rules

### 1.1 Overview
Foreign Function Interface (FFI) allows Alya programs to bind directly to native shared libraries (`.so`, `.dylib`, `.dll`) or standard C runtime functions without compilation overhead.

### 1.2 The `extern` Block
External functions are declared inside an `extern` block terminated with `end`:
```alya
extern "C"
    function puts(s: str) -> i32
    function abs(n: i32) -> i32
end
```
- **ABI Specification:** The string following `extern` specifies the target ABI convention (e.g. `"C"`, `"system"`).
- **Library Linking (`from <library>`):** If functions reside in a specific dynamic library rather than libc/kernel32, specify the library name with `from`:
  ```alya
  extern "C" from "sqlite3"
      function sqlite3_libversion() -> str
  end
  ```

### 1.3 Interop Types
To facilitate ABI layout compatibility, the following explicit fixed-width scalar types are used in `extern` signatures:
- **Signed Integers:** `i8`, `i16`, `i32`, `i64`, `isize`
- **Unsigned Integers:** `u8`, `u16`, `u32`, `u64`, `usize`
- **Floating-point:** `f32`, `f64`
- **Pointers & C Strings:** `ptr`, `str` (null-terminated C string pointer)
- **Void:** `void` (for functions that return no value)

### 1.4 Idiomatic Safe Wrappers
Direct calls to foreign C functions are considered unchecked and inherently low-level. Best practice in Alya is to encapsulate foreign signatures within safe, ergonomic wrapper functions:
```alya
extern "C"
    function strlen(s: str) -> usize
end

function get_native_len(text: string) -> int
    let len = strlen(text)
    return int(len)
end
```

### 1.5 Extern/Function Name Collisions
An `extern` declaration and an Alya `function` must not share one name in the same scope (including across imports merged into that scope). Calls cannot resolve across the extern/function boundary — arity is checked against the function while codegen binds the extern — so the collision is a check-time `Duplicate definition` error. Name the extern distinctly (e.g. `strlen_raw`) and call it from the wrapper; qualified (`mod::name`) and method (`Type::name`) definitions do not collide with a bare extern.

### 1.6 Foreign Calling Conventions (Float Arguments)
`extern` calls follow the target C ABI exactly; the internal Alya calling convention never leaks across the boundary:
- **System V AMD64 (Linux/macOS x64):** integer/pointer arguments ride `rdi, rsi, rdx, rcx, r8, r9` while `f32`/`f64` arguments ride `xmm0-xmm7` — two independent sequences, each spilling its overflow to 8-byte stack slots. `AL` carries the number of XMM registers used (required by variadic callees, ignored by fixed ones).
- **AAPCS64 (ARM64):** integer/pointer arguments ride `x0-x7`, floats ride `d0-d7`, same spill rule; `sp` stays 16-byte aligned.
- **Win64:** one sequence (`rcx, rdx, r8, r9`) mirrored into `xmm0-xmm3`.
- An `int`-typed value passed to a float-typed extern parameter converts to double on the shared tag-guarded emission (the same conversion float-param call args use); dynamically-typed values ride their current bits. Declared parameter types are authoritative for register assignment; extra variadic arguments classify by their static kind.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
ExternBlock   ::= "extern" StringLit ( "from" StringLit )? ( ExternFnDecl )* "end"

ExternFnDecl  ::= "function" Ident "(" ExternParamList? ")" ( "->" Type )?

ExternParamList ::= ExternParam ( "," ExternParam )*
ExternParam   ::= Ident ":" Type
```
