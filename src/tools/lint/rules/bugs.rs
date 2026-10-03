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
        Expr::Number(n) if *n == 1 => Some(true),
        Expr::Number(n) if *n == 0 => Some(false),
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

/// Tracks the enclosing method (if any) while walking statements.
#[derive(Default)]
struct MethodCtx {
    /// Full name of the enclosing `Type.method`, if inside one.
    enclosing_full: Option<String>,
    /// Bare name of the enclosing method.
    enclosing_bare: Option<String>,
    /// Bare names of free (non-method) functions defined in this file.
    /// A same-file free target means the call delegates (via the compiler
    /// delegation guard) instead of recursing forever.
    free_fns: std::collections::HashSet<String>,
    /// Occurrence counter per callee name, for roughly-stable line numbers.
    occurrences: std::collections::HashMap<String, usize>,
}

/// Last identifier segment of a possibly qualified name. Methods are
/// stored `__`-joined by the parser (`Box__is_positive` -> `is_positive`);
/// dotted (`Regex.is_match`) and namespaced (`re::is_match`) spellings are
/// handled too.
fn method_bare(name: &str) -> &str {
    let after_dot = name.rsplit('.').next().unwrap_or(name);
    after_dot.rsplit("__").next().unwrap_or(after_dot)
}

/// True for a method definition name (as opposed to a free function).
fn is_method_name(name: &str) -> bool {
    name.contains('.') || name.contains("__")
}

/// A bare `name(self, ...)` call inside the same-named method, with
/// all-identifier arguments, re-enters that same body with identical
/// values and never terminates (UFCS resolves it back to the method
/// instead of a same-named free function). Genuine recursion (changed
/// or computed arguments) and explicitly qualified calls are untouched.
fn check_expr_self_recursion(
    expr: &Expr,
    ctx: &mut MethodCtx,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match expr {
        Expr::Call { name, args } => {
            check_call_self_recursion(name, args, ctx, tokens, file_path, diags);
            for a in args {
                check_expr_self_recursion(a, ctx, tokens, file_path, diags);
            }
        }
        Expr::OptionalCall { callee, args } => {
            check_call_self_recursion(callee, args, ctx, tokens, file_path, diags);
            for a in args {
                check_expr_self_recursion(a, ctx, tokens, file_path, diags);
            }
        }
        Expr::Binary { left, right, .. } => {
            check_expr_self_recursion(left, ctx, tokens, file_path, diags);
            check_expr_self_recursion(right, ctx, tokens, file_path, diags);
        }
        Expr::Unary { expr: inner, .. } => {
            check_expr_self_recursion(inner, ctx, tokens, file_path, diags);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expr_self_recursion(condition, ctx, tokens, file_path, diags);
            check_expr_self_recursion(then_branch, ctx, tokens, file_path, diags);
            check_expr_self_recursion(else_branch, ctx, tokens, file_path, diags);
        }
        Expr::Array(items) => {
            for it in items {
                check_expr_self_recursion(it, ctx, tokens, file_path, diags);
            }
        }
        Expr::Index { array, index } => {
            check_expr_self_recursion(array, ctx, tokens, file_path, diags);
            check_expr_self_recursion(index, ctx, tokens, file_path, diags);
        }
        Expr::FieldAccess { object, .. } => {
            check_expr_self_recursion(object, ctx, tokens, file_path, diags);
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                check_expr_self_recursion(k, ctx, tokens, file_path, diags);
                check_expr_self_recursion(v, ctx, tokens, file_path, diags);
            }
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields {
                check_expr_self_recursion(v, ctx, tokens, file_path, diags);
            }
        }
        Expr::InterpolatedString(parts) => {
            for p in parts {
                check_expr_self_recursion(p, ctx, tokens, file_path, diags);
            }
        }
        Expr::NullCoalesce { value, default } => {
            check_expr_self_recursion(value, ctx, tokens, file_path, diags);
            check_expr_self_recursion(default, ctx, tokens, file_path, diags);
        }
        Expr::ForceUnwrap(inner) => {
            check_expr_self_recursion(inner, ctx, tokens, file_path, diags);
        }
        _ => {}
    }
}

fn nth_ident_token_line(tokens: &[Token], ident: &str, n: usize) -> Option<usize> {
    let mut seen = 0usize;
    for tok in tokens {
        if let TokenType::Identifier(ref name) = tok.token_type {
            if name == ident {
                seen += 1;
                if seen == n {
                    return Some(tok.line);
                }
            }
        }
    }
    None
}

fn check_call_self_recursion(
    name: &str,
    args: &[Expr],
    ctx: &mut MethodCtx,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    let enclosing_full = match ctx.enclosing_full.clone() {
        Some(e) => e,
        None => return,
    };
    let enclosing = match ctx.enclosing_bare.clone() {
        Some(e) => e,
        None => return,
    };
    let bare = method_bare(name);
    if name.contains('.') || name.contains("::") || name.contains("__") || bare != enclosing {
        return;
    }
    let forwards_self = args
        .first()
        .is_some_and(|a| matches!(a, Expr::Identifier(id) if id == "self"))
        && args.iter().all(|a| matches!(a, Expr::Identifier(_)));
    if !forwards_self {
        return;
    }
    // A same-file free function with that name means the call delegates
    // (compiler guard) instead of recursing: still fragile and worth an
    // informational nudge, but not a likely-infinite loop.
    // Display `Box__is_match` as `Box.is_match`.
    let display_method = enclosing_full.replace("__", ".");
    let delegated = ctx.free_fns.contains(bare);
    let (severity, message, help) = if delegated {
        (
            LintSeverity::Info,
            format!(
                "call '{name}(self, ...)' inside method '{display_method}' delegates to the same-file free function via the compiler delegation guard; qualify the call explicitly for clarity and older compilers"
            ),
            "qualify the call (e.g. with the module path) so the delegation is explicit",
        )
    } else {
        (
            LintSeverity::Warning,
            format!(
                "call '{name}(self, ...)' inside method '{display_method}' resolves back to this same method and never terminates; call the free function explicitly or rename one side"
            ),
            "if delegation to a same-named free function was intended, qualify the call so it does not resolve to this method",
        )
    };
    let count = ctx.occurrences.entry(name.to_string()).or_insert(0);
    *count += 1;
    let line = nth_ident_token_line(tokens, name, *count).unwrap_or(1);
    diags.push(LintDiagnostic {
        rule: "method-self-recursion".to_string(),
        severity,
        message,
        file_path: file_path.to_path_buf(),
        line,
        col: 1,
        end_line: line,
        end_col: name.len().max(1),
        help: Some(help.to_string()),
        fix: None,
    });
}

fn check_stmt_self_recursion(
    stmt: &Stmt,
    ctx: &mut MethodCtx,
    tokens: &[Token],
    file_path: &Path,
    diags: &mut Vec<LintDiagnostic>,
) {
    match stmt.inner_stmt() {
        Stmt::Function { name, body, .. } => {
            let prev_full = ctx.enclosing_full.clone();
            let prev_bare = ctx.enclosing_bare.clone();
            // Only `Type.method` definitions create a self-recursion scope;
            // free functions legitimately recurse by bare name.
            if is_method_name(name) {
                ctx.enclosing_full = Some(name.clone());
                ctx.enclosing_bare = Some(method_bare(name).to_string());
            } else {
                ctx.enclosing_full = None;
                ctx.enclosing_bare = None;
            };
            for s in body {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
            ctx.enclosing_full = prev_full;
            ctx.enclosing_bare = prev_bare;
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            check_expr_self_recursion(condition, ctx, tokens, file_path, diags);
            for s in then_block {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
                }
            }
        }
        Stmt::While { condition, body } => {
            check_expr_self_recursion(condition, ctx, tokens, file_path, diags);
            for s in body {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            for s in body {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
            for s in catch_block {
                check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    check_stmt_self_recursion(s, ctx, tokens, file_path, diags);
                }
            }
        }
        Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
            check_expr_self_recursion(value, ctx, tokens, file_path, diags);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            check_expr_self_recursion(array, ctx, tokens, file_path, diags);
            check_expr_self_recursion(index, ctx, tokens, file_path, diags);
            check_expr_self_recursion(value, ctx, tokens, file_path, diags);
        }
        Stmt::FieldAssign { object, value, .. } => {
            check_expr_self_recursion(object, ctx, tokens, file_path, diags);
            check_expr_self_recursion(value, ctx, tokens, file_path, diags);
        }
        Stmt::Say(expr) | Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) | Stmt::Expr(expr) => {
            check_expr_self_recursion(expr, ctx, tokens, file_path, diags);
        }
        Stmt::Defer(inner) => {
            check_stmt_self_recursion(inner, ctx, tokens, file_path, diags);
        }
        _ => {}
    }
}

/// Checks for bare same-name calls inside a method that resolve back into
/// that same method (`method-self-recursion`, warning).
pub fn check_method_self_recursion(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    let mut ctx = MethodCtx::default();
    // Same-file free functions: a same-named one means a bare call
    // delegates (via the compiler guard) instead of recursing forever.
    for stmt in &program.statements {
        if let Stmt::Function { name, .. } = stmt.inner_stmt() {
            if !is_method_name(name) {
                ctx.free_fns.insert(method_bare(name).to_string());
            }
        }
    }
    for stmt in &program.statements {
        check_stmt_self_recursion(stmt, &mut ctx, tokens, file_path, &mut diags);
    }
    diags
}

struct MapFrame {
    /// (normalized key text, token index) of string/number keys at depth 0.
    keys: Vec<(String, usize)>,
}

fn is_key_token(tt: &TokenType) -> Option<String> {
    match tt {
        TokenType::String(s) => Some(format!("s:{s}")),
        TokenType::Number(n) => Some(format!("n:{n}")),
        _ => None,
    }
}

/// Duplicate literal keys in a `{...}` literal (`duplicate-map-key`,
/// warning). The last value wins at runtime, so earlier entries are dead
/// weight and almost always a copy-paste bug. The earlier entry carries an
/// auto-fix deleting it through its trailing comma.
pub fn check_duplicate_map_keys(tokens: &[Token], file_path: &Path) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();
    // Stack of open `{` frames plus a generic nesting depth for
    // parens/brackets/inner braces (keys only count at depth 0).
    let mut frames: Vec<MapFrame> = Vec::new();
    let mut in_brace = false;
    let mut inner_depth = 0usize;
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i].token_type {
            TokenType::LeftBrace => {
                frames.push(MapFrame { keys: Vec::new() });
                in_brace = true;
                inner_depth = 0;
                i += 1;
            }
            TokenType::RightBrace => {
                if in_brace && inner_depth == 0 {
                    frames.pop();
                    if frames.is_empty() {
                        in_brace = false;
                    }
                } else {
                    inner_depth = inner_depth.saturating_sub(1);
                }
                i += 1;
            }
            TokenType::LeftParen | TokenType::LeftBracket => {
                if in_brace {
                    inner_depth += 1;
                }
                i += 1;
            }
            TokenType::RightParen | TokenType::RightBracket => {
                if in_brace && inner_depth > 0 {
                    inner_depth -= 1;
                }
                i += 1;
            }
            TokenType::String(_) | TokenType::Number(_) => {
                let key = match is_key_token(&tokens[i].token_type) {
                    Some(k) => k,
                    None => {
                        i += 1;
                        continue;
                    }
                };
                // Key candidates only at depth 0 of the innermost `{`,
                // immediately followed by `:`.
                let is_key = in_brace
                    && inner_depth == 0
                    && matches!(
                        tokens.get(i + 1).map(|t| &t.token_type),
                        Some(TokenType::Colon)
                    );
                if !is_key {
                    i += 1;
                    continue;
                }
                // Ternary guard: a `"a" :` pair preceded by `?` (rather
                // than `{` or `,`) is a conditional arm, not a key.
                let mut k = i;
                let mut ternary = false;
                let mut is_map_key = false;
                loop {
                    if k == 0 {
                        break;
                    }
                    k -= 1;
                    match &tokens[k].token_type {
                        TokenType::LeftBrace | TokenType::Comma => {
                            is_map_key = true;
                            break;
                        }
                        TokenType::Question => {
                            ternary = true;
                            break;
                        }
                        TokenType::Colon | TokenType::RightBrace | TokenType::FatArrow => {
                            break;
                        }
                        _ => {}
                    }
                }
                if ternary || !is_map_key {
                    i += 1;
                    continue;
                }
                if let Some(frame) = frames.last_mut() {
                    if let Some((_, first_idx)) =
                        frame.keys.iter().find(|(k, _)| *k == key).cloned()
                    {
                        let first_tok = &tokens[first_idx];
                        // Fix: delete the earlier entry through its trailing
                        // comma at the same depth. The earlier key always
                        // has a follower (the duplicate itself).
                        let mut fix = None;
                        let mut j = first_idx;
                        let mut d = 0usize;
                        while let Some(tok) = tokens.get(j) {
                            match &tok.token_type {
                                TokenType::LeftParen
                                | TokenType::LeftBracket
                                | TokenType::LeftBrace => d += 1,
                                TokenType::RightParen | TokenType::RightBracket => {
                                    d = d.saturating_sub(1);
                                }
                                TokenType::RightBrace if d == 0 => break,
                                TokenType::Comma if d == 0 => {
                                    fix = Some(crate::tools::lint::types::LintFix {
                                        description: "remove the shadowed earlier entry"
                                            .to_string(),
                                        replacement: String::new(),
                                        start_line: first_tok.line,
                                        start_col: first_tok.column,
                                        end_line: tok.line,
                                        end_col: tok.column + 1,
                                    });
                                    break;
                                }
                                _ => {}
                            }
                            j += 1;
                            // Give up on pathological spans.
                            if j > first_idx + 200 {
                                break;
                            }
                        }
                        diags.push(LintDiagnostic {
                            rule: "duplicate-map-key".to_string(),
                            severity: LintSeverity::Warning,
                            message: format!(
                                "duplicate key {} in map literal; the last value wins, earlier entries are dead",
                                key.split_once(':').map(|(_, v)| v).unwrap_or(&key)
                            ),
                            file_path: file_path.to_path_buf(),
                            line: first_tok.line,
                            col: first_tok.column,
                            end_line: first_tok.line,
                            end_col: first_tok.column + 1,
                            help: Some(
                                "remove the earlier entry or rename one of the keys".to_string(),
                            ),
                            fix,
                        });
                    }
                    frame.keys.push((key, i));
                }
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    diags
}
