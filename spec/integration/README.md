# Alya Language Specification: Integration Fixtures
#
# This directory complements `spec/syntax/` (core language, single-file,
# happy-path). Files here are minimal reproducers distilled from real
# breakages found while building `Lib/*` packages and `App/*` binaries.
#
# Constitution:
# 1. Every file maps to a real Lib/App failure mode, not a synthetic grammar case.
# 2. Entry points are numbered (`01_*`, `02_*_main`, `03_*`). Helper modules
#    (e.g. `02_facade_types.alya`) are NOT entries; they exist to exercise
#    multi-file relative imports and `pub` visibility.
# 3. Every compiler fix that unbreaks a Lib/App pattern MUST add or extend a
#    fixture here. No fixture-less compiler fixes.
# 4. Harness: `tests/integration_spec_tests.rs` (lex + parse + resolve +
#    5-target codegen + execution). Helpers are excluded from the entry list.
#
# | Fixture | Source pattern | Failure mode it guards |
# |---|---|---|
# | `01_toml_mini.alya` | `Lib/toml/src/parser.alya` + `types.alya` | recursion + tuple destructure + map/array mutation + `continue`/`break` in `while` |
# | `02_facade_types.alya` + `02_facade_main.alya` | `Lib/template/src/*`, `Lib/toml/src/lib.alya`, `App/vpn/src/main.alya` | relative `import "./x"` + `as` alias + `from ... import` + `pub`/private boundary |
# | `03_ffi_struct.alya` | `Lib/sqlite/src/ffi.alya`, `spec/syntax/ffi.alya` + `structs.alya` | `extern "C"` + `@repr(C)` struct + safe wrapper + tuple return + null safety |
# | `04_float_return_index.alya` | `Lib/math/src/matrix.alya` `_mat_at` | `-> float` function returning `w[idx]` must sync the float return register |
# | `05_push_float_array.alya` | `Lib/math/src/matrix.alya` builders | `push`-built float arrays must read back floats, incl. across a builder return |
# | `06_tag_dispatch.alya` | `Lib/math` mixed builders + `kind()` | per-element kind dispatch in `say` + `is int`/`is float` on element reads |
# | `07_shared_helper_is_checks.alya` | `Lib/csv` + `Lib/json` polymorphic helpers | fold `is array`/`is map` only on whole-caller proof |
# | `08_struct_float_call_arg.alya` | `Lib/math` complex arithmetic on x86 | struct float fields passed to float params must push full 8-byte double |
# | `09_private_float_param.alya` | `Lib/math/src/matrix.alya` `_mat_absf` & `_mat_at` | mangled private float helpers push 8-byte doubles & index return preserves xmm0 |
# | `10_int_store_no_retain.alya` | `Src/benchmarks/cross_lang/algorithms/quicksort.alya` fill + swap | `push`/index stores of proven ints skip `rc_retain`; aliased heap keeps it |
# | `11_map_index_literal_retain.alya` | `Lib/crypto/src/rsa.alya` `rsa_try_pubkey_at` | map literal constructing `{ "k": map["v"] }` retains value so dropping source map doesn't cause UAF |
# | `12_tuple_struct_return_alias.alya` | `Lib/xml/src/parser.alya` + `Lib/xml/src/lib.alya` | tuple return with struct elements infers struct type for destructured bindings; borrowed-struct call into reassigned variable retains (no UAF) |
# | `13_return_index_retain.alya` | `Lib/tls/src/core/record.alya` (`tls_alert_bytes`) | `return rec["bytes"]` retains the indexed heap value before the container is released in epilogue cleanup |
# | `14_method_delegation.alya` | `Lib/regex/src/lib.alya` `Regex.is_match` -> `is_match` | bare same-name calls in methods prefer the free fn over self-recursion |
# | `15_forwarding_param_veto.alya` | `Lib/cache` (`store_set` via untyped `set` wrapper) | same-arg forward of a vetoed dynamic param records transitive `fn_param_nonstr` veto, keeping the retain for dynamic values |
# | `16_bit_int_push.alya` | `Lib/crypto` SHA-256 message schedule | bitwise builtins are int->int word ops; `let`-bound results earn `var_is_int` so stores skip `rc_retain` (faults on large aligned ints) |
# | `17_return_null_index_tag.alya` | `Lib/cache` `store_get` | `return null` materializes KIND_UNKNOWN; return retain spills the tag across `rc_retain` (clobbers rdx/w1) |
# | `18_dynamic_parse_shapes.alya` | `Lib/json` `_parser_parse_val` | literal non-string `return` records `fn_ret_nonstr` veto gating `fn_ret_str`, keeping mixed-shape functions dynamic |
# | `19_null_string_ops.alya` | `App/vpn/tests/test_config.alya` (wrong dir) | `lower`/`upper`/`trim`/`substring` on `null` return `""` instead of faulting; `len(null)` is 0 |
# | `20_implicit_float_convert.alya` | `std/math` `dot_product` | unknown-kind ints into float positions (`: float`, float params/returns/fields) convert instead of int-bits-as-double |
# | `21_dynamic_index_assign.alya` | `std/collections` `set_add` | `s[k] = v` on statically-unknown container dispatches on the header at runtime; only exact evidence earns the static fast path |
# | `22_operator_dispatch.alya` | struct `==` without overload (issue #85) | `a == b` on a struct without its own overload uses identity, never an unrelated type's overload |
# | `23_bare_module_calls.alya` | `std/str` via bare `import "std/str"` (issue #86) | unaliased import namespaces under basename (Ch.11 §1.3); `str.f(x)` resolves for all arg shapes; local `str` keeps UFCS |
# | `24_nested_functions.alya` | named nested `function` (alya-lang/alya#99) | nested defs hoist with lexical scope (`outer$inner`); sibling/recursive calls resolve inward; no capture (check-time error) |
# | `25_bare_collision.alya` | `inner` + `outer__inner` bare collision (alya-lang/alya#101) | qualified defs never seed bare markers; qualified calls never consult them (all marker families) |
# | `26_dynamic_split_key.alya` | mustache `context_find` (alya-lang/alya#102) | `split` on proven string classifies pieces as strings; map read keyed by split piece routes to lookup |
# | `27_method_field_map.alya` | `Lib/cache` `store_has` (alya-lang/alya#105) | bare UFCS param evidence under the bare name must still derive the exact-spelled marker for map lookup |
# | `28_map_float_array.alya` | `Lib/math` `mat_solve` (alya-lang/alya#106) | dynamic-array element reads dispatch on the per-slot tag instead of assuming int |
# | `29_mixed_push_reads.alya` | `Lib/yaml` `flow_parse_value` (alya-lang/alya#107) | return-position vetoes cover builtin int/float calls, nulls, transitive callees; string-array positives veto-gated |
# | `30_struct_field_bigint.alya` | `Lib/gui` `image_set_px`/`image_get_px` (alya-lang/alya#108) | `return field[i]` with a stale tag still guards its return retain on the fresh slot tag |
# | `31_modjoin_helper.alya` + `31_midjoin_helper.alya` + `31_aliased_bare_builtin.alya` | `Lib/url` `join` vs builtin via `Lib/http` (alya-lang/alya#112) | aliased imports expose names only via `m::`; bare calls keep builtin resolution, incl. modules that import nothing |
# | `32_fnfield_helper.alya` + `32_fnfield_match.alya` | `Lib/http` `router_match` (alya-lang/alya#113) | dynamic fn store + string literal in same-named fields must not set bare `struct_field_str`; stored fn stays callable |
# | `33_uuid_loop_flat_memory.alya` | `Lib/uuid` `v4()` loop (alya-lang/alya#116) | self-accumulating `res += ...` reassignments must not dup transients into the stable store; loop memory stays flat |
# | `34_method_self_shadow_builtin.alya` | `spike/selfhost/lexer.alya` `Lexer.read_string` vs `fs.read_string` | unannotated `self` in methods infers struct type to prevent fallback to global same-name builtin |
# | `35_return_borrowed_alias.alya` | `archive` entry loop via pass-through helper (alya-lang/alya#125) | `return data` of an untyped param retains the borrowed alias so loop-end releases never reclaim the shared referent |
# | `36_catch_struct_message.alya` | `std/process` `ProcessError` handling (alya-lang/alya#126) | `.message` on a catch binding prefers the throw-recorded text over the struct-pointer identity, with value fallback for string throws |
# | `37_mixed_kind_field.alya` | `http` `SseEvent.data` vs `event` `TcpStream.data` (alya-lang/alya#131) | reachable string-`data` literal must not flip untyped `.data` reads on array holders; mixed fields demote to dynamic reads |
# | `38_mixed_kind_variable.alya` | `http` `WebSocketFrame.payload` text vs bytes paths (alya-lang/alya#132) | every construction site passing a *variable* still earns both kind markers, so the contradiction demotes reads, params, and `say` to dynamic dispatch; no truncation, no segfault |
# | `39_map_method_call.alya` | struct method on map-stored value (alya-lang/alya#133) | unknown-receiver method call binds the single program-wide same-named struct method (arity-checked); zero/several candidates keep the legacy bare call |
#
# ## Known collision (deliberate rename)
#
# `Lib/toml` names its helpers `is_space` / `is_whitespace` / `_map_has`.
# The fixture uses `mini_is_space` / `mini_is_blank` / `mini_map_has` instead:
# user functions are emitted as `fn_<name>` labels, which collides with the
# runtime's own `fn_is_space` / `fn_is_whitespace` in a flat single-file build
# (`symbol fn_is_space is already defined` at link time). Inside real packages
# the collision is hidden by package symbol mangling. Fix the `fn_*` namespace
# collision in codegen, then rename the fixture helpers back to Lib names.
