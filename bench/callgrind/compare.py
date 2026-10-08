#!/usr/bin/env python3
"""Compare base vs head callgrind results.json files.

Prints a markdown table with per-workload deltas. Writes the regressed
workload ids (one per line) to the optional fourth argument. Always exits 0;
the CI gate step decides pass/fail from the regressed list, so the table is
published even on regression.

Usage: compare.py <base.json> <head.json> [limit_pct=5.0] [regressed_out]
"""
import json
import sys


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def main():
    base = load(sys.argv[1])
    head = load(sys.argv[2])
    limit = float(sys.argv[3]) if len(sys.argv) > 3 else 5.0
    out_path = sys.argv[4] if len(sys.argv) > 4 else None

    rows = []
    regressed = []
    for w, h in head.items():
        if w not in base:
            rows.append((w, "-", str(h["ir"]), "new", f"{h['ir'] / h['ops']:.1f}", "new"))
            continue
        b = base[w]["ir"]
        if base[w]["checksum"] != h["checksum"]:
            rows.append((w, str(b), str(h["ir"]), "CHECKSUM CHANGED", "-", "fail"))
            regressed.append(w)
            continue
        pct = (h["ir"] - b) / b * 100.0 if b else 0.0
        verdict = "OK" if pct <= limit else "REGRESSION"
        if pct > limit:
            regressed.append(w)
        rows.append((w, str(b), str(h["ir"]), f"{pct:+.1f}%", f"{h['ir'] / h['ops']:.1f}", verdict))
    for w, b in base.items():
        if w not in head:
            rows.append((w, str(b["ir"]), "-", "removed", "-", "info"))

    print("| workload | base Ir | head Ir | delta | Ir/op | verdict |")
    print("|---|---|---|---|---|---|")
    for r in rows:
        print(f"| {r[0]} | {r[1]} | {r[2]} | {r[3]} | {r[4]} | {r[5]} |")

    if out_path is not None:
        with open(out_path, "w", encoding="utf-8") as f:
            if regressed:
                f.write("\n".join(regressed) + "\n")


if __name__ == "__main__":
    main()
