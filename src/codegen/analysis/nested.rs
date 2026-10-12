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

use crate::ast::{Attribute, Expr, Program, Stmt};

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

/// Closure-capture plan (alya-lang/alya#100), computed alongside
/// hoisting and consumed by codegen emission:
/// - `owners`: owner fn (qualified) -> captured var names (first-capture order).
/// - `chains`: fn -> ancestor owners with captures, outermost-first
///   (the runtime chain-array content order).
/// - `captured`: fn -> vars it reads as captured (needs the chain param).
/// - `parents`: hoisted/adopted fn (qualified) -> parent fn (qualified).
#[derive(Debug, Default)]
pub struct CapturePlan {
    pub owners: HashMap<String, Vec<String>>,
    pub chains: HashMap<String, Vec<String>>,
    pub captured: HashMap<String, Vec<String>>,
    pub parents: HashMap<String, String>,
}

impl CapturePlan {
    /// Functions whose body reads via chain (take the chain param and
    /// receive chain args at direct calls).
    pub fn needs_chain(&self, name: &str) -> bool {
        self.captured.get(name).is_some_and(|v| !v.is_empty())
    }
}

/// Lift every nested function definition to top level, rewriting
/// lexically-scoped references to qualified names. Infallible by
/// construction: name collisions take a numeric suffix, and anything
/// unresolvable keeps today's loud link-time failure.
///
/// Parse-desugared functions (`alya::enclosing`) are spliced into
/// their lexical parent first, so they hoist structurally like any
/// nested definition. Returns the closure-capture plan alongside.
pub fn hoist_nested_functions(program: &mut Program) -> CapturePlan {
    let mut plan = CapturePlan::default();
    splice_adopted(program, &mut plan.parents);
    // Capture analysis runs on the structural program (post-splice,
    // pre-extraction) with live scope stacks; paths stitch to qualified
    // names after extraction via `qualmap`.
    let raw_caps = collect_captures(program);
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
    let mut qualmap: HashMap<(String, String), String> = HashMap::new();
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
                hoist_body(
                    body,
                    &mut chain,
                    &mut taken,
                    &mut suffixes,
                    &mut hoisted,
                    &mut plan.parents,
                    &mut qualmap,
                );
            }
        }
    }
    program.statements.extend(hoisted);
    finish_capture_plan(&mut plan, raw_caps, &qualmap);
    rewrite_captures(program, &plan);
    plan
}

/// Reads the `alya::enclosing` attribute (lexical parent recorded by
/// the parser for desugared functions).
fn enclosing_attr_of(attributes: &[crate::ast::Attribute]) -> Option<String> {
    attributes
        .iter()
        .find(|a| a.name == "alya::enclosing")
        .and_then(|a| {
            a.args
                .iter()
                .find(|(k, _)| k.is_none())
                .map(|(_, v)| v.clone())
        })
}

/// Splices parse-desugared functions into their lexical parent's body
/// so hoisting treats them structurally. Records `parents[child] =
/// parent` for the capture plan. Global-scope lambdas (no attribute)
/// stay top-level; missing parents (defensive: same-file lexical order
/// plus import renames keep them consistent) stay put and degrade to
/// today's check-time behavior for captures.
fn splice_adopted(program: &mut Program, parents: &mut HashMap<String, String>) {
    let mut moves: Vec<(String, String)> = Vec::new();
    for stmt in &program.statements {
        if let Stmt::Function {
            name, attributes, ..
        } = stmt.inner_stmt()
        {
            if let Some(enc) = enclosing_attr_of(attributes) {
                moves.push((name.clone(), enc));
            }
        }
    }
    for (child, parent) in moves {
        let pos = program
            .statements
            .iter()
            .position(|s| matches!(s.inner_stmt(), Stmt::Function { name, .. } if name == &child));
        let Some(idx) = pos else { continue };
        let child_stmt = program.statements.remove(idx);
        if let Some(body) = find_fn_body_mut(&mut program.statements, &parent) {
            body.push(child_stmt);
            parents.insert(child, parent);
        } else {
            // Defensive: restore top-level (captures keep check-time
            // behavior; see the checker pre-scans).
            program.statements.push(child_stmt);
        }
    }
}

/// Finds a function body by name, descending through function bodies
/// and blocks.
fn find_fn_body_mut<'a>(stmts: &'a mut [Stmt], name: &str) -> Option<&'a mut Vec<Stmt>> {
    for stmt in stmts.iter_mut() {
        match stmt.inner_stmt_mut() {
            Stmt::Function {
                name: fname, body, ..
            } => {
                if fname == name {
                    return Some(body);
                }
                if let Some(found) = find_fn_body_mut(body, name) {
                    return Some(found);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(found) = find_fn_body_mut(then_block, name) {
                    return Some(found);
                }
                if let Some(eb) = else_block {
                    if let Some(found) = find_fn_body_mut(eb, name) {
                        return Some(found);
                    }
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                if let Some(found) = find_fn_body_mut(body, name) {
                    return Some(found);
                }
            }
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                if let Some(found) = find_fn_body_mut(body, name) {
                    return Some(found);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                if let Some(found) = find_fn_body_mut(try_block, name) {
                    return Some(found);
                }
                if let Some(found) = find_fn_body_mut(catch_block, name) {
                    return Some(found);
                }
                if let Some(fb) = finally_block {
                    if let Some(found) = find_fn_body_mut(fb, name) {
                        return Some(found);
                    }
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                if let Some(found) = find_fn_body_mut(std::slice::from_mut(inner), name) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Process one function body: collect this scope's definitions and
/// variables, then rewrite references and extract hoisted definitions.
fn hoist_body(
    body: &mut Vec<Stmt>,
    chain: &mut Vec<Frame>,
    taken: &mut HashSet<String>,
    suffixes: &mut HashMap<String, usize>,
    hoisted: &mut Vec<Stmt>,
    parents: &mut HashMap<String, String>,
    qualmap: &mut HashMap<(String, String), String>,
) {
    let prefix = chain.last().map(|f| f.prefix.clone()).unwrap_or_default();
    collect_defs(body, &prefix, taken, suffixes, chain, qualmap);
    collect_vars(body, chain);
    process_stmts(body, chain, taken, suffixes, hoisted, parents, qualmap);
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
    qualmap: &mut HashMap<(String, String), String>,
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
                // Record the rename for capture-plan stitching.
                qualmap.insert((prefix.to_string(), bare.clone()), qualified.clone());
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
            collect_defs_in_stmt(stmt, prefix, taken, suffixes, chain, qualmap);
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
    qualmap: &mut HashMap<(String, String), String>,
) {
    match stmt.inner_stmt() {
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            collect_defs(then_block, prefix, taken, suffixes, chain, qualmap);
            if let Some(eb) = else_block {
                collect_defs(eb, prefix, taken, suffixes, chain, qualmap);
            }
        }
        Stmt::While { body, .. } | Stmt::Repeat { body } => {
            collect_defs(body, prefix, taken, suffixes, chain, qualmap);
        }
        Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
            collect_defs(body, prefix, taken, suffixes, chain, qualmap);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            collect_defs(try_block, prefix, taken, suffixes, chain, qualmap);
            collect_defs(catch_block, prefix, taken, suffixes, chain, qualmap);
            if let Some(fb) = finally_block {
                collect_defs(fb, prefix, taken, suffixes, chain, qualmap);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            collect_defs_in_stmt(inner, prefix, taken, suffixes, chain, qualmap);
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
    parents: &mut HashMap<String, String>,
    qualmap: &mut HashMap<(String, String), String>,
) {
    let mut i = 0;
    while i < stmts.len() {
        if !is_def_stmt(&stmts[i]) {
            rewrite_contents(
                &mut stmts[i],
                chain,
                taken,
                suffixes,
                hoisted,
                parents,
                qualmap,
            );
            i += 1;
            continue;
        }
        let parent_prefix = chain.last().map(|f| f.prefix.clone()).unwrap_or_default();
        let mut def = stmts.remove(i);
        if let Stmt::Function {
            name, params, body, ..
        } = def.inner_stmt_mut()
        {
            let bare = name.clone();
            let qualified = resolve_in_chain(&bare, chain).unwrap_or_else(|| {
                // `__`-named nested definitions (e.g. spliced adopted
                // lambdas): the `__` guard above rejects them, but they
                // are real definitions with collect-assigned names —
                // look those up directly. Var-shadowed plain names keep
                // the bare fallback exactly as before.
                if bare.contains("__") {
                    chain
                        .iter()
                        .rev()
                        .find_map(|f| f.defs.iter().find(|d| d.bare == bare))
                        .map(|d| d.qualified.clone())
                        .unwrap_or_else(|| bare.clone())
                } else {
                    bare.clone()
                }
            });
            *name = qualified.clone();
            // Record the lexical parent for the capture plan: the
            // innermost frame at extraction time.
            parents.insert(qualified.clone(), parent_prefix);
            let child_vars: HashSet<String> = params.iter().cloned().collect();
            chain.push(Frame {
                prefix: qualified,
                defs: Vec::new(),
                vars: child_vars,
            });
            hoist_body(body, chain, taken, suffixes, hoisted, parents, qualmap);
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
    parents: &mut HashMap<String, String>,
    qualmap: &mut HashMap<(String, String), String>,
) {
    match stmt.inner_stmt_mut() {
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_expr(condition, chain);
            process_stmts(
                then_block, chain, taken, suffixes, hoisted, parents, qualmap,
            );
            if let Some(eb) = else_block {
                process_stmts(eb, chain, taken, suffixes, hoisted, parents, qualmap);
            }
        }
        Stmt::While { condition, body } => {
            rewrite_expr(condition, chain);
            process_stmts(body, chain, taken, suffixes, hoisted, parents, qualmap);
        }
        Stmt::Repeat { body } => {
            process_stmts(body, chain, taken, suffixes, hoisted, parents, qualmap);
        }
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_expr(start, chain);
            rewrite_expr(end, chain);
            process_stmts(body, chain, taken, suffixes, hoisted, parents, qualmap);
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_expr(iterable, chain);
            process_stmts(body, chain, taken, suffixes, hoisted, parents, qualmap);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            process_stmts(try_block, chain, taken, suffixes, hoisted, parents, qualmap);
            process_stmts(
                catch_block,
                chain,
                taken,
                suffixes,
                hoisted,
                parents,
                qualmap,
            );
            if let Some(fb) = finally_block {
                process_stmts(fb, chain, taken, suffixes, hoisted, parents, qualmap);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => {
            rewrite_single(inner, chain, taken, suffixes, hoisted, parents, qualmap);
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
    parents: &mut HashMap<String, String>,
    qualmap: &mut HashMap<(String, String), String>,
) {
    if is_def_stmt(stmt) {
        // Multi-wrapped definitions (`defer pub function`) stay where
        // they are: renaming without extraction would desync from the
        // recorded scope, so they keep exactly today's behavior (loud
        // link error on use).
        return;
    }
    rewrite_contents(stmt, chain, taken, suffixes, hoisted, parents, qualmap);
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
/// innermost visible definition wins. Adopted lambdas (`__alya_*`,
/// spliced structural by hoisting) resolve through frame defs — after
/// stripping one module qualifier (imports rename them to
/// `prefix::__alya_*` while tracking the enclosing attribute). Other
/// `::`/`__` spellings are already qualified and resolve as written
/// (None). Narrowed to `__alya_*` so same-spelled method calls
/// (`T__m`) never route into nested definitions.
fn resolve_in_chain(bare: &str, chain: &[Frame]) -> Option<String> {
    // Module-qualified adopted reference: match the trailing segment.
    let probe = bare.rsplit("::").next().unwrap_or(bare);
    if !probe.starts_with("__alya_") && (bare.contains("::") || bare.contains("__")) {
        return None;
    }
    for frame in chain.iter().rev() {
        if frame.vars.contains(bare) || frame.vars.contains(probe) {
            return None;
        }
        // Adopted defs may carry an import prefix
        // (`__priv_m::__alya_spread_arr_0`) while the reference spells
        // the same: match exact first, then trailing segment. Scoped
        // defs are unique post-validation, so the suffix is exact.
        if let Some(d) = frame.defs.iter().find(|d| {
            d.bare == bare || (probe.starts_with("__alya_") && d.bare.ends_with(probe))
        }) {
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

// ---- closure-capture analysis, phase 0.5 (alya-lang/alya#100) ----
//
// Runs after adopted splicing, before hoisting extraction, while the
// program is still structural. Walks every function body in order
// with live scope stacks (mirroring check discipline: sequential,
// block-aware), analyzing each nested body at its definition point
// and each adopted body at its first creation site. Loop/catch-owned
// references are an internal error here: the checker pre-scan rejects
// them before codegen ever runs.

/// Binder kinds for capture ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CapBinder {
    Param,
    Let,
    Loop,
    Catch,
}

/// One live scope level during the capture walk.
struct CapScope {
    /// Structural path of the nearest enclosing function (root segment
    /// is a stable top-level name; deeper segments are bare).
    fn_path: Vec<String>,
    /// Whether this level is a function frame (holds `defs`).
    is_fn: bool,
    /// Sequentially bound names (this level).
    bound: HashSet<String>,
    kinds: HashMap<String, CapBinder>,
    /// Nested-def bares visible at this function level (pre-collected).
    defs: HashSet<String>,
}

/// One capture record: function `func_path` reads `var` owned by
/// `owner_path` (structural paths; stitched to qualified names after
/// hoisting extraction).
#[derive(Debug, Clone)]
struct CapRef {
    func_path: Vec<String>,
    var: String,
    owner_path: Vec<String>,
}

struct CapWalk {
    /// Adopted lambda bare names (carry `alya::enclosing`).
    adopted: HashSet<String>,
    /// Adopted bodies already analyzed (first creation wins).
    analyzed_lambda: HashSet<String>,
    out: Vec<CapRef>,
}

impl CapWalk {
    /// Records a capture reference resolved through the live stack.
    /// Def-hits (nested functions, renamed by hoisting) and globals
    /// (no binding) produce no record. Same-function reads (including
    /// the function's own loop/catch bindings) are plain locals, not
    /// captures. Loop/catch-owned references from a *different*
    /// function are unreachable post-check (the checker pre-scan
    /// rejects them) and panic loudly.
    fn capture_ref(&mut self, var: &str, stack: &[CapScope]) {
        let current = self.current_fn_path(stack);
        for level in stack.iter().rev() {
            if let Some(kind) = level.kinds.get(var) {
                if level.fn_path == current {
                    // Own frame (params, lets, loop/catch locals).
                    return;
                }
                match kind {
                    CapBinder::Loop | CapBinder::Catch => {
                        panic!(
                            "internal error: capture of loop/catch-bound '{}' escaped check-time rejection (alya-lang/alya#100)",
                            var
                        );
                    }
                    CapBinder::Param | CapBinder::Let => {
                        self.out.push(CapRef {
                            func_path: current,
                            var: var.to_string(),
                            owner_path: level.fn_path.clone(),
                        });
                        return;
                    }
                }
            }
            if level.is_fn && level.defs.contains(var) {
                // Nested definition: renamed by hoisting, not captured.
                return;
            }
        }
        // Global (top-level let, function, struct, ...): direct access,
        // no cell.
    }

    fn current_fn_path(&self, stack: &[CapScope]) -> Vec<String> {
        stack
            .iter()
            .rev()
            .find(|l| l.is_fn)
            .map(|l| l.fn_path.clone())
            .unwrap_or_default()
    }
}

/// Collects adopted lambda names anywhere in the program (they carry
/// `alya::enclosing`, including spliced ones).
fn collect_adopted(stmts: &[Stmt], set: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Function {
                name,
                attributes,
                body,
                ..
            } => {
                if enclosing_attr_of(attributes).is_some() {
                    set.insert(name.clone());
                }
                collect_adopted(body, set);
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                collect_adopted(then_block, set);
                if let Some(eb) = else_block {
                    collect_adopted(eb, set);
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => collect_adopted(body, set),
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } => collect_adopted(body, set),
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                collect_adopted(try_block, set);
                collect_adopted(catch_block, set);
                if let Some(fb) = finally_block {
                    collect_adopted(fb, set);
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                collect_adopted(std::slice::from_ref(inner), set)
            }
            _ => {}
        }
    }
}

/// Pre-collects nested-def bare names of one body (any depth except
/// deeper function bodies), mirroring `collect_defs`.
fn collect_cap_defs(stmts: &[Stmt], defs: &mut HashSet<String>) {
    for stmt in stmts {
        if let Stmt::Function { name, .. } = stmt.inner_stmt() {
            defs.insert(name.clone());
        } else {
            match stmt.inner_stmt() {
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    collect_cap_defs(then_block, defs);
                    if let Some(eb) = else_block {
                        collect_cap_defs(eb, defs);
                    }
                }
                Stmt::While { body, .. } | Stmt::Repeat { body } => collect_cap_defs(body, defs),
                Stmt::For { body, .. } | Stmt::ForEach { body, .. } => collect_cap_defs(body, defs),
                Stmt::TryCatch {
                    try_block,
                    catch_block,
                    finally_block,
                    ..
                } => {
                    collect_cap_defs(try_block, defs);
                    collect_cap_defs(catch_block, defs);
                    if let Some(fb) = finally_block {
                        collect_cap_defs(fb, defs);
                    }
                }
                Stmt::Defer(inner) | Stmt::Pub(inner) => {
                    collect_cap_defs(std::slice::from_ref(inner), defs)
                }
                _ => {}
            }
        }
    }
}

/// Finds a function statement by bare name (DFS through bodies and blocks).
fn find_fn<'a>(stmts: &'a [Stmt], name: &str) -> Option<&'a Stmt> {
    for stmt in stmts {
        match stmt.inner_stmt() {
            Stmt::Function { name: n, body, .. } => {
                if n == name {
                    return Some(stmt);
                }
                if let Some(found) = find_fn(body, name) {
                    return Some(found);
                }
            }
            Stmt::If {
                then_block,
                else_block,
                ..
            } => {
                if let Some(found) = find_fn(then_block, name) {
                    return Some(found);
                }
                if let Some(eb) = else_block {
                    if let Some(found) = find_fn(eb, name) {
                        return Some(found);
                    }
                }
            }
            Stmt::While { body, .. } | Stmt::Repeat { body } => {
                if let Some(found) = find_fn(body, name) {
                    return Some(found);
                }
            }
            Stmt::For { body, .. } | Stmt::ForEach { body, .. } => {
                if let Some(found) = find_fn(body, name) {
                    return Some(found);
                }
            }
            Stmt::TryCatch {
                try_block,
                catch_block,
                finally_block,
                ..
            } => {
                if let Some(found) = find_fn(try_block, name) {
                    return Some(found);
                }
                if let Some(found) = find_fn(catch_block, name) {
                    return Some(found);
                }
                if let Some(fb) = finally_block {
                    if let Some(found) = find_fn(fb, name) {
                        return Some(found);
                    }
                }
            }
            Stmt::Defer(inner) | Stmt::Pub(inner) => {
                if let Some(found) = find_fn(std::slice::from_ref(inner), name) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

/// Binds a name with a kind in the innermost live level.
fn cap_bind(stack: &mut [CapScope], name: &str, kind: CapBinder) {
    if let Some(top) = stack.last_mut() {
        top.bound.insert(name.to_string());
        top.kinds.insert(name.to_string(), kind);
    }
}

/// Runs the capture walk over the whole program (post-splice,
/// pre-extraction). Returns raw capture records with structural paths.
fn collect_captures(program: &Program) -> Vec<CapRef> {
    let mut adopted = HashSet::new();
    collect_adopted(&program.statements, &mut adopted);
    let mut walk = CapWalk {
        adopted,
        analyzed_lambda: HashSet::new(),
        out: Vec::new(),
    };
    for stmt in &program.statements {
        if let Stmt::Function {
            name, params, body, ..
        } = stmt.inner_stmt()
        {
            // Adopted roots are analyzed at their creation sites, not here.
            if walk.adopted.contains(name) {
                continue;
            }
            let mut defs = HashSet::new();
            collect_cap_defs(body, &mut defs);
            let mut stack = vec![CapScope {
                fn_path: vec![name.clone()],
                is_fn: true,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs,
            }];
            for p in params {
                cap_bind(&mut stack, p, CapBinder::Param);
            }
            cap_walk_body(&program.statements, body, &mut stack, &mut walk);
        }
    }
    walk.out
}

/// Walks one body in order with the live stack.
fn cap_walk_body(root: &[Stmt], body: &[Stmt], stack: &mut Vec<CapScope>, walk: &mut CapWalk) {
    for stmt in body {
        cap_walk_stmt(root, stmt, stack, walk);
    }
}

fn cap_walk_stmt(root: &[Stmt], stmt: &Stmt, stack: &mut Vec<CapScope>, walk: &mut CapWalk) {
    match stmt.inner_stmt() {
        Stmt::Say(e) | Stmt::Expr(e) => cap_walk_expr(root, e, stack, walk),
        Stmt::Return(opt) | Stmt::Throw(opt) => {
            if let Some(e) = opt {
                cap_walk_expr(root, e, stack, walk);
            }
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::Let { name, value, .. } | Stmt::Const { name, value } => {
            // Mirror the check pre-bind: a closure right-hand side sees
            // its own binding (self-recursion). Body analysis triggers
            // from the value walk below (first wins).
            if let Expr::Identifier(n) = value {
                if walk.adopted.contains(n) {
                    cap_bind(stack, name, CapBinder::Let);
                }
            }
            cap_walk_expr(root, value, stack, walk);
            cap_bind(stack, name, CapBinder::Let);
        }
        Stmt::Assign { name, value } => {
            // Mirror the check pre-bind for new names.
            if let Expr::Identifier(n) = value {
                if walk.adopted.contains(n) && !stack.iter().any(|l| l.bound.contains(name)) {
                    cap_bind(stack, name, CapBinder::Let);
                }
            }
            cap_walk_expr(root, value, stack, walk);
            // Assignment to an outer binding is a capture-write.
            walk.capture_ref(name, stack);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            cap_walk_expr(root, array, stack, walk);
            cap_walk_expr(root, index, stack, walk);
            cap_walk_expr(root, value, stack, walk);
        }
        Stmt::FieldAssign { object, value, .. } => {
            cap_walk_expr(root, object, stack, walk);
            cap_walk_expr(root, value, stack, walk);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            cap_walk_expr(root, condition, stack, walk);
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path,
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_walk_body(root, then_block, stack, walk);
            stack.pop();
            if let Some(eb) = else_block {
                let path = walk.current_fn_path(stack);
                stack.push(CapScope {
                    fn_path: path,
                    is_fn: false,
                    bound: HashSet::new(),
                    kinds: HashMap::new(),
                    defs: HashSet::new(),
                });
                cap_walk_body(root, eb, stack, walk);
                stack.pop();
            }
        }
        Stmt::While { condition, body } => {
            cap_walk_expr(root, condition, stack, walk);
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path,
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_walk_body(root, body, stack, walk);
            stack.pop();
        }
        Stmt::Repeat { body } => {
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path,
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_walk_body(root, body, stack, walk);
            stack.pop();
        }
        Stmt::For {
            var,
            start,
            end,
            body,
            ..
        } => {
            cap_walk_expr(root, start, stack, walk);
            cap_walk_expr(root, end, stack, walk);
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path,
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_bind(stack, var, CapBinder::Loop);
            cap_walk_body(root, body, stack, walk);
            stack.pop();
        }
        Stmt::ForEach {
            var,
            value_var,
            iterable,
            body,
        } => {
            cap_walk_expr(root, iterable, stack, walk);
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path,
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_bind(stack, var, CapBinder::Loop);
            if let Some(vv) = value_var {
                cap_bind(stack, vv, CapBinder::Loop);
            }
            cap_walk_body(root, body, stack, walk);
            stack.pop();
        }
        Stmt::TryCatch {
            try_block,
            catch_var,
            catch_block,
            finally_block,
        } => {
            let path = walk.current_fn_path(stack);
            stack.push(CapScope {
                fn_path: path.clone(),
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            cap_walk_body(root, try_block, stack, walk);
            stack.pop();
            stack.push(CapScope {
                fn_path: path.clone(),
                is_fn: false,
                bound: HashSet::new(),
                kinds: HashMap::new(),
                defs: HashSet::new(),
            });
            if let Some(cv) = catch_var {
                cap_bind(stack, cv, CapBinder::Catch);
            }
            cap_walk_body(root, catch_block, stack, walk);
            stack.pop();
            if let Some(fb) = finally_block {
                stack.push(CapScope {
                    fn_path: path,
                    is_fn: false,
                    bound: HashSet::new(),
                    kinds: HashMap::new(),
                    defs: HashSet::new(),
                });
                cap_walk_body(root, fb, stack, walk);
                stack.pop();
            }
        }
        Stmt::Function {
            name, params, body, ..
        } => {
            // Nested definition: analyze the body now with the live
            // parent stack (definition-point state), then bind the bare
            // name. Adopted bodies are creation-driven (skipped here).
            if !walk.adopted.contains(name) {
                let mut child_path = walk.current_fn_path(stack);
                child_path.push(name.clone());
                let mut defs = HashSet::new();
                collect_cap_defs(body, &mut defs);
                stack.push(CapScope {
                    fn_path: child_path,
                    is_fn: true,
                    bound: HashSet::new(),
                    kinds: HashMap::new(),
                    defs,
                });
                for p in params {
                    cap_bind(stack, p, CapBinder::Param);
                }
                cap_walk_body(root, body, stack, walk);
                stack.pop();
            }
            // Defaults evaluate in the enclosing scope: analyze with the
            // live (parent) stack.
            if let Stmt::Function { defaults, .. } = stmt.inner_stmt() {
                for d in defaults.iter().flatten() {
                    cap_walk_expr(root, d, stack, walk);
                }
            }
            if let Some(top) = stack.last_mut() {
                top.bound.insert(name.clone());
            }
        }
        Stmt::Defer(inner) => {
            // Opaque for definitions (kept in place with link-error
            // behavior, mirroring the unchecked checker path): bind the
            // name and walk defaults only. Transparent otherwise.
            match inner.inner_stmt() {
                Stmt::Function { name, defaults, .. } => {
                    for d in defaults.iter().flatten() {
                        cap_walk_expr(root, d, stack, walk);
                    }
                    if let Some(top) = stack.last_mut() {
                        top.bound.insert(name.clone());
                    }
                }
                _ => cap_walk_stmt(root, inner, stack, walk),
            }
        }
        Stmt::Pub(inner) => cap_walk_stmt(root, inner, stack, walk),
        // Mirror `check_stmt`: deferred/published, struct/enum,
        // import/extern contents are not checked scopes.
        _ => {}
    }
}

/// Analyzes an adopted lambda body with the live stack at its first
/// creation site (first wins).
fn cap_analyze_lambda(root: &[Stmt], name: &str, stack: &mut Vec<CapScope>, walk: &mut CapWalk) {
    if !walk.analyzed_lambda.insert(name.to_string()) {
        return;
    }
    let Some(found) = find_fn(root, name) else {
        return;
    };
    if let Stmt::Function {
        params,
        body,
        defaults,
        ..
    } = found.inner_stmt()
    {
        // Defaults belong to the enclosing scope: analyze with the live
        // (parent) stack before pushing the child frame.
        for d in defaults.iter().flatten() {
            cap_walk_expr(root, d, stack, walk);
        }
        let mut child_path = walk.current_fn_path(stack);
        child_path.push(name.to_string());
        let mut defs = HashSet::new();
        collect_cap_defs(body, &mut defs);
        stack.push(CapScope {
            fn_path: child_path,
            is_fn: true,
            bound: HashSet::new(),
            kinds: HashMap::new(),
            defs,
        });
        for p in params {
            cap_bind(stack, p, CapBinder::Param);
        }
        cap_walk_body(root, body, stack, walk);
        stack.pop();
    }
}

fn cap_walk_expr(root: &[Stmt], expr: &Expr, stack: &mut Vec<CapScope>, walk: &mut CapWalk) {
    match expr {
        Expr::Identifier(name) => {
            // Adopted references in value position are creations when
            // reached here outside a creation-let (e.g. call arguments):
            // analyze with the live stack (first wins).
            if walk.adopted.contains(name) {
                cap_analyze_lambda(root, name, stack, walk);
            }
            walk.capture_ref(name, stack);
        }
        Expr::Call { name, args } => {
            walk.capture_ref(name, stack);
            for a in args {
                cap_walk_expr(root, a, stack, walk);
            }
        }
        Expr::OptionalCall { callee, args } => {
            walk.capture_ref(callee, stack);
            for a in args {
                cap_walk_expr(root, a, stack, walk);
            }
        }
        Expr::Binary { left, right, .. } => {
            cap_walk_expr(root, left, stack, walk);
            cap_walk_expr(root, right, stack, walk);
        }
        Expr::Unary { expr: e, .. }
        | Expr::ForceUnwrap(e)
        | Expr::Cast { expr: e, .. }
        | Expr::TypeCheck { expr: e, .. } => {
            cap_walk_expr(root, e, stack, walk);
        }
        Expr::InterpolatedString(parts) | Expr::Array(parts) => {
            for p in parts {
                cap_walk_expr(root, p, stack, walk);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            cap_walk_expr(root, array, stack, walk);
            cap_walk_expr(root, index, stack, walk);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            cap_walk_expr(root, object, stack, walk);
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields {
                cap_walk_expr(root, v, stack, walk);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                cap_walk_expr(root, k, stack, walk);
                cap_walk_expr(root, v, stack, walk);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            cap_walk_expr(root, condition, stack, walk);
            cap_walk_expr(root, then_branch, stack, walk);
            cap_walk_expr(root, else_branch, stack, walk);
        }
        Expr::NullCoalesce { value, default } => {
            cap_walk_expr(root, value, stack, walk);
            cap_walk_expr(root, default, stack, walk);
        }
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null => {}
    }
}

// ---- capture plan finalization + rewrites, phase 2 (alya-lang/alya#100) ----

// Stitches a structural path (root top-level name + bare segments) to
// a qualified name via the recorded rename map.
fn qualify_path(path: &[String], qualmap: &HashMap<(String, String), String>) -> Option<String> {
    let mut q = path.first()?.clone();
    for bare in &path[1..] {
        q = qualmap.get(&(q.clone(), bare.clone()))?.clone();
    }
    Some(q)
}

// Folds raw capture records into the plan: stitch paths to qualified
// names, then derive ancestor chains (outermost-first).
fn finish_capture_plan(
    plan: &mut CapturePlan,
    raw: Vec<CapRef>,
    qualmap: &HashMap<(String, String), String>,
) {
    // Pass 1: owners — strict descendant capture only. Self-hits (a
    // function reading its own bound names) are not captures.
    for cap in &raw {
        let (Some(fq), Some(oq)) = (
            qualify_path(&cap.func_path, qualmap),
            qualify_path(&cap.owner_path, qualmap),
        ) else {
            continue;
        };
        if fq == oq {
            continue;
        }
        let owned = plan.owners.entry(oq).or_default();
        if !owned.contains(&cap.var) {
            owned.push(cap.var.clone());
        }
    }
    // Pass 2: captured reads — strict captures plus owner-frame reads
    // of owned cells (the cell is the single home: every access,
    // including the owner's own, routes through it).
    for cap in &raw {
        let (Some(fq), Some(oq)) = (
            qualify_path(&cap.func_path, qualmap),
            qualify_path(&cap.owner_path, qualmap),
        ) else {
            continue;
        };
        let strict = fq != oq;
        let self_owned = fq == oq && plan.owners.get(&fq).is_some_and(|v| v.contains(&cap.var));
        if strict || self_owned {
            let read = plan.captured.entry(fq).or_default();
            if !read.contains(&cap.var) {
                read.push(cap.var.clone());
            }
        }
    }
    let readers: Vec<String> = plan.captured.keys().cloned().collect();
    for f in readers {
        let mut chain = Vec::new();
        let mut cur = f.clone();
        while let Some(p) = plan.parents.get(&cur) {
            if plan.owners.contains_key(p) {
                chain.push(p.clone());
            }
            cur = p.clone();
        }
        chain.reverse();
        plan.chains.insert(f, chain);
    }
}

// Marker callee for closure materialization (custom-emitted, never linked).
pub(crate) const CLOSURE_NEW: &str = "alya$closure_new";
// Marker callee for chain-array construction (custom-emitted, never linked).
pub(crate) const CLOSURE_CHAIN: &str = "alya$closure_chain";
// Hidden trailing chain parameter for capturing functions.
const CHAIN_PARAM: &str = "chain$clo";

// Rewrites for capture: hidden chain params (+ `alya::capturing`
// marks for the dynspec gate), direct-call chain args, and
// value-reference materializations.
fn rewrite_captures(program: &mut Program, plan: &CapturePlan) {
    for stmt in program.statements.iter_mut() {
        if let Stmt::Function {
            name,
            params,
            param_types,
            body,
            attributes,
            ..
        } = stmt.inner_stmt_mut()
        {
            if plan.needs_chain(name) {
                params.push(CHAIN_PARAM.to_string());
                param_types.push(None);
                attributes.push(Attribute {
                    name: "alya::capturing".to_string(),
                    args: vec![],
                });
            }
            rewrite_cap_body(body, plan);
        }
    }
}

fn rewrite_cap_body(body: &mut [Stmt], plan: &CapturePlan) {
    for stmt in body.iter_mut() {
        rewrite_cap_stmt(stmt, plan);
    }
}

fn rewrite_cap_stmt(stmt: &mut Stmt, plan: &CapturePlan) {
    match stmt.inner_stmt_mut() {
        Stmt::Say(e) | Stmt::Expr(e) => rewrite_cap_expr(e, plan),
        Stmt::Return(opt) | Stmt::Throw(opt) => {
            if let Some(e) = opt {
                rewrite_cap_expr(e, plan);
            }
        }
        Stmt::Break | Stmt::Continue => {}
        Stmt::Let { value, .. } | Stmt::Const { value, .. } | Stmt::Assign { value, .. } => {
            rewrite_cap_expr(value, plan)
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            rewrite_cap_expr(array, plan);
            rewrite_cap_expr(index, plan);
            rewrite_cap_expr(value, plan);
        }
        Stmt::FieldAssign { object, value, .. } => {
            rewrite_cap_expr(object, plan);
            rewrite_cap_expr(value, plan);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            rewrite_cap_expr(condition, plan);
            rewrite_cap_body(then_block, plan);
            if let Some(eb) = else_block {
                rewrite_cap_body(eb, plan);
            }
        }
        Stmt::While { condition, body } => {
            rewrite_cap_expr(condition, plan);
            rewrite_cap_body(body, plan);
        }
        Stmt::Repeat { body } => rewrite_cap_body(body, plan),
        Stmt::For {
            start, end, body, ..
        } => {
            rewrite_cap_expr(start, plan);
            rewrite_cap_expr(end, plan);
            rewrite_cap_body(body, plan);
        }
        Stmt::ForEach { iterable, body, .. } => {
            rewrite_cap_expr(iterable, plan);
            rewrite_cap_body(body, plan);
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            rewrite_cap_body(try_block, plan);
            rewrite_cap_body(catch_block, plan);
            if let Some(fb) = finally_block {
                rewrite_cap_body(fb, plan);
            }
        }
        Stmt::Defer(inner) | Stmt::Pub(inner) => rewrite_cap_stmt(inner, plan),
        // Unextracted (defer-wrapped) definitions and declarations:
        // nothing to rewrite.
        _ => {}
    }
}

fn rewrite_cap_expr(expr: &mut Expr, plan: &CapturePlan) {
    match expr {
        Expr::Call { name, args } => {
            for a in args.iter_mut() {
                rewrite_cap_expr(a, plan);
            }
            if plan.needs_chain(name) {
                args.push(Expr::Call {
                    name: CLOSURE_CHAIN.to_string(),
                    args: vec![Expr::Identifier(name.clone())],
                });
            }
        }
        Expr::OptionalCall { callee, args } => {
            for a in args.iter_mut() {
                rewrite_cap_expr(a, plan);
            }
            if plan.needs_chain(callee) {
                args.push(Expr::Call {
                    name: CLOSURE_CHAIN.to_string(),
                    args: vec![Expr::Identifier(callee.clone())],
                });
            }
        }
        Expr::Identifier(n) => {
            // Capturing names are `$`-qualified post-hoist and cannot
            // be shadowed by user locals (`$` is unspellable), so no
            // shadow check is needed here.
            if plan.needs_chain(n) {
                let nm = n.clone();
                *expr = Expr::Call {
                    name: CLOSURE_NEW.to_string(),
                    args: vec![Expr::Identifier(nm)],
                };
            }
        }
        Expr::Binary { left, right, .. } => {
            rewrite_cap_expr(left, plan);
            rewrite_cap_expr(right, plan);
        }
        Expr::Unary { expr: e, .. }
        | Expr::ForceUnwrap(e)
        | Expr::Cast { expr: e, .. }
        | Expr::TypeCheck { expr: e, .. } => {
            rewrite_cap_expr(e, plan);
        }
        Expr::InterpolatedString(parts) | Expr::Array(parts) => {
            for p in parts.iter_mut() {
                rewrite_cap_expr(p, plan);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            rewrite_cap_expr(array, plan);
            rewrite_cap_expr(index, plan);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            rewrite_cap_expr(object, plan);
        }
        Expr::StructInit { fields, .. } => {
            for (_, v) in fields.iter_mut() {
                rewrite_cap_expr(v, plan);
            }
        }
        Expr::Map(entries) => {
            // Keys stay data (consistent with `rewrite_expr`).
            for (_, v) in entries.iter_mut() {
                rewrite_cap_expr(v, plan);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            rewrite_cap_expr(condition, plan);
            rewrite_cap_expr(then_branch, plan);
            rewrite_cap_expr(else_branch, plan);
        }
        Expr::NullCoalesce { value, default } => {
            rewrite_cap_expr(value, plan);
            rewrite_cap_expr(default, plan);
        }
        Expr::Number(_) | Expr::Float(_) | Expr::String(_) | Expr::Null => {}
    }
}
