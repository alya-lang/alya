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

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
ExternBlock   ::= "extern" StringLit ( "from" StringLit )? ( ExternFnDecl )* "end"

ExternFnDecl  ::= "function" Ident "(" ExternParamList? ")" ( "->" Type )?

ExternParamList ::= ExternParam ( "," ExternParam )*
ExternParam   ::= Ident ":" Type
```
