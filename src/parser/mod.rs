pub mod constants;
pub mod enums;
pub mod expr;
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
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
            lambda_functions: Vec::new(),
            lambda_counter: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        self.skip_newlines();

        while !matches!(self.current_token().token_type, TokenType::Eof) {
            statements.extend(self.parse_statement()?);
            self.skip_newlines();
        }

        statements.extend(self.lambda_functions.drain(..));

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
}

pub fn resolve_imports_with_sources(
    program: &mut Program,
    base_dir: &std::path::Path,
) -> Result<std::collections::HashSet<std::path::PathBuf>, String> {
    let mut visited = std::collections::HashSet::new();
    let mut resolved_stmts = Vec::new();

    for stmt in std::mem::take(&mut program.statements) {
        resolve_stmt_imports(stmt, base_dir, &mut visited, &mut resolved_stmts)?;
    }

    validate_unique_functions(&resolved_stmts)?;

    program.statements = resolved_stmts;
    expand_default_args(program);
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
            for s in body {
                prefix_stmt(s, alias, local_fns);
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
        _ => {}
    }
}

fn get_embedded_stdlib(module: &str) -> Option<&'static str> {
    let clean = module
        .strip_prefix("std/")
        .or_else(|| module.strip_prefix("std::"))
        .unwrap_or(module);
    let clean = clean.strip_suffix(".alya").unwrap_or(clean);
    match clean {
        "math" => Some(include_str!("../../stdlib/math.alya")),
        "time" => Some(include_str!("../../stdlib/time.alya")),
        "os" => Some(include_str!("../../stdlib/os.alya")),
        "json" => Some(include_str!("../../stdlib/json.alya")),
        "mem" => Some(include_str!("../../stdlib/mem.alya")),
        "str" => Some(include_str!("../../stdlib/str.alya")),
        "path" => Some(include_str!("../../stdlib/path.alya")),
        "fs" => Some(include_str!("../../stdlib/fs.alya")),
        "hash" => Some(include_str!("../../stdlib/hash.alya")),
        "collections" => Some(include_str!("../../stdlib/collections.alya")),
        "test" => Some(include_str!("../../stdlib/test.alya")),
        "bench" => Some(include_str!("../../stdlib/bench.alya")),
        "rand" => Some(include_str!("../../stdlib/rand.alya")),
        "cli" | "argparse" => Some(include_str!("../../stdlib/cli.alya")),
        "color" | "term" | "ansi" => Some(include_str!("../../stdlib/color.alya")),
        "log" | "logger" => Some(include_str!("../../stdlib/log.alya")),
        "glob" => Some(include_str!("../../stdlib/glob.alya")),
        "console" => Some(include_str!("../../stdlib/console.alya")),
        "net" | "http" => Some(include_str!("../../stdlib/net.alya")),
        "sync" | "synchronization" => Some(include_str!("../../stdlib/sync.alya")),
        "thread" | "threads" | "concurrency" => Some(include_str!("../../stdlib/thread.alya")),
        _ => None,
    }
}

fn resolve_stmt_imports(
    stmt: Stmt,
    current_dir: &std::path::Path,
    visited: &mut std::collections::HashSet<(std::path::PathBuf, Option<String>)>,
    out: &mut Vec<Stmt>,
) -> Result<std::collections::HashSet<String>, String> {
    match stmt {
        Stmt::Import {
            path: import_path_str,
            alias,
            symbols,
        } => {
            // Normalize path separators to '/' so Windows-style '\' works across Linux, macOS, and Windows
            let normalized_path = import_path_str.replace('\\', "/");
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
                let std_dir = current_dir.join("stdlib").join(clean);
                let std_root = std::path::Path::new("stdlib").join(clean);
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
                if let Some(src) = get_embedded_stdlib(&normalized_path) {
                    let synthetic =
                        std::path::PathBuf::from(format!("<embedded:{}>", normalized_path));
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
            let mut local_fns: std::collections::HashSet<String> =
                if !is_embedded_stdlib || alias.is_some() {
                    sub_program
                        .statements
                        .iter()
                        .filter_map(|s| match s {
                            Stmt::Function { name, .. } => Some(name.clone()),
                            _ => None,
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
                    resolve_stmt_imports(sub_stmt, sub_dir, visited, &mut sub_resolved)?;
                if is_unaliased_import && (!is_embedded_stdlib || alias.is_some()) {
                    local_fns.extend(child_fns);
                }
            }

            if let Some(ref syms) = symbols {
                // Selective import: from "..." import a, b [as c]
                // 1. Verify requested symbols exist
                for sym in syms {
                    if sym.name == "*" {
                        continue;
                    }
                    let exists = sub_resolved.iter().any(|s| match s {
                        Stmt::Function { name, .. } => name == &sym.name,
                        Stmt::StructDef { name, .. } => name == &sym.name,
                        Stmt::EnumDef { name, .. } => name == &sym.name,
                        Stmt::Const { name, .. } => name == &sym.name,
                        _ => false,
                    });
                    if !exists {
                        return Err(format!(
                            "Module '{}' does not export symbol '{}'",
                            import_path_str, sym.name
                        ));
                    }
                }

                // 2. For aliased symbols, clone and rename definitions
                let mut additional_stmts = Vec::new();
                for sym in syms {
                    if let Some(ref alias_name) = sym.alias {
                        for s in &sub_resolved {
                            match s {
                                Stmt::Function {
                                    name,
                                    params,
                                    param_types,
                                    return_type,
                                    defaults,
                                    body,
                                } if name == &sym.name => {
                                    additional_stmts.push(Stmt::Function {
                                        name: alias_name.clone(),
                                        params: params.clone(),
                                        param_types: param_types.clone(),
                                        return_type: return_type.clone(),
                                        defaults: defaults.clone(),
                                        body: body.clone(),
                                    });
                                }
                                Stmt::StructDef {
                                    name,
                                    fields,
                                    defaults,
                                } if name == &sym.name => {
                                    additional_stmts.push(Stmt::StructDef {
                                        name: alias_name.clone(),
                                        fields: fields.clone(),
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
                                _ => {}
                            }
                        }
                    }
                }
                sub_resolved.extend(additional_stmts);
                out.extend(sub_resolved);

                let mut exposed_fns = std::collections::HashSet::new();
                for sym in syms {
                    if sym.name == "*" {
                        exposed_fns.extend(local_fns.clone());
                    } else {
                        let final_name = sym.alias.as_ref().unwrap_or(&sym.name);
                        exposed_fns.insert(final_name.clone());
                    }
                }
                Ok(exposed_fns)
            } else if let Some(ref alias_str) = alias {
                apply_module_alias(&mut sub_resolved, alias_str, &local_fns);
                out.extend(sub_resolved);
                Ok(std::collections::HashSet::new())
            } else {
                out.extend(sub_resolved);
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
    let mut fn_defs: std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)> =
        std::collections::HashMap::new();

    collect_fn_defaults(&program.statements, &mut fn_defs);

    for stmt in &mut program.statements {
        expand_defaults_in_stmt(stmt, &fn_defs);
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
            _ => {}
        }
    }
}

fn expand_defaults_in_stmt(
    stmt: &mut Stmt,
    fn_defs: &std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)>,
) {
    match stmt {
        Stmt::Function { body, .. } => {
            for s in body {
                expand_defaults_in_stmt(s, fn_defs);
            }
        }
        Stmt::Say(expr)
        | Stmt::Expr(expr)
        | Stmt::Let { value: expr, .. }
        | Stmt::Assign { value: expr, .. } => {
            expand_defaults_in_expr(expr, fn_defs);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            expand_defaults_in_expr(array, fn_defs);
            expand_defaults_in_expr(index, fn_defs);
            expand_defaults_in_expr(value, fn_defs);
        }
        Stmt::FieldAssign { object, value, .. } => {
            expand_defaults_in_expr(object, fn_defs);
            expand_defaults_in_expr(value, fn_defs);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expand_defaults_in_expr(condition, fn_defs);
            for s in then_block {
                expand_defaults_in_stmt(s, fn_defs);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    expand_defaults_in_stmt(s, fn_defs);
                }
            }
        }
        Stmt::While { condition, body } => {
            expand_defaults_in_expr(condition, fn_defs);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                expand_defaults_in_stmt(s, fn_defs);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expand_defaults_in_expr(start, fn_defs);
            expand_defaults_in_expr(end, fn_defs);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            expand_defaults_in_expr(iterable, fn_defs);
            for s in body {
                expand_defaults_in_stmt(s, fn_defs);
            }
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            expand_defaults_in_expr(expr, fn_defs);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                expand_defaults_in_stmt(s, fn_defs);
            }
            for s in catch_block {
                expand_defaults_in_stmt(s, fn_defs);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    expand_defaults_in_stmt(s, fn_defs);
                }
            }
        }
        _ => {}
    }
}

fn expand_defaults_in_expr(
    expr: &mut Expr,
    fn_defs: &std::collections::HashMap<String, (usize, Vec<Option<Expr>>, bool)>,
) {
    match expr {
        Expr::Call { name, args } => {
            for arg in args.iter_mut() {
                expand_defaults_in_expr(arg, fn_defs);
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
            expand_defaults_in_expr(left, fn_defs);
            expand_defaults_in_expr(right, fn_defs);
        }
        Expr::Unary { expr, .. } => {
            expand_defaults_in_expr(expr, fn_defs);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expand_defaults_in_expr(condition, fn_defs);
            expand_defaults_in_expr(then_branch, fn_defs);
            expand_defaults_in_expr(else_branch, fn_defs);
        }
        Expr::NullCoalesce { value, default } => {
            expand_defaults_in_expr(value, fn_defs);
            expand_defaults_in_expr(default, fn_defs);
        }
        Expr::Array(elems) => {
            for elem in elems {
                expand_defaults_in_expr(elem, fn_defs);
            }
        }
        Expr::Index { array, index } => {
            expand_defaults_in_expr(array, fn_defs);
            expand_defaults_in_expr(index, fn_defs);
        }
        Expr::FieldAccess { object, .. } => {
            expand_defaults_in_expr(object, fn_defs);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                expand_defaults_in_expr(val, fn_defs);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                expand_defaults_in_expr(k, fn_defs);
                expand_defaults_in_expr(v, fn_defs);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                expand_defaults_in_expr(part, fn_defs);
            }
        }
        _ => {}
    }
}
