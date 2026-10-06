//! Hoist nested function definitions to top level (alya-lang/alya#99).
//!
//! The parser and checker accept `function` definitions inside function
//! bodies with lexical scope and no capture, but codegen only collected
//! and emitted top-level functions, so nested calls linked against a
//! missing `fn_inner` symbol. This pass runs at codegen entry (after
//! checking, before DCE/inference): every nested definition is lifted
//! to a top-level `enclosing$inner` function and lexically-scoped
//! references are rewritten to the qualified name. Everything
//! downstream sees flat code, so DCE, CallIndex, inference, and
//! emission need no nested-awareness.
//!
//! The `$` separator is load-bearing, not cosmetic: the whole
//! inference/codegen stack resolves bare names by stripping `::` and
//! `__` suffixes, so an `enclosing__inner` shape would share its bare
//! `inner` with any same-named function and inherit its markers
//! (observed: int calls misread as float on arm64). `$` is rejected
//! by the lexer ("Unexpected character '$'"), so qualified names can
//! never collide with — or be consulted for — user code.
//!
//! Rules (mirror check scoping; capture stays rejected per #100):
//! - A nested definition is visible throughout its enclosing function
//!   body, including deeper blocks — but never outside it. References
//!   from outside keep today's behavior (loud link error, as for any
//!   unknown function).
//! - The innermost enclosing definition wins on shadowing.
//! - A same-named local variable (param/let/const/loop/catch binding
//!   anywhere in an enclosing body) shadows the definition: references
//!   are left alone, preserving indirect-call/value semantics exactly
//!   as for top-level shadowing.
//! - Only truly bare names are rewritten (no `::` or `__`); an
//!   explicitly qualified `outer__inner` reference resolves as written.
//! - Map-literal keys are never rewritten (conventionally data, and a
//!   function-valued key via bare name is pathological).
//! - Collisions with existing top-level names get a deterministic
//!   `$N` suffix (same unspellable shape, so still collision-free).

use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, Program, Stmt};

/// One visible nested definition: bare source name and hoisted target.
#[derive(Debug, Clone)]
struct ScopeDef {
    bare: String,
    qualified: String,
}

/// One lexical function scope: its own definitions plus the variable
/// names bound anywhere in its body (params included).
#[derive(Debug, Default)]
struct Frame {
    prefix: String,
    defs: Vec<ScopeDef>,
    vars: HashSet<String>,
}

/// Lift every nested function definition to top level, rewriting
/// lexically-scoped references to qualified names. Infallible by
/// construction: name collisions take a numeric suffix, and anything
/// unresolvable keeps today's loud link-time failure.
pub fn hoist_nested_functions(program: &mut Program) {
    // Symbol-namespace occupancy: top-level function names plus their
    // `::`→`__` mangled forms (the emitter mangles; see ctx.functions).
    let mut taken: HashSet<String> = HashSet::new();
    for stmt in &program.statements {
        if let Stmt::Function { name, .. } = stmt.inner_stmt() {
            taken.insert(name.clone());
            taken.insert(name.replace("::", "__"));
        }
    }
    let mut suffixes: HashMap<String, usize> = HashMap::new();
    let mut hoisted: Vec<Stmt> = Vec::new();
    for stmt in program.statements.iter_mut() {
        let head = match stmt.inner_stmt() {
            Stmt::Function { name, params, .. } => Some((name.clone(), params.clone())),
            _ => None,
        };
        if let Some((prefix, params)) = head {
            if let Stmt::Function { body, .. } = stmt.inner_stmt_mut() {
                let mut chain = vec![Frame {
                    prefix,
                    defs: Vec::new(),
                    vars: params.into_iter().collect(),
                }];
                hoist_body(body, &mut chain, &mut taken, &mut suffixes, &mut hoisted);
            }
        }
    }
    program.statements.extend(hoisted);
}

/// Process one function body: collect this scope's definitions and
/// variables, then rewrite references and extract hoisted definitions.
fn hoist_body(
    body: &mut Vec<Stmt>,
    chain: &mut Vec<Frame>,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    hoisted: &mut Vec<Stmt>,
) {
    let prefix = chain.last().map(|f| f.prefix.clone()).unwrap_or_default();
    collect_defs(body, &prefix, taken, suffixes, chain);
    collect_vars(body, chain);
    process_stmts(body, chain, taken, suffixes, hoisted);
}

/// Collect nested definitions belonging to the current (innermost)
/// scope: every `Function` in the subtree except inside deeper
/// function bodies (those belong to deeper scopes). Assigns qualified
/// names up front so forward references resolve.
fn collect_defs(
    stmts: &[Stmt],
    prefix: &str,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    chain: &mut Vec<Frame>,
) {
    for stmt in stmts {
        if is_def_stmt(stmt) {
            if let Stmt::Function { name, .. } = stmt.inner_stmt() {
                // `defer function` unwraps to the same inner function.
                let bare = match stmt.inner_stmt() {
                    Stmt::Defer(inner) => match inner.inner_stmt() {
                        Stmt::Function { name, .. } => name.clone(),
                        _ => continue,
                    },
                    _ => name.clone(),
                };
                let mut qualified = format!("{}${}", prefix, bare);
                if taken.contains(&qualified) || taken.contains(&qualified.replace("::", "__")) {
                    let n = suffixes.entry(qualified.clone()).or_insert(2);
                    loop {
                        let cand = format!("{}${}", qualified, *n);
                        *n += 1;
                        if !taken.contains(&cand) {
                            qualified = cand;
                            break;
                        }
                    }
                }
                taken.insert(qualified.clone());
                taken.insert(qualified.replace("::", "__"));
                if let Some(frame) = chain.last_mut() {
                    // First definition wins on same-scope duplicates
                    // (a duplicate top-level definition is a check-time
                    // error; nested duplicates degrade to first-wins
                    // rather than silent merging).
                    if !frame.defs.iter().any(|d| d.bare == bare) {
                        frame.defs.push(ScopeDef { bare, qualified });
                    }
                }
            }
            // Do not descend: the body belongs to a deeper scope.
        } else {
            collect_defs_in_stmt(stmt, prefix, taken, suffixes, chain);
        }
    }
}

/// A statement that introduces a nested definition: a function,
/// possibly under `pub` (peeled by `inner_stmt`). Anything under
/// `defer` keeps exactly today's behavior (dropped at emission, loud
/// link error on use).
fn is_def_stmt(stmt: &Stmt) -> bool {
    matches!(stmt.inner_stmt(), Stmt::Function { .. })
}

/// Recurse `collect_defs` through all child statement positions of one
/// non-definition statement.
fn collect_defs_in_stmt(
    stmt: &Stmt,
    prefix: &str,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    chain: &mut Vec<Frame>,
) {
    match stmt.inner_stmt() {
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            collect_defs(then_block, prefix, taken, suffixes, chain);
            if let Some(eb) = else_block {
                collect_defs(eb, prefix, taken, suffixes, chain);
            }
        }
        Stmt::While { body, .. } | Stmt::Repeat { body } => {
            collect_defs(body, prefix, taken, suffixes, chain);
        }
        Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            collect_defs(body, prefix, taken, suffixes, chain);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            collect_defs(try_block, prefix, taken, suffixes, chain);
            collect_defs(catch_block, prefix, taken, suffixes, chain);
            if let Some(fb) = finally_block {
                collect_defs(fb, prefix, taken, suffixes, chain);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            collect_defs_in_stmt(inner, prefix, taken, suffixes, chain);
        }
        _ => {}
    }
}

/// Collect variable names bound in this scope's subtree (params arrive
/// with the frame): let/const, loop variables, catch variables.
/// Nested function bodies and their params are excluded (deeper scope).
fn collect_vars(stmts: &[Stmt], chain: &mut Vec<Frame>) {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Function { .. } => {}
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                collect_vars(std::slice::from_ref(inner), chain);
            }
            Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                if let Some(frame) = chain.last_mut() {
                    frame.vars.insert(name.clone());
                }
            }
            Stmt::For { var, body, .. } => {
                if let Some(frame) = chain.last_mut() {
                    frame.vars.insert(var.clone());
                }
                collect_vars(body, chain);
            }
            Stmt::ForEach {
                var,
                value_var,
                body,
                ..
            } => {
                if let Some(frame) = chain.last_mut() {
                    frame.vars.insert(var.clone());
                    if let Some(vv) = value_var {
                        frame.vars.insert(vv.clone());
                    }
                }
                collect_vars(body, chain);
            }
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
                ..
            } => {
                if let Some(cv) = catch_var {
                    if let Some(frame) = chain.last_mut() {
                        frame.vars.insert(cv.clone());
                    }
                }
                collect_vars(try_block, chain);
                collect_vars(catch_block, chain);
                if let Some(fb) = finally_block {
                    collect_vars(fb, chain);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_vars(then_block, chain);
                if let Some(eb) = else_block {
                    collect_vars(eb, chain);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                collect_vars(body, chain);
            }
            _ => {}
        }
    }
}

/// Rewrite references and extract hoisted definitions at every depth
/// of one body (definitions inside blocks are extracted too — anything
/// less would keep today's link failure for those positions).
fn process_stmts(
    stmts: &mut Vec<Stmt>,
    chain: &mut Vec<Frame>,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    hoisted: &mut Vec<Stmt>,
) {
    let mut i = 0;
    while i < stmts.len() {
        if !is_def_stmt(&stmts[i]) {
            rewrite_contents(&mut stmts[i], chain, taken, suffixes, hoisted);
            i += 1;
            continue;
        }
        let mut def = stmts.remove(i);
        if let Stmt::Function {
            name, params, body, ..
        } = def.inner_stmt_mut()
        {
            let bare = name.clone();
            let qualified = resolve_in_chain(&bare, chain).unwrap_or_else(|| bare.clone());
            *name = qualified.clone();
            let child_vars: HashSet<String> = params.iter().cloned().collect();
            chain.push(Frame {
                prefix: qualified,
                defs: Vec::new(),
                vars: child_vars,
            });
            hoist_body(body, chain, taken, suffixes, hoisted);
            chain.pop();
        }
        // Defaults are written in the enclosing scope: rewrite them
        // against the current chain (the child frame is already popped).
        if let Stmt::Function { defaults, .. } = def.inner_stmt_mut() {
            for d in defaults.iter_mut().flatten() {
                rewrite_expr(d, chain);
            }
        }
        hoisted.push(def);
    }
}

/// Rewrite one non-definition statement in place. Blocks recurse
/// through process_stmts so definitions at any depth are extracted;
/// pure expressions rewrite directly against the live chain.
fn rewrite_contents(
    stmt: &mut Stmt,
    chain: &mut Vec<Frame>,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    hoisted: &mut Vec<Stmt>,
) {
    match stmt.inner_stmt_mut() {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_expr(condition, chain);
            process_stmts(then_block, chain, taken, suffixes, hoisted);
            if let Some(eb) = else_block {
                process_stmts(eb, chain, taken, suffixes, hoisted);
            }
        }
        Stmt::While { condition, body } => {
            rewrite_expr(condition, chain);
            process_stmts(body, chain, taken, suffixes, hoisted);
        }
        Stmt::Repeat { body } => {
            process_stmts(body, chain, taken, suffixes, hoisted);
        }
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_expr(start, chain);
            rewrite_expr(end, chain);
            process_stmts(body, chain, taken, suffixes, hoisted);
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_expr(iterable, chain);
            process_stmts(body, chain, taken, suffixes, hoisted);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            process_stmts(try_block, chain, taken, suffixes, hoisted);
            process_stmts(catch_block, chain, taken, suffixes, hoisted);
            if let Some(fb) = finally_block {
                process_stmts(fb, chain, taken, suffixes, hoisted);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            rewrite_single(inner, chain, taken, suffixes, hoisted);
        }
        _ => rewrite_stmt(stmt, chain),
    }
}

/// Rewrite a single wrapped statement (under `defer`/`pub`).
fn rewrite_single(
    stmt: &mut Stmt,
    chain: &mut Vec<Frame>,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    hoisted: &mut Vec<Stmt>,
) {
    if is_def_stmt(stmt) {
        // Multi-wrapped definitions (`defer pub function`) stay where
        // they are: renaming without extraction would desync from the
        // recorded scope, so they keep exactly today's behavior (loud
        // link error on use).
        return;
    }
    rewrite_contents(stmt, chain, taken, suffixes, hoisted);
}

/// Rewrite one statement's expressions in place. Function definitions
/// never reach here (extracted by process_stmts first).
fn rewrite_stmt(stmt: &mut Stmt, chain: &[Frame]) {
    match stmt.inner_stmt_mut() {
        Stmt::Say(e) | Stmt::Expr(e) => {
            rewrite_expr(e, chain);
        }
        Stmt::Return(opt) | Stmt::Throw(opt) => {
            if let Some(e) = opt {
                rewrite_expr(e, chain);
            }
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
            rewrite_expr(value, chain);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            rewrite_expr(array, chain);
            rewrite_expr(index, chain);
            rewrite_expr(value, chain);
        }
        Stmt::FieldAssign { object, value, .. } => {
            rewrite_expr(object, chain);
            rewrite_expr(value, chain);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_expr(condition, chain);
            for s in then_block.iter_mut() {
                rewrite_stmt(s, chain);
            }
            if let Some(eb) = else_block {
                for s in eb.iter_mut() {
                    rewrite_stmt(s, chain);
                }
            }
        }
        Stmt::While { condition, body } => {
            rewrite_expr(condition, chain);
            for s in body.iter_mut() {
                rewrite_stmt(s, chain);
            }
        }
        Stmt::Repeat { body } => {
            for s in body.iter_mut() {
                rewrite_stmt(s, chain);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_expr(start, chain);
            rewrite_expr(end, chain);
            for s in body.iter_mut() {
                rewrite_stmt(s, chain);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_expr(iterable, chain);
            for s in body.iter_mut() {
                rewrite_stmt(s, chain);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block.iter_mut() {
                rewrite_stmt(s, chain);
            }
            for s in catch_block.iter_mut() {
                rewrite_stmt(s, chain);
            }
            if let Some(fb) = finally_block {
                for s in fb.iter_mut() {
                    rewrite_stmt(s, chain);
                }
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            rewrite_stmt(inner, chain);
        }
        Stmt::StructDef { defaults, .. } => {
            for d in defaults.iter_mut().flatten() {
                rewrite_expr(d, chain);
            }
        }
        Stmt::EnumDef { variants, .. } => {
            for (_, v) in variants.iter_mut() {
                if let Some(e) = v {
                    rewrite_expr(e, chain);
                }
            }
        }
        Stmt::Function { .. } => {
            // Unreachable: process_stmts extracts definitions before
            // generic rewriting ever sees them.
            debug_assert!(false, "nested function escaped hoisting");
        }
        Stmt::Import { .. } | Stmt::ExternBlock { .. } | Stmt::InterfaceDef { .. } => {}
    }
}

/// Resolve a bare name through the scope chain, innermost first: a
/// bound variable shadows (return None = leave alone), otherwise the
/// innermost visible definition wins.
fn resolve_in_chain(bare: &str, chain: &[Frame]) -> Option<String> {
    if bare.contains("::") || bare.contains("__") {
        return None;
    }
    for frame in chain.iter().rev() {
        if frame.vars.contains(bare) {
            return None;
        }
        if let Some(d) = frame.defs.iter().find(|d| d.bare == bare) {
            return Some(d.qualified.clone());
        }
    }
    None
}

/// Rewrite calls and value references in one expression.
fn rewrite_expr(expr: &mut Expr, chain: &[Frame]) {
    match expr {
        Expr::Call { name, args } => {
            if let Some(q) = resolve_in_chain(name, chain) {
                *name = q;
            }
            for a in args.iter_mut() {
                rewrite_expr(a, chain);
            }
        }
        Expr::OptionalCall { callee, args } => {
            if let Some(q) = resolve_in_chain(callee, chain) {
                *callee = q;
            }
            for a in args.iter_mut() {
                rewrite_expr(a, chain);
            }
        }
        Expr::Identifier(name) => {
            if let Some(q) = resolve_in_chain(name, chain) {
                *name = q;
            }
        }
        Expr::Binary { left, right, .. } => {
            rewrite_expr(left, chain);
            rewrite_expr(right, chain);
        }
        Expr::Unary { expr: e, .. }
        | Expr::ForceUnwrap(e)
        | Expr::Cast { expr: e, .. }
        | Expr::TypeCheck { expr: e, .. } => {
            rewrite_expr(e, chain);
        }
        Expr::InterpolatedString(parts) | Expr::Array(parts) => {
            for p in parts.iter_mut() {
                rewrite_expr(p, chain);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            rewrite_expr(array, chain);
            rewrite_expr(index, chain);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            rewrite_expr(object, chain);
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields.iter_mut() {
                rewrite_expr(v, chain);
            }
        }
        Expr::Map(entries) => {
            // Keys are conventionally data; a function-valued key via
            // bare name is pathological — values rewrite normally.
            for (_, v) in entries.iter_mut() {
                rewrite_expr(v, chain);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_expr(condition, chain);
            rewrite_expr(then_branch, chain);
            rewrite_expr(else_branch, chain);
        }
        Expr::NullCoalesce { value, default } => {
            rewrite_expr(value, chain);
            rewrite_expr(default, chain);
        }
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null => {}
    }
}
