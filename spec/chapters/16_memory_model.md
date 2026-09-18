# Chapter 16: Memory Model, ARC & Lifecycle

## 1. Specification Rules

### 1.1 Overview
Alya is designed for predictable, deterministic memory management without the unpredictable stop-the-world pauses of tracing Garbage Collectors (GC).

### 1.2 Stack vs Heap Allocation
- **Stack Allocation (Value Semantics)**:
  - Primitive types (`int`, `float`, `bool`, `u8`..`u64`, `i8`..`i64`) and fixed-size tuples are stored directly on the stack frame.
  - Assigned and passed by value (copied).
  - Deallocated automatically upon function frame pop with zero CPU overhead.
- **Heap Allocation (Reference Semantics)**:
  - Dynamic types (`string`, `array`, `map`, `struct`) reside on the heap.
  - Variables hold reference pointers to heap memory.
  - Managed deterministically via **Automatic Reference Counting (ARC)**.

### 1.3 Automatic Reference Counting (ARC)
- Every heap object contains an internal metadata header with a 64-bit reference count.
- **Retain**: When a new variable or field references the object, the reference count increments.
- **Release**: When a reference leaves its lexical scope, is reassigned, or its enclosing struct is destroyed, the reference count decrements.
- **Immediate Reclaim**: When the reference count drops to `0`, memory is returned to the allocator immediately. Destructors and child releases trigger recursively.

### 1.4 Breaking Reference Cycles: Weak References (`weak`)
Cyclic relationships (such as parent-child nodes or graphs) can prevent reference counts from ever reaching zero. Alya solves this with the `weak` keyword:
- Strong references (`Node`) increment reference count and preserve object life.
- Weak references (`weak Node`) reference an object without incrementing its strong counter.
```alya
struct TreeNode
    name: string
    parent: weak TreeNode
    children: TreeNode[]
end
```
- A weak reference safely becomes `null` if the referenced object is deallocated.

### 1.5 High-Throughput Memory Arenas (`std/mem`)
For batch workloads (such as per-HTTP-request buffers or game loop frames), Alya provides contiguous memory Arenas:
- Memory is allocated sequentially from a pre-reserved chunk.
- Individual allocations incur zero free/ARC overhead.
- The entire arena is reclaimed in $O(1)$ constant time via `arena.reset()` or upon exiting scope via `defer arena.destroy()`.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
WeakType     ::= "weak" TypeIdent

ArenaStmt    ::= "let" Ident "=" "Arena.new" "(" Expr? ")"
```
