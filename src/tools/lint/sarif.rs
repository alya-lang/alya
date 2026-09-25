use std::collections::BTreeSet;
use std::path::Path;

use super::types::{LintDiagnostic, LintReport, LintSeverity};

/// Short rule metadata for the SARIF `rules` dictionary: rule id,
/// human-readable description, and default severity.
fn rule_metadata(rule: &str) -> (&'static str, &'static str) {
    match rule {
        "unused-var" => ("Unused variable", "warning"),
        "unused-param" => ("Unused function parameter", "warning"),
        "unused-import" => ("Unused import", "warning"),
        "dead-code" => ("Dead or unreachable code", "warning"),
        "idiomatic-style" => ("Unidiomatic style or anti-pattern", "warning"),
        "self-comparison" => ("Suspicious self-comparison", "warning"),
        "constant-condition" => ("Constant condition", "warning"),
        "useless-expression" => ("Useless expression", "warning"),
        "naming-convention" => ("Naming convention violation", "note"),
        _ => ("Alya lint finding", "warning"),
    }
}

fn sarif_level(severity: LintSeverity) -> &'static str {
    match severity {
        LintSeverity::Error => "error",
        LintSeverity::Warning => "warning",
        LintSeverity::Info => "note",
    }
}

/// Formats a file path as a SARIF artifact URI: relative to the working
/// directory when possible, always with forward slashes and no `./` prefix
/// (code scanning matches URIs against repository paths).
fn artifact_uri(file_path: &Path) -> String {
    let mut s = file_path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("./") {
        s = stripped.to_string();
    }
    if Path::new(&s).is_absolute() {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        if let Some(rel) = s.strip_prefix(&format!("{}/", cwd)) {
            return rel.to_string();
        }
    }
    s
}

fn result_object(diag: &LintDiagnostic) -> serde_json::Value {
    let mut text = diag.message.clone();
    if let Some(help) = &diag.help {
        text.push_str("\nHelp: ");
        text.push_str(help);
    }
    // SARIF columns are 1-based, matching the lint span convention.
    let (end_line, end_col) =
        if diag.end_line > diag.line || (diag.end_line == diag.line && diag.end_col > diag.col) {
            (diag.end_line.max(1), diag.end_col.max(1))
        } else {
            (diag.line.max(1), diag.col.max(1) + 1)
        };
    serde_json::json!({
        "ruleId": diag.rule,
        "level": sarif_level(diag.severity),
        "message": { "text": text },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": { "uri": artifact_uri(&diag.file_path) },
                "region": {
                    "startLine": diag.line.max(1),
                    "startColumn": diag.col.max(1),
                    "endLine": end_line,
                    "endColumn": end_col,
                }
            }
        }]
    })
}

/// Builds a SARIF v2.1.0 log for a lint report. The `rules` dictionary
/// covers every rule id present in the report; an empty report still yields
/// a valid log with empty `rules` and `results` (accepted by code scanning).
pub fn sarif_log(report: &LintReport, tool_version: &str) -> serde_json::Value {
    let mut rule_ids: BTreeSet<&str> = BTreeSet::new();
    for d in &report.diagnostics {
        rule_ids.insert(d.rule.as_str());
    }
    // The static default level describes the rule; per-result `level`
    // carries the actual diagnostic severity.
    let rules: Vec<serde_json::Value> = rule_ids
        .iter()
        .map(|id| {
            let (description, default_level) = rule_metadata(id);
            serde_json::json!({
                "id": id,
                "shortDescription": { "text": description },
                "defaultConfiguration": { "level": default_level },
            })
        })
        .collect();
    let results: Vec<serde_json::Value> = report.diagnostics.iter().map(result_object).collect();
    serde_json::json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Alya Linter",
                    "version": tool_version,
                    "informationUri": "https://github.com/alya-lang/alya",
                    "rules": rules,
                }
            },
            "results": results,
        }]
    })
}

/// Renders a lint report as pretty-printed SARIF JSON.
pub fn render_sarif(report: &LintReport) -> String {
    serde_json::to_string_pretty(&sarif_log(report, env!("CARGO_PKG_VERSION")))
        .unwrap_or_else(|_| "{\"version\":\"2.1.0\",\"runs\":[]}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_report() -> LintReport {
        LintReport {
            files_scanned: 1,
            files_with_issues: 1,
            total_diagnostics: 2,
            warning_count: 1,
            error_count: 0,
            info_count: 1,
            fixes_applied: 0,
            diagnostics: vec![
                LintDiagnostic {
                    rule: "unused-var".to_string(),
                    severity: LintSeverity::Warning,
                    message: "Unused variable 'x'".to_string(),
                    file_path: PathBuf::from("src/main.alya"),
                    line: 3,
                    col: 5,
                    end_line: 3,
                    end_col: 6,
                    help: Some("Remove or use 'x'".to_string()),
                    fix: None,
                },
                LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: "Function name should be snake_case".to_string(),
                    file_path: PathBuf::from("src/main.alya"),
                    line: 1,
                    col: 10,
                    end_line: 1,
                    end_col: 18,
                    help: None,
                    fix: None,
                },
            ],
        }
    }

    #[test]
    fn test_sarif_envelope() {
        let log = sarif_log(&sample_report(), "0.0.19");
        assert_eq!(log["version"], "2.1.0");
        assert_eq!(log["runs"][0]["tool"]["driver"]["name"], "Alya Linter");
        assert_eq!(log["runs"][0]["tool"]["driver"]["version"], "0.0.19");
        assert_eq!(log["runs"][0]["results"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_sarif_result_mapping() {
        let log = sarif_log(&sample_report(), "0.0.19");
        let first = &log["runs"][0]["results"][0];
        assert_eq!(first["ruleId"], "unused-var");
        assert_eq!(first["level"], "warning");
        assert_eq!(
            first["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "src/main.alya"
        );
        let region = &first["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 3);
        assert_eq!(region["startColumn"], 5);
        assert_eq!(region["endLine"], 3);
        assert_eq!(region["endColumn"], 6);
        assert!(first["message"]["text"]
            .as_str()
            .unwrap()
            .contains("Remove or use"));
        // Info severity maps to SARIF note.
        assert_eq!(log["runs"][0]["results"][1]["level"], "note");
    }

    #[test]
    fn test_sarif_rules_dictionary() {
        let log = sarif_log(&sample_report(), "0.0.19");
        let rules = log["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        assert_eq!(rules.len(), 2);
        let ids: Vec<&str> = rules.iter().map(|r| r["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"unused-var"));
        assert!(ids.contains(&"naming-convention"));
        for r in rules {
            assert!(!r["shortDescription"]["text"].as_str().unwrap().is_empty());
            assert!(["error", "warning", "note"]
                .contains(&r["defaultConfiguration"]["level"].as_str().unwrap()));
        }
    }

    #[test]
    fn test_sarif_empty_report_is_valid() {
        let log = sarif_log(&LintReport::default(), "0.0.19");
        assert_eq!(log["version"], "2.1.0");
        assert_eq!(log["runs"][0]["results"].as_array().unwrap().len(), 0);
        assert_eq!(
            log["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
