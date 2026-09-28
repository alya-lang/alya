//! Static call-site specialization for dynamic type tests
//! (alya-lang/alya#39 Phase 2).
//!
//! A function whose untyped parameters are tested with `is` (parsed as
//! `Expr::TypeCheck`) cannot serve every call site with one static type:
//! #14 proved no single static rule correct. This pass clones such
//! functions per call-site kind signature (`{fn}__spk__{codes}`, codes
//! below) and rewrites calls whose untyped arguments are all statically
//! classifiable (literals in this phase). All other call sites keep
//! calling the generic version, whose behavior is unchanged.
//!
//! Clones keep untyped params (no new TypeErrors possible); codegen
//! decodes the suffix and presets the parameter value types, mirroring
//! explicit annotations. Test discovery ignores `__`-mangled names, so
//! clones never double-run suites. Nested function definitions are not
//! specialized (hoisting would break captures); generic templates are
//! left to `resolve_generics`, which runs before this pass.
use crate::ast::{Expr, Program, Stmt};
use std::collections::{HashMap, HashSet};

const MAX_VERSIONS_PER_FN: usize = 8;
const MAX_ROUNDS: usize = 5;

/// Kind codes in `__spk__` suffixes. Single static alphabet shared with
/// the codegen decoder (`crate::codegen` reads these back).
fn arg_kind_code(arg: &Expr) -> Option<char> {
    match arg {
        Expr::Number(_) => Some('i'),
        Expr::Float(_) => Some('f'),
        Expr::String(_) => Some('s'),
        Expr::Array(_) => Some('a'),
        Expr::Map(_) => Some('m'),
        _ => None,
    }
}

/// Decode a specialization suffix. Returns the per-param kind codes, or
/// `None` when `name` is not a specialization this pass could have made.
pub fn dynspec_codes(fn_name: &str) -> Option<Vec<char>> {
    let (_, suffix) = fn_name.rsplit_once("__spk__")?;
    if suffix.is_empty()
        || !suffix
            .chars()
            .all(|c| matches!(c, 'i' | 'f' | 's' | 'a' | 'm'))
    {
        return None;
    }
    Some(suffix.chars().collect())
}

fn expr_tests_params(expr: &Expr, params: &HashSet<String>) -> bool {
    match expr {
        Expr::TypeCheck { expr: inner, .. } => {
            if let Expr::Identifier(name) = &**inner {
                if params.contains(name) {
                    return true;
                }
            }
            expr_tests_params(inner, params)
        }
        Expr::Binary { left, right, .. } => {
            expr_tests_params(left, params) || expr_tests_params(right, params)
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            expr_tests_params(inner, params)
        }
        Expr::Call { args, .. } | Expr::Array(args) | Expr::InterpolatedString(args) => {
            args.iter().any(|a| expr_tests_params(a, params))
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            expr_tests_params(array, params) || expr_tests_params(index, params)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            expr_tests_params(object, params)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_tests_params(condition, params)
                || expr_tests_params(then_branch, params)
                || expr_tests_params(else_branch, params)
        }
        Expr::NullCoalesce { value, default } => {
            expr_tests_params(value, params) || expr_tests_params(default, params)
        }
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_tests_params(v, params)),
        Expr::Map(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_tests_params(k, params) || expr_tests_params(v, params)),
        Expr::Cast { expr: inner, .. } => expr_tests_params(inner, params),
        Expr::OptionalCall { args, .. } => args.iter().any(|a| expr_tests_params(a, params)),
        _ => false,
    }
}

fn stmts_test_params(stmts: &[Stmt], params: &HashSet<String>) -> bool {
    stmts.iter().any(|s| stmt_tests_params(s, params))
}

fn stmt_tests_params(stmt: &Stmt, params: &HashSet<String>) -> bool {
    match stmt.inner_stmt() {
        Stmt::Let { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => expr_tests_params(value, params),
        Stmt::Const { value, .. } => expr_tests_params(value, params),
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            expr_tests_params(array, params)
                || expr_tests_params(index, params)
                || expr_tests_params(value, params)
        }
        Stmt::FieldAssign { object, .. } => expr_tests_params(object, params),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_tests_params(condition, params)
                || stmts_test_params(then_block, params)
                || else_block
                    .as_ref()
                    .is_some_and(|eb| stmts_test_params(eb, params))
        }
        Stmt::While { condition, body } => {
            expr_tests_params(condition, params) || stmts_test_params(body, params)
        }
        Stmt::Repeat { body } => stmts_test_params(body, params),
        Stmt::ForEach { iterable, body, .. } => {
            expr_tests_params(iterable, params) || stmts_test_params(body, params)
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expr_tests_params(start, params)
                || expr_tests_params(end, params)
                || stmts_test_params(body, params)
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            stmts_test_params(try_block, params)
                || stmts_test_params(catch_block, params)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| stmts_test_params(fb, params))
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => stmt_tests_params(inner, params),
        // Nested definitions close over their own params; scanning them
        // with outer params is a conservative superset (more candidates,
        // still sound: clones copy whole bodies verbatim).
        Stmt::Function { body, .. } => stmts_test_params(body, params),
        _ => false,
    }
}

struct Candidate {
    name: String,
    stmt: Stmt,
    params: Vec<String>,
    /// Indexes of untyped, non-`self` params (the signature positions).
    open_idx: Vec<usize>,
}

fn collect_candidates(program: &Program) -> HashMap<String, Candidate> {
    let mut out = HashMap::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name,
            params,
            param_types,
            body,
            type_params,
            ..
        } = stmt.inner_stmt()
        {
            if !type_params.is_empty() || name.contains("__spk__") {
                continue;
            }
            let untyped: HashSet<String> = params
                .iter()
                .zip(param_types.iter())
                .filter(|(p, t)| t.is_none() && p.as_str() != "self")
                .map(|(p, _)| p.clone())
                .collect();
            if untyped.is_empty() {
                continue;
            }
            if !stmts_test_params(body, &untyped) {
                continue;
            }
            let open_idx: Vec<usize> = params
                .iter()
                .enumerate()
                .filter(|(_, p)| untyped.contains(*p))
                .map(|(i, _)| i)
                .collect();
            out.insert(
                name.clone(),
                Candidate {
                    name: name.clone(),
                    stmt: stmt.inner_stmt().clone(),
                    params: params.clone(),
                    open_idx,
                },
            );
        }
    }
    out
}

fn rewrite_calls(
    stmts: &mut [Stmt],
    candidates: &HashMap<String, Candidate>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    for stmt in stmts.iter_mut() {
        rewrite_stmt_calls(stmt, candidates, versions, new_clones);
    }
}

fn rewrite_expr_calls(
    expr: &mut Expr,
    candidates: &HashMap<String, Candidate>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    match expr {
        Expr::Call { name, args } => {
            for arg in args.iter_mut() {
                rewrite_expr_calls(arg, candidates, versions, new_clones);
            }
            if name.contains("__spk__") {
                return;
            }
            let target = candidates.get(name).or_else(|| {
                // Bare-name fallback for qualified/UFCS spellings; routing
                // stays sound because it requires proven kinds below.
                let bare = name.rsplit("::").next().unwrap_or(name);
                let bare = bare.rsplit("__").next().unwrap_or(bare);
                candidates
                    .iter()
                    .find(|(k, _)| {
                        let kb = k.rsplit("::").next().unwrap_or(k);
                        let kb = kb.rsplit("__").next().unwrap_or(kb);
                        kb == bare
                    })
                    .map(|(_, c)| c)
            });
            let Some(cand) = target else { return };
            if args.len() != cand.params.len() {
                return;
            }
            let mut codes = String::new();
            for &i in &cand.open_idx {
                match args.get(i).and_then(arg_kind_code) {
                    Some(c) => codes.push(c),
                    None => return,
                }
            }
            let count = versions.entry(cand.name.clone()).or_insert(0);
            let spec_name = format!("{}__spk__{}", cand.name, codes);
            if !program_has(&*new_clones, &spec_name) && *count < MAX_VERSIONS_PER_FN {
                *count += 1;
                new_clones.push(clone_for_codes(cand, &spec_name));
            }
            // Rewrite only when the version exists (cap overflow keeps
            // the generic call, unchanged behavior).
            if program_has(&*new_clones, &spec_name) {
                *name = spec_name;
            }
        }
        Expr::Binary { left, right, .. } => {
            rewrite_expr_calls(left, candidates, versions, new_clones);
            rewrite_expr_calls(right, candidates, versions, new_clones);
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            rewrite_expr_calls(inner, candidates, versions, new_clones);
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            for item in items {
                rewrite_expr_calls(item, candidates, versions, new_clones);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            rewrite_expr_calls(array, candidates, versions, new_clones);
            rewrite_expr_calls(index, candidates, versions, new_clones);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            rewrite_expr_calls(object, candidates, versions, new_clones);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_expr_calls(condition, candidates, versions, new_clones);
            rewrite_expr_calls(then_branch, candidates, versions, new_clones);
            rewrite_expr_calls(else_branch, candidates, versions, new_clones);
        }
        Expr::NullCoalesce { value, default } => {
            rewrite_expr_calls(value, candidates, versions, new_clones);
            rewrite_expr_calls(default, candidates, versions, new_clones);
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields {
                rewrite_expr_calls(v, candidates, versions, new_clones);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                rewrite_expr_calls(k, candidates, versions, new_clones);
                rewrite_expr_calls(v, candidates, versions, new_clones);
            }
        }
        Expr::Cast { expr: inner, .. } => {
            rewrite_expr_calls(inner, candidates, versions, new_clones);
        }
        Expr::OptionalCall { args, .. } => {
            for arg in args {
                rewrite_expr_calls(arg, candidates, versions, new_clones);
            }
        }
        _ => {}
    }
}

fn program_has(clones: &[Stmt], spec_name: &str) -> bool {
    clones.iter().any(|s| match s.inner_stmt() {
        Stmt::Function { name, .. } => name == spec_name,
        _ => false,
    })
}

fn clone_for_codes(cand: &Candidate, spec_name: &str) -> Stmt {
    // Verbatim copy: params stay untyped (no new TypeErrors possible);
    // codegen decodes the suffix and presets the value types.
    match &cand.stmt {
        Stmt::Function {
            params,
            param_types,
            return_type,
            defaults,
            body,
            type_params,
            attributes,
            ..
        } => Stmt::Function {
            name: spec_name.to_string(),
            params: params.clone(),
            param_types: param_types.clone(),
            return_type: return_type.clone(),
            defaults: defaults.clone(),
            body: body.clone(),
            type_params: type_params.clone(),
            attributes: attributes.clone(),
        },
        other => other.clone(),
    }
}

fn rewrite_stmt_calls(
    stmt: &mut Stmt,
    candidates: &HashMap<String, Candidate>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    match stmt.inner_stmt_mut() {
        Stmt::Let { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => rewrite_expr_calls(value, candidates, versions, new_clones),
        Stmt::Const { value, .. } => rewrite_expr_calls(value, candidates, versions, new_clones),
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            rewrite_expr_calls(array, candidates, versions, new_clones);
            rewrite_expr_calls(index, candidates, versions, new_clones);
            rewrite_expr_calls(value, candidates, versions, new_clones);
        }
        Stmt::FieldAssign { object, .. } => {
            rewrite_expr_calls(object, candidates, versions, new_clones)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_expr_calls(condition, candidates, versions, new_clones);
            rewrite_calls(then_block, candidates, versions, new_clones);
            if let Some(eb) = else_block {
                rewrite_calls(eb, candidates, versions, new_clones);
            }
        }
        Stmt::While { condition, body } => {
            rewrite_expr_calls(condition, candidates, versions, new_clones);
            rewrite_calls(body, candidates, versions, new_clones);
        }
        Stmt::Repeat { body } => rewrite_calls(body, candidates, versions, new_clones),
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_expr_calls(start, candidates, versions, new_clones);
            rewrite_expr_calls(end, candidates, versions, new_clones);
            rewrite_calls(body, candidates, versions, new_clones);
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_expr_calls(iterable, candidates, versions, new_clones);
            rewrite_calls(body, candidates, versions, new_clones);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            rewrite_calls(try_block, candidates, versions, new_clones);
            rewrite_calls(catch_block, candidates, versions, new_clones);
            if let Some(fb) = finally_block {
                rewrite_calls(fb, candidates, versions, new_clones);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            rewrite_stmt_calls(inner, candidates, versions, new_clones)
        }
        Stmt::Function { body, .. } => rewrite_calls(body, candidates, versions, new_clones),
        _ => {}
    }
}

/// Static call-site specialization driver. Runs after `resolve_generics`
/// (templates already expanded); originals are never modified.
pub fn resolve_dynspec(program: &mut Program) {
    let candidates = collect_candidates(program);
    if candidates.is_empty() {
        return;
    }
    let mut versions: HashMap<String, usize> = HashMap::new();
    let mut round = 0;
    loop {
        let mut new_clones: Vec<Stmt> = Vec::new();
        for stmt in program.statements.iter_mut() {
            rewrite_stmt_calls(stmt, &candidates, &mut versions, &mut new_clones);
        }
        if new_clones.is_empty() {
            break;
        }
        // Visit nested calls inside the new clones before publishing,
        // so chains resolve within the round budget.
        for mut clone in new_clones {
            rewrite_stmt_calls(&mut clone, &candidates, &mut versions, &mut Vec::new());
            program.statements.push(clone);
        }
        round += 1;
        if round >= MAX_ROUNDS {
            break;
        }
    }
}
