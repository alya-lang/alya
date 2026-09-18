# Chapter 14: Interfaces & Structural Polymorphism

## 1. Specification Rules

### 1.1 Philosophy: Structural Subtyping (Duck Typing with Static Rigor)
Alya adopts Go's structural interface model. Interfaces define contracts of behavior through sets of method signatures:
- **Zero Explicit Declarations**: A struct does not declare that it implements an interface (no `implements` or `extends` keywords).
- **Implicit Satisfaction**: Any struct that defines all methods required by an interface automatically satisfies that interface.
- **Decoupled Packages**: Consumers define the interfaces they need, without requiring producers to know about them.

### 1.2 Interface Declaration
An interface is declared with `interface <Name>` and closed with `end`:
```alya
interface Reader
    function read(buf: byte[]) -> int
end

interface Closer
    function close() -> bool
end
```

### 1.3 Interface Composition (Embedding)
Interfaces can embed other interfaces to form composite contracts:
```alya
interface ReadCloser
    Reader
    Closer
end
```

### 1.4 Dynamic Dispatch & Fat Pointers
When an interface is used as a parameter type or variable type:
- The compiler constructs a **Fat Pointer** consisting of two words (16 bytes on 64-bit platforms):
  1. Pointer to the underlying concrete data instance.
  2. Pointer to the compiler-generated virtual method table (`vtable`) for that specific type-interface pair.
- Direct struct method calls (`point.dist()`) remain static direct calls with zero indirection. Interface method calls incur only a single table lookup.

### 1.5 Type Assertions & Queries (`is`)
Code can query or downcast an interface value to a concrete struct:
```alya
function log_stream(r: Reader)
    if r is File
        say "Stream is backed by a disk file"
    end
end
```
- In pattern matching:
  ```alya
  when r
      is File   => say "Handling file stream"
      is Socket => say "Handling network socket"
      else      => say "Handling custom stream"
  end
  ```

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
InterfaceDef     ::= "interface" Ident ( InterfaceMember )* "end"

InterfaceMember  ::= MethodSignature
                   | Ident

MethodSignature  ::= "function" Ident "(" ParamList? ")" ( "->" ReturnType )?
```
