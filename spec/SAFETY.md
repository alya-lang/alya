# Alya Safety Case

> Which bug classes cannot happen in Alya programs, and the gate that
> proves each one. Rule: no claim without a gate — a sentence added
> here without a linked test or fixture is reverted on sight.
> (alya-lang/alya#198.)

## 1. Use-after-free: impossible through owned/borrowed boundaries

Every heap share follows retain-before-release pairing
(`spec/RC_OWNERSHIP.md`): alias throws retain at the throw site,
borrowed returns retain before scope cleanup, containers retain on
store and cascade on free, spawned fibers/threads own their func/arg
across the spawner frame.

- Fixtures `11_map_index_literal_retain`, `12_tuple_struct_return_alias`,
  `13_return_index_retain`, `35_return_borrowed_alias` (each a
  distilled Lib UAF), `69_spawn_closure_capture` (spawn ownership).
- `test_e2e_mem_trace_catch_struct_freed`,
  `test_e2e_mem_trace_catch_escapes_freed` (catch shares).

## 2. Double-free: impossible

Releases are single-shot by construction: catch-end drops the slot
once (rethrow/return paths jump away), scope restores null rebinding
slots, and every release is probed (non-heap words skip).

- `test_e2e_mem_trace_if_rebind_freed` (the #80 rebind case),
  `test_e2e_mem_trace_catch_struct_freed`.

## 3. Null dereference: traps, never faults silently

Null is a first-class kind (`KIND_NULL`), distinct from 0, and null
misuse traps as catchable errors instead of faulting: string builtins
on null return `""`, `len(null)` is 0, provably-foreign calls trap.

- Fixtures `48_trapped_type_errors`, `55_dynamic_is_not_and_throw`,
  `56_null_zero_distinction`, `58_kind_null_transport`,
  `61_null_tag_reassign`.

## 4. Out-of-bounds access: traps, never reads garbage

Index stores keep a single unsigned bounds check (negative and
overflow in one cycle); violations raise catchable
`index out of bounds`, including empty-pop.

- `test_e2e_array_bounds_catch`, `test_e2e_arrays`
  (`caught empty pop: index out of bounds`).

## 5. Divide-by-zero: traps

Integer `/` and `%` by zero raise catchable
`Runtime error: division by zero` (hardware trap or guarded branch
per Chapter 03), never wrap or fault the process.

- `test_e2e_div_by_zero_protection`,
  `test_e2e_modulo_by_zero_protection`, `test_e2e_try_catch_basic`.

## 6. Static mistypes: check-time errors or dynamic dispatch, never silent miscompiles

Where one static claim cannot serve every value, reads demote to
runtime tag dispatch instead of guessing: mixed-kind fields/variables
(`struct_field_mixed`), contested map keys, bare-name collisions,
unrelated overloads (identity fallback), builtin-name hijacks, and
dynamically-tagged catch bindings.

- Fixtures `22_operator_dispatch`, `25_bare_collision`,
  `37_mixed_kind_field`, `38_mixed_kind_variable`,
  `41_mixed_kind_map_key`, `42_len_builtin_method_shadow`,
  `43_len_dynamic_receiver`, `70_catch_escape_roundtrip`.
- Negative suite (`spec/negative/`, 42 files): mistyped programs in
  every chapter fail at lex/parse/check with the contracted message —
  including loud rejections for unsound shapes (`07_index_callee_call`,
  `07_call_result_call`, `07_closure_catch_capture`).

## 7. Concurrency: no use-after-spawn, no silent captures

`spawn`/thread procs own heap func/arg and release exactly once on
completion; parameterized spawn closures are check-time errors (no
syntax can supply them, so no capture can dangle).

- Fixture `69_spawn_closure_capture`; negative
  `07_closure_catch_capture`, `07_closure_loop_capture`.
- Not claimed: data-race freedom. Fibers are cooperative; OS threads
  share memory under the usual rules.

## 8. Backstop: the fuzzer

The seeded crash fuzzer (`tests/fuzz_crash.rs`) generates valid
programs across the struct/operator/try/fiber/dynamic/float/channel
axes and requires exit 0 **plus** the `done` marker on stdout: N=40
in every CI run, N=400 nightly on all six platforms. Anything that
crashes, hangs, or silently truncates output is a finding with the
full repro attached.

## 9. Explicit non-goals (what this page does NOT claim)

- Cyclic references leak by design (no cycle collector; the trace
  report flags them — `RC_OWNERSHIP.md` §7).
- Bounded one-share residuals exist and are documented, not gated:
  fresh argument temps, var-less catches, returns inside `catch`.
- `volatile_*` is compiler-level volatility only, not a hardware
  fence (Chapter 16 §1.7). No inline asm, no interrupts.
- Bare `say` of a struct-typed dynamic prints by static typing
  (known cosmetic residual, §8 of the #186 notes).
