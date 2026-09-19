use std::collections::HashSet;
use std::path::Path;

use crate::ast::{Expr, Program, Stmt};
use crate::lexer::{Lexer, Token, TokenType};
use crate::tools::lint::types::{LintDiagnostic, LintFix, LintSeverity};

/// Collects all read identifiers from an expression.
fn collect_expr_identifiers(expr: &Expr, idents: &mut HashSet<String>) {
    match expr {
        Expr::Identifier(name) => {
            idents.insert(name.clone());
        }
        Expr::Binary { left, right, .. } => {
            collect_expr_identifiers(left, idents);
            collect_expr_identifiers(right, idents);
        }
        Expr::Unary { expr, .. } => {
            collect_expr_identifiers(expr, idents);
        }
        Expr::Call { name, args } => {
            if let Some((ns, _)) = name.split_once("::") {
                idents.insert(ns.to_string());
            } else {
                idents.insert(name.clone());
            }
            for a in args {
                collect_expr_identifiers(a, idents);
            }
        }
        Expr::OptionalCall { callee, args } => {
            if let Some((ns, _)) = callee.split_once("::") {
                idents.insert(ns.to_string());
            } else {
                idents.insert(callee.clone());
            }
            for a in args {
                collect_expr_identifiers(a, idents);
            }
        }
        Expr::InterpolatedString(parts) => {
            for p in parts {
                collect_expr_identifiers(p, idents);
            }
        }
        Expr::Array(items) => {
            for item in items {
                collect_expr_identifiers(item, idents);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_expr_identifiers(array, idents);
            collect_expr_identifiers(index, idents);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_expr_identifiers(object, idents);
        }
        Expr::StructInit { name, fields } => {
            idents.insert(name.clone());
            for (_, val) in fields {
                collect_expr_identifiers(val, idents);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                collect_expr_identifiers(k, idents);
                collect_expr_identifiers(v, idents);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_identifiers(condition, idents);
            collect_expr_identifiers(then_branch, idents);
            collect_expr_identifiers(else_branch, idents);
        }
        Expr::NullCoalesce { value, default } => {
            collect_expr_identifiers(value, idents);
            collect_expr_identifiers(default, idents);
        }
        Expr::TypeCheck { expr, target, .. } => {
            collect_expr_identifiers(expr, idents);
            idents.insert(target.clone());
        }
        Expr::Cast { expr, target } => {
            collect_expr_identifiers(expr, idents);
            idents.insert(target.clone());
        }
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null => {}
    }
}

/// Collects all read identifiers from statements.
fn collect_stmt_identifiers(stmts: &[Stmt], idents: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Say(e) | Stmt::Expr(e) | Stmt::Throw(Some(e)) | Stmt::Return(Some(e)) => {
                collect_expr_identifiers(e, idents);
            }
            Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
                collect_expr_identifiers(value, idents);
            }
            Stmt::Assign { value, .. } => {
                collect_expr_identifiers(value, idents);
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                collect_expr_identifiers(array, idents);
                collect_expr_identifiers(index, idents);
                collect_expr_identifiers(value, idents);
            }
            Stmt::FieldAssign { object, value, .. } => {
                collect_expr_identifiers(object, idents);
                collect_expr_identifiers(value, idents);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_expr_identifiers(condition, idents);
                collect_stmt_identifiers(then_block, idents);
                if let Some(eb) = else_block {
                    collect_stmt_identifiers(eb, idents);
                }
            }
            Stmt::While { condition, body } => {
                collect_expr_identifiers(condition, idents);
                collect_stmt_identifiers(body, idents);
            }
            Stmt::Repeat { body } => {
                collect_stmt_identifiers(body, idents);
            }
            Stmt::For {
                start, end, body, ..
            } => {
                collect_expr_identifiers(start, idents);
                collect_expr_identifiers(end, idents);
                collect_stmt_identifiers(body, idents);
            }
            Stmt::ForEach { iterable, body, .. } => {
                collect_expr_identifiers(iterable, idents);
                collect_stmt_identifiers(body, idents);
            }
            Stmt::Function { defaults, body, .. } => {
                for d in defaults.iter().flatten() {
                    collect_expr_identifiers(d, idents);
                }
                collect_stmt_identifiers(body, idents);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_stmt_identifiers(try_block, idents);
                collect_stmt_identifiers(catch_block, idents);
                if let Some(fb) = finally_block {
                    collect_stmt_identifiers(fb, idents);
                }
            }
            Stmt::Defer(s) | Stmt::Pub(s) => {
                collect_stmt_identifiers(&[s.as_ref().clone()], idents);
            }
            _ => {}
        }
    }
}

/// Finds the token position of a variable declared with `let` or `const`.
fn find_var_declaration_token(tokens: &[Token], var_name: &str) -> Option<Token> {
    for i in 0..tokens.len() {
        if matches!(tokens[i].token_type, TokenType::Let | TokenType::Const) {
            if let Some(Token {
                token_type: TokenType::Identifier(ref name),
                ..
            }) = tokens.get(i + 1)
            {
                if name == var_name {
                    return tokens.get(i + 1).cloned();
                }
            }
        }
    }
    None
}

/// Finds the token position of a parameter in a function declaration.
fn find_param_token(tokens: &[Token], fn_name: &str, param_name: &str) -> Option<Token> {
    let mut in_fn = false;
    for i in 0..tokens.len() {
        if matches!(tokens[i].token_type, TokenType::Function) {
            if let Some(Token {
                token_type: TokenType::Identifier(ref name),
                ..
            }) = tokens.get(i + 1)
            {
                if name == fn_name {
                    in_fn = true;
                    continue;
                }
            }
        }
        if in_fn {
            if matches!(tokens[i].token_type, TokenType::Newline) {
                in_fn = false;
                continue;
            }
            if let TokenType::Identifier(ref name) = tokens[i].token_type {
                if name == param_name {
                    return Some(tokens[i].clone());
                }
            }
        }
    }
    None
}

/// Checks for unused local and module variables (`unused-var`).
pub fn check_unused_variables(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();

    // Check variables inside functions
    for stmt in &program.statements {
        let actual_stmt = stmt.inner_stmt();
        if let Stmt::Function { body, .. } = actual_stmt {
            let mut used_idents = HashSet::new();
            collect_stmt_identifiers(body, &mut used_idents);

            for s in body {
                if let Stmt::Let { name, .. } | Stmt::Const { name, .. } = s {
                    if !name.starts_with('_') && !used_idents.contains(name) {
                        if let Some(tok) = find_var_declaration_token(tokens, name) {
                            let len = name.len();
                            diags.push(LintDiagnostic {
                                rule: "unused-var".to_string(),
                                severity: LintSeverity::Warning,
                                message: format!("variable '{}' is declared but never read", name),
                                file_path: file_path.to_path_buf(),
                                line: tok.line,
                                col: tok.column,
                                end_line: tok.line,
                                end_col: tok.column + len,
                                help: Some(format!(
                                    "if this is intentional, prefix with an underscore: '_{}'",
                                    name
                                )),
                                fix: Some(LintFix {
                                    description: format!("Prefix with underscore: '_{}'", name),
                                    replacement: format!("_{}", name),
                                    start_line: tok.line,
                                    start_col: tok.column,
                                    end_line: tok.line,
                                    end_col: tok.column + len,
                                }),
                            });
                        }
                    }
                }
            }
        }
    }

    // Check top-level non-pub variables
    let mut top_used_idents = HashSet::new();
    collect_stmt_identifiers(&program.statements, &mut top_used_idents);

    for stmt in &program.statements {
        if stmt.is_pub() {
            continue;
        }
        if let Stmt::Let { name, .. } | Stmt::Const { name, .. } = stmt {
            if !name.starts_with('_') && !top_used_idents.contains(name) {
                if let Some(tok) = find_var_declaration_token(tokens, name) {
                    let len = name.len();
                    diags.push(LintDiagnostic {
                        rule: "unused-var".to_string(),
                        severity: LintSeverity::Warning,
                        message: format!("variable '{}' is declared but never read", name),
                        file_path: file_path.to_path_buf(),
                        line: tok.line,
                        col: tok.column,
                        end_line: tok.line,
                        end_col: tok.column + len,
                        help: Some(format!(
                            "if this is intentional, prefix with an underscore: '_{}'",
                            name
                        )),
                        fix: Some(LintFix {
                            description: format!("Prefix with underscore: '_{}'", name),
                            replacement: format!("_{}", name),
                            start_line: tok.line,
                            start_col: tok.column,
                            end_line: tok.line,
                            end_col: tok.column + len,
                        }),
                    });
                }
            }
        }
    }

    diags
}

/// Checks for unused function parameters (`unused-param`).
pub fn check_unused_parameters(
    program: &Program,
    tokens: &[Token],
    file_path: &Path,
) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();

    for stmt in &program.statements {
        let actual_stmt = stmt.inner_stmt();
        if let Stmt::Function {
            name: fn_name,
            params,
            body,
            ..
        } = actual_stmt
        {
            let mut used_idents = HashSet::new();
            collect_stmt_identifiers(body, &mut used_idents);

            for param in params {
                if param == "self" || param.starts_with('_') {
                    continue;
                }
                if !used_idents.contains(param) {
                    if let Some(tok) = find_param_token(tokens, fn_name, param) {
                        let len = param.len();
                        diags.push(LintDiagnostic {
                            rule: "unused-param".to_string(),
                            severity: LintSeverity::Warning,
                            message: format!(
                                "parameter '{}' is defined in function '{}' but never used",
                                param, fn_name
                            ),
                            file_path: file_path.to_path_buf(),
                            line: tok.line,
                            col: tok.column,
                            end_line: tok.line,
                            end_col: tok.column + len,
                            help: Some(format!(
                                "if this is intentional, prefix with an underscore: '_{}'",
                                param
                            )),
                            fix: Some(LintFix {
                                description: format!("Prefix with underscore: '_{}'", param),
                                replacement: format!("_{}", param),
                                start_line: tok.line,
                                start_col: tok.column,
                                end_line: tok.line,
                                end_col: tok.column + len,
                            }),
                        });
                    }
                }
            }
        }
    }

    diags
}

/// Extracts all `pub` symbol names declared in source code.
fn extract_pub_symbols(src: &str) -> HashSet<String> {
    let mut symbols = HashSet::new();
    let mut lexer = Lexer::new(src);
    if let Ok(tokens) = lexer.tokenize() {
        let mut idx = 0;
        while idx < tokens.len() {
            if matches!(tokens[idx].token_type, TokenType::Pub) {
                idx += 1;
                if idx < tokens.len()
                    && matches!(
                        tokens[idx].token_type,
                        TokenType::Function
                            | TokenType::Struct
                            | TokenType::Enum
                            | TokenType::Interface
                            | TokenType::Const
                            | TokenType::Let
                    )
                {
                    idx += 1;
                }
                if idx < tokens.len() {
                    if let TokenType::Identifier(name) = &tokens[idx].token_type {
                        symbols.insert(name.clone());
                    }
                }
            }
            idx += 1;
        }
    }
    symbols
}

/// Discovers exported symbols for a given imported module path.
fn get_exported_symbols_from_module(
    module_path: &str,
    current_file: &Path,
) -> Option<HashSet<String>> {
    // 1. Embedded standard library check
    if module_path.starts_with("std/") || module_path.starts_with("std::") {
        let clean = module_path
            .strip_prefix("std/")
            .or_else(|| module_path.strip_prefix("std::"))
            .unwrap_or(module_path);
        let clean = clean.strip_suffix(".alya").unwrap_or(clean);
        if let Some(src) = crate::parser::get_embedded_stdlib(clean) {
            return Some(extract_pub_symbols(src));
        }
    }

    // 2. Relative file on disk
    if let Some(parent) = current_file.parent() {
        let p = parent.join(module_path);
        if p.exists() {
            if let Ok(src) = std::fs::read_to_string(&p) {
                return Some(extract_pub_symbols(&src));
            }
        }
        let p_alya = p.with_extension("alya");
        if p_alya.exists() {
            if let Ok(src) = std::fs::read_to_string(&p_alya) {
                return Some(extract_pub_symbols(&src));
            }
        }
    }

    // 3. Fallback: direct standard library module name (e.g. "hash")
    if let Some(src) = crate::parser::get_embedded_stdlib(module_path) {
        return Some(extract_pub_symbols(src));
    }

    None
}

/// Checks for unused import statements or symbols.
pub fn check_unused_imports(tokens: &[Token], file_path: &Path) -> Vec<LintDiagnostic> {
    let mut diags = Vec::new();

    // 1. Identify all import declarations and their tokens
    struct ImportInfo {
        name_to_check: String,
        raw_path: Option<String>,
        has_alias: bool,
        line: usize,
        col: usize,
        is_full_line: bool,
    }

    let mut imports = Vec::new();
    let mut import_token_ranges = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i].token_type, TokenType::Import) {
            let start_idx = i;
            let import_line = tokens[i].line;
            let import_col = tokens[i].column;
            i += 1;

            let mut imported_name = None;
            let mut raw_path = None;
            if i < tokens.len() {
                match &tokens[i].token_type {
                    TokenType::String(path) => {
                        raw_path = Some(path.clone());
                        let base = path
                            .split('/')
                            .last()
                            .unwrap_or(path)
                            .trim_end_matches(".alya");
                        imported_name = Some(base.to_string());
                        i += 1;
                    }
                    TokenType::Identifier(id) => {
                        raw_path = Some(id.clone());
                        imported_name = Some(id.clone());
                        i += 1;
                    }
                    _ => {}
                }
            }

            let mut has_alias = false;
            if i < tokens.len() && matches!(tokens[i].token_type, TokenType::As) {
                i += 1;
                if i < tokens.len() {
                    if let TokenType::Identifier(alias) = &tokens[i].token_type {
                        imported_name = Some(alias.clone());
                        has_alias = true;
                        i += 1;
                    }
                }
            }

            if let Some(name) = imported_name {
                imports.push(ImportInfo {
                    name_to_check: name,
                    raw_path,
                    has_alias,
                    line: import_line,
                    col: import_col,
                    is_full_line: true,
                });
                import_token_ranges.push((start_idx, i));
            }
            continue;
        }

        if matches!(tokens[i].token_type, TokenType::From) {
            let start_idx = i;
            i += 1;

            let mut raw_path = None;
            // Skip module path
            if i < tokens.len() {
                match &tokens[i].token_type {
                    TokenType::String(p) | TokenType::Identifier(p) => {
                        raw_path = Some(p.clone());
                        i += 1;
                    }
                    _ => {}
                }
            }

            if i < tokens.len() && matches!(tokens[i].token_type, TokenType::Import) {
                i += 1;
                while i < tokens.len()
                    && !matches!(tokens[i].token_type, TokenType::Newline | TokenType::Eof)
                {
                    if let TokenType::Identifier(sym) = &tokens[i].token_type {
                        let sym_line = tokens[i].line;
                        let sym_col = tokens[i].column;
                        let mut name = sym.clone();
                        let mut has_alias = false;
                        i += 1;
                        if i < tokens.len() && matches!(tokens[i].token_type, TokenType::As) {
                            i += 1;
                            if i < tokens.len() {
                                if let TokenType::Identifier(alias) = &tokens[i].token_type {
                                    name = alias.clone();
                                    has_alias = true;
                                    i += 1;
                                }
                            }
                        }
                        imports.push(ImportInfo {
                            name_to_check: name,
                            raw_path: raw_path.clone(),
                            has_alias,
                            line: sym_line,
                            col: sym_col,
                            is_full_line: false,
                        });
                    } else {
                        i += 1;
                    }
                }
                import_token_ranges.push((start_idx, i));
            }
            continue;
        }

        i += 1;
    }

    if imports.is_empty() {
        return diags;
    }

    // 2. Gather all identifier tokens outside of import lines
    let mut code_idents = HashSet::new();
    for (idx, tok) in tokens.iter().enumerate() {
        let inside_import = import_token_ranges
            .iter()
            .any(|&(start, end)| idx >= start && idx < end);
        if !inside_import {
            match &tok.token_type {
                TokenType::Identifier(id) => {
                    if let Some((ns, _)) = id.split_once("::") {
                        code_idents.insert(ns.to_string());
                    }
                    code_idents.insert(id.clone());
                }
                TokenType::Test => {
                    code_idents.insert("test".to_string());
                }
                TokenType::Assert => {
                    code_idents.insert("assert".to_string());
                }
                TokenType::Bench => {
                    code_idents.insert("bench".to_string());
                }
                _ => {}
            }
        }
    }

    for imp in imports {
        let mut is_used = code_idents.contains(&imp.name_to_check)
            || code_idents.iter().any(|id| {
                id.starts_with(&format!("{}_", imp.name_to_check))
                    || id.starts_with(&format!("{}::", imp.name_to_check))
                    || id.starts_with(&format!("{}.", imp.name_to_check))
            });

        // If it's a full-line import without an alias (e.g. `import "std/hash"`),
        // check if any of the symbols exported by the module are used directly.
        if !is_used && !imp.has_alias && imp.is_full_line {
            if let Some(path) = &imp.raw_path {
                if let Some(exported) = get_exported_symbols_from_module(path, file_path) {
                    if exported.iter().any(|sym| code_idents.contains(sym)) {
                        is_used = true;
                    }
                }
            }
        }

        if !is_used {
            let len = imp.name_to_check.len();
            let fix = if imp.is_full_line {
                Some(LintFix {
                    description: format!("Remove unused import '{}'", imp.name_to_check),
                    replacement: String::new(),
                    start_line: imp.line,
                    start_col: 1,
                    end_line: imp.line + 1,
                    end_col: 1,
                })
            } else {
                None
            };

            diags.push(LintDiagnostic {
                rule: "unused-import".to_string(),
                severity: LintSeverity::Warning,
                message: format!("imported module '{}' is never used", imp.name_to_check),
                file_path: file_path.to_path_buf(),
                line: imp.line,
                col: imp.col,
                end_line: imp.line,
                end_col: imp.col + len,
                help: Some("remove unused import".to_string()),
                fix,
            });
        }
    }

    diags
}
