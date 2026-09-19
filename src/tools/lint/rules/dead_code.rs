use std::path::Path;

use crate::ast::{Program, Stmt};
use crate::lexer::{Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintFix, LintSeverity};

fn is_unconditional_jump(stmt: &Stmt) -> Option<&'static str> {
    match stmt.inner_stmt() {
        Stmt::Return(_) => Some("return"),
        Stmt::Throw(_) => Some("throw"),
        Stmt::Break => Some("break"),
        Stmt::Continue => Some("continue"),
        _ => None,
    }
}

/// Finds the line and column of the first token corresponding to a dead statement.
fn find_next_stmt_token_after(tokens: &[Token], after_line: usize) -> Option<Token> {
    for tok in tokens {
        if tok.line > after_line && !matches!(tok.token_type, TokenType::Newline | TokenType::End | TokenType::Eof) {
            return Some(tok.clone());
        }
    }
    None
}

/// Finds the line of a jump token in tokens.
fn find_jump_token_line(tokens: &[Token], jump_kind: &str) -> Option<usize> {
    for tok in tokens {
        let matches = match (jump_kind, &tok.token_type) {
            ("return", TokenType::Return) => true,
            ("throw", TokenType::Throw) => true,
            ("break", TokenType::Break) => true,
            ("continue", TokenType::Continue) => true,
            _ => false,
        };
        if matches {
            return Some(tok.line);
        }
    }
    None
}

fn check_block_dead_code(
    block: &[Stmt],
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    let mut jump_found: Option<(&'static str, usize)> = None;

    for stmt in block {
        if let Some((jump_name, jump_line)) = jump_found {
            // This statement is dead code!
            let dead_tok = find_next_stmt_token_after(tokens, jump_line);
            let line = dead_tok.as_ref().map(|t| t.line).unwrap_or(jump_line + 1);
            let col = dead_tok.as_ref().map(|t| t.column).unwrap_or(1);

            diags.push(LintDiagnostic {
                rule: "dead-code".to_string(),
                severity: LintSeverity::Warning,
                message: format!("unreachable statement following '{}'", jump_name),
                file_path: file_path.to_path_buf(),
                line,
                col,
                end_line: line,
                end_col: col + 1,
                help: Some("remove unreachable code".to_string()),
                fix: Some(LintFix {
                    description: "Remove unreachable statement".to_string(),
                    replacement: String::new(),
                    start_line: line,
                    start_col: 1,
                    end_line: line + 1,
                    end_col: 1,
                }),
            });
            continue;
        }

        if let Some(jump_name) = is_unconditional_jump(stmt) {
            let line = find_jump_token_line(tokens, jump_name).unwrap_or(1);
            jump_found = Some((jump_name, line));
        }

        // Recursively inspect inner blocks of this statement
        match stmt.inner_stmt() {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                check_block_dead_code(then_block, tokens, file_path, diags);
                if let Some(eb) = else_block {
                    check_block_dead_code(eb, tokens, file_path, diags);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Function { body, .. } => {
                check_block_dead_code(body, tokens, file_path, diags);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                check_block_dead_code(try_block, tokens, file_path, diags);
                check_block_dead_code(catch_block, tokens, file_path, diags);
                if let Some(fb) = finally_block {
                    check_block_dead_code(fb, tokens, file_path, diags);
                }
            }
            _ => {}
        }
    }
}

/// Checks for unreachable dead code following return, throw, break, or continue (`dead-code`).
pub fn check_dead_code(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    check_block_dead_code(&program.statements, tokens, file_path, &mut diags);
    diags
}
