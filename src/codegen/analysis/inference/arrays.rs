use super::common::{collect_function_defs, count_assignments};
use crate::ast::*;
use crate::codegen::analysis::predicates::is_simple_name;
use crate::codegen::analysis::traversal::CallIndex;
use crate::parser::dynspec::{dynspec_codes, spec_open_param_kind};
use std::collections::{HashMap, HashSet};

fn expr_is_definitely_array(
    expr: &Expr,
    fn_scope: Option<&str>,
    known_arrays: &HashSet<String>,
) -> bool {
    match expr {
        Expr::Array(_) => true,
        Expr::Identifier(name) => {
            if let Some(scope) = fn_scope {
                known_arrays.contains(&format!("{}:{}", scope, name)) || known_arrays.contains(name)
            } else {
                known_arrays.contains(name)
            }
        }
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "split"
                    | "args"
                    | "cli_args"
                    | "keys"
                    | "values"
                    | "lines"
                    | "read_lines"
                    | "set_to_array"
                    | "stack_new"
                    | "queue_new"
                    | "array_slice"
                    | "array_clone"
                    | "array_concat"
                    | "array_reverse"
                    | "array_reverse_in_place"
                    | "array_unique"
                    | "array_sort"
                    | "array_sort_in_place"
                    | "array_chunk"
                    | "array_fill"
                    | "map_entries"
                    | "rand_sample"
                    | "rand_shuffle"
                    | "rand_shuffled"
                    | "list_dir"
                    | "read_dir"
                    | "fs_list_dir"
                    | "fs_read_dir"
                    | "list_dir_recursive"
                    | "fs_list_dir_recursive"
                    | "glob"
                    | "glob_dir"
            ) || (bare == "slice"
                && !args.is_empty()
                && expr_is_definitely_array(&args[0], fn_scope, known_arrays))
                || known_arrays.contains(&format!("fn_ret_arr:{}", name))
                // #101: qualified-spelled calls consult exact markers only.
                || (is_simple_name(name)
                    && known_arrays.contains(&format!("fn_ret_arr:{}", bare)))
        }
        Expr::ForceUnwrap(inner) => expr_is_definitely_array(inner, fn_scope, known_arrays),
        _ => false,
    }
}

fn expr_is_definitely_non_array(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_)
        | Expr::Float(_)
        | Expr::String(_)
        | Expr::Map(_)
        | Expr::StructInit { .. } => true,
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "map"
                    | "set_new"
                    | "map_new"
                    | "set_from_array"
                    | "set_union"
                    | "set_intersection"
                    | "set_difference"
                    | "map_clone"
                    | "map_merge"
                    | "map_from_entries"
            )
        }
        Expr::Binary {
            op:
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo
                | BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::And
                | BinaryOp::Or
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
                | BinaryOp::Shl
                | BinaryOp::Shr,
            ..
        } => true,
        Expr::Unary {
            op: UnaryOp::Not | UnaryOp::Negate | UnaryOp::BitNot,
            ..
        } => true,
        Expr::ForceUnwrap(inner) => expr_is_definitely_non_array(inner),
        _ => false,
    }
}

fn collect_return_exprs<'a>(stmts: &'a [Stmt], returns: &mut Vec<&'a Expr>) {
    for s in stmts {
        match s {
            Stmt::Return(Some(expr)) => returns.push(expr),
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_return_exprs(then_block, returns);
                if let Some(eb) = else_block {
                    collect_return_exprs(eb, returns);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => collect_return_exprs(body, returns),
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_return_exprs(try_block, returns);
                collect_return_exprs(catch_block, returns);
                if let Some(fb) = finally_block {
                    collect_return_exprs(fb, returns);
                }
            }
            _ => {}
        }
    }
}

fn stmts_return_array(
    stmts: &[Stmt],
    fn_scope: Option<&str>,
    known_arrays: &HashSet<String>,
) -> bool {
    let mut returns = Vec::new();
    collect_return_exprs(stmts, &mut returns);
    let non_null: Vec<_> = returns
        .into_iter()
        .filter(|e| !matches!(e, Expr::Null))
        .collect();
    !non_null.is_empty()
        && non_null
            .iter()
            .all(|e| expr_is_definitely_array(e, fn_scope, known_arrays))
}

fn expr_uses_param_as_array(param: &str, expr: &Expr) -> bool {
    match expr {
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if matches!(
                bare,
                "push"
                    | "pop"
                    | "insert"
                    | "remove"
                    | "array_push"
                    | "array_pop"
                    | "array_len"
                    | "arr_len"
                    | "array_slice"
                    | "array_clone"
                    | "array_reverse"
                    | "array_sort"
                    | "array_reverse_in_place"
                    | "array_sort_in_place"
            ) {
                if let Some(first) = args.first() {
                    if matches!(first, Expr::Identifier(id) if id == param) {
                        return true;
                    }
                }
            }
            args.iter().any(|arg| expr_uses_param_as_array(param, arg))
        }
        Expr::Binary { left, right, .. } => {
            expr_uses_param_as_array(param, left) || expr_uses_param_as_array(param, right)
        }
        Expr::Unary { expr, .. } => expr_uses_param_as_array(param, expr),
        Expr::ForceUnwrap(inner) => expr_uses_param_as_array(param, inner),
        Expr::Array(elements) => elements.iter().any(|e| expr_uses_param_as_array(param, e)),
        Expr::Index { array, index } => {
            if let Expr::Identifier(id) = &**array {
                if id == param && matches!(&**index, Expr::Number(_)) {
                    return true;
                }
            }
            expr_uses_param_as_array(param, array) || expr_uses_param_as_array(param, index)
        }
        Expr::FieldAccess { object, .. } => expr_uses_param_as_array(param, object),
        _ => false,
    }
}

fn stmt_uses_param_as_array(param: &str, stmt: &Stmt) -> bool {
    match stmt {
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            if let Expr::Identifier(id) = array {
                if id == param && !matches!(index, Expr::String(_)) {
                    return true;
                }
            }
            expr_uses_param_as_array(param, array)
                || expr_uses_param_as_array(param, index)
                || expr_uses_param_as_array(param, value)
        }
        Stmt::Expr(expr) | Stmt::Say(expr) => expr_uses_param_as_array(param, expr),
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
            expr_uses_param_as_array(param, value)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_uses_param_as_array(param, condition)
                || then_block
                    .iter()
                    .any(|s| stmt_uses_param_as_array(param, s))
                || else_block
                    .as_ref()
                    .is_some_and(|eb| eb.iter().any(|s| stmt_uses_param_as_array(param, s)))
        }
        Stmt::While { condition, body } => {
            expr_uses_param_as_array(param, condition)
                || body.iter().any(|s| stmt_uses_param_as_array(param, s))
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } => {
            body.iter().any(|s| stmt_uses_param_as_array(param, s))
        }
        Stmt::ForEach { iterable, body, .. } => {
            expr_uses_param_as_array(param, iterable)
                || matches!(iterable, Expr::Identifier(id) if id == param)
                || body.iter().any(|s| stmt_uses_param_as_array(param, s))
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => expr_uses_param_as_array(param, expr),
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            try_block.iter().any(|s| stmt_uses_param_as_array(param, s))
                || catch_block
                    .iter()
                    .any(|s| stmt_uses_param_as_array(param, s))
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| fb.iter().any(|s| stmt_uses_param_as_array(param, s)))
        }
        _ => false,
    }
}

pub fn param_is_used_as_array(param: &str, stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| stmt_uses_param_as_array(param, s))
}

fn expr_forwards_param_to_array(expr: &Expr, param: &str, known_arrays: &HashSet<String>) -> bool {
    match expr {
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (idx, arg) in args.iter().enumerate() {
                if matches!(arg, Expr::Identifier(id) if id == param)
                    && (known_arrays.contains(&format!("fn_param_arr:{}:{}", name, idx))
                        // #101: qualified-spelled calls consult exact only.
                        || (is_simple_name(name)
                            && known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx))))
                {
                    return true;
                }
                if expr_forwards_param_to_array(arg, param, known_arrays) {
                    return true;
                }
            }
            false
        }
        Expr::Binary { left, right, .. } => {
            expr_forwards_param_to_array(left, param, known_arrays)
                || expr_forwards_param_to_array(right, param, known_arrays)
        }
        Expr::Unary { expr, .. } => expr_forwards_param_to_array(expr, param, known_arrays),
        Expr::ForceUnwrap(inner) => expr_forwards_param_to_array(inner, param, known_arrays),
        Expr::Array(elements) => elements
            .iter()
            .any(|e| expr_forwards_param_to_array(e, param, known_arrays)),
        _ => false,
    }
}

fn stmt_forwards_param_to_array(stmt: &Stmt, param: &str, known_arrays: &HashSet<String>) -> bool {
    match stmt {
        Stmt::Expr(expr) | Stmt::Say(expr) => {
            expr_forwards_param_to_array(expr, param, known_arrays)
        }
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } => {
            expr_forwards_param_to_array(value, param, known_arrays)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_forwards_param_to_array(condition, param, known_arrays)
                || then_block
                    .iter()
                    .any(|s| stmt_forwards_param_to_array(s, param, known_arrays))
                || else_block.as_ref().is_some_and(|eb| {
                    eb.iter()
                        .any(|s| stmt_forwards_param_to_array(s, param, known_arrays))
                })
        }
        Stmt::While { condition, body } => {
            expr_forwards_param_to_array(condition, param, known_arrays)
                || body
                    .iter()
                    .any(|s| stmt_forwards_param_to_array(s, param, known_arrays))
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } => body
            .iter()
            .any(|s| stmt_forwards_param_to_array(s, param, known_arrays)),
        Stmt::ForEach { iterable, body, .. } => {
            expr_forwards_param_to_array(iterable, param, known_arrays)
                || matches!(iterable, Expr::Identifier(id) if id == param)
                || body
                    .iter()
                    .any(|s| stmt_forwards_param_to_array(s, param, known_arrays))
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            expr_forwards_param_to_array(expr, param, known_arrays)
        }
        _ => false,
    }
}

fn find_param_forwarded_call(stmts: &[Stmt], param: &str, known_arrays: &HashSet<String>) -> bool {
    stmts
        .iter()
        .any(|s| stmt_forwards_param_to_array(s, param, known_arrays))
}

fn collect_array_vars_from_stmts(
    stmts: &[Stmt],
    fn_scope: Option<&str>,
    known_arrays: &mut HashSet<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
            } if type_ann.as_deref() == Some("array")
                || type_ann.as_ref().is_some_and(|t| t.ends_with("[]"))
                || expr_is_definitely_array(value, fn_scope, known_arrays) =>
            {
                if let Some(scope) = fn_scope {
                    known_arrays.insert(format!("{}:{}", scope, name));
                } else {
                    known_arrays.insert(name.clone());
                }
            }
            Stmt::Assign { name, value, .. }
                if expr_is_definitely_array(value, fn_scope, known_arrays) =>
            {
                if let Some(scope) = fn_scope {
                    known_arrays.insert(format!("{}:{}", scope, name));
                } else {
                    known_arrays.insert(name.clone());
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_array_vars_from_stmts(try_block, fn_scope, known_arrays);
                collect_array_vars_from_stmts(catch_block, fn_scope, known_arrays);
                if let Some(finally_block) = finally_block {
                    collect_array_vars_from_stmts(finally_block, fn_scope, known_arrays);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_array_vars_from_stmts(then_block, fn_scope, known_arrays);
                if let Some(else_stmts) = else_block {
                    collect_array_vars_from_stmts(else_stmts, fn_scope, known_arrays);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_array_vars_from_stmts(body, fn_scope, known_arrays);
            }
            Stmt::Function {
                name,
                params,
                param_types,
                body,
                ..
            } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                for (idx, param) in params.iter().enumerate() {
                    let is_arr_type =
                        param_types
                            .get(idx)
                            .and_then(|t| t.as_deref())
                            .is_some_and(|t| {
                                t == "..."
                                    || t.starts_with("...")
                                    || t == "array"
                                    || t.ends_with("[]")
                            });
                    if is_arr_type
                        || known_arrays.contains(&format!("fn_param_arr:{}:{}", name, idx))
                        // #101: qualified-spelled functions consult exact only.
                        || (is_simple_name(name)
                            && known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx)))
                    {
                        if is_arr_type {
                            known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                            // #101: bare markers only for simple names.
                            if is_simple_name(name) {
                                known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                            }
                        }
                        known_arrays.insert(format!("{}:{}", name, param));
                        if bare != name {
                            known_arrays.insert(format!("{}:{}", bare, param));
                        }
                    }
                }
                collect_array_vars_from_stmts(body, Some(name), known_arrays);
                // #101: no bare-scope scan. For simple names it would
                // duplicate the scan above (bare == name); for qualified
                // names its bare markers would be inherited by unrelated
                // same-bare callers.
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_array_vars_from_stmts(std::slice::from_ref(inner), fn_scope, known_arrays);
            }
            _ => {}
        }
    }
}

pub fn collect_known_array_vars(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_array_vars_with_index(program, &call_index)
}

pub fn collect_known_array_vars_with_index(
    program: &Program,
    call_index: &CallIndex,
) -> HashSet<String> {
    let mut known_arrays = HashSet::new();
    let mut funcs = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
    for stmt in &program.statements {
        let stmt = stmt.inner_stmt();
        if let Stmt::StructDef {
            name,
            fields,
            field_types,
            ..
        } = stmt
        {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (f, ft) in fields.iter().zip(field_types.iter()) {
                if let Some(t) = ft {
                    if t == "array" || t.ends_with("[]") {
                        known_arrays.insert(format!("struct_field_arr:{}.{}", name, f));
                        known_arrays.insert(format!("struct_field_arr:{}.{}", bare, f));
                        known_arrays.insert(format!("struct_field_arr:{}", f));
                    }
                }
            }
        }
    }
    for _ in 0..7 {
        let prev_len = known_arrays.len();
        collect_array_vars_from_stmts(&program.statements, None, &mut known_arrays);
        for (name, params, param_types, body) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);

            if stmts_return_array(body, Some(name), &known_arrays)
                || (bare != *name && stmts_return_array(body, Some(bare), &known_arrays))
            {
                known_arrays.insert(format!("fn_ret_arr:{}", name));
                // #101: bare markers only for simple names.
                if is_simple_name(name) {
                    known_arrays.insert(format!("fn_ret_arr:{}", bare));
                }
            }

            for (idx, param) in params.iter().enumerate() {
                if let Some(t) = param_types.get(idx).and_then(|t| t.as_deref()) {
                    if t == "..." || t.starts_with("...") || t == "array" || t.ends_with("[]") {
                        known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                        // #101: bare markers only for simple names.
                        if is_simple_name(name) {
                            known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                        }
                        continue;
                    }
                }

                if known_arrays.contains(&format!("fn_param_arr:{}:{}", name, idx))
                    // #101: qualified-spelled functions consult exact only.
                    || (is_simple_name(name)
                        && known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx)))
                {
                    continue;
                }

                if param_is_used_as_array(param, body) {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    if is_simple_name(name) {
                        known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    }
                    continue;
                }

                if find_param_forwarded_call(body, param, &known_arrays) {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    if is_simple_name(name) {
                        known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    }
                    continue;
                }

                let mut call_args = Vec::new();
                call_index.collect_all_call_args_scoped(name, bare, idx, &mut call_args);
                // alya-lang/alya#47: heap unless proven scalar. Requiring
                // ALL call args to be provably arrays dropped entry
                // retains when a single arg was merely unclassifiable
                // (e.g. a map index), causing use-after-free with
                // run-varying garbage. Over-retaining is safe (retain +
                // release are paired and the runtime guards non-heap
                // values); under-retaining is catastrophic. Proven
                // scalars still block the marking.
                if !call_args.is_empty()
                    && call_args.iter().any(|(caller_scope, arg)| {
                        expr_is_definitely_array(arg, *caller_scope, &known_arrays)
                    })
                    && call_args.iter().all(|(caller_scope, arg)| {
                        matches!(arg, Expr::Null)
                            || expr_is_definitely_array(arg, *caller_scope, &known_arrays)
                            || !expr_is_definitely_non_array(arg)
                    })
                {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    // #101: bare markers only for simple names.
                    if is_simple_name(name) {
                        known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    }
                }
            }
        }
        if known_arrays.len() == prev_len {
            break;
        }
    }
    known_arrays
}

pub fn infer_param_is_array_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_arrays: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    known_arrays.contains(&format!("fn_param_arr:{}:{}", func_name, param_idx))
        // #101: qualified-spelled callees consult exact markers only.
        || (is_simple_name(func_name)
            && known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, param_idx)))
}

pub fn infer_param_is_array(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_arrays = collect_known_array_vars(program);
    infer_param_is_array_with(func_name, param_idx, program, &known_arrays)
}

// ═══════════════════════════════════════════════════════════════
// Strict (must-) array facts for `is array` folding (alya-lang/alya#70).
//
// The may-fact above is existential: one definitely-array caller plus
// merely-unclassifiable rest marks the param (deliberate, alya-lang/alya#47:
// over-retaining is safe). Folding `is array` to a constant needs the
// universal claim instead: EVERY caller passes an array. In particular
// the generic survivor of call-site specialization serves exactly the
// unclassifiable callers, so clone-scope witnesses must never prove it.
// This fixpoint therefore admits only exact evidence — enforced
// annotations, single-assignment literal aliases, structural calls, and
// dynspec suffix codes — and never consults the may-set.
// ═══════════════════════════════════════════════════════════════

type FuncDef<'a> = (&'a str, &'a [String], &'a [Option<String>], &'a [Stmt]);

/// Resolves one identifier to its dynspec suffix kind inside a clone
/// scope (`None` for generic scopes). Exact: clones only run through
/// classified call sites (same contract as the codegen decoder).
fn spec_scope_kind(scope: &str, param: &str, funcs: &[FuncDef]) -> Option<char> {
    if let Some((_, params, param_types, _)) = funcs.iter().find(|(n, _, _, _)| *n == scope) {
        if let Some(k) = spec_open_param_kind(scope, param, params, param_types) {
            return Some(k);
        }
    }
    // Clone bodies are verbatim copies: fall back to the origin
    // definition (and its bare name) when the clone itself is missing.
    let origin = scope.split("__spk__").next().unwrap_or(scope);
    if origin != scope {
        for cand in [origin, origin.rsplit("::").next().unwrap_or(origin)] {
            if let Some((_, params, param_types, _)) = funcs.iter().find(|(n, _, _, _)| *n == cand)
            {
                if let Some(k) = spec_open_param_kind(scope, param, params, param_types) {
                    return Some(k);
                }
            }
        }
    }
    None
}

/// Exact counterpart of `expr_is_definitely_array`: identifiers resolve
/// only through strict facts (never the may-set) or the dynspec suffix
/// codes. `null` is not an array and blocks strictness.
fn expr_is_definitely_array_strict(
    expr: &Expr,
    fn_scope: Option<&str>,
    strict: &HashSet<String>,
    funcs: &[FuncDef],
) -> bool {
    if expr_is_definitely_array(expr, fn_scope, strict) {
        return true;
    }
    if let (Expr::Identifier(name), Some(scope)) = (expr, fn_scope) {
        return spec_scope_kind(scope, name, funcs) == Some('a');
    }
    false
}

fn strict_scope_counts<'a>(
    fn_scope: Option<&str>,
    counts: &'a HashMap<String, HashMap<String, usize>>,
) -> Option<&'a HashMap<String, usize>> {
    let scope = fn_scope.unwrap_or("");
    if let Some(c) = counts.get(scope) {
        return Some(c);
    }
    // Clone bodies are verbatim copies: their assignment counts match
    // the origin's.
    let origin = scope.split("__spk__").next().unwrap_or(scope);
    counts.get(origin)
}

fn collect_strict_array_vars_from_stmts(
    stmts: &[Stmt],
    fn_scope: Option<&str>,
    counts: &HashMap<String, HashMap<String, usize>>,
    strict: &mut HashSet<String>,
    funcs: &[FuncDef],
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => {
                // Single-assignment only: a later reassignment could swap
                // in a non-array (the may-rule has no such guard).
                let single = strict_scope_counts(fn_scope, counts)
                    .and_then(|c| c.get(name))
                    .is_some_and(|n| *n == 1);
                if single
                    && (type_ann.as_deref() == Some("array")
                        || type_ann.as_ref().is_some_and(|t| t.ends_with("[]"))
                        || expr_is_definitely_array_strict(value, fn_scope, strict, funcs))
                {
                    if let Some(scope) = fn_scope {
                        strict.insert(format!("{}:{}", scope, name));
                    } else {
                        strict.insert(name.clone());
                    }
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_strict_array_vars_from_stmts(try_block, fn_scope, counts, strict, funcs);
                collect_strict_array_vars_from_stmts(catch_block, fn_scope, counts, strict, funcs);
                if let Some(fb) = finally_block {
                    collect_strict_array_vars_from_stmts(fb, fn_scope, counts, strict, funcs);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_strict_array_vars_from_stmts(then_block, fn_scope, counts, strict, funcs);
                if let Some(eb) = else_block {
                    collect_strict_array_vars_from_stmts(eb, fn_scope, counts, strict, funcs);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_strict_array_vars_from_stmts(body, fn_scope, counts, strict, funcs);
            }
            Stmt::Function {
                name,
                params,
                param_types,
                body,
                ..
            } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                for (idx, param) in params.iter().enumerate() {
                    // Explicit enforced annotations are exact (the type
                    // checker rejects mismatched arguments and lets).
                    let annotated =
                        param_types
                            .get(idx)
                            .and_then(|t| t.as_deref())
                            .is_some_and(|t| {
                                t == "..."
                                    || t.starts_with("...")
                                    || t == "array"
                                    || t.ends_with("[]")
                            });
                    if annotated
                        || strict.contains(&format!("fn_param_arr:{}:{}", name, idx))
                        // #101: qualified-spelled functions consult exact only.
                        || (is_simple_name(name)
                            && strict.contains(&format!("fn_param_arr:{}:{}", bare, idx)))
                    {
                        strict.insert(format!("{}:{}", name, param));
                        // Bare-sharing crosses module qualification only:
                        // clones (`__spk__`) are different functions, and
                        // sharing their proofs with the generic is exactly
                        // the alya-lang/alya#70 hole.
                        if !name.contains("__spk__") && bare != name {
                            strict.insert(format!("{}:{}", bare, param));
                        }
                    }
                }
                collect_strict_array_vars_from_stmts(body, Some(name), counts, strict, funcs);
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_strict_array_vars_from_stmts(
                    std::slice::from_ref(inner),
                    fn_scope,
                    counts,
                    strict,
                    funcs,
                );
            }
            _ => {}
        }
    }
}

pub fn collect_known_array_vars_strict_with_index(
    program: &Program,
    call_index: &CallIndex,
) -> HashSet<String> {
    let mut strict = HashSet::new();
    let mut funcs: Vec<FuncDef> = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
    let mut counts: HashMap<String, HashMap<String, usize>> = HashMap::new();
    {
        let mut top = HashMap::new();
        count_assignments(&program.statements, &mut top);
        counts.insert(String::new(), top);
    }
    for (name, _, _, body) in &funcs {
        let mut c = HashMap::new();
        count_assignments(body, &mut c);
        counts.insert(name.to_string(), c);
    }
    // Seed exact per-clone param kinds from the dynspec suffix
    // contract (clones only run through classified call sites).
    for (name, params, param_types, _) in &funcs {
        if let Some(codes) = dynspec_codes(name) {
            let mut open_idx = 0usize;
            for (p, t) in params.iter().zip(param_types.iter()) {
                if t.is_none() && p != "self" {
                    if codes.get(open_idx) == Some(&'a') {
                        strict.insert(format!("{}:{}", name, p));
                    }
                    open_idx += 1;
                }
            }
        }
    }
    for _ in 0..7 {
        let prev_len = strict.len();
        collect_strict_array_vars_from_stmts(
            &program.statements,
            None,
            &counts,
            &mut strict,
            &funcs,
        );
        for (name, params, param_types, _) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (idx, param) in params.iter().enumerate() {
                if param_types
                    .get(idx)
                    .and_then(|t| t.as_deref())
                    .is_some_and(|t| {
                        t == "..." || t.starts_with("...") || t == "array" || t.ends_with("[]")
                    })
                {
                    strict.insert(format!("fn_param_arr:{}:{}", name, idx));
                    // #101: bare markers only for simple names.
                    if is_simple_name(name) {
                        strict.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    }
                    strict.insert(format!("{}:{}", name, param));
                    if !name.contains("__spk__") && bare != *name {
                        strict.insert(format!("{}:{}", bare, param));
                    }
                    continue;
                }
                if strict.contains(&format!("fn_param_arr:{}:{}", name, idx))
                    // #101: qualified-spelled functions consult exact only.
                    || (is_simple_name(name)
                        && strict.contains(&format!("fn_param_arr:{}:{}", bare, idx)))
                {
                    continue;
                }
                let mut call_args = Vec::new();
                call_index.collect_all_call_args_scoped(name, bare, idx, &mut call_args);
                // Universal (not existential): every caller must pass an
                // array. Unknowns and null block the proof — they are the
                // generic survivor's callers.
                if !call_args.is_empty()
                    && call_args.iter().all(|(caller_scope, arg)| {
                        expr_is_definitely_array_strict(arg, *caller_scope, &strict, &funcs)
                    })
                {
                    strict.insert(format!("fn_param_arr:{}:{}", name, idx));
                    // #101: bare markers only for simple names.
                    if is_simple_name(name) {
                        strict.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    }
                }
            }
        }
        if strict.len() == prev_len {
            break;
        }
    }
    strict
}

pub fn collect_known_array_vars_strict(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_array_vars_strict_with_index(program, &call_index)
}

pub fn infer_param_is_array_strict_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    strict: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    strict.contains(&format!("fn_param_arr:{}:{}", func_name, param_idx))
        // #101: qualified-spelled callees consult exact markers only.
        || (is_simple_name(func_name)
            && strict.contains(&format!("fn_param_arr:{}:{}", bare, param_idx)))
}

pub fn infer_param_is_array_strict(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let strict = collect_known_array_vars_strict(program);
    infer_param_is_array_strict_with(func_name, param_idx, program, &strict)
}
