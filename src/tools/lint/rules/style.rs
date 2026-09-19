use std::path::Path;

use crate::ast::{Expr, Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintSeverity};

/// Scans the token stream for long if/elif chains at each exact 'if' token location.
fn check_token_if_chains(tokens: &[Token], file_path: &Path, diags: &mut Vec<LintDiagnostic>) {
    for (i, tok) in tokens.iter().enumerate() {
        if matches!(tok.token_type, TokenType::If) {
            // Skip 'else if' where 'else' immediately preceded it
            if i > 0 && matches!(tokens[i - 1].token_type, TokenType::Else) {
                continue;
            }

            let start_line = tok.line;
            let start_col = tok.column;

            let mut depth = 1;
            let mut elif_count = 0;
            let mut has_else = false;

            for next_tok in &tokens[i + 1..] {
                match next_tok.token_type {
                    TokenType::If
                    | TokenType::While
                    | TokenType::For
                    | TokenType::Repeat
                    | TokenType::Try
                    | TokenType::When
                    | TokenType::Function
                    | TokenType::Struct
                    | TokenType::Enum
                    | TokenType::Interface => {
                        depth += 1;
                    }
                    TokenType::End => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    TokenType::Elif if depth == 1 => {
                        elif_count += 1;
                    }
                    TokenType::Else if depth == 1 => {
                        has_else = true;
                    }
                    TokenType::Eof => break,
                    _ => {}
                }
            }

            let total_branches = 1 + elif_count + if has_else { 1 } else { 0 };

            // Recommend 'when' if there are 2 or more 'elif' branches
            if elif_count >= 2 {
                diags.push(LintDiagnostic {
                    rule: "idiomatic-style".to_string(),
                    severity: LintSeverity::Warning,
                    message: format!(
                        "long 'if/elif' chain ({} branches); consider using 'when' pattern matching for clearer branching",
                        total_branches
                    ),
                    file_path: file_path.to_path_buf(),
                    line: start_line,
                    col: start_col,
                    end_line: start_line,
                    end_col: start_col + 2,
                    help: Some("rewrite using 'when' pattern matching: 'when x is ... end'".to_string()),
                    fix: None,
                });
            }
        }
    }
}

/// Checks AST for redundant boolean returns like `if cond return true else return false`.
fn check_boolean_returns(stmt: &Stmt, file_path: &Path, diags: &mut Vec<LintDiagnostic>) {
    let inner = stmt.inner_stmt();
    if let Stmt::If {
        then_block,
        else_block,
        ..
    } = inner
    {
        if then_block.len() == 1 {
            if let Some(eb) = else_block {
                if eb.len() == 1 {
                    if let (
                        Stmt::Return(Some(Expr::Identifier(t_val))),
                        Stmt::Return(Some(Expr::Identifier(f_val))),
                    ) = (then_block[0].inner_stmt(), eb[0].inner_stmt())
                    {
                        if t_val == "true" && f_val == "false" {
                            diags.push(LintDiagnostic {
                                rule: "idiomatic-style".to_string(),
                                severity: LintSeverity::Info,
                                message: "redundant 'if/else' returning boolean literals; consider returning the condition directly".to_string(),
                                file_path: file_path.to_path_buf(),
                                line: 1,
                                col: 1,
                                end_line: 1,
                                end_col: 3,
                                help: Some("replace with 'return condition'".to_string()),
                                fix: None,
                            });
                        }
                    }
                }
            }
        }

        for s in then_block {
            check_boolean_returns(s, file_path, diags);
        }
        if let Some(eb) = else_block {
            for s in eb {
                check_boolean_returns(s, file_path, diags);
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
                    check_boolean_returns(s, file_path, diags);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                for s in try_block {
                    check_boolean_returns(s, file_path, diags);
                }
                for s in catch_block {
                    check_boolean_returns(s, file_path, diags);
                }
                if let Some(fb) = finally_block {
                    for s in fb {
                        check_boolean_returns(s, file_path, diags);
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
    check_token_if_chains(tokens, file_path, &mut diags);
    for s in &program.statements {
        check_boolean_returns(s, file_path, &mut diags);
    }
    diags
}
