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

/// Proven kind of one parameter inside a `__spk__` clone scope.
///
/// Codes align with the origin's open (untyped, non-`self`) params in
/// order — the same contract the codegen decoder relies on. Returns
/// `None` for generic scopes, unknown params, or uncovered positions.
/// Exact: a clone is only ever invoked through rewritten call sites
/// whose arguments classified to these codes.
pub fn spec_open_param_kind(
    scope_fn: &str,
    param: &str,
    params: &[String],
    param_types: &[Option<String>],
) -> Option<char> {
    let codes = dynspec_codes(scope_fn)?;
    let mut open_idx = 0usize;
    for (p, t) in params.iter().zip(param_types.iter()) {
        if t.is_none() && p != "self" {
            if p == param {
                return codes.get(open_idx).copied();
            }
            open_idx += 1;
        }
    }
    None
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
    /// Whether the body tests an untyped param (only these get versions).
    tests: bool,
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
            let tests = !untyped.is_empty() && stmts_test_params(body, &untyped);
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
                    tests,
                },
            );
        }
    }
    out
}

fn rewrite_calls(
    stmts: &mut [Stmt],
    candidates: &HashMap<String, Candidate>,
    binds: &HashMap<String, HashMap<String, BindVal>>,
    scope: Option<&str>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    for stmt in stmts.iter_mut() {
        rewrite_stmt_calls(stmt, candidates, binds, scope, versions, new_clones);
    }
}

fn rewrite_expr_calls(
    expr: &mut Expr,
    candidates: &HashMap<String, Candidate>,
    binds: &HashMap<String, HashMap<String, BindVal>>,
    scope: Option<&str>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    match expr {
        Expr::Call { name, args } => {
            for arg in args.iter_mut() {
                rewrite_expr_calls(arg, candidates, binds, scope, versions, new_clones);
            }
            if name.contains("__spk__") {
                return;
            }
            let target = callee_def(candidates, name);
            let Some(cand) = target else { return };
            if !cand.tests {
                return;
            }
            if args.len() != cand.params.len() {
                return;
            }
            let mut codes = String::new();
            let scope_binds = scope.and_then(|s| binds.get(s));
            for &i in &cand.open_idx {
                let arg = match args.get(i) {
                    Some(a) => a,
                    None => return,
                };
                match classify_arg(arg, scope_binds).or_else(|| {
                    // Calls classify through the callee's returns
                    // (single-assignment bindings + literal args only;
                    // no nested calls, so this always terminates).
                    if let Expr::Call {
                        name: callee,
                        args: call_args,
                    } = arg
                    {
                        callee_def(candidates, callee).and_then(|def| {
                            callee_binds(binds, &def.name)
                                .and_then(|cb| classify_call_return(def, call_args, cb))
                        })
                    } else {
                        None
                    }
                }) {
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
            rewrite_expr_calls(left, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(right, candidates, binds, scope, versions, new_clones);
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            rewrite_expr_calls(inner, candidates, binds, scope, versions, new_clones);
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => {
            for item in items {
                rewrite_expr_calls(item, candidates, binds, scope, versions, new_clones);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            rewrite_expr_calls(array, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(index, candidates, binds, scope, versions, new_clones);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            rewrite_expr_calls(object, candidates, binds, scope, versions, new_clones);
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_expr_calls(condition, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(then_branch, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(else_branch, candidates, binds, scope, versions, new_clones);
        }
        Expr::NullCoalesce { value, default } => {
            rewrite_expr_calls(value, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(default, candidates, binds, scope, versions, new_clones);
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields {
                rewrite_expr_calls(v, candidates, binds, scope, versions, new_clones);
            }
        }
        Expr::Map(pairs) => {
            for (k, v) in pairs {
                rewrite_expr_calls(k, candidates, binds, scope, versions, new_clones);
                rewrite_expr_calls(v, candidates, binds, scope, versions, new_clones);
            }
        }
        Expr::Cast { expr: inner, .. } => {
            rewrite_expr_calls(inner, candidates, binds, scope, versions, new_clones);
        }
        Expr::OptionalCall { args, .. } => {
            for arg in args {
                rewrite_expr_calls(arg, candidates, binds, scope, versions, new_clones);
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
    binds: &HashMap<String, HashMap<String, BindVal>>,
    scope: Option<&str>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    match stmt.inner_stmt_mut() {
        Stmt::Let { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => {
            rewrite_expr_calls(value, candidates, binds, scope, versions, new_clones)
        }
        Stmt::Const { value, .. } => {
            rewrite_expr_calls(value, candidates, binds, scope, versions, new_clones)
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            rewrite_expr_calls(array, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(index, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(value, candidates, binds, scope, versions, new_clones);
        }
        Stmt::FieldAssign { object, .. } => {
            rewrite_expr_calls(object, candidates, binds, scope, versions, new_clones)
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_expr_calls(condition, candidates, binds, scope, versions, new_clones);
            rewrite_calls(then_block, candidates, binds, scope, versions, new_clones);
            if let Some(eb) = else_block {
                rewrite_calls(eb, candidates, binds, scope, versions, new_clones);
            }
        }
        Stmt::While { condition, body } => {
            rewrite_expr_calls(condition, candidates, binds, scope, versions, new_clones);
            rewrite_calls(body, candidates, binds, scope, versions, new_clones);
        }
        Stmt::Repeat { body } => {
            rewrite_calls(body, candidates, binds, scope, versions, new_clones)
        }
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_expr_calls(start, candidates, binds, scope, versions, new_clones);
            rewrite_expr_calls(end, candidates, binds, scope, versions, new_clones);
            rewrite_calls(body, candidates, binds, scope, versions, new_clones);
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_expr_calls(iterable, candidates, binds, scope, versions, new_clones);
            rewrite_calls(body, candidates, binds, scope, versions, new_clones);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            rewrite_calls(try_block, candidates, binds, scope, versions, new_clones);
            rewrite_calls(catch_block, candidates, binds, scope, versions, new_clones);
            if let Some(fb) = finally_block {
                rewrite_calls(fb, candidates, binds, scope, versions, new_clones);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            rewrite_stmt_calls(inner, candidates, binds, scope, versions, new_clones)
        }
        // Nested definitions close over unknown bindings: no scope map.
        Stmt::Function { body, .. } => {
            rewrite_calls(body, candidates, binds, None, versions, new_clones)
        }
        _ => {}
    }
}

/// A single-assignment binding: either a proven kind, a literal array
/// whose constant indexes classify per element, or a literal map whose
/// constant string keys classify per value.
#[derive(Clone)]
enum BindVal {
    Kind(char),
    LitArray(Vec<Expr>),
    LitMap(Vec<(String, Expr)>),
}

fn count_assigns(stmts: &[Stmt], counts: &mut HashMap<String, usize>) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Let { name, .. } | Stmt::Assign { name, .. } => {
                *counts.entry(name.clone()).or_insert(0) += 1;
            }
            Stmt::For { var, body, .. } => {
                *counts.entry(var.clone()).or_insert(0) += 1;
                count_assigns(body, counts);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                *counts.entry(var.clone()).or_insert(0) += 1;
                if let Some(v) = value_var {
                    *counts.entry(v.clone()).or_insert(0) += 1;
                }
                count_assigns(body, counts);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                count_assigns(then_block, counts);
                if let Some(eb) = else_block {
                    count_assigns(eb, counts);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => count_assigns(body, counts),
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                count_assigns(try_block, counts);
                count_assigns(catch_block, counts);
                if let Some(fb) = finally_block {
                    count_assigns(fb, counts);
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                count_assigns(std::slice::from_ref(inner), counts)
            }
            // Nested definitions are separate scopes: never counted here.
            _ => {}
        }
    }
}

fn literal_binds(stmts: &[Stmt], counts: &HashMap<String, usize>) -> HashMap<String, BindVal> {
    let mut out = HashMap::new();
    // Pass 1: direct literals.
    for s in stmts {
        if let Stmt::Let { name, value, .. } = s.inner_stmt() {
            if counts.get(name).copied().unwrap_or(0) != 1 {
                continue;
            }
            match value {
                Expr::Array(elems) => {
                    out.insert(name.clone(), BindVal::LitArray(elems.clone()));
                }
                Expr::Map(pairs) => {
                    let mut entries = Vec::new();
                    let mut all_str_keys = true;
                    for (k, v) in pairs {
                        if let Expr::String(key) = k {
                            entries.push((key.clone(), v.clone()));
                        } else {
                            all_str_keys = false;
                            break;
                        }
                    }
                    if all_str_keys {
                        out.insert(name.clone(), BindVal::LitMap(entries));
                    } else {
                        out.insert(name.clone(), BindVal::Kind('m'));
                    }
                }
                other => {
                    if let Some(c) = arg_kind_code(other) {
                        out.insert(name.clone(), BindVal::Kind(c));
                    }
                }
            }
        }
    }
    // Pass 2: single-level aliases (`let x = y`).
    for s in stmts {
        if let Stmt::Let { name, value, .. } = s.inner_stmt() {
            if counts.get(name).copied().unwrap_or(0) != 1 {
                continue;
            }
            if let Expr::Identifier(src) = value {
                if let Some(BindVal::Kind(c)) = out.get(src) {
                    out.insert(name.clone(), BindVal::Kind(*c));
                }
            }
        }
    }
    out
}

/// Classify a call argument: literals always; identifiers and constant
/// indexes through single-assignment bindings; anything else stays
/// generic (sound fallback).
fn classify_arg(arg: &Expr, binds: Option<&HashMap<String, BindVal>>) -> Option<char> {
    if let Some(c) = arg_kind_code(arg) {
        return Some(c);
    }
    let binds = binds?;
    match arg {
        Expr::Identifier(name) => match binds.get(name) {
            Some(BindVal::Kind(c)) => Some(*c),
            _ => None,
        },
        Expr::Index { array, index } => {
            if let (Expr::Identifier(arr), Expr::Number(idx)) = (&**array, &**index) {
                if let Some(BindVal::LitArray(elems)) = binds.get(arr) {
                    if let Some(elem) = elems.get(*idx as usize) {
                        return arg_kind_code(elem);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Classify a call's return kind: every top-level `return` must agree
/// on one literal kind under caller substitution (single-assignment
/// map/array bindings + literal args). No nested calls are classified,
/// so this always terminates.
/// Resolve a callee to its definition: exact name, else bare-name
/// fallback for qualified/UFCS spellings. Routing stays sound because
/// every use requires proven kinds. Specializations are never
/// re-entered (their calls were already rewritten).
fn callee_def<'a>(candidates: &'a HashMap<String, Candidate>, name: &str) -> Option<&'a Candidate> {
    if name.contains("__spk__") {
        return None;
    }
    candidates.get(name).or_else(|| {
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
    })
}

fn callee_binds<'a>(
    binds: &'a HashMap<String, HashMap<String, BindVal>>,
    name: &str,
) -> Option<&'a HashMap<String, BindVal>> {
    binds.get(name).or_else(|| {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        binds.get(bare)
    })
}

fn classify_call_return(
    cand: &Candidate,
    call_args: &[Expr],
    binds: &HashMap<String, BindVal>,
) -> Option<char> {
    let Stmt::Function { params, body, .. } = &cand.stmt else {
        return None;
    };
    // Substitute literal args for params; shadowed params void.
    let mut assigned = HashMap::new();
    count_assigns(body, &mut assigned);
    let mut subst: HashMap<&str, &Expr> = HashMap::new();
    for (param, arg) in params.iter().zip(call_args.iter()) {
        if assigned.contains_key(param) {
            continue;
        }
        match arg {
            Expr::Number(_) | Expr::Float(_) | Expr::String(_) => {
                subst.insert(param.as_str(), arg);
            }
            _ => {}
        }
    }
    let mut returns = Vec::new();
    let mut tainted = false;
    collect_live_returns(body, &subst, &mut returns, &mut tainted);
    if tainted || returns.is_empty() {
        return None;
    }
    let mut kind = None;
    for ret in returns {
        match (kind, classify_subst(ret, binds, &subst)) {
            (_, None) => return None,
            (Some(k), Some(c)) if k != c => return None,
            (_, Some(c)) => kind = Some(c),
        }
    }
    kind
}

/// Tri-state truthiness of an expression under caller substitution.
/// Only literals, substituted params, and `not` fold; everything else
/// is Unknown (conservative: unknown conditions keep all branches).
#[derive(Clone, Copy, PartialEq)]
enum Truth {
    Yes,
    No,
    Unknown,
}

fn eval_truth(expr: &Expr, subst: &HashMap<&str, &Expr>) -> Truth {
    match expr {
        Expr::Number(n) => {
            if *n != 0 {
                Truth::Yes
            } else {
                Truth::No
            }
        }
        Expr::Float(f) => {
            if *f != 0.0 {
                Truth::Yes
            } else {
                Truth::No
            }
        }
        Expr::String(_) => Truth::Yes,
        Expr::Null => Truth::No,
        Expr::Identifier(p) => match subst.get(p.as_str()) {
            Some(e) => eval_truth(e, subst),
            None => Truth::Unknown,
        },
        Expr::Unary { op, expr: inner } => match eval_truth(inner, subst) {
            Truth::Yes if *op == crate::ast::UnaryOp::Not => Truth::No,
            Truth::No if *op == crate::ast::UnaryOp::Not => Truth::Yes,
            _ => Truth::Unknown,
        },
        _ => Truth::Unknown,
    }
}

fn subtree_has_return(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s.inner_stmt() {
        Stmt::Return(_) => true,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            subtree_has_return(then_block)
                || else_block.as_ref().is_some_and(|eb| subtree_has_return(eb))
        }
        Stmt::While { body, .. }
        | Stmt::Repeat { body }
        | Stmt::For { body, .. }
        | Stmt::ForEach { body, .. } => subtree_has_return(body),
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            subtree_has_return(try_block)
                || subtree_has_return(catch_block)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| subtree_has_return(fb))
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => subtree_has_return(std::slice::from_ref(inner)),
        // Nested definitions own their returns.
        _ => false,
    })
}

/// Collects returns on paths that provably execute under `subst`.
/// Sets `tainted` when a valueless exit is possible (if-without-else
/// on a non-true condition, loops/try blocks containing returns):
/// taint forces the generic fallback, so over-tainting only ever
/// misses optimizations, never misroutes.
fn collect_live_returns<'a>(
    stmts: &'a [Stmt],
    subst: &HashMap<&str, &Expr>,
    out: &mut Vec<&'a Expr>,
    tainted: &mut bool,
) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Return(expr_opt) => match expr_opt {
                Some(expr) => out.push(expr),
                None => *tainted = true,
            },
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => match eval_truth(condition, subst) {
                Truth::Yes => collect_live_returns(then_block, subst, out, tainted),
                Truth::No => {
                    if let Some(eb) = else_block {
                        collect_live_returns(eb, subst, out, tainted);
                    }
                }
                Truth::Unknown => {
                    collect_live_returns(then_block, subst, out, tainted);
                    match else_block {
                        Some(eb) => collect_live_returns(eb, subst, out, tainted),
                        None => *tainted = true,
                    }
                }
            },
            Stmt::While { body, .. }
            | Stmt::Repeat { body }
            | Stmt::For { body, .. }
            | Stmt::ForEach { body, .. }
                if subtree_has_return(body) =>
            {
                *tainted = true;
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } if subtree_has_return(try_block)
                || subtree_has_return(catch_block)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| subtree_has_return(fb)) =>
            {
                *tainted = true;
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                collect_live_returns(std::slice::from_ref(inner), subst, out, tainted)
            }
            _ => {}
        }
    }
}

fn classify_subst(
    expr: &Expr,
    binds: &HashMap<String, BindVal>,
    subst: &HashMap<&str, &Expr>,
) -> Option<char> {
    if let Some(c) = arg_kind_code(expr) {
        return Some(c);
    }
    match expr {
        Expr::Identifier(name) => {
            if let Some(arg) = subst.get(name.as_str()) {
                return arg_kind_code(arg);
            }
            match binds.get(name) {
                Some(BindVal::Kind(c)) => Some(*c),
                _ => None,
            }
        }
        Expr::Index { array, index } => {
            let arr_name = match &**array {
                Expr::Identifier(n) => n,
                _ => return None,
            };
            let idx_num: Option<usize> = match &**index {
                Expr::Number(n) if *n >= 0 => Some(*n as usize),
                Expr::Identifier(p) => match subst.get(p.as_str()).copied() {
                    Some(Expr::Number(n)) if *n >= 0 => Some(*n as usize),
                    _ => None,
                },
                _ => None,
            };
            let idx_str: Option<&str> = match &**index {
                Expr::String(s) => Some(s),
                Expr::Identifier(p) => match subst.get(p.as_str()).copied() {
                    Some(Expr::String(s)) => Some(s),
                    _ => None,
                },
                _ => None,
            };
            match binds.get(arr_name) {
                Some(BindVal::LitArray(elems)) => {
                    idx_num.and_then(|i| elems.get(i)).and_then(arg_kind_code)
                }
                Some(BindVal::LitMap(entries)) => {
                    let key = idx_str?;
                    entries
                        .iter()
                        .find(|(k, _)| k == key)
                        .and_then(|(_, v)| arg_kind_code(v))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Static call-site specialization driver. Runs after `resolve_generics`
/// (templates already expanded); originals are never modified.
pub fn resolve_dynspec(program: &mut Program) {
    let candidates = collect_candidates(program);
    if candidates.is_empty() {
        return;
    }
    // Per-scope single-assignment literal bindings (`fn name` or `""`
    // for top level). Reassignment anywhere in the scope voids the
    // entry, so routing stays sound. Nested function bodies get no
    // map (shadowing would be unsound); their calls use literals only.
    let mut binds: HashMap<String, HashMap<String, BindVal>> = HashMap::new();
    {
        let mut top_counts = HashMap::new();
        count_assigns(&program.statements, &mut top_counts);
        binds.insert(
            String::new(),
            literal_binds(&program.statements, &top_counts),
        );
    }
    for stmt in &program.statements {
        if let Stmt::Function { name, body, .. } = stmt.inner_stmt() {
            let mut counts = HashMap::new();
            count_assigns(body, &mut counts);
            binds.insert(name.clone(), literal_binds(body, &counts));
            let bare = name.rsplit("::").next().unwrap_or(name);
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if bare != name {
                binds.insert(
                    bare.to_string(),
                    binds.get(name).cloned().unwrap_or_default(),
                );
            }
        }
    }
    let mut versions: HashMap<String, usize> = HashMap::new();
    let mut round = 0;
    loop {
        let mut new_clones: Vec<Stmt> = Vec::new();
        for stmt in program.statements.iter_mut() {
            walk_top_stmt(stmt, &candidates, &binds, &mut versions, &mut new_clones);
        }
        if new_clones.is_empty() {
            break;
        }
        // Visit nested calls inside the new clones before publishing,
        // so chains resolve within the round budget.
        for mut clone in new_clones {
            walk_top_stmt(
                &mut clone,
                &candidates,
                &binds,
                &mut versions,
                &mut Vec::new(),
            );
            program.statements.push(clone);
        }
        round += 1;
        if round >= MAX_ROUNDS {
            break;
        }
    }
}

/// Walks one top-level statement: function bodies use their own scope
/// map (clones reuse their origin's); everything else uses the
/// top-level map.
fn walk_top_stmt(
    stmt: &mut Stmt,
    candidates: &HashMap<String, Candidate>,
    binds: &HashMap<String, HashMap<String, BindVal>>,
    versions: &mut HashMap<String, usize>,
    new_clones: &mut Vec<Stmt>,
) {
    let scope: Option<String> = match stmt.inner_stmt() {
        Stmt::Function { name, .. } => {
            if binds.contains_key(name as &str) {
                Some(name.clone())
            } else {
                let origin = name.split("__spk__").next().unwrap_or(name);
                binds.get_key_value(origin).map(|(k, _)| k.clone())
            }
        }
        _ => Some(String::new()),
    };
    match stmt.inner_stmt_mut() {
        Stmt::Function { body, .. } => rewrite_calls(
            body,
            candidates,
            binds,
            scope.as_deref(),
            versions,
            new_clones,
        ),
        _ => rewrite_stmt_calls(stmt, candidates, binds, Some(""), versions, new_clones),
    }
}
