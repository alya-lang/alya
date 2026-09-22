# Chapter 18: Attributes, Directives & Comptime

## 1. Specification Rules

### 1.1 Overview
Attributes (`@attribute` or `@attribute(...)`) provide compiler directives, ABI annotations, and build-time metadata without introducing intrusive language keywords.

### 1.2 Function & Struct Attributes
- **Optimization Directives**:
  - `@inline`: Recommends that the compiler inline the annotated function call directly into the caller's frame.
  - `@noinline`: Strictly prevents inlining (useful for stack trace preservation and debugging).
  - `@cold`: Marks error handlers or unlikely branches to guide branch prediction away from this code path.
- **Lifecycle & Deprecation**:
  - `@deprecated("message")`: Simple form that triggers a compile-time warning pointing to the replacement symbol.
  - `@deprecated(since = "...", note = "...", error = false)`: Structured metadata form providing:
    - `since`: The version where deprecation took effect (e.g. `"0.2.0"`).
    - `note`: Migration instructions and replacement symbol recommendation.
    - `error`: Optional boolean flag. When `false` (default), triggers a compile-time warning. When set to `true`, elevates the deprecation into a fatal compile-time error.
- **ABI & Memory Layout**:
  - `@repr(C)`: Enforces standard C structure layout, padding, and field alignment for seamless FFI.
  - `@export("external_name")`: Exports the function as an unmangled native symbol for dynamic library consumers.

### 1.3 Conditional Compilation (`@cfg`)
Directs the compiler to include or exclude items based on platform parameters:
```alya
@cfg(os = "windows")
function get_platform_name() -> string
    return "Windows NT"
end

@cfg(os = "linux")
function get_platform_name() -> string
    return "Linux"
end
```
Supported configurations include:
- `os = "windows"` | `"linux"` | `"macos"`
- `arch = "x64"` | `"arm64"` | `"x86"`
- `debug = true` | `false`

### 1.4 Test & Benchmark Integration (`@test`, `@bench`)
Built-in testing harness integration:
```alya
@test
function test_addition()
    let sum = 2 + 2
    assert(sum == 4)
end
```
- The `alya test` command discovers all functions decorated with `@test` across project files and executes them in an isolated test runner. Discovery is by filename (`test_*`, `*_test`) AND by content (any file declaring `test` blocks or `@test` functions); `alya bench` mirrors this for `bench` blocks and `@bench` functions.

### 1.5 Introspection & Compile-Time Evaluation (`comptime`)
- **`sizeof(Type)`**: Returns the exact byte footprint of a type as an immediate compile-time constant integer.
- **`alignof(Type)`**: Returns the memory byte alignment requirement.
- **`typeof(expression)`**: Returns the resolved static type name.
- **`comptime` expressions**: Evaluates constant math, string concatenation, bitwise operations, or configuration pre-computations strictly during compilation:
  ```alya
  const MAX_PACKET_SIZE = comptime { 1024 * 64 }
  const SECONDS_PER_DAY = comptime(24 * 60 * 60)
  let compile_banner = comptime { "ALYA_" + "NATIVE" }
  ```

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
Attribute     ::= "@" Ident ( "(" AttributeArgs? ")" )?
AttributeArgs ::= AttributeArg ( "," AttributeArg )*
AttributeArg  ::= Ident ( "=" ( StringLit | BoolLit | Ident ) )?
                | StringLit

AnnotatedDecl ::= ( Attribute )* ( FunctionDecl | StructDef | ConstDecl )

ComptimeExpr  ::= "comptime" ( Block "end" | "{" Expr "}" | "(" Expr ")" | Expr )
IntrospectExpr::= ( "sizeof" | "alignof" ) "(" Type ")"
                | "typeof" "(" Expr ")"
```
