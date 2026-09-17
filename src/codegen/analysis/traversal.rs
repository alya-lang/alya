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
        Stmt::Defer(inner) => find_call_arg(inner, func_name, param_idx),
        _ => None,
    }
}

pub fn find_call_arg_in_expr<'a>(
    expr: &'a Expr,
    func_name: &str,
    param_idx: usize,
) -> Option<&'a Expr> {
    match expr {
        Expr::Call { name, args } | Expr::OptionalCall { callee: name, args } => {
            if name == func_name && param_idx < args.len() {
                return Some(&args[param_idx]);
            }
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            let target_bare = func_name.rsplit("::").next().unwrap_or(func_name);
            let target_bare = target_bare.rsplit("__").next().unwrap_or(target_bare);
            if bare == target_bare && param_idx < args.len() {
                return Some(&args[param_idx]);
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
        Expr::InterpolatedString(parts) => {
            for part in parts {
                if let Some(arg) = find_call_arg_in_expr(part, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::Array(elements) => {
            for elem in elements {
                if let Some(arg) = find_call_arg_in_expr(elem, func_name, param_idx) {
                    return Some(arg);
                }
            }
            None
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            find_call_arg_in_expr(array, func_name, param_idx)
                .or_else(|| find_call_arg_in_expr(index, func_name, param_idx))
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            find_call_arg_in_expr(object, func_name, param_idx)
        }
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
        Stmt::Defer(inner) => {
            collect_call_args_in_stmt(inner, func_name, bare_name, param_idx, args);
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
        }
        | Expr::OptionalCall {
            callee: name,
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
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_call_args_in_expr(array, func_name, bare_name, param_idx, args);
            collect_call_args_in_expr(index, func_name, bare_name, param_idx, args);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
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
        Stmt::Defer(inner) => {
            collect_call_args_in_stmt_scoped(
                inner,
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
        }
        | Expr::OptionalCall {
            callee: name,
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
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
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
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
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

pub type CallSite<'a> = (Option<&'a str>, &'a str, &'a [Expr]);

#[derive(Debug, Default, Clone)]
pub struct CallIndex<'a> {
    /// Maps bare name -> list of (caller_scope, full_call_name, &'a [Expr])
    by_bare: std::collections::HashMap<String, Vec<CallSite<'a>>>,
}

impl<'a> CallIndex<'a> {
    pub fn build(stmts: &'a [Stmt]) -> Self {
        let mut index = Self::default();
        for s in stmts {
            index.collect_stmt(s, None);
        }
        index
    }

    fn record_call(&mut self, name: &'a str, args: &'a [Expr], current_scope: Option<&'a str>) {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        self.by_bare
            .entry(bare.to_string())
            .or_default()
            .push((current_scope, name, args));
    }

    fn collect_stmt(&mut self, stmt: &'a Stmt, current_scope: Option<&'a str>) {
        match stmt {
            Stmt::Expr(expr) | Stmt::Say(expr) => self.collect_expr(expr, current_scope),
            Stmt::Let { value, .. } | Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
                self.collect_expr(value, current_scope);
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.collect_expr(condition, current_scope);
                for s in then_block {
                    self.collect_stmt(s, current_scope);
                }
                if let Some(else_stmts) = else_block {
                    for s in else_stmts {
                        self.collect_stmt(s, current_scope);
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.collect_expr(condition, current_scope);
                for s in body {
                    self.collect_stmt(s, current_scope);
                }
            }
            Stmt::Repeat { body } | Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                for s in body {
                    self.collect_stmt(s, current_scope);
                }
            }
            Stmt::Throw(opt_expr) | Stmt::Return(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.collect_expr(expr, current_scope);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                for s in try_block {
                    self.collect_stmt(s, current_scope);
                }
                for s in catch_block {
                    self.collect_stmt(s, current_scope);
                }
                if let Some(finally_block) = finally_block {
                    for s in finally_block {
                        self.collect_stmt(s, current_scope);
                    }
                }
            }
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                self.collect_expr(array, current_scope);
                self.collect_expr(index, current_scope);
                self.collect_expr(value, current_scope);
            }
            Stmt::FieldAssign { object, value, .. } => {
                self.collect_expr(object, current_scope);
                self.collect_expr(value, current_scope);
            }
            Stmt::Function { name, body, .. } => {
                for s in body {
                    self.collect_stmt(s, Some(name.as_str()));
                }
            }
            Stmt::Defer(inner) => self.collect_stmt(inner, current_scope),
            Stmt::Pub(inner) => self.collect_stmt(inner, current_scope),
            _ => {}
        }
    }

    fn collect_expr(&mut self, expr: &'a Expr, current_scope: Option<&'a str>) {
        match expr {
            Expr::Call { name, args } => {
                self.record_call(name.as_str(), args.as_slice(), current_scope);
                for arg in args {
                    self.collect_expr(arg, current_scope);
                }
            }
            Expr::OptionalCall { callee, args } => {
                self.record_call(callee.as_str(), args.as_slice(), current_scope);
                for arg in args {
                    self.collect_expr(arg, current_scope);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.collect_expr(left, current_scope);
                self.collect_expr(right, current_scope);
            }
            Expr::Unary { expr, .. } => self.collect_expr(expr, current_scope),
            Expr::Array(elements) => {
                for elem in elements {
                    self.collect_expr(elem, current_scope);
                }
            }
            Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
                self.collect_expr(array, current_scope);
                self.collect_expr(index, current_scope);
            }
            Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
                self.collect_expr(object, current_scope);
            }
            Expr::StructInit { fields, .. } => {
                for (_, val) in fields {
                    self.collect_expr(val, current_scope);
                }
            }
            Expr::Map(entries) => {
                for (k, v) in entries {
                    self.collect_expr(k, current_scope);
                    self.collect_expr(v, current_scope);
                }
            }
            Expr::InterpolatedString(parts) => {
                for part in parts {
                    self.collect_expr(part, current_scope);
                }
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                self.collect_expr(condition, current_scope);
                self.collect_expr(then_branch, current_scope);
                self.collect_expr(else_branch, current_scope);
            }
            Expr::NullCoalesce { value, default } => {
                self.collect_expr(value, current_scope);
                self.collect_expr(default, current_scope);
            }
            Expr::TypeCheck { expr, .. } | Expr::Cast { expr, .. } => {
                self.collect_expr(expr, current_scope)
            }
            _ => {}
        }
    }

    pub fn collect_all_call_args(
        &self,
        func_name: &str,
        bare_name: &str,
        param_idx: usize,
        args: &mut Vec<&'a Expr>,
    ) {
        if let Some(list) = self.by_bare.get(bare_name) {
            for (_, call_name, call_args) in list {
                let call_bare = bare_name;
                if *call_name == func_name
                    || call_bare == bare_name
                    || *call_name == bare_name
                    || call_bare == func_name
                {
                    if let Some(arg) = call_args.get(param_idx) {
                        args.push(arg);
                    }
                }
            }
        }
    }

    pub fn collect_all_call_args_scoped(
        &self,
        func_name: &str,
        bare_name: &str,
        param_idx: usize,
        args: &mut Vec<(Option<&'a str>, &'a Expr)>,
    ) {
        if let Some(list) = self.by_bare.get(bare_name) {
            for (scope, call_name, call_args) in list {
                let call_bare = bare_name;
                if *call_name == func_name
                    || call_bare == bare_name
                    || *call_name == bare_name
                    || call_bare == func_name
                {
                    if let Some(arg) = call_args.get(param_idx) {
                        args.push((*scope, arg));
                    }
                }
            }
        }
    }

    pub fn has_matching_call_arg<F>(
        &self,
        func_name: &str,
        bare_name: &str,
        param_idx: usize,
        mut predicate: F,
    ) -> bool
    where
        F: FnMut(&'a Expr) -> bool,
    {
        if let Some(list) = self.by_bare.get(bare_name) {
            for (_, call_name, call_args) in list {
                let call_bare = bare_name;
                if *call_name == func_name
                    || call_bare == bare_name
                    || *call_name == bare_name
                    || call_bare == func_name
                {
                    if let Some(arg) = call_args.get(param_idx) {
                        if predicate(arg) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
