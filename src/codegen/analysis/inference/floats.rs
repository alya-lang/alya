use crate::ast::*;
use crate::codegen::analysis::inference::common::collect_function_defs;
use crate::codegen::analysis::traversal::CallIndex;
use std::collections::HashSet;

fn expr_is_definitely_float(expr: &Expr, known_floats: &HashSet<String>) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::Number(n) => n.fract() != 0.0,
        Expr::Identifier(name) => known_floats.contains(name),
        Expr::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Modulo,
            right,
        } => {
            expr_is_definitely_float(left, known_floats)
                || expr_is_definitely_float(right, known_floats)
        }
        Expr::Unary {
            op: UnaryOp::Negate,
            expr,
        } => expr_is_definitely_float(expr, known_floats),
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "float"
                    | "to_float"
                    | "parse_float"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "mean"
                    | "deg_to_rad"
                    | "rad_to_deg"
                    | "radians"
                    | "degrees"
                    | "lerp"
                    | "norm"
                    | "smoothstep"
                    | "variance"
                    | "_normalize_angle"
                    | "rand_float"
                    | "rand_float_range"
                    | "float_range"
                    | "rand_rng_float"
                    | "uniform_float"
                    | "rng_float"
                    | "dist_float"
                    | "dist_uniform_float"
                    | "dist_normal"
                    | "dist_exponential"
                    | "normal"
                    | "exponential"
                    | "native_sin"
                    | "native_cos"
                    | "native_tan"
                    | "native_asin"
                    | "native_acos"
                    | "native_atan"
                    | "native_atan2"
                    | "native_sinh"
                    | "native_cosh"
                    | "native_tanh"
                    | "native_log"
                    | "native_log2"
                    | "native_log10"
                    | "native_exp"
                    | "native_sqrt"
                    | "native_ceil"
                    | "native_floor"
                    | "native_fmod"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "atan2"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "log_n"
                    | "log2"
                    | "log10"
                    | "exp_f"
                    | "fmod"
                    | "sqrt_f"
            ) || known_floats.contains(&format!("fn_ret_flt:{}", name))
                || known_floats.contains(&format!("fn_ret_flt:{}", bare))
        }
        Expr::Index { array, index } => {
            if let (Expr::Identifier(arr_name), Expr::Number(idx)) = (&**array, &**index) {
                if known_floats.contains(&format!("tuple_elem_flt:{}:{}", arr_name, *idx as usize))
                {
                    return true;
                }
            }
            match &**array {
                Expr::Identifier(arr_name) => {
                    known_floats.contains(&format!("arr_is_flt:{}", arr_name))
                }
                Expr::ForceUnwrap(inner) => expr_is_definitely_float(inner, known_floats),
                _ => false,
            }
        }
        Expr::Cast {
            expr: inner,
            target,
        } => {
            let t = target.to_lowercase();
            if t == "float" || t == "f64" || t == "f32" {
                true
            } else if t == "int"
                || t == "i64"
                || t == "i32"
                || t == "usize"
                || t == "str"
                || t == "string"
            {
                false
            } else {
                expr_is_definitely_float(inner, known_floats)
            }
        }
        Expr::ForceUnwrap(inner) => expr_is_definitely_float(inner, known_floats),
        _ => false,
    }
}

fn expr_is_float_array(expr: &Expr, known_floats: &HashSet<String>) -> bool {
    match expr {
        Expr::Array(elems) => elems
            .first()
            .is_some_and(|e| expr_is_definitely_float(e, known_floats)),
        Expr::Identifier(name) => known_floats.contains(&format!("arr_is_flt:{}", name)),
        Expr::ForceUnwrap(inner) => expr_is_float_array(inner, known_floats),
        _ => false,
    }
}

fn stmts_return_float(stmts: &[Stmt], known_floats: &HashSet<String>) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return(Some(expr)) => expr_is_definitely_float(expr, known_floats),
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            stmts_return_float(then_block, known_floats)
                || else_block
                    .as_ref()
                    .is_some_and(|eb| stmts_return_float(eb, known_floats))
        }
        Stmt::While { body, .. }
        | Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. } => stmts_return_float(body, known_floats),
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            stmts_return_float(try_block, known_floats)
                || stmts_return_float(catch_block, known_floats)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| stmts_return_float(fb, known_floats))
        }
        _ => false,
    })
}

fn collect_tuple_returns_float(
    stmts: &[Stmt],
    known_floats: &HashSet<String>,
    fn_name: &str,
    target_floats: &mut HashSet<String>,
) {
    for s in stmts {
        match s {
            Stmt::Return(Some(Expr::Array(elements))) => {
                for (i, elem) in elements.iter().enumerate() {
                    if expr_is_definitely_float(elem, known_floats) {
                        target_floats.insert(format!("fn_ret_tuple_flt:{}:{}", fn_name, i));
                    }
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_tuple_returns_float(then_block, known_floats, fn_name, target_floats);
                if let Some(eb) = else_block {
                    collect_tuple_returns_float(eb, known_floats, fn_name, target_floats);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_tuple_returns_float(body, known_floats, fn_name, target_floats);
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_tuple_returns_float(
                    std::slice::from_ref(inner),
                    known_floats,
                    fn_name,
                    target_floats,
                );
            }
            _ => {}
        }
    }
}

fn collect_float_vars_from_stmts(
    stmts: &[Stmt],
    scope: &mut HashSet<String>,
    known_floats: &mut HashSet<String>,
    is_top_level: bool,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let {
                name,
                type_ann,
                value,
            } => {
                if let Some(t) = type_ann {
                    if t == "float" || t == "f64" || t == "f32" {
                        scope.insert(name.clone());
                        if is_top_level {
                            known_floats.insert(name.clone());
                        }
                    } else if t == "float[]" || t == "f64[]" || t == "f32[]" {
                        scope.insert(format!("arr_is_flt:{}", name));
                        if is_top_level {
                            known_floats.insert(format!("arr_is_flt:{}", name));
                        }
                    }
                }
                if expr_is_definitely_float(value, scope) {
                    scope.insert(name.clone());
                    if is_top_level {
                        known_floats.insert(name.clone());
                    }
                }
                if expr_is_float_array(value, scope) {
                    scope.insert(format!("arr_is_flt:{}", name));
                    if is_top_level {
                        known_floats.insert(format!("arr_is_flt:{}", name));
                    }
                }
                if let Expr::Call { name: cname, .. } = value {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let prefix1 = format!("fn_ret_tuple_flt:{}:", cname);
                    let prefix2 = format!("fn_ret_tuple_flt:{}:", bare);
                    for item in known_floats.clone() {
                        if item.starts_with(&prefix1) {
                            if let Some(idx_str) = item.strip_prefix(&prefix1) {
                                scope.insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                if is_top_level {
                                    known_floats
                                        .insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                }
                            }
                        } else if item.starts_with(&prefix2) {
                            if let Some(idx_str) = item.strip_prefix(&prefix2) {
                                scope.insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                if is_top_level {
                                    known_floats
                                        .insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                }
                            }
                        }
                    }
                } else if let Expr::Array(elems) = value {
                    for (i, elem) in elems.iter().enumerate() {
                        if expr_is_definitely_float(elem, scope) {
                            scope.insert(format!("tuple_elem_flt:{}:{}", name, i));
                            if is_top_level {
                                known_floats.insert(format!("tuple_elem_flt:{}:{}", name, i));
                            }
                        }
                    }
                }
                if let Some(ann) = type_ann {
                    let ann = ann.trim();
                    if ann.starts_with('(') && ann.ends_with(')') {
                        for (i, ty) in ann[1..ann.len() - 1].split(',').enumerate() {
                            let ty = ty.trim();
                            if ty == "float" || ty == "f64" || ty == "f32" {
                                scope.insert(format!("tuple_elem_flt:{}:{}", name, i));
                                if is_top_level {
                                    known_floats.insert(format!("tuple_elem_flt:{}:{}", name, i));
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Assign { name, value, .. } => {
                if expr_is_definitely_float(value, scope) {
                    scope.insert(name.clone());
                    if is_top_level {
                        known_floats.insert(name.clone());
                    }
                }
                if expr_is_float_array(value, scope) {
                    scope.insert(format!("arr_is_flt:{}", name));
                    if is_top_level {
                        known_floats.insert(format!("arr_is_flt:{}", name));
                    }
                }
                if let Expr::Call { name: cname, .. } = value {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    let prefix1 = format!("fn_ret_tuple_flt:{}:", cname);
                    let prefix2 = format!("fn_ret_tuple_flt:{}:", bare);
                    for item in known_floats.clone() {
                        if item.starts_with(&prefix1) {
                            if let Some(idx_str) = item.strip_prefix(&prefix1) {
                                scope.insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                if is_top_level {
                                    known_floats
                                        .insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                }
                            }
                        } else if item.starts_with(&prefix2) {
                            if let Some(idx_str) = item.strip_prefix(&prefix2) {
                                scope.insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                if is_top_level {
                                    known_floats
                                        .insert(format!("tuple_elem_flt:{}:{}", name, idx_str));
                                }
                            }
                        }
                    }
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                let mut then_scope = scope.clone();
                collect_float_vars_from_stmts(
                    then_block,
                    &mut then_scope,
                    known_floats,
                    is_top_level,
                );
                if let Some(else_stmts) = else_block {
                    let mut else_scope = scope.clone();
                    collect_float_vars_from_stmts(
                        else_stmts,
                        &mut else_scope,
                        known_floats,
                        is_top_level,
                    );
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } | Stmt::For { body, .. } => {
                let mut loop_scope = scope.clone();
                collect_float_vars_from_stmts(body, &mut loop_scope, known_floats, is_top_level);
            }
            Stmt::ForEach {
                var,
                value_var,
                iterable,
                body,
            } => {
                let mut loop_scope = scope.clone();
                let target_var = value_var.as_ref().unwrap_or(var);
                if expr_is_float_array(iterable, scope) {
                    loop_scope.insert(target_var.clone());
                    if is_top_level {
                        known_floats.insert(target_var.clone());
                    }
                }
                collect_float_vars_from_stmts(body, &mut loop_scope, known_floats, is_top_level);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                let mut try_scope = scope.clone();
                collect_float_vars_from_stmts(
                    try_block,
                    &mut try_scope,
                    known_floats,
                    is_top_level,
                );
                let mut catch_scope = scope.clone();
                collect_float_vars_from_stmts(
                    catch_block,
                    &mut catch_scope,
                    known_floats,
                    is_top_level,
                );
                if let Some(finally_block) = finally_block {
                    let mut finally_scope = scope.clone();
                    collect_float_vars_from_stmts(
                        finally_block,
                        &mut finally_scope,
                        known_floats,
                        is_top_level,
                    );
                }
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                let mut fn_locals = scope.clone();
                for (idx, param) in params.iter().enumerate() {
                    if known_floats.contains(&format!("fn_param_flt:{}:{}", name, idx))
                        || known_floats.contains(&format!("fn_param_flt:{}:{}", bare, idx))
                    {
                        fn_locals.insert(param.clone());
                    }
                    if known_floats.contains(&format!("fn_param_flt_arr:{}:{}", name, idx))
                        || known_floats.contains(&format!("fn_param_flt_arr:{}:{}", bare, idx))
                    {
                        fn_locals.insert(format!("arr_is_flt:{}", param));
                    }
                }
                collect_float_vars_from_stmts(body, &mut fn_locals, known_floats, false);
                collect_tuple_returns_float(body, &fn_locals, name, known_floats);
                collect_tuple_returns_float(body, &fn_locals, bare, known_floats);
                if stmts_return_float(body, &fn_locals) {
                    known_floats.insert(format!("fn_ret_flt:{}", name));
                    known_floats.insert(format!("fn_ret_flt:{}", bare));
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_float_vars_from_stmts(
                    std::slice::from_ref(inner),
                    scope,
                    known_floats,
                    is_top_level,
                );
            }
            _ => {}
        }
    }
}

pub fn collect_known_float_vars(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_float_vars_with_index(program, &call_index)
}

pub fn collect_known_float_vars_with_index(
    program: &Program,
    call_index: &CallIndex,
) -> HashSet<String> {
    let mut known_floats = HashSet::new();
    for stmt in &program.statements {
        let stmt = stmt.inner_stmt();
        if let Stmt::ExternBlock { functions, .. } = stmt {
            for f in functions {
                let bare = f.name.rsplit("::").next().unwrap_or(&f.name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                if let Some(ret) = &f.return_type {
                    let ret_trimmed = ret.trim();
                    if ret_trimmed == "float" || ret_trimmed == "f64" || ret_trimmed == "f32" {
                        known_floats.insert(format!("fn_ret_flt:{}", f.name));
                        if bare != f.name {
                            known_floats.insert(format!("fn_ret_flt:{}", bare));
                        }
                    }
                }
            }
        } else if let Stmt::Function {
            name,
            body,
            param_types,
            return_type,
            ..
        } = stmt
        {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if let Some(ret) = return_type {
                let ret_trimmed = ret.trim();
                if ret_trimmed == "float" || ret_trimmed == "f64" || ret_trimmed == "f32" {
                    known_floats.insert(format!("fn_ret_flt:{}", name));
                    known_floats.insert(format!("fn_ret_flt:{}", bare));
                } else if ret_trimmed.starts_with('(') && ret_trimmed.ends_with(')') {
                    for (i, ty) in ret_trimmed[1..ret_trimmed.len() - 1].split(',').enumerate() {
                        let ty = ty.trim();
                        if ty == "float" || ty == "f64" || ty == "f32" {
                            known_floats.insert(format!("fn_ret_tuple_flt:{}:{}", name, i));
                            known_floats.insert(format!("fn_ret_tuple_flt:{}:{}", bare, i));
                        }
                    }
                }
            }
            let kf_snapshot = known_floats.clone();
            collect_tuple_returns_float(body, &kf_snapshot, name, &mut known_floats);
            collect_tuple_returns_float(body, &kf_snapshot, bare, &mut known_floats);
            for (idx, p_type) in param_types.iter().enumerate() {
                if let Some(pt) = p_type {
                    if pt == "float" || pt == "f64" || pt == "f32" {
                        known_floats.insert(format!("fn_param_flt:{}:{}", name, idx));
                        known_floats.insert(format!("fn_param_flt:{}:{}", bare, idx));
                    } else if pt == "float[]" || pt == "f64[]" || pt == "f32[]" {
                        known_floats.insert(format!("fn_param_flt_arr:{}:{}", name, idx));
                        known_floats.insert(format!("fn_param_flt_arr:{}:{}", bare, idx));
                    }
                }
            }
        } else if let Stmt::StructDef {
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
                    if t == "float" || t == "f64" || t == "f32" {
                        known_floats.insert(format!("struct_field_flt:{}.{}", name, f));
                        known_floats.insert(format!("struct_field_flt:{}.{}", bare, f));
                        known_floats.insert(format!("struct_field_flt:{}", f));
                    } else if t == "float[]" || t == "f64[]" || t == "f32[]" {
                        known_floats.insert(format!("struct_field_arr_flt:{}.{}", name, f));
                        known_floats.insert(format!("struct_field_arr_flt:{}.{}", bare, f));
                        known_floats.insert(format!("struct_field_arr_flt:{}", f));
                    }
                }
            }
        }
    }
    let mut funcs = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
    for _ in 0..5 {
        let prev_len = known_floats.len();
        let mut scope = known_floats.clone();
        collect_float_vars_from_stmts(&program.statements, &mut scope, &mut known_floats, true);
        for (name, params, _, _) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (idx, _param) in params.iter().enumerate() {
                if !known_floats.contains(&format!("fn_param_flt:{}:{}", name, idx))
                    && !known_floats.contains(&format!("fn_param_flt:{}:{}", bare, idx))
                {
                    let mut call_args = Vec::new();
                    call_index.collect_all_call_args(name, bare, idx, &mut call_args);
                    if !call_args.is_empty()
                        && call_args
                            .iter()
                            .all(|arg| expr_is_definitely_float(arg, &known_floats))
                    {
                        known_floats.insert(format!("fn_param_flt:{}:{}", name, idx));
                        known_floats.insert(format!("fn_param_flt:{}:{}", bare, idx));
                    }
                }
                if !known_floats.contains(&format!("fn_param_flt_arr:{}:{}", name, idx))
                    && !known_floats.contains(&format!("fn_param_flt_arr:{}:{}", bare, idx))
                {
                    let mut call_args = Vec::new();
                    call_index.collect_all_call_args(name, bare, idx, &mut call_args);
                    if !call_args.is_empty()
                        && call_args
                            .iter()
                            .all(|arg| expr_is_float_array(arg, &known_floats))
                    {
                        known_floats.insert(format!("fn_param_flt_arr:{}:{}", name, idx));
                        known_floats.insert(format!("fn_param_flt_arr:{}:{}", bare, idx));
                    }
                }
            }
        }
        if known_floats.len() == prev_len {
            break;
        }
    }
    known_floats
}

pub fn infer_param_is_float_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_floats: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    known_floats.contains(&format!("fn_param_flt:{}:{}", func_name, param_idx))
        || known_floats.contains(&format!("fn_param_flt:{}:{}", bare, param_idx))
}

pub fn infer_param_is_float_array_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_floats: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    known_floats.contains(&format!("fn_param_flt_arr:{}:{}", func_name, param_idx))
        || known_floats.contains(&format!("fn_param_flt_arr:{}:{}", bare, param_idx))
}

pub fn infer_param_is_float(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_floats = collect_known_float_vars(program);
    infer_param_is_float_with(func_name, param_idx, program, &known_floats)
}

pub fn infer_param_is_float_array(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_floats = collect_known_float_vars(program);
    infer_param_is_float_array_with(func_name, param_idx, program, &known_floats)
}
