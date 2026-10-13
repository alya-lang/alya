# Alya Module Naming & Cross-Module Resolution

> How names survive imports, and in what order callables and types
> resolve. Most "works same-file, breaks imported" bugs are a
> spelling split across this boundary (alya-lang/alya#48, #86, #101,
> #112, #147, #159, #161, #180).

## What aliasing rewrites

`import "./lib.alya" as pp` prefixes every **callable/value**
definition with `pp::` (functions, consts, top-level lets, including
`Type__method` and `Type__operator+` spellings). It does **not**
rewrite:

- `struct` / `enum` definition names (they keep their bare
  spelling),
- type annotations (`self: Pair`, `-> Pair`),
- `StructInit` spellings (`Pair { ... }`).

So after an aliased import the program holds a bare `Pair` struct
next to a qualified `pp::Pair__operator+` method. Any lookup that
assumes one spelling for both misses.

## Callable resolution order

1. **Exact** spelling (`pp::Pair__operator+`).
2. **Bare** spelling (`Pair__operator+`), only when the bare name
   has a single owner (`may_record_bare_vars`, alya-lang/alya#101);
   ambiguous bares never seed or consult bare markers.
3. **Separator-normalized exact** (`::` vs `__` unified,
   alya-lang/alya#85): same type, different separator spelling.
4. **Unique type-qualified suffix** (`find_operator_overload`,
   alya-lang/alya#180): normalized names ending in
   `__{bare_type}__operator{op}` with exactly one distinct match.
   The suffix keeps the type, so another type's overload can never
   match (#85). Zero or ambiguous matches keep the legacy path.

Bare `name(...)` calls that could be either globals or UFCS methods
are never alias-prefixed at parse time (`is_method_ambiguous`);
codegen disambiguates by receiver (struct-qualified markers first).

## Type resolution order

Struct references resolve exact, then bare. Two different structs
sharing a bare name across modules collide in the shared table
(last insert wins) — genuinely ambiguous layouts panic at check
time with both candidates listed; do not guess.

## Import forms

- Whole-module (`import "x"` / `import "x" as a`): all public
  symbols (or alias-prefixed copies) merge into the program.
- Selective (`from "x" import a, b as c`): only listed symbols;
  definitions are cloned under the alias where given.
- Segregated packages (`alya-lang/*` with majors) compile under a
  `_Alya_{pkg}_v{major}` prefix with call rewrites.
- Unaliased stdlib imports never seed the importer's alias
  namespace: std names stay bare program-wide, so an on-disk
  `stdlib/` shadow behaves like the embedded copy
  (alya-lang/alya#147).
- Assembly symbols mangle `::` to `__` (`mangle_symbol_name`;
  Darwin `_` prefix excepted for internal runtime labels).

## Assembly identity

`ctx.functions` holds both the source spelling
(`pp::Pair__operator+`) and the mangled label
(`pp__Pair__operator_add_`). Operator methods are always live
for DCE (`starts_with("operator")`); all `__`/`::` variants resolve.
