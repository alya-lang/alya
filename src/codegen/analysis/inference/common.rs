use crate::ast::*;
use std::collections::HashMap;

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
