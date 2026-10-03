# Spike: Alya lexer in Alya (self-hosting feasibility, slice 1)

Branch: `spike/alya-lexer`. Artifacts: `spike/selfhost/lexer.alya` (~1100 lines,
stdlib-only), `spike/selfhost/diff_lex.py` (differential harness),
`spike/selfhost/arc_segfault_repro.alya` (30-line runtime crash repro).

Method: canonical dump `line|col|kind|norm` from both lexers over
`spec/syntax/*.alya` + `stdlib/*.alya` + `spec/negative` lex cases.
Numbers/floats/strings compared by VALUE (decimal, bit-equal doubles,
JSON-decoded unicode); error files compare failure position.

## Result: fidelity proven where the runtime holds

- 43/44 corpus files token-identical (59,738 tokens, zero diffs). The single
  failure is B1 crashing on `stdlib/test.alya` — no fidelity signal.
- 6/6 negative lex cases agree on exact error position.
- Matched subtleties: `f/r/b` prefixes, triple quotes, brace-depth format
  strings, `.5` leading-dot rule with prev-token condition, `&&/||/!` →
  `and/or/not` canonicalization, radix overflow errors, `\u{}` error
  attribution to string start, BOM strip, char-based columns, `fn`/`nil`
  aliases, `{{`/`}}` processing in f-strings only, raw strings keeping
  backslashes literally (JSON-escaped at dump).
- Three fidelity bugs were mine, all fixed: `\u` branch double-consuming
  `}` (Rust leaves the cursor ON it), redundant newline recount next to
  `advance()`, and non-ASCII rune over-advance in `read_rune`.

## Verdict: NOT YET — runtime blockers, not language-expressiveness blockers

### B1. Segfault-class memory bug (hard blocker, repro included)
`arc_segfault_repro.alya`: 2000 function-built strings → array → read+concat
segfaults (`0xC0000005`), silently. Isolated: array reads alone OK, fresh-built
concat alone OK, literal pushes OK — only array-read+concat at volume (≥~500
elements) dies. Larger inputs crash deterministically-ish; the lexer run on
`stdlib/test.alya` additionally shows NON-deterministic output (same input,
different bytes across runs: dangling pointers, future content, log-buffer
fragments in token slots). A compiler must be deterministic — this alone
stops any self-host step until fixed.

### B2. Unicode runtime broken on develop (fresh binary, not version lag)
- `for ch in s` yields mojibake + `int(ch)` = 0 on non-ASCII.
- `chr(cp)` broken for cp > 127; `char_at` byte-indexed.
- Workaround used: byte scan + manual UTF-8 decode (columns exact).
- `u64` arithmetic wraps mod 2^64 with exact bits, but comparisons,
  division, remainder, and display are SIGNED (`u64::MAX` prints `-1`,
  `MAX > x` is false, `MAX % 10` is `-1`). The spike carries exact helpers
  (`u64_lt/gt` via high-bit split, binary long-divmod for printing); a real
  port needs true unsigned codegen or pervasive helpers.

### B3. Silent failures (diagnostics gap)
- Runtime segfault: exit 1, zero output, no message.
- Module-global write from a function: compiles, dies silently at runtime
  (state had to move into a struct).
- For compiler development this turns every bug into a bisect session
  (this spike paid that cost twice).

### B4. Printer rounding (accommodated, not blocking)
`0.1+0.2` prints `0.3` — float->string is not round-trip. Differential design
compares float LEXEMEs by parsed bit value instead. A real self-hosted lexer
needs exact literal values; either a round-trip printer or bit-level access.

### Minor gaps (worked around)
- `Array.join` needs strings (`int[]` fails at runtime despite checking OK).
- `str_repeat` arg order differs from its doc comment (compile-time catch).
- `now_ms` is Windows-timer-quantum coarse (~15.6 ms): most files lex in
  "0 ms" (sub-quantum); precise throughput needs an ns clock.

## Throughput (rough, same ballpark)
- Rust `tokens` on 318 KB (startup included): ~216 ms.
- Alya self-time: sub-quantum (<15.6 ms) for every passing file up to
  5142 tokens / 27 KB; 3–4 K-token stdlib files read 15–16 ms (= 1 quantum).
- No precise ratio possible with the ms clock; both are comfortably fast
  enough that lexer speed is NOT the self-host risk. The risks are B1–B3.

## Other verified language facts (useful for the parser step)
- Struct methods mutate state reliably; `u64` arithmetic wraps mod 2^64;
  `int(str)` parses; `os.cli_args`/`exit_process`/`fs.read_string`/
  `bytes_from_string` all solid; map `in` + index works; string `+` concat,
  slicing, `len` (bytes) all as spec'd.
- Plain `"..."` literals interpolate `{holes}` AND decode `{{`/`}}` — spike
  source must avoid runtime text in f-strings and literal `{{`.
- `chr()`/`char_at` safe for ASCII only.

## Recommended next steps
1. File B1 with `arc_segfault_repro.alya` (needs `alya` labels; suggest
   `area: codegen` or `area: runtime` per labels.yml — owner to confirm).
2. Fix B2 (Unicode iteration/chr) — required before any parser step that
   touches strings beyond ASCII.
3. Give runtime crashes a diagnostic (even a bare trap message + location).
4. Then: parser slice (recursive descent for expressions?) reusing this
   harness pattern with AST dumps.
