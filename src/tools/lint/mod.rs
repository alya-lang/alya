pub mod config;
pub mod fix;
pub mod rules;
pub mod sarif;
pub mod suppression;
pub mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use crate::lexer::Lexer;
use crate::parser::Parser;
pub use config::*;
pub use fix::*;
pub use rules::*;
pub use sarif::{render_sarif, sarif_log};
pub use suppression::*;
pub use types::*;

/// Analyzes Alya source code and returns a list of lint diagnostics.
pub fn lint_source(source: &str, file_path: &Path) -> Result<Vec<LintDiagnostic>, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer
        .tokenize()
        .map_err(|e| format!("Lexer error while linting: {}", e))?;

    let mut parser = Parser::new(tokens.clone());
    let program = match parser.parse() {
        Ok(p) => p,
        Err(_) => {
            // If the code has syntax errors, defer to compiler/lsp syntax diagnostics
            return Ok(Vec::new());
        }
    };

    let raw_diags = run_all_rules(&program, &tokens, file_path);

    // 1. Filter out inline comment suppressions
    let suppression = SuppressionFilter::from_source(source);
    let filtered_diags = suppression.filter_diagnostics(raw_diags);

    // 2. Filter out project config rules & apply severity overrides
    let config = LintConfig::discover(file_path);
    let final_diags = config.apply_to_diagnostics(filtered_diags);

    Ok(final_diags)
}

/// Discovers all Alya files to be linted.
pub fn find_lint_files(path: &Path) -> Vec<PathBuf> {
    crate::tools::fmt::find_alya_files(path)
}

/// Executes the CLI `alya lint` command.
///
/// In SARIF mode no human-readable text is printed: the JSON payload goes to
/// `output` (a file) or stdout, so stdout stays machine-parseable. The
/// `--check` gate still applies afterwards, so CI keeps its exit-code
/// semantics while the artifact is always written first.
pub fn run_lint_cli(
    path_opt: Option<&str>,
    fix: bool,
    check: bool,
    format: LintFormat,
    output: Option<&str>,
) -> Result<(), String> {
    let target_str = path_opt.unwrap_or(".");
    let target_path = Path::new(target_str);
    if !target_path.exists() {
        return Err(format!("Error: Path does not exist '{}'", target_str));
    }

    let config = LintConfig::discover(target_path);

    let all_files = find_lint_files(target_path);
    let files: Vec<PathBuf> = all_files
        .into_iter()
        .filter(|p| !config.is_path_excluded(p))
        .collect();

    if files.is_empty() {
        if format == LintFormat::Sarif {
            let payload = render_sarif(&LintReport::default());
            match output {
                Some(out) => {
                    fs::write(out, &payload).map_err(|e| {
                        format!("Error: Failed to write SARIF report to '{}': {}", out, e)
                    })?;
                    println!("✓ Wrote SARIF report to '{}' (0 finding(s)).", out);
                }
                None => println!("{}", payload),
            }
            return Ok(());
        }
        println!("No .alya files found in '{}'.", target_str);
        return Ok(());
    }

    let mut report = LintReport {
        files_scanned: files.len(),
        ..Default::default()
    };

    for file in &files {
        let display_path = file.display().to_string();
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading '{}': {}", display_path, e);
                continue;
            }
        };

        let diags = match lint_source(&content, file) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        if !diags.is_empty() {
            report.files_with_issues += 1;
            report.total_diagnostics += diags.len();

            for d in &diags {
                match d.severity {
                    LintSeverity::Warning => report.warning_count += 1,
                    LintSeverity::Error => report.error_count += 1,
                    LintSeverity::Info => report.info_count += 1,
                }
                if format == LintFormat::Text {
                    print!("{}", d.render(&content));
                }
            }

            if fix {
                let (new_content, applied) = apply_fixes_to_source(&content, &diags);
                if applied > 0 {
                    if let Err(e) = fs::write(file, &new_content) {
                        eprintln!("Error writing fixes to '{}': {}", display_path, e);
                    } else {
                        report.fixes_applied += applied;
                        println!(
                            "  \x1b[1;32m✓ Fixed\x1b[0m {} issue(s) in {}",
                            applied,
                            display_path.replace('\\', "/")
                        );
                    }
                }
            }

            report.diagnostics.extend(diags);
        }
    }

    if format == LintFormat::Sarif {
        let payload = render_sarif(&report);
        match output {
            Some(out) => {
                fs::write(out, &payload).map_err(|e| {
                    format!("Error: Failed to write SARIF report to '{}': {}", out, e)
                })?;
                println!(
                    "✓ Wrote SARIF report to '{}' ({} finding(s)).",
                    out, report.total_diagnostics
                );
            }
            None => println!("{}", payload),
        }
        // The artifact is written first; the check gate keeps CI semantics.
        let failure_count = report.warning_count + report.error_count;
        if check && failure_count > 0 {
            return Err(format!(
                "Lint check failed: {} warning(s)/error(s) detected across {} file(s).",
                failure_count, report.files_with_issues
            ));
        }
        return Ok(());
    }

    if report.total_diagnostics == 0 {
        println!(
            "\x1b[1;32m✓ All clean: no lint warnings found across {} file(s).\x1b[0m",
            report.files_scanned
        );
        return Ok(());
    }

    println!();
    if fix && report.fixes_applied > 0 {
        println!(
            "\x1b[1;32m✓ Applied {} automated fix(es) across {} file(s).\x1b[0m",
            report.fixes_applied, report.files_with_issues
        );
    }

    let failure_count = report.warning_count + report.error_count;
    if check {
        if failure_count > 0 {
            Err(format!(
                "Lint check failed: {} warning(s)/error(s) detected across {} file(s).",
                failure_count, report.files_with_issues
            ))
        } else {
            if report.info_count > 0 {
                println!(
                    "\x1b[1;36m✓ Lint check passed with {} style suggestion(s) across {} file(s).\x1b[0m",
                    report.info_count, report.files_with_issues
                );
            } else {
                println!("\x1b[1;32m✓ Lint check passed.\x1b[0m");
            }
            Ok(())
        }
    } else {
        if failure_count > 0 {
            println!(
                "\x1b[1;33mFound {} issue(s) ({} warning(s), {} error(s), {} suggestion(s)) across {} file(s).\x1b[0m",
                report.total_diagnostics, report.warning_count, report.error_count, report.info_count, report.files_with_issues
            );
        } else {
            println!(
                "\x1b[1;36mFound {} style suggestion(s) across {} file(s).\x1b[0m",
                report.info_count, report.files_with_issues
            );
        }
        Ok(())
    }
}
