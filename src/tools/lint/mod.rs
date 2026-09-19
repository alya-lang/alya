pub mod fix;
pub mod rules;
pub mod types;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use crate::lexer::Lexer;
use crate::parser::Parser;
pub use fix::*;
pub use rules::*;
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

    Ok(run_all_rules(&program, &tokens, file_path))
}

/// Discovers all Alya files to be linted.
pub fn find_lint_files(path: &Path) -> Vec<PathBuf> {
    crate::tools::fmt::find_alya_files(path)
}

/// Executes the CLI `alya lint` command.
pub fn run_lint_cli(path_opt: Option<&str>, fix: bool, check: bool) -> Result<(), String> {
    let target_str = path_opt.unwrap_or(".");
    let target_path = Path::new(target_str);
    if !target_path.exists() {
        return Err(format!("Error: Path does not exist '{}'", target_str));
    }

    let files = find_lint_files(target_path);
    if files.is_empty() {
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
                print!("{}", d.render(&content));
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
        }
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

    if check {
        Err(format!(
            "Lint check failed: {} warning(s) detected across {} file(s).",
            report.total_diagnostics, report.files_with_issues
        ))
    } else {
        println!(
            "\x1b[1;33mFound {} warning(s) across {} file(s).\x1b[0m",
            report.total_diagnostics, report.files_with_issues
        );
        Ok(())
    }
}
