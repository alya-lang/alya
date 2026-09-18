# Chapter 19: Null Safety & Optionals (`T?`)

## 1. Specification Rules

### 1.1 Philosophy: Eliminating Null Reference Exceptions
Alya enforces **Compile-Time Null Safety** to eradicate runtime null dereference panics (Tony Hoare's "Billion-Dollar Mistake"):
- **Non-Nullable by Default**: All types (both primitives and composite heap objects) cannot hold `null` unless explicitly marked as nullable.
  ```alya
  let name: string = "Alice" # Valid
  name = null                # Compile-time ERROR: Cannot assign null to non-nullable type 'string'
  ```
- **Explicit Nullable Type (`T?`)**: Types that may contain `null` must be declared with a trailing question mark (`?`):
  ```alya
  let nickname: string? = null # Valid
  ```

### 1.2 Safe Navigation Operator (`?.`)
Attempting to directly invoke a method or access a field on a nullable type is a compile-time error. The safe navigation operator (`?.`) must be used:
```alya
let len: int? = nickname?.length()
```
- If `nickname` is `null`, evaluation short-circuits and returns `null`.
- If `nickname` is non-null, the method is invoked normally.

### 1.3 Null Coalescing Operator (`??`)
Provides a default fallback value when dealing with nullable expressions:
```alya
let display_name: string = nickname ?? "Anonymous"
```
- If the left operand is non-null, it is returned.
- If the left operand is `null`, the right operand is evaluated and returned.

### 1.4 Flow-Sensitive Type Narrowing (Smart Casting)
When a nullable variable is checked against `null` inside an `if` condition, the compiler automatically **promotes** the variable to its non-nullable counterpart within that block:
```alya
let email: string? = fetch_user_email()

if email != null
    # Inside this block, 'email' is statically guaranteed to be 'string' (not string?)
    say email.upper()
    say f"Email length: {email.length()}"
end
```

### 1.5 Guard Clauses (`guard let`)
Allows early exit if an optional value is `null`, unwrapping it into a non-nullable variable for the remainder of the current scope:
```alya
function process_user(input: string?)
    guard let username = input else
        say "Input was null, aborting"
        return
    end

    # 'username' is now a non-nullable 'string' for the rest of the function
    say f"Processing user: {username}"
end
```

### 1.6 Force Unwrap Operator (`!`)
When a developer is mathematically certain that a nullable value cannot be `null` (e.g. after external verification), the postfix force-unwrap operator (`!`) casts `T?` to `T`:
```alya
let valid_str: string = nickname!
```
- If the value is unexpectedly `null` at runtime, a panic with file and line diagnostics is raised immediately.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
NullableType   ::= Type "?"

SafeNavExpr    ::= Expr "?." Ident ( "(" ArgList? ")" )?
CoalesceExpr   ::= Expr "??" Expr
ForceUnwrap    ::= Expr "!"

GuardStmt      ::= "guard" "let" Ident "=" Expr "else" Block "end"
```
