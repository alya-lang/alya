# Chapter 05: Pattern Matching (`when`)

## 1. Specification Rules

### 1.1 Overview
The `when` construct in Alya is a unified, expressive pattern-matching mechanism that subsumes traditional `switch`/`case` and conditional chains (`if-elif-else`). It can be evaluated both as a **statement** and as an **expression** returning a value.

A `when` block is terminated strictly with `end`.

### 1.2 Target Form (`when <expr>`)
When an expression is provided to `when`, each subsequent branch begins with `is`:
- **Single value match:** `is 1`
- **Multiple value match:** `is 2, 3, 5, 7` (matches any of the listed values)
- **Range match:** `is 10..20` (inclusive range)
- **Relational match:** `is > 100`, `is <= 0` (relational operator applied to the target)
- **Type match:** `is int`, `is string`, `is null`
- **Enum match:** `is Status.Active`, `is Status.Pending`

```alya
when http_code
    is 200, 201
        say "Success"
    is 400..499
        say "Client Error"
    is 500..599
        say "Server Error"
    else
        say "Unknown Status"
end
```

### 1.3 Argumentless Form (`when`)
When `when` is invoked without a target expression, it acts as a clean boolean dispatcher where each branch is an independent boolean condition:

```alya
when
    score >= 90 => "A"
    score >= 80 => "B"
    score >= 70 => "C"
    else        => "F"
end
```

### 1.4 Statement Form vs Expression Form
- **Statement Form:** Each branch contains one or more statements indented on new lines.
- **Expression Form:** Each branch uses `=>` followed by an expression. When used as an expression, the matched arm's evaluated expression becomes the return value of the `when` block.

```alya
let message = when status
    is 200 => "OK"
    is 404 => "Not Found"
    else   => "Other"
end
```

### 1.5 Pattern Guards (`if <guard>`)
Arms can optionally include an additional guard predicate:
```alya
when val
    is int if val > 100 => say "Big integer"
    is int              => say "Normal integer"
    else                => say "Not an integer"
end
```

### 1.6 Exhaustiveness & Fallback (`else`)
- When `when` is used as an expression, it **must be exhaustive**. The compiler enforces that either all possible enum variants are covered, or a fallback `else` branch is provided.
- If a non-exhaustive expression `when` is encountered, compilation fails.

### 1.7 Destructuring Pattern Matching (Tuples & Fixed-Size Structures)
Alya supports deep tuple and fixed-size array destructuring in `is` arms. Variable bindings introduced in the pattern are scoped to the arm's block or expression, and can be combined with guard clauses:

```alya
let coords = (10, 25)

when coords
    is (0, 0) =>
        say "Coordinates at origin"
    is (x, y) if x == y =>
        say f"Diagonal point at {x}"
    is (x, y) =>
        say f"Cartesian point: x={x}, y={y}"
end
```

- Wildcards (`_`) can be used to ignore unused elements: `is (x, _)`.
- Literal checks and variable bindings can be mixed: `is (0, y)` checks that the first element is `0` and binds the second element to `y`.

### 1.8 Tagged Union & Variant Pattern Matching
`when` can inspect dynamically typed objects (e.g. `any`, interfaces, or union types) and extract payload fields from concrete struct variants:

```alya
struct Success
    value: int
end

struct Failure
    message: string
end

function check_status_code(code: int) -> any
    if code >= 200 and code < 300
        return Success { value: code }
    else
        return Failure { message: "Service Unavailable" }
    end
end

let res = check_status_code(200)

when res
    is Success(code) =>
        say f"HTTP request succeeded with status {code}"
    is Failure(err_msg) =>
        say f"HTTP request failed: {err_msg}"
    else =>
        say "Unknown response state"
end
```

- Flow-sensitive type narrowing applies automatically inside the matched arm, allowing direct typed field access and method calls.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
WhenStmt       ::= "when" ( Expr )? ( WhenArm )* ( ElseArm )? "end"

WhenArm        ::= ( TargetArm | CondArm )
TargetArm      ::= "is" PatternList ( "if" Expr )? ( Block | "=>" Expr )
CondArm        ::= Expr ( Block | "=>" Expr )
ElseArm        ::= "else" ( Block | "=>" Expr )

PatternList    ::= Pattern ( "," Pattern )*
Pattern        ::= Literal
                 | RangeExpr
                 | RelationalPattern
                 | TypeIdent
                 | EnumAccess
                 | TuplePattern
                 | VariantPattern

TuplePattern   ::= "(" ( PatternElem ( "," PatternElem )* )? ")"
PatternElem    ::= Ident | "_" | Literal

VariantPattern ::= Ident "(" ( Ident | "_" ) ( "," ( Ident | "_" ) )* ")"

RelationalPattern ::= ( ">" | ">=" | "<" | "<=" | "==" | "!=" ) Expr
```
