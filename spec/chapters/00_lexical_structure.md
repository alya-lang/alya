# Chapter 00: Lexical Structure & Literals

## 1. Specification Rules

### 1.1 Source Text & Encoding
- Alya source files (`.alya`) are strictly encoded in **UTF-8**. Non-UTF-8 byte sequences produce a fatal lexical error.
- Line terminators can be either Unix LF (`\n`) or Windows CRLF (`\r\n`). The compiler normalizes all line endings internally.
- Whitespace consists of spaces and horizontal tabs (`\t`). Indentation is encouraged for visual clarity but blocks are delimited structurally by keywords (e.g. `end`), not whitespace indentation alone.

### 1.2 Comments
- **Single-line comments**: Begin with `#` and continue to the end of the line:
  ```alya
  # This is a single-line comment
  let x = 10 # Inline comment
  ```
- **Documentation comments (Docstrings)**: Begin with `##`. The compiler and language server (LSP) parse `##` comments immediately preceding functions, structs, interfaces, enums, or modules as rich **GitHub-Flavored Markdown**.
  ```alya
  ## Calculates the Euclidean distance between two points.
  ##
  ## ### Parameters
  ## - `p1`: The origin coordinate.
  ## - `p2`: The destination coordinate.
  ##
  ## ### Returns
  ## The calculated float distance.
  ##
  ## ### Throws
  ## `MathError` if either coordinate contains NaN.
  function distance(p1: Point, p2: Point) -> float
  ```
  - IDEs render these comments directly as rich HTML/Markdown tooltips.
  - The `alya doc` command automatically extracts these blocks to generate static documentation websites and API references.

### 1.3 Identifiers & Naming Conventions
- Identifiers begin with an ASCII letter (`a-z`, `A-Z`) or underscore (`_`), followed by any combination of letters, digits, and underscores.
- **Idiomatic Naming Conventions**:
  - Variables, parameters, and function names: `snake_case` (e.g., `user_count`, `calculate_total`)
  - Structs, Interfaces, and Enums: `PascalCase` (e.g., `HttpRequest`, `ConnectionState`)
  - Constants: `UPPER_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`, `DEFAULT_TIMEOUT`)

### 1.4 Master Reserved Keywords List
The following 44 tokens are strictly reserved keywords. They cannot be used as variable names, function names, type names, or field names:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ALYA RESERVED KEYWORDS (44)                        │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────┤
│ let             │ const           │ function        │ fn                    │
│ return          │ defer           │ if              │ then                  │
│ elif            │ else            │ when            │ is                    │
│ while           │ for             │ in              │ repeat                │
│ break           │ continue        │ struct          │ enum                  │
│ interface       │ try             │ catch           │ finally               │
│ throw           │ pub             │ import          │ as                    │
│ from            │ extern          │ spawn           │ select                │
│ assert          │ test            │ bench           │ say                   │
│ and             │ or              │ not             │ true                  │
│ false           │ null            │ self            │ weak                  │
│ comptime        │ sizeof          │ alignof         │ typeof                │
│ end             │                 │                 │                       │
└─────────────────┴─────────────────┴─────────────────┴───────────────────────┘
```

- **Contextual Keywords**: `@inline`, `@cold`, `@repr`, `@export`, `@cfg`, `@test` are reserved strictly in attribute prefix position (`@`).

### 1.5 Numeric Literals
- **Integers**:
  - Decimal: `42`, `1000`
  - Hexadecimal: `0xFF`, `0x1A4B` (prefixed with `0x` or `0X`)
  - Binary: `0b1010_0110` (prefixed with `0b` or `0B`)
  - Octal: `0o755` (prefixed with `0o` or `0O`)
  - Digit separators: Underscores `_` can be inserted anywhere inside numbers for readability (e.g. `1_000_000`, `0xFF_AA_BB`).
- **Floating-point**:
  - Decimal dot: `3.14159`, `0.5`
  - Scientific exponent: `1e-4`, `2.5e+3`, `6.022e23`

### 1.6 String Literals & Escapes
- **Standard String (`"..."`)**: Supports standard escape sequences:
  - `\n` (newline), `\r` (carriage return), `\t` (tab), `\\` (backslash), `\"` (quote), `\0` (null byte)
  - Unicode escapes: `\u{1F680}`
- **Interpolated String (`f"..."`)**: Allows embedding arbitrary expressions within `{}`:
  ```alya
  f"Hello {name}, your score is {score * 10}!"
  ```
- **Raw String (`r"..."`)**: Backslashes are treated as literal characters. No escape sequences are processed. Ideal for regexes and Windows file paths:
  ```alya
  let path = r"C:\Users\Taiizor\AppData\Local"
  let regex = r"\d{4}-\d{2}-\d{2}"
  ```
- **Multiline String (`"""..."""`)**: Encloses text spanning multiple lines, preserving whitespace and newlines:
  ```alya
  let sql = """
  SELECT id, username, email
  FROM users
  WHERE active = true;
  """
  ```

### 1.7 Byte Literals
- Raw byte characters are prefixed with `b'...'` and evaluate to `u8`: `let b = b'A'` (value: `65`).
- Byte strings are prefixed with `b"..."` and evaluate to an array of bytes (`u8[]`): `let magic = b"RIFF"`.

---

## 2. Formal Grammar (EBNF Snippet)

```ebnf
SourceFile   ::= ( Statement | Comment | Newline )*

Comment      ::= "#" [^\n]*
DocComment   ::= "##" [^\n]*

IntegerLit   ::= DecimalLit | HexLit | BinaryLit | OctalLit
DecimalLit   ::= [0-9] ( [0-9_] )*
HexLit       ::= "0" [xX] [0-9a-fA-F_]+
BinaryLit    ::= "0" [bB] [01_]+
OctalLit     ::= "0" [oO] [0-7_]+

FloatLit     ::= [0-9]+ "." [0-9]+ ( [eE] [+-]? [0-9]+ )?

StringLit    ::= StandardString | FormattedString | RawString | MultilineString
StandardString  ::= '"' ( EscapeSeq | [^"\\] )* '"'
FormattedString ::= 'f"' ( "{" Expr "}" | EscapeSeq | [^"\\{] )* '"'
RawString       ::= 'r"' [^"]* '"'
MultilineString ::= '"""' ( [^"] | '"' [^"] | '""' [^"] )* '"""'
```
