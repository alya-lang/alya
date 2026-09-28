#!/usr/bin/env bun
/**
 * brand_parity_check.ts
 *
 * Dark/light geometry parity checker for Alya brand assets (Bun port of
 * brand_parity_check.py — same rules, same output, zero dependencies).
 *
 * Every `*-dark.svg` must be geometrically identical to its `*-light.svg`
 * twin: same canvas, same shapes, same positions, same sizes, same transforms.
 * Only paint may differ between themes (fills, strokes, opacities, gradient
 * stop colors/positions, blur/glow filters).
 *
 * Covers: docs/, icons/, logos/ (VS Code extension icons derive from these
 * sources, so they are checked at the origin, not at the copy).
 *
 * Usage:
 *   bun assets/brand/brand_parity_check.ts            # exit 1 on drift
 *   bun assets/brand/brand_parity_check.ts --strict   # also flag stop-offset drift
 */

import { readFileSync } from "fs";
import { join, dirname, basename } from "path";

const BRAND_DIR = dirname(new URL(import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"));
const SCOPE_DIRS = ["docs", "icons", "logos"];

const GEO_ATTRS = new Set([
  "x", "y", "width", "height", "rx", "ry", "cx", "cy", "r",
  "x1", "y1", "x2", "y2", "points", "d", "transform", "viewBox",
]);

const SKIP_TAGS = new Set([
  "filter", "feGaussianBlur", "feDropShadow", "feMerge",
  "feMergeNode", "feOffset", "feFlood", "feComposite",
]);

function norm(v: string): string {
  return v.split("-light").join("").split("-dark").join("")
    .split("light-").join("").split("dark-").join("");
}

function signature(path: string, strict: boolean): Map<string, number> {
  const src = readFileSync(path, "utf-8");
  const sig = new Map<string, number>();
  const add = (k: string) => sig.set(k, (sig.get(k) ?? 0) + 1);
  const elRe = /<([a-zA-Z][a-zA-Z0-9]*)((?:\s+[^>]*?)?)\s*\/?>/g;
  let m: RegExpExecArray | null;
  while ((m = elRe.exec(src)) !== null) {
    const tag = m[1];
    const attrs = m[2];
    if (SKIP_TAGS.has(tag)) continue;
    if (tag === "stop") {
      if (strict) {
        const off = attrs.match(/offset="([^"]*)"/);
        add(`${tag}|offset=${off ? off[1] : ""}`);
      }
      continue;
    }
    const parts = [tag];
    const attrRe = /([\w-]+)="([^"]*)"/g;
    let a: RegExpExecArray | null;
    const found: Array<[string, string]> = [];
    while ((a = attrRe.exec(attrs)) !== null) {
      if (GEO_ATTRS.has(a[1])) found.push([a[1], norm(a[2])]);
    }
    found.sort((p, q) => (p[0] < q[0] ? -1 : 1));
    for (const [k, v] of found) parts.push(`${k}=${v}`);
    add(parts.join("|"));
  }
  return sig;
}

function diff(a: Map<string, number>, b: Map<string, number>): string[] {
  const out: string[] = [];
  for (const [k, n] of a) {
    const d = n - (b.get(k) ?? 0);
    for (let i = 0; i < d; i++) out.push(k);
  }
  return out;
}

async function main(): Promise<number> {
  const strict = process.argv.includes("--strict");
  const glob = new Bun.Glob("*-dark.svg");
  const pairs: Array<[string, string]> = [];
  for (const dir of SCOPE_DIRS) {
    for await (const f of glob.scan({ cwd: join(BRAND_DIR, dir), absolute: true })) {
      const light = f.replace("-dark.svg", "-light.svg");
      try {
        readFileSync(light);
        pairs.push([f, light]);
      } catch {
        console.log(`WARN: no light twin for ${f}`);
      }
    }
  }
  pairs.sort();
  if (pairs.length === 0) {
    console.log("No dark/light pairs found.");
    return 1;
  }
  let failed = 0;
  for (const [dark, light] of pairs) {
    const name = `${basename(dirname(dark))}/${basename(dark).replace("-dark.svg", "")}`;
    const sd = signature(dark, strict);
    const sl = signature(light, strict);
    const onlyD = diff(sd, sl);
    const onlyL = diff(sl, sd);
    if (onlyD.length === 0 && onlyL.length === 0) {
      console.log(`OK   ${name}`);
    } else {
      failed++;
      console.log(`FAIL ${name} (${onlyD.length + onlyL.length} geometric drift(s))`);
      for (const e of [...onlyD.map((x) => "dark-only: " + x), ...onlyL.map((x) => "light-only: " + x)].slice(0, 10)) {
        console.log(`       ${e}`);
      }
    }
  }
  console.log(`${pairs.length - failed}/${pairs.length} pairs geometrically identical${strict ? " (strict)" : ""}`);
  return failed ? 1 : 0;
}

process.exit(await main());
