use std::collections::HashSet;
use std::path::Path;

use crate::ast::expr::Expr;
use crate::ast::{Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

fn is_float_target(target: &str) -> bool {
    matches!(target, "float" | "f64" | "f32")
}

fn bare_name(name: &str) -> &str {
    let ns_bare = name.rsplit("::").next().unwrap_or(name);
    ns_bare.rsplit("__").next().unwrap_or(ns_bare)
}

/// Functions in this file promising a float result: calls into them
/// are statically proven, so `is float` on their result is exact.
fn float_fn_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name, return_type, ..
        } = stmt.inner_stmt()
        {
            if return_type.as_deref().is_some_and(is_float_target) {
                names.insert(name.clone());
                names.insert(bare_name(name).to_string());
            }
        }
    }
    names
}

fn subject_token_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(name) => Some(name.clone()),
        Expr::Call { name, .. } | Expr::OptionalCall { callee: name, .. } => {
            Some(bare_name(name).to_string())
        }
        Expr::Index { array, .. } | Expr::OptionalIndex { array, .. } => subject_token_name(array),
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            subject_token_name(object)
        }
        Expr::ForceUnwrap(inner) => subject_token_name(inner),
        _ => None,
    }
}

fn find_token_line(tokens: &[Token], name: &str) -> Option<usize> {
    for tok in tokens {
        if let TokenType::Identifier(ref id) = tok.token_type {
            if id == name {
                return Some(tok.line);
            }
        }
    }
    None
}

fn find_is_line(tokens: &[Token], after_line: usize) -> Option<usize> {
    // Prefer the `is` keyword at or after the subject: the subject's
    // own identifier may first occur far earlier (e.g. a parameter).
    let mut fallback = None;
    for tok in tokens {
        if tok.token_type == TokenType::Is {
            if tok.line >= after_line {
                return Some(tok.line);
            }
            fallback = Some(tok.line);
        }
    }
    fallback
}

fn check_expr(
    expr: &Expr,
    float_fns: &HashSet<String>,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match expr {
        Expr::TypeCheck {
            expr: sub, target, ..
        } if is_float_target(target) => {
            let proven = match sub.as_ref() {
                // A float literal trivially is float.
                Expr::Float(_) => true,
                // Calls into `-> float` functions of this file.
                Expr::Call { name, .. } | Expr::OptionalCall { callee: name, .. } => {
                    float_fns.contains(name) || float_fns.contains(bare_name(name))
                }
                // Int/string literals never need a classifier either;
                // they belong to constant-folding, not this rule.
                Expr::Number(_) | Expr::String(_) => true,
                _ => false,
            };
            if !proven {
                let label = subject_token_name(sub).unwrap_or_else(|| "...".to_string());
                let subj_line = find_token_line(tokens, &label).unwrap_or(1);
                let line = find_is_line(tokens, subj_line).unwrap_or(subj_line);
                diags.push(LintDiagnostic {
                    rule: "dynamic-is-float".to_string(),
                    severity: LintSeverity::Warning,
                    message: format!(
                        "`is float` on '{}' is best-effort: a float sharing an int's bit pattern reads as not float (alya-lang/alya#55)",
                        label
                    ),
                    file_path: file_path.to_path_buf(),
                    line,
                    col: 1,
                    end_line: line,
                    end_col: 10,
                    help: Some(
                        "annotate the source with `-> float`, or check a literal or statically-proven value instead"
                            .to_string(),
                    ),
                    fix: None,
                });
            }
            check_expr(sub, float_fns, tokens, file_path, diags);
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, float_fns, tokens, file_path, diags);
            check_expr(right, float_fns, tokens, file_path, diags);
        }
        Expr::Unary { expr: inner, .. }
        | Expr::Cast { expr: inner, .. }
        | Expr::TypeCheck { expr: inner, .. }
        | Expr::ForceUnwrap(inner) => {
            check_expr(inner, float_fns, tokens, file_path, diags);
        }
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            for arg in args {
                check_expr(arg, float_fns, tokens, file_path, diags);
            }
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            for item in items {
                check_expr(item, float_fns, tokens, file_path, diags);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            check_expr(array, float_fns, tokens, file_path, diags);
            check_expr(index, float_fns, tokens, file_path, diags);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            check_expr(object, float_fns, tokens, file_path, diags);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                check_expr(val, float_fns, tokens, file_path, diags);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                check_expr(k, float_fns, tokens, file_path, diags);
                check_expr(v, float_fns, tokens, file_path, diags);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expr(condition, float_fns, tokens, file_path, diags);
            check_expr(then_branch, float_fns, tokens, file_path, diags);
            check_expr(else_branch, float_fns, tokens, file_path, diags);
        }
        Expr::NullCoalesce { value, default } => {
            check_expr(value, float_fns, tokens, file_path, diags);
            check_expr(default, float_fns, tokens, file_path, diags);
        }
        _ => {}
    }
}

fn check_stmt(
    stmt: &Stmt,
    float_fns: &HashSet<String>,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match stmt.inner_stmt() {
        Stmt::Expr(e) | Stmt::Say(e) | Stmt::Return(Some(e)) | Stmt::Throw(Some(e)) => {
            check_expr(e, float_fns, tokens, file_path, diags);
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
            check_expr(value, float_fns, tokens, file_path, diags);
        }
        Stmt::Assign { value, .. } => {
            check_expr(value, float_fns, tokens, file_path, diags);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            check_expr(array, float_fns, tokens, file_path, diags);
            check_expr(index, float_fns, tokens, file_path, diags);
            check_expr(value, float_fns, tokens, file_path, diags);
        }
        Stmt::FieldAssign { object, value, .. } => {
            check_expr(object, float_fns, tokens, file_path, diags);
            check_expr(value, float_fns, tokens, file_path, diags);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            check_expr(condition, float_fns, tokens, file_path, diags);
            for s in then_block {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt(s, float_fns, tokens, file_path, diags);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr(condition, float_fns, tokens, file_path, diags);
            for s in body {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            check_expr(start, float_fns, tokens, file_path, diags);
            check_expr(end, float_fns, tokens, file_path, diags);
            for s in body {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            check_expr(iterable, float_fns, tokens, file_path, diags);
            for s in body {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
        }
        Stmt::Defer(inner) => {
            check_stmt(inner, float_fns, tokens, file_path, diags);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
            for s in catch_block {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    check_stmt(s, float_fns, tokens, file_path, diags);
                }
            }
        }
        Stmt::Function { body, .. } => {
            for s in body {
                check_stmt(s, float_fns, tokens, file_path, diags);
            }
        }
        Stmt::Pub(inner) => {
            check_stmt(inner, float_fns, tokens, file_path, diags);
        }
        _ => {}
    }
}

/// Warns on `is float` (and `is not float`) applied to values whose
/// kind is not statically provable (alya-lang/alya#55-A): map/array
/// reads, untyped calls, and bare identifiers. Floats sharing an
/// int's bit pattern are indistinguishable at runtime, so the check
/// is best-effort and may read `not float` for a real float.
/// Float literals and same-file `-> float` calls are exact and stay
/// silent; cross-file annotations are out of scope (documented).
pub fn check_dynamic_is_float(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let float_fns = float_fn_names(program);
    let mut diags = Vec::new();
    for stmt in &program.statements {
        check_stmt(stmt, &float_fns, tokens, file_path, &mut diags);
    }
    diags
}
