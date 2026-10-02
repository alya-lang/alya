use crate::ast::*;
use crate::codegen::analysis::inference::common::collect_function_defs;
use crate::codegen::analysis::traversal::CallIndex;
use std::collections::HashSet;

fn expr_is_definitely_float(expr: &Expr, known_floats: &HashSet<String>) -> bool {
    match expr {
        Expr::Float(_) => true,
        Expr::Number(_) => false,
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
                name,
                params,
                body,
                return_type,
                ..
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
                // An explicit integer return annotation vetoes body-based
                // float marking: integer SIMD methods (e.g. `-> int`) share
                // call shapes with float code, and the body inference alone
                // cannot tell them apart.
                let annotated_int = return_type.as_deref().is_some_and(|rt| {
                    let base = rt.rsplit("::").next().unwrap_or(rt);
                    let base = base.rsplit("__").next().unwrap_or(base);
                    matches!(
                        base.trim(),
                        "int"
                            | "i64"
                            | "isize"
                            | "uint"
                            | "u64"
                            | "usize"
                            | "i32"
                            | "i16"
                            | "i8"
                            | "u32"
                            | "u16"
                            | "u8"
                            | "byte"
                            | "bool"
                    )
                });
                if !annotated_int && stmts_return_float(body, &fn_locals) {
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
                    // Provenance: an explicit `-> float` annotation is
                    // enforced by the type checker, so these returns are
                    // exclusively float (unlike inference markers, which
                    // fire when ANY branch returns float).
                    known_floats.insert(format!("fn_ret_flt_ann:{}", name));
                    known_floats.insert(format!("fn_ret_flt_ann:{}", bare));
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
    // alya-lang/alya#50: push-built float arrays. Runs after the main
    // fixpoint with converged knowledge; rounds resolve builder-return
    // chains (`let row = build()` where `build` itself calls builders).
    for _ in 0..5 {
        let prev_len = known_floats.len();
        collect_push_float_program(program, &known_floats.clone(), &mut known_floats);
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

// alya-lang/alya#50: push-built float arrays.
//
// Array literals earn `arr_is_flt` from their first element, but arrays
// assembled with `push` never did, so untyped reads of their float slots
// returned raw f64 bit patterns. This pass promotes arrays whose locally
// visible sources are all float (literal inits, float pushes, float-array
// aliases, builder returns) to `arr_is_flt`. Safety rails (each degrades
// to today's behavior, never to a new misread):
// - any int-literal or unknown source vetoes the name globally;
// - a push to an array with no visible local init (params, globals,
//   aliases) disables push marking program-wide;
// - commits happen once per round from aggregated evidence, so statement
//   order within a walk cannot strand a stale mark.
// `fn_ret_arr_flt` propagates builder returns to caller bindings,
// mirroring `fn_ret_tuple_flt`. Mixed arrays keep int-biased reads;
// true mixed-type discrimination needs runtime tags (#39).
#[derive(Default)]
struct PushArrayState {
    init: HashSet<String>,
    veto: HashSet<String>,
    float_seen: HashSet<String>,
    /// A push to an array with no visible local init whose value is not
    /// provably float (params, globals, aliases could hold anything).
    paranoid: bool,
}

impl PushArrayState {
    fn clean(&self, name: &str) -> bool {
        self.init.contains(name) && self.float_seen.contains(name) && !self.veto.contains(name)
    }
}

fn push_call_target(args: &[Expr]) -> Option<(&str, &Expr)> {
    if args.len() != 2 {
        return None;
    }
    match &args[0] {
        Expr::Identifier(arr) => Some((arr.as_str(), &args[1])),
        _ => None,
    }
}

fn note_push_sources(arr: &str, value: &Expr, scope: &HashSet<String>, state: &mut PushArrayState) {
    // A read of a clean array is itself float evidence (`e.push(d[0])`).
    if let Expr::Index { array, .. } = value {
        if let Expr::Identifier(src) = &**array {
            if state.clean(src) {
                state.float_seen.insert(arr.to_string());
                return;
            }
        }
    }
    if expr_is_definitely_float(value, scope) {
        state.float_seen.insert(arr.to_string());
    } else {
        // Integer literals are definite-int evidence; anything else is
        // unprovable either way. Both veto: the former proves mixed
        // content, the latter cannot prove float-only.
        state.veto.insert(arr.to_string());
    }
}

fn note_array_binding(
    name: &str,
    value: &Expr,
    scope: &mut HashSet<String>,
    state: &mut PushArrayState,
    snapshot: &HashSet<String>,
) {
    match value {
        Expr::Array(elems) => {
            state.init.insert(name.to_string());
            for elem in elems {
                if expr_is_definitely_float(elem, scope) {
                    state.float_seen.insert(name.to_string());
                } else {
                    state.veto.insert(name.to_string());
                }
            }
        }
        Expr::Call { name: cname, .. } => {
            let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if snapshot.contains(&format!("fn_ret_arr_flt:{}", cname))
                || snapshot.contains(&format!("fn_ret_arr_flt:{}", bare))
            {
                state.init.insert(name.to_string());
                state.float_seen.insert(name.to_string());
                scope.insert(format!("arr_is_flt:{}", name));
            } else if state.init.contains(name) {
                // Rebinding an array to unknown content drops the
                // source set. Fresh unknown bindings stay neutral:
                // a later init or float-array alias can still claim
                // the name, and pushes hit the no-init rule below.
                state.veto.insert(name.to_string());
            }
        }
        Expr::Identifier(src) => {
            if scope.contains(&format!("arr_is_flt:{}", src))
                || snapshot.contains(&format!("arr_is_flt:{}", src))
                || state.clean(src)
            {
                state.init.insert(name.to_string());
                state.float_seen.insert(name.to_string());
                scope.insert(format!("arr_is_flt:{}", name));
            } else if state.init.contains(name) {
                state.veto.insert(name.to_string());
            }
        }
        _ => {
            if state.init.contains(name) {
                state.veto.insert(name.to_string());
            }
        }
    }
}

fn collect_push_array_returns(
    stmts: &[Stmt],
    state: &PushArrayState,
    fn_name: &str,
    bare: &str,
    out: &mut HashSet<String>,
) {
    for s in stmts {
        match s {
            Stmt::Return(Some(Expr::Identifier(ret))) if state.clean(ret) => {
                out.insert(format!("fn_ret_arr_flt:{}", fn_name));
                out.insert(format!("fn_ret_arr_flt:{}", bare));
            }
            Stmt::Return(_) => {}
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_push_array_returns(then_block, state, fn_name, bare, out);
                if let Some(eb) = else_block {
                    collect_push_array_returns(eb, state, fn_name, bare, out);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_push_array_returns(body, state, fn_name, bare, out);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_push_array_returns(try_block, state, fn_name, bare, out);
                collect_push_array_returns(catch_block, state, fn_name, bare, out);
                if let Some(fb) = finally_block {
                    collect_push_array_returns(fb, state, fn_name, bare, out);
                }
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_push_array_returns(std::slice::from_ref(inner), state, fn_name, bare, out);
            }
            _ => {}
        }
    }
}

fn handle_push_stmt(expr: &Expr, scope: &HashSet<String>, state: &mut PushArrayState) {
    let Expr::Call { name, args } = expr else {
        return;
    };
    let bare = name.rsplit("::").next().unwrap_or(name.as_str());
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    if bare != "push" {
        return;
    }
    let Some((arr, value)) = push_call_target(args) else {
        return;
    };
    if state.init.contains(arr) {
        note_push_sources(arr, value, scope, state);
    } else if !expr_is_definitely_float(value, scope) {
        // No visible local init: params, globals, or aliases the walker
        // cannot see into. A provably-float value is consistent with
        // float reads and needs no action; anything else disables push
        // marking for this round rather than risking a misread.
        state.paranoid = true;
    }
}

fn collect_push_float_from_stmts(
    stmts: &[Stmt],
    scope: &mut HashSet<String>,
    state: &mut PushArrayState,
    snapshot: &HashSet<String>,
    out: &mut HashSet<String>,
    fn_states: &mut Vec<PushArrayState>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, value, .. } => {
                note_array_binding(name, value, scope, state, snapshot);
            }
            Stmt::Assign { name, value } => {
                note_array_binding(name, value, scope, state, snapshot);
            }
            Stmt::Expr(expr) => {
                handle_push_stmt(expr, scope, state);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                let mut then_scope = scope.clone();
                collect_push_float_from_stmts(
                    then_block,
                    &mut then_scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
                if let Some(else_stmts) = else_block {
                    let mut else_scope = scope.clone();
                    collect_push_float_from_stmts(
                        else_stmts,
                        &mut else_scope,
                        state,
                        snapshot,
                        out,
                        fn_states,
                    );
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } | Stmt::For { body, .. } => {
                let mut loop_scope = scope.clone();
                collect_push_float_from_stmts(
                    body,
                    &mut loop_scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
            }
            Stmt::ForEach {
                var,
                value_var,
                iterable,
                body,
            } => {
                let mut loop_scope = scope.clone();
                let target_var = value_var.as_ref().unwrap_or(var);
                let iterable_clean = match iterable {
                    Expr::Identifier(n) => state.clean(n),
                    _ => false,
                };
                if expr_is_float_array(iterable, scope) || iterable_clean {
                    loop_scope.insert(target_var.clone());
                }
                collect_push_float_from_stmts(
                    body,
                    &mut loop_scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                let mut try_scope = scope.clone();
                collect_push_float_from_stmts(
                    try_block,
                    &mut try_scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
                let mut catch_scope = scope.clone();
                collect_push_float_from_stmts(
                    catch_block,
                    &mut catch_scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
                if let Some(finally_block) = finally_block {
                    let mut finally_scope = scope.clone();
                    collect_push_float_from_stmts(
                        finally_block,
                        &mut finally_scope,
                        state,
                        snapshot,
                        out,
                        fn_states,
                    );
                }
            }
            Stmt::Function {
                name, params, body, ..
            } => {
                let bare = name.rsplit("::").next().unwrap_or(name.as_str());
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                let mut fn_scope = HashSet::new();
                for (idx, param) in params.iter().enumerate() {
                    if snapshot.contains(&format!("fn_param_flt:{}:{}", name, idx))
                        || snapshot.contains(&format!("fn_param_flt:{}:{}", bare, idx))
                    {
                        fn_scope.insert(param.clone());
                    }
                    if snapshot.contains(&format!("fn_param_flt_arr:{}:{}", name, idx))
                        || snapshot.contains(&format!("fn_param_flt_arr:{}:{}", bare, idx))
                    {
                        fn_scope.insert(format!("arr_is_flt:{}", param));
                    }
                }
                let mut fn_state = PushArrayState::default();
                collect_push_float_from_stmts(
                    body,
                    &mut fn_scope,
                    &mut fn_state,
                    snapshot,
                    out,
                    fn_states,
                );
                collect_push_array_returns(body, &fn_state, name, bare, out);
                fn_states.push(fn_state);
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_push_float_from_stmts(
                    std::slice::from_ref(inner),
                    scope,
                    state,
                    snapshot,
                    out,
                    fn_states,
                );
            }
            _ => {}
        }
    }
}

/// Commits one round's aggregated evidence. Vetoes are round-fresh on
/// purpose: a value unclassified in an early round may prove float in
/// a later one (call-site param inference runs inside the same loop),
/// and only int literals (which never change their mind) plus values
/// still unknown at commit time block a mark. Any veto of a name in
/// any walk blocks its marking; any naked non-float push disables
/// marking for the round. Arrays already float-marked by literal
/// evidence are left alone.
fn commit_push_round(
    states: &[PushArrayState],
    snapshot: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    let mut vetoed: HashSet<&str> = HashSet::new();
    let mut paranoid = false;
    for state in states {
        for n in &state.veto {
            vetoed.insert(n.as_str());
        }
        paranoid = paranoid || state.paranoid;
    }
    if paranoid {
        return;
    }
    for state in states {
        for n in state.init.iter().chain(state.float_seen.iter()) {
            let marked = out.contains(&format!("arr_is_flt:{}", n))
                || snapshot.contains(&format!("arr_is_flt:{}", n));
            if state.clean(n) && !vetoed.contains(n.as_str()) && !marked {
                out.insert(format!("arr_is_flt:{}", n));
            }
        }
    }
}

fn collect_push_float_program(
    program: &Program,
    snapshot: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    let mut scope = snapshot.clone();
    let mut top_state = PushArrayState::default();
    let mut fn_states: Vec<PushArrayState> = Vec::new();
    collect_push_float_from_stmts(
        &program.statements,
        &mut scope,
        &mut top_state,
        snapshot,
        out,
        &mut fn_states,
    );
    let mut states = vec![top_state];
    states.extend(fn_states);
    commit_push_round(&states, snapshot, out);
}
