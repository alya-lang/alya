# Chapter 06: Loops & Iteration

## 1. Specification Rules

### 1.1 Overview
Alya provides clean, structured looping primitives designed for clarity and safety. Every loop block is terminated with `end`.

### 1.2 The `while` Loop
Executes a block of code as long as the conditional expression evaluates to truthy (`true`, non-null).
```alya
while condition
    # body
end
```
- The condition expression does not require parentheses.
- If the condition is false initially, the loop body never executes.

### 1.3 The `for .. in` Range Loop
Iterates over a numeric sequence defined by a range expression (`start..end`):
```alya
for i in 0..10
    say i
end
```
- By default, ranges in Alya are inclusive of `start` and exclusive of `end` (half-open `[start, end)`), or inclusive `start..=end`.
- The loop index variable (`i`) is implicitly scoped to the loop block and cannot be mutated outside the iteration step.

### 1.4 The `for .. in` Collection Iteration
Iterates directly over elements of an iterable (e.g. `array`, `string`):
```alya
for item in items
    say item
end
```

### 1.5 Key-Value and Indexed Iteration (`for key, value in`)
When iterating over:
- **Arrays**: `for index, item in array` binds the zero-based numeric index to the first variable and the element to the second variable.
- **Maps**: `for key, value in map` binds the entry key to the first variable and entry value to the second variable.

```alya
for index, item in fruits
    say f"Item {index}: {item}"
end

for key, value in user_profile
    say f"{key} -> {value}"
end
```

### 1.6 The `repeat` Loop (Unconditional Loop)
Provides an explicit infinite loop construct without requiring synthetic condition flags (`while true`):
```alya
repeat
    let packet = read_socket()
    if packet.is_closed()
        break
    end
end
```
- Executes indefinitely until explicitly exited via `break` or `return`.

### 1.7 Flow Control (`break` & `continue`)
- `break`: Immediately exits the innermost enclosing loop.
- `continue`: Skips the remainder of the current iteration and jumps to the evaluation of the next cycle.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
LoopStmt       ::= WhileLoop | ForLoop | RepeatLoop

WhileLoop      ::= "while" Expr Block "end"

ForLoop        ::= "for" LoopBinding "in" Expr ( ".." Expr )? Block "end"

LoopBinding    ::= Ident ( "," Ident )?

RepeatLoop     ::= "repeat" Block "end"

ControlStmt    ::= "break" | "continue"
```
