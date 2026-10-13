# Alya RC Ownership Contract

> Which share each boundary retains and releases. Violations are
> use-after-free or leaks; when in doubt, leak (never fault), then
> file the leak with a `--mem-trace` report.

Heap values carry a refcount at -8 (magic at -16: `0x5A110001`
array, `002` map, `003` struct, `005` closure; `004` fat pointers
are excluded from retain/release throughout). Immortal regions
(rodata, `str_buf`, `str_stable`) never refcount: `fn_rc_retain` /
`fn_rc_release` (probed) skip small ints, out-of-range words,
immortal regions, and unmapped memory (alya-lang/alya#117), so they
are safe to call on any value, including code pointers.

## Rules by boundary

1. **Locals.** Creation owns (+1). Scope-end cleanup releases every
   heap-typed slot. Slots without a heap `VarType` are skipped —
   including `let`-bound closures (known gap, alya-lang/alya#183).
2. **Params.** Proven-heap params take an entry retain; scope release
   drops it as a balanced pair. `nonescaping_params` (borrowed for
   the whole call) skip both. Struct/interface params carry no
   entry retain; their lifetime is the caller's.
3. **Calls.** Argument temps are borrowed, not shared: the caller
   pushes values and forgets them (transfer for fresh values, whose
   creation share moves to the callee). The callee's own
   entry-retain/scope-release pair balances its side. Consequence:
   fresh values passed as arguments leak exactly one share today
   (pre-existing; e.g. `show(P { a: 5 })` reports 1 alloc,
   0 deallocs) — consistent everywhere, not specific to any call.
4. **Returns.** Borrowed values (index/field reads, aliasing
   identifiers) are retained before scope cleanup
   (alya-lang/alya#125); owned slots transfer via `skip_offset`
   (retaining again would leak).
5. **Closures (alya-lang/alya#100).** The chain array retains each env
   struct; the closure object takes the chain by transfer. Freeing a
   closure releases its chain (and transitively the holders).
   Direct calls append a caller-built chain (released post-call);
   indirect calls resolve code+chain from the object.
6. **Fibers and OS threads (alya-lang/alya#181).** `spawn` /
   `thread_spawn` retain heap func/arg (the fiber/thread outlives
   the spawner frame; drain/join run later). The trampoline/thread
   proc releases both exactly once on completion. Entry points are
   closure-aware: closure objects run their code with the stored
   chain as the single argument; anything else keeps the legacy
   direct call. `spawn` closures with user parameters are a
   check-time error (no syntax can supply them).
7. **Containers.** Element/field stores retain; frees cascade
   (array/map/struct inner then outer). Cyclic references leak by
   design (no cycle collector); the trace report flags them.
8. **Throw/catch.** The thrown value travels creation → throw temp →
   thread-block record (raw) → catch-var slot. The catch slot is
   forgotten at `end_label` and never scope-released: struct throws
   leak one share each (known gap, alya-lang/alya#184). Strings are
   unaffected (immortal).

## Checker/codegen view split (alya-lang/alya#182)

The type checker sees the full program; codegen runs on the
DCE-pruned one. A struct referenced only by untyped field access
may be pruned while dynamic code still names the field — so
"unknown field" verdicts (and all loudness) live in the checker.
Codegen keeps the legacy index-0 fallback with a rationale comment;
a codegen-side panic there is a false positive (it fired on the
null-field-trap test during development of this rule).

## Leak gate status

`--mem-trace` + `STATUS: [OK] Clean execution` family in
`tests/e2e_system.rs` gates locals/params/returns/containers.
Spawn/closure/catch flows cannot join the zero-leak gate until
#183 and #184 land; the crash fuzzer (`tests/fuzz_crash.rs`)
covers them with an exit-code oracle instead.
