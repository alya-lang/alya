# Chapter 03: Operators & Expressions

## 1. Specification Rules

### 1.1 Arithmetic Operators
- `+` (Addition), `-` (Subtraction / Negation), `*` (Multiplication), `/` (Division), `%` (Modulo).
- Division by zero and modulo by zero trigger a deterministic runtime exception (`DivideByZeroError`), protected at the assembly level by hardware traps or conditional branch checks.

### 1.2 Comparison / Relational Operators
- `==` (Equality), `!=` (Inequality).
- `<` (Less than), `<=` (Less than or equal), `>` (Greater than), `>=` (Greater than or equal).
- Return type is always `bool`.
- Can be overloaded via special methods (see Chapter 20).

### 1.3 Logical Operators
- Canonical standard: **`and`**, **`or`**, **`not`**.
- Symbolic compatibility: `&&`, `||`, `!`.
- **Short-circuiting**: 
  - `left and right`: If `left` is false, `right` is never evaluated.
  - `left or right`: If `left` is true, `right` is never evaluated.

### 1.4 Bitwise Operators
- `&` (Bitwise AND), `|` (Bitwise OR), `^` (Bitwise XOR), `~` (Bitwise NOT).
- `<<` (Left Shift), `>>` (Arithmetic Right Shift).
- Only applicable to integer primitive types (`int`, `i8`..`i64`, `u8`..`u64`, `usize`).

### 1.5 Modern Expressive Operators
- **Ternary Operator**: `condition ? value_if_true : value_if_false`.
- **Null Coalescing (`??`)**: `value ?? default_value`. Evaluates to `default_value` only if `value` is `null`. Short-circuits if `value` is non-null.
- **Optional Chaining (`?.`)**: Safely navigates potentially null objects, arrays, and callable invocations (`user?.address?.city`, `records?[0]`).
- **Membership Operator (`in` / `not in`)**: Tests element presence in arrays, key presence in maps, and substring presence in strings (`"admin" in roles`).
- **Range Operators**: `start..end` (exclusive half-open range), `start..=end` (inclusive range).

---

### 1.6 Operator Precedence & Associativity Table

The following table defines the binding power and evaluation order of all operators from highest precedence (Level 14) to lowest precedence (Level 0). A Pratt parser or precedence climber must strictly conform to these levels:

| Level | Operators | Description | Associativity |
|:---:|---|---|:---:|
| **14** (Highest) | `()`, `[]`, `.`, `?.`, `!()` | Grouping, Indexing, Member Access, Optional Chaining, Force Unwrap | Left-to-Right |
| **13** | `-`, `+`, `not`, `!`, `~` | Unary Negation, Unary Plus, Logical NOT, Bitwise NOT | Right-to-Left |
| **12** | `as` | Type Casting | Left-to-Right |
| **11** | `*`, `/`, `%` | Multiplicative (Multiplication, Division, Modulo) | Left-to-Right |
| **10** | `+`, `-` | Additive (Addition, Subtraction) | Left-to-Right |
| **9** | `<<`, `>>` | Bitwise Shift (Left, Right) | Left-to-Right |
| **8** | `..`, `..=` | Range Construction (Exclusive, Inclusive) | Non-Associative |
| **7** | `<`, `<=`, `>`, `>=`, `in`, `not in`, `is` | Relational, Membership, Dynamic Type Check | Left-to-Right |
| **6** | `==`, `!=` | Equality and Inequality | Left-to-Right |
| **5** | `&` | Bitwise AND | Left-to-Right |
| **4** | `^` | Bitwise XOR | Left-to-Right |
| **3** | `\|` | Bitwise OR | Left-to-Right |
| **2** | `and`, `&&` | Logical Conjunction (Short-Circuiting) | Left-to-Right |
| **1** | `or`, `\|\|` | Logical Disjunction (Short-Circuiting) | Left-to-Right |
| **0.5** | `??` | Null Coalescing (Short-Circuiting) | Right-to-Left |
| **0.3** | `? :` | Ternary Conditional (`cond ? expr1 : expr2`) | Right-to-Left |
| **0** (Lowest) | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `\|=`, `^=` | Assignment and Compound Assignment | Right-to-Left |

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
Expr             ::= AssignExpr

AssignExpr       ::= TernaryExpr ( AssignOp AssignExpr )?
AssignOp         ::= "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^="

TernaryExpr      ::= CoalesceExpr ( "?" Expr ":" TernaryExpr )?
CoalesceExpr     ::= LogicalOrExpr ( "??" CoalesceExpr )?

LogicalOrExpr    ::= LogicalAndExpr ( ( "or" | "||" ) LogicalAndExpr )*
LogicalAndExpr   ::= BitwiseOrExpr ( ( "and" | "&&" ) BitwiseOrExpr )*

BitwiseOrExpr    ::= BitwiseXorExpr ( "|" BitwiseXorExpr )*
BitwiseXorExpr   ::= BitwiseAndExpr ( "^" BitwiseAndExpr )*
BitwiseAndExpr   ::= EqualityExpr ( "&" EqualityExpr )*

EqualityExpr     ::= RelationalExpr ( ( "==" | "!=" ) RelationalExpr )*
RelationalExpr   ::= RangeExpr ( ( "<" | "<=" | ">" | ">=" | "in" | "not in" | "is" ) RangeExpr )*
RangeExpr        ::= ShiftExpr ( ( ".." | "..=" ) ShiftExpr )?

ShiftExpr        ::= AdditiveExpr ( ( "<<" | ">>" ) AdditiveExpr )*
AdditiveExpr     ::= MultiplicativeExpr ( ( "+" | "-" ) MultiplicativeExpr )*
MultiplicativeExpr ::= CastExpr ( ( "*" | "/" | "%" ) CastExpr )*
CastExpr         ::= UnaryExpr ( "as" Type )*

UnaryExpr        ::= ( "-" | "+" | "not" | "!" | "~" ) UnaryExpr | PostfixExpr
PostfixExpr      ::= PrimaryExpr ( MemberAccess | IndexAccess | CallAccess | SafeMemberAccess | ForceUnwrap )*

MemberAccess     ::= "." Identifier
SafeMemberAccess ::= "?." Identifier
IndexAccess      ::= "[" Expr "]"
CallAccess       ::= "(" ( Expr ( "," Expr )* )? ")"
ForceUnwrap      ::= "!"

PrimaryExpr      ::= Literal | Identifier | "(" Expr ")" | ArrayLit | MapLit
```
