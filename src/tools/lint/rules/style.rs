use std::path::Path;

use crate::ast::{Expr, Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

fn count_if_chain_branches(stmt: &Stmt) -> usize {
    if let Stmt::If { else_block, .. } = stmt.inner_stmt() {
        if let Some(eb) = else_block {
            if eb.len() == 1 {
                if let Stmt::If { .. } = eb[0].inner_stmt() {
                    return 1 + count_if_chain_branches(&eb[0]);
                }
            }
            return 2;
        }
        return 1;
    }
    0
}

fn check_stmt_style(
    stmt: &Stmt,
    is_elif_child: bool,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    let inner = stmt.inner_stmt();
    if let Stmt::If {
        then_block,
        else_block,
        ..
    } = inner
    {
        let branches = count_if_chain_branches(stmt);
        if !is_elif_child && branches >= 3 {
            // Find the line of the first 'if' token
            let if_tok = tokens
                .iter()
                .find(|t| matches!(t.token_type, TokenType::If));
            let line = if_tok.map(|t| t.line).unwrap_or(1);
            let col = if_tok.map(|t| t.column).unwrap_or(1);

            diags.push(LintDiagnostic {
                rule: "idiomatic-style".to_string(),
                severity: LintSeverity::Warning,
                message: format!(
                    "long 'if/elif' chain ({} branches); consider using 'when' pattern matching for clearer branching",
                    branches
                ),
                file_path: file_path.to_path_buf(),
                line,
                col,
                end_line: line,
                end_col: col + 2,
                help: Some("rewrite using 'when' pattern matching: 'when x is ... end'".to_string()),
                fix: None,
            });
        }

        // Check redundant boolean return: if cond return true else return false
        if then_block.len() == 1 {
            if let Some(eb) = else_block {
                if eb.len() == 1 {
                    if let (
                        Stmt::Return(Some(Expr::Identifier(t_val))),
                        Stmt::Return(Some(Expr::Identifier(f_val))),
                    ) = (then_block[0].inner_stmt(), eb[0].inner_stmt())
                    {
                        if t_val == "true" && f_val == "false" {
                            let if_tok = tokens
                                .iter()
                                .find(|t| matches!(t.token_type, TokenType::If));
                            let line = if_tok.map(|t| t.line).unwrap_or(1);
                            let col = if_tok.map(|t| t.column).unwrap_or(1);

                            diags.push(LintDiagnostic {
                                rule: "idiomatic-style".to_string(),
                                severity: LintSeverity::Info,
                                message: "redundant 'if/else' returning boolean literals; consider returning the condition directly".to_string(),
                                file_path: file_path.to_path_buf(),
                                line,
                                col,
                                end_line: line,
                                end_col: col + 2,
                                help: Some("replace with 'return condition'".to_string()),
                                fix: None,
                            });
                        }
                    }
                }
            }
        }

        for s in then_block {
            check_stmt_style(s, false, tokens, file_path, diags);
        }
        if let Some(eb) = else_block {
            for s in eb {
                let is_child_if = matches!(s.inner_stmt(), Stmt::If { .. });
                check_stmt_style(s, is_child_if, tokens, file_path, diags);
            }
        }
    } else {
        match inner {
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Function { body, .. } => {
                for s in body {
                    check_stmt_style(s, false, tokens, file_path, diags);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                for s in try_block {
                    check_stmt_style(s, false, tokens, file_path, diags);
                }
                for s in catch_block {
                    check_stmt_style(s, false, tokens, file_path, diags);
                }
                if let Some(fb) = finally_block {
                    for s in fb {
                        check_stmt_style(s, false, tokens, file_path, diags);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Checks for idiomatic style and anti-patterns (`idiomatic-style`).
pub fn check_idiomatic_style(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    for s in &program.statements {
        check_stmt_style(s, false, tokens, file_path, &mut diags);
    }
    diags
}
