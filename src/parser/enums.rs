use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

/// Collect all enum definitions and resolve their variants to compile-time constant expressions.
pub fn resolve_enums(program: &mut Program) {
    let mut enums: HashMap<String, HashMap<String, Expr>> = HashMap::new();

    for stmt in &program.statements {
        if let Stmt::EnumDef { name, variants } = stmt {
            let mut variant_map = HashMap::new();
            let mut next_auto_int = 0.0;
            for (vname, val_opt) in variants {
                let expr = match val_opt {
                    Some(e) => {
                        if let Expr::Number(n) = e {
                            next_auto_int = n + 1.0;
                        }
                        e.clone()
                    }
                    None => {
                        let e = Expr::Number(next_auto_int);
                        next_auto_int += 1.0;
                        e
                    }
                };
                variant_map.insert(vname.clone(), expr);
            }
            enums.insert(name.clone(), variant_map.clone());
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare != name {
                enums.insert(bare.to_string(), variant_map);
            }
        }
    }

    if enums.is_empty() {
        return;
    }

    for stmt in &mut program.statements {
        resolve_enums_in_stmt(stmt, &enums);
    }
}

pub fn resolve_enums_in_stmt(stmt: &mut Stmt, enums: &HashMap<String, HashMap<String, Expr>>) {
    match stmt {
        Stmt::Say(expr) | Stmt::Expr(expr) => resolve_enums_in_expr(expr, enums),
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            resolve_enums_in_expr(value, enums);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            resolve_enums_in_expr(condition, enums);
            for s in then_block {
                resolve_enums_in_stmt(s, enums);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    resolve_enums_in_stmt(s, enums);
                }
            }
        }
        Stmt::While { condition, body } => {
            resolve_enums_in_expr(condition, enums);
            for s in body {
                resolve_enums_in_stmt(s, enums);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                resolve_enums_in_stmt(s, enums);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            resolve_enums_in_expr(start, enums);
            resolve_enums_in_expr(end, enums);
            for s in body {
                resolve_enums_in_stmt(s, enums);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            resolve_enums_in_expr(iterable, enums);
            for s in body {
                resolve_enums_in_stmt(s, enums);
            }
        }
        Stmt::Function { defaults, body, .. } => {
            for def in defaults.iter_mut().flatten() {
                resolve_enums_in_expr(def, enums);
            }
            for s in body {
                resolve_enums_in_stmt(s, enums);
            }
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            resolve_enums_in_expr(expr, enums);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            resolve_enums_in_expr(array, enums);
            resolve_enums_in_expr(index, enums);
            resolve_enums_in_expr(value, enums);
        }
        Stmt::FieldAssign { object, value, .. } => {
            resolve_enums_in_expr(object, enums);
            resolve_enums_in_expr(value, enums);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                resolve_enums_in_stmt(s, enums);
            }
            for s in catch_block {
                resolve_enums_in_stmt(s, enums);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    resolve_enums_in_stmt(s, enums);
                }
            }
        }
        Stmt::StructDef { defaults, .. } => {
            for def in defaults.iter_mut().flatten() {
                resolve_enums_in_expr(def, enums);
            }
        }
        _ => {}
    }
}

pub fn resolve_enums_in_expr(expr: &mut Expr, enums: &HashMap<String, HashMap<String, Expr>>) {
    match expr {
        Expr::FieldAccess { object, field } => {
            resolve_enums_in_expr(object, enums);
            if let Expr::Identifier(name) = &**object {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(variants) = enums.get(name).or_else(|| enums.get(bare)) {
                    if let Some(val) = variants.get(field) {
                        *expr = val.clone();
                    }
                }
            }
        }
        Expr::Identifier(ident) if ident.contains("::") => {
            let parts: Vec<&str> = ident.split("::").collect();
            if parts.len() >= 2 {
                let variant_name = parts.last().unwrap();
                let enum_name = parts[..parts.len() - 1].join("::");
                let bare_enum = parts[parts.len() - 2];
                if let Some(variants) = enums.get(&enum_name).or_else(|| enums.get(bare_enum)) {
                    if let Some(val) = variants.get(*variant_name) {
                        *expr = val.clone();
                    }
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            resolve_enums_in_expr(left, enums);
            resolve_enums_in_expr(right, enums);
        }
        Expr::Unary { expr: inner, .. } => {
            resolve_enums_in_expr(inner, enums);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                resolve_enums_in_expr(arg, enums);
            }
        }
        Expr::Array(items) => {
            for item in items {
                resolve_enums_in_expr(item, enums);
            }
        }
        Expr::Index { array, index } => {
            resolve_enums_in_expr(array, enums);
            resolve_enums_in_expr(index, enums);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                resolve_enums_in_expr(val, enums);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                resolve_enums_in_expr(k, enums);
                resolve_enums_in_expr(v, enums);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            resolve_enums_in_expr(condition, enums);
            resolve_enums_in_expr(then_branch, enums);
            resolve_enums_in_expr(else_branch, enums);
        }
        Expr::NullCoalesce { value, default } => {
            resolve_enums_in_expr(value, enums);
            resolve_enums_in_expr(default, enums);
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                resolve_enums_in_expr(part, enums);
            }
        }
        _ => {}
    }
}
