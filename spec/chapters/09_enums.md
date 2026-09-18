# Chapter 09: Enums & Discriminants

## 1. Specification Rules

### 1.1 Overview
An enumeration (`enum`) defines a distinct nominal type with a fixed set of named variants. An `enum` declaration starts with `enum <Name>` and terminates with `end`.

### 1.2 Basic Enums
When variants are declared without explicit values, the compiler automatically assigns zero-based sequential 64-bit integer values starting at `0`:
```alya
enum Direction
    North
    East
    South
    West
end
```
- `Direction.North` evaluates to `0`.
- `Direction.East` evaluates to `1`.
- `Direction.South` evaluates to `2`.
- `Direction.West` evaluates to `3`.

### 1.3 Explicit Discriminants (Valued Enums)
Variants can be assigned explicit constant values (integers or strings):
```alya
enum HttpStatus
    Ok = 200
    Created = 201
    BadRequest = 400
    NotFound = 404
    InternalServerError = 500
end
```
- When an integer variant follows an explicitly assigned integer variant, it continues sequential incrementation from that value:
  ```alya
  enum Step
      First = 10
      Second      # 11
      Third       # 12
  end
  ```

### 1.4 String Enums
Variants can also hold string values for readable serialization:
```alya
enum Color
    Red = "RED"
    Green = "GREEN"
    Blue = "BLUE"
end
```

### 1.5 Pattern Matching (`when`) and Exhaustiveness
Enums integrate seamlessly with pattern matching:
```alya
when status
    is HttpStatus.Ok => say "Request Succeeded"
    is HttpStatus.NotFound => say "Resource Missing"
    else => say "Other Status"
end
```
- If every variant of an enum is covered in a `when` expression, the fallback `else` branch may be safely omitted.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
EnumDef       ::= "enum" Ident ( VariantDecl )* "end"

VariantDecl   ::= Ident ( "=" Expr )? ( "," )?
```
