# Chapter 01: Variables, Mutability & Constants

## 1. Specification Rules

### 1.1 Variable Declaration (`let`)
- Variables declared with `let` are **mutable**.
- Type annotations on initialized variables are **optional**. The type is inferred deterministically from the right-hand side expression.
- When an initial value is omitted, an explicit type annotation is **mandatory**.
- **Definite Assignment Rule**: Reading a variable before it has been assigned a value is a **compile-time error**.

### 1.2 Constants (`const`)
- Declared with `const`.
- Must be assigned an immutable expression at declaration.
- Reassignment to a `const` is a **compile-time error**.
- Stored as immediate values or in the read-only data section (`.rodata`).

### 1.3 Multiple Assignment & Swap
- Multiple variables can be declared and initialized simultaneously: `let a, b = 1, 2`.
- Atomic swapping is natively supported without explicit temp variables: `a, b = b, a`.

### 1.4 Destructuring
- Tuple destructuring: `let (x, y) = tuple_expr`.
- Array destructuring: `let [head, tail] = array_expr`.
- Map destructuring: `let { key1, key2 } = map_expr`.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
VarDecl      ::= "let" ( Ident ( ":" Type )? ( "=" Expr )?
                       | IdentList "=" ExprList
                       | Pattern "=" Expr )

ConstDecl    ::= "const" Ident ( ":" Type )? "=" Expr

Pattern      ::= "(" IdentList ")"
               | "[" IdentList "]"
               | "{" IdentList "}"

IdentList    ::= Ident ( "," Ident )*
ExprList     ::= Expr ( "," Expr )*
```
