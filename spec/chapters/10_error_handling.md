# Chapter 10: Error Handling (`try` / `catch` / `throw` / `finally`)

## 1. Specification Rules

### 1.1 Overview
Alya supports structured exception handling via `try`, `catch`, `finally`, and `throw`. It combines modern exception propagation with deterministic RAII-style `defer` execution.

### 1.2 Throwing Errors (`throw`)
Any value (a `string`, an `int` code, or a structured `struct`) can be thrown:
```alya
throw "Invalid operation: divisor cannot be zero"
```
- A bare `throw` inside a `catch` block re-throws the currently captured error up the call stack.
- Uncaught struct errors print their string `message` field (`Runtime error: ...`); the thrown value itself is unchanged, so `catch` still binds the struct.

### 1.3 The `try / catch / finally` Block
The construct begins with `try`, contains at least a `catch` or `finally` clause, and ends with `end`.

```alya
try
    risky_operation()
catch err
    say f"Caught error: {err}"
finally
    say "Cleanup executed unconditionally"
end
```
- **Optional Catch Variable:** `catch err` binds the thrown object. If the variable is omitted (`catch`), errors are caught anonymously.
- **The `finally` Block:** Always executes, whether the `try` block completes successfully, encounters a handled error, or encounters an unhandled error.

### 1.4 Structured Error Payloads & Pattern Matching
Idiomatic Alya code creates domain-specific error structs. The payload preserves all concrete fields and reference counts:

```alya
struct NetworkError
    code: int
    endpoint: string
    reason: string
end

try
    throw NetworkError { code: 503, endpoint: "offline.service.internal", reason: "Target gateway unreachable" }
catch err
    when err
        is NetworkError =>
            say f"Network failed ({err.code}) at {err.endpoint}: {err.reason}"
        is string =>
            say f"Simple message error: {err}"
        else =>
            say "Unknown error payload"
    end
end
```
- Flow-sensitive type narrowing automatically narrows `err` to `NetworkError` in the matched arm, enabling direct field access without unsafe casts.
- Structured payloads can be re-thrown via `throw` or `throw err` with preserved semantics.

### 1.5 Execution Order with `defer`
When an exception occurs inside a block with `defer` statements:
1. Active `defer` statements in the failing scope execute in LIFO order.
2. Control transfers to the nearest enclosing matching `catch` block.
3. The corresponding `finally` block executes.

### 1.6 Trapped Runtime Type Errors
Dynamic type errors at call and builtin boundaries trap as catchable errors instead of crashing the process: calling a non-function value throws `"value is not callable"`, `keys()` on a non-container throws `"keys() requires a map or array"`, and a nonzero small integer reaching a declared `string` parameter throws `"type mismatch: expected 'string' argument"`. Only provably-foreign values trap (nonzero small integers, and managed strings for calls/`keys()`); null keeps the lenient-null contract and ambiguous heap values keep the legacy path (see Chapter 02 §1.7).

### 1.7 Catch-Escape Round-Trip (Dynamic Kind)
A `catch` binding is dynamically typed: each execution may deliver a `string` or a `struct` (or any other thrown shape). The binding's value round-trips through every escape path — `push` into arrays, map/array index-assign, struct fields, derived locals, and globals — with its exact runtime kind preserved, so struct fields read back structs and strings read back strings. String-throw handling is unchanged (type-driven, no behavioral delta). Net: `push(bag, err); bag[0].a` observes the thrown struct's fields, and `push(bag2, e2); bag2[0]` observes the thrown string (see `spec/integration/70_catch_escape_roundtrip.alya`).

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
TryCatchStmt  ::= "try" Block ( CatchClause )? ( FinallyClause )? "end"

CatchClause   ::= "catch" ( "("? Ident ")"? )? Block

FinallyClause ::= "finally" Block

ThrowStmt     ::= "throw" ( Expr )?
```
