#!/usr/bin/env python3
"""Differential parser test: Rust parser (`alya ast`) vs Alya parser.

Usage: python3 diff_parse.py [--alya BIN] [--lexer LEXER] [--parser PARSER]
                             [--corpus DIR...] [files...]

Pipeline per file:
  rust:  alya ast <file>                    -> S-expr lines (or nonzero exit)
  alya:  alya run lexer -- <file> > tok     -> token dump
         alya run parser -- tok             -> S-expr lines (or 0|0|error)

Compares canonical S-expression streams. Exit 0 when all files agree.
Floats compare by bit equality of parsed doubles; strings compare by
JSON-decoded value; everything else compares textually.
Files needing slice-2 constructs (closures/comprehensions/when/comptime/
cfg/select) are SKIP-listed with reasons instead of failing.
"""

import json
import re
import struct
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
ALYA = str(ROOT / "target" / "debug" / "alya.exe")
LEXER = str(ROOT / "spike" / "selfhost" / "parser.alya").replace(
    "parser.alya", "lexer.alya"
)
PARSER = str(ROOT / "spike" / "selfhost" / "parser.alya")

# Files whose Rust parse SUCCEEDS but needs slice-2 constructs.
# Format: substring -> reason. Revisit as the Alya parser grows.
SKIP_SUBSTR = {
    "collections.alya": "comprehensions (slice 3)",
    "functions.alya": "closures (slice 3)",
    "concurrency.alya": "closures/spawn (slice 3)",
    "stdlib_contracts.alya": "closures/spawn (slice 3)",
}

FLOAT_RE = re.compile(r"\(float ([^()]+)\)")
STR_RE = re.compile(r"\(str \"((?:[^\"\\]|\\.)*)\"\)")


def run_utf8(*args):
    return subprocess.run(args, capture_output=True, text=True, encoding="utf-8")


def rust_ast(alya_bin, path):
    p = run_utf8(alya_bin, "ast", str(path))
    if p.returncode != 0:
        text = p.stdout + p.stderr
        pos = None
        m = re.search(r"\.alya:(\d+):(\d+)", text)
        if m:
            pos = (int(m.group(1)), int(m.group(2)))
        else:
            m = re.search(r"line (\d+), column (\d+)", text)
            if m:
                pos = (int(m.group(1)), int(m.group(2)))
            else:
                # Line-only diagnostics (e.g. @cfg key errors: `file:2`).
                m = re.search(r"\.alya:(\d+)", text)
                if m:
                    pos = (int(m.group(1)), 0)
        return (False, pos, [])
    lines = [l for l in p.stdout.splitlines() if l.strip()]
    return (True, None, lines)


def alya_ast(alya_bin, lexer, parser, path, tmpdir):
    tok = Path(tmpdir) / "dump.tok"
    p1 = run_utf8(alya_bin, "run", lexer, "--", str(path))
    if p1.returncode != 0:
        # Lexer-level failure: compare lexer error positions (diff_lex turf).
        return (None, None, [])
    tok.write_text(p1.stdout, encoding="utf-8")
    p2 = run_utf8(alya_bin, "run", parser, "--", str(tok))
    lines = []
    err_pos = None
    for line in p2.stdout.splitlines():
        if line.startswith("0|0|error|"):
            err_pos = tuple(int(x) for x in line.split("|", 3)[3].split(":"))
            continue
        if line.strip():
            lines.append(line)
    ok = p2.returncode == 0
    return (ok, err_pos, lines)


def float_bits(lex):
    return struct.pack(">d", float(lex))


def compare(rust_lines, alya_lines):
    """Compare S-expr streams; returns list of diff strings."""
    diffs = []
    if len(rust_lines) != len(alya_lines):
        diffs.append(
            f"stmt count: rust={len(rust_lines)} alya={len(alya_lines)}"
        )
    for i, (r, a) in enumerate(zip(rust_lines, alya_lines)):
        rf = FLOAT_RE.findall(r)
        af = FLOAT_RE.findall(a)
        if len(rf) != len(af):
            diffs.append(f"line[{i}]: float count rust={len(rf)} alya={len(af)}")
            continue
        bad = False
        for rl, al in zip(rf, af):
            try:
                rb, ab = float_bits(rl), float_bits(al)
            except Exception:
                diffs.append(f"line[{i}]: unparsable float rust={rl!r} alya={al!r}")
                bad = True
                break
            if rb != ab:
                diffs.append(
                    f"line[{i}]: float bits differ: rust={rl!r} alya={al!r}"
                )
                bad = True
                break
        if bad:
            continue
        r2 = FLOAT_RE.sub("(float #)", r)
        a2 = FLOAT_RE.sub("(float #)", a)
        rs = STR_RE.findall(r2)
        asl = STR_RE.findall(a2)
        if len(rs) != len(asl):
            diffs.append(f"line[{i}]: str count rust={len(rs)} alya={len(asl)}")
            continue
        for rv, av in zip(rs, asl):
            try:
                rval = json.loads(f'"{rv}"')
                aval = json.loads(f'"{av}"')
            except Exception as e:
                diffs.append(f"line[{i}]: str not JSON ({e})")
                bad = True
                break
            rval = rval.split("\0", 1)[0]
            aval = aval.split("\0", 1)[0]
            if rval != aval:
                diffs.append(
                    f"line[{i}]: str value differ: rust={rval!r} alya={aval!r}"
                )
                bad = True
                break
        if bad:
            continue
        r3 = STR_RE.sub('(str "#")', r2)
        a3 = STR_RE.sub('(str "#")', a2)
        if r3 != a3:
            diffs.append(f"line[{i}]: differ:\n    rust={r[:200]}\n    alya={a[:200]}")
    return diffs


def skip_reason(f):
    for sub, why in SKIP_SUBSTR.items():
        if f.name == sub or str(f).endswith(sub):
            return why
    return ""


def main(argv):
    alya_bin = ALYA
    lexer = LEXER
    parser = PARSER
    files = []
    it = iter(argv)
    for a in it:
        if a == "--alya":
            alya_bin = next(it)
        elif a == "--lexer":
            lexer = next(it)
        elif a == "--parser":
            parser = next(it)
        else:
            files.append(a)
    if not files:
        files = sorted((ROOT / "spec" / "syntax").glob("*.alya"))
        files += sorted((ROOT / "stdlib").glob("*.alya"))
    fails = 0
    skips = 0
    passes = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        for f in files:
            f = Path(f)
            rok, rerr, rtoks = rust_ast(alya_bin, f)
            aok, aerr, atoks = alya_ast(alya_bin, lexer, parser, f, tmpdir)
            if aok is None:
                print(f"skip {f.name} (lexer-level failure; diff_lex turf)")
                skips += 1
                continue
            if rok != aok:
                skipped = skip_reason(f)
                if skipped:
                    print(f"SKIP {f.name} [{skipped}]")
                    skips += 1
                else:
                    print(
                        f"FAIL {f.name}: status rust={rok} alya={aok} "
                        f"(rust_err={rerr} alya_err={aerr})"
                    )
                    fails += 1
                continue
            if not rok:
                if rerr != aerr:
                    skipped = skip_reason(f)
                    if skipped:
                        print(f"SKIP {f.name} [{skipped}]")
                        skips += 1
                    else:
                        print(f"FAIL {f.name}: error pos rust={rerr} alya={aerr}")
                        fails += 1
                else:
                    print(f"ok   {f.name} (both error at {rerr})")
                    passes += 1
                continue
            diffs = compare(rtoks, atoks)
            if diffs:
                print(f"FAIL {f.name}: {len(diffs)} diffs")
                for d in diffs[:6]:
                    print(f"    {d}")
                fails += 1
            else:
                print(f"ok   {f.name} ({len(rtoks)} stmts)")
                passes += 1
    print(f"---\nfiles={len(files)} pass={passes} skip={skips} fail={fails}")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
