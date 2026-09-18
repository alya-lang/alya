# Chapter 13: Collections, Slicing & Comprehensions

## 1. Specification Rules

### 1.1 Arrays (Dynamic Lists)
Arrays in Alya are contiguous, dynamically-sized heap structures managed via ARC.
- **Literal Syntax**: `let numbers = [10, 20, 30]`
- **Type Annotation**: `int[]` or `array[int]`
- **Indexing**:
  - Zero-based: `numbers[0]` evaluates to the first element.
  - Negative indexing: `numbers[-1]` evaluates to the last element (`length - 1`), `numbers[-2]` to the second last. Out-of-bounds indexing produces a runtime panic or exception.
- **Slicing**:
  - `numbers[1..3]`: Produces a new array containing elements from index 1 up to (exclusive) index 3.
  - `numbers[2..]`: From index 2 to the end of the array.
  - `numbers[..2]`: From start to index 2 (exclusive).
  - `numbers[..]`: Shallow copy of the entire array.
- **Intrinsic Methods**:
  - `arr.length()`: Returns the number of active elements.
  - `arr.capacity()`: Returns current internal buffer capacity.
  - `arr.push(val)`: Appends an element to the end.
  - `arr.pop()`: Removes and returns the last element.
  - `arr.insert(index, val)`: Inserts element at specified index.
  - `arr.remove_at(index)`: Removes element at index and shifts subsequent elements.
  - `arr.contains(val)`: Equivalent to `val in arr`.

### 1.2 Hash Maps (Associative Dictionaries)
Hash Maps associate keys with values using high-throughput hash buckets.
- **Literal Syntax**: `let user = { "id": 101, "name": "Alya", "role": "admin" }`
- **Type Annotation**: `map[string, any]` or `map[string, int]`
- **Key Types**: Keys must be hashable types (`string`, `int`, `bool`).
- **Access & Mutation**:
  - `map[key]`: Look up key. If missing, returns `null` or raises KeyError depending on strict mode.
  - `map.get(key, default_value)`: Safe lookup returning `default_value` if the key is absent.
  - `map[key] = value`: Inserts or updates key.
  - `map.remove(key)`: Deletes key and returns boolean indicating whether key existed.
  - `key in map` / `key not in map`: Fast $O(1)$ membership check.
- **Iteration**:
  - `for key, value in map`: Iterates over key-value pairs.
  - `map.keys()`: Returns an array of keys.
  - `map.values()`: Returns an array of values.

### 1.3 Tuples
Tuples represent fixed-size, heterogeneous groupings allocated on the stack.
- **Literal Syntax**: `let entry = (1, "Active", 99.5)`
- **Positional Access**: `entry.0`, `entry.1`, `entry.2`
- **Destructuring**: `let (id, status, score) = entry`

### 1.4 List & Map Comprehensions
To provide Python/Ruby-grade ergonomic data transformation, Alya natively supports comprehensions:
- **Array Comprehension**:
  ```alya
  let squares = [x * x for x in 1..10]
  let evens = [n for n in numbers if n % 2 == 0]
  ```
- **Map Comprehension**:
  ```alya
  let word_lengths = {w: w.length() for w in words}
  ```

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
ArrayLit     ::= "[" ( ExprList | Comprehension )? "]"
MapLit       ::= "{" ( KeyValueList | MapComprehension )? "}"
TupleLit     ::= "(" Expr "," ( Expr ( "," Expr )* )? ")"

Comprehension    ::= Expr "for" Ident "in" Expr ( "if" Expr )?
MapComprehension ::= Expr ":" Expr "for" Ident "in" Expr ( "if" Expr )?

SliceExpr    ::= Expr "[" ( Expr )? ".." ( Expr )? "]"
IndexExpr    ::= Expr "[" Expr "]"

KeyValueList ::= KeyValue ( "," KeyValue )*
KeyValue     ::= ( StringLit | Ident | Expr ) ":" Expr
```
