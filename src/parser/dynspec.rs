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
use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use std::collections::{HashMap, HashSet};

const MAX_VERSIONS_PER_FN: usize = 8;
const MAX_ROUNDS: usize = 5;

/// Kind codes in `__spk__` suffixes. Single static alphabet shared with
/// the codegen decoder (`crate::codegen` reads these back).
fn arg_kind_code(arg: &Expr) -> Option<char> {
    match arg {
        Expr::Number(_) => Some('i'),
        Expr::Float(_) => Some('f'),
        Expr::String(_) | Expr::InterpolatedString(_) => Some('s'),
        Expr::Array(_) => Some('a'),
        Expr::Map(_) => Some('m'),
        Expr::Null => Some('n'),
        // The sign never changes a literal's kind. Without this
        // `set("x", -2.5)` stayed unclassifiable and ran the generic
        // clone, whose runtime `is null` check rejected `0` payloads.
        Expr::Unary {
            op: UnaryOp::Negate,
            expr,
        } => match &**expr {
            Expr::Float(_) => Some('f'),
            Expr::Number(_) => Some('i'),
            _ => None,
        },
        // `str(...)` always builds a string.
        Expr::Call { name, .. } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if matches!(bare, "str" | "to_string") {
                Some('s')
            } else {
                None
            }
        }
        // String concatenation is always a string. A dynamic key
        // (`"k" + str(i)`) previously made the whole call unclassifiable:
        // the generic clone's runtime `value is null` then rejected a
        // legitimate `0` payload (0 and null share the zero word).
        Expr::Binary {
            op: BinaryOp::Add,
            left,
            right,
        } => {
            if arg_kind_code(left) == Some('s') || arg_kind_code(right) == Some('s') {
                Some('s')
            } else {
                None
            }
        }
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
            .all(|c| matches!(c, 'i' | 'f' | 's' | 'a' | 'm' | 'n'))
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
        Expr::Binary { left, op, right } => {
            if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                if let (Expr::Identifier(name), Expr::Null) = (&**left, &**right) {
                    if params.contains(name) {
                        return true;
                    }
                }
                if let (Expr::Null, Expr::Identifier(name)) = (&**left, &**right) {
                    if params.contains(name) {
                        return true;
                    }
                }
            }
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

fn expr_poly_params(expr: &Expr, params: &HashSet<String>) -> bool {
    match expr {
        Expr::Call { name, args } => {
            let bare = name.rsplit("::").next().unwrap_or(name.as_str());
            let bare = bare.rsplit("__").next().unwrap_or(bare);
            if matches!(
                bare,
                "int"
                    | "to_int"
                    | "parse_int"
                    | "float"
                    | "to_float"
                    | "str"
                    | "to_str"
                    | "len"
                    | "length"
            ) {
                if let Some(Expr::Identifier(pname)) = args.first() {
                    if params.contains(pname) {
                        return true;
                    }
                }
            }
            args.iter().any(|a| expr_poly_params(a, params))
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            if let Expr::Identifier(name) = &**array {
                if params.contains(name) {
                    return true;
                }
            }
            expr_poly_params(array, params) || expr_poly_params(index, params)
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            expr_poly_params(inner, params)
        }
        Expr::Binary { left, right, .. } => {
            expr_poly_params(left, params) || expr_poly_params(right, params)
        }
        Expr::Array(args) | Expr::InterpolatedString(args) => {
            args.iter().any(|a| expr_poly_params(a, params))
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            expr_poly_params(object, params)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_poly_params(condition, params)
                || expr_poly_params(then_branch, params)
                || expr_poly_params(else_branch, params)
        }
        Expr::NullCoalesce { value, default } => {
            expr_poly_params(value, params) || expr_poly_params(default, params)
        }
        Expr::StructInit { fields, .. } => fields.iter().any(|(_, v)| expr_poly_params(v, params)),
        Expr::Map(pairs) => pairs
            .iter()
            .any(|(k, v)| expr_poly_params(k, params) || expr_poly_params(v, params)),
        Expr::Cast { expr: inner, .. } => expr_poly_params(inner, params),
        Expr::OptionalCall { args, .. } => args.iter().any(|a| expr_poly_params(a, params)),
        _ => false,
    }
}

fn stmts_poly_params(stmts: &[Stmt], params: &HashSet<String>) -> bool {
    stmts.iter().any(|s| stmt_poly_params(s, params))
}

fn stmt_poly_params(stmt: &Stmt, params: &HashSet<String>) -> bool {
    match stmt.inner_stmt() {
        Stmt::Let { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => expr_poly_params(value, params),
        Stmt::Const { value, .. } => expr_poly_params(value, params),
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            expr_poly_params(array, params)
                || expr_poly_params(index, params)
                || expr_poly_params(value, params)
        }
        Stmt::FieldAssign { object, .. } => expr_poly_params(object, params),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_poly_params(condition, params)
                || stmts_poly_params(then_block, params)
                || else_block
                    .as_ref()
                    .is_some_and(|eb| stmts_poly_params(eb, params))
        }
        Stmt::While { condition, body } => {
            expr_poly_params(condition, params) || stmts_poly_params(body, params)
        }
        Stmt::Repeat { body } => stmts_poly_params(body, params),
        Stmt::ForEach { iterable, body, .. } => {
            expr_poly_params(iterable, params) || stmts_poly_params(body, params)
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expr_poly_params(start, params)
                || expr_poly_params(end, params)
                || stmts_poly_params(body, params)
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            stmts_poly_params(try_block, params)
                || stmts_poly_params(catch_block, params)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| stmts_poly_params(fb, params))
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => stmt_poly_params(inner, params),
        Stmt::Function { body, .. } => stmts_poly_params(body, params),
        _ => false,
    }
}

fn expr_tests_param_null(expr: &Expr, param: &str) -> bool {
    match expr {
        Expr::TypeCheck {
            expr: inner,
            target,
            ..
        } => {
            if let Expr::Identifier(name) = &**inner {
                if name == param && (target == "null" || target == "nil") {
                    return true;
                }
            }
            expr_tests_param_null(inner, param)
        }
        Expr::Binary { left, op, right } => {
            if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                if let (Expr::Identifier(name), Expr::Null) = (&**left, &**right) {
                    if name == param {
                        return true;
                    }
                }
                if let (Expr::Null, Expr::Identifier(name)) = (&**left, &**right) {
                    if name == param {
                        return true;
                    }
                }
            }
            expr_tests_param_null(left, param) || expr_tests_param_null(right, param)
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            expr_tests_param_null(inner, param)
        }
        Expr::Call { args, .. } | Expr::Array(args) | Expr::InterpolatedString(args) => {
            args.iter().any(|a| expr_tests_param_null(a, param))
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            expr_tests_param_null(array, param) || expr_tests_param_null(index, param)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            expr_tests_param_null(object, param)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_tests_param_null(condition, param)
                || expr_tests_param_null(then_branch, param)
                || expr_tests_param_null(else_branch, param)
        }
        Expr::NullCoalesce { value, default } => {
            expr_tests_param_null(value, param) || expr_tests_param_null(default, param)
        }
        _ => false,
    }
}

fn stmts_test_param_null(stmts: &[Stmt], param: &str) -> bool {
    stmts.iter().any(|s| stmt_tests_param_null(s, param))
}

fn stmt_tests_param_null(stmt: &Stmt, param: &str) -> bool {
    match stmt.inner_stmt() {
        Stmt::Let { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::Say(value)
        | Stmt::Expr(value)
        | Stmt::Return(Some(value))
        | Stmt::Throw(Some(value)) => expr_tests_param_null(value, param),
        Stmt::Const { value, .. } => expr_tests_param_null(value, param),
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            expr_tests_param_null(array, param)
                || expr_tests_param_null(index, param)
                || expr_tests_param_null(value, param)
        }
        Stmt::FieldAssign { object, .. } => expr_tests_param_null(object, param),
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_tests_param_null(condition, param)
                || stmts_test_param_null(then_block, param)
                || else_block
                    .as_ref()
                    .is_some_and(|eb| stmts_test_param_null(eb, param))
        }
        Stmt::While { condition, body } => {
            expr_tests_param_null(condition, param) || stmts_test_param_null(body, param)
        }
        Stmt::Repeat { body } => stmts_test_param_null(body, param),
        Stmt::ForEach { iterable, body, .. } => {
            expr_tests_param_null(iterable, param) || stmts_test_param_null(body, param)
        }
        Stmt::For {
            start, end, body, ..
        } => {
            expr_tests_param_null(start, param)
                || expr_tests_param_null(end, param)
                || stmts_test_param_null(body, param)
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            stmts_test_param_null(try_block, param)
                || stmts_test_param_null(catch_block, param)
                || finally_block
                    .as_ref()
                    .is_some_and(|fb| stmts_test_param_null(fb, param))
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => stmt_tests_param_null(inner, param),
        Stmt::Function { body, .. } => stmts_test_param_null(body, param),
        _ => false,
    }
}

fn cand_tests_param_null(cand: &Candidate, param: &str) -> bool {
    stmt_tests_param_null(&cand.stmt, param)
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
            return_type,
            attributes,
            ..
        } = stmt.inner_stmt()
        {
            if !type_params.is_empty() || name.contains("__spk__") {
                continue;
            }
            // alya-lang/alya#100: capturing functions read via cells at
            // emission; kind specialization by AST would desync from the
            // capture plan (instance names miss the plan).
            if attributes.iter().any(|a| a.name == "alya::capturing") {
                continue;
            }
            let untyped: HashSet<String> = params
                .iter()
                .zip(param_types.iter())
                .filter(|(p, t)| t.is_none() && p.as_str() != "self")
                .map(|(p, _)| p.clone())
                .collect();
            // Specialize untyped params indexed (a[i]), sized (len(a)), or cast (int(v)/float(v))
            // per call site to prevent string inference poisoning (#132) and float truncation (#152).
            // Guard: return_type.is_none() avoids over-specializing annotated return functions.
            let tests = !untyped.is_empty()
                && (stmts_test_params(body, &untyped)
                    || (return_type.is_none() && stmts_poly_params(body, &untyped)));
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
    // Transitive forwarder fixpoint: a facade that passes one of its own
    // open params into a candidate's open position is itself a candidate.
    // Without this a typed chain (`Cache.set` → `store_set` →
    // `stored_repr`) never specializes: the leaf's kind tests fold FALSE on
    // the generic path and float/array payloads take the wrong branch at
    // runtime (raw floats stored as ints, arrays persisted as scalars).
    loop {
        let mut newly: Vec<String> = Vec::new();
        for (name, cand) in out.iter() {
            if cand.tests || cand.open_idx.is_empty() {
                continue;
            }
            let open: HashSet<String> = cand
                .open_idx
                .iter()
                .filter_map(|&i| cand.params.get(i).cloned())
                .collect();
            let Stmt::Function { body, .. } = &cand.stmt else {
                continue;
            };
            if stmts_forward_open(body, &open, &out) {
                newly.push(name.clone());
            }
        }
        if newly.is_empty() {
            break;
        }
        for name in newly {
            if let Some(cand) = out.get_mut(&name) {
                cand.tests = true;
            }
        }
    }
    out
}

/// True when `expr` forwards one of `open` into an open parameter position
/// of a call to an already-known candidate.
fn expr_forwards_open(
    expr: &Expr,
    open: &HashSet<String>,
    candidates: &HashMap<String, Candidate>,
) -> bool {
    match expr {
        Expr::Call { name, args } => {
            if args.iter().any(|a| expr_forwards_open(a, open, candidates)) {
                return true;
            }
            let Some(cand) = callee_def(candidates, name) else {
                return false;
            };
            if !cand.tests || args.len() != cand.params.len() {
                return false;
            }
            cand.open_idx
                .iter()
                .any(|&i| matches!(args.get(i), Some(Expr::Identifier(p)) if open.contains(p)))
        }
        Expr::Binary { left, right, .. } => {
            expr_forwards_open(left, open, candidates)
                || expr_forwards_open(right, open, candidates)
        }
        Expr::Unary { expr: inner, .. } | Expr::ForceUnwrap(inner) => {
            expr_forwards_open(inner, open, candidates)
        }
        Expr::Array(items) | Expr::InterpolatedString(items) => items
            .iter()
            .any(|e| expr_forwards_open(e, open, candidates)),
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            expr_forwards_open(array, open, candidates)
                || expr_forwards_open(index, open, candidates)
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            expr_forwards_open(object, open, candidates)
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_forwards_open(condition, open, candidates)
                || expr_forwards_open(then_branch, open, candidates)
                || expr_forwards_open(else_branch, open, candidates)
        }
        Expr::NullCoalesce { value, default } => {
            expr_forwards_open(value, open, candidates)
                || expr_forwards_open(default, open, candidates)
        }
        Expr::StructInit { fields, .. } => fields
            .iter()
            .any(|(_, v)| expr_forwards_open(v, open, candidates)),
        Expr::Map(pairs) => pairs.iter().any(|(k, v)| {
            expr_forwards_open(k, open, candidates) || expr_forwards_open(v, open, candidates)
        }),
        Expr::Cast { expr: inner, .. } => expr_forwards_open(inner, open, candidates),
        Expr::OptionalCall { args, .. } => {
            args.iter().any(|a| expr_forwards_open(a, open, candidates))
        }
        _ => false,
    }
}

fn stmts_forward_open(
    stmts: &[Stmt],
    open: &HashSet<String>,
    candidates: &HashMap<String, Candidate>,
) -> bool {
    for stmt in stmts {
        let found = match stmt.inner_stmt() {
            Stmt::Let { value, .. }
            | Stmt::Assign { value, .. }
            | Stmt::Say(value)
            | Stmt::Expr(value)
            | Stmt::Return(Some(value))
            | Stmt::Throw(Some(value)) => expr_forwards_open(value, open, candidates),
            Stmt::IndexAssign {
                array,
                index,
                value,
            } => {
                expr_forwards_open(array, open, candidates)
                    || expr_forwards_open(index, open, candidates)
                    || expr_forwards_open(value, open, candidates)
            }
            Stmt::FieldAssign { object, value, .. } => {
                expr_forwards_open(object, open, candidates)
                    || expr_forwards_open(value, open, candidates)
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                expr_forwards_open(condition, open, candidates)
                    || stmts_forward_open(then_block, open, candidates)
                    || else_block
                        .as_ref()
                        .is_some_and(|eb| stmts_forward_open(eb, open, candidates))
            }
            Stmt::While { condition, body } => {
                expr_forwards_open(condition, open, candidates)
                    || stmts_forward_open(body, open, candidates)
            }
            Stmt::Repeat { body } => stmts_forward_open(body, open, candidates),
            Stmt::For {
                start, end, body, ..
            } => {
                expr_forwards_open(start, open, candidates)
                    || expr_forwards_open(end, open, candidates)
                    || stmts_forward_open(body, open, candidates)
            }
            Stmt::ForEach { iterable, body, .. } => {
                expr_forwards_open(iterable, open, candidates)
                    || stmts_forward_open(body, open, candidates)
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                stmts_forward_open(try_block, open, candidates)
                    || stmts_forward_open(catch_block, open, candidates)
                    || finally_block
                        .as_ref()
                        .is_some_and(|fb| stmts_forward_open(fb, open, candidates))
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                stmts_forward_open(std::slice::from_ref(inner), open, candidates)
            }
            // Nested functions own their bodies and scopes.
            _ => false,
        };
        if found {
            return true;
        }
    }
    false
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
            let mut all_static = true;
            for &i in &cand.open_idx {
                let arg = match args.get(i) {
                    Some(a) => a,
                    None => {
                        all_static = false;
                        break;
                    }
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
                    None => {
                        all_static = false;
                        break;
                    }
                }
            }
            if all_static {
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
                return;
            }

            // alya-lang/alya#170: ternary dynamic dispatch when an argument is nullable
            // (e.g. from a loop or map holding null and one non-null kind) and the callee
            // tests that parameter for null.
            let mut nullable_param: Option<(usize, char)> = None;
            let mut codes_null = String::new();
            let mut codes_val = String::new();
            let mut nullable_eligible = true;
            for &i in &cand.open_idx {
                let arg = match args.get(i) {
                    Some(a) => a,
                    None => {
                        nullable_eligible = false;
                        break;
                    }
                };
                if let Some(c) = classify_arg(arg, scope_binds) {
                    codes_null.push(c);
                    codes_val.push(c);
                } else if nullable_param.is_none() {
                    if let Some(val_k) = classify_nullable_arg(arg, scope_binds) {
                        if cand_tests_param_null(cand, &cand.params[i]) {
                            nullable_param = Some((i, val_k));
                            codes_null.push('n');
                            codes_val.push(val_k);
                        } else {
                            nullable_eligible = false;
                            break;
                        }
                    } else {
                        nullable_eligible = false;
                        break;
                    }
                } else {
                    nullable_eligible = false;
                    break;
                }
            }

            if nullable_eligible {
                if let Some((null_idx, _)) = nullable_param {
                    let count = versions.entry(cand.name.clone()).or_insert(0);
                    let spec_null = format!("{}__spk__{}", cand.name, codes_null);
                    let spec_val = format!("{}__spk__{}", cand.name, codes_val);
                    if !program_has(&*new_clones, &spec_null) && *count < MAX_VERSIONS_PER_FN {
                        *count += 1;
                        new_clones.push(clone_for_codes(cand, &spec_null));
                    }
                    if !program_has(&*new_clones, &spec_val) && *count < MAX_VERSIONS_PER_FN {
                        *count += 1;
                        new_clones.push(clone_for_codes(cand, &spec_val));
                    }
                    if program_has(&*new_clones, &spec_null) && program_has(&*new_clones, &spec_val)
                    {
                        let arg_expr = args[null_idx].clone();
                        let mut args_null = args.clone();
                        args_null[null_idx] = Expr::Null;
                        let args_val = args.clone();
                        let then_call = Expr::Call {
                            name: spec_null,
                            args: args_null,
                        };
                        let else_call = Expr::Call {
                            name: spec_val,
                            args: args_val,
                        };
                        *expr = Expr::Ternary {
                            condition: Box::new(Expr::TypeCheck {
                                expr: Box::new(arg_expr),
                                target: "null".to_string(),
                                negated: false,
                            }),
                            then_branch: Box::new(then_call),
                            else_branch: Box::new(else_call),
                        };
                    }
                }
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
    LitString(String),
    NullableKind(char),
}

/// `x = x + n` / `x = x - n` (compound assignment lowers to this) keeps
/// `x`'s kind: numeric arithmetic and string concatenation preserve the
/// base operand's kind. Counting it as a rebind made loop counters
/// unclassifiable, so `cache.set(key, i)` ran the generic clone whose
/// runtime `is null` check rejected a legitimate `0` payload (0 and null
/// share the zero word).
fn is_self_update(name: &str, value: &Expr) -> bool {
    let Expr::Binary { left, op, right } = value else {
        return false;
    };
    if !matches!(op, BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply) {
        return false;
    }
    let Expr::Identifier(base) = &**left else {
        return false;
    };
    base == name && arg_kind_code(right).is_some_and(|c| matches!(c, 'i' | 'f' | 's'))
}

fn count_assigns(stmts: &[Stmt], counts: &mut HashMap<String, usize>) {
    for s in stmts {
        match s.inner_stmt() {
            Stmt::Assign { name, value } if is_self_update(name, value) => {}
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

fn walk_scope_stmts<'a>(stmts: &'a [Stmt], out: &mut Vec<&'a Stmt>) {
    for s in stmts {
        out.push(s);
        match s.inner_stmt() {
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                walk_scope_stmts(then_block, out);
                if let Some(eb) = else_block {
                    walk_scope_stmts(eb, out);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                walk_scope_stmts(body, out);
            }
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                walk_scope_stmts(body, out);
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                walk_scope_stmts(try_block, out);
                walk_scope_stmts(catch_block, out);
                if let Some(fb) = finally_block {
                    walk_scope_stmts(fb, out);
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                walk_scope_stmts(std::slice::from_ref(inner), out);
            }
            _ => {}
        }
    }
}

fn literal_binds(stmts: &[Stmt], counts: &HashMap<String, usize>) -> HashMap<String, BindVal> {
    let mut out = HashMap::new();
    let mut flat = Vec::new();
    walk_scope_stmts(stmts, &mut flat);

    // Pass 1: direct literals and empty map() constructors.
    for s in &flat {
        if let Stmt::Let { name, value, .. } = s.inner_stmt() {
            if counts.get(name).copied().unwrap_or(0) != 1 {
                continue;
            }
            match value {
                Expr::String(s) => {
                    out.insert(name.clone(), BindVal::LitString(s.clone()));
                }
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
                Expr::Call { name: cname, args } => {
                    let bare = cname.rsplit("::").next().unwrap_or(cname.as_str());
                    let bare = bare.rsplit("__").next().unwrap_or(bare);
                    if bare == "map" && args.is_empty() {
                        out.insert(name.clone(), BindVal::LitMap(Vec::new()));
                    } else if let Some(c) = arg_kind_code(value) {
                        out.insert(name.clone(), BindVal::Kind(c));
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
    // Pass 1b: single-assignment map writes (`m["key"] = val` or `m[k] = val`).
    for s in &flat {
        if let Stmt::IndexAssign {
            array,
            index,
            value,
        } = s.inner_stmt()
        {
            let key_str = match index {
                Expr::String(k) => Some(k.clone()),
                Expr::Identifier(kid) => match out.get(kid) {
                    Some(BindVal::LitString(s)) => Some(s.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Expr::Identifier(map_name) = array {
                if let Some(key) = key_str {
                    if let Some(BindVal::LitMap(entries)) = out.get_mut(map_name) {
                        if let Some(pos) = entries.iter().position(|(k, _)| k == &key) {
                            entries[pos].1 = value.clone();
                        } else {
                            entries.push((key, value.clone()));
                        }
                    }
                } else {
                    out.remove(map_name);
                }
            }
        }
    }
    // Pass 1c: resolve let bindings whose value can be classified (e.g. `let u = m[k]` or `let u = m["id"]`).
    for s in &flat {
        if let Stmt::Let { name, value, .. } = s.inner_stmt() {
            if counts.get(name).copied().unwrap_or(0) != 1 {
                continue;
            }
            if !out.contains_key(name) {
                if let Some(c) = classify_arg(value, Some(&out)) {
                    out.insert(name.clone(), BindVal::Kind(c));
                } else if let Some(c) = classify_nullable_arg(value, Some(&out)) {
                    out.insert(name.clone(), BindVal::NullableKind(c));
                }
            }
        }
    }
    // Pass 2: single-level aliases (`let x = y`).
    for s in &flat {
        if let Stmt::Let { name, value, .. } = s.inner_stmt() {
            if counts.get(name).copied().unwrap_or(0) != 1 {
                continue;
            }
            if let Expr::Identifier(src) = value {
                if let Some(val) = out.get(src).cloned() {
                    out.insert(name.clone(), val);
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
            Some(BindVal::LitString(_)) => Some('s'),
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
            // alya-lang/alya#170: specialize call sites taking map-read args (e.g. c(m["id"]) or c(m[k]))
            // through single-assignment bindings populated by map literals/constructors and index-assigns.
            let key_str: Option<&str> = match &**index {
                Expr::String(key) => Some(key.as_str()),
                Expr::Identifier(kid) => match binds.get(kid) {
                    Some(BindVal::LitString(s)) => Some(s.as_str()),
                    _ => None,
                },
                _ => None,
            };
            if let (Expr::Identifier(arr), Some(key)) = (&**array, key_str) {
                if let Some(BindVal::LitMap(entries)) = binds.get(arr) {
                    if let Some((_, val)) = entries.iter().rev().find(|(k, _)| k == key) {
                        return arg_kind_code(val).or_else(|| classify_arg(val, Some(binds)));
                    }
                }
            }
            if let Expr::Identifier(arr) = &**array {
                if let Some(BindVal::LitMap(entries)) = binds.get(arr) {
                    if !entries.is_empty() {
                        let first_kind = arg_kind_code(&entries[0].1)
                            .or_else(|| classify_arg(&entries[0].1, Some(binds)));
                        if let Some(k) = first_kind {
                            if entries.iter().all(|(_, val)| {
                                arg_kind_code(val).or_else(|| classify_arg(val, Some(binds)))
                                    == Some(k)
                            }) {
                                return Some(k);
                            }
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// Classify a call argument as nullable: returns Some(c) when the value
/// is proven to be either null or of kind `c` (e.g. a map or array holding
/// both null and `c` values).
fn classify_nullable_arg(arg: &Expr, binds: Option<&HashMap<String, BindVal>>) -> Option<char> {
    let binds = binds?;
    match arg {
        Expr::Identifier(name) => match binds.get(name) {
            Some(BindVal::NullableKind(c)) => Some(*c),
            _ => None,
        },
        Expr::Index { array, .. } => {
            let arr_name = match &**array {
                Expr::Identifier(arr) => arr,
                _ => return None,
            };
            match binds.get(arr_name) {
                Some(BindVal::LitMap(entries)) => {
                    if entries.is_empty() {
                        return None;
                    }
                    let mut has_null = false;
                    let mut non_null_kind = None;
                    for (_, val) in entries {
                        let k = arg_kind_code(val).or_else(|| classify_arg(val, Some(binds)));
                        match k {
                            Some('n') => has_null = true,
                            Some(c) => {
                                if let Some(existing) = non_null_kind {
                                    if existing != c {
                                        return None;
                                    }
                                } else {
                                    non_null_kind = Some(c);
                                }
                            }
                            None => return None,
                        }
                    }
                    if has_null {
                        non_null_kind
                    } else {
                        None
                    }
                }
                Some(BindVal::LitArray(elems)) => {
                    if elems.is_empty() {
                        return None;
                    }
                    let mut has_null = false;
                    let mut non_null_kind = None;
                    for val in elems {
                        let k = arg_kind_code(val).or_else(|| classify_arg(val, Some(binds)));
                        match k {
                            Some('n') => has_null = true,
                            Some(c) => {
                                if let Some(existing) = non_null_kind {
                                    if existing != c {
                                        return None;
                                    }
                                } else {
                                    non_null_kind = Some(c);
                                }
                            }
                            None => return None,
                        }
                    }
                    if has_null {
                        non_null_kind
                    } else {
                        None
                    }
                }
                _ => None,
            }
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
/// The bare fallback bails on ambiguity (alya-lang/event#5): several
/// same-named methods (e.g. `TcpStream.write` and `RingBuffer.write`)
/// share one bare name, and picking by hash order misroutes receivers
/// to the wrong type (flaky across builds). With several matches the
/// call stays generic; codegen resolves it by receiver or fails loudly.
fn callee_def<'a>(candidates: &'a HashMap<String, Candidate>, name: &str) -> Option<&'a Candidate> {
    if name.contains("__spk__") {
        return None;
    }
    candidates.get(name).or_else(|| {
        let bare = name.rsplit("::").next().unwrap_or(name);
        let bare = bare.rsplit("__").next().unwrap_or(bare);
        let mut matches = candidates.iter().filter(|(k, _)| {
            let kb = k.rsplit("::").next().unwrap_or(k);
            let kb = kb.rsplit("__").next().unwrap_or(kb);
            kb == bare
        });
        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(first.1)
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
        // Publish the round's clones plus every clone their bodies request:
        // facade chains are deeper than one level (`Cache.set` →
        // `store_set` → `stored_repr`). Each clone gets its parameter kind
        // binds before its body is walked, so nested calls classify.
        let mut pending = new_clones;
        while !pending.is_empty() {
            let mut next: Vec<Stmt> = Vec::new();
            for mut clone in pending {
                if let Stmt::Function { name, .. } = clone.inner_stmt() {
                    if let Some((origin, codes)) = name.split_once("__spk__") {
                        if let Some(cand) = candidates.get(origin) {
                            let mut kind_binds = HashMap::new();
                            for (code, idx) in codes.chars().zip(cand.open_idx.iter()) {
                                if let Some(param) = cand.params.get(*idx) {
                                    kind_binds.insert(param.clone(), BindVal::Kind(code));
                                }
                            }
                            binds.insert(name.clone(), kind_binds);
                        }
                    }
                    // A later round may request a version already published;
                    // keep one definition (calls already point at it).
                    if program_has(&program.statements, name) {
                        continue;
                    }
                }
                walk_top_stmt(&mut clone, &candidates, &binds, &mut versions, &mut next);
                program.statements.push(clone);
            }
            pending = next;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str) -> Candidate {
        Candidate {
            name: name.to_string(),
            stmt: Stmt::Function {
                name: name.to_string(),
                params: vec!["self".to_string(), "data".to_string()],
                param_types: vec![None, None],
                return_type: None,
                defaults: vec![None, None],
                body: vec![],
                type_params: vec![],
                attributes: vec![],
            },
            params: vec!["self".to_string(), "data".to_string()],
            open_idx: vec![1],
            tests: true,
        }
    }

    #[test]
    fn callee_def_exact_name_wins() {
        let mut candidates = HashMap::new();
        candidates.insert("RingB__write".to_string(), candidate("RingB__write"));
        let found = callee_def(&candidates, "RingB__write").unwrap();
        assert_eq!(found.name, "RingB__write");
    }

    #[test]
    fn callee_def_single_bare_match_routes() {
        // One bare-name match keeps the legacy routing: there is no
        // alternative to confuse it with.
        let mut candidates = HashMap::new();
        candidates.insert("RingB__write".to_string(), candidate("RingB__write"));
        let found = callee_def(&candidates, "write").unwrap();
        assert_eq!(found.name, "RingB__write");
    }

    #[test]
    fn callee_def_ambiguous_bare_name_bails() {
        // alya-lang/event#5: several same-named methods share one bare
        // name (`TcpStream.write`, `RingBuffer.write`, ...). Routing by
        // hash order misroutes receivers to the wrong type (flaky across
        // builds), so ambiguous bare matches resolve to nothing and the
        // call stays generic for receiver-based codegen instead.
        let mut candidates = HashMap::new();
        candidates.insert("TcpS__write".to_string(), candidate("TcpS__write"));
        candidates.insert("RingB__write".to_string(), candidate("RingB__write"));
        candidates.insert("StreamB__write".to_string(), candidate("StreamB__write"));
        assert!(callee_def(&candidates, "write").is_none());
        // Exact names still resolve.
        assert_eq!(
            callee_def(&candidates, "TcpS__write").unwrap().name,
            "TcpS__write"
        );
    }

    #[test]
    fn collect_candidates_untyped_poly_indexing_and_casts() {
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "usame".to_string(),
                    params: vec!["a".to_string(), "b".to_string()],
                    param_types: vec![None, None],
                    return_type: None,
                    defaults: vec![None, None],
                    body: vec![Stmt::Return(Some(Expr::Binary {
                        left: Box::new(Expr::Index {
                            array: Box::new(Expr::Identifier("a".to_string())),
                            index: Box::new(Expr::Number(0)),
                        }),
                        op: BinaryOp::Equal,
                        right: Box::new(Expr::Index {
                            array: Box::new(Expr::Identifier("b".to_string())),
                            index: Box::new(Expr::Number(0)),
                        }),
                    }))],
                    type_params: vec![],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "to_num".to_string(),
                    params: vec!["v".to_string()],
                    param_types: vec![None],
                    return_type: None,
                    defaults: vec![None],
                    body: vec![Stmt::Return(Some(Expr::Call {
                        name: "int".to_string(),
                        args: vec![Expr::Identifier("v".to_string())],
                    }))],
                    type_params: vec![],
                    attributes: vec![],
                },
                Stmt::Function {
                    name: "annotated_geti".to_string(),
                    params: vec!["arr".to_string()],
                    param_types: vec![None],
                    return_type: Some("int".to_string()),
                    defaults: vec![None],
                    body: vec![Stmt::Return(Some(Expr::Index {
                        array: Box::new(Expr::Identifier("arr".to_string())),
                        index: Box::new(Expr::Number(0)),
                    }))],
                    type_params: vec![],
                    attributes: vec![],
                },
            ],
        };
        let candidates = collect_candidates(&program);
        assert!(
            candidates.contains_key("usame"),
            "usame should be a candidate"
        );
        assert!(candidates.get("usame").unwrap().tests);
        assert!(
            candidates.contains_key("to_num"),
            "to_num should be a candidate"
        );
        assert!(candidates.get("to_num").unwrap().tests);
        assert!(
            !candidates.contains_key("annotated_geti")
                || !candidates.get("annotated_geti").unwrap().tests
        );
    }

    #[test]
    fn arg_kind_code_classifies_negated_literals() {
        // `set("x", -2.5)` must specialize like `set("x", 2.5)`; without
        // this the generic clone's runtime `is null` rejected zero payloads.
        let neg_float = Expr::Unary {
            op: UnaryOp::Negate,
            expr: Box::new(Expr::Float(2.5)),
        };
        let neg_int = Expr::Unary {
            op: UnaryOp::Negate,
            expr: Box::new(Expr::Number(3)),
        };
        assert_eq!(arg_kind_code(&neg_float), Some('f'));
        assert_eq!(arg_kind_code(&neg_int), Some('i'));
    }

    #[test]
    fn arg_kind_code_classifies_string_concatenation() {
        // `"k" + str(i)` is always a string, so the whole call site stays
        // classifiable even with a dynamic key.
        let concat = Expr::Binary {
            left: Box::new(Expr::String("k".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Call {
                name: "str".to_string(),
                args: vec![Expr::Identifier("i".to_string())],
            }),
        };
        assert_eq!(arg_kind_code(&concat), Some('s'));
        let numeric = Expr::Binary {
            left: Box::new(Expr::Number(1)),
            op: BinaryOp::Add,
            right: Box::new(Expr::Number(2)),
        };
        assert_eq!(arg_kind_code(&numeric), None);
    }

    #[test]
    fn self_update_preserves_numeric_and_string_kinds() {
        // Loop counters (`i += 1`) keep the `let i = 0` kind: counting the
        // update as a rebind made `set(key, i)` unclassifiable.
        let inc = Expr::Binary {
            left: Box::new(Expr::Identifier("i".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Number(1)),
        };
        assert!(is_self_update("i", &inc));
        assert!(!is_self_update("j", &inc));
        let append = Expr::Binary {
            left: Box::new(Expr::Identifier("s".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::String("x".to_string())),
        };
        assert!(is_self_update("s", &append));
        let rebind = Expr::Binary {
            left: Box::new(Expr::Identifier("i".to_string())),
            op: BinaryOp::Add,
            right: Box::new(Expr::Call {
                name: "next".to_string(),
                args: vec![],
            }),
        };
        assert!(!is_self_update("i", &rebind));
    }
}
