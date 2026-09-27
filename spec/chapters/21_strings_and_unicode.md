# Chapter 21: Strings, Unicode & Runes

## 1. Specification Rules

### 1.1 Overview
Strings in Alya are immutable, UTF-8 encoded sequences of bytes managed via ARC. The language provides first-class support for Unicode while maintaining bare-metal string throughput.

### 1.2 The `rune` (Character) Type
- Represents a single 32-bit Unicode Scalar Value (`u32`).
- Declared using single quotes: `'A'`, `'ç'`, `'木'`, `'🚀'`.
- Supports direct arithmetic and integer conversion: `int('A') == 65`.

### 1.3 String Internal Representation
A string struct contains:
1. `ptr`: Pointer to heap-allocated UTF-8 byte payload.
2. `byte_len`: Total length in bytes.
3. `char_len`: Total count of Unicode codepoints (runes), calculated lazily or cached.
4. `arc_ref`: Reference counter for memory reclamation.

### 1.4 Iteration Semantics
- **Codepoint Iteration (`for char in text`)**: Automatically decodes multi-byte UTF-8 sequences and yields `rune` values.
- **Raw Byte Iteration (`for b in text.bytes()`)**: Yields individual `u8` bytes for high-speed binary parsing.

### 1.5 Substrings & Slicing
- Slicing `text[0..5]` extracts the specified range of codepoints.
- Substring slices are valid UTF-8 strings. Slicing in the middle of a multi-byte sequence automatically adjusts to the nearest codepoint boundary or raises an exception in strict mode.

### 1.6 Formatted String Specifiers (`f"..."`)
Alya string interpolation supports formatting specifiers via the `:spec` suffix:
- **Precision**: `f"{pi:.2f}"` -> `"3.14"`
- **Padding & Width**: `f"{id:05}"` -> `"00042"`
- **Alignment**: `f"{name:>10}"` (right align), `f"{name:<10}"` (left align)
- **Radix Conversion**: `f"{val:#x}"` (hex `0xff`), `f"{val:#b}"` (binary `0b1010`)

### 1.7 Interpolation Brace Semantics
- `{expr}` interpolates the value of `expr`; any expression (calls, indexing,
  field access, format specs) may appear inside the hole.
- `{{` and `}}` are escapes rendering single literal braces: `"{{}}"` -> `"{}"`.
- A lone `{` or `}` with no valid hole stays literal: `"{"` -> `"{"`, `"{}"` -> `"{}"`.
- Per-hole fallback: when a `{...}` region does not parse as an expression,
  only its braces stay literal while valid holes nested inside still
  interpolate: `"{\"level\":\"{lvl}\"}"` with `lvl = "INFO"` renders
  `"{"level":"INFO"}"`. The region terminator is preserved verbatim, so a
  following literal `}` is never merged into an escape pair.

### 1.8 Plain vs `f` Strings
- Plain `"..."`, `f"..."`, and `r"..."` literals all interpolate `{holes}`
  identically for simple expressions: `say "Player {player}"` and
  `say f"Player {player}"` both render `"Player Taiizor"`.
- The `f` prefix switches the lexer to brace-depth-aware scanning, so holes
  may contain string literals with quotes: `f"{m["key"]}"` renders the
  looked-up value, while the plain form terminates the literal at the inner
  quote and miscompiles.
- Convention: use the `f` prefix whenever a hole contains quotes or a
  `:spec` suffix; plain strings suffice for simple variable and field holes.

### 1.9 Text-Only Contract (Binary Data)
- Strings are UTF-8 **text**: the runtime representation is
  null-terminated, so embedded `\0` bytes truncate values
  (`"a\0b"` behaves as `"a"`; see `spec/syntax/lexical.alya` §8).
- Binary protocols must not route frames through `string`. Use the
  explicit-length byte path instead: `b"..."` literals and integer
  arrays (`u8[]`) preserve every byte including zeros, with
  `std/net` `send_bytes`/`recv_bytes` for transport and `std/str`
  `bytes_from_string` / `bytes_to_hex` / `hex_to_bytes` /
  `bytes_equal` for conversion and comparison.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
RuneLit        ::= "'" ( [^'\\] | EscapeSeq ) "'"

FormatSpec     ::= ":" [0-9]* ( "." [0-9]+ )? [a-zA-Z%]?
```
