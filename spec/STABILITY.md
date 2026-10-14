# Alya Stability Contract

> What a release guarantees, and how the guarantee is enforced
> mechanically. Aspirations without gates are wishes; this file lists
> both the promise and the test that holds it.

## 1. Release train (as-shipped)

1. PRs target `develop` (conventional commits).
2. `develop` CI: fmt + clippy, 6-platform tests, spike differentials.
3. Fast-forward merge `develop` → `main`.
4. Tag `vX.Y.Z` → release workflow builds archives → `setup-alya`
   `latest` resolves dynamically → all 32 package CIs consume it.
5. `update-alya` bumps `alya.toml` git deps weekly.

The weak link is step 4→5: every package consumes a release
simultaneously. Mitigation is the soak rule (§4), not pinning:
packages must always build against `latest`.

## 2. Versioning (SemVer, pre-1.0 honest)

- `MAJOR.MINOR.PATCH`; today `0.0.x`.
- Pre-1.0, SemVer permits anything to break on a minor bump. Alya
  narrows that: **Stable chapters and their golden fixtures are the
  compatibility floor** — breaking them requires a MAJOR bump even
  at `0.x` (spec-first constitution §3). Everything else (Draft
  sections, internals, CLI flags) may change on a minor with a
  changelog note.
- `Cargo.toml` version is the single source of truth; it must be
  `X.Y.Z`-shaped (gated by `stability_contract_version_is_semver`).

## 3. Deprecation procedure

1. One minor release with a loud checker diagnostic pointing at the
   replacement (warnings, never silent).
2. Removal at the next minor (non-Stable surface) or next major
   (Stable surface). No removal without step 1.

## 4. Soak rule

A release is announced only after the ecosystem suite (`Tst/Test`,
develop-branch compiler × all packages) is green for that tag.
Automation of this gate is follow-up work; until then it is a
manual checklist item on every release.

## 5. Support window

No LTS before 1.0: the supported line is the latest minor. Security
fixes land forward only.

## 6. Mechanical gates (this file is tested)

- `stability_contract_chapter_table_matches_fixtures`: every
  `spec/README.md` chapter-table row names an existing
  `spec/syntax/*.alya` fixture AND its `spec/chapters/*.md`
  contract, and every fixture on disk is in the table — README
  drift or fixture-less chapters fail CI.
- `stability_contract_version_is_semver`: `Cargo.toml` parses as
  `MAJOR.MINOR.PATCH`.
- Existing: `test_golden_spec_fixtures_count` (exactly 25 golden
  files), the golden conformance matrix, and the integration
  `entry_files` existence test.
