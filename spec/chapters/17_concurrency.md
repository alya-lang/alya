# Chapter 17: Concurrency, Fibers & Channels

## 1. Specification Rules

### 1.1 Concurrency Philosophy: Colorless Concurrency
Alya rejects the "function coloring" problem imposed by `async`/`await` in languages like JavaScript, Rust, or Python. In Alya:
- All functions are normal functions. There are no `async fn` vs `sync fn` splits.
- Concurrency is built on **Communicating Sequential Processes (CSP)** with lightweight green fibers and typed channels.
- I/O operations (sockets, file reads) yield the current fiber cooperatively to an underlying reactor event loop without blocking the host OS thread.

### 1.2 Spawning Lightweight Fibers (`spawn`)
Fibers have tiny initial stacks (~4KB) and are multiplexed M:N across available CPU cores:
- Spawning a function:
  ```alya
  spawn worker_task(job_id)
  ```
- Spawning an anonymous closure:
  ```alya
  spawn ||
      say "Executing concurrent background fiber"
  end
  ```

### 1.3 Channels (`Channel[T]`)
Channels provide thread-safe, synchronization-free data pipelines between concurrent fibers:
- **Unbuffered Channel (Rendezvous)**: Sends block until a receiver is ready.
  ```alya
  let ch = Channel[int].new()
  ```
- **Buffered Channel**: Can hold up to $N$ elements before blocking sends:
  ```alya
  let buf_ch = Channel[string].new(capacity: 10)
  ```
- **Operations**:
  - `ch.send(data)` (or `ch <- data`)
  - `let msg = ch.recv()` (or `<-ch`)
  - `ch.close()`: Closes channel; subsequent receives yield remaining buffered items, then terminate.
  - Channel iteration: `for item in ch` continues until closed.

### 1.4 The `select` Multiplexer
The `select` statement waits on multiple concurrent channel operations simultaneously:
```alya
select
    case msg = data_ch.recv()
        say f"Received data: {msg}"
    case out_ch.send("ping")
        say "Ping dispatched"
    timeout 2000
        say "Timeout: No response within 2000ms"
    else
        say "Non-blocking immediate fallback"
end
```

### 1.5 OS Worker Threads vs Green Fibers
- Use `spawn` (Green Fibers) for high-concurrency network servers, I/O bound pipelines, and event handling.
- Use `std/thread` (OS Threads) for CPU-heavy computing workloads (image encoding, cryptographic hashing, matrix multiplication).

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
SpawnStmt    ::= "spawn" ( CallExpr | ClosureExpr )

SelectStmt   ::= "select" ( SelectCase )* ( TimeoutCase )? ( ElseCase )? "end"

SelectCase   ::= "case" ( Ident "=" )? ChannelOp Block
TimeoutCase  ::= "timeout" Expr Block
ElseCase     ::= "else" Block

ChannelOp    ::= Expr ( ".recv()" | ".send(" Expr ")" | "<-" Expr )
```
