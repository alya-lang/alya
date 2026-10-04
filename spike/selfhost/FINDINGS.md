# Spike: Alya lexer in Alya (self-hosting feasibility, slice 1)

Branch: `spike/alya-lexer`. Artifacts: `spike/selfhost/lexer.alya` (~1100 lines,
stdlib-only), `spike/selfhost/diff_lex.py` (differential harness),
`spike/selfhost/arc_segfault_repro.alya` (30-line runtime crash repro).

Method: canonical dump `line|col|kind|norm` from both lexers over
`spec/syntax/*.alya` + `stdlib/*.alya` + `spec/negative` lex cases.
Numbers/floats/strings compared by VALUE (decimal, bit-equal doubles,
JSON-decoded unicode); error files compare failure position.

## Result: fidelity proven where the runtime holds

- 44/44 corpus files token-identical (63,970 tokens, zero diffs; was 43/44
  with B1 crashing on `stdlib/test.alya` — fixed with the stable string
  store plus B2–B4 runtime fixes, now on develop).
  Re-verified 2026-10-04 on develop (`alya 0.0.20`): `stdlib/test.alya`
  lexes 4208 tokens deterministically across runs (stats line excluded)
  and the B1 repro exits 0 printing 40335.
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

## Verdict: DONE — all blockers closed on develop (re-verified 2026-10-04)
(B1 fixed; B2 fixed with chr() single-byte legacy kept by design; B3 fixed;
B4 fixed via shortest-round-trip printing)

### B1. Segfault-class memory bug (FIXED on develop, re-verified 2026-10-04)
`arc_segfault_repro.alya`: 2000 function-built strings → array → read+concat
previously segfaulted (`0xC0000005`), silently. Today it exits 0 printing
40335, and the lexer run on `stdlib/test.alya` is deterministic across runs
(same 4208 tokens, stats line excluded). No filing needed unless it
regresses; keep the repro as a regression guard.

### B2. Unicode runtime (FIXED on develop, re-verified 2026-10-04)
- FIXED: `chr(cp)` works for cp > 127 (`chr(287)` → `ğ`, `chr(128512)` →
  `😀`); `u64` display/comparison/remainder are exact (`u64::MAX` prints
  `18446744073709551615`, `MAX > 5` is true, `MAX % 10` is `5`); the
  `runes()`/`ord()` codepoint path works (`tests/e2e_strings.rs`:
  `test_e2e_ord_utf8_decode`, `test_e2e_runes_unicode_iteration`).
  The spike's `u64_lt/gt` and `u64_divmod10` helpers are now redundant
  for new code but harmless to keep in the frozen spike.
- FIXED: `for ch in s` yields rune codepoints (`typeof(ch)` is `int`,
  spec ch. 21 §1.4). New `fn_string_codepoints` runtime (x64+arm64) feeds
  the foreach temp; loop vars type as `Number`/`Rune`. The old 1-char
  string behavior contradicted ch. 21 (and `loops.alya` §9, which is
  rewritten to the rune form).
- KEPT BY DESIGN: `chr(1..255)` still emits single raw bytes (byte-oriented
  packages rely on it: `stdlib/json.alya`, `test_e2e_chr_unicode`). Use
  `char_at()` (codepoint-indexed) to render glyphs back from a loop.
- VERIFIED: `char_at` is codepoint-indexed (`char_at("héllo", 1)` is `é`;
  `tests/e2e_strings.rs: test_e2e_char_at_unicode`), while `len()` stays
  bytes. 0-based indexing.

### B3. Silent failures (FIXED, re-verified 2026-10-04)
- FIXED: module-global write from a function now works (probe prints 41,
  exit 0) — state no longer has to move into a struct for this reason.
- FIXED: every binary installs a crash handler first thing in `main`
  (`alya_crash_init`; `src/codegen/runtime/{x64,arm64}/crash.rs`). A real
  segfault now prints `Alya runtime crash: <fault> (<signal|code>)` to
  stderr and exits 134 — verified for SIGSEGV on Windows x64 (vectored
  handler, `0xC0000005`), Linux x64 and Linux arm64 (`signal()` path).
  Unix `alya run` already named the killing signal; Windows `alya run`
  now names the NTSTATUS too (`src/driver/runner.rs`).
  `tests/e2e_system.rs: test_e2e_crash_diagnostic_segfault` guards it.
- DOCUMENTED LIMITS: message names the fault, not the source location
  (no debug tables yet); stack-overflow delivery is best-effort (the
  faulting stack may be unusable); `_exit(134)` skips the mem-trace
  report on crash.

### B4. Float printing (FIXED: shortest-round-trip, re-verified 2026-10-04)
`say`/`str()`/interpolation/concat render the shortest string that parses
back to the input bits (`snprintf` prec 6→17 + `strtod` check inside
`fn_str_from_float`, x64+arm64). `0.1+0.2` prints `0.30000000000000004`;
short values keep their shape (`3.14`, `1`, `3e+08`).
`tests/e2e_system.rs: test_e2e_float_shortest_round_trip` guards it.
Spec fallout, all intended: `lexical.alya` golden now shows full `Pi`
digits and the true `150.75*1.05 = 158.2875`; `ffi.alya` trig demo uses
explicit `{c_val:.6f}` (exact doubles vary 1 ulp across libms).

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
  enough that lexer speed is NOT the self-host risk. The former B1–B3
  risks are closed; remaining self-host work is the parser slice.

## Other verified language facts (useful for the parser step)
- Struct methods mutate state reliably; `u64` arithmetic wraps mod 2^64;
  `int(str)` parses; `os.cli_args`/`exit_process`/`fs.read_string`/
  `bytes_from_string` solid ONLY via aliased imports with variables
  (bare-import + temporary args corrupt — see below); map `in` + index
  works; string `+` concat, slicing, `len` (bytes) all as spec'd.
- Plain `"..."` literals interpolate `{holes}` AND decode `{{`/`}}` — spike
  source must avoid runtime text in f-strings and literal `{{`.
- `chr()` works past ASCII (B2 fixed); `char_at` is codepoint-indexed
  (verified); `chr(1..255)` stays single-byte by design (byte packages
  rely on it).

## Recommended next steps
1. ~~File B1 with `arc_segfault_repro.alya`~~ DONE (fixed on develop;
   keep the repro as a regression guard, do not delete).
2. ~~Fix the `for-in yields string, not rune` gap~~ DONE
   (`fn_string_codepoints`, x64+arm64; `loops.alya` §9 rewritten).
3. ~~Give runtime crashes a diagnostic~~ DONE (`alya_crash_init` +
   per-arch handlers, exit 134; `test_e2e_crash_diagnostic_segfault`).
4. ~~Round-trip float printing~~ DONE (shortest-round-trip in
   `fn_str_from_float`; `test_e2e_float_shortest_round_trip`).
5. Parser slice (IN PROGRESS, 2026-10-04): `spike/selfhost/parser.alya`
   (~6300 lines, stdlib-only) + `diff_parse.py` over `alya ast`
   (new `src/driver/ast_sexpr.rs` canonical dump; `parse_raw()` split
   so the dump is pre-expansion). Result: 39/44 corpus files
   token-identical S-expr (0 diffs), 5 SKIP (closures/comprehensions),
   0 fail; 4/4 parse-negative files agree.
   Scope: full Pratt L1–L16, all statements, when-subset
   (Exact/Relational/Range/Type) + when-destructuring (tuple/variant
   with bindings/substitution), f-string holes with hole mini-lexer,
   comptime const-folding, @cfg eval (host os/arch, dev debug=true).
   Deferred: closures, comprehensions, select.
6. Next: slice 3 (spill templates for closures/comprehensions), then
   codegen port investigation.

## Parser-slice runtime facts (compiler bugs filed separately)
- Bare `import "std/str"` + temporary argument (literal/concat/call
  result) returns garbage; `as str` alias is correct for all shapes.
  The spike uses the alias (see header note); variables are unaffected.
- Mixed-type arrays (`[bool, string]`, `[array, int, int]`) corrupt tag
  dispatch on read (`say arr[i]` segfaults while `let x = arr[i]`
  works). The spike uses structs (HoleRes/ScanRes/DumpRes/HoleTok/
  WhenArm) for all heterogeneous results.
- Struct literals silently ignore unknown fields (`WhenArm { ..., val: }`
  with field `v` compiled and ran with `v == ""`). Cost real debugging
  time here; a compile-time error would be strictly better.
- `say arr[i]` on tag-carrying dynamic reads vs plain loads behave
  differently (see above); loop-var and temp-array handling is sound
  otherwise (B1-era fixes hold).
