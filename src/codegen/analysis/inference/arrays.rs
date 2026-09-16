use super::common::collect_function_defs;
use crate::ast::*;
use crate::codegen::analysis::traversal::collect_all_call_args_scoped;
use std::collections::HashSet;

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
                || known_arrays.contains(&format!("fn_ret_arr:{}", bare))
        }
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
                if id == param && matches!(index, Expr::Number(_)) {
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
                        || known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx)))
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
            Stmt::Let { name, value, .. } | Stmt::Assign { name, value, .. }
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
                    let is_rest = param_types.get(idx).and_then(|t| t.as_deref()) == Some("...");
                    if is_rest
                        || known_arrays.contains(&format!("fn_param_arr:{}:{}", name, idx))
                        || known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx))
                    {
                        if is_rest {
                            known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                            known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                        }
                        known_arrays.insert(format!("{}:{}", name, param));
                        if bare != name {
                            known_arrays.insert(format!("{}:{}", bare, param));
                        }
                    }
                }
                collect_array_vars_from_stmts(body, Some(name), known_arrays);
                if bare != name {
                    collect_array_vars_from_stmts(body, Some(bare), known_arrays);
                }
            }
            _ => {}
        }
    }
}

pub fn collect_known_array_vars(program: &Program) -> HashSet<String> {
    let mut known_arrays = HashSet::new();
    let mut funcs = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
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
                known_arrays.insert(format!("fn_ret_arr:{}", bare));
            }

            for (idx, param) in params.iter().enumerate() {
                if param_types.get(idx).and_then(|t| t.as_deref()) == Some("...") {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    continue;
                }

                if known_arrays.contains(&format!("fn_param_arr:{}:{}", name, idx))
                    || known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, idx))
                {
                    continue;
                }

                if param_is_used_as_array(param, body) {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    continue;
                }

                if find_param_forwarded_call(body, param, &known_arrays) {
                    known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
                    known_arrays.insert(format!("fn_param_arr:{}:{}", bare, idx));
                    continue;
                }

                let mut call_args = Vec::new();
                collect_all_call_args_scoped(&program.statements, name, bare, idx, &mut call_args);
                if !call_args.is_empty() {
                    let has_def_arr = call_args.iter().any(|(caller_scope, arg)| {
                        expr_is_definitely_array(arg, *caller_scope, &known_arrays)
                    });
                    let has_conflict = call_args
                        .iter()
                        .any(|(_, arg)| expr_is_definitely_non_array(arg));
                    if has_def_arr && !has_conflict {
                        known_arrays.insert(format!("fn_param_arr:{}:{}", name, idx));
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
        || known_arrays.contains(&format!("fn_param_arr:{}:{}", bare, param_idx))
}

pub fn infer_param_is_array(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_arrays = collect_known_array_vars(program);
    infer_param_is_array_with(func_name, param_idx, program, &known_arrays)
}
