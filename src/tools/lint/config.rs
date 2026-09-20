use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::types::{LintDiagnostic, LintSeverity};

/// Project-level configuration for Alya linter.
#[derive(Debug, Default, Clone)]
pub struct LintConfig {
    /// Rules completely disabled project-wide.
    pub disabled_rules: HashSet<String>,
    /// Relative path patterns or directories to exclude from linting.
    pub exclude: Vec<String>,
    /// Rule-specific severity overrides (e.g. "unused-var" -> Error).
    pub severity_overrides: HashMap<String, LintSeverity>,
}

impl LintConfig {
    /// Searches for `alya.toml` or `.alyalint` starting from `start_path` upwards.
    pub fn discover(start_path: &Path) -> Self {
        let mut curr = if start_path.is_file() {
            start_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."))
        } else {
            start_path.to_path_buf()
        };

        loop {
            // Check .alyalint first
            let dot_lint = curr.join(".alyalint");
            if dot_lint.is_file() {
                if let Ok(content) = fs::read_to_string(&dot_lint) {
                    return Self::parse_config_str(&content);
                }
            }

            // Check alya.toml
            let manifest = curr.join("alya.toml");
            if manifest.is_file() {
                if let Ok(content) = fs::read_to_string(&manifest) {
                    return Self::parse_config_str(&content);
                }
            }

            match curr.parent() {
                Some(p) if p != curr => curr = p.to_path_buf(),
                _ => break,
            }
        }

        Self::default()
    }

    /// Parses configuration from TOML-formatted string.
    pub fn parse_config_str(content: &str) -> Self {
        let mut config = Self::default();
        let mut in_lint_section = false;

        for line_raw in content.lines() {
            let line = line_raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = line[1..line.len() - 1].trim();
                in_lint_section = section == "lint";
                continue;
            }

            if in_lint_section {
                if let Some((k, v)) = line.split_once('=') {
                    let key = k.trim();
                    let val = v.trim();

                    match key {
                        "disabled_rules" | "disable" => {
                            for r in parse_str_list(val) {
                                config.disabled_rules.insert(r);
                            }
                        }
                        "exclude" => {
                            config.exclude.extend(parse_str_list(val));
                        }
                        _ => {
                            // Check for rule severity override, e.g. `unused-var = "error"`
                            let sev = match val
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_lowercase()
                                .as_str()
                            {
                                "warning" | "warn" => Some(LintSeverity::Warning),
                                "error" => Some(LintSeverity::Error),
                                "info" => Some(LintSeverity::Info),
                                "off" | "disabled" => {
                                    config.disabled_rules.insert(key.to_string());
                                    None
                                }
                                _ => None,
                            };
                            if let Some(s) = sev {
                                config.severity_overrides.insert(key.to_string(), s);
                            }
                        }
                    }
                }
            }
        }

        config
    }

    /// Returns true if the file path should be excluded based on config.
    pub fn is_path_excluded(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().replace('\\', "/");
        for ex in &self.exclude {
            let ex_clean = ex.trim_matches('/').replace('\\', "/");
            if path_str.contains(&ex_clean) {
                return true;
            }
        }
        false
    }

    /// Applies configuration filtering and severity overrides to diagnostics.
    pub fn apply_to_diagnostics(&self, diags: Vec<LintDiagnostic>) -> Vec<LintDiagnostic> {
        let mut result = Vec::new();
        for mut d in diags {
            if self.disabled_rules.contains(&d.rule) {
                continue;
            }
            if let Some(&new_sev) = self.severity_overrides.get(&d.rule) {
                d.severity = new_sev;
            }
            result.push(d);
        }
        result
    }
}

fn parse_str_list(val: &str) -> Vec<String> {
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
