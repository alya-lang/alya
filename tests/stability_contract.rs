//! Stability-contract gates (alya-lang/alya#194, `spec/STABILITY.md` §6).
//!
//! The contract promises that Stable chapters are backed by fixtures
//! and that versions are SemVer-shaped. These tests hold the promise:
//! README-table drift or fixture-less chapters fail CI instead of
//! rotting silently.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn spec_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let embedded = manifest_dir.join("spec");
    if embedded.exists() {
        embedded
    } else {
        manifest_dir.parent().unwrap().join("spec")
    }
}

/// `(chapter_contract, syntax_fixture)` pairs scraped from the
/// `spec/README.md` chapter table.
fn chapter_table_entries() -> Vec<(String, String)> {
    let readme = fs::read_to_string(spec_dir().join("README.md")).expect("spec/README.md");
    let mut entries = Vec::new();
    for line in readme.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let mut chapters = Vec::new();
        let mut fixtures = Vec::new();
        // Link targets look like `](chapters/00_lexical_structure.md)`
        // and `](syntax/lexical.alya)`; plain-text cells never do.
        let mut rest = line;
        while let Some(start) = rest.find("](") {
            let after = &rest[start + 2..];
            if let Some(end) = after.find(')') {
                let target = &after[..end];
                if let Some(name) = target.strip_prefix("chapters/") {
                    chapters.push(name.to_string());
                } else if let Some(name) = target.strip_prefix("syntax/") {
                    if name.ends_with(".alya") {
                        fixtures.push(name.to_string());
                    }
                }
                rest = &after[end + 1..];
            } else {
                break;
            }
        }
        for fixture in fixtures {
            // Exactly one contract per fixture row in this table.
            assert_eq!(
                chapters.len(),
                1,
                "chapter-table row must name exactly one contract: {}",
                line
            );
            entries.push((chapters[0].clone(), fixture));
        }
    }
    entries
}

#[test]
fn stability_contract_chapter_table_matches_fixtures() {
    let entries = chapter_table_entries();
    assert!(
        entries.len() >= 25,
        "chapter table must list all canonical chapters, found {}",
        entries.len()
    );
    for (chapter, fixture) in &entries {
        assert!(
            spec_dir().join("chapters").join(chapter).exists(),
            "chapter contract missing: spec/chapters/{}",
            chapter
        );
        assert!(
            spec_dir().join("syntax").join(fixture).exists(),
            "chapter fixture missing: spec/syntax/{}",
            fixture
        );
    }
    // No orphans: every fixture on disk is claimed by the table, so a
    // chapter can never lose its fixture silently (and unlisted files
    // cannot dodge the golden harness, which runs the whole dir).
    let on_disk: HashSet<String> = fs::read_dir(spec_dir().join("syntax"))
        .expect("spec/syntax")
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "alya"))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let in_table: HashSet<String> = entries.iter().map(|(_, f)| f.clone()).collect();
    assert_eq!(
        on_disk, in_table,
        "spec/syntax fixtures must exactly match the README chapter table"
    );
}

#[test]
fn stability_contract_version_is_semver() {
    let manifest = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("Cargo.toml");
    let version = manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = "))
        .expect("Cargo.toml has a version")
        .trim()
        .trim_matches('"')
        .to_string();
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "Cargo.toml version must be MAJOR.MINOR.PATCH, got '{}'",
        version
    );
    assert!(
        parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())),
        "Cargo.toml version must be numeric X.Y.Z, got '{}'",
        version
    );
}
