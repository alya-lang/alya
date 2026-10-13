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
   heap-typed slot. `let`-bound closures additionally track a
   `closure_owned` share (transfer for direct materializations,
   retain for copies) dropped by scope cleanup
   (alya-lang/alya#183); single-use alias-forwarded direct calls
   keep a bounded holder-plus-chain residual.
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
8. **Throw/catch (alya-lang/alya#184).** The thrown value travels
    creation → throw temp → thread-block record (raw) → catch-var
    slot, and the slot owns one share: fresh throws (literals,
    calls) transfer their creation share (abandoned on the handler
    jump), alias throws (`Identifier` of a live heap slot, field /
    index reads, ambiguous composites) add a throw-site retain, and
    catch-var rethrows transfer the share already in flight.
    Catch-end drops the slot once via probed release (strings and
    statics skipped). Rethrow/return paths jump away, so the drop
    only runs when the exception is consumed. Slot escapes are all
    borrows (catch vars type as `StringOffset`, so no store retains
    them); var-less catches and returns inside `catch` keep a
    bounded one-share residual.

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
`tests/e2e_system.rs` gates locals/params/returns/containers,
closure slots (#183), and catch bindings (#184). Bounded residuals
outside the gate (fresh argument temps, alias-forwarded direct
calls, var-less catches, returns inside `catch`); the crash fuzzer
(`tests/fuzz_crash.rs`) covers all flows with an exit-code oracle
instead.
