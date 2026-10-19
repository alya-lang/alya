use crate::ast::*;
use std::collections::{HashMap, HashSet};

type FunctionDef<'a> = (&'a str, &'a [String], &'a [Option<String>], &'a [Stmt]);

pub fn collect_function_defs<'a>(stmts: &'a [Stmt], defs: &mut Vec<FunctionDef<'a>>) {
    for stmt in stmts {
        match stmt {
            Stmt::Function {
                name,
                params,
                param_types,
                body,
                ..
            } => {
                defs.push((
                    name.as_str(),
                    params.as_slice(),
                    param_types.as_slice(),
                    body.as_slice(),
                ));
                collect_function_defs(body, defs);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_function_defs(then_block, defs);
                if let Some(eb) = else_block {
                    collect_function_defs(eb, defs);
                }
            }
            Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::Repeat { body } => {
                collect_function_defs(body, defs);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_function_defs(try_block, defs);
                collect_function_defs(catch_block, defs);
                if let Some(finally_block) = finally_block {
                    collect_function_defs(finally_block, defs);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_function_defs(std::slice::from_ref(inner), defs);
            }
            _ => {}
        }
    }
}

/// Counts assignments per name within one scope (nested function
/// definitions own their scopes and are skipped). Single-assignment
/// (`counts == 1`) lets strict facts treat a `let` binding as an
/// alias for its value.
pub fn count_assignments(stmts: &[Stmt], counts: &mut HashMap<String, usize>) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Let { name, .. } | Stmt::Assign { name, .. } => {
                *counts.entry(name.clone()).or_insert(0) += 1;
            }
            Stmt::For { var, body, .. } => {
                *counts.entry(var.clone()).or_insert(0) += 1;
                count_assignments(body, counts);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                *counts.entry(var.clone()).or_insert(0) += 1;
                if let Some(v) = value_var {
                    *counts.entry(v.clone()).or_insert(0) += 1;
                }
                count_assignments(body, counts);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                count_assignments(then_block, counts);
                if let Some(eb) = else_block {
                    count_assignments(eb, counts);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => count_assignments(body, counts),
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                count_assignments(try_block, counts);
                count_assignments(catch_block, counts);
                if let Some(fb) = finally_block {
                    count_assignments(fb, counts);
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                count_assignments(std::slice::from_ref(inner), counts)
            }
            // Nested definitions are separate scopes: never counted here.
            _ => {}
        }
    }
}

/// Whether `expr` carries one of `names` through unchanged: a bare read or
/// a transparent wrapper (ternary/null-coalesce arms, force-unwrap), the
/// shapes `when`/`else` sugar lowers to. Shared by the string and float
/// return-position vetoes.
pub fn expr_passthroughs_dynamic(expr: &Expr, names: &HashSet<String>) -> bool {
    match expr {
        Expr::Identifier(n) => names.contains(n),
        Expr::Ternary {
            then_branch,
            else_branch,
            ..
        } => {
            expr_passthroughs_dynamic(then_branch, names)
                || expr_passthroughs_dynamic(else_branch, names)
        }
        Expr::NullCoalesce { value, default } => {
            expr_passthroughs_dynamic(value, names) || expr_passthroughs_dynamic(default, names)
        }
        Expr::ForceUnwrap(inner) => expr_passthroughs_dynamic(inner, names),
        _ => false,
    }
}

/// Extends `names` with locals that receive one of `names` through a direct
/// passthrough assignment, recursing through the function body (nested
/// functions own their scopes).
pub fn collect_passthrough_names(stmts: &[Stmt], names: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Let { name, value, .. }
            | Stmt::Const { name, value }
            | Stmt::Assign { name, value } => {
                if expr_passthroughs_dynamic(value, names) {
                    names.insert(name.clone());
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_passthrough_names(then_block, names);
                if let Some(eb) = else_block {
                    collect_passthrough_names(eb, names);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_passthrough_names(body, names);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_passthrough_names(try_block, names);
                collect_passthrough_names(catch_block, names);
                if let Some(fb) = finally_block {
                    collect_passthrough_names(fb, names);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_passthrough_names(std::slice::from_ref(inner), names);
            }
            // Nested functions own their returns.
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
}

/// True when a `return` in this body forwards one of `names`.
pub fn returns_passthrough_dynamic(stmts: &[Stmt], names: &HashSet<String>) -> bool {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Return(Some(expr)) if expr_passthroughs_dynamic(expr, names) => return true,
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if returns_passthrough_dynamic(then_block, names)
                    || else_block
                        .as_ref()
                        .is_some_and(|eb| returns_passthrough_dynamic(eb, names))
                {
                    return true;
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                if returns_passthrough_dynamic(body, names) {
                    return true;
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                if returns_passthrough_dynamic(try_block, names)
                    || returns_passthrough_dynamic(catch_block, names)
                    || finally_block
                        .as_ref()
                        .is_some_and(|fb| returns_passthrough_dynamic(fb, names))
                {
                    return true;
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                if returns_passthrough_dynamic(std::slice::from_ref(inner), names) {
                    return true;
                }
            }
            // Nested functions own their returns; do not descend.
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
    false
}

/// Collects every local that owns an assignment in this body: `let`,
/// `const`, `=` targets, loop variables, and catch bindings (the last two
/// get no tracked expression, so callers treat them as unknown-kind).
/// Nested functions are separate scopes and are skipped.
pub fn collect_local_assignments<'a>(
    stmts: &'a [Stmt],
    exprs: &mut HashMap<String, Vec<&'a Expr>>,
    opaque: &mut HashSet<String>,
    typed_floats: &mut HashSet<String>,
) {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => {
                // An explicit float annotation is a static contract: the
                // slot is float even when the RHS kind is unknown (the
                // codegen converts int/unknown values on the way in).
                if matches!(
                    type_ann.as_deref(),
                    Some("float") | Some("f64") | Some("f32") | Some("double")
                ) {
                    typed_floats.insert(name.clone());
                }
                exprs.entry(name.clone()).or_default().push(value);
            }
            Stmt::Const { name, value } => {
                exprs.entry(name.clone()).or_default().push(value);
            }
            Stmt::Assign { name, value } => {
                exprs.entry(name.clone()).or_default().push(value);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_local_assignments(then_block, exprs, opaque, typed_floats);
                if let Some(eb) = else_block {
                    collect_local_assignments(eb, exprs, opaque, typed_floats);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                collect_local_assignments(body, exprs, opaque, typed_floats);
            }
            Stmt::For { var, body, .. } => {
                opaque.insert(var.clone());
                collect_local_assignments(body, exprs, opaque, typed_floats);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                opaque.insert(var.clone());
                if let Some(v) = value_var {
                    opaque.insert(v.clone());
                }
                collect_local_assignments(body, exprs, opaque, typed_floats);
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
            } => {
                if let Some(v) = catch_var {
                    opaque.insert(v.clone());
                }
                collect_local_assignments(try_block, exprs, opaque, typed_floats);
                collect_local_assignments(catch_block, exprs, opaque, typed_floats);
                if let Some(fb) = finally_block {
                    collect_local_assignments(fb, exprs, opaque, typed_floats);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_local_assignments(std::slice::from_ref(inner), exprs, opaque, typed_floats);
            }
            // Nested functions own their returns.
            Stmt::Function { .. } => {}
            _ => {}
        }
    }
}
