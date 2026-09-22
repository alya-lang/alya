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

/// Scans the token stream for redundant variable assignment immediately followed by returning that variable.
/// Pattern: `let <var> = <expr>` immediately followed by `return <var>`.
fn check_redundant_return_var(tokens: &[Token], file_path: &Path, diags: &mut Vec<LintDiagnostic>) {
    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i].token_type, TokenType::Let | TokenType::Const) {
            if let Some(Token {
                token_type: TokenType::Identifier(ref var_name),
                line: var_line,
                column: var_col,
            }) = tokens.get(i + 1)
            {
                if !var_name.starts_with('_') {
                    let mut j = i + 2;
                    let mut depth = 0;
                    let mut found_stmt_end = false;

                    while j < tokens.len() {
                        match tokens[j].token_type {
                            TokenType::LeftParen
                            | TokenType::LeftBracket
                            | TokenType::LeftBrace => {
                                depth += 1;
                            }
                            TokenType::RightParen
                            | TokenType::RightBracket
                            | TokenType::RightBrace
                                if depth > 0 =>
                            {
                                depth -= 1;
                            }
                            TokenType::Function
                            | TokenType::If
                            | TokenType::While
                            | TokenType::For
                            | TokenType::Repeat
                            | TokenType::Try
                            | TokenType::When => {
                                depth += 1;
                            }
                            TokenType::End if depth > 0 => {
                                depth -= 1;
                            }
                            TokenType::Newline if depth == 0 => {
                                found_stmt_end = true;
                                break;
                            }
                            _ => {}
                        }
                        j += 1;
                    }

                    if found_stmt_end {
                        let mut next_stmt_idx = j + 1;
                        while next_stmt_idx < tokens.len()
                            && matches!(tokens[next_stmt_idx].token_type, TokenType::Newline)
                        {
                            next_stmt_idx += 1;
                        }

                        if next_stmt_idx < tokens.len()
                            && matches!(tokens[next_stmt_idx].token_type, TokenType::Return)
                        {
                            if let Some(Token {
                                token_type: TokenType::Identifier(ref ret_name),
                                ..
                            }) = tokens.get(next_stmt_idx + 1)
                            {
                                if ret_name == var_name {
                                    let term_idx = next_stmt_idx + 2;
                                    let is_terminated = term_idx >= tokens.len()
                                        || matches!(
                                            tokens[term_idx].token_type,
                                            TokenType::Newline | TokenType::End | TokenType::Eof
                                        );

                                    if is_terminated {
                                        let let_tok = &tokens[i];
                                        let ret_tok = &tokens[next_stmt_idx];
                                        let len = var_name.len();

                                        // Reconstruct expression tokens between '=' and statement newline
                                        let mut expr_tokens = Vec::new();
                                        let mut k = i + 2;
                                        if k < tokens.len()
                                            && matches!(tokens[k].token_type, TokenType::Colon)
                                        {
                                            k += 1;
                                            while k < tokens.len()
                                                && !matches!(
                                                    tokens[k].token_type,
                                                    TokenType::Assign
                                                )
                                            {
                                                k += 1;
                                            }
                                        }
                                        if k < tokens.len()
                                            && matches!(tokens[k].token_type, TokenType::Assign)
                                        {
                                            k += 1;
                                            while k < j {
                                                expr_tokens.push(&tokens[k]);
                                                k += 1;
                                            }
                                        }

                                        let fix = if !expr_tokens.is_empty() {
                                            let mut expr_str = String::new();
                                            for (t_idx, tok) in expr_tokens.iter().enumerate() {
                                                if t_idx > 0 {
                                                    match tok.token_type {
                                                        TokenType::Comma
                                                        | TokenType::RightParen
                                                        | TokenType::RightBracket
                                                        | TokenType::LeftParen
                                                        | TokenType::LeftBracket
                                                        | TokenType::Dot => {}
                                                        _ => {
                                                            let prev = &expr_tokens[t_idx - 1];
                                                            match prev.token_type {
                                                                TokenType::LeftParen
                                                                | TokenType::LeftBracket
                                                                | TokenType::Dot => {}
                                                                _ => expr_str.push(' '),
                                                            }
                                                        }
                                                    }
                                                }
                                                match &tok.token_type {
                                                    TokenType::Identifier(s) => {
                                                        expr_str.push_str(s)
                                                    }
                                                    TokenType::Number(n) => {
                                                        expr_str.push_str(&n.to_string())
                                                    }
                                                    TokenType::Float(f) => {
                                                        expr_str.push_str(&f.to_string())
                                                    }
                                                    TokenType::String(s) => {
                                                        expr_str.push('"');
                                                        expr_str.push_str(s);
                                                        expr_str.push('"');
                                                    }
                                                    TokenType::Plus => expr_str.push('+'),
                                                    TokenType::Minus => expr_str.push('-'),
                                                    TokenType::Multiply => expr_str.push('*'),
                                                    TokenType::Divide => expr_str.push('/'),
                                                    TokenType::LeftParen => expr_str.push('('),
                                                    TokenType::RightParen => expr_str.push(')'),
                                                    TokenType::LeftBracket => expr_str.push('['),
                                                    TokenType::RightBracket => expr_str.push(']'),
                                                    TokenType::LeftBrace => expr_str.push('{'),
                                                    TokenType::RightBrace => expr_str.push('}'),
                                                    TokenType::Comma => expr_str.push(','),
                                                    TokenType::Dot => expr_str.push('.'),
                                                    TokenType::True => expr_str.push_str("true"),
                                                    TokenType::False => expr_str.push_str("false"),
                                                    TokenType::Null => expr_str.push_str("null"),
                                                    _ => {}
                                                }
                                            }

                                            if !expr_str.is_empty() {
                                                let indent =
                                                    " ".repeat(let_tok.column.saturating_sub(1));
                                                Some(crate::tools::lint::types::LintFix {
                                                    description: format!(
                                                        "Return expression directly: 'return {}'",
                                                        expr_str
                                                    ),
                                                    replacement: format!(
                                                        "{}return {}\n",
                                                        indent, expr_str
                                                    ),
                                                    start_line: let_tok.line,
                                                    start_col: 1,
                                                    end_line: ret_tok.line + 1,
                                                    end_col: 1,
                                                })
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        };

                                        diags.push(LintDiagnostic {
                                            rule: "idiomatic-style".to_string(),
                                            severity: LintSeverity::Info,
                                            message: format!(
                                                "redundant variable assignment '{}' immediately before 'return'",
                                                var_name
                                            ),
                                            file_path: file_path.to_path_buf(),
                                            line: let_tok.line,
                                            col: let_tok.column,
                                            end_line: *var_line,
                                            end_col: *var_col + len,
                                            help: Some(
                                                "consider returning the expression directly: 'return ...'"
                                                    .to_string(),
                                            ),
                                            fix,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
}

/// Checks expressions for redundant boolean comparisons (`== true`, `== false`, etc.).
fn check_expr_boolean_comparisons(expr: &Expr, file_path: &Path, diags: &mut Vec<LintDiagnostic>) {
    match expr {
        Expr::Binary { left, op, right } => {
            if matches!(
                *op,
                crate::ast::expr::BinaryOp::Equal | crate::ast::expr::BinaryOp::NotEqual
            ) {
                let left_is_bool = match **left {
                    Expr::Identifier(ref s) => s == "true" || s == "false",
                    _ => false,
                };
                let right_is_bool = match **right {
                    Expr::Identifier(ref s) => s == "true" || s == "false",
                    _ => false,
                };

                if left_is_bool || right_is_bool {
                    let bool_val = if right_is_bool {
                        match **right {
                            Expr::Identifier(ref s) => s.as_str(),
                            _ => "",
                        }
                    } else {
                        match **left {
                            Expr::Identifier(ref s) => s.as_str(),
                            _ => "",
                        }
                    };

                    let is_eq = *op == crate::ast::expr::BinaryOp::Equal;
                    let (pattern_desc, suggestion) = match (is_eq, bool_val) {
                        (true, "true") => ("'== true'", "use the condition directly"),
                        (true, "false") => ("'== false'", "use negation '!condition'"),
                        (false, "true") => ("'!= true'", "use negation '!condition'"),
                        (false, "false") => ("'!= false'", "use the condition directly"),
                        _ => ("", ""),
                    };

                    if !pattern_desc.is_empty() {
                        diags.push(LintDiagnostic {
                            rule: "idiomatic-style".to_string(),
                            severity: LintSeverity::Info,
                            message: format!(
                                "redundant comparison with boolean literal ({}); consider simplifying",
                                pattern_desc
                            ),
                            file_path: file_path.to_path_buf(),
                            line: 1,
                            col: 1,
                            end_line: 1,
                            end_col: 3,
                            help: Some(suggestion.to_string()),
                            fix: None,
                        });
                    }
                }
            }
            check_expr_boolean_comparisons(left, file_path, diags);
            check_expr_boolean_comparisons(right, file_path, diags);
        }
        Expr::Unary { expr: inner, .. } => {
            check_expr_boolean_comparisons(inner, file_path, diags);
        }
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            for a in args {
                check_expr_boolean_comparisons(a, file_path, diags);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expr_boolean_comparisons(condition, file_path, diags);
            check_expr_boolean_comparisons(then_branch, file_path, diags);
            check_expr_boolean_comparisons(else_branch, file_path, diags);
        }
        _ => {}
    }
}

fn check_stmt_boolean_comparisons(stmt: &Stmt, file_path: &Path, diags: &mut Vec<LintDiagnostic>) {
    match stmt.inner_stmt() {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            check_expr_boolean_comparisons(condition, file_path, diags);
            for s in then_block {
                check_stmt_boolean_comparisons(s, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt_boolean_comparisons(s, file_path, diags);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_boolean_comparisons(condition, file_path, diags);
            for s in body {
                check_stmt_boolean_comparisons(s, file_path, diags);
            }
        }
        Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. }
        | Stmt::Function { body, .. } => {
            for s in body {
                check_stmt_boolean_comparisons(s, file_path, diags);
            }
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
            check_expr_boolean_comparisons(value, file_path, diags);
        }
        Stmt::Say(expr) | Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) | Stmt::Expr(expr) => {
            check_expr_boolean_comparisons(expr, file_path, diags);
        }
        _ => {}
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
    check_redundant_return_var(tokens, file_path, &mut diags);
    for s in &program.statements {
        check_boolean_returns(s, file_path, &mut diags);
        check_stmt_boolean_comparisons(s, file_path, &mut diags);
    }
    diags
}
