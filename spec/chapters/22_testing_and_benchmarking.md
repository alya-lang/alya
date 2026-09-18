# Chapter 22: Testing, Assertions & Benchmarking

## 1. Specification Rules

### 1.1 Philosophy: Built-in Verification Harness
Following Go and Zig's philosophy, testing and benchmarking in Alya are first-class language citizens rather than afterthought external frameworks:
- **Zero Framework Setup**: No third-party dependencies required to write or run tests.
- **Production Stripping**: Test and benchmark blocks are completely stripped during release builds (`alya build --release`), incurring zero binary size or runtime cost in production.

### 1.2 The `assert` Statement
- Evaluates a boolean condition. If false, aborts execution with file name, line number, and diagnostic details:
  ```alya
  assert count > 0, "Counter must be strictly positive"
  ```
- Equality helper:
  ```alya
  assert_eq(actual, expected, "Mismatch in calculated result")
  ```

### 1.3 Native `test` Blocks
Dedicated test blocks are declared with `test "<name>"` and closed with `end`:
```alya
test "array push and length growth"
    let list: int[] = []
    list.push(10)
    list.push(20)

    assert list.length() == 2
    assert list[0] == 10
end
```
- Can be placed directly alongside production code or in separate `*_test.alya` files.
- Executed automatically via `alya test`.

### 1.4 Native `bench` Blocks
Micro-benchmarks measure operations per second and execution latency:
```alya
bench "matrix multiplication"
    let m1 = Matrix.identity(4)
    let m2 = Matrix.identity(4)
    let res = m1 * m2
end
```
- The compiler driver (`alya bench`) runs the block through warmup iterations, then measures latency over multiple sample windows.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
TestBlock    ::= "test" StringLit Block "end"
BenchBlock   ::= "bench" StringLit Block "end"

AssertStmt   ::= "assert" Expr ( "," StringLit )?
               | "assert_eq" "(" Expr "," Expr ( "," StringLit )? ")"
```
