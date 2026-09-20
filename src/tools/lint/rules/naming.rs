use std::path::Path;

use crate::ast::{Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

fn is_snake_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    // Snake case should not have uppercase characters and should not have double underscores
    !s.chars().any(|c| c.is_uppercase()) && !s.contains("__")
}

fn is_screaming_snake_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    s.chars()
        .all(|c| c.is_uppercase() || c.is_ascii_digit() || c == '_')
}

fn is_pascal_case(s: &str) -> bool {
    let s = s.trim_start_matches('_');
    if s.is_empty() {
        return true;
    }
    // Starts with uppercase, no underscores
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && !s.contains('_')
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn to_pascal_case(s: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in s.chars() {
        if ch == '_' {
            capitalize = true;
        } else if capitalize {
            out.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn find_ident_token(tokens: &[Token], ident: &str) -> Option<Token> {
    for tok in tokens {
        if let TokenType::Identifier(ref name) = tok.token_type {
            if name == ident {
                return Some(tok.clone());
            }
        }
    }
    None
}

fn check_stmt_naming(
    stmt: &Stmt,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match stmt.inner_stmt() {
        Stmt::Function { name, body, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if let Some((type_part, method_part)) =
                bare.split_once("__").or_else(|| bare.split_once('.'))
            {
                if !type_part.starts_with('_') && !is_pascal_case(type_part) {
                    let tok = find_ident_token(tokens, type_part);
                    let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                    let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                    diags.push(LintDiagnostic {
                        rule: "naming-convention".to_string(),
                        severity: LintSeverity::Info,
                        message: format!(
                            "struct method receiver type '{}' should follow 'PascalCase' naming convention",
                            type_part
                        ),
                        file_path: file_path.to_path_buf(),
                        line,
                        col,
                        end_line: line,
                        end_col: col + type_part.len(),
                        help: Some(format!("consider renaming to '{}'", to_pascal_case(type_part))),
                        fix: None,
                    });
                }
                if !method_part.starts_with('_') && !is_snake_case(method_part) {
                    let tok = find_ident_token(tokens, method_part);
                    let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                    let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                    diags.push(LintDiagnostic {
                        rule: "naming-convention".to_string(),
                        severity: LintSeverity::Info,
                        message: format!(
                            "method '{}' should follow 'snake_case' naming convention",
                            method_part
                        ),
                        file_path: file_path.to_path_buf(),
                        line,
                        col,
                        end_line: line,
                        end_col: col + method_part.len(),
                        help: Some(format!(
                            "consider renaming to '{}'",
                            to_snake_case(method_part)
                        )),
                        fix: None,
                    });
                }
            } else if !bare.starts_with('_') && !is_snake_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let len = bare.len();
                let suggested = to_snake_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "function '{}' should follow 'snake_case' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + len,
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }

            for s in body {
                check_stmt_naming(s, tokens, file_path, diags);
            }
        }
        Stmt::StructDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "struct '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::EnumDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "enum '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::InterfaceDef { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name);
            if !bare.starts_with('_') && !is_pascal_case(bare) {
                let tok = find_ident_token(tokens, bare);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);
                let suggested = to_pascal_case(bare);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "interface '{}' should follow 'PascalCase' naming convention",
                        bare
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + bare.len(),
                    help: Some(format!("consider renaming to '{}'", suggested)),
                    fix: None,
                });
            }
        }
        Stmt::Const { name, .. } => {
            if !name.starts_with('_') && !is_screaming_snake_case(name) && !is_snake_case(name) {
                let tok = find_ident_token(tokens, name);
                let line = tok.as_ref().map(|t| t.line).unwrap_or(1);
                let col = tok.as_ref().map(|t| t.column).unwrap_or(1);

                diags.push(LintDiagnostic {
                    rule: "naming-convention".to_string(),
                    severity: LintSeverity::Info,
                    message: format!(
                        "constant '{}' should follow 'SCREAMING_SNAKE_CASE' naming convention",
                        name
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col,
                    end_line: line,
                    end_col: col + name.len(),
                    help: Some(format!("consider renaming to '{}'", name.to_uppercase())),
                    fix: None,
                });
            }
        }
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt_naming(s, tokens, file_path, diags);
                }
            }
        }
        Stmt::While { body, .. }
        | Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. } => {
            for s in body {
                check_stmt_naming(s, tokens, file_path, diags);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            for s in catch_block {
                check_stmt_naming(s, tokens, file_path, diags);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    check_stmt_naming(s, tokens, file_path, diags);
                }
            }
        }
        _ => {}
    }
}

/// Checks naming conventions for functions, types, and constants.
pub fn check_naming_conventions(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for stmt in &program.statements {
        check_stmt_naming(stmt, tokens, file_path, &mut diags);
    }
    diags
}
