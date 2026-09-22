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
