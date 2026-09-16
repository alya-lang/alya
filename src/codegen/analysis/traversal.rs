use crate::ast::*;

pub fn find_call_arg<'a>(stmt: &'a Stmt, func_name: &str, param_idx: usize) -> Option<&'a Expr> {
    match stmt {
        Stmt::Expr(expr) | Stmt::Say(expr) => find_call_arg_in_expr(expr, func_name, param_idx),
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            find_call_arg_in_expr(value, func_name, param_idx)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            if let Some(arg) = find_call_arg_in_expr(condition, func_name, param_idx) {
                return Some(arg);
            }
            for s in then_block {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                        return Some(arg);
                    }
                }
            }
            None
        }
        Stmt::While { condition, body } => {
            if let Some(arg) = find_call_arg_in_expr(condition, func_name, param_idx) {
                return Some(arg);
            }
            for s in body {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            for s in body {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Stmt::Throw(opt_expr) => opt_expr
            .as_ref()
            .and_then(|expr| find_call_arg_in_expr(expr, func_name, param_idx)),
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            for s in catch_block {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            if let Some(finally_block) = finally_block {
                for s in finally_block {
                    if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                        return Some(arg);
                    }
                }
            }
            None
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => find_call_arg_in_expr(array, func_name, param_idx)
            .or_else(|| find_call_arg_in_expr(index, func_name, param_idx))
            .or_else(|| find_call_arg_in_expr(value, func_name, param_idx)),
        Stmt::FieldAssign { object, value, .. } => {
            find_call_arg_in_expr(object, func_name, param_idx)
                .or_else(|| find_call_arg_in_expr(value, func_name, param_idx))
        }
        Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                find_call_arg_in_expr(expr, func_name, param_idx)
            } else {
                None
            }
        }
        Stmt::Function { body, .. } => {
            for s in body {
                if let Some(arg) = find_call_arg(s, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        _ => None,
    }
}

pub fn find_call_arg_in_expr<'a>(
    expr: &'a Expr,
    func_name: &str,
    param_idx: usize,
) -> Option<&'a Expr> {
    match expr {
        Expr::Call { name, args } => {
            if name == func_name {
                if let Some(arg) = args.get(param_idx) {
                    return Some(arg);
                }
            }
            for arg in args {
                if let Some(res) = find_call_arg_in_expr(arg, func_name, param_idx) {
                    return Some(res);
                }
            }
            None
        }
        Expr::Binary { left, right, .. } => find_call_arg_in_expr(left, func_name, param_idx)
            .or_else(|| find_call_arg_in_expr(right, func_name, param_idx)),
        Expr::Unary { expr, .. } => find_call_arg_in_expr(expr, func_name, param_idx),
        Expr::Array(elements) => {
            for elem in elements {
                if let Some(arg) = find_call_arg_in_expr(elem, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::Index { array, index } => find_call_arg_in_expr(array, func_name, param_idx)
            .or_else(|| find_call_arg_in_expr(index, func_name, param_idx)),
        Expr::FieldAccess { object, .. } => find_call_arg_in_expr(object, func_name, param_idx),
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                if let Some(arg) = find_call_arg_in_expr(val, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                if let Some(arg) = find_call_arg_in_expr(k, func_name, param_idx) {
                    return Some(arg);
                }
                if let Some(arg) = find_call_arg_in_expr(v, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                if let Some(arg) = find_call_arg_in_expr(part, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => find_call_arg_in_expr(condition, func_name, param_idx)
            .or_else(|| find_call_arg_in_expr(then_branch, func_name, param_idx))
            .or_else(|| find_call_arg_in_expr(else_branch, func_name, param_idx)),
        Expr::NullCoalesce { value, default } => find_call_arg_in_expr(value, func_name, param_idx)
            .or_else(|| find_call_arg_in_expr(default, func_name, param_idx)),
        _ => None,
    }
}

pub fn collect_all_call_args<'a>(
    stmts: &'a [Stmt],
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    args: &mut Vec<&'a Expr>,
) {
    for s in stmts {
        collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
    }
}

pub fn collect_call_args_in_stmt<'a>(
    stmt: &'a Stmt,
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    args: &mut Vec<&'a Expr>,
) {
    match stmt {
        Stmt::Expr(expr) | Stmt::Say(expr) => {
            collect_call_args_in_expr(expr, func_name, bare_name, param_idx, args);
        }
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            collect_call_args_in_expr(value, func_name, bare_name, param_idx, args);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_call_args_in_expr(condition, func_name, bare_name, param_idx, args);
            for s in then_block {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_call_args_in_expr(condition, func_name, bare_name, param_idx, args);
            for s in body {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            for s in body {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
        }
        Stmt::Throw(opt_expr) | Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_call_args_in_expr(expr, func_name, bare_name, param_idx, args);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
            for s in catch_block {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
            if let Some(finally_block) = finally_block {
                for s in finally_block {
                    collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
                }
            }
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_call_args_in_expr(array, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(index, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(value, func_name, bare_name, param_idx, args);
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_call_args_in_expr(object, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(value, func_name, bare_name, param_idx, args);
        }
        Stmt::Function { body, .. } => {
            for s in body {
                collect_call_args_in_stmt(s, func_name, bare_name, param_idx, args);
            }
        }
        _ => {}
    }
}

pub fn collect_call_args_in_expr<'a>(
    expr: &'a Expr,
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    args: &mut Vec<&'a Expr>,
) {
    match expr {
        Expr::Call {
            name,
            args: call_args,
        } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if name == func_name || bare == bare_name || name == bare_name || bare == func_name {
                if let Some(arg) = call_args.get(param_idx) {
                    args.push(arg);
                }
            }
            for arg in call_args {
                collect_call_args_in_expr(arg, func_name, bare_name, param_idx, args);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_call_args_in_expr(left, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(right, func_name, bare_name, param_idx, args);
        }
        Expr::Unary { expr, .. } => {
            collect_call_args_in_expr(expr, func_name, bare_name, param_idx, args);
        }
        Expr::Array(elements) => {
            for elem in elements {
                collect_call_args_in_expr(elem, func_name, bare_name, param_idx, args);
            }
        }
        Expr::Index { array, index } => {
            collect_call_args_in_expr(array, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(index, func_name, bare_name, param_idx, args);
        }
        Expr::FieldAccess { object, .. } => {
            collect_call_args_in_expr(object, func_name, bare_name, param_idx, args);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_call_args_in_expr(val, func_name, bare_name, param_idx, args);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_call_args_in_expr(k, func_name, bare_name, param_idx, args);
                collect_call_args_in_expr(v, func_name, bare_name, param_idx, args);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                collect_call_args_in_expr(part, func_name, bare_name, param_idx, args);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_call_args_in_expr(condition, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(then_branch, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(else_branch, func_name, bare_name, param_idx, args);
        }
        Expr::NullCoalesce { value, default } => {
            collect_call_args_in_expr(value, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(default, func_name, bare_name, param_idx, args);
        }
        _ => {}
    }
}

pub fn collect_all_call_args_scoped<'a>(
    stmts: &'a [Stmt],
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    args: &mut Vec<(Option<&'a str>, &'a Expr)>,
) {
    for s in stmts {
        collect_call_args_in_stmt_scoped(s, func_name, bare_name, param_idx, None, args);
    }
}

pub fn collect_call_args_in_stmt_scoped<'a>(
    stmt: &'a Stmt,
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    current_scope: Option<&'a str>,
    args: &mut Vec<(Option<&'a str>, &'a Expr)>,
) {
    match stmt {
        Stmt::Expr(expr) | Stmt::Say(expr) => {
            collect_call_args_in_expr_scoped(
                expr,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            collect_call_args_in_expr_scoped(
                value,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_call_args_in_expr_scoped(
                condition,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            for s in then_block {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    collect_call_args_in_stmt_scoped(
                        s,
                        func_name,
                        bare_name,
                        param_idx,
                        current_scope,
                        args,
                    );
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_call_args_in_expr_scoped(
                condition,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            for s in body {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            for s in body {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Stmt::Throw(opt_expr) | Stmt::Return(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_call_args_in_expr_scoped(
                    expr,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
            for s in catch_block {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
            if let Some(finally_block) = finally_block {
                for s in finally_block {
                    collect_call_args_in_stmt_scoped(
                        s,
                        func_name,
                        bare_name,
                        param_idx,
                        current_scope,
                        args,
                    );
                }
            }
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_call_args_in_expr_scoped(
                array,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                index,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                value,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_call_args_in_expr_scoped(
                object,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                value,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Stmt::Function { name, body, .. } => {
            for s in body {
                collect_call_args_in_stmt_scoped(
                    s,
                    func_name,
                    bare_name,
                    param_idx,
                    Some(name.as_str()),
                    args,
                );
            }
        }
        _ => {}
    }
}

pub fn collect_call_args_in_expr_scoped<'a>(
    expr: &'a Expr,
    func_name: &str,
    bare_name: &str,
    param_idx: usize,
    current_scope: Option<&'a str>,
    args: &mut Vec<(Option<&'a str>, &'a Expr)>,
) {
    match expr {
        Expr::Call {
            name,
            args: call_args,
        } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if name == func_name || bare == bare_name || name == bare_name || bare == func_name {
                if let Some(arg) = call_args.get(param_idx) {
                    args.push((current_scope, arg));
                }
            }
            for arg in call_args {
                collect_call_args_in_expr_scoped(
                    arg,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_call_args_in_expr_scoped(
                left,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                right,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Expr::Unary { expr, .. } => {
            collect_call_args_in_expr_scoped(
                expr,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Expr::Array(elements) => {
            for elem in elements {
                collect_call_args_in_expr_scoped(
                    elem,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Expr::Index { array, index } => {
            collect_call_args_in_expr_scoped(
                array,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                index,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Expr::FieldAccess { object, .. } => {
            collect_call_args_in_expr_scoped(
                object,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                collect_call_args_in_expr_scoped(
                    val,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_call_args_in_expr_scoped(
                    k,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
                collect_call_args_in_expr_scoped(
                    v,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                collect_call_args_in_expr_scoped(
                    part,
                    func_name,
                    bare_name,
                    param_idx,
                    current_scope,
                    args,
                );
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_call_args_in_expr_scoped(
                condition,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                then_branch,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                else_branch,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        Expr::NullCoalesce { value, default } => {
            collect_call_args_in_expr_scoped(
                value,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
            collect_call_args_in_expr_scoped(
                default,
                func_name,
                bare_name,
                param_idx,
                current_scope,
                args,
            );
        }
        _ => {}
    }
}
