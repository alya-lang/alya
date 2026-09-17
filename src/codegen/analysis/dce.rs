use crate::ast::*;
use std::collections::{HashMap, HashSet};

/// Eliminates unreferenced functions and unused structs from the AST.
/// Starts from entry points (top-level statements and any `main` function)
/// and transitively retains only reachable functions and structs.
pub fn eliminate_dead_code(program: &Program) -> Program {
    let mut function_defs: HashMap<String, Stmt> = HashMap::new();
    let mut functions_by_bare: HashMap<String, Vec<String>> = HashMap::new();

    let mut struct_defs: HashMap<String, Stmt> = HashMap::new();
    let mut structs_by_bare: HashMap<String, Vec<String>> = HashMap::new();

    let mut top_level: Vec<&Stmt> = Vec::new();

    for stmt in &program.statements {
        match stmt.inner_stmt() {
            Stmt::Function { name, .. } => {
                let bare = bare_name(name);
                function_defs.insert(name.clone(), stmt.clone());
                functions_by_bare
                    .entry(bare.to_string())
                    .or_default()
                    .push(name.clone());
            }
            Stmt::StructDef { name, .. } => {
                let bare = bare_name(name);
                struct_defs.insert(name.clone(), stmt.clone());
                structs_by_bare
                    .entry(bare.to_string())
                    .or_default()
                    .push(name.clone());
            }
            _ => {
                top_level.push(stmt);
            }
        }
    }

    // Check if there is any `main` function defined
    let has_main = functions_by_bare.contains_key("main");

    // If there are no top-level statements and no `main` function, this is a pure
    // library file compiled in isolation. Keep all definitions to preserve compilation.
    if top_level.is_empty() && !has_main {
        return program.clone();
    }

    let mut reachable_functions: HashSet<String> = HashSet::new();
    let mut reachable_structs: HashSet<String> = HashSet::new();

    enum WorkItem {
        Function(String),
        Struct(String),
    }

    let mut worklist: Vec<WorkItem> = Vec::new();

    let mark_function =
        |fn_name: &str, reachable_functions: &mut HashSet<String>, worklist: &mut Vec<WorkItem>| {
            if reachable_functions.insert(fn_name.to_string()) {
                worklist.push(WorkItem::Function(fn_name.to_string()));
            }
        };

    let resolve_symbol = |symbol: &str,
                          function_defs: &HashMap<String, Stmt>,
                          functions_by_bare: &HashMap<String, Vec<String>>,
                          struct_defs: &HashMap<String, Stmt>,
                          structs_by_bare: &HashMap<String, Vec<String>>,
                          reachable_functions: &mut HashSet<String>,
                          reachable_structs: &mut HashSet<String>,
                          worklist: &mut Vec<WorkItem>| {
        let bare = bare_name(symbol);

        // Check exact function
        if function_defs.contains_key(symbol) && reachable_functions.insert(symbol.to_string()) {
            worklist.push(WorkItem::Function(symbol.to_string()));
        }
        // Also check any candidate functions matching bare name (handles aliases and struct methods)
        if let Some(candidates) = functions_by_bare.get(bare) {
            for cand in candidates {
                if reachable_functions.insert(cand.clone()) {
                    worklist.push(WorkItem::Function(cand.clone()));
                }
            }
        }

        // Check exact struct
        if struct_defs.contains_key(symbol) && reachable_structs.insert(symbol.to_string()) {
            worklist.push(WorkItem::Struct(symbol.to_string()));
        }
        if let Some(candidates) = structs_by_bare.get(bare) {
            for cand in candidates {
                if reachable_structs.insert(cand.clone()) {
                    worklist.push(WorkItem::Struct(cand.clone()));
                }
            }
        }
    };

    // 1. If any `main` function exists, it is always a root
    if let Some(main_fns) = functions_by_bare.get("main") {
        for m in main_fns {
            mark_function(m, &mut reachable_functions, &mut worklist);
        }
    }

    // 2. All top-level statements are roots
    let mut initial_refs = HashSet::new();
    for stmt in &top_level {
        collect_references_in_stmt(stmt, &mut initial_refs);
    }
    for r in &initial_refs {
        resolve_symbol(
            r,
            &function_defs,
            &functions_by_bare,
            &struct_defs,
            &structs_by_bare,
            &mut reachable_functions,
            &mut reachable_structs,
            &mut worklist,
        );
    }

    // 3. Process worklist until fixpoint
    while let Some(item) = worklist.pop() {
        let mut item_refs = HashSet::new();
        match item {
            WorkItem::Function(fn_name) => {
                if let Some(stmt) = function_defs.get(&fn_name) {
                    if let Stmt::Function {
                        params: _,
                        param_types,
                        return_type,
                        defaults,
                        body,
                        ..
                    } = stmt.inner_stmt()
                    {
                        for t in param_types.iter().flatten() {
                            let clean = clean_type_name(t);
                            item_refs.insert(clean);
                        }
                        if let Some(rt) = return_type {
                            let clean = clean_type_name(rt);
                            item_refs.insert(clean);
                        }
                        for expr in defaults.iter().flatten() {
                            collect_references_in_expr(expr, &mut item_refs);
                        }
                        for s in body {
                            collect_references_in_stmt(s, &mut item_refs);
                        }
                    }
                }
            }
            WorkItem::Struct(st_name) => {
                if let Some(stmt) = struct_defs.get(&st_name) {
                    if let Stmt::StructDef {
                        field_types,
                        defaults,
                        ..
                    } = stmt.inner_stmt()
                    {
                        for t in field_types.iter().flatten() {
                            let clean = clean_type_name(t);
                            item_refs.insert(clean);
                        }
                        for expr in defaults.iter().flatten() {
                            collect_references_in_expr(expr, &mut item_refs);
                        }
                    }
                }
            }
        }

        for r in &item_refs {
            resolve_symbol(
                r,
                &function_defs,
                &functions_by_bare,
                &struct_defs,
                &structs_by_bare,
                &mut reachable_functions,
                &mut reachable_structs,
                &mut worklist,
            );
        }
    }

    // 4. Rebuild program statements retaining only reachable definitions
    let mut pruned_statements = Vec::new();
    for stmt in &program.statements {
        match stmt.inner_stmt() {
            Stmt::Function { name, .. } => {
                if reachable_functions.contains(name) {
                    pruned_statements.push(stmt.clone());
                }
            }
            Stmt::StructDef { name, .. } => {
                if reachable_structs.contains(name) {
                    pruned_statements.push(stmt.clone());
                }
            }
            _ => {
                pruned_statements.push(stmt.clone());
            }
        }
    }

    Program {
        statements: pruned_statements,
    }
}

fn bare_name(name: &str) -> &str {
    let bare = name.rsplit("::").next().unwrap_or(name);
    bare.rsplit("__").next().unwrap_or(bare)
}

fn clean_type_name(t: &str) -> String {
    let t = t.trim_end_matches("[]");
    let bare = bare_name(t);
    bare.to_string()
}

fn collect_references_in_stmt(stmt: &Stmt, refs: &mut HashSet<String>) {
    match stmt {
        Stmt::Expr(expr) | Stmt::Say(expr) => {
            collect_references_in_expr(expr, refs);
        }
        Stmt::Let {
            value, type_ann, ..
        } => {
            if let Some(t) = type_ann {
                refs.insert(clean_type_name(t));
            }
            collect_references_in_expr(value, refs);
        }
        Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
            collect_references_in_expr(value, refs);
        }
        Stmt::IndexAssign {
            array,
            index,
            value,
        } => {
            collect_references_in_expr(array, refs);
            collect_references_in_expr(index, refs);
            collect_references_in_expr(value, refs);
        }
        Stmt::FieldAssign { object, value, .. } => {
            collect_references_in_expr(object, refs);
            collect_references_in_expr(value, refs);
        }
        Stmt::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_references_in_expr(condition, refs);
            for s in then_block {
                collect_references_in_stmt(s, refs);
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    collect_references_in_stmt(s, refs);
                }
            }
        }
        Stmt::While { condition, body } => {
            collect_references_in_expr(condition, refs);
            for s in body {
                collect_references_in_stmt(s, refs);
            }
        }
        Stmt::Repeat { body } => {
            for s in body {
                collect_references_in_stmt(s, refs);
            }
        }
        Stmt::For {
            start, end, body, ..
        } => {
            collect_references_in_expr(start, refs);
            collect_references_in_expr(end, refs);
            for s in body {
                collect_references_in_stmt(s, refs);
            }
        }
        Stmt::ForEach { iterable, body, .. } => {
            collect_references_in_expr(iterable, refs);
            for s in body {
                collect_references_in_stmt(s, refs);
            }
        }
        Stmt::Return(opt_expr) | Stmt::Throw(opt_expr) => {
            if let Some(expr) = opt_expr {
                collect_references_in_expr(expr, refs);
            }
        }
        Stmt::TryCatch {
            try_block,
            catch_block,
            finally_block,
            ..
        } => {
            for s in try_block {
                collect_references_in_stmt(s, refs);
            }
            for s in catch_block {
                collect_references_in_stmt(s, refs);
            }
            if let Some(finally_stmts) = finally_block {
                for s in finally_stmts {
                    collect_references_in_stmt(s, refs);
                }
            }
        }
        Stmt::Defer(inner) => {
            collect_references_in_stmt(inner, refs);
        }
        Stmt::Pub(inner) => {
            collect_references_in_stmt(inner, refs);
        }
        _ => {}
    }
}

fn collect_references_in_expr(expr: &Expr, refs: &mut HashSet<String>) {
    match expr {
        Expr::Identifier(id) => {
            refs.insert(id.clone());
        }
        Expr::Call { name, args } => {
            refs.insert(name.clone());
            for arg in args {
                collect_references_in_expr(arg, refs);
            }
        }
        Expr::OptionalCall { callee, args } => {
            refs.insert(callee.clone());
            for arg in args {
                collect_references_in_expr(arg, refs);
            }
        }
        Expr::Binary { left, right, .. } => {
            collect_references_in_expr(left, refs);
            collect_references_in_expr(right, refs);
        }
        Expr::Unary { expr, .. } => {
            collect_references_in_expr(expr, refs);
        }
        Expr::Array(elements) => {
            for elem in elements {
                collect_references_in_expr(elem, refs);
            }
        }
        Expr::Index { array, index } | Expr::OptionalIndex { array, index } => {
            collect_references_in_expr(array, refs);
            collect_references_in_expr(index, refs);
        }
        Expr::FieldAccess { object, .. } | Expr::OptionalFieldAccess { object, .. } => {
            collect_references_in_expr(object, refs);
        }
        Expr::StructInit { name, fields } => {
            refs.insert(name.clone());
            for (_, val) in fields {
                collect_references_in_expr(val, refs);
            }
        }
        Expr::Map(entries) => {
            for (k, v) in entries {
                collect_references_in_expr(k, refs);
                collect_references_in_expr(v, refs);
            }
        }
        Expr::InterpolatedString(parts) => {
            for part in parts {
                collect_references_in_expr(part, refs);
            }
        }
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_references_in_expr(condition, refs);
            collect_references_in_expr(then_branch, refs);
            collect_references_in_expr(else_branch, refs);
        }
        Expr::NullCoalesce { value, default } => {
            collect_references_in_expr(value, refs);
            collect_references_in_expr(default, refs);
        }
        Expr::TypeCheck { expr, target, .. } => {
            refs.insert(target.clone());
            collect_references_in_expr(expr, refs);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dce_prunes_unreferenced_function() {
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "used_fn".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(42.0))],
                },
                Stmt::Function {
                    name: "dead_fn".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(99.0))],
                },
                Stmt::Expr(Expr::Call {
                    name: "used_fn".into(),
                    args: vec![],
                }),
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let fn_names: Vec<String> = pruned
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::Function { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(fn_names, vec!["used_fn"]);
    }

    #[test]
    fn test_dce_transitive_reachability() {
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "entry_fn".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Expr(Expr::Call {
                        name: "helper_fn".into(),
                        args: vec![],
                    })],
                },
                Stmt::Function {
                    name: "helper_fn".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![Stmt::Say(Expr::Number(1.0))],
                },
                Stmt::Function {
                    name: "unreachable_fn".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Expr(Expr::Call {
                    name: "entry_fn".into(),
                    args: vec![],
                }),
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let fn_names: Vec<String> = pruned
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::Function { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(fn_names.contains(&"entry_fn".to_string()));
        assert!(fn_names.contains(&"helper_fn".to_string()));
        assert!(!fn_names.contains(&"unreachable_fn".to_string()));
    }

    #[test]
    fn test_dce_callback_identifier() {
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "on_event".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "register_cb".into(),
                    params: vec!["cb".into()],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Expr(Expr::Call {
                    name: "register_cb".into(),
                    args: vec![Expr::Identifier("on_event".into())],
                }),
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let fn_names: Vec<String> = pruned
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::Function { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert!(fn_names.contains(&"on_event".to_string()));
        assert!(fn_names.contains(&"register_cb".to_string()));
    }

    #[test]
    fn test_dce_struct_reachability() {
        let program = Program {
            statements: vec![
                Stmt::StructDef {
                    name: "ActivePoint".into(),
                    fields: vec!["x".into(), "y".into()],
                    field_types: vec![None, None],
                    defaults: vec![None, None],
                },
                Stmt::StructDef {
                    name: "UnusedPoint".into(),
                    fields: vec!["z".into()],
                    field_types: vec![None],
                    defaults: vec![None],
                },
                Stmt::Expr(Expr::StructInit {
                    name: "ActivePoint".into(),
                    fields: vec![
                        ("x".into(), Expr::Number(1.0)),
                        ("y".into(), Expr::Number(2.0)),
                    ],
                }),
            ],
        };

        let pruned = eliminate_dead_code(&program);
        let struct_names: Vec<String> = pruned
            .statements
            .iter()
            .filter_map(|s| {
                if let Stmt::StructDef { name, .. } = s.inner_stmt() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(struct_names, vec!["ActivePoint"]);
    }

    #[test]
    fn test_dce_preserves_pure_library() {
        let program = Program {
            statements: vec![
                Stmt::Function {
                    name: "lib_a".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
                Stmt::Function {
                    name: "lib_b".into(),
                    params: vec![],
                    param_types: vec![],
                    return_type: None,
                    defaults: vec![],
                    body: vec![],
                },
            ],
        };

        let pruned = eliminate_dead_code(&program);
        assert_eq!(pruned.statements.len(), 2);
    }
}
