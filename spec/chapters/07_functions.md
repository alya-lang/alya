# Chapter 07: Functions, Closures & Defer

## 1. Specification Rules

### 1.1 Function Declaration
Functions in Alya are declared using the `function` keyword and closed with `end`.
```alya
function add(a: int, b: int) -> int
    return a + b
end
```
- Parameters can specify explicit type annotations at the signature boundary (`function add(a: int, b: int) -> int`). Parameters without annotations default to dynamic `any`.
- The return type is specified with `-> <Type>`. If a function does not return a value, `-> void` may be specified or omitted.
- **Static Return & Call Validation**:
  - Returning a value from a `-> void` function is a **compile-time error**.
  - Omitting a return expression in a non-void function (`return` without a value) is a **compile-time error**.
  - Returning an expression whose static type is incompatible with the declared return type is a **compile-time error** (`TypeError`).
  - Passing arguments with static types incompatible with declared parameter types at call sites is a **compile-time error**.
  - When an execution path reaches `end` in a void function, it returns implicitly. In non-void functions, missing a return statement on any control path is a **compile-time error**.

### 1.2 Default Parameter Values
Parameters may specify optional default values. Defaulted parameters must follow all required parameters in the signature:
```alya
function greet(name: string, prefix: string = "Hello") -> string
    return f"{prefix}, {name}!"
end
```

### 1.3 Variadic Parameters (Rest Args)
A function can accept a variable number of arguments using the `...` rest syntax. The rest parameter must be the last parameter in the declaration and is gathered into an array:
```alya
function sum_all(...numbers: int[]) -> int
    let total = 0
    for n in numbers
        total += n
    end
    return total
end
```

### 1.4 Multiple Return Values
Functions can return multiple values packed as a tuple without requiring an explicit intermediate struct:
```alya
function divmod(dividend: int, divisor: int) -> (int, int)
    let quotient = dividend / divisor
    let remainder = dividend % divisor
    return quotient, remainder
end

let q, r = divmod(17, 5)
```

### 1.5 Method Declarations (Associated Functions & UFCS)
Alya does not place method definitions inside struct bodies. Instead, methods are declared at the namespace level using dot syntax:
- **Instance method**: First parameter is named `self`:
  ```alya
  function Vector2D.length(self) -> float
      return sqrt(self.x * self.x + self.y * self.y)
  end
  ```
- **Static / Associated method**: No `self` parameter:
  ```alya
  function Vector2D.zero() -> Vector2D
      return Vector2D { x: 0.0, y: 0.0 }
  end
  ```

### 1.6 Lambdas & Closures
Anonymous functions (closures) capture variables from their enclosing lexical scope:
- **Compact single-line arrow lambda**:
  ```alya
  let double = |x: int| => x * 2
  ```
- **Multi-line block closure**:
  ```alya
  let process = |data: string| -> bool
      say f"Processing: {data}"
      return true
  end
  ```

### 1.7 The `defer` Statement
The `defer` statement schedules a statement or block to execute immediately before the enclosing function returns or exits (even in the event of an uncaught error/throw).
- Multiple `defer` statements inside the same scope execute in **Last-In, First-Out (LIFO)** order.
- Ideal for deterministic resource management, freeing allocations, and closing file handles.

```alya
function read_file(path: string) -> string
    let handle = open(path)
    defer close(handle)
    
    return read_all(handle)
end
```

### 1.8 Program Entry Point Protocol (`main` Function)
When compiling a standalone executable binary (`alya build` or `alya run`), the runtime establishes the entry point based on the designated `main` function in the root package file (conventionally `src/main.alya`):

#### Permissible Signatures:
1. **Zero Arguments, Void Return**:
   ```alya
   function main()
       say "Hello, Alya!"
   end
   ```
   The process automatically terminates with exit code `0` upon normal completion.

2. **Zero Arguments, Explicit Exit Code**:
   ```alya
   function main() -> int
       if not system_ready()
           return 1
       end
       return 0
   end
   ```

3. **Command-Line Arguments Injection**:
   ```alya
   function main(args: string[])
       for arg in args
           say f"CLI Argument: {arg}"
       end
   end
   ```
   The `args` parameter receives user arguments passed after program flags (with `args[0]` representing the first user argument, excluding compiler flags).

4. **Command-Line Arguments with Exit Code**:
   ```alya
   function main(args: string[]) -> int
       if args.length() == 0
           say "Usage: app <command>"
           return 1
       end
       return 0
   end
   ```

- **Global Argument Access**: Regardless of whether `args` is passed to `main()`, any module across the codebase can inspect arguments via `std/os.args()`.
- **Top-Level Script Statements**: For lightweight scripts, top-level statements are supported. If an explicit `function main(...)` is declared, it is automatically invoked after top-level variable/constant initializations finish.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
FunctionDecl ::= "function" ( Ident "." )? Ident "(" ParamList? ")" ( "->" ReturnType )? Block "end"

ParamList    ::= Param ( "," Param )*
Param        ::= "..."? Ident ( ":" Type )? ( "=" Expr )?

ReturnType   ::= Type
               | "(" Type ( "," Type )* ")"

LambdaExpr   ::= "|" ParamList? "|" ( "->" Type )? ( "=>" Expr | Block "end" )

DeferStmt    ::= "defer" Statement
```
