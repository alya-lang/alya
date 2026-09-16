use crate::ast::{BinaryOp, Expr, Program, Stmt};
use std::collections::{HashMap, HashSet};

/// Scope tracker for constant resolution and validation.
struct ScopeStack {
    scopes: Vec<HashMap<String, Expr>>,
    var_scopes: Vec<HashSet<String>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            var_scopes: vec![HashSet::new()],
        }
    }

    fn push(&mut self) {
        self.scopes.push(HashMap::new());
        self.var_scopes.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
        self.var_scopes.pop();
    }

    fn define_const(&mut self, name: &str, val: Expr) -> Result<(), String> {
        let current_constants = self.scopes.last_mut().unwrap();
        if current_constants.contains_key(name) {
            return Err(format!("Cannot redeclare constant '{}'", name));
        }
        let current_vars = self.var_scopes.last().unwrap();
        if current_vars.contains(name) {
            return Err(format!("Cannot redeclare variable '{}' as constant", name));
        }
        current_constants.insert(name.to_string(), val.clone());
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        if bare != name {
            current_constants.insert(bare.to_string(), val);
        }
        Ok(())
    }

    fn define_var(&mut self, name: &str) -> Result<(), String> {
        let current_constants = self.scopes.last().unwrap();
        if current_constants.contains_key(name) {
            return Err(format!("Cannot redeclare constant '{}'", name));
        }
        self.var_scopes.last_mut().unwrap().insert(name.to_string());
        Ok(())
    }

    fn check_assign(&self, name: &str) -> Result<(), String> {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return Err(format!("Cannot assign to constant '{}'", name));
            }
        }
        Ok(())
    }

    fn get_const(&self, name: &str) -> Option<&Expr> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val);
            }
        }
        None
    }
}

/// Resolves compile-time constants in AST and validates that constants are never reassigned or redeclared.
pub fn resolve_and_validate_constants(program: &mut Program) -> Result<(), String> {
    let mut stack = ScopeStack::new();
    for stmt in &mut program.statements {
        resolve_and_validate_stmt(stmt, &mut stack)?;
    }
    Ok(())
}

fn resolve_and_validate_stmt(stmt: &mut Stmt, stack: &mut ScopeStack) -> Result<(), String> {
    match stmt {
        Stmt::Const { name, value } => {
            resolve_expr(value, stack);
            fold_expr(value);
            stack.define_const(name, value.clone())?;
        }
        Stmt::Let { name, value } => {
            resolve_expr(value, stack);
            stack.define_var(name)?;
        }
        Stmt::Assign { name, value } => {
            stack.check_assign(name)?;
            resolve_expr(value, stack);
        }
        Stmt::Say(expr) | Stmt::Expr(expr) => {
            resolve_expr(expr, stack);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            resolve_expr(condition, stack);
            stack.push();
            for s in then_block {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
            if let Some(eb) = else_block {
                stack.push();
                for s in eb {
                    resolve_and_validate_stmt(s, stack)?;
                }
                stack.pop();
            }
        }
        Stmt::While { condition, body } => {
            resolve_expr(condition, stack);
            stack.push();
            for s in body {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
        }
        Stmt::Repeat { body } => {
            stack.push();
            for s in body {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
        }
        Stmt::For {
            var,
            start,
            end,
            body,
            ..
        } => {
            stack.check_assign(var)?;
            resolve_expr(start, stack);
            resolve_expr(end, stack);
            stack.push();
            stack.define_var(var)?;
            for s in body {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
        }
        Stmt::ForEach {
            var,
            iterable,
            body,
        } => {
            stack.check_assign(var)?;
            resolve_expr(iterable, stack);
            stack.push();
            stack.define_var(var)?;
            for s in body {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
        }
        Stmt::Function {
            params,
            defaults,
            body,
            ..
        } => {
            for def in defaults.iter_mut().flatten() {
                resolve_expr(def, stack);
            }
            stack.push();
            for p in params {
                stack.define_var(p)?;
            }
            for s in body {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();
        }
        Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            resolve_expr(expr, stack);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            resolve_expr(array, stack);
            resolve_expr(index, stack);
            resolve_expr(value, stack);
        }
        Stmt::FieldAssign { object, value, .. } => {
            resolve_expr(object, stack);
            resolve_expr(value, stack);
        }
        Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
        } => {
            stack.push();
            for s in try_block {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();

            stack.push();
            if let Some(cvar) = catch_var {
                stack.define_var(cvar)?;
            }
            for s in catch_block {
                resolve_and_validate_stmt(s, stack)?;
            }
            stack.pop();

            if let Some(fb) = finally_block {
                stack.push();
                for s in fb {
                    resolve_and_validate_stmt(s, stack)?;
                }
                stack.pop();
            }
        }
        Stmt::StructDef { defaults, .. } => {
            for def in defaults.iter_mut().flatten() {
                resolve_expr(def, stack);
                fold_expr(def);
            }
        }
        _ => {}
    }
    Ok(())
}

fn resolve_expr(expr: &mut Expr, stack: &ScopeStack) {
    match expr {
        Expr::Identifier(ident) => {
            if let Some(val) = stack.get_const(ident) {
                *expr = val.clone();
                return;
            }
            if ident.contains("::") {
                let bare = ident.rsplit("::").next().unwrap_or(ident);
                if let Some(val) = stack.get_const(bare) {
                    *expr = val.clone();
                    return;
                }
            }
        }
        Expr::FieldAccess { object, field } => {
            resolve_expr(object, stack);
            if let Expr::Identifier(name) = &**object {
                let scoped_colon = format!("{}::{}", name, field);
                let scoped_dot = format!("{}.{}", name, field);
                if let Some(val) = stack
                    .get_const(&scoped_colon)
                    .or_else(|| stack.get_const(&scoped_dot))
                {
                    *expr = val.clone();
                    return;
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            resolve_expr(left, stack);
            resolve_expr(right, stack);
        }
        Expr::Unary { expr: inner, .. } => {
            resolve_expr(inner, stack);
        }
        Expr::Call { args, .. } => {
            for arg in args {
                resolve_expr(arg, stack);
            }
        }
        Expr::Array(items) => {
            for item in items {
                resolve_expr(item, stack);
            }
        }
        Expr::Index { array, index } => {
            resolve_expr(array, stack);
            resolve_expr(index, stack);
        }
        Expr::StructInit { fields, .. } => {
            for (_, val) in fields {
                resolve_expr(val, stack);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                resolve_expr(k, stack);
                resolve_expr(v, stack);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            resolve_expr(condition, stack);
            resolve_expr(then_branch, stack);
            resolve_expr(else_branch, stack);
        }
        Expr::NullCoalesce { value, default } => {
            resolve_expr(value, stack);
            resolve_expr(default, stack);
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                resolve_expr(part, stack);
            }
        }
        _ => {}
    }
}

/// Constant folding for basic compile-time constant arithmetic.
fn fold_expr(expr: &mut Expr) {
    if let Expr::Binary { op, left, right } = expr {
        if let (Expr::Number(l), Expr::Number(r)) = (&**left, &**right) {
            let folded = match op {
                BinaryOp::Add => Some(l + r),
                BinaryOp::Subtract => Some(l - r),
                BinaryOp::Multiply => Some(l * r),
                BinaryOp::Divide if *r != 0.0 => Some(l / r),
                BinaryOp::Modulo if *r != 0.0 => Some(l % r),
                _ => None,
            };
            if let Some(val) = folded {
                *expr = Expr::Number(val);
            }
        }
    } else if let Expr::Unary { op, expr: inner } = expr {
        if let Expr::Number(n) = &**inner {
            if matches!(op, crate::ast::UnaryOp::Negate) {
                *expr = Expr::Number(-n);
            }
        }
    }
}
