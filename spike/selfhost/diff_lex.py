#!/usr/bin/env python3
"""Differential lexer test: Rust lexer (`alya tokens`) vs Alya lexer.

Usage: python3 diff_lex.py [--alya BIN] [--lexer LEXER] [--corpus DIR...] [files...]

Compares canonical token streams. Exit 0 when all files agree, 1 otherwise.
Floats compare by bit equality of parsed doubles (lexeme vs Debug value).
NUL-containing strings compare truncated at the first NUL (Alya strings
cannot hold NUL; documented spike divergence).
"""

import json
import re
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
ALYA = str(ROOT / "target" / "debug" / "alya.exe")
LEXER = str(ROOT / "spike" / "selfhost" / "lexer.alya")

VARIANT_TO_KIND = {
    # keywords: variant -> canonical name
    "Say": ("keyword", "say"), "Let": ("keyword", "let"), "If": ("keyword", "if"),
    "Else": ("keyword", "else"), "Elif": ("keyword", "elif"), "While": ("keyword", "while"),
    "For": ("keyword", "for"), "In": ("keyword", "in"), "Function": ("keyword", "function"),
    "End": ("keyword", "end"), "Return": ("keyword", "return"), "When": ("keyword", "when"),
    "Is": ("keyword", "is"), "Then": ("keyword", "then"), "Repeat": ("keyword", "repeat"),
    "Break": ("keyword", "break"), "Continue": ("keyword", "continue"), "Ask": ("keyword", "ask"),
    "Try": ("keyword", "try"), "Catch": ("keyword", "catch"), "Finally": ("keyword", "finally"),
    "Throw": ("keyword", "throw"), "Import": ("keyword", "import"), "As": ("keyword", "as"),
    "Struct": ("keyword", "struct"), "Enum": ("keyword", "enum"), "Const": ("keyword", "const"),
    "Extern": ("keyword", "extern"), "From": ("keyword", "from"), "Defer": ("keyword", "defer"),
    "Pub": ("keyword", "pub"), "Interface": ("keyword", "interface"), "Spawn": ("keyword", "spawn"),
    "Select": ("keyword", "select"), "Assert": ("keyword", "assert"), "Test": ("keyword", "test"),
    "Bench": ("keyword", "bench"), "SelfKw": ("keyword", "self"), "Weak": ("keyword", "weak"),
    "Comptime": ("keyword", "comptime"), "Sizeof": ("keyword", "sizeof"),
    "Alignof": ("keyword", "alignof"), "Typeof": ("keyword", "typeof"),
    "True": ("keyword", "true"), "False": ("keyword", "false"), "Null": ("keyword", "null"),
    "And": ("keyword", "and"), "Or": ("keyword", "or"), "Not": ("keyword", "not"),
    "At": ("op", "@"),
    # operators
    "Plus": ("op", "+"), "Minus": ("op", "-"), "Multiply": ("op", "*"),
    "Divide": ("op", "/"), "Modulo": ("op", "%"), "Arrow": ("op", "->"),
    "FatArrow": ("op", "=>"), "Assign": ("op", "="), "PlusAssign": ("op", "+="),
    "MinusAssign": ("op", "-="), "MultiplyAssign": ("op", "*="),
    "DivideAssign": ("op", "/="), "ModuloAssign": ("op", "%="),
    "BitAndAssign": ("op", "&="), "BitOrAssign": ("op", "|="),
    "BitXorAssign": ("op", "^="), "ShlAssign": ("op", "<<="), "ShrAssign": ("op", ">>="),
    "Equal": ("op", "=="), "NotEqual": ("op", "!="), "Less": ("op", "<"),
    "Greater": ("op", ">"), "LessEqual": ("op", "<="), "GreaterEqual": ("op", ">="),
    "BitAnd": ("op", "&"), "BitOr": ("op", "|"), "BitXor": ("op", "^"),
    "BitNot": ("op", "~"), "Shl": ("op", "<<"), "Shr": ("op", ">>"),
    # delimiters
    "LeftParen": ("delim", "("), "RightParen": ("delim", ")"),
    "LeftBracket": ("delim", "["), "RightBracket": ("delim", "]"),
    "LeftBrace": ("delim", "{"), "RightBrace": ("delim", "}"),
    "Comma": ("delim", ","), "Colon": ("delim", ":"), "ColonColon": ("delim", "::"),
    "Question": ("delim", "?"), "QuestionDot": ("delim", "?."), "NullCoalesce": ("delim", "??"),
    "Dot": ("delim", "."), "DotDot": ("delim", ".."), "DotDotEqual": ("delim", "..="),
    "DotDotDot": ("delim", "..."),
    "Newline": ("newline", ""), "Eof": ("eof", ""),
}

RUST_ESCAPES = {
    "n": "\n", "t": "\t", "r": "\r", "0": "\0", "\\": "\\",
    '"': '"', "'": "'",
}


def rust_unescape(s):
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c == "\\" and i + 1 < len(s):
            nxt = s[i + 1]
            if nxt == "u" and s[i + 2:i + 3] == "{":
                j = s.index("}", i + 3)
                out.append(chr(int(s[i + 3:j], 16)))
                i = j + 1
                continue
            out.append(RUST_ESCAPES.get(nxt, nxt))
            i += 2
        else:
            out.append(c)
            i += 1
    return "".join(out)


def parse_rust_debug(dbg):
    """Rust `{:?}` TokenType -> (kind, norm, aux) where aux carries raw text
    for floats (bit compare) and strings (unicode compare)."""
    if dbg in VARIANT_TO_KIND:
        kind, norm = VARIANT_TO_KIND[dbg]
        return (kind, norm, None)
    m = re.fullmatch(r"Number\((.+)\)", dbg)
    if m:
        return ("number", m.group(1), None)
    m = re.fullmatch(r"Float\((.+)\)", dbg)
    if m:
        return ("float", m.group(1), m.group(1))
    m = re.fullmatch(r'Identifier\("(.+)"\)', dbg)
    if m:
        return ("ident", m.group(1), None)
    m = re.fullmatch(r'String\("(.*)"\)', dbg, re.DOTALL)
    if m:
        return ("string", dbg, rust_unescape(m.group(1)))
    m = re.fullmatch(r"Rune\('(.*)'\)", dbg, re.DOTALL)
    if m:
        return ("rune", dbg, rust_unescape(m.group(1)))
    raise ValueError(f"unparsed Rust token debug: {dbg!r}")


def run_utf8(*args):
    return subprocess.run(args, capture_output=True, text=True, encoding="utf-8")


def rust_tokens(alya_bin, path):
    t0 = time.perf_counter()
    p = run_utf8(alya_bin, "tokens", str(path))
    dt = (time.perf_counter() - t0) * 1000
    if p.returncode != 0:
        text = p.stdout + p.stderr
        m = re.search(r"line (\d+), column (\d+)", text)
        if not m:
            m = re.search(r"\.alya:(\d+):(\d+)", text)
        pos = (int(m.group(1)), int(m.group(2))) if m else (None, None)
        return (False, pos, [], dt)
    toks = []
    for line in p.stdout.splitlines():
        if not line.strip() or line.startswith("POSITION") or line.startswith("---"):
            continue
        pos, dbg = line.split(None, 1)
        ln, col = pos.split(":")
        toks.append((int(ln), int(col)) + parse_rust_debug(dbg.strip()))
    return (True, None, toks, dt)


def alya_tokens(alya_bin, lexer, path):
    t0 = time.perf_counter()
    p = run_utf8(alya_bin, "run", lexer, "--", str(path))
    wall = (time.perf_counter() - t0) * 1000
    toks = []
    err_pos = None
    ms = None
    ok = p.returncode == 0
    for line in p.stdout.splitlines():
        parts = line.split("|", 3)
        if len(parts) != 4:
            continue
        ln, col, kind, norm = int(parts[0]), int(parts[1]), parts[2], parts[3]
        if kind == "stats":
            m = re.search(r"ms=(\d+)", norm)
            ms = int(m.group(1)) if m else None
            continue
        if kind == "error":
            err_pos = tuple(int(x) for x in norm.split(":"))
            continue
        toks.append((ln, col, kind, norm, None))
    return (ok, err_pos, toks, wall, ms)


def norm_key(t):
    # (line, col, kind, norm-or-aux)
    return t[:4]


def compare(rust, alya):
    """Compare canonical streams; returns list of diff strings."""
    diffs = []
    rtoks = [(ln, col, kind, norm) for (ln, col, kind, norm, _aux) in rust[2]]
    atoks = [(ln, col, kind, norm) for (ln, col, kind, norm, _aux) in alya[2]]
    # value-level comparison for strings/runes/floats
    r_aux = {i: t[4] for i, t in enumerate(rust[2])}
    if len(rtoks) != len(atoks):
        diffs.append(f"token count: rust={len(rtoks)} alya={len(atoks)}")
    for i, (r, a) in enumerate(zip(rtoks, atoks)):
        if r[:3] != a[:3]:
            diffs.append(f"tok[{i}]: rust={r[:3]} alya={a[:3]}")
            continue
        kind = r[2]
        if kind in ("string", "rune"):
            try:
                aval = json.loads(f'"{a[3]}"')
            except Exception as e:
                diffs.append(f"tok[{i}]: alya norm not JSON: {a[3]!r} ({e})")
                continue
            rval = r_aux[i]
            # NUL cannot survive Alya strings (null-terminated runtime):
            # both sides compare truncated at the first NUL.
            if "\0" in rval:
                rval = rval.split("\0", 1)[0]
            if "\0" in aval:
                aval = aval.split("\0", 1)[0]
            if aval != rval:
                diffs.append(f"tok[{i}]: {kind} value differ: rust={rval!r} alya={aval!r}")
        elif kind == "float":
            rb = struct.pack(">d", float(r[3]))
            try:
                ab = struct.pack(">d", float(a[3]))
            except Exception:
                diffs.append(f"tok[{i}]: alya float lexeme unparsable: {a[3]!r}")
                continue
            if rb != ab:
                diffs.append(f"tok[{i}]: float bits differ: rust={r[3]!r} alya={a[3]!r}")
        elif r[3] != a[3]:
            diffs.append(f"tok[{i}]: norm differ: rust={r[3]!r} alya={a[3]!r}")
    return diffs


def main(argv):
    alya_bin = ALYA
    lexer = LEXER
    files = []
    it = iter(argv)
    for a in it:
        if a == "--alya":
            alya_bin = next(it)
        elif a == "--lexer":
            lexer = next(it)
        else:
            files.append(a)
    if not files:
        files = sorted((ROOT / "spec" / "syntax").glob("*.alya"))
        files += sorted((ROOT / "stdlib").glob("*.alya"))
    fails = 0
    total_rust_ms = 0.0
    total_alya_ms = 0
    total_toks = 0
    for f in files:
        f = Path(f)
        rok, rerr, rtoks, rms = rust_tokens(alya_bin, f)
        aok, aerr, atoks, wall, ams = alya_tokens(alya_bin, lexer, f)
        total_rust_ms += rms
        if ams is not None:
            total_alya_ms += ams
        if rok != aok:
            print(f"FAIL {f.name}: status rust={rok} alya={aok} "
                  f"(rust_err={rerr} alya_err={aerr})")
            fails += 1
            continue
        if not rok:
            if rerr != aerr:
                print(f"FAIL {f.name}: error pos rust={rerr} alya={aerr}")
                fails += 1
            else:
                print(f"ok   {f.name} (both error at {rerr})")
            continue
        diffs = compare((rok, rerr, rtoks, rms), (aok, aerr, atoks, wall, ams))
        total_toks += len(rtoks)
        if diffs:
            print(f"FAIL {f.name}: {len(diffs)} diffs")
            for d in diffs[:8]:
                print(f"    {d}")
            fails += 1
        else:
            print(f"ok   {f.name} ({len(rtoks)} toks, rust={rms:.1f}ms alya={ams}ms)")
    print(f"---\nfiles={len(files)} fails={fails} toks={total_toks} "
          f"rust_total={total_rust_ms:.0f}ms alya_lex_total={total_alya_ms}ms")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
