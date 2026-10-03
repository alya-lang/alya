use super::common::{collect_function_defs, count_assignments};
use crate::ast::*;
use crate::codegen::analysis::traversal::CallIndex;
use crate::parser::dynspec::{dynspec_codes, spec_open_param_kind};
use std::collections::{HashMap, HashSet};

fn expr_is_definitely_map(
    expr: &Expr,
    fn_scope: Option<&str>,
    known_maps: &HashSet<String>,
) -> bool {
    match expr {
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "map"
                    | "set_new"
                    | "set_from_array"
                    | "set_union"
                    | "set_intersection"
                    | "set_difference"
                    | "map_clone"
                    | "map_merge"
                    | "map_from_entries" // NOTE: `json_parse` is deliberately absent: it forwards
                                         // to the dynamic JSON parser (any value kind), so a static
                                         // map claim miscompiles array/string results.
            ) || known_maps.contains(&format!("fn_ret_map:{}", name))
                || known_maps.contains(&format!("fn_ret_map:{}", bare))
        }
        Expr::Map(_) => true,
        Expr::Identifier(name) => {
            if let Some(scope) = fn_scope {
                let bare_scope = scope.rsplit("::").next().unwrap_or(scope);
                let bare_scope = bare_scope.rsplit("__").next().unwrap_or(bare_scope);
                known_maps.contains(&format!("{}:{}", scope, name))
                    || known_maps.contains(&format!("{}:{}", bare_scope, name))
            } else {
                known_maps.contains(name)
            }
        }
        Expr::Index { array, index } => {
            if let Expr::String(field) = &**index {
                if known_maps.contains(&format!("map_field_map:{}", field)) {
                    return true;
                }
            }
            if let (Expr::Identifier(map_name), Expr::String(field)) = (&**array, &**index) {
                if let Some(scope) = fn_scope {
                    if known_maps.contains(&format!("{}:{}.{}", scope, map_name, field)) {
                        return true;
                    }
                }
                if known_maps.contains(&format!("map_map:{}.{}", map_name, field)) {
                    return true;
                }
            }
            false
        }
        Expr::ForceUnwrap(inner) => expr_is_definitely_map(inner, fn_scope, known_maps),
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

fn stmts_return_map(stmts: &[Stmt], fn_scope: Option<&str>, known_maps: &HashSet<String>) -> bool {
    let mut returns = Vec::new();
    collect_return_exprs(stmts, &mut returns);
    let non_null: Vec<_> = returns
        .into_iter()
        .filter(|e| !matches!(e, Expr::Null))
        .collect();
    !non_null.is_empty()
        && non_null
            .iter()
            .all(|e| expr_is_definitely_map(e, fn_scope, known_maps))
}

fn collect_map_vars_from_stmts(
    stmts: &[Stmt],
    fn_scope: Option<&str>,
    known_maps: &mut HashSet<String>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, value, .. } | Stmt::Assign { name, value, .. } => {
                if expr_is_definitely_map(value, fn_scope, known_maps) {
                    if let Some(scope) = fn_scope {
                        known_maps.insert(format!("{}:{}", scope, name));
                    } else {
                        known_maps.insert(name.clone());
                    }
                }
                if let Expr::Map(entries) = value {
                    for (k, v) in entries {
                        if expr_is_definitely_map(v, fn_scope, known_maps) {
                            if let Expr::String(field) = k {
                                known_maps.insert(format!("map_field_map:{}", field));
                                if let Some(scope) = fn_scope {
                                    known_maps.insert(format!("{}:{}.{}", scope, name, field));
                                }
                                known_maps.insert(format!("map_map:{}.{}", name, field));
                            }
                        }
                    }
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_map_vars_from_stmts(try_block, fn_scope, known_maps);
                collect_map_vars_from_stmts(catch_block, fn_scope, known_maps);
                if let Some(finally_block) = finally_block {
                    collect_map_vars_from_stmts(finally_block, fn_scope, known_maps);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_map_vars_from_stmts(then_block, fn_scope, known_maps);
                if let Some(else_stmts) = else_block {
                    collect_map_vars_from_stmts(else_stmts, fn_scope, known_maps);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_map_vars_from_stmts(body, fn_scope, known_maps);
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
                    let is_map_type =
                        param_types
                            .get(idx)
                            .and_then(|t| t.as_deref())
                            .is_some_and(|t| {
                                t == "map"
                                    || t.starts_with("map[")
                                    || (t.starts_with('[') && t.contains(':') && t.ends_with(']'))
                            });
                    if is_map_type
                        || known_maps.contains(&format!("fn_param_map:{}:{}", name, idx))
                        || known_maps.contains(&format!("fn_param_map:{}:{}", bare, idx))
                    {
                        if is_map_type {
                            known_maps.insert(format!("fn_param_map:{}:{}", name, idx));
                            known_maps.insert(format!("fn_param_map:{}:{}", bare, idx));
                        }
                        known_maps.insert(format!("{}:{}", name, param));
                        if bare != name {
                            known_maps.insert(format!("{}:{}", bare, param));
                        }
                    }
                }
                collect_map_vars_from_stmts(body, Some(name), known_maps);
                if bare != name {
                    collect_map_vars_from_stmts(body, Some(bare), known_maps);
                }
            }
            _ => {}
        }
    }
}

#[allow(dead_code)]
fn expr_is_definitely_non_map(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Array(_) => true,
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            matches!(
                bare,
                "split"
                    | "args"
                    | "keys"
                    | "values"
                    | "lines"
                    | "read_lines"
                    | "array_slice"
                    | "array_clone"
                    | "array_concat"
                    | "len"
                    | "str"
            )
        }
        Expr::Binary { .. } | Expr::Unary { .. } => true,
        Expr::ForceUnwrap(inner) => expr_is_definitely_non_map(inner),
        _ => false,
    }
}

pub fn collect_known_map_vars(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_map_vars_with_index(program, &call_index)
}

pub fn collect_known_map_vars_with_index(
    program: &Program,
    call_index: &CallIndex,
) -> HashSet<String> {
    let mut known_maps = HashSet::new();
    let mut funcs = Vec::new();
    collect_function_defs(&program.statements, &mut funcs);
    for _ in 0..5 {
        let prev_len = known_maps.len();
        collect_map_vars_from_stmts(&program.statements, None, &mut known_maps);
        for (name, params, param_types, body) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if stmts_return_map(body, Some(name), &known_maps)
                || (bare != *name && stmts_return_map(body, Some(bare), &known_maps))
            {
                known_maps.insert(format!("fn_ret_map:{}", name));
                known_maps.insert(format!("fn_ret_map:{}", bare));
            }
            for (idx, _param) in params.iter().enumerate() {
                if let Some(t) = param_types.get(idx).and_then(|t| t.as_deref()) {
                    if t == "map"
                        || t.starts_with("map[")
                        || (t.starts_with('[') && t.contains(':') && t.ends_with(']'))
                    {
                        known_maps.insert(format!("fn_param_map:{}:{}", name, idx));
                        known_maps.insert(format!("fn_param_map:{}:{}", bare, idx));
                        continue;
                    }
                }
                if known_maps.contains(&format!("fn_param_map:{}:{}", name, idx))
                    || known_maps.contains(&format!("fn_param_map:{}:{}", bare, idx))
                {
                    continue;
                }
                let mut call_args = Vec::new();
                call_index.collect_all_call_args_scoped(name, bare, idx, &mut call_args);
                if !call_args.is_empty()
                    && call_args.iter().any(|(caller_scope, arg)| {
                        expr_is_definitely_map(arg, *caller_scope, &known_maps)
                    })
                    && call_args.iter().all(|(caller_scope, arg)| {
                        matches!(arg, Expr::Null)
                            || expr_is_definitely_map(arg, *caller_scope, &known_maps)
                    })
                {
                    known_maps.insert(format!("fn_param_map:{}:{}", name, idx));
                    known_maps.insert(format!("fn_param_map:{}:{}", bare, idx));
                }
            }
        }
        if known_maps.len() == prev_len {
            break;
        }
    }
    known_maps
}

pub fn infer_param_is_map_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    known_maps: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    known_maps.contains(&format!("fn_param_map:{}:{}", func_name, param_idx))
        || known_maps.contains(&format!("fn_param_map:{}:{}", bare, param_idx))
}

pub fn infer_param_is_map(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let known_maps = collect_known_map_vars(program);
    infer_param_is_map_with(func_name, param_idx, program, &known_maps)
}

// ═══════════════════════════════════════════════════════════════
// Strict (must-) map facts for `is map` folding (alya-lang/alya#70).
//
// Same contract as the array strict-set: only exact evidence
// (enforced annotations, single-assignment literal aliases,
// structural calls, dynspec suffix codes). In particular `null`
// never counts as a map: the may-rule admits it for retain safety,
// but `is map` of `null` is false at runtime.
// ═══════════════════════════════════════════════════════════════

type FuncDef<'a> = (&'a str, &'a [String], &'a [Option<String>], &'a [Stmt]);

/// Resolves one identifier to its dynspec suffix kind inside a clone
/// scope (`None` for generic scopes). Exact: clones only run through
/// classified call sites (same contract as the codegen decoder).
fn spec_scope_kind_map(scope: &str, param: &str, funcs: &[FuncDef]) -> Option<char> {
    if let Some((_, params, param_types, _)) = funcs.iter().find(|(n, _, _, _)| *n == scope) {
        if let Some(k) = spec_open_param_kind(scope, param, params, param_types) {
            return Some(k);
        }
    }
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

/// Exact counterpart of `expr_is_definitely_map`: identifiers resolve
/// only through strict facts (never the may-set) or the dynspec
/// suffix codes.
fn expr_is_definitely_map_strict(
    expr: &Expr,
    fn_scope: Option<&str>,
    strict: &HashSet<String>,
    funcs: &[FuncDef],
) -> bool {
    if expr_is_definitely_map(expr, fn_scope, strict) {
        return true;
    }
    if let (Expr::Identifier(name), Some(scope)) = (expr, fn_scope) {
        return spec_scope_kind_map(scope, name, funcs) == Some('m');
    }
    false
}

fn strict_scope_counts_map<'a>(
    fn_scope: Option<&str>,
    counts: &'a HashMap<String, HashMap<String, usize>>,
) -> Option<&'a HashMap<String, usize>> {
    let scope = fn_scope.unwrap_or("");
    if let Some(c) = counts.get(scope) {
        return Some(c);
    }
    let origin = scope.split("__spk__").next().unwrap_or(scope);
    counts.get(origin)
}

fn is_map_annotation(t: &str) -> bool {
    t == "map"
        || t.starts_with("map[")
        || (t.starts_with('[') && t.contains(':') && t.ends_with(']'))
}

fn collect_strict_map_vars_from_stmts(
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
                // Single-assignment only (the may-rule has no such
                // guard): a later reassignment could swap in a non-map.
                let single = strict_scope_counts_map(fn_scope, counts)
                    .and_then(|c| c.get(name))
                    .is_some_and(|n| *n == 1);
                if single
                    && (type_ann.as_ref().is_some_and(|t| is_map_annotation(t))
                        || expr_is_definitely_map_strict(value, fn_scope, strict, funcs))
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
                collect_strict_map_vars_from_stmts(try_block, fn_scope, counts, strict, funcs);
                collect_strict_map_vars_from_stmts(catch_block, fn_scope, counts, strict, funcs);
                if let Some(fb) = finally_block {
                    collect_strict_map_vars_from_stmts(fb, fn_scope, counts, strict, funcs);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_strict_map_vars_from_stmts(then_block, fn_scope, counts, strict, funcs);
                if let Some(eb) = else_block {
                    collect_strict_map_vars_from_stmts(eb, fn_scope, counts, strict, funcs);
                }
            }
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. } => {
                collect_strict_map_vars_from_stmts(body, fn_scope, counts, strict, funcs);
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
                    let annotated = param_types
                        .get(idx)
                        .and_then(|t| t.as_deref())
                        .is_some_and(is_map_annotation);
                    if annotated
                        || strict.contains(&format!("fn_param_map:{}:{}", name, idx))
                        || strict.contains(&format!("fn_param_map:{}:{}", bare, idx))
                    {
                        strict.insert(format!("{}:{}", name, param));
                        if !name.contains("__spk__") && bare != name {
                            strict.insert(format!("{}:{}", bare, param));
                        }
                    }
                }
                collect_strict_map_vars_from_stmts(body, Some(name), counts, strict, funcs);
            }
            Stmt::Pub(inner) | Stmt::Defer(inner) => {
                collect_strict_map_vars_from_stmts(
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

pub fn collect_known_map_vars_strict_with_index(
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
    // Seed exact per-clone param kinds from the dynspec suffix contract.
    for (name, params, param_types, _) in &funcs {
        if let Some(codes) = dynspec_codes(name) {
            let mut open_idx = 0usize;
            for (p, t) in params.iter().zip(param_types.iter()) {
                if t.is_none() && p != "self" {
                    if codes.get(open_idx) == Some(&'m') {
                        strict.insert(format!("{}:{}", name, p));
                    }
                    open_idx += 1;
                }
            }
        }
    }
    for _ in 0..5 {
        let prev_len = strict.len();
        collect_strict_map_vars_from_stmts(&program.statements, None, &counts, &mut strict, &funcs);
        for (name, params, param_types, _) in &funcs {
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            for (idx, param) in params.iter().enumerate() {
                if param_types
                    .get(idx)
                    .and_then(|t| t.as_deref())
                    .is_some_and(is_map_annotation)
                {
                    strict.insert(format!("fn_param_map:{}:{}", name, idx));
                    if !name.contains("__spk__") {
                        strict.insert(format!("fn_param_map:{}:{}", bare, idx));
                    }
                    strict.insert(format!("{}:{}", name, param));
                    if !name.contains("__spk__") && bare != *name {
                        strict.insert(format!("{}:{}", bare, param));
                    }
                    continue;
                }
                if strict.contains(&format!("fn_param_map:{}:{}", name, idx))
                    || strict.contains(&format!("fn_param_map:{}:{}", bare, idx))
                {
                    continue;
                }
                let mut call_args = Vec::new();
                call_index.collect_all_call_args_scoped(name, bare, idx, &mut call_args);
                // Universal: every caller must pass a map. Unknowns and
                // (unlike the may-rule) null block the proof.
                if !call_args.is_empty()
                    && call_args.iter().all(|(caller_scope, arg)| {
                        expr_is_definitely_map_strict(arg, *caller_scope, &strict, &funcs)
                    })
                {
                    strict.insert(format!("fn_param_map:{}:{}", name, idx));
                    if !name.contains("__spk__") {
                        strict.insert(format!("fn_param_map:{}:{}", bare, idx));
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

pub fn collect_known_map_vars_strict(program: &Program) -> HashSet<String> {
    let call_index = CallIndex::build(&program.statements);
    collect_known_map_vars_strict_with_index(program, &call_index)
}

pub fn infer_param_is_map_strict_with(
    func_name: &str,
    param_idx: usize,
    _program: &Program,
    strict: &HashSet<String>,
) -> bool {
    let bare = func_name.rsplit("::").next().unwrap_or(func_name);
    let bare = bare.rsplit("__").next().unwrap_or(bare);
    strict.contains(&format!("fn_param_map:{}:{}", func_name, param_idx))
        || strict.contains(&format!("fn_param_map:{}:{}", bare, param_idx))
}

pub fn infer_param_is_map_strict(func_name: &str, param_idx: usize, program: &Program) -> bool {
    let strict = collect_known_map_vars_strict(program);
    infer_param_is_map_strict_with(func_name, param_idx, program, &strict)
}
