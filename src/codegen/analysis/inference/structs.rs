use crate::ast::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone)]
pub struct StructInference {
    /// Maps function name -> returned struct type (e.g. "buffer_new" -> "ByteBuffer")
    pub fn_returns: HashMap<String, String>,
    /// Maps (function_name, param_idx) -> struct type
    pub fn_params: HashMap<(String, usize), String>,
    /// Maps variable name -> struct type
    pub var_types: HashMap<String, String>,
    /// Maps (struct_name, field_name) -> struct type
    pub field_types: HashMap<(String, String), String>,
    /// Tracks (function_name, param_idx) that received multiple conflicting struct types (polymorphic)
    pub conflicted_params: HashSet<(String, usize)>,
    pub conflicted_returns: HashSet<String>,
    pub struct_names: HashSet<String>,
    pub fn_names: HashSet<String>,
}

pub(crate) fn resolve_func_bare<'a>(name: &'a str, struct_names: &HashSet<String>) -> &'a str {
    let ns_bare = name.rsplit("::").next().unwrap_or(name);
    if let Some((prefix, _)) = ns_bare.split_once("__") {
        if struct_names.contains(prefix) {
            ns_bare
        } else {
            ns_bare.rsplit("__").next().unwrap_or(ns_bare)
        }
    } else {
        ns_bare
    }
}

impl StructInference {
    pub fn analyze(program: &Program) -> Self {
        let mut inf = StructInference::default();
        let mut struct_names = HashSet::new();
        for s in &program.statements {
            if let Stmt::StructDef { name, .. } = s.inner_stmt() {
                struct_names.insert(name.clone());
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if bare != name {
                    struct_names.insert(bare.to_string());
                }
            }
        }

        let mut fn_names = HashSet::new();
        for s in &program.statements {
            if let Stmt::Function { name, .. } = s.inner_stmt() {
                fn_names.insert(name.clone());
                let bare = resolve_func_bare(name, &struct_names);
                fn_names.insert(bare.to_string());
            }
        }

        for s in &program.statements {
            if let Stmt::StructDef {
                name,
                fields,
                field_types,
                ..
            } = s.inner_stmt()
            {
                for (f, ft) in fields.iter().zip(field_types.iter()) {
                    if let Some(t) = ft {
                        if struct_names.contains(t) {
                            inf.field_types.insert((name.clone(), f.clone()), t.clone());
                            let bare = name.rsplit("::").next().unwrap_or(name);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            if bare != name {
                                inf.field_types
                                    .insert((bare.to_string(), f.clone()), t.clone());
                            }
                        }
                    }
                }
            }
        }

        for _ in 0..6 {
            let prev_len = inf.fn_returns.len()
                + inf.fn_params.len()
                + inf.var_types.len()
                + inf.field_types.len();
            inf.scan_stmts(&program.statements, None, &struct_names, &fn_names);
            if inf.fn_returns.len()
                + inf.fn_params.len()
                + inf.var_types.len()
                + inf.field_types.len()
                == prev_len
            {
                break;
            }
        }

        inf.struct_names = struct_names;
        inf.fn_names = fn_names;
        inf
    }

    pub fn expr_struct_type(
        &self,
        expr: &Expr,
        current_fn: Option<&str>,
        struct_names: &HashSet<String>,
    ) -> Option<String> {
        match expr {
            Expr::StructInit { name, .. } => Some(name.clone()),
            Expr::Call { name, args } => {
                if let Some(first_arg) = args.first() {
                    if let Some(st) = self.expr_struct_type(first_arg, current_fn, struct_names) {
                        let bare_st = resolve_func_bare(&st, struct_names);
                        let candidate1 = format!("{}__{}", st, name);
                        let candidate2 = format!("{}__{}", bare_st, name);
                        if let Some(ret_st) = self
                            .fn_returns
                            .get(&candidate1)
                            .or_else(|| self.fn_returns.get(&candidate2))
                        {
                            return Some(ret_st.clone());
                        }
                    }
                }
                let bare = resolve_func_bare(name, struct_names);
                if struct_names.contains(name) {
                    Some(name.clone())
                } else if struct_names.contains(bare) {
                    Some(bare.to_string())
                } else if let Some(st) = self.fn_returns.get(name) {
                    Some(st.clone())
                } else {
                    self.fn_returns.get(bare).cloned()
                }
            }
            Expr::Identifier(vname) => {
                let bare = resolve_func_bare(vname, struct_names);
                if struct_names.contains(vname) {
                    return Some(vname.clone());
                } else if struct_names.contains(bare) {
                    return Some(bare.to_string());
                }
                if let Some(fn_name) = current_fn {
                    if let Some(st) = self.var_types.get(&format!("{}::{}", fn_name, vname)) {
                        return Some(st.clone());
                    }
                    let bare_fn = resolve_func_bare(fn_name, struct_names);
                    if let Some(st) = self.var_types.get(&format!("{}::{}", bare_fn, vname)) {
                        return Some(st.clone());
                    }
                }
                self.var_types.get(vname).cloned()
            }
            Expr::Array(elems) => elems
                .first()
                .and_then(|e| self.expr_struct_type(e, current_fn, struct_names)),
            Expr::FieldAccess { object, field } => {
                if let Some(parent_st) = self.expr_struct_type(object, current_fn, struct_names) {
                    let bare = resolve_func_bare(&parent_st, struct_names);
                    self.field_types
                        .get(&(parent_st.clone(), field.clone()))
                        .or_else(|| self.field_types.get(&(bare.to_string(), field.clone())))
                        .cloned()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn scan_stmts(
        &mut self,
        stmts: &[Stmt],
        current_fn: Option<&str>,
        struct_names: &HashSet<String>,
        fn_names: &HashSet<String>,
    ) {
        for s in stmts {
            match s {
                Stmt::Function {
                    name,
                    params,
                    param_types,
                    return_type,
                    body,
                    ..
                } => {
                    let bare = resolve_func_bare(name, struct_names);

                    if let Some(ref rt) = return_type {
                        let rt_bare = resolve_func_bare(rt, struct_names);
                        if rt == "any" || rt == "void" {
                            self.conflicted_returns.insert(name.to_string());
                            if bare != name {
                                self.conflicted_returns.insert(bare.to_string());
                            }
                        } else if struct_names.contains(rt) {
                            self.fn_returns.insert(name.to_string(), rt.clone());
                            if bare != name {
                                self.fn_returns.insert(bare.to_string(), rt.clone());
                            }
                        } else if struct_names.contains(rt_bare) {
                            self.fn_returns
                                .insert(name.to_string(), rt_bare.to_string());
                            if bare != name {
                                self.fn_returns
                                    .insert(bare.to_string(), rt_bare.to_string());
                            }
                        }
                    }

                    if let Some(first_p) = params.first() {
                        let parts: Vec<&str> = bare.split("__").collect();
                        if parts.len() >= 2 {
                            for type_name in &parts[..parts.len() - 1] {
                                if struct_names.contains(*type_name) {
                                    self.fn_params
                                        .insert((name.clone(), 0), type_name.to_string());
                                    if bare != name {
                                        self.fn_params
                                            .insert((bare.to_string(), 0), type_name.to_string());
                                    }
                                    self.var_types.insert(
                                        format!("{}::{}", name, first_p),
                                        type_name.to_string(),
                                    );
                                    if bare != name {
                                        self.var_types.insert(
                                            format!("{}::{}", bare, first_p),
                                            type_name.to_string(),
                                        );
                                    }
                                    break;
                                }
                            }
                        }
                    }

                    for (i, p) in params.iter().enumerate() {
                        let st = if let Some(Some(t)) = param_types.get(i) {
                            let b = resolve_func_bare(t, struct_names);
                            if struct_names.contains(t) {
                                Some(t.clone())
                            } else if struct_names.contains(b) {
                                Some(b.to_string())
                            } else {
                                None
                            }
                        } else {
                            self.fn_params
                                .get(&(name.clone(), i))
                                .or_else(|| self.fn_params.get(&(bare.to_string(), i)))
                                .cloned()
                        };
                        if let Some(st) = st {
                            self.fn_params.insert((name.clone(), i), st.clone());
                            if bare != name {
                                self.fn_params.insert((bare.to_string(), i), st.clone());
                            }
                            self.var_types
                                .insert(format!("{}::{}", name, p), st.clone());
                            if bare != name {
                                self.var_types.insert(format!("{}::{}", bare, p), st);
                            }
                        }
                    }
                    self.scan_stmts(body, Some(name), struct_names, fn_names);
                }
                Stmt::Return(Some(expr)) => {
                    self.scan_expr(expr, current_fn, struct_names, fn_names);
                    if let Some(fn_name) = current_fn {
                        if !self.conflicted_returns.contains(fn_name) {
                            if let Some(st) = self.expr_struct_type(expr, current_fn, struct_names) {
                                let bare = resolve_func_bare(fn_name, struct_names);
                                if let Some(existing) = self.fn_returns.get(fn_name) {
                                    if existing != &st {
                                        self.fn_returns.remove(fn_name);
                                        self.fn_returns.remove(bare);
                                        self.conflicted_returns.insert(fn_name.to_string());
                                        if bare != fn_name {
                                            self.conflicted_returns.insert(bare.to_string());
                                        }
                                    }
                                } else {
                                    self.fn_returns.insert(fn_name.to_string(), st.clone());
                                    if bare != fn_name {
                                        self.fn_returns.insert(bare.to_string(), st);
                                    }
                                }
                            }
                        }
                    }
                }
                Stmt::Let {
                    name,
                    type_ann,
                    value,
                } => {
                    self.scan_expr(value, current_fn, struct_names, fn_names);
                    let st = if let Some(t) = type_ann.as_ref() {
                        let b = resolve_func_bare(t, struct_names);
                        if struct_names.contains(t) {
                            Some(t.clone())
                        } else if struct_names.contains(b) {
                            Some(b.to_string())
                        } else {
                            struct_names
                                .iter()
                                .find(|s| {
                                    s.ends_with(&format!("__{}", b))
                                        || s.ends_with(&format!("::{}", b))
                                })
                                .cloned()
                        }
                    } else {
                        self.expr_struct_type(value, current_fn, struct_names)
                    };
                    if let Some(st) = st {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, name), st.clone());
                            let bare = resolve_func_bare(fn_name, struct_names);
                            if bare != fn_name {
                                self.var_types.insert(format!("{}::{}", bare, name), st);
                            }
                        } else {
                            self.var_types.insert(name.clone(), st);
                        }
                    }
                }
                Stmt::Assign { name, value } => {
                    self.scan_expr(value, current_fn, struct_names, fn_names);
                    if let Some(st) = self.expr_struct_type(value, current_fn, struct_names) {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, name), st.clone());
                            let bare = resolve_func_bare(fn_name, struct_names);
                            if bare != fn_name {
                                self.var_types.insert(format!("{}::{}", bare, name), st);
                            }
                        } else {
                            self.var_types.insert(name.clone(), st);
                        }
                    }
                }
                Stmt::ForEach {
                    var,
                    value_var,
                    iterable,
                    body,
                } => {
                    self.scan_expr(iterable, current_fn, struct_names, fn_names);
                    let target_var = value_var.as_ref().unwrap_or(var);
                    if let Some(st) = self.expr_struct_type(iterable, current_fn, struct_names) {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, target_var), st.clone());
                            let bare = resolve_func_bare(fn_name, struct_names);
                            if bare != fn_name {
                                self.var_types
                                    .insert(format!("{}::{}", bare, target_var), st);
                            }
                        } else {
                            self.var_types.insert(target_var.clone(), st);
                        }
                    }
                    self.scan_stmts(body, current_fn, struct_names, fn_names);
                }
                Stmt::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.scan_expr(condition, current_fn, struct_names, fn_names);
                    self.scan_stmts(then_block, current_fn, struct_names, fn_names);
                    if let Some(eb) = else_block {
                        self.scan_stmts(eb, current_fn, struct_names, fn_names);
                    }
                }
                Stmt::While { condition, body } => {
                    self.scan_expr(condition, current_fn, struct_names, fn_names);
                    self.scan_stmts(body, current_fn, struct_names, fn_names);
                }
                Stmt::Repeat { body } => {
                    self.scan_stmts(body, current_fn, struct_names, fn_names);
                }
                Stmt::For {
                    start, end, body, ..
                } => {
                    self.scan_expr(start, current_fn, struct_names, fn_names);
                    self.scan_expr(end, current_fn, struct_names, fn_names);
                    self.scan_stmts(body, current_fn, struct_names, fn_names);
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    self.scan_stmts(try_block, current_fn, struct_names, fn_names);
                    self.scan_stmts(catch_block, current_fn, struct_names, fn_names);
                    if let Some(fb) = finally_block {
                        self.scan_stmts(fb, current_fn, struct_names, fn_names);
                    }
                }
                Stmt::Expr(expr) | Stmt::Say(expr) => {
                    self.scan_expr(expr, current_fn, struct_names, fn_names);
                }
                Stmt::FieldAssign {
                    object,
                    field,
                    value,
                } => {
                    self.scan_expr(object, current_fn, struct_names, fn_names);
                    self.scan_expr(value, current_fn, struct_names, fn_names);
                    if let Some(parent_st) = self.expr_struct_type(object, current_fn, struct_names)
                    {
                        if let Some(val_st) = self.expr_struct_type(value, current_fn, struct_names)
                        {
                            self.field_types
                                .insert((parent_st.clone(), field.clone()), val_st.clone());
                            let bare = resolve_func_bare(&parent_st, struct_names);
                            if bare != parent_st {
                                self.field_types
                                    .insert((bare.to_string(), field.clone()), val_st);
                            }
                        }
                    }
                }
                Stmt::IndexAssign {
                    array,
                    index,
                    value,
                } => {
                    self.scan_expr(array, current_fn, struct_names, fn_names);
                    self.scan_expr(index, current_fn, struct_names, fn_names);
                    self.scan_expr(value, current_fn, struct_names, fn_names);
                }
                Stmt::Pub(inner) | Stmt::Defer(inner) => {
                    self.scan_stmts(
                        std::slice::from_ref(inner),
                        current_fn,
                        struct_names,
                        fn_names,
                    );
                }
                _ => {}
            }
        }
    }

    fn scan_expr(
        &mut self,
        expr: &Expr,
        current_fn: Option<&str>,
        struct_names: &HashSet<String>,
        fn_names: &HashSet<String>,
    ) {
        match expr {
            Expr::Call { name, args } => {
                let resolved_call_name = if let Some(first_arg) = args.first() {
                    if let Some(st) = self.expr_struct_type(first_arg, current_fn, struct_names) {
                        let bare_st = resolve_func_bare(&st, struct_names);
                        let candidate1 = format!("{}__{}", st, name);
                        let candidate2 = format!("{}__{}", bare_st, name);
                        if fn_names.contains(&candidate1) {
                            Some(candidate1)
                        } else if fn_names.contains(&candidate2) {
                            Some(candidate2)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let effective_name = resolved_call_name.as_deref().unwrap_or(name);
                let bare = resolve_func_bare(effective_name, struct_names);
                for (i, arg) in args.iter().enumerate() {
                    if let Some(st) = self.expr_struct_type(arg, current_fn, struct_names) {
                        let key1 = (effective_name.to_string(), i);
                        let key2 = (bare.to_string(), i);
                        if self.conflicted_params.contains(&key1)
                            || self.conflicted_params.contains(&key2)
                        {
                            // Already marked ambiguous, do not assign
                        } else if let Some(existing) = self.fn_params.get(&key1).cloned() {
                            if existing != st {
                                self.conflicted_params.insert(key1.clone());
                                self.conflicted_params.insert(key2.clone());
                                self.fn_params.remove(&key1);
                                self.fn_params.remove(&key2);
                            }
                        } else {
                            self.fn_params.insert(key1, st.clone());
                            if bare != effective_name {
                                self.fn_params.insert(key2, st);
                            }
                        }
                    }
                    self.scan_expr(arg, current_fn, struct_names, fn_names);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left, current_fn, struct_names, fn_names);
                self.scan_expr(right, current_fn, struct_names, fn_names);
            }
            Expr::Unary { expr, .. } => {
                self.scan_expr(expr, current_fn, struct_names, fn_names);
            }
            Expr::Array(elems) => {
                for elem in elems {
                    self.scan_expr(elem, current_fn, struct_names, fn_names);
                }
            }
            Expr::Index { array, index } => {
                self.scan_expr(array, current_fn, struct_names, fn_names);
                self.scan_expr(index, current_fn, struct_names, fn_names);
            }
            Expr::FieldAccess { object, .. } => {
                self.scan_expr(object, current_fn, struct_names, fn_names);
            }
            Expr::StructInit { name, fields } => {
                let bare = resolve_func_bare(name, struct_names);
                for (fname, fval) in fields {
                    if let Some(st) = self.expr_struct_type(fval, current_fn, struct_names) {
                        self.field_types
                            .insert((name.clone(), fname.clone()), st.clone());
                        if bare != name {
                            self.field_types
                                .insert((bare.to_string(), fname.clone()), st);
                        }
                    }
                    self.scan_expr(fval, current_fn, struct_names, fn_names);
                }
            }
            Expr::Map(entries) => {
                for (k, v) in entries {
                    self.scan_expr(k, current_fn, struct_names, fn_names);
                    self.scan_expr(v, current_fn, struct_names, fn_names);
                }
            }
            Expr::InterpolatedString(parts) => {
                for p in parts {
                    self.scan_expr(p, current_fn, struct_names, fn_names);
                }
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                self.scan_expr(condition, current_fn, struct_names, fn_names);
                self.scan_expr(then_branch, current_fn, struct_names, fn_names);
                self.scan_expr(else_branch, current_fn, struct_names, fn_names);
            }
            Expr::NullCoalesce { value, default } => {
                self.scan_expr(value, current_fn, struct_names, fn_names);
                self.scan_expr(default, current_fn, struct_names, fn_names);
            }
            _ => {}
        }
    }
}

pub fn infer_param_struct_type(
    func_name: &str,
    param_idx: usize,
    program: &Program,
) -> Option<String> {
    let inf = StructInference::analyze(program);
    let bare = resolve_func_bare(func_name, &inf.struct_names);
    if inf
        .conflicted_params
        .contains(&(func_name.to_string(), param_idx))
        || inf
            .conflicted_params
            .contains(&(bare.to_string(), param_idx))
    {
        return None;
    }
    inf.fn_params
        .get(&(func_name.to_string(), param_idx))
        .or_else(|| inf.fn_params.get(&(bare.to_string(), param_idx)))
        .cloned()
}

pub fn infer_function_return_struct_type(func_name: &str, program: &Program) -> Option<String> {
    let inf = StructInference::analyze(program);
    let bare = resolve_func_bare(func_name, &inf.struct_names);
    inf.fn_returns
        .get(func_name)
        .or_else(|| inf.fn_returns.get(bare))
        .cloned()
}

pub fn infer_expr_struct_type(expr: &Expr, program: &Program) -> Option<String> {
    let inf = StructInference::analyze(program);
    let mut struct_names = HashSet::new();
    for s in &program.statements {
        if let Stmt::StructDef { name, .. } = s.inner_stmt() {
            struct_names.insert(name.clone());
        }
    }
    inf.expr_struct_type(expr, None, &struct_names)
}

pub fn infer_struct_field_type(
    struct_name: &str,
    field: &str,
    program: &Program,
) -> Option<String> {
    let inf = StructInference::analyze(program);
    let bare = resolve_func_bare(struct_name, &inf.struct_names);
    inf.field_types
        .get(&(struct_name.to_string(), field.to_string()))
        .or_else(|| inf.field_types.get(&(bare.to_string(), field.to_string())))
        .cloned()
}
