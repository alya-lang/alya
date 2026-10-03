# Alya Lint Rules

Reference for the rules reported by `alya lint` (and surfaced in SARIF
reports as `ruleId`). Severities: `error`, `warning`, `note` (shown as
`info` in terminal output).

## unused-var

Flags local variables that are declared but never read. Prefix intentional
unused bindings with an underscore (`_tmp`). Severity: `warning`.

## unused-param

Flags function parameters that are never used in the body. Prefix
intentional ones with an underscore. Severity: `warning`.

## unused-import

Flags imported modules or symbols that are never referenced. Severity:
`warning`.

## dead-code

Flags unreachable statements (code after `return`, `throw`, or infinite
loops). Severity: `warning`.

## idiomatic-style

Flags unidiomatic constructs and anti-patterns with a preferred
replacement. Some findings are informational suggestions (`note`).
Severity: `warning` / `note`.

## self-comparison

Flags comparisons of a value with itself (`x == x`), which are almost
always bugs. Severity: `warning`.

## constant-condition

Flags `if`/`while` conditions that are constant, making a branch dead or
infinite. Severity: `warning`.

## useless-expression

Flags expression statements whose value is discarded and which have no
side effects. Severity: `warning`.

## naming-convention

Flags declarations that do not follow `snake_case` naming (types keep
their own convention). Severity: `note`.

## dynamic-is-float

Flags `is float` (and `is not float`) applied to values whose kind is
not statically provable (map/array reads, untyped calls, bare
identifiers). A float sharing an int's bit pattern is
indistinguishable at runtime, so such checks are best-effort and may
read `not float` for a real float. Annotate the source with
`-> float` or check a literal or statically-proven value instead.
Float literals and same-file `-> float` calls are exact and stay
silent. Severity: `warning`.

## boolean-literals

Flags `1`/`0` used where a boolean belongs: `return 0`/`1` inside
predicate-named functions (or `-> bool` ones), `pred(...) == 1`
comparisons, and `-> int` on functions whose every return is
boolean-shaped. All findings are auto-fixable (`alya lint --fix`)
to `true`/`false` and `-> bool`; the encoding is identical, so
behavior never changes. Severity: `note` (never fails `--check`).

## method-self-recursion

Flags a bare `name(self, ...)` call inside the same-named method whose
arguments are all identifiers: UFCS resolves it back into the method
itself instead of a same-named free function, so it re-enters the
same body with identical values and never terminates. Qualify the
call or rename one side. Genuine recursion (changed or computed
arguments) and explicitly qualified calls stay silent. When a
same-file free function with that name exists, the call delegates
(via the compiler guard) instead of hanging, so the finding is
informational fragility rather than a warning.
Severity: `warning` (no free target) / `note` (delegation).

## duplicate-map-key

Flags a repeated literal key in a `{...}` literal. The last value
wins at runtime, so earlier entries are dead weight and almost always
a copy-paste bug. The earlier entry carries an auto-fix deleting it
through its trailing comma. Severity: `warning`.

## null-equality

Flags `x == null` / `x != null`, which spell the same check as
`x is null` / `x is not null` less idiomatically. Auto-fixable
(`alya lint --fix`). Severity: `note` (never fails `--check`).

## compound-assign

Flags `x = x + 1` (and `-`, `*`, `/`, `%`) on plain-identifier or
dotted-field targets, which reads better as `x += 1`. Call arguments,
indexes, struct literals, and `let` declarations are untouched.
Auto-fixable (`alya lint --fix`). Severity: `note` (never fails
`--check`).

## float-equality

Flags exact `==` / `!=` against a float literal, which is fragile;
an epsilon comparison is usually intended. No auto-fix (the right
epsilon is contextual). Severity: `note` (never fails `--check`).
