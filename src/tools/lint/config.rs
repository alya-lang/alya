use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::types::{LintDiagnostic, LintSeverity};
use crate::tools::tool_config;

/// Project-level configuration for Alya linter.
///
/// Discovery and file parsing are shared with `fmt`/`test`/`bench` (see
/// [`tool_config`]); only rule filtering and severity overrides are
/// lint-specific.
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
        match tool_config::discover_config_content(start_path, ".alyalint") {
            Some(content) => Self::parse_config_str(&content),
            None => Self::default(),
        }
    }

    /// Parses configuration from TOML-formatted string (`[lint]` section).
    pub fn parse_config_str(content: &str) -> Self {
        let mut config = Self::default();

        for (key, val) in tool_config::parse_section_entries(content, "lint") {
            match key.as_str() {
                "disabled_rules" | "disable" => {
                    for r in tool_config::parse_string_list(&val) {
                        config.disabled_rules.insert(r);
                    }
                }
                "exclude" => {
                    config.exclude.extend(tool_config::parse_string_list(&val));
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
                            config.disabled_rules.insert(key.clone());
                            None
                        }
                        _ => None,
                    };
                    if let Some(s) = sev {
                        config.severity_overrides.insert(key.clone(), s);
                    }
                }
            }
        }

        config
    }

    /// Returns true if the file path should be excluded based on config.
    pub fn is_path_excluded(&self, path: &Path) -> bool {
        tool_config::path_is_excluded(path, &self.exclude)
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
