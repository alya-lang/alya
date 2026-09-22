//! Project-level configuration discovery shared by `lint`, `fmt`, `test`
//! and `bench`.
//!
//! Lookup pattern: starting from a target path, walk upwards looking first
//! for a tool-specific dotfile (`.alyalint`, `.alyafmt`, `.alyatest`) and
//! then for `alya.toml`, parsing the matching `[section]` (`[lint]`,
//! `[fmt]`, `[test]` / `[bench]`). The first file found wins. When nothing
//! is found, built-in defaults apply (empty excludes).

use std::fs;
use std::path::{Path, PathBuf};

/// Walks upwards from `start_path` and returns the contents of the first
/// config source found: `dotfile_name` first, then `alya.toml`.
pub fn discover_config_content(start_path: &Path, dotfile_name: &str) -> Option<String> {
    let mut curr = if start_path.is_file() {
        start_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        start_path.to_path_buf()
    };

    loop {
        let dotfile = curr.join(dotfile_name);
        if dotfile.is_file() {
            if let Ok(content) = fs::read_to_string(&dotfile) {
                return Some(content);
            }
        }

        let manifest = curr.join("alya.toml");
        if manifest.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest) {
                return Some(content);
            }
        }

        match curr.parent() {
            Some(p) if p != curr => curr = p.to_path_buf(),
            _ => break,
        }
    }

    None
}

/// Returns raw `(key, value)` pairs under a `[section]` header from TOML-ish
/// content (same minimal dialect as below). Used by tools like `lint` that
/// need value interpretation beyond plain string lists.
pub fn parse_section_entries(content: &str, section: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut in_section = false;

    for line_raw in content.lines() {
        let line = line_raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            in_section = name == section;
            continue;
        }

        if in_section {
            if let Some((k, v)) = line.split_once('=') {
                entries.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }

    entries
}

/// Parses a `key = ["a", "b"]` string list under a `[section]` header from
/// TOML-ish content (same minimal dialect as the lint config parser:
/// `#` comments, `[section]` headers, `key = value` pairs).
pub fn parse_section_string_list(content: &str, section: &str, key: &str) -> Vec<String> {
    let mut items = Vec::new();
    for (k, v) in parse_section_entries(content, section) {
        if k == key {
            items.extend(parse_string_list(&v));
        }
    }
    items
}

/// Substring matching on `/`-normalized paths, identical to
/// `LintConfig::is_path_excluded`: an entry matches when the normalized
/// candidate path contains the normalized entry (so both `spec/negative`
/// and bare directory names like `vendor` work).
pub fn path_is_excluded(path: &Path, excludes: &[String]) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    for ex in excludes {
        let ex_clean = ex.trim_matches('/').replace('\\', "/");
        if !ex_clean.is_empty() && path_str.contains(&ex_clean) {
            return true;
        }
    }
    false
}

/// Parses a `["a", 'b']` value into items (quotes optional, comma-separated).
pub fn parse_string_list(val: &str) -> Vec<String> {
    let inner = val.trim().trim_start_matches('[').trim_end_matches(']');
    let mut items = Vec::new();
    for part in inner.split(',') {
        let clean = part.trim().trim_matches('"').trim_matches('\'').trim();
        if !clean.is_empty() {
            items.push(clean.to_string());
        }
    }
    items
}

/// Project-level configuration for `alya fmt`.
///
/// Sources (first hit wins): `.alyafmt`, then `alya.toml`, `[fmt]` section.
/// Built-in directory skips (`negative`, `fixtures`, … in `find_alya_files`)
/// always apply on top; `exclude` only *adds* project-specific entries.
/// Inline `# fmt: off` / `# fmt: on` ranges keep working independently —
/// they suppress formatting line-wise, while `exclude` skips whole files.
#[derive(Debug, Default, Clone)]
pub struct FmtConfig {
    /// Extra relative path patterns or directories to skip while formatting.
    pub exclude: Vec<String>,
}

impl FmtConfig {
    /// Discovers configuration starting from `start_path` upwards.
    pub fn discover(start_path: &Path) -> Self {
        let mut config = Self::default();
        if let Some(content) = discover_config_content(start_path, ".alyafmt") {
            config.exclude = parse_section_string_list(&content, "fmt", "exclude");
        }
        config
    }

    /// Returns true if the file should be skipped by the formatter.
    pub fn is_path_excluded(&self, path: &Path) -> bool {
        path_is_excluded(path, &self.exclude)
    }
}

/// Project-level configuration for `alya test` / `alya bench` discovery.
///
/// Sources (first hit wins): `.alyatest`, then `alya.toml`, `[test]` and
/// `[bench]` sections. Built-in skips (`negative`, `fixtures`, `common`, …)
/// always apply on top; these lists only *add* project-specific entries.
/// Applies to directory discovery; an explicitly named single file is still
/// honored.
#[derive(Debug, Default, Clone)]
pub struct SuiteConfig {
    /// Extra exclusions for `alya test` discovery.
    pub test_exclude: Vec<String>,
    /// Extra exclusions for `alya bench` discovery.
    pub bench_exclude: Vec<String>,
}

impl SuiteConfig {
    /// Discovers configuration starting from `start_path` upwards.
    pub fn discover(start_path: &Path) -> Self {
        let mut config = Self::default();
        if let Some(content) = discover_config_content(start_path, ".alyatest") {
            config.test_exclude = parse_section_string_list(&content, "test", "exclude");
            config.bench_exclude = parse_section_string_list(&content, "bench", "exclude");
        }
        config
    }

    /// Returns true if the path should be skipped during suite discovery.
    /// `is_bench` selects the `[bench]` list, otherwise `[test]`.
    pub fn is_path_excluded(&self, path: &Path, is_bench: bool) -> bool {
        if is_bench {
            path_is_excluded(path, &self.bench_exclude)
        } else {
            path_is_excluded(path, &self.test_exclude)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "alya_tool_cfg_{}_{}_{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_parse_section_string_list() {
        let content = r#"
[package]
name = "demo"

[fmt]
exclude = ["spec/negative", "vendor"]

[test]
exclude = ["slow"]
"#;
        assert_eq!(
            parse_section_string_list(content, "fmt", "exclude"),
            vec!["spec/negative".to_string(), "vendor".to_string()]
        );
        assert_eq!(
            parse_section_string_list(content, "test", "exclude"),
            vec!["slow".to_string()]
        );
        assert!(parse_section_string_list(content, "bench", "exclude").is_empty());
        // Raw entries preserve values for tool-specific interpretation.
        assert_eq!(
            parse_section_entries(content, "fmt"),
            vec![(
                "exclude".to_string(),
                "[\"spec/negative\", \"vendor\"]".to_string()
            )]
        );
        // Single-quoted items and missing section.
        let single = "[fmt]\nexclude = ['a', 'b']\n";
        assert_eq!(
            parse_section_string_list(single, "fmt", "exclude"),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn test_dotfile_preferred_over_manifest() {
        let dir = unique_temp_dir("prefer");
        fs::write(
            dir.join(".alyafmt"),
            "[fmt]\nexclude = [\"from-dotfile\"]\n",
        )
        .unwrap();
        fs::write(
            dir.join("alya.toml"),
            "[fmt]\nexclude = [\"from-manifest\"]\n",
        )
        .unwrap();

        let cfg = FmtConfig::discover(&dir);
        assert_eq!(cfg.exclude, vec!["from-dotfile".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_manifest_fallback_and_upwards_search() {
        let dir = unique_temp_dir("fallback");
        let nested = dir.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            dir.join("alya.toml"),
            "[test]\nexclude = [\"slow\"]\n[bench]\nexclude = [\"heavy\"]\n",
        )
        .unwrap();

        let cfg = SuiteConfig::discover(&nested);
        assert_eq!(cfg.test_exclude, vec!["slow".to_string()]);
        assert_eq!(cfg.bench_exclude, vec!["heavy".to_string()]);

        // No config anywhere under temp root beyond our dir: parent lookup
        // from a bare subdir without dotfile still resolves via alya.toml.
        let fmt_cfg = FmtConfig::discover(&nested);
        assert!(fmt_cfg.exclude.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_path_matching_semantics() {
        let excludes = vec!["spec/negative".to_string(), "vendor".to_string()];
        assert!(path_is_excluded(
            Path::new("spec/negative/foo.alya"),
            &excludes
        ));
        assert!(path_is_excluded(Path::new("spec/negative"), &excludes));
        assert!(path_is_excluded(Path::new("libs/vendor/x.alya"), &excludes));
        assert!(!path_is_excluded(
            Path::new("spec/syntax/x.alya"),
            &excludes
        ));
        assert!(!path_is_excluded(Path::new("spec/syntax/x.alya"), &[]));
    }
}
