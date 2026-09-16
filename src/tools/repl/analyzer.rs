use crate::ast::{Expr, Stmt};
use crate::lexer::{Lexer, TokenType};

/// Checks whether an input buffer represents an incomplete statement or block
/// that requires further lines of input before being compiled and executed.
pub fn is_input_incomplete(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return false;
    }

    let mut lexer = Lexer::new(input);
    let tokens = match lexer.tokenize() {
        Ok(t) => t,
        Err(_) => {
            // Lexer errors usually indicate unclosed string literals or multiline quotes
            return true;
        }
    };

    let mut block_depth: i32 = 0;
    let mut paren_depth: i32 = 0;

    for token in &tokens {
        match &token.token_type {
            TokenType::Function
            | TokenType::If
            | TokenType::While
            | TokenType::Repeat
            | TokenType::For
            | TokenType::When
            | TokenType::Try
            | TokenType::Struct => {
                block_depth += 1;
            }
            TokenType::Identifier(s) if s == "fn" => {
                block_depth += 1;
            }
            TokenType::End => {
                block_depth = (block_depth - 1).max(0);
            }
            TokenType::LeftParen | TokenType::LeftBracket | TokenType::LeftBrace => {
                paren_depth += 1;
            }
            TokenType::RightParen | TokenType::RightBracket | TokenType::RightBrace => {
                paren_depth = (paren_depth - 1).max(0);
            }
            _ => {}
        }
    }

    if block_depth > 0 || paren_depth > 0 {
        return true;
    }

    // Check if the last meaningful token is a continuation operator
    let last_meaningful_token = tokens
        .iter()
        .rev()
        .find(|t| !matches!(t.token_type, TokenType::Newline | TokenType::Eof));

    if let Some(t) = last_meaningful_token {
        match &t.token_type {
            TokenType::Comma
            | TokenType::Plus
            | TokenType::Minus
            | TokenType::Multiply
            | TokenType::Divide
            | TokenType::Modulo
            | TokenType::Assign
            | TokenType::Equal
            | TokenType::NotEqual
            | TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::And
            | TokenType::Or
            | TokenType::BitAnd
            | TokenType::BitOr
            | TokenType::BitXor
            | TokenType::Colon
            | TokenType::Question
            | TokenType::FatArrow
            | TokenType::Arrow
            | TokenType::In
            | TokenType::Is
            | TokenType::DotDot
            | TokenType::DotDotDot
            | TokenType::Not => return true,
            _ => {}
        }
    }

    false
}

/// Recursively checks if an expression invokes the `ask` builtin or any custom function
/// that references `ask`.
pub fn expr_contains_ask(expr: &Expr, session_funcs: &[(String, String)]) -> bool {
    match expr {
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare == "ask" {
                return true;
            }
            if session_funcs
                .iter()
                .any(|(fn_name, fn_code)| fn_name == bare && fn_code.contains("ask"))
            {
                return true;
            }
            args.iter().any(|a| expr_contains_ask(a, session_funcs))
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_ask(left, session_funcs) || expr_contains_ask(right, session_funcs)
        }
        Expr::Unary { expr, .. } => expr_contains_ask(expr, session_funcs),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_ask(condition, session_funcs)
                || expr_contains_ask(then_branch, session_funcs)
                || expr_contains_ask(else_branch, session_funcs)
        }
        Expr::NullCoalesce { value, default } => {
            expr_contains_ask(value, session_funcs) || expr_contains_ask(default, session_funcs)
        }
        Expr::Array(items) => items.iter().any(|i| expr_contains_ask(i, session_funcs)),
        Expr::Map(pairs) => pairs.iter().any(|(k, v)| {
            expr_contains_ask(k, session_funcs) || expr_contains_ask(v, session_funcs)
        }),
        Expr::Index { array, index } => {
            expr_contains_ask(array, session_funcs) || expr_contains_ask(index, session_funcs)
        }
        Expr::FieldAccess { object, .. } => expr_contains_ask(object, session_funcs),
        Expr::StructInit { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_contains_ask(v, session_funcs)),
        Expr::InterpolatedString(parts) => {
            parts.iter().any(|p| expr_contains_ask(p, session_funcs))
        }
        _ => false,
    }
}

/// Recursively checks if any statement in a statement list invokes `ask`.
pub fn stmt_contains_ask(stmt: &Stmt, session_funcs: &[(String, String)]) -> bool {
    match stmt {
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
            expr_contains_ask(value, session_funcs)
        }
        Stmt::Expr(expr) | Stmt::Say(expr) => expr_contains_ask(expr, session_funcs),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_contains_ask(condition, session_funcs)
                || then_block
                    .iter()
                    .any(|s| stmt_contains_ask(s, session_funcs))
                || else_block
                    .as_ref()
                    .is_some_and(|b| b.iter().any(|s| stmt_contains_ask(s, session_funcs)))
        }
        Stmt::While { condition, body } => {
            expr_contains_ask(condition, session_funcs)
                || body.iter().any(|s| stmt_contains_ask(s, session_funcs))
        }
        Stmt::Repeat { body } => body.iter().any(|s| stmt_contains_ask(s, session_funcs)),
        Stmt::For {
            start, end, body, ..
        } => {
            expr_contains_ask(start, session_funcs)
                || expr_contains_ask(end, session_funcs)
                || body.iter().any(|s| stmt_contains_ask(s, session_funcs))
        }
        Stmt::ForEach { iterable, body, .. } => {
            expr_contains_ask(iterable, session_funcs)
                || body.iter().any(|s| stmt_contains_ask(s, session_funcs))
        }
        _ => false,
    }
}
