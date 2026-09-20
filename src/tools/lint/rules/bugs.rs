use std::path::Path;

use crate::ast::expr::{BinaryOp, Expr};
use crate::ast::{Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

fn is_comparison_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
    )
}

fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Identifier(name) => name.clone(),
        Expr::Number(n) => n.to_string(),
        Expr::Float(f) => f.to_string(),
        Expr::String(s) => format!("\"{}\"", s),
        Expr::Null => "null".to_string(),
        Expr::FieldAccess { object, field } => format!("{}.{}", expr_to_string(object), field),
        _ => "...".to_string(),
    }
}

fn find_expr_token_line(tokens: &[Token], expr_ident: &str) -> Option<usize> {
    for tok in tokens {
        if let TokenType::Identifier(ref name) = tok.token_type {
            if name == expr_ident {
                return Some(tok.line);
            }
        }
    }
    None
}

fn check_expr_self_comparison(
    expr: &Expr,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match expr {
        Expr::Binary { left, op, right } => {
            if is_comparison_op(*op) && left == right {
                if let Expr::Identifier(ref name) = **left {
                    let line = find_expr_token_line(tokens, name).unwrap_or(1);
                    diags.push(LintDiagnostic {
                        rule: "self-comparison".to_string(),
                        severity: LintSeverity::Warning,
                        message: format!(
                            "comparison of identical expressions '{}' is always redundant and likely a bug",
                            name
                        ),
                        file_path: file_path.to_path_buf(),
                        line,
                        col: 1,
                        end_line: line,
                        end_col: 10,
                        help: Some("check if one side was intended to compare a different variable".to_string()),
                        fix: None,
                    });
                }
            }
            check_expr_self_comparison(left, tokens, file_path, diags);
            check_expr_self_comparison(right, tokens, file_path, diags);
        }
        Expr::Unary { expr: inner, .. } => {
            check_expr_self_comparison(inner, tokens, file_path, diags);
        }
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            for a in args {
                check_expr_self_comparison(a, tokens, file_path, diags);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expr_self_comparison(condition, tokens, file_path, diags);
            check_expr_self_comparison(then_branch, tokens, file_path, diags);
            check_expr_self_comparison(else_branch, tokens, file_path, diags);
        }
        Expr::Array(items) => {
            for it in items {
                check_expr_self_comparison(it, tokens, file_path, diags);
            }
        }
        _ => {}
    }
}

fn is_constant_bool(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Number(n) if *n == 1.0 => Some(true),
        Expr::Number(n) if *n == 0.0 => Some(false),
        Expr::Identifier(s) if s == "true" => Some(true),
        Expr::Identifier(s) if s == "false" => Some(false),
        _ => None,
    }
}

fn check_stmt_bugs(
    stmt: &Stmt,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match stmt.inner_stmt() {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            if let Some(val) = is_constant_bool(condition) {
                diags.push(LintDiagnostic {
                    rule: "constant-condition".to_string(),
                    severity: LintSeverity::Warning,
                    message: format!(
                        "'if' condition is a constant boolean literal ('{}')",
                        if val { "true" } else { "false" }
                    ),
                    file_path: file_path.to_path_buf(),
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 3,
                    help: Some(
                        "remove the redundant condition or remove the dead branch".to_string(),
                    ),
                    fix: None,
                });
            }
            check_expr_self_comparison(condition, tokens, file_path, diags);
            for s in then_block {
                check_stmt_bugs(s, tokens, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt_bugs(s, tokens, file_path, diags);
                }
            }
        }
        Stmt::While { condition, body } => {
            if let Some(false) = is_constant_bool(condition) {
                diags.push(LintDiagnostic {
                    rule: "constant-condition".to_string(),
                    severity: LintSeverity::Warning,
                    message: "'while' loop condition is 'false'; body will never execute"
                        .to_string(),
                    file_path: file_path.to_path_buf(),
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: 6,
                    help: Some("remove the unreachable while loop".to_string()),
                    fix: None,
                });
            }
            check_expr_self_comparison(condition, tokens, file_path, diags);
            for s in body {
                check_stmt_bugs(s, tokens, file_path, diags);
            }
        }
        Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. }
        | Stmt::Function { body, .. } => {
            for s in body {
                check_stmt_bugs(s, tokens, file_path, diags);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                check_stmt_bugs(s, tokens, file_path, diags);
            }
            for s in catch_block {
                check_stmt_bugs(s, tokens, file_path, diags);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    check_stmt_bugs(s, tokens, file_path, diags);
                }
            }
        }
        Stmt::Expr(expr) => {
            check_expr_self_comparison(expr, tokens, file_path, diags);

            // Check useless expression
            let has_no_side_effects = matches!(
                expr,
                Expr::Number(_)
                    | Expr::Float(_)
                    | Expr::String(_)
                    | Expr::Identifier(_)
                    | Expr::Null
                    | Expr::Binary { .. }
                    | Expr::Unary { .. }
            );
            if has_no_side_effects {
                let desc = expr_to_string(expr);
                diags.push(LintDiagnostic {
                    rule: "useless-expression".to_string(),
                    severity: LintSeverity::Warning,
                    message: format!(
                        "statement has no side-effects; expression result '{}' is unused",
                        desc
                    ),
                    file_path: file_path.to_path_buf(),
                    line: 1,
                    col: 1,
                    end_line: 1,
                    end_col: desc.len().max(1),
                    help: Some(
                        "assign the result to a variable or remove the statement".to_string(),
                    ),
                    fix: None,
                });
            }
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
            check_expr_self_comparison(value, tokens, file_path, diags);
        }
        Stmt::Say(expr) | Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            check_expr_self_comparison(expr, tokens, file_path, diags);
        }
        _ => {}
    }
}

/// Checks for suspicious bugs: self-comparison, constant conditions, and useless expressions.
pub fn check_suspicious_bugs(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for stmt in &program.statements {
        check_stmt_bugs(stmt, tokens, file_path, &mut diags);
    }
    diags
}
