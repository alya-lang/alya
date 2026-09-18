# Chapter 20: Operator Overloading & Special Methods

## 1. Specification Rules

### 1.1 Philosophy: Controlled & Predictable Overloading
To balance Python/Ruby's mathematical elegance with Go's readability and prevent "operator soup":
- **Fixed Operator Set**: Only standard, pre-defined language operators can be overloaded. Arbitrary custom symbolic operator creation (such as `>>=~*`) is strictly forbidden.
- **Explicit Signature**: Overloaded operators are declared as struct methods using the `operator<Symbol>` naming convention.
- **No Side Effects in Arithmetic**: Arithmetic operators (`+`, `-`, `*`, `/`) must return new instances and must not mutate operands.

### 1.2 Overloadable Operators

| Category | Operators | Method Signature |
|---|---|---|
| **Arithmetic** | `+`, `-`, `*`, `/`, `%` | `function Type.operator+(self, other: Type) -> Type` |
| **Unary** | `-`, `not` | `function Type.operator-neg(self) -> Type` |
| **Equality** | `==`, `!=` | `function Type.operator==(self, other: Type) -> bool` |
| **Relational** | `<`, `<=`, `>`, `>=` | `function Type.operator<(self, other: Type) -> bool` |
| **Index Read** | `obj[index]` | `function Type.operator[](self, index: IdxType) -> RetType` |
| **Index Write**| `obj[index] = val` | `function Type.operator[]=(self, index: IdxType, val: ValType)` |

### 1.3 Equivalence & Derivation Rules
- Overloading `==` automatically provides the inverse `!=` unless explicitly overridden.
- Overloading `<` and `==` allows the compiler to derive `<=`, `>`, and `>=`.

### 1.4 Special Utility Methods
Certain methods are recognized by language built-ins:
- **`to_string(self) -> string`**: Automatically invoked when the struct is passed to `say` or embedded inside string interpolation `f"{obj}"`.
- **`hash(self) -> int`**: Evaluates hash value for hash map indexing when the struct is used as a map key (`map[MyStruct, Value]`).

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
OperatorMethod ::= "function" Ident ".operator" OpSymbol "(" ParamList ")" ( "->" ReturnType )? Block "end"

OpSymbol       ::= "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "[]" | "[]=" | "-neg"
```
