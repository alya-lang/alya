# Chapter 04: Conditionals (`if` / `elif` / `else`)

## 1. Specification Rules

### 1.1 Block Structure
- Opened with `if <condition>`.
- Secondary branches opened with `elif <condition>` (canonical) or `else if <condition>` (alias).
- Fallback branch opened with `else`.
- The conditional block is terminated strictly with `end`.
- Parentheses around conditions are **optional**. Parentheses should only be used when necessary for disambiguating nested boolean logic precedence.

### 1.2 Truthiness Semantics
- In conditional evaluation, only `false` and `null` evaluate to falsy.
- Numbers (`0`, `1`, `-1`), non-empty strings, and instantiated objects evaluate to truthy.
- For strict and robust codebases, explicit comparisons (e.g., `count == 0` or `user is not null`) are idiomatic and recommended.

### 1.3 Scope Rules
- Variables declared with `let` or `const` inside any branch of an `if` construct are strictly scoped to that branch.
- They are deallocated / decremented in reference count upon exiting the branch scope.

### 1.4 Inline / Single-Line `if`
- For concise guard clauses, single-line form is supported: `if <condition> then <statement>`.
- The single-line form does not use `elif` or `else` and does **not** require a terminating `end`.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
IfStmt       ::= "if" Expr Block ( ElifBranch )* ( ElseBranch )? "end"
               | "if" Expr "then" Statement

ElifBranch   ::= ( "elif" | "else" "if" ) Expr Block
ElseBranch   ::= "else" Block

Block        ::= ( Statement )*
```
