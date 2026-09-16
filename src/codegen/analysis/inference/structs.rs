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
}

impl StructInference {
    pub fn analyze(program: &Program) -> Self {
        let mut inf = StructInference::default();
        let mut struct_names = HashSet::new();
        for s in &program.statements {
            if let Stmt::StructDef {
                name,
                fields,
                field_types,
                ..
            } = s.inner_stmt()
            {
                struct_names.insert(name.clone());
                for (f, ft) in fields.iter().zip(field_types.iter()) {
                    if let Some(t) = ft {
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

        for _ in 0..6 {
            let prev_len = inf.fn_returns.len()
                + inf.fn_params.len()
                + inf.var_types.len()
                + inf.field_types.len();
            inf.scan_stmts(&program.statements, None, &struct_names);
            if inf.fn_returns.len()
                + inf.fn_params.len()
                + inf.var_types.len()
                + inf.field_types.len()
                == prev_len
            {
                break;
            }
        }

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
            Expr::Call { name, .. } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
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
                if let Some(fn_name) = current_fn {
                    if let Some(st) = self.var_types.get(&format!("{}::{}", fn_name, vname)) {
                        return Some(st.clone());
                    }
                    let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if let Some(st) = self.var_types.get(&format!("{}::{}", bare, vname)) {
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
                    let bare = parent_st.rsplit("::").next().unwrap_or(&parent_st);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
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
    ) {
        for s in stmts {
            match s {
                Stmt::Function {
                    name, params, body, ..
                } => {
                    let bare = name.rsplit("::").next().unwrap_or(name);
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    for (i, p) in params.iter().enumerate() {
                        let st = self
                            .fn_params
                            .get(&(name.clone(), i))
                            .or_else(|| self.fn_params.get(&(bare.to_string(), i)))
                            .cloned();
                        if let Some(st) = st {
                            self.var_types
                                .insert(format!("{}::{}", name, p), st.clone());
                            if bare != name {
                                self.var_types.insert(format!("{}::{}", bare, p), st);
                            }
                        }
                    }
                    self.scan_stmts(body, Some(name), struct_names);
                }
                Stmt::Return(Some(expr)) => {
                    self.scan_expr(expr, current_fn, struct_names);
                    if let Some(fn_name) = current_fn {
                        if let Some(st) = self.expr_struct_type(expr, current_fn, struct_names) {
                            self.fn_returns.insert(fn_name.to_string(), st.clone());
                            let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            if bare != fn_name {
                                self.fn_returns.insert(bare.to_string(), st);
                            }
                        }
                    }
                }
                Stmt::Let {
                    name,
                    type_ann,
                    value,
                } => {
                    self.scan_expr(value, current_fn, struct_names);
                    let st = type_ann
                        .as_ref()
                        .filter(|t| struct_names.contains(*t))
                        .cloned()
                        .or_else(|| self.expr_struct_type(value, current_fn, struct_names));
                    if let Some(st) = st {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, name), st.clone());
                            let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            if bare != fn_name {
                                self.var_types.insert(format!("{}::{}", bare, name), st);
                            }
                        } else {
                            self.var_types.insert(name.clone(), st);
                        }
                    }
                }
                Stmt::Assign { name, value } => {
                    self.scan_expr(value, current_fn, struct_names);
                    if let Some(st) = self.expr_struct_type(value, current_fn, struct_names) {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, name), st.clone());
                            let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
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
                    self.scan_expr(iterable, current_fn, struct_names);
                    let target_var = value_var.as_ref().unwrap_or(var);
                    if let Some(st) = self.expr_struct_type(iterable, current_fn, struct_names) {
                        if let Some(fn_name) = current_fn {
                            self.var_types
                                .insert(format!("{}::{}", fn_name, target_var), st.clone());
                            let bare = fn_name.rsplit("::").next().unwrap_or(fn_name);
                            let bare = bare.rsplit("__").next().unwrap_or(bare);
                            if bare != fn_name {
                                self.var_types
                                    .insert(format!("{}::{}", bare, target_var), st);
                            }
                        } else {
                            self.var_types.insert(target_var.clone(), st);
                        }
                    }
                    self.scan_stmts(body, current_fn, struct_names);
                }
                Stmt::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    self.scan_expr(condition, current_fn, struct_names);
                    self.scan_stmts(then_block, current_fn, struct_names);
                    if let Some(eb) = else_block {
                        self.scan_stmts(eb, current_fn, struct_names);
                    }
                }
                Stmt::While { condition, body } => {
                    self.scan_expr(condition, current_fn, struct_names);
                    self.scan_stmts(body, current_fn, struct_names);
                }
                Stmt::Repeat { body } => {
                    self.scan_stmts(body, current_fn, struct_names);
                }
                Stmt::For {
                    start, end, body, ..
                } => {
                    self.scan_expr(start, current_fn, struct_names);
                    self.scan_expr(end, current_fn, struct_names);
                    self.scan_stmts(body, current_fn, struct_names);
                }
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    self.scan_stmts(try_block, current_fn, struct_names);
                    self.scan_stmts(catch_block, current_fn, struct_names);
                    if let Some(fb) = finally_block {
                        self.scan_stmts(fb, current_fn, struct_names);
                    }
                }
                Stmt::Expr(expr) | Stmt::Say(expr) => {
                    self.scan_expr(expr, current_fn, struct_names);
                }
                Stmt::FieldAssign { object, value, .. } => {
                    self.scan_expr(object, current_fn, struct_names);
                    self.scan_expr(value, current_fn, struct_names);
                }
                Stmt::IndexAssign {
                    array,
                    index,
                    value,
                } => {
                    self.scan_expr(array, current_fn, struct_names);
                    self.scan_expr(index, current_fn, struct_names);
                    self.scan_expr(value, current_fn, struct_names);
                }
                Stmt::Pub(inner) | Stmt::Defer(inner) => {
                    self.scan_stmts(std::slice::from_ref(inner), current_fn, struct_names);
                }
                _ => {}
            }
        }
    }

    fn scan_expr(&mut self, expr: &Expr, current_fn: Option<&str>, struct_names: &HashSet<String>) {
        match expr {
            Expr::Call { name, args } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                for (i, arg) in args.iter().enumerate() {
                    if let Some(st) = self.expr_struct_type(arg, current_fn, struct_names) {
                        self.fn_params.insert((name.clone(), i), st.clone());
                        if bare != name {
                            self.fn_params.insert((bare.to_string(), i), st);
                        }
                    }
                    self.scan_expr(arg, current_fn, struct_names);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left, current_fn, struct_names);
                self.scan_expr(right, current_fn, struct_names);
            }
            Expr::Unary { expr, .. } => {
                self.scan_expr(expr, current_fn, struct_names);
            }
            Expr::Array(elems) => {
                for elem in elems {
                    self.scan_expr(elem, current_fn, struct_names);
                }
            }
            Expr::Index { array, index } => {
                self.scan_expr(array, current_fn, struct_names);
                self.scan_expr(index, current_fn, struct_names);
            }
            Expr::FieldAccess { object, .. } => {
                self.scan_expr(object, current_fn, struct_names);
            }
            Expr::StructInit { name, fields } => {
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                for (fname, fval) in fields {
                    if let Some(st) = self.expr_struct_type(fval, current_fn, struct_names) {
                        self.field_types
                            .insert((name.clone(), fname.clone()), st.clone());
                        if bare != name {
                            self.field_types
                                .insert((bare.to_string(), fname.clone()), st);
                        }
                    }
                    self.scan_expr(fval, current_fn, struct_names);
                }
            }
            Expr::Map(entries) => {
                for (k, v) in entries {
                    self.scan_expr(k, current_fn, struct_names);
                    self.scan_expr(v, current_fn, struct_names);
                }
            }
            Expr::InterpolatedString(parts) => {
                for p in parts {
                    self.scan_expr(p, current_fn, struct_names);
                }
            }
            Expr::Ternary {
                condition,
                then_branch,
                else_branch,
            } => {
                self.scan_expr(condition, current_fn, struct_names);
                self.scan_expr(then_branch, current_fn, struct_names);
                self.scan_expr(else_branch, current_fn, struct_names);
            }
            Expr::NullCoalesce { value, default } => {
                self.scan_expr(value, current_fn, struct_names);
                self.scan_expr(default, current_fn, struct_names);
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
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    inf.fn_params
        .get(&(func_name.to_string(), param_idx))
        .or_else(|| inf.fn_params.get(&(bare.to_string(), param_idx)))
        .cloned()
}

pub fn infer_function_return_struct_type(func_name: &str, program: &Program) -> Option<String> {
    let inf = StructInference::analyze(program);
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
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
    let bare = struct_name.rsplit("::").next().unwrap_or(struct_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    inf.field_types
        .get(&(struct_name.to_string(), field.to_string()))
        .or_else(|| inf.field_types.get(&(bare.to_string(), field.to_string())))
        .cloned()
}
