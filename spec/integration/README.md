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
# | `14_method_delegation.alya` | `Lib/regex/src/lib.alya` `Regex.is_match` -> `is_match` | bare same-name calls in methods prefer the free fn over self-recursion |
# | `11_map_index_literal_retain.alya` | `Lib/crypto/src/rsa.alya` `rsa_try_pubkey_at` | map literal constructing `{ "k": map["v"] }` retains value so dropping source map doesn't cause UAF |
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
