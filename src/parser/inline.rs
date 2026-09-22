use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

/// Inline candidate: a non-generic, non-recursive function whose body is a
/// single `return EXPR;` statement.
struct InlineCandidate {
    params: Vec<String>,
    body: Expr,
}

/// Returns true for side-effect-free argument expressions. Only pure
/// arguments are substituted (in any expression position) so evaluation
/// order and effect counts are preserved exactly.
fn is_pure_arg(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null | Expr::Identifier(_) => {
            true
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => items.iter().all(is_pure_arg),
        Expr::Map(entries) => entries
            .iter()
            .all(|(k, v)| is_pure_arg(k) && is_pure_arg(v)),
        Expr::StructInit { fields, .. } => fields.iter().all(|(_, v)| is_pure_arg(v)),
        Expr::Binary { left, right, .. } => is_pure_arg(left) && is_pure_arg(right),
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => is_pure_arg(expr),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => is_pure_arg(condition) && is_pure_arg(then_branch) && is_pure_arg(else_branch),
        Expr::NullCoalesce { value, default } => is_pure_arg(value) && is_pure_arg(default),
        Expr::Index { array, index } => is_pure_arg(array) && is_pure_arg(index),
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            is_pure_arg(object)
        }
        Expr::OptionalIndex { array, index } => is_pure_arg(array) && is_pure_arg(index),
        Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => is_pure_arg(expr),
        // Calls (including optional/method calls) may have side effects.
        _ => false,
    }
}

/// Substitutes parameter identifiers with argument expressions.
fn substitute_params(expr: &Expr, subst: &HashMap<String, Expr>) -> Expr {
    match expr {
        Expr::Identifier(name) => subst.get(name).cloned().unwrap_or_else(|| expr.clone()),
        Expr::Binary { left, op, right } => Expr::Binary {
            left: Box::new(substitute_params(left, subst)),
            op: *op,
            right: Box::new(substitute_params(right, subst)),
        },
        Expr::Unary { op, expr } => Expr::Unary {
            op: *op,
            expr: Box::new(substitute_params(expr, subst)),
        },
        Expr::Call { name, args } => Expr::Call {
            name: name.clone(),
            args: args.iter().map(|a| substitute_params(a, subst)).collect(),
        },
        Expr::OptionalCall { callee, args } => Expr::OptionalCall {
            callee: callee.clone(),
            args: args.iter().map(|a| substitute_params(a, subst)).collect(),
        },
        Expr::Array(items) => {
            Expr::Array(items.iter().map(|e| substitute_params(e, subst)).collect())
        }
        Expr::InterpolatedString(parts) => {
            Expr::InterpolatedString(parts.iter().map(|e| substitute_params(e, subst)).collect())
        }
        Expr::Index { array, index } => Expr::Index {
            array: Box::new(substitute_params(array, subst)),
            index: Box::new(substitute_params(index, subst)),
        },
        Expr::OptionalIndex { array, index } => Expr::OptionalIndex {
            array: Box::new(substitute_params(array, subst)),
            index: Box::new(substitute_params(index, subst)),
        },
        Expr::FieldAccess { object, field } => Expr::FieldAccess {
            object: Box::new(substitute_params(object, subst)),
            field: field.clone(),
        },
        Expr::OptionalFieldAccess { object, field } => Expr::OptionalFieldAccess {
            object: Box::new(substitute_params(object, subst)),
            field: field.clone(),
        },
        Expr::StructInit { name, fields } => Expr::StructInit {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), substitute_params(v, subst)))
                .collect(),
        },
        Expr::Map(entries) => Expr::Map(
            entries
                .iter()
                .map(|(k, v)| (substitute_params(k, subst), substitute_params(v, subst)))
                .collect(),
        ),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => Expr::Ternary {
            condition: Box::new(substitute_params(condition, subst)),
            then_branch: Box::new(substitute_params(then_branch, subst)),
            else_branch: Box::new(substitute_params(else_branch, subst)),
        },
        Expr::NullCoalesce { value, default } => Expr::NullCoalesce {
            value: Box::new(substitute_params(value, subst)),
            default: Box::new(substitute_params(default, subst)),
        },
        Expr::Cast { expr, target } => Expr::Cast {
            expr: Box::new(substitute_params(expr, subst)),
            target: target.clone(),
        },
        Expr::TypeCheck {
            expr,
            target,
            negated,
        } => Expr::TypeCheck {
            expr: Box::new(substitute_params(expr, subst)),
            target: target.clone(),
            negated: *negated,
        },
        Expr::ForceUnwrap(inner) => Expr::ForceUnwrap(Box::new(substitute_params(inner, subst))),
        _ => expr.clone(),
    }
}

/// True when the expression contains a call to `name` (bare or qualified).
fn calls_function(expr: &Expr, name: &str) -> bool {
    let bare_match = |callee: &str| {
        callee == name
            || callee.rsplit("::").next().unwrap_or(callee) == name
            || callee.rsplit("__").next().unwrap_or(callee) == name
    };
    match expr {
        Expr::Call { name: callee, args } | Expr::OptionalCall { callee, args } => {
            if bare_match(callee) {
                return true;
            }
            args.iter().any(|a| calls_function(a, name))
        }
        Expr::Binary { left, right, .. } => {
            calls_function(left, name) || calls_function(right, name)
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => calls_function(expr, name),
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            items.iter().any(|e| calls_function(e, name))
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            calls_function(array, name) || calls_function(index, name)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            calls_function(object, name)
        }
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| calls_function(v, name)),
        Expr::Map(entries) => entries
            .iter()
            .any(|(k, v)| calls_function(k, name) || calls_function(v, name)),
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            calls_function(condition, name)
                || calls_function(then_branch, name)
                || calls_function(else_branch, name)
        }
        Expr::NullCoalesce { value, default } => {
            calls_function(value, name) || calls_function(default, name)
        }
        Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => calls_function(expr, name),
        _ => false,
    }
}

fn collect_candidates(program: &Program) -> HashMap<String, InlineCandidate> {
    let mut out = HashMap::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name,
            params,
            param_types,
            body,
            type_params,
            attributes,
            ..
        } = stmt.inner_stmt()
        {
            let wants_inline = attributes.iter().any(|a| a.name == "inline");
            let blocked = attributes.iter().any(|a| a.name == "noinline");
            if !wants_inline || blocked || !type_params.is_empty() {
                continue;
            }
            // Rest (`...`) parameters change arity semantics; skip for v1.
            if param_types.iter().any(|t| {
                t.as_deref()
                    .is_some_and(|s| s.starts_with("...") || s == "...")
            }) {
                continue;
            }
            if let [Stmt::Return(Some(ret))] = body.as_slice() {
                if calls_function(ret, name) {
                    continue;
                }
                out.insert(
                    name.clone(),
                    InlineCandidate {
                        params: params.clone(),
                        body: ret.clone(),
                    },
                );
            }
        }
    }
    out
}

/// Rewrites eligible calls in an expression. Returns true on any change.
fn inline_in_expr(expr: &mut Expr, candidates: &HashMap<String, InlineCandidate>) -> bool {
    // Recurse first so nested inline calls normalize bottom-up.
    let changed = match expr {
        Expr::Binary { left, right, .. } => {
            inline_in_expr(left, candidates) | inline_in_expr(right, candidates)
        }
        Expr::Unary { expr, .. } | Expr::ForceUnwrap(expr) => inline_in_expr(expr, candidates),
        Expr::Call { args, .. } | Expr::OptionalCall { args, .. } => {
            let mut c = false;
            for a in args.iter_mut() {
                c |= inline_in_expr(a, candidates);
            }
            c
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            let mut c = false;
            for e in items.iter_mut() {
                c |= inline_in_expr(e, candidates);
            }
            c
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            inline_in_expr(array, candidates) | inline_in_expr(index, candidates)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            inline_in_expr(object, candidates)
        }
        Expr::StructInit { fields, .. } => {
            let mut c = false;
            for (_, v) in fields.iter_mut() {
                c |= inline_in_expr(v, candidates);
            }
            c
        }
        Expr::Map(entries) => {
            let mut c = false;
            for (k, v) in entries.iter_mut() {
                c |= inline_in_expr(k, candidates);
                c |= inline_in_expr(v, candidates);
            }
            c
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            inline_in_expr(condition, candidates)
                | inline_in_expr(then_branch, candidates)
                | inline_in_expr(else_branch, candidates)
        }
        Expr::NullCoalesce { value, default } => {
            inline_in_expr(value, candidates) | inline_in_expr(default, candidates)
        }
        Expr::Cast { expr, .. } | Expr::TypeCheck { expr, .. } => inline_in_expr(expr, candidates),
        _ => false,
    };

    // Then try this node itself (covers chains created by recursion).
    let target = match expr {
        Expr::Call { name, .. } => Some(name.clone()),
        Expr::OptionalCall { callee, .. } => Some(callee.clone()),
        _ => None,
    };
    if let Some(callee) = target {
        if let Some(cand) = lookup_candidate(&callee, candidates) {
            if let Expr::Call { args, .. } | Expr::OptionalCall { args, .. } = expr {
                if args.len() == cand.params.len() && args.iter().all(is_pure_arg) {
                    let subst: HashMap<String, Expr> = cand
                        .params
                        .iter()
                        .cloned()
                        .zip(args.iter().cloned())
                        .collect();
                    *expr = substitute_params(&cand.body, &subst);
                    return true;
                }
            }
        }
    }
    changed
}

fn lookup_candidate<'a>(
    callee: &str,
    candidates: &'a HashMap<String, InlineCandidate>,
) -> Option<&'a InlineCandidate> {
    if let Some(c) = candidates.get(callee) {
        return Some(c);
    }
    let bare = callee.rsplit("::").next().unwrap_or(callee);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    candidates.get(bare)
}

fn inline_in_stmt(stmt: &mut Stmt, candidates: &HashMap<String, InlineCandidate>) -> bool {
    match stmt.inner_stmt_mut() {
        Stmt::Function { body, .. } => {
            let mut c = false;
            for s in body.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            c
        }
        Stmt::Let { value, .. }
        | Stmt::Const { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => inline_in_expr(value, candidates),
        Stmt::Assign { value, .. } => inline_in_expr(value, candidates),
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            inline_in_expr(array, candidates)
                | inline_in_expr(index, candidates)
                | inline_in_expr(value, candidates)
        }
        Stmt::FieldAssign { object, value, .. } => {
            inline_in_expr(object, candidates) | inline_in_expr(value, candidates)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            let mut c = inline_in_expr(condition, candidates);
            for s in then_block.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            if let Some(eb) = else_block {
                for s in eb.iter_mut() {
                    c |= inline_in_stmt(s, candidates);
                }
            }
            c
        }
        Stmt::While { condition, body } => {
            let mut c = inline_in_expr(condition, candidates);
            for s in body.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            c
        }
        Stmt::Repeat { body } => {
            let mut c = false;
            for s in body.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            c
        }
        Stmt::For {
            start, end, body, ..
        } => {
            let mut c = inline_in_expr(start, candidates);
            c |= inline_in_expr(end, candidates);
            for s in body.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            c
        }
        Stmt::ForEach { iterable, body, .. } => {
            let mut c = inline_in_expr(iterable, candidates);
            for s in body.iter_mut() {
                c |= inline_in_stmt(s, candidates);
            }
            c
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            let mut c = false;
            for s in try_block.iter_mut().chain(catch_block.iter_mut()) {
                c |= inline_in_stmt(s, candidates);
            }
            if let Some(fb) = finally_block {
                for s in fb.iter_mut() {
                    c |= inline_in_stmt(s, candidates);
                }
            }
            c
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => inline_in_stmt(inner, candidates),
        _ => false,
    }
}

/// Inlines `@inline` function calls (Chapter 18 §1.2) ahead of codegen.
///
/// Scope (v1): single-`return`-expression bodies, exact-arity calls with
/// side-effect-free arguments, non-recursive. `@noinline` always wins.
/// Runs to fixpoint (bounded) so nested inline calls collapse layer by layer.
pub fn inline_functions(program: &mut Program) {
    for _ in 0..8 {
        let candidates = collect_candidates(program);
        if candidates.is_empty() {
            return;
        }
        let mut changed = false;
        for stmt in program.statements.iter_mut() {
            changed |= inline_in_stmt(stmt, &candidates);
        }
        if !changed {
            return;
        }
    }
}
