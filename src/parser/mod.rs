pub mod constants;
pub mod enums;
pub mod expr;
pub mod generics;
pub mod stmt;
#[cfg(test)]
mod tests;

use crate::ast::{Expr, Program, Stmt};
use crate::lexer::{Token, TokenType};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    pub(super) lambda_functions: Vec<Stmt>,
    pub(super) lambda_counter: usize,
    pub(super) fn_depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            lambda_functions: Vec::new(),
            lambda_counter: 0,
            fn_depth: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !matches!(self.current_token().token_type, TokenType::Eof) {
            statements.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        statements.append(&mut self.lambda_functions);

        let mut program = Program { statements };
        expand_default_args(&mut program);
        enums::resolve_enums(&mut program);
        constants::resolve_and_validate_constants(&mut program)?;
        Ok(program)
    }

    pub(super) fn current_token(&self) -> &Token {
        &self.tokens[self.position]
    }

    pub(super) fn peek_token(&self) -> Option<&Token> {
        if self.position + 1 < self.tokens.len() {
            Some(&self.tokens[self.position + 1])
        } else {
            None
        }
    }

    pub(super) fn advance(&mut self) {
        if self.position < self.tokens.len() - 1 {
            self.position += 1;
        }
    }

    pub(super) fn expect(&mut self, expected: TokenType) -> Result<(), String> {
        if std::mem::discriminant(&self.current_token().token_type)
            != std::mem::discriminant(&expected)
        {
            return Err(format!(
                "Expected {}, found {} at line {}, column {}",
                expected,
                self.current_token().token_type,
                self.current_token().line,
                self.current_token().column
            ));
        }
        self.advance();
        Ok(())
    }

    pub(super) fn skip_newlines(&mut self) {
        while matches!(self.current_token().token_type, TokenType::Newline) {
            self.advance();
        }
    }

    pub(crate) fn parse_type_annotation(&mut self) -> Result<String, String> {
        let mut t_str = String::new();

        // 1. Prefix modifiers: weak, ...
        if matches!(self.current_token().token_type, TokenType::Weak) {
            self.advance();
            t_str.push_str("weak ");
        }
        if matches!(
            self.current_token().token_type,
            TokenType::DotDotDot | TokenType::DotDot
        ) {
            self.advance();
            t_str.push_str("...");
        }

        // 2. Base type:
        if matches!(self.current_token().token_type, TokenType::LeftBracket) {
            // [KeyType: ValueType] (Map) or [T]
            self.advance();
            self.skip_newlines();
            let key_type = self.parse_type_annotation()?;
            self.skip_newlines();
            if matches!(self.current_token().token_type, TokenType::Colon) {
                self.advance();
                self.skip_newlines();
                let val_type = self.parse_type_annotation()?;
                self.skip_newlines();
                self.expect(TokenType::RightBracket)?;
                t_str.push_str(&format!("[{}: {}]", key_type, val_type));
            } else {
                self.expect(TokenType::RightBracket)?;
                t_str.push_str(&format!("[{}]", key_type));
            }
        } else if matches!(self.current_token().token_type, TokenType::LeftParen) {
            // Tuple: (T1, T2)
            self.advance();
            self.skip_newlines();
            let mut parts = Vec::new();
            while !matches!(
                self.current_token().token_type,
                TokenType::RightParen | TokenType::Eof
            ) {
                parts.push(self.parse_type_annotation()?);
                self.skip_newlines();
                if matches!(self.current_token().token_type, TokenType::Comma) {
                    self.advance();
                    self.skip_newlines();
                } else {
                    break;
                }
            }
            self.expect(TokenType::RightParen)?;
            t_str.push_str(&format!("({})", parts.join(", ")));
        } else if matches!(self.current_token().token_type, TokenType::BitOr | TokenType::Or) {
            let is_empty = matches!(self.current_token().token_type, TokenType::Or);
            self.advance();
            let mut param_types = Vec::new();
            if !is_empty {
                while !matches!(self.current_token().token_type, TokenType::BitOr | TokenType::Eof) {
                    param_types.push(self.parse_type_annotation()?);
                    self.skip_newlines();
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                        self.skip_newlines();
                    } else {
                        break;
                    }
                }
                self.expect(TokenType::BitOr)?;
            }
            let ret = if matches!(self.current_token().token_type, TokenType::Arrow) {
                self.advance();
                self.skip_newlines();
                format!(" -> {}", self.parse_type_annotation()?)
            } else {
                String::new()
            };
            t_str.push_str(&format!("|{}|{}", param_types.join(", "), ret));
        } else if let TokenType::Identifier(s) = &self.current_token().token_type {
            let mut name = s.clone();
            self.advance();

            // Module or enum namespace: A::B or A.B
            while matches!(
                self.current_token().token_type,
                TokenType::ColonColon | TokenType::Dot
            ) {
                self.advance();
                if let TokenType::Identifier(member) = &self.current_token().token_type {
                    name.push_str("::");
                    name.push_str(member);
                    self.advance();
                } else {
                    break;
                }
            }

            // Generic type arguments: Channel[int], Stack[T], Result[T, E]
            if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                if self.position + 1 < self.tokens.len()
                    && matches!(self.tokens[self.position + 1].token_type, TokenType::RightBracket)
                {
                    // array suffix, leave for loop
                } else {
                    self.advance(); // consume '['
                    self.skip_newlines();
                    let mut gen_args = Vec::new();
                    while !matches!(
                        self.current_token().token_type,
                        TokenType::RightBracket | TokenType::Eof
                    ) {
                        gen_args.push(self.parse_type_annotation()?);
                        self.skip_newlines();
                        if matches!(self.current_token().token_type, TokenType::Comma) {
                            self.advance();
                            self.skip_newlines();
                        } else {
                            break;
                        }
                    }
                    self.expect(TokenType::RightBracket)?;
                    name.push_str(&format!("[{}]", gen_args.join(", ")));
                }
            }

            t_str.push_str(&name);
        } else if matches!(self.current_token().token_type, TokenType::SelfKw) {
            self.advance();
            t_str.push_str("Self");
        } else if matches!(self.current_token().token_type, TokenType::Function) {
            // Function type: fn(T) -> R
            self.advance();
            let mut fn_sig = "fn".to_string();
            if matches!(self.current_token().token_type, TokenType::LeftParen) {
                self.advance();
                let mut params = Vec::new();
                while !matches!(
                    self.current_token().token_type,
                    TokenType::RightParen | TokenType::Eof
                ) {
                    params.push(self.parse_type_annotation()?);
                    if matches!(self.current_token().token_type, TokenType::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenType::RightParen)?;
                fn_sig.push_str(&format!("({})", params.join(", ")));
            }
            if matches!(self.current_token().token_type, TokenType::Arrow) {
                self.advance();
                let ret = self.parse_type_annotation()?;
                fn_sig.push_str(&format!(" -> {}", ret));
            }
            t_str.push_str(&fn_sig);
        } else {
            return Err(format!(
                "Expected type at line {}, column {}",
                self.current_token().line,
                self.current_token().column
            ));
        }

        // 3. Suffix modifiers: ?, []
        loop {
            if matches!(self.current_token().token_type, TokenType::Question) {
                self.advance();
                t_str.push('?');
            } else if matches!(self.current_token().token_type, TokenType::LeftBracket) {
                if self.position + 1 < self.tokens.len()
                    && matches!(self.tokens[self.position + 1].token_type, TokenType::RightBracket)
                {
                    self.advance(); // [
                    self.advance(); // ]
                    t_str.push_str("[]");
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(t_str)
    }
}

pub fn resolve_imports_with_sources(
    program: &mut Program,
    base_dir: &std::path::Path,
) -> Result<std::collections::HashSet<std::path::PathBuf>, String> {
    resolve_imports_with_sources_ext(program, base_dir, false)
}

pub fn resolve_imports_with_sources_ext(
    program: &mut Program,
    base_dir: &std::path::Path,
    no_std: bool,
) -> Result<std::collections::HashSet<std::path::PathBuf>, String> {
    let mut visited = std::collections::HashSet::new();
    let mut resolved_stmts = Vec::new();

    for stmt in std::mem::take(&mut program.statements) {
        resolve_stmt_imports_ext(stmt, base_dir, &mut visited, &mut resolved_stmts, no_std)?;
    }

    // Deduplicate private module functions (__priv_*) that were imported via multiple paths
    let mut seen_privates = std::collections::HashSet::new();
    resolved_stmts.retain(|stmt| {
        if let Stmt::Function { name, .. } = stmt.inner_stmt() {
            if name.starts_with("__priv_") {
                return seen_privates.insert(name.clone());
            }
        }
        true
    });

    validate_unique_functions(&resolved_stmts)?;
    program.statements = resolved_stmts;

    let mut module_stems = std::collections::HashSet::new();
    for std_mod in ["math", "time", "fs", "os", "path", "net", "sync", "color", "env", "process", "io", "crypto", "json", "random"] {
        module_stems.insert(std_mod.to_string());
    }
    for (path, alias) in &visited {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            module_stems.insert(stem.to_string());
        }
        if let Some(ref a) = alias {
            module_stems.insert(a.clone());
        }
    }
    expand_default_args_with_modules(program, &module_stems);
    enums::resolve_enums(program);
    constants::resolve_and_validate_constants(program)?;

    let imported_files = visited.into_iter().map(|(path, _)| path).collect();
    Ok(imported_files)
}

pub fn resolve_imports(program: &mut Program, base_dir: &std::path::Path) -> Result<(), String> {
    resolve_imports_with_sources(program, base_dir).map(|_| ())
}

pub fn validate_unique_functions(stmts: &[Stmt]) -> Result<(), String> {
    let mut seen_functions: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for stmt in stmts {
        if let Stmt::Function { name, .. } = stmt {
            let count = seen_functions.entry(name.clone()).or_insert(0);
            *count += 1;
            if *count > 1 {
                return Err(format!(
                    "Duplicate function definition '{}'. If importing multiple modules containing '{}', use 'import \"...\" as <alias>' to assign distinct namespaces.",
                    name, name
                ));
            }
        }
    }
    Ok(())
}

fn collect_local_vars(stmts: &[Stmt], vars: &mut std::collections::HashSet<String>) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                vars.insert(name.clone());
            }
            Stmt::For { var, body, .. } => {
                vars.insert(var.clone());
                collect_local_vars(body, vars);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                vars.insert(var.clone());
                if let Some(ref v) = value_var {
                    vars.insert(v.clone());
                }
                collect_local_vars(body, vars);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_local_vars(then_block, vars);
                if let Some(ref eb) = else_block {
                    collect_local_vars(eb, vars);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                collect_local_vars(body, vars);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                catch_var,
                finally_block,
                ..
            } => {
                collect_local_vars(try_block, vars);
                if let Some(ref cv) = catch_var {
                    vars.insert(cv.clone());
                }
                collect_local_vars(catch_block, vars);
                if let Some(ref fb) = finally_block {
                    collect_local_vars(fb, vars);
                }
            }
            _ => {}
        }
    }
}

fn apply_module_alias(
    stmts: &mut [Stmt],
    alias: &str,
    local_fns: &std::collections::HashSet<String>,
) {
    for stmt in stmts {
        prefix_stmt(stmt, alias, local_fns);
    }
}

fn prefix_stmt(stmt: &mut Stmt, alias: &str, local_fns: &std::collections::HashSet<String>) {
    match stmt {
        Stmt::Function {
            name,
            params,
            defaults,
            body,
            ..
        } => {
            if local_fns.contains(name) {
                *name = format!("{}::{}", alias, name);
            }
            for def in defaults.iter_mut().flatten() {
                prefix_expr(def, alias, local_fns);
            }
            let mut inner_fns = local_fns.clone();
            for p in params {
                inner_fns.remove(p);
            }
            let mut local_vars = std::collections::HashSet::new();
            collect_local_vars(body, &mut local_vars);
            for v in &local_vars {
                inner_fns.remove(v);
            }
            for s in body {
                prefix_stmt(s, alias, &inner_fns);
            }
        }
        Stmt::Say(expr) => prefix_expr(expr, alias, local_fns),
        Stmt::Expr(expr) => prefix_expr(expr, alias, local_fns),
        Stmt::Let { value, .. } => prefix_expr(value, alias, local_fns),
        Stmt::Assign { value, .. } => prefix_expr(value, alias, local_fns),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            prefix_expr(condition, alias, local_fns);
            for s in then_block {
                prefix_stmt(s, alias, local_fns);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    prefix_stmt(s, alias, local_fns);
                }
            }
        }
        Stmt::While { condition, body } => {
            prefix_expr(condition, alias, local_fns);
            for s in body {
                prefix_stmt(s, alias, local_fns);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            prefix_expr(start, alias, local_fns);
            prefix_expr(end, alias, local_fns);
            for s in body {
                prefix_stmt(s, alias, local_fns);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            prefix_expr(iterable, alias, local_fns);
            for s in body {
                prefix_stmt(s, alias, local_fns);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                prefix_stmt(s, alias, local_fns);
            }
        }
        Stmt::Return(Some(e)) => {
            prefix_expr(e, alias, local_fns);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            prefix_expr(array, alias, local_fns);
            prefix_expr(index, alias, local_fns);
            prefix_expr(value, alias, local_fns);
        }
        Stmt::FieldAssign { object, value, .. } => {
            prefix_expr(object, alias, local_fns);
            prefix_expr(value, alias, local_fns);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                prefix_stmt(s, alias, local_fns);
            }
            for s in catch_block {
                prefix_stmt(s, alias, local_fns);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    prefix_stmt(s, alias, local_fns);
                }
            }
        }
        Stmt::Throw(Some(e)) => {
            prefix_expr(e, alias, local_fns);
        }
        Stmt::Const { name, value } => {
            prefix_expr(value, alias, local_fns);
            *name = format!("{}::{}", alias, name);
        }
        Stmt::EnumDef { name, .. } => {
            *name = format!("{}::{}", alias, name);
        }
        Stmt::Pub(inner) => {
            prefix_stmt(inner, alias, local_fns);
        }
        _ => {}
    }
}

fn prefix_expr(expr: &mut Expr, alias: &str, local_fns: &std::collections::HashSet<String>) {
    match expr {
        Expr::Call { name, args } => {
            if local_fns.contains(name) {
                *name = format!("{}::{}", alias, name);
            }
            for arg in args {
                prefix_expr(arg, alias, local_fns);
            }
        }
        Expr::Binary { left, right, .. } => {
            prefix_expr(left, alias, local_fns);
            prefix_expr(right, alias, local_fns);
        }
        Expr::Unary { expr, .. } => {
            prefix_expr(expr, alias, local_fns);
        }
        Expr::Array(items) => {
            for item in items {
                prefix_expr(item, alias, local_fns);
            }
        }
        Expr::Index { array, index } => {
            prefix_expr(array, alias, local_fns);
            prefix_expr(index, alias, local_fns);
        }
        Expr::FieldAccess { object, .. } => {
            prefix_expr(object, alias, local_fns);
        }
        Expr::StructInit { fields, .. } => {
            for (_, field_expr) in fields {
                prefix_expr(field_expr, alias, local_fns);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                prefix_expr(k, alias, local_fns);
                prefix_expr(v, alias, local_fns);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                prefix_expr(part, alias, local_fns);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            prefix_expr(condition, alias, local_fns);
            prefix_expr(then_branch, alias, local_fns);
            prefix_expr(else_branch, alias, local_fns);
        }
        Expr::NullCoalesce { value, default } => {
            prefix_expr(value, alias, local_fns);
            prefix_expr(default, alias, local_fns);
        }
        Expr::OptionalCall { callee, args } => {
            if local_fns.contains(callee) {
                *callee = format!("{}::{}", alias, callee);
            }
            for arg in args {
                prefix_expr(arg, alias, local_fns);
            }
        }
        Expr::TypeCheck { expr, .. } | Expr::Cast { expr, .. } => {
            prefix_expr(expr, alias, local_fns);
        }
        Expr::Identifier(name) if local_fns.contains(name) => {
            *name = format!("{}::{}", alias, name);
        }
        _ => {}
    }
}

pub(crate) fn canonical_stdlib_module(clean: &str) -> &str {
    match clean {
        "rand" => "math",
        "color" | "term" | "ansi" => "console",
        "glob" => "path",
        "thread" | "threads" | "concurrency" => "sync",
        "bench" => "test",
        other => other,
    }
}

fn get_embedded_stdlib(module: &str) -> Option<&'static str> {
    let clean = module
        .strip_prefix("std/")
        .or_else(|| module.strip_prefix("std::"))
        .unwrap_or(module);
    let clean = clean.strip_suffix(".alya").unwrap_or(clean);
    let canonical = canonical_stdlib_module(clean);
    match canonical {
        "math" => Some(include_str!("../../stdlib/math.alya")),
        "time" => Some(include_str!("../../stdlib/time.alya")),
        "os" => Some(include_str!("../../stdlib/os.alya")),
        "process" => Some(include_str!("../../stdlib/process.alya")),
        "io" => Some(include_str!("../../stdlib/io.alya")),
        "json" => Some(include_str!("../../stdlib/json.alya")),
        "mem" => Some(include_str!("../../stdlib/mem.alya")),
        "str" => Some(include_str!("../../stdlib/str.alya")),
        "path" => Some(include_str!("../../stdlib/path.alya")),
        "fs" => Some(include_str!("../../stdlib/fs.alya")),
        "hash" => Some(include_str!("../../stdlib/hash.alya")),
        "collections" => Some(include_str!("../../stdlib/collections.alya")),
        "test" => Some(include_str!("../../stdlib/test.alya")),
        "cli" | "argparse" => Some(include_str!("../../stdlib/cli.alya")),
        "console" => Some(include_str!("../../stdlib/console.alya")),
        "log" | "logger" => Some(include_str!("../../stdlib/log.alya")),
        "net" | "http" => Some(include_str!("../../stdlib/net.alya")),
        "sync" | "synchronization" => Some(include_str!("../../stdlib/sync.alya")),
        _ => None,
    }
}

#[allow(dead_code)]
pub(crate) fn resolve_stmt_imports(
    stmt: Stmt,
    current_dir: &std::path::Path,
    visited: &mut std::collections::HashSet<(std::path::PathBuf, Option<String>)>,
    out: &mut Vec<Stmt>,
) -> Result<std::collections::HashSet<String>, String> {
    resolve_stmt_imports_ext(stmt, current_dir, visited, out, false)
}

pub(crate) fn resolve_stmt_imports_ext(
    stmt: Stmt,
    current_dir: &std::path::Path,
    visited: &mut std::collections::HashSet<(std::path::PathBuf, Option<String>)>,
    out: &mut Vec<Stmt>,
    no_std: bool,
) -> Result<std::collections::HashSet<String>, String> {
    match stmt {
        Stmt::Import {
            path: import_path_str,
            alias,
            symbols,
        } => {
            // Normalize path separators to '/' so Windows-style '\' works across Linux, macOS, and Windows
            let normalized_path = import_path_str.replace('\\', "/");
            if no_std && (normalized_path.starts_with("std/") || normalized_path.starts_with("std::")) {
                return Err(format!(
                    "Cannot import '{}' in --no-std bare-metal mode",
                    import_path_str
                ));
            }
            let path = std::path::Path::new(&normalized_path);
            let target_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                current_dir.join(path)
            };

            let candidate = if target_path.exists() {
                Some(target_path.clone())
            } else if target_path.with_extension("alya").exists() {
                Some(target_path.with_extension("alya"))
            } else if normalized_path.starts_with("std/") || normalized_path.starts_with("std::") {
                let clean = normalized_path
                    .strip_prefix("std/")
                    .or_else(|| normalized_path.strip_prefix("std::"))
                    .unwrap_or(&normalized_path);
                let clean = clean.strip_suffix(".alya").unwrap_or(clean);
                let canonical_name = canonical_stdlib_module(clean);
                let std_dir = current_dir.join("stdlib").join(canonical_name);
                let std_root = std::path::Path::new("stdlib").join(canonical_name);
                if std_dir.exists() {
                    Some(std_dir)
                } else if std_dir.with_extension("alya").exists() {
                    Some(std_dir.with_extension("alya"))
                } else if std_root.exists() {
                    Some(std_root)
                } else if std_root.with_extension("alya").exists() {
                    Some(std_root.with_extension("alya"))
                } else {
                    None
                }
            } else {
                crate::tools::pkg::resolve_package_import(&normalized_path, current_dir)?
            };

            let (canonical, source) = if let Some(cand) = candidate {
                let canon = std::fs::canonicalize(&cand)
                    .map_err(|e| format!("Failed to resolve path '{}': {}", cand.display(), e))?;
                if visited.contains(&(canon.clone(), alias.clone())) {
                    return Ok(std::collections::HashSet::new());
                }
                let src = std::fs::read_to_string(&canon).map_err(|e| {
                    format!(
                        "Failed to read imported module '{}': {}",
                        canon.display(),
                        e
                    )
                })?;
                (canon, src)
            } else if normalized_path.starts_with("std/") || normalized_path.starts_with("std::") {
                let clean = normalized_path
                    .strip_prefix("std/")
                    .or_else(|| normalized_path.strip_prefix("std::"))
                    .unwrap_or(&normalized_path);
                let clean = clean.strip_suffix(".alya").unwrap_or(clean);
                let canonical_name = canonical_stdlib_module(clean);
                if let Some(src) = get_embedded_stdlib(canonical_name) {
                    let synthetic =
                        std::path::PathBuf::from(format!("<embedded:std/{}>", canonical_name));
                    if visited.contains(&(synthetic.clone(), alias.clone())) {
                        return Ok(std::collections::HashSet::new());
                    }
                    (synthetic, src.to_string())
                } else {
                    return Err(format!(
                        "Cannot find standard library module '{}'",
                        import_path_str
                    ));
                }
            } else {
                return Err(format!(
                    "Cannot find imported module '{}' (looked at '{}')",
                    import_path_str,
                    target_path.display()
                ));
            };

            visited.insert((canonical.clone(), alias.clone()));

            let mut lexer = crate::lexer::Lexer::new(&source);
            let tokens = lexer.tokenize().map_err(|e| {
                format!(
                    "Lexer error in imported module '{}': {}",
                    canonical.display(),
                    e
                )
            })?;

            let mut parser = Parser::new(tokens);
            let sub_program = parser.parse().map_err(|e| {
                format!(
                    "Parser error in imported module '{}': {}",
                    canonical.display(),
                    e
                )
            })?;

            let is_embedded_stdlib = canonical.to_string_lossy().starts_with("<embedded:");
            let has_any_pub = sub_program.statements.iter().any(|s| s.is_pub());
            let pub_symbol_names: std::collections::HashSet<String> = if has_any_pub {
                sub_program
                    .statements
                    .iter()
                    .filter(|s| s.is_pub())
                    .filter_map(|s| s.declared_symbol_name().map(|n| n.to_string()))
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

            let private_fns: std::collections::HashSet<String> = if has_any_pub {
                sub_program
                    .statements
                    .iter()
                    .filter(|s| !s.is_pub())
                    .filter_map(|s| match s.inner_stmt() {
                        Stmt::Function { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

            let mut local_fns: std::collections::HashSet<String> =
                if !is_embedded_stdlib || alias.is_some() {
                    sub_program
                        .statements
                        .iter()
                        .filter_map(|s| {
                            if has_any_pub && !s.is_pub() {
                                None
                            } else {
                                match s.inner_stmt() {
                                    Stmt::Function { name, .. } => Some(name.clone()),
                                    _ => None,
                                }
                            }
                        })
                        .collect()
                } else {
                    std::collections::HashSet::new()
                };

            let sub_dir = canonical.parent().unwrap_or(current_dir);
            let mut sub_resolved = Vec::new();
            for sub_stmt in sub_program.statements {
                let is_unaliased_import = matches!(&sub_stmt, Stmt::Import { alias: None, .. });
                let child_fns =
                    resolve_stmt_imports_ext(sub_stmt, sub_dir, visited, &mut sub_resolved, no_std)?;
                if is_unaliased_import && (!is_embedded_stdlib || alias.is_some()) {
                    local_fns.extend(child_fns);
                }
            }

            if let Some(ref syms) = symbols {
                // Selective import: from "..." import a, b [as c]
                // 1. Verify requested symbols exist and check visibility
                for sym in syms {
                    if sym.name == "*" {
                        continue;
                    }
                    let matching_stmt = sub_resolved.iter().find(|s| match s.inner_stmt() {
                        Stmt::Function { name, .. } => name == &sym.name,
                        Stmt::StructDef { name, .. } => name == &sym.name,
                        Stmt::EnumDef { name, .. } => name == &sym.name,
                        Stmt::Const { name, .. } => name == &sym.name,
                        Stmt::Let { name, .. } => name == &sym.name,
                        _ => false,
                    });
                    match matching_stmt {
                        Some(stmt) => {
                            if has_any_pub && !stmt.is_pub() {
                                return Err(format!(
                                    "Cannot import private symbol '{}' from module '{}' (must be declared with 'pub')",
                                    sym.name, import_path_str
                                ));
                            }
                        }
                        None => {
                            return Err(format!(
                                "Module '{}' does not export symbol '{}'",
                                import_path_str, sym.name
                            ));
                        }
                    }
                }
            }

            if !private_fns.is_empty() {
                let mod_stem = canonical
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("mod");
                let clean_stem: String = mod_stem
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                canonical.hash(&mut hasher);
                let priv_alias = format!("__priv_{}_{:x}", clean_stem, hasher.finish());
                apply_module_alias(&mut sub_resolved, &priv_alias, &private_fns);
            }

            if let Some(ref syms) = symbols {
                // 2. For aliased symbols, clone and rename definitions
                let mut additional_stmts = Vec::new();
                for sym in syms {
                    if let Some(ref alias_name) = sym.alias {
                        for s in &sub_resolved {
                            match s.inner_stmt() {
                                Stmt::Function {
                                    name,
                                    params,
                                    param_types,
                                    return_type,
                                    defaults,
                                    body,
                                    type_params,
                                } if name == &sym.name => {
                                    additional_stmts.push(Stmt::Function {
                                        name: alias_name.clone(),
                                        params: params.clone(),
                                        param_types: param_types.clone(),
                                        return_type: return_type.clone(),
                                        defaults: defaults.clone(),
                                        body: body.clone(),
                                        type_params: type_params.clone(),
                                    });
                                }
                                Stmt::StructDef {
                                    name,
                                    fields,
                                    field_types,
                                    defaults,
                                } if name == &sym.name => {
                                    additional_stmts.push(Stmt::StructDef {
                                        name: alias_name.clone(),
                                        fields: fields.clone(),
                                        field_types: field_types.clone(),
                                        defaults: defaults.clone(),
                                    });
                                }
                                Stmt::EnumDef { name, variants } if name == &sym.name => {
                                    additional_stmts.push(Stmt::EnumDef {
                                        name: alias_name.clone(),
                                        variants: variants.clone(),
                                    });
                                }
                                Stmt::Const { name, value } if name == &sym.name => {
                                    additional_stmts.push(Stmt::Const {
                                        name: alias_name.clone(),
                                        value: value.clone(),
                                    });
                                }
                                Stmt::Let {
                                    name,
                                    type_ann,
                                    value,
                                } if name == &sym.name => {
                                    additional_stmts.push(Stmt::Let {
                                        name: alias_name.clone(),
                                        type_ann: type_ann.clone(),
                                        value: value.clone(),
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
                sub_resolved.extend(additional_stmts);
                let unwrapped_resolved: Vec<Stmt> = sub_resolved
                    .into_iter()
                    .map(|s| s.inner_stmt().clone())
                    .collect();
                out.extend(unwrapped_resolved);

                let mut exposed_fns = std::collections::HashSet::new();
                for sym in syms {
                    if sym.name == "*" {
                        if has_any_pub {
                            exposed_fns.extend(pub_symbol_names.clone());
                        } else {
                            exposed_fns.extend(local_fns.clone());
                        }
                    } else {
                        let final_name = sym.alias.as_ref().unwrap_or(&sym.name);
                        exposed_fns.insert(final_name.clone());
                    }
                }
                Ok(exposed_fns)
            } else if let Some(ref alias_str) = alias {
                let mut final_sub: Vec<Stmt> = sub_resolved
                    .into_iter()
                    .map(|s| s.inner_stmt().clone())
                    .collect();
                apply_module_alias(&mut final_sub, alias_str, &local_fns);
                out.extend(final_sub);
                Ok(std::collections::HashSet::new())
            } else {
                let unwrapped_resolved: Vec<Stmt> = sub_resolved
                    .into_iter()
                    .map(|s| s.inner_stmt().clone())
                    .collect();
                out.extend(unwrapped_resolved);
                Ok(local_fns)
            }
        }
        other => {
            out.push(other);
            Ok(std::collections::HashSet::new())
        }
    }
}

pub fn expand_default_args(program: &mut Program) {
    let mut module_stems = std::collections::HashSet::new();
    for std_mod in ["math", "time", "fs", "os", "path", "net", "sync", "color", "env", "process", "io", "crypto", "json", "random"] {
        module_stems.insert(std_mod.to_string());
    }
    expand_default_args_with_modules(program, &module_stems);
}

pub fn expand_default_args_with_modules(
    program: &mut Program,
    module_stems: &std::collections::HashSet<String>,
) {
    let mut fn_defs: std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)> =
        std::collections::HashMap::new();

    collect_fn_defaults(&program.statements, &mut fn_defs);

    for stmt in &mut program.statements {
        expand_defaults_in_stmt(stmt, &fn_defs, module_stems);
    }
}

fn collect_fn_defaults(
    stmts: &[Stmt],
    fn_defs: &mut std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                name,
                params,
                param_types,
                defaults,
                body,
                ..
            } => {
                let has_rest = param_types.last().and_then(|t| t.as_deref()) == Some("...");
                fn_defs.insert(name.clone(), (params.len(), defaults.clone(), has_rest));
                let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                if bare != name {
                    fn_defs.insert(bare.to_string(), (params.len(), defaults.clone(), has_rest));
                }
                collect_fn_defaults(body, fn_defs);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_fn_defaults(then_block, fn_defs);
                if let Some(eb) = else_block {
                    collect_fn_defaults(eb, fn_defs);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body, .. }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_fn_defaults(body, fn_defs);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_fn_defaults(try_block, fn_defs);
                collect_fn_defaults(catch_block, fn_defs);
                if let Some(fb) = finally_block {
                    collect_fn_defaults(fb, fn_defs);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_fn_defaults(std::slice::from_ref(inner), fn_defs);
            }
            _ => {}
        }
    }
}

fn expand_defaults_in_stmt(
    stmt: &mut Stmt,
    fn_defs: &std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)>,
    module_stems: &std::collections::HashSet<String>,
) {
    match stmt {
        Stmt::Function { body, .. } => {
            for s in body {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
        }
        Stmt::Say(expr)
        | Stmt::Expr(expr)
        | Stmt::Let { value: expr, .. }
        | Stmt::Assign { value: expr, .. } => {
            expand_defaults_in_expr(expr, fn_defs, module_stems);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            expand_defaults_in_expr(array, fn_defs, module_stems);
            expand_defaults_in_expr(index, fn_defs, module_stems);
            expand_defaults_in_expr(value, fn_defs, module_stems);
        }
        Stmt::FieldAssign { object, value, .. } => {
            expand_defaults_in_expr(object, fn_defs, module_stems);
            expand_defaults_in_expr(value, fn_defs, module_stems);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expand_defaults_in_expr(condition, fn_defs, module_stems);
            for s in then_block {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    expand_defaults_in_stmt(s, fn_defs, module_stems);
                }
            }
        }
        Stmt::While { condition, body } => {
            expand_defaults_in_expr(condition, fn_defs, module_stems);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expand_defaults_in_expr(start, fn_defs, module_stems);
            expand_defaults_in_expr(end, fn_defs, module_stems);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            expand_defaults_in_expr(iterable, fn_defs, module_stems);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            expand_defaults_in_expr(expr, fn_defs, module_stems);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
            for s in catch_block {
                expand_defaults_in_stmt(s, fn_defs, module_stems);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    expand_defaults_in_stmt(s, fn_defs, module_stems);
                }
            }
        }
        Stmt::Pub(inner) | Stmt::Defer(inner) => {
            expand_defaults_in_stmt(inner, fn_defs, module_stems);
        }
        _ => {}
    }
}

fn expand_defaults_in_expr(
    expr: &mut Expr,
    fn_defs: &std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)>,
    module_stems: &std::collections::HashSet<String>,
) {
    match expr {
        Expr::Call { name, args } => {
            for arg in args.iter_mut() {
                expand_defaults_in_expr(arg, fn_defs, module_stems);
            }
            if let Some(Expr::Identifier(prefix)) = args.first().cloned() {
                let cand_double = format!("{}__{}", prefix, name);
                let cand_colon = format!("{}::{}", prefix, name);
                if fn_defs.contains_key(&cand_double) {
                    *name = cand_double;
                    args.remove(0);
                } else if fn_defs.contains_key(&cand_colon) {
                    *name = cand_colon;
                    args.remove(0);
                } else if module_stems.contains(&prefix) && fn_defs.contains_key(name) {
                    args.remove(0);
                }
            }
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            if let Some((param_count, defaults, has_rest)) =
                fn_defs.get(name).or_else(|| fn_defs.get(bare))
            {
                if *has_rest {
                    let fixed_count = param_count.saturating_sub(1);
                    if args.len() < fixed_count {
                        for i in args.len()..fixed_count {
                            if let Some(Some(def_expr)) = defaults.get(i) {
                                args.push(def_expr.clone());
                            }
                        }
                        args.push(Expr::Array(vec![]));
                    } else if args.len() == fixed_count {
                        args.push(Expr::Array(vec![]));
                    } else if args.len() == *param_count {
                        if !matches!(args.last(), Some(Expr::Array(_))) {
                            let last = args.pop().unwrap();
                            args.push(Expr::Array(vec![last]));
                        }
                    } else {
                        let rest_items: Vec<Expr> = args.drain(fixed_count..).collect();
                        args.push(Expr::Array(rest_items));
                    }
                } else if args.len() < *param_count {
                    for i in args.len()..*param_count {
                        if let Some(Some(def_expr)) = defaults.get(i) {
                            args.push(def_expr.clone());
                        }
                    }
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            expand_defaults_in_expr(left, fn_defs, module_stems);
            expand_defaults_in_expr(right, fn_defs, module_stems);
        }
        Expr::Unary { expr, .. } => {
            expand_defaults_in_expr(expr, fn_defs, module_stems);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expand_defaults_in_expr(condition, fn_defs, module_stems);
            expand_defaults_in_expr(then_branch, fn_defs, module_stems);
            expand_defaults_in_expr(else_branch, fn_defs, module_stems);
        }
        Expr::NullCoalesce { value, default } => {
            expand_defaults_in_expr(value, fn_defs, module_stems);
            expand_defaults_in_expr(default, fn_defs, module_stems);
        }
        Expr::Array(elems) => {
            for elem in elems {
                expand_defaults_in_expr(elem, fn_defs, module_stems);
            }
        }
        Expr::Index { array, index } => {
            expand_defaults_in_expr(array, fn_defs, module_stems);
            expand_defaults_in_expr(index, fn_defs, module_stems);
        }
        Expr::FieldAccess { object, .. } => {
            expand_defaults_in_expr(object, fn_defs, module_stems);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                expand_defaults_in_expr(val, fn_defs, module_stems);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                expand_defaults_in_expr(k, fn_defs, module_stems);
                expand_defaults_in_expr(v, fn_defs, module_stems);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                expand_defaults_in_expr(part, fn_defs, module_stems);
            }
        }
        Expr::TypeCheck { expr, .. } | Expr::Cast { expr, .. } => {
            expand_defaults_in_expr(expr, fn_defs, module_stems);
        }
        _ => {}
    }
}
