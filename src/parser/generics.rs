use crate::ast::{Expr, Program, Stmt};
use std::collections::HashMap;

/// Perform monomorphization of generic functions in the AST.
/// Transforms generic templates into concrete specialized functions based on call sites.
pub fn resolve_generics(program: &mut Program) {
    // 1. Collect generic function templates
    let mut generic_funcs: HashMap<String, Stmt> = HashMap::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name, type_params, ..
        } = stmt.inner_stmt()
        {
            if !type_params.is_empty() {
                generic_funcs.insert(name.clone(), stmt.inner_stmt().clone());
            }
        }
    }

    if generic_funcs.is_empty() {
        return;
    }

    // 2. Track variable types for call argument inference
    let mut known_vars: HashMap<String, String> = HashMap::new();
    let mut specializations: HashMap<String, Stmt> = HashMap::new();

    // 3. Monomorphize all call sites in statements
    for stmt in &mut program.statements {
        monomorphize_stmt(stmt, &generic_funcs, &mut known_vars, &mut specializations);
    }

    // 4. Also monomorphize any calls within the specialized functions themselves (up to fixpoint)
    let mut pass = 0;
    while pass < 5 {
        let current_specs: Vec<Stmt> = specializations.values().cloned().collect();
        let prev_len = specializations.len();
        for mut spec_fn in current_specs {
            monomorphize_stmt(
                &mut spec_fn,
                &generic_funcs,
                &mut known_vars,
                &mut specializations,
            );
        }
        if specializations.len() == prev_len {
            break;
        }
        pass += 1;
    }

    // 5. Remove unspecialized generic templates and add concrete specialized functions
    program.statements.retain(|stmt| {
        if let Stmt::Function { type_params, .. } = stmt.inner_stmt() {
            type_params.is_empty()
        } else {
            true
        }
    });

    for (_, spec_fn) in specializations {
        program.statements.push(spec_fn);
    }
}

fn monomorphize_stmt(
    stmt: &mut Stmt,
    generic_funcs: &HashMap<String, Stmt>,
    known_vars: &mut HashMap<String, String>,
    specializations: &mut HashMap<String, Stmt>,
) {
    match stmt {
        Stmt::Let {
            name,
            type_ann,
            value,
        } => {
            monomorphize_expr(value, generic_funcs, known_vars, specializations);
            if let Some(ann) = type_ann {
                known_vars.insert(name.clone(), ann.clone());
            } else if let Expr::Call { name: callee, .. } = value {
                if let Some(Stmt::Function {
                    return_type: Some(rt),
                    ..
                }) = specializations.get(callee)
                {
                    known_vars.insert(name.clone(), rt.clone());
                }
            } else if let Some(ty) = infer_concrete_type(value, known_vars) {
                known_vars.insert(name.clone(), ty);
            }
        }
        Stmt::Assign { name, value } => {
            monomorphize_expr(value, generic_funcs, known_vars, specializations);
            if let Some(ty) = infer_concrete_type(value, known_vars) {
                known_vars.insert(name.clone(), ty);
            }
        }
        Stmt::Say(expr) | Stmt::Expr(expr) | Stmt::Return(Some(expr)) | Stmt::Throw(Some(expr)) => {
            monomorphize_expr(expr, generic_funcs, known_vars, specializations);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            monomorphize_expr(condition, generic_funcs, known_vars, specializations);
            for s in then_block {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
            if let Some(eb) = else_block {
                for s in eb {
                    monomorphize_stmt(s, generic_funcs, known_vars, specializations);
                }
            }
        }
        Stmt::While { condition, body } => {
            monomorphize_expr(condition, generic_funcs, known_vars, specializations);
            for s in body {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            monomorphize_expr(start, generic_funcs, known_vars, specializations);
            monomorphize_expr(end, generic_funcs, known_vars, specializations);
            for s in body {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            monomorphize_expr(iterable, generic_funcs, known_vars, specializations);
            for s in body {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
        }
        Stmt::Function { body, .. } => {
            for s in body {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
            for s in catch_block {
                monomorphize_stmt(s, generic_funcs, known_vars, specializations);
            }
            if let Some(fb) = finally_block {
                for s in fb {
                    monomorphize_stmt(s, generic_funcs, known_vars, specializations);
                }
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            monomorphize_stmt(inner, generic_funcs, known_vars, specializations);
        }
        _ => {}
    }
}

fn monomorphize_expr(
    expr: &mut Expr,
    generic_funcs: &HashMap<String, Stmt>,
    known_vars: &mut HashMap<String, String>,
    specializations: &mut HashMap<String, Stmt>,
) {
    match expr {
        Expr::Call { name, args } => {
            for arg in args.iter_mut() {
                monomorphize_expr(arg, generic_funcs, known_vars, specializations);
            }

            // Check if this call targets a generic function template:
            // either direct name "swap" or explicit call "swap__int"
            let (target_template_name, explicit_type_opt) =
                if let Some(_template) = generic_funcs.get(name) {
                    (Some(name.clone()), None)
                } else if let Some((base, type_suffix)) = name.split_once("__") {
                    if generic_funcs.contains_key(base) {
                        (Some(base.to_string()), Some(type_suffix.to_string()))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

            if let Some(template_name) = target_template_name {
                if let Some(Stmt::Function {
                    params: t_params,
                    param_types: t_param_types,
                    return_type: t_return_type,
                    defaults: t_defaults,
                    body: t_body,
                    type_params: t_type_params,
                    attributes: t_attributes,
                    ..
                }) = generic_funcs.get(&template_name).cloned()
                {
                    let mut subst: HashMap<String, String> = HashMap::new();

                    if let Some(explicit_suffix) = explicit_type_opt {
                        let parts: Vec<&str> = explicit_suffix.split('_').collect();
                        for (i, tp) in t_type_params.iter().enumerate() {
                            if let Some(concrete) = parts.get(i) {
                                subst.insert(tp.clone(), concrete.to_string());
                            }
                        }
                    } else {
                        // Infer type arguments from concrete arguments at call site
                        for (i, arg) in args.iter().enumerate() {
                            if let Some(Some(ptype)) = t_param_types.get(i) {
                                if t_type_params.contains(ptype) && !subst.contains_key(ptype) {
                                    if let Some(concrete_ty) = infer_concrete_type(arg, known_vars)
                                    {
                                        subst.insert(ptype.clone(), concrete_ty);
                                    }
                                } else {
                                    // Parameterized types mentioning a type
                                    // parameter (e.g. `T[]`, `map[string, T]`):
                                    // infer the parameter from the argument's
                                    // element type instead of defaulting.
                                    for tp in t_type_params.iter() {
                                        if subst.contains_key(tp) {
                                            continue;
                                        }
                                        if type_str_mentions_param(ptype, tp) {
                                            if let Some(concrete_ty) =
                                                infer_param_from_arg(arg, known_vars)
                                            {
                                                subst.insert(tp.clone(), concrete_ty);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Default any remaining unmapped type parameters to "int"
                    for tp in &t_type_params {
                        subst.entry(tp.clone()).or_insert_with(|| "int".to_string());
                    }

                    let type_suffix = t_type_params
                        .iter()
                        .map(|tp| subst.get(tp).cloned().unwrap_or_else(|| "int".to_string()))
                        .collect::<Vec<_>>()
                        .join("_");

                    let specialized_name = format!("{}__{}", template_name, type_suffix);
                    *name = specialized_name.clone();

                    specializations
                        .entry(specialized_name.clone())
                        .or_insert_with(|| {
                            let spec_param_types = t_param_types
                                .iter()
                                .map(|opt_t| opt_t.as_ref().map(|t| substitute_type_str(t, &subst)))
                                .collect();
                            let spec_return_type = t_return_type
                                .as_ref()
                                .map(|t| substitute_type_str(t, &subst));
                            let spec_body = substitute_in_stmts(&t_body, &subst);

                            Stmt::Function {
                                name: specialized_name,
                                params: t_params,
                                param_types: spec_param_types,
                                return_type: spec_return_type,
                                defaults: t_defaults,
                                body: spec_body,
                                type_params: vec![],
                                attributes: t_attributes,
                            }
                        });
                }
            }
        }
        Expr::Binary { left, right, .. } => {
            monomorphize_expr(left, generic_funcs, known_vars, specializations);
            monomorphize_expr(right, generic_funcs, known_vars, specializations);
        }
        Expr::Unary { expr: operand, .. } => {
            monomorphize_expr(operand, generic_funcs, known_vars, specializations);
        }
        Expr::Array(elements) | Expr::InterpolatedString(elements) => {
            for elem in elements {
                monomorphize_expr(elem, generic_funcs, known_vars, specializations);
            }
        }
        Expr::Index { array, index } => {
            monomorphize_expr(array, generic_funcs, known_vars, specializations);
            monomorphize_expr(index, generic_funcs, known_vars, specializations);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            monomorphize_expr(object, generic_funcs, known_vars, specializations);
        }
        Expr::ForceUnwrap(inner) => {
            monomorphize_expr(inner, generic_funcs, known_vars, specializations);
        }
        Expr::StructInit { fields, .. } => {
            for (_, fval) in fields {
                monomorphize_expr(fval, generic_funcs, known_vars, specializations);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            monomorphize_expr(condition, generic_funcs, known_vars, specializations);
            monomorphize_expr(then_branch, generic_funcs, known_vars, specializations);
            monomorphize_expr(else_branch, generic_funcs, known_vars, specializations);
        }
        Expr::NullCoalesce { value, default } => {
            monomorphize_expr(value, generic_funcs, known_vars, specializations);
            monomorphize_expr(default, generic_funcs, known_vars, specializations);
        }
        _ => {}
    }
}

/// Word-boundary check: does the type string mention the type parameter?
/// `T` matches in `T[]` or `map[string, T]` but not in `TT` or `Target`.
fn type_str_mentions_param(ptype: &str, tp: &str) -> bool {
    let mut chars = ptype.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            let mut word = String::new();
            word.push(ch);
            while let Some(&next_ch) = chars.peek() {
                if next_ch.is_alphanumeric() || next_ch == '_' {
                    word.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            if word == tp {
                return true;
            }
        }
    }
    false
}

/// Infers a type parameter's concrete type from a call argument by looking
/// through one container level: array literals use their first element's
/// type, identifiers use their known element type (`string[]` -> `string`).
fn infer_param_from_arg(arg: &Expr, known_vars: &HashMap<String, String>) -> Option<String> {
    match arg {
        Expr::Array(elems) => {
            let first = elems.first()?;
            infer_concrete_type(first, known_vars)
        }
        Expr::Identifier(id) => {
            let ty = known_vars.get(id)?;
            if let Some(elem) = ty.strip_suffix("[]") {
                Some(elem.to_string())
            } else {
                Some(ty.clone())
            }
        }
        other => infer_concrete_type(other, known_vars),
    }
}

fn infer_concrete_type(expr: &Expr, known_vars: &HashMap<String, String>) -> Option<String> {
    match expr {
        Expr::Number(n) => {
            if n.fract() == 0.0 {
                Some("int".to_string())
            } else {
                Some("float".to_string())
            }
        }
        Expr::Float(_) => Some("float".to_string()),
        Expr::String(_) | Expr::InterpolatedString(_) => Some("string".to_string()),
        Expr::Array(_) => Some("array".to_string()),
        Expr::StructInit { name, .. } => Some(name.clone()),
        Expr::Identifier(id) => known_vars.get(id).cloned(),
        Expr::Index { array, index } => {
            if let (Expr::Identifier(id), Expr::Number(idx)) = (&**array, &**index) {
                if let Some(tuple_ty) = known_vars.get(id) {
                    let inner = tuple_ty.trim_matches(|c| c == '(' || c == ')');
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
                    if let Some(ty) = parts.get(*idx as usize) {
                        return Some(ty.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn substitute_type_str(s: &str, subst: &HashMap<String, String>) -> String {
    let mut res = s.to_string();
    for (param, concrete) in subst {
        let mut out = String::new();
        let mut chars = res.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch.is_alphanumeric() || ch == '_' {
                let mut word = String::new();
                word.push(ch);
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        word.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if word == *param {
                    out.push_str(concrete);
                } else {
                    out.push_str(&word);
                }
            } else {
                out.push(ch);
            }
        }
        res = out;
    }
    res
}

fn substitute_in_stmts(stmts: &[Stmt], subst: &HashMap<String, String>) -> Vec<Stmt> {
    stmts
        .iter()
        .map(|s| match s {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => Stmt::Let {
                name: name.clone(),
                type_ann: type_ann.as_ref().map(|t| substitute_type_str(t, subst)),
                value: value.clone(),
            },
            Stmt::Function {
                name,
                params,
                param_types,
                return_type,
                defaults,
                body,
                type_params,
                attributes,
            } => Stmt::Function {
                name: name.clone(),
                params: params.clone(),
                param_types: param_types
                    .iter()
                    .map(|t| t.as_ref().map(|ty| substitute_type_str(ty, subst)))
                    .collect(),
                return_type: return_type.as_ref().map(|t| substitute_type_str(t, subst)),
                defaults: defaults.clone(),
                body: substitute_in_stmts(body, subst),
                type_params: type_params.clone(),
                attributes: attributes.clone(),
            },
            other => other.clone(),
        })
        .collect()
}
