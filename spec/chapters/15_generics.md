# Chapter 15: Generics & Parametric Polymorphism

## 1. Specification Rules

### 1.1 Overview
Generics enable code reusability without sacrificing static type safety or runtime efficiency. Alya uses square brackets `[...]` for type parameters on structs, functions, and interfaces.

### 1.2 Generic Functions
Type parameters are specified immediately after the function name:
```alya
function swap[T](a: T, b: T) -> (T, T)
    return b, a
end
```
- **Type Inference at Call Sites**: Type arguments can be omitted when they can be inferred unambiguously from argument types:
  ```alya
  let x, y = swap(10, 20)      # Infers T = int
  let s1, s2 = swap("a", "b")  # Infers T = string
  ```

### 1.3 Generic Structs
Structs can declare one or more type parameters:
```alya
struct Pair[T, U]
    first: T
    second: U
end

struct Stack[T]
    elements: T[]
end
```
- Instantiation:
  ```alya
  let p = Pair[int, string] { first: 1, second: "One" }
  ```

### 1.4 Generic Methods
Methods associated with generic structs declare their type parameters:
```alya
function Stack[T].push(self, item: T)
    self.elements.push(item)
end

function Stack[T].pop(self) -> T
    return self.elements.pop()
end
```

### 1.5 Type Constraints (Bounded Polymorphism)
Type parameters can be bounded by interfaces to restrict which types can be substituted:
```alya
function summarize[T: Describable](item: T) -> string
    return item.describe()
end
```
- Multiple constraints: `[T: Reader + Closer]`

### 1.6 Compilation Strategy: Zero-Cost Monomorphization
Alya uses **Monomorphization** for generic code generation:
- Each unique concrete instantiation (e.g. `Stack[int]`, `Stack[float]`) produces a specialized, dedicated native assembly implementation.
- Primitives remain completely unboxed (no dynamic heap allocation for `int` or `float`).
- Code achieves maximum execution speed and compiler inlining opportunities, with zero runtime boxing penalties.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
TypeParams    ::= "[" TypeParam ( "," TypeParam )* "]"
TypeParam     ::= Ident ( ":" TypeBound )?
TypeBound     ::= Ident ( "+" Ident )*

GenericFnDecl ::= "function" ( Ident "." )? Ident TypeParams "(" ParamList? ")" ( "->" ReturnType )? Block "end"
GenericStruct ::= "struct" Ident TypeParams StructFieldList "end"
```
