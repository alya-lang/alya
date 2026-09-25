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
